// Original Pitex PostScript image and stream operators, from the Adobe PLRM.
// Copyright (c) 2026 Pitex contributors. SPDX-License-Identifier: AGPL-3.0-or-later
use super::*;
pub fn recognizes(op: &str) -> bool {
    matches!(
        op,
        "image"
            | "imagemask"
            | "colorimage"
            | "setcolor"
            | "currentcolor"
            | "setcolorspace"
            | "currentcolorspace"
            | "filter"
            | "currentfile"
            | "read"
            | "readstring"
            | "readhexstring"
            | "closefile"
            | "bytesavailable"
            | "resetfile"
            | "fileposition"
            | "setfileposition"
    )
}
fn bytes(value: &Value) -> Result<Vec<u8>, String> {
    match value {
        Value::String(v) => {
            v.read()?;
            Ok(v.borrow().to_vec())
        }
        Value::File(v) => {
            let mut file = v.borrow_mut();
            let result = file.0[file.1..].to_vec();
            file.1 = file.0.len();
            Ok(result)
        }
        _ => Err("typecheck: image source".into()),
    }
}
fn decode(input: &[u8], name: &str) -> Result<(Vec<u8>, usize), String> {
    decode_parameters(input, name, &BTreeMap::new())
}
fn decode_parameters(
    input: &[u8],
    name: &str,
    parameters: &BTreeMap<String, Value>,
) -> Result<(Vec<u8>, usize), String> {
    let error = || format!("ioerror: {name}");
    match name {
        "ASCIIHexDecode" => {
            let mut digits = Vec::new();
            let mut consumed = input.len();
            for (i, b) in input.iter().enumerate() {
                if *b == b'>' {
                    consumed = i + 1;
                    break;
                }
                if b.is_ascii_whitespace() {
                    continue;
                }
                digits.push((*b as char).to_digit(16).ok_or_else(error)? as u8);
            }
            if digits.len() % 2 != 0 {
                digits.push(0);
            }
            Ok((
                digits.chunks(2).map(|p| p[0] * 16 + p[1]).collect(),
                consumed,
            ))
        }
        "ASCII85Decode" => {
            let mut out = Vec::new();
            let mut group = Vec::new();
            let mut i = 0;
            while i < input.len() {
                let b = input[i];
                i += 1;
                if b.is_ascii_whitespace() {
                    continue;
                }
                if b == b'~' {
                    if input.get(i) != Some(&b'>') {
                        return Err(error());
                    }
                    i += 1;
                    break;
                }
                if b == b'z' && group.is_empty() {
                    out.extend([0; 4]);
                    continue;
                }
                if !(b'!'..=b'u').contains(&b) {
                    return Err(error());
                }
                group.push((b - b'!') as u64);
                if group.len() == 5 {
                    let value = group.iter().fold(0u64, |a, b| a * 85 + b);
                    if value > u32::MAX as u64 {
                        return Err(error());
                    }
                    out.extend((value as u32).to_be_bytes());
                    group.clear();
                }
            }
            if group.len() == 1 {
                return Err(error());
            }
            if !group.is_empty() {
                let count = group.len() - 1;
                group.resize(5, 84);
                let value = group.iter().fold(0u64, |a, b| a * 85 + b);
                if value > u32::MAX as u64 {
                    return Err(error());
                }
                out.extend_from_slice(&(value as u32).to_be_bytes()[..count]);
            }
            Ok((out, i))
        }
        "RunLengthDecode" => {
            let mut out = Vec::new();
            let mut i = 0;
            while i < input.len() {
                let n = input[i];
                i += 1;
                if n == 128 {
                    break;
                }
                if n < 128 {
                    let count = n as usize + 1;
                    out.extend_from_slice(input.get(i..i + count).ok_or_else(error)?);
                    i += count;
                } else {
                    out.extend(
                        std::iter::repeat(*input.get(i).ok_or_else(error)?).take(257 - n as usize),
                    );
                    i += 1;
                }
                if out.len() > 128 * 1024 * 1024 {
                    return Err("limitcheck: decoded stream".into());
                }
            }
            Ok((out, i))
        }
        "FlateDecode" => {
            unsafe extern "C" {
                fn uncompress2(
                    dest: *mut u8,
                    len: *mut libc::c_ulong,
                    source: *const u8,
                    source_len: *mut libc::c_ulong,
                ) -> i32;
            }
            let mut size = (input.len() * 4).max(1024);
            while size <= 128 * 1024 * 1024 {
                let mut out = vec![0; size];
                let mut count = size as libc::c_ulong;
                let mut used = input.len() as libc::c_ulong;
                let result =
                    unsafe { uncompress2(out.as_mut_ptr(), &mut count, input.as_ptr(), &mut used) };
                if result == 0 {
                    out.truncate(count as usize);
                    return Ok((out, used as usize));
                }
                if result != -5 {
                    return Err(error());
                }
                size *= 2;
            }
            Err("limitcheck: decoded stream".into())
        }
        "DCTDecode" => {
            let prefix = input.iter().take_while(|b| b.is_ascii_whitespace()).count();
            let mut cursor = std::io::Cursor::new(&input[prefix..]);
            let result = {
                let mut decoder = jpeg_decoder::Decoder::new(&mut cursor);
                decoder.set_max_decoding_buffer_size(128 * 1024 * 1024);
                if parameters
                    .get("ColorTransform")
                    .and_then(|v| v.number().ok())
                    == Some(0.)
                {
                    decoder.set_color_transform(jpeg_decoder::ColorTransform::None);
                }
                decoder
                    .decode()
                    .map_err(|e| format!("ioerror: DCTDecode {e}"))?
            };
            Ok((result, prefix + cursor.position() as usize))
        }
        "LZWDecode" => {
            let mut dictionary = (0u16..256).map(|v| vec![v as u8]).collect::<Vec<_>>();
            dictionary.extend([vec![], vec![]]);
            let (mut width, mut bit, mut previous, mut out) =
                (9usize, 0usize, None::<Vec<u8>>, Vec::new());
            loop {
                if bit + width > input.len() * 8 {
                    break;
                }
                let mut code = 0usize;
                for _ in 0..width {
                    code = (code << 1) | ((input[bit / 8] >> (7 - bit % 8)) & 1) as usize;
                    bit += 1;
                }
                if code == 256 {
                    dictionary.truncate(258);
                    width = 9;
                    previous = None;
                    continue;
                }
                if code == 257 {
                    break;
                }
                let entry = if code < dictionary.len() {
                    dictionary[code].clone()
                } else if code == dictionary.len() {
                    let mut value = previous.clone().ok_or_else(error)?;
                    value.push(value[0]);
                    value
                } else {
                    return Err(error());
                };
                if entry.is_empty() {
                    return Err(error());
                }
                out.extend(&entry);
                if let Some(mut value) = previous {
                    if dictionary.len() < 4096 {
                        value.push(entry[0]);
                        dictionary.push(value);
                        if width < 12
                            && dictionary.len()
                                + parameters
                                    .get("EarlyChange")
                                    .and_then(|v| v.number().ok())
                                    .unwrap_or(1.)
                                    .clamp(0., 1.) as usize
                                == (1 << width)
                        {
                            width += 1;
                        }
                    }
                }
                previous = Some(entry);
                if out.len() > 128 * 1024 * 1024 {
                    return Err("limitcheck: decoded stream".into());
                }
            }
            Ok((out, (bit + 7) / 8))
        }
        _ => Err(format!("undefined: filter {name}")),
    }
}
fn undo_predictor(
    mut bytes: Vec<u8>,
    parameters: &BTreeMap<String, Value>,
) -> Result<Vec<u8>, String> {
    let number = |key: &str, default: usize| {
        parameters
            .get(key)
            .and_then(|v| v.number().ok())
            .map(|v| v as usize)
            .unwrap_or(default)
    };
    let predictor = number("Predictor", 1);
    if predictor == 1 {
        return Ok(bytes);
    }
    let colors = number("Colors", 1);
    let bits = number("BitsPerComponent", 8);
    let columns = number("Columns", 1);
    if colors == 0 || columns == 0 || !matches!(bits, 1 | 2 | 4 | 8 | 16) {
        return Err("rangecheck: filter predictor".into());
    }
    let samples = columns
        .checked_mul(colors)
        .ok_or("limitcheck: filter predictor")?;
    let row = samples
        .checked_mul(bits)
        .ok_or("limitcheck: filter predictor")?
        .div_ceil(8);
    if predictor == 2 {
        if bytes.len() % row != 0 {
            return Err("ioerror: short predictor row".into());
        }
        for line in bytes.chunks_mut(row) {
            for sample in colors..samples {
                let mut left = 0u32;
                let mut current = 0u32;
                for bit in 0..bits {
                    let a = (sample - colors) * bits + bit;
                    let b = sample * bits + bit;
                    left = (left << 1) | ((line[a / 8] >> (7 - a % 8)) & 1) as u32;
                    current = (current << 1) | ((line[b / 8] >> (7 - b % 8)) & 1) as u32;
                }
                let value = (left + current) & ((1u32 << bits) - 1);
                for bit in 0..bits {
                    let at = sample * bits + bit;
                    let mask = 1u8 << (7 - at % 8);
                    line[at / 8] = (line[at / 8] & !mask)
                        | ((((value >> (bits - 1 - bit)) & 1) as u8) << (7 - at % 8));
                }
            }
        }
        return Ok(bytes);
    }
    if !(10..=15).contains(&predictor) || bytes.len() % (row + 1) != 0 {
        return Err("rangecheck: PNG predictor".into());
    }
    let bpp = (colors * bits).div_ceil(8);
    let mut previous = vec![0u8; row];
    let mut out = Vec::new();
    for line in bytes.chunks(row + 1) {
        let filter = line[0];
        let mut decoded = line[1..].to_vec();
        for i in 0..row {
            let a = if i >= bpp { decoded[i - bpp] } else { 0 };
            let b = previous[i];
            let c = if i >= bpp { previous[i - bpp] } else { 0 };
            let predicted = match filter {
                0 => 0,
                1 => a,
                2 => b,
                3 => ((a as u16 + b as u16) / 2) as u8,
                4 => {
                    let p = a as i32 + b as i32 - c as i32;
                    let (pa, pb, pc) = (
                        (p - a as i32).abs(),
                        (p - b as i32).abs(),
                        (p - c as i32).abs(),
                    );
                    if pa <= pb && pa <= pc {
                        a
                    } else if pb <= pc {
                        b
                    } else {
                        c
                    }
                }
                _ => return Err("rangecheck: PNG predictor filter".into()),
            };
            decoded[i] = decoded[i].wrapping_add(predicted);
        }
        out.extend(&decoded);
        previous = decoded;
    }
    Ok(out)
}

struct SourceBuffer {
    value: Value,
    pending: Vec<u8>,
    initialized: bool,
}
pub(super) fn sample(data: &[u8], bit: usize, bits: usize) -> u32 {
    (0..bits).fold(0, |value, i| {
        (value << 1) | ((data[(bit + i) / 8] >> (7 - (bit + i) % 8)) & 1) as u32
    })
}
fn store_sample(data: &mut [u8], bit: usize, bits: usize, value: u32) {
    for i in 0..bits {
        data[(bit + i) / 8] |= (((value >> (bits - 1 - i)) & 1) as u8) << (7 - (bit + i) % 8);
    }
}
fn dimensions(
    width: usize,
    height: usize,
    bits: usize,
    components: usize,
) -> Result<(usize, usize), String> {
    if width == 0
        || height == 0
        || width > 30000
        || height > 30000
        || !matches!(bits, 1 | 2 | 4 | 8 | 12 | 16)
        || components == 0
        || components > 32
    {
        return Err("rangecheck: image dimensions".into());
    }
    let row = width
        .checked_mul(components * bits)
        .map(|n| (n + 7) / 8)
        .ok_or("limitcheck: image")?;
    let length = row
        .checked_mul(height)
        .filter(|n| *n <= 128 * 1024 * 1024)
        .ok_or("limitcheck: image")?;
    Ok((row, length))
}
fn normalize_12(
    data: Vec<u8>,
    width: usize,
    height: usize,
    components: usize,
    bits: usize,
) -> (Vec<u8>, usize) {
    if bits != 12 {
        return (data, bits);
    }
    let row = (width * components * 12 + 7) / 8;
    let mut out = Vec::with_capacity(width * height * components * 2);
    for y in 0..height {
        for x in 0..width * components {
            let value = sample(&data, y * row * 8 + x * 12, 12);
            out.extend((((value * 65535 + 2047) / 4095) as u16).to_be_bytes());
        }
    }
    (out, 16)
}
#[derive(Clone)]
struct Image {
    width: usize,
    height: usize,
    bits: usize,
    matrix: [f64; 6],
    sources: Vec<Value>,
    decode: Option<Vec<f64>>,
    interpolate: bool,
}
impl Interpreter {
    fn image_row(&mut self, source: &mut SourceBuffer, length: usize) -> Result<Vec<u8>, String> {
        if let Value::File(_) = &source.value {
            return self.image_source(&source.value, length);
        }
        if let Value::String(data) = &source.value {
            if !source.initialized {
                data.read()?;
                source.pending = data.borrow().to_vec();
                source.initialized = true;
            }
        }
        while source.pending.len() < length {
            if matches!(source.value, Value::String(_)) {
                return Err("ioerror: image source exhausted".into());
            }
            self.execute(source.value.clone())?;
            let value = self.pop()?;
            let Value::String(data) = value else {
                return Err("typecheck: image procedure source".into());
            };
            data.read()?;
            if data.borrow().is_empty() {
                return Err("ioerror: empty image source".into());
            }
            source.pending.extend(data.borrow().iter());
        }
        Ok(source.pending.drain(..length).collect())
    }
    fn image_samples(&mut self, image: &Image, components: usize) -> Result<Vec<u8>, String> {
        let (row, length) = dimensions(image.width, image.height, image.bits, components)?;
        if image.sources.len() != 1 && image.sources.len() != components {
            return Err("rangecheck: image sources".into());
        }
        let mut sources = image
            .sources
            .iter()
            .cloned()
            .map(|value| SourceBuffer {
                value,
                pending: vec![],
                initialized: false,
            })
            .collect::<Vec<_>>();
        let mut result = vec![0; length];
        for y in 0..image.height {
            if sources.len() == 1 {
                let bytes = self.image_row(&mut sources[0], row)?;
                result[y * row..(y + 1) * row].copy_from_slice(&bytes);
            } else {
                let plane_row = (image.width * image.bits + 7) / 8;
                for (c, source) in sources.iter_mut().enumerate() {
                    let plane = self.image_row(source, plane_row)?;
                    for x in 0..image.width {
                        store_sample(
                            &mut result,
                            y * row * 8 + (x * components + c) * image.bits,
                            image.bits,
                            sample(&plane, x * image.bits, image.bits),
                        );
                    }
                }
            }
        }
        Ok(result)
    }
    fn image_dictionary(&self, value: &Value, mask: bool) -> Result<Image, String> {
        let Value::Dict(dict) = value else {
            return Err("typecheck: image dictionary".into());
        };
        self.vm.read(value)?;
        let dict = dict.borrow();
        let get = |key: &str| {
            dict.get(key)
                .ok_or_else(|| format!("undefined: Image {key}"))
        };
        let sources = match dict.get("DataSource").unwrap_or(&Value::Null) {
            Value::Array(a) => {
                a.read()?;
                a.borrow().to_vec()
            }
            value => vec![value.clone()],
        };
        let decode = dict.get("Decode").map(color::numbers).transpose()?;
        Ok(Image {
            width: get("Width")?.number()? as usize,
            height: get("Height")?.number()? as usize,
            bits: if mask {
                1
            } else {
                get("BitsPerComponent")?.number()? as usize
            },
            matrix: Self::matrix_value(get("ImageMatrix")?)?,
            sources,
            decode,
            interpolate: dict
                .get("Interpolate")
                .map(Value::boolean)
                .transpose()?
                .unwrap_or(false),
        })
    }
    fn image_begin(&mut self, placement: [f64; 6]) {
        self.output += "q\n";
        for (path, even) in &self.graphics.clips {
            self.output += &format_path(path);
            self.output += if *even { "W* n\n" } else { "W n\n" };
        }
        if self.graphics.fill_alpha != 1. || self.graphics.stroke_alpha != 1. {
            let key = self
                .alphas
                .iter()
                .find(|(_, a, b)| {
                    *a == self.graphics.fill_alpha && *b == self.graphics.stroke_alpha
                })
                .map(|(key, _, _)| key.clone())
                .unwrap_or_else(|| {
                    let key = format!("PSTalpha{}", self.alphas.len());
                    self.alphas.push((
                        key.clone(),
                        self.graphics.fill_alpha,
                        self.graphics.stroke_alpha,
                    ));
                    key
                });
            self.output += &format!("/{key} gs\n");
        }
        self.output += &format!(
            "{}\n{} cm\n",
            self.graphics.color,
            placement
                .iter()
                .map(|v| format!("{v:.6}"))
                .collect::<Vec<_>>()
                .join(" ")
        );
    }
    fn image_placement(&self, image: &Image) -> Result<[f64; 6], String> {
        Ok(multiply(
            self.graphics.matrix,
            multiply(
                inverse(image.matrix)?,
                [
                    image.width as f64,
                    0.,
                    0.,
                    -(image.height as f64),
                    0.,
                    image.height as f64,
                ],
            ),
        ))
    }
    fn image_source(&mut self, value: &Value, length: usize) -> Result<Vec<u8>, String> {
        if let Value::File(file) = value {
            self.vm.read(value)?;
            let mut file = file.borrow_mut();
            let end = file.1.checked_add(length).ok_or("limitcheck: image")?;
            let result = file
                .0
                .get(file.1..end)
                .ok_or("ioerror: image source exhausted")?
                .to_vec();
            file.1 = end;
            return Ok(result);
        }
        if let Value::String(data) = value {
            data.read()?;
            let data = data.borrow();
            return data
                .get(..length)
                .map(<[u8]>::to_vec)
                .ok_or("ioerror: image source exhausted".into());
        }
        let mut out = Vec::new();
        while out.len() < length {
            self.execute(value.clone())?;
            let result = self.pop()?;
            if !matches!(result, Value::String(_)) {
                return Err("typecheck: procedure image source must return a string".into());
            }
            let data = bytes(&result)?;
            if data.is_empty() {
                return Err("ioerror: empty image source".into());
            }
            out.extend(data);
        }
        out.truncate(length);
        Ok(out)
    }
    fn emit_image(
        &mut self,
        width: usize,
        height: usize,
        bits: usize,
        matrix: [f64; 6],
        sources: Vec<Value>,
        components: usize,
        mask: Option<bool>,
        decode_values: Option<Vec<f64>>,
        interpolate: bool,
        selected_space: bool,
    ) -> Result<(), String> {
        let image = Image {
            width,
            height,
            bits,
            matrix,
            sources,
            decode: decode_values.clone(),
            interpolate,
        };
        let data = self.image_samples(&image, components)?;
        let (data, bits) = normalize_12(data, width, height, components, bits);
        let placement = self.image_placement(&image)?;
        self.image_begin(placement);
        self.output += &format!("BI /W {width} /H {height} /BPC {bits} /F /ASCIIHexDecode ");
        if let Some(polarity) = mask {
            self.output += &format!("/IM true /D [{}] ", if polarity { "1 0" } else { "0 1" });
        } else {
            let space = if selected_space {
                self.graphics.color_space.clone()
            } else {
                match components {
                    1 => color::Space::Gray,
                    3 => color::Space::Rgb,
                    _ => color::Space::Cmyk,
                }
            };
            let resource = self.color_resource(&space)?;
            self.output += &format!("/CS {resource} ");
            let decoded = space.decode(decode_values, image.bits)?;
            self.output += &format!(
                "/D [{}] ",
                decoded
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(" ")
            );
        }
        if interpolate {
            self.output += "/I true ";
        }
        self.output += "ID\n";
        for byte in data {
            self.output += &format!("{byte:02X}");
        }
        self.output += ">\nEI\nQ\n";
        Ok(())
    }
    fn named_image(
        &mut self,
        image: &Image,
        data: Vec<u8>,
        mask: Option<(Image, Vec<u8>)>,
        keys: Option<Vec<f64>>,
    ) -> Result<(), String> {
        let components = self.graphics.image_components;
        let (data, bits) = normalize_12(data, image.width, image.height, components, image.bits);
        let space = self.graphics.color_space.clone();
        let mut objects = Vec::new();
        let color = self.color_pdf(&space, &mut objects)?;
        let mut mask_entry = String::new();
        if let Some((mask, bytes)) = mask {
            let id = objects.len();
            let decode = mask.decode.unwrap_or(vec![0., 1.]);
            if decode.len() != 2 {
                return Err("rangecheck: mask Decode".into());
            }
            objects.push(shading::Object{dictionary:format!("/Type /XObject /Subtype /Image /Width {} /Height {} /ImageMask true /BitsPerComponent 1 /Decode [{} {}] /Interpolate {}",mask.width,mask.height,decode[0],decode[1],mask.interpolate),data:Some(bytes)});
            mask_entry = format!("/Mask @@{id}@@");
        }
        if let Some(mut values) = keys {
            if values.len() == components {
                values = values.iter().flat_map(|v| [*v, *v]).collect();
            }
            if values.len() != components * 2 || values.chunks(2).any(|v| v[0] > v[1]) {
                return Err("rangecheck: MaskColor".into());
            }
            if image.bits == 12 {
                for v in &mut values {
                    *v = (*v * 65535. / 4095.).round();
                }
            }
            mask_entry = format!(
                "/Mask [{}]",
                values
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(" ")
            );
        }
        let decode = space.decode(image.decode.clone(), image.bits)?;
        let root = objects.len();
        objects.push(shading::Object{dictionary:format!("/Type /XObject /Subtype /Image /Width {} /Height {} /BitsPerComponent {bits} /ColorSpace {color} /Decode [{}] /Interpolate {} {mask_entry}",image.width,image.height,decode.iter().map(ToString::to_string).collect::<Vec<_>>().join(" "),image.interpolate),data:Some(data)});
        let name = format!("PSTimage{}", self.shadings.len());
        self.shadings.push(shading::Shading {
            name: name.clone(),
            objects,
            root,
        });
        let placement = self.image_placement(image)?;
        self.image_begin(placement);
        self.output += &format!("/{name} Do\nQ\n");
        Ok(())
    }
    fn masked_image(&mut self, dict: &BTreeMap<String, Value>) -> Result<(), String> {
        let get = |key: &str| {
            dict.get(key)
                .ok_or_else(|| format!("undefined: Image {key}"))
        };
        let kind = get("ImageType")?.number()? as i32;
        if kind == 4 {
            let image =
                self.image_dictionary(&Value::Dict(Rc::new(RefCell::new(dict.clone()))), false)?;
            let data = self.image_samples(&image, self.graphics.image_components)?;
            let keys = color::numbers(get("MaskColor")?)?;
            return self.named_image(&image, data, None, Some(keys));
        }
        let image = self.image_dictionary(get("DataDict")?, false)?;
        let mut mask = self.image_dictionary(get("MaskDict")?, false)?;
        let interleave = get("InterleaveType")?.number()? as i32;
        let components = self.graphics.image_components;
        let (color_row, color_length) =
            dimensions(image.width, image.height, image.bits, components)?;
        let (mask_row, mask_length) = dimensions(mask.width, mask.height, 1, 1)?;
        let image_matrix = multiply(
            inverse(image.matrix)?,
            [image.width as f64, 0., 0., image.height as f64, 0., 0.],
        );
        let mask_matrix = multiply(
            inverse(mask.matrix)?,
            [mask.width as f64, 0., 0., mask.height as f64, 0., 0.],
        );
        if image_matrix
            .iter()
            .zip(mask_matrix)
            .any(|(a, b)| (a - b).abs() > 0.000001)
        {
            return Err("typecheck: mask placement does not align with image".into());
        }
        let (mut color, mut masked) = (vec![0; color_length], vec![0; mask_length]);
        match interleave {
            1 => {
                if image.sources.len() != 1
                    || image.width != mask.width
                    || image.height != mask.height
                    || image.bits != mask.bits
                {
                    return Err("typecheck: sample-interleaved mask".into());
                }
                let (row, length) =
                    dimensions(image.width, image.height, image.bits, components + 1)?;
                let mut source = SourceBuffer {
                    value: image.sources[0].clone(),
                    pending: vec![],
                    initialized: false,
                };
                let data = self.image_row(&mut source, length)?;
                for y in 0..image.height {
                    for x in 0..image.width {
                        let bit = y * row * 8 + x * (components + 1) * image.bits;
                        let value = sample(&data, bit, image.bits);
                        store_sample(&mut masked, y * mask_row * 8 + x, 1, u32::from(value != 0));
                        for c in 0..components {
                            store_sample(
                                &mut color,
                                y * color_row * 8 + (x * components + c) * image.bits,
                                image.bits,
                                sample(&data, bit + (c + 1) * image.bits, image.bits),
                            );
                        }
                    }
                }
            }
            2 => {
                if image.sources.len() != 1
                    || mask.bits != 1
                    || (image.height % mask.height != 0 && mask.height % image.height != 0)
                {
                    return Err("typecheck: row-interleaved mask".into());
                }
                let mut source = SourceBuffer {
                    value: image.sources[0].clone(),
                    pending: vec![],
                    initialized: false,
                };
                let blocks = image.height.min(mask.height);
                let image_rows = image.height / blocks;
                let mask_rows = mask.height / blocks;
                for block in 0..blocks {
                    let bytes = self.image_row(&mut source, mask_rows * mask_row)?;
                    masked[block * mask_rows * mask_row..(block + 1) * mask_rows * mask_row]
                        .copy_from_slice(&bytes);
                    let bytes = self.image_row(&mut source, image_rows * color_row)?;
                    color[block * image_rows * color_row..(block + 1) * image_rows * color_row]
                        .copy_from_slice(&bytes);
                }
            }
            3 => {
                if mask.bits != 1 || mask.sources.len() != 1 {
                    return Err("typecheck: separate mask".into());
                }
                masked = self.image_samples(&mask, 1)?;
                color = self.image_samples(&image, components)?;
            }
            _ => return Err("rangecheck: InterleaveType".into()),
        }
        mask.bits = 1;
        self.named_image(&image, color, Some((mask, masked)), None)
    }
    pub(super) fn image_operator(&mut self, op: &str) -> Result<(), String> {
        match op {
            "setcolor" => {
                let values = self.nums(self.graphics.image_components)?;
                self.set_selected_color(values)?;
            }
            "currentcolor" => {
                for value in self.graphics.color_values.clone() {
                    self.pushnum(value)?;
                }
            }
            "currentfile" => self.stack.push(Value::File(
                self.current_file
                    .clone()
                    .ok_or("invalidaccess: no current file")?,
            )),
            "setcolorspace" => {
                let value = self.pop()?;
                self.select_space(value)?;
            }
            "currentcolorspace" => {
                let value = match &self.graphics.color_definition {
                    Value::Array(a) => Value::Array(a.clone()),
                    other => Value::Array(vec![other.clone()].into()),
                };
                self.stack.push(value);
            }
            "filter" => {
                let name = self.pop()?.name()?;
                let parameters = if matches!(self.stack.last(), Some(Value::Dict(_))) {
                    let Value::Dict(dict) = self.pop()? else {
                        unreachable!()
                    };
                    self.vm.read(&Value::Dict(dict.clone()))?;
                    let value = dict.borrow().clone();
                    value
                } else {
                    BTreeMap::new()
                };
                let source = self.pop()?;
                self.vm.read(&source)?;
                let (input, start) = match &source {
                    Value::File(file) => {
                        let f = file.borrow();
                        (f.0[f.1..].to_vec(), f.1)
                    }
                    Value::String(s) => (s.borrow().to_vec(), 0),
                    _ => return Err("typecheck: filter source".into()),
                };
                let (data, used) = decode_parameters(&input, &name, &parameters)?;
                let data = undo_predictor(data, &parameters)?;
                if let Value::File(file) = source {
                    file.borrow_mut().1 = start + used;
                }
                self.stack
                    .push(Value::File(Rc::new(RefCell::new((data, 0))).into()));
            }
            "read" => {
                let Value::File(file) = self.pop()? else {
                    return Err("typecheck: read".into());
                };
                self.vm.read(&Value::File(file.clone()))?;
                let mut file = file.borrow_mut();
                let value = file.0.get(file.1).copied();
                if let Some(value) = value {
                    file.1 += 1;
                    self.stack.push(Value::Number(value as f64));
                }
                self.stack.push(Value::Bool(value.is_some()));
            }
            "readstring" | "readhexstring" => {
                let Value::String(target) = self.pop()? else {
                    return Err("typecheck: readstring".into());
                };
                let Value::File(file) = self.pop()? else {
                    return Err("typecheck: readstring file".into());
                };
                target.write()?;
                self.vm.read(&Value::File(file.clone()))?;
                let length = target.borrow().len();
                let mut data = Vec::new();
                let mut file = file.borrow_mut();
                while data.len() < length && file.1 < file.0.len() {
                    if op == "readstring" {
                        data.push(file.0[file.1]);
                        file.1 += 1;
                    } else {
                        let mut digits = Vec::new();
                        while digits.len() < 2 && file.1 < file.0.len() {
                            let b = file.0[file.1];
                            file.1 += 1;
                            if b.is_ascii_whitespace() {
                                continue;
                            }
                            digits.push(
                                (b as char).to_digit(16).ok_or("ioerror: readhexstring")? as u8
                            );
                        }
                        if digits.is_empty() {
                            break;
                        }
                        data.push(digits[0] * 16 + digits.get(1).copied().unwrap_or(0));
                    }
                }
                target.borrow_mut()[..data.len()].copy_from_slice(&data);
                let complete = data.len() == length;
                self.stack.push(if complete {
                    Value::String(target)
                } else {
                    Value::String(Rc::new(RefCell::new(data)).into())
                });
                self.stack.push(Value::Bool(complete));
            }
            "closefile" | "bytesavailable" | "resetfile" | "fileposition" | "setfileposition" => {
                let position = if op == "setfileposition" {
                    Some(self.num()? as usize)
                } else {
                    None
                };
                let Value::File(file) = self.pop()? else {
                    return Err("typecheck: file".into());
                };
                let mut file = file.borrow_mut();
                match op {
                    "closefile" => file.1 = file.0.len(),
                    "resetfile" => file.1 = 0,
                    "setfileposition" => file.1 = position.unwrap().min(file.0.len()),
                    "fileposition" => self.stack.push(Value::Number(file.1 as f64)),
                    _ => self
                        .stack
                        .push(Value::Number((file.0.len() - file.1) as f64)),
                }
            }
            "image" | "imagemask" if matches!(self.stack.last(), Some(Value::Dict(_))) => {
                let Value::Dict(dict) = self.pop()? else {
                    unreachable!()
                };
                self.vm.read(&Value::Dict(dict.clone()))?;
                let dict = dict.borrow();
                let get = |key: &str| {
                    dict.get(key)
                        .cloned()
                        .ok_or_else(|| format!("undefined: Image {key}"))
                };
                let kind = dict
                    .get("ImageType")
                    .map(Value::number)
                    .transpose()?
                    .unwrap_or(1.) as i32;
                if matches!(kind, 3 | 4) && op == "image" {
                    let values = dict.clone();
                    drop(dict);
                    self.masked_image(&values)?;
                    return Ok(());
                }
                if kind != 1 {
                    return Err("rangecheck: ImageType".into());
                }
                let width = get("Width")?.number()? as usize;
                let height = get("Height")?.number()? as usize;
                let bits = if op == "imagemask" {
                    1
                } else {
                    get("BitsPerComponent")?.number()? as usize
                };
                let matrix = Self::matrix_value(&get("ImageMatrix")?)?;
                let data = get("DataSource")?;
                let sources = if let Value::Array(array) = data {
                    array.read()?;
                    array.borrow().to_vec()
                } else {
                    vec![data]
                };
                let decoded = if let Some(Value::Array(a)) = dict.get("Decode") {
                    a.read()?;
                    Some(
                        a.borrow()
                            .iter()
                            .map(Value::number)
                            .collect::<Result<Vec<_>, _>>()?,
                    )
                } else {
                    None
                };
                let mask = if op == "imagemask" {
                    Some(
                        decoded
                            .as_ref()
                            .and_then(|v| v.first())
                            .copied()
                            .unwrap_or(0.)
                            != 0.,
                    )
                } else {
                    None
                };
                let interpolate = dict
                    .get("Interpolate")
                    .map(Value::boolean)
                    .transpose()?
                    .unwrap_or(false);
                drop(dict);
                self.emit_image(
                    width,
                    height,
                    bits,
                    matrix,
                    sources,
                    if mask.is_some() {
                        1
                    } else {
                        self.graphics.image_components
                    },
                    mask,
                    if mask.is_some() { None } else { decoded },
                    interpolate,
                    true,
                )?;
            }
            "image" | "imagemask" | "colorimage" => {
                let components = if op == "colorimage" {
                    self.num()? as usize
                } else {
                    1
                };
                if !matches!(components, 1 | 3 | 4) {
                    return Err("rangecheck: colorimage components".into());
                }
                let multiple = if op == "colorimage" {
                    self.pop()?.boolean()?
                } else {
                    false
                };
                let count = if multiple { components } else { 1 };
                let mut sources = Vec::new();
                for _ in 0..count {
                    sources.push(self.pop()?);
                }
                sources.reverse();
                let matrix = Self::matrix_value(&self.pop()?)?;
                let (bits, mask) = if op == "imagemask" {
                    (1, Some(self.pop()?.boolean()?))
                } else {
                    (self.num()? as usize, None)
                };
                let height = self.num()? as usize;
                let width = self.num()? as usize;
                self.emit_image(
                    width, height, bits, matrix, sources, components, mask, None, false, false,
                )?;
            }
            _ => unreachable!(),
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn real_filters() {
        assert_eq!(decode(b"61 62 6>", "ASCIIHexDecode").unwrap().0, b"ab`");
        assert_eq!(
            decode(b"87cURD]j7BEbo80~>", "ASCII85Decode").unwrap().0,
            b"Hello world!"
        );
        assert_eq!(
            decode(&[2, b'a', b'b', b'c', 254, b'd', 128], "RunLengthDecode")
                .unwrap()
                .0,
            b"abcddd"
        );
    }
    #[test]
    fn pixels_and_currentfile() {
        let mut i = Interpreter::new();
        let out=i.special("pst: 20 20 scale 2 1 8 [2 0 0 -1 0 1] {currentfile 6 string readhexstring pop} false 3 colorimage FF00000000FF",0.,0.,|_|None).unwrap().unwrap();
        assert!(out.contains("FF00000000FF"));
        assert!(out.contains("/DeviceRGB"));
        assert!(out.contains("20.000000 0.000000 0.000000 20.000000"));
    }
}

#[cfg(test)]
mod regression_tests {
    use super::*;
    #[test]
    fn shared_file_planes_follow_scanline_order(){
        let mut i=Interpreter::new();let output=i.special("pst: /f (FF000000FF00>) /ASCIIHexDecode filter def 1 2 8 [1 0 0 2 0 0] {f 1 string readstring pop} {f 1 string readstring pop} {f 1 string readstring pop} true 3 colorimage",0.,0.,|_|None).unwrap().unwrap();assert!(output.contains("ID\nFF000000FF00>"),"{output}");
    }
    #[test]
    fn alpha_and_twelve_bit_samples_reach_real_pdf_output(){
        let mut i=Interpreter::new();let output=i.special("pst: .25 .setopacityalpha 1 1 12 [1 0 0 1 0 0] <FFF0> image",0.,0.,|_|None).unwrap().unwrap();assert!(output.contains("/PSTalpha0 gs"));assert!(output.contains("/BPC 16"));assert!(output.contains("ID\nFFFF>"));assert_eq!(i.opacity_resources(),vec![("PSTalpha0".into(),0.25,0.25)]);
    }
    #[test]
    fn mask_objects_and_color_key_ranges_are_not_presence_stubs(){
        let mut i=Interpreter::new();i.special("pst: /DeviceRGB setcolorspace << /ImageType 3 /InterleaveType 1 /DataDict << /ImageType 1 /Width 1 /Height 1 /BitsPerComponent 8 /ImageMatrix [1 0 0 1 0 0] /Decode [0 1 0 1 0 1] /DataSource <FF123456> >> /MaskDict << /ImageType 1 /Width 1 /Height 1 /BitsPerComponent 8 /ImageMatrix [1 0 0 1 0 0] /Decode [0 1] >> >> image",0.,0.,|_|None).unwrap();let image=i.shadings.last().unwrap();assert!(image.objects[image.root].dictionary.contains("/Mask @@"));assert_eq!(image.objects[image.root].data.as_deref(),Some(&[0x12,0x34,0x56][..]));let mask=image.objects.iter().find(|obj|obj.dictionary.contains("/ImageMask true")).unwrap();assert_eq!(mask.data.as_deref(),Some(&[0x80][..]));
    }
}
