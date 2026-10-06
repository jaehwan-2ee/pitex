// Original Pitex color-space evaluation, from Adobe PLRM 4.8 and ISO 32000.
// Copyright (c) 2026 Pitex contributors. SPDX-License-Identifier: AGPL-3.0-or-later
use super::*;
#[derive(Clone)]
pub(super) enum Space {
    Gray,
    Rgb,
    Cmyk,
    Indexed {
        base: Box<Space>,
        high: usize,
        table: Vec<f64>,
    },
    Cie {
        kind: String,
        parameters: BTreeMap<String, Value>,
        count: usize,
        ranges: Vec<f64>,
    },
    Tint {
        names: Vec<String>,
        base: Box<Space>,
        procedure: Value,
    },
    Cal {
        count: usize,
        white: Vec<f64>,
        black: Vec<f64>,
        gamma: Vec<f64>,
        matrix: Vec<f64>,
    },
}
pub(super) fn numbers(value: &Value) -> Result<Vec<f64>, String> {
    match value {
        Value::Array(a) | Value::Proc(a) => {
            a.read()?;
            a.borrow().iter().map(Value::number).collect()
        }
        _ => Err("typecheck: color parameter array".into()),
    }
}
fn parameter(
    dict: &BTreeMap<String, Value>,
    key: &str,
    default: &[f64],
) -> Result<Vec<f64>, String> {
    let value = dict
        .get(key)
        .map(numbers)
        .transpose()?
        .unwrap_or_else(|| default.to_vec());
    if value.len() != default.len() || value.iter().any(|v| !v.is_finite()) {
        return Err(format!("rangecheck: {key}"));
    }
    Ok(value)
}
fn vector(dict: &BTreeMap<String, Value>, key: &str) -> Result<Vec<Value>, String> {
    match dict.get(key) {
        Some(Value::Array(a)) => {
            a.read()?;
            Ok(a.borrow().to_vec())
        }
        None => Ok(vec![]),
        _ => Err(format!("typecheck: {key}")),
    }
}
fn product(matrix: &[f64], values: &[f64]) -> Vec<f64> {
    (0..3)
        .map(|row| {
            (0..values.len())
                .map(|col| matrix[col * 3 + row] * values[col])
                .sum()
        })
        .collect()
}
fn list(values: &[f64]) -> String {
    format!(
        "[{}]",
        values
            .iter()
            .map(|v| format!("{v:.10}"))
            .collect::<Vec<_>>()
            .join(" ")
    )
}
fn pdf_name(value: &str) -> String {
    let mut out = String::from("/");
    for byte in value.bytes() {
        if byte <= 32 || byte >= 127 || b"#/%()<>[]{}".contains(&byte) {
            out += &format!("#{byte:02X}");
        } else {
            out.push(byte as char);
        }
    }
    out
}
fn range(count: usize) -> Vec<f64> {
    (0..count).flat_map(|_| [0., 1.]).collect()
}
impl Space {
    pub fn components(&self) -> usize {
        match self {
            Self::Gray => 1,
            Self::Rgb => 3,
            Self::Cmyk => 4,
            Self::Indexed { .. } => 1,
            Self::Cie { count, .. } | Self::Cal { count, .. } => *count,
            Self::Tint { names, .. } => names.len(),
        }
    }
    pub fn ranges(&self) -> Vec<f64> {
        match self {
            Self::Indexed { high, .. } => vec![0., *high as f64],
            Self::Cie { ranges, .. } => ranges.clone(),
            _ => range(self.components()),
        }
    }
    pub fn initial(&self) -> Vec<f64> {
        if matches!(self, Self::Cmyk) {
            vec![0., 0., 0., 1.]
        } else if matches!(self, Self::Tint { .. }) {
            vec![1.; self.components()]
        } else {
            self.ranges()
                .chunks(2)
                .map(|r| 0f64.clamp(r[0], r[1]))
                .collect()
        }
    }
    pub fn clip(&self, values: &[f64]) -> Vec<f64> {
        values
            .iter()
            .zip(self.ranges().chunks(2))
            .map(|(v, r)| {
                if matches!(self, Self::Indexed { .. }) {
                    v.round().clamp(r[0], r[1])
                } else {
                    v.clamp(r[0], r[1])
                }
            })
            .collect()
    }
    pub fn normalized(&self, values: &[f64]) -> Vec<f64> {
        let values = self.clip(values);
        match self {
            Self::Indexed { high, .. } if *high > 255 => {
                values.iter().map(|v| v / (*high as f64)).collect()
            }
            Self::Cie { ranges, .. } => values
                .iter()
                .zip(ranges.chunks(2))
                .map(|(v, r)| {
                    if r[1] == r[0] {
                        0.
                    } else {
                        (v - r[0]) / (r[1] - r[0])
                    }
                })
                .collect(),
            _ => values,
        }
    }
    pub fn decode(&self, values: Option<Vec<f64>>, bits: usize) -> Result<Vec<f64>, String> {
        let default = match self {
            Self::Indexed { .. } => vec![0., ((1usize << bits) - 1) as f64],
            _ => self.ranges(),
        };
        let values = values.unwrap_or(default);
        if values.len() != self.components() * 2 {
            return Err("rangecheck: image Decode".into());
        }
        Ok(match self {
            Self::Indexed { high, .. } if *high > 255 => {
                values.iter().map(|v| v / (*high as f64)).collect()
            }
            Self::Cie { ranges, .. } => values
                .iter()
                .enumerate()
                .map(|(i, v)| {
                    (v - ranges[(i / 2) * 2]) / (ranges[(i / 2) * 2 + 1] - ranges[(i / 2) * 2])
                })
                .collect(),
            _ => values,
        })
    }
}
impl Interpreter {
    fn color_call(
        &mut self,
        procedure: &Value,
        input: &[f64],
        outputs: usize,
    ) -> Result<Vec<f64>, String> {
        let previous = std::mem::take(&mut self.stack);
        self.stack.extend(input.iter().map(|v| Value::Number(*v)));
        let result = (|| {
            self.execute(procedure.clone())?;
            if self.stack.len() < outputs {
                return Err("rangecheck: color procedure result".into());
            }
            self.stack[self.stack.len() - outputs..]
                .iter()
                .map(Value::number)
                .collect()
        })();
        self.stack = previous;
        result
    }
    pub(super) fn color_space(&mut self, value: &Value) -> Result<Space, String> {
        self.vm.read(value)?;
        let values = match value {
            Value::Array(a) => a.borrow().to_vec(),
            Value::Name(_) | Value::Word(_) => vec![value.clone()],
            _ => return Err("typecheck: ColorSpace".into()),
        };
        let name = values.first().ok_or("rangecheck: ColorSpace")?.name()?;
        Ok(match name.as_str() {
            "DeviceGray" => Space::Gray,
            "DeviceRGB" => Space::Rgb,
            "DeviceCMYK" => Space::Cmyk,
            "Indexed" => {
                if values.len() != 4 {
                    return Err("rangecheck: Indexed".into());
                }
                let base = self.color_space(&values[1])?;
                let high = values[2].number()? as usize;
                if high > 4095 {
                    return Err("rangecheck: Indexed high value".into());
                }
                let n = base.components();
                let count = (high + 1) * n;
                let table = match &values[3] {
                    Value::String(s) => {
                        s.read()?;
                        if s.borrow().len() < count {
                            return Err("rangecheck: Indexed lookup".into());
                        }
                        let ranges = base.ranges();
                        s.borrow()[..count]
                            .iter()
                            .enumerate()
                            .map(|(i, b)| {
                                ranges[2 * (i % n)]
                                    + *b as f64 / 255.
                                        * (ranges[2 * (i % n) + 1] - ranges[2 * (i % n)])
                            })
                            .collect()
                    }
                    lookup => {
                        let mut result = Vec::new();
                        for index in 0..=high {
                            result.extend(self.color_call(lookup, &[index as f64], n)?);
                        }
                        result
                    }
                };
                Space::Indexed {
                    base: Box::new(base),
                    high,
                    table,
                }
            }
            "Separation" | "DeviceN" => {
                if values.len() < 4 {
                    return Err("rangecheck: tint ColorSpace".into());
                }
                let names = if name == "Separation" {
                    vec![values[1].name()?]
                } else {
                    let Value::Array(names) = &values[1] else {
                        return Err("typecheck: DeviceN names".into());
                    };
                    names.read()?;
                    names
                        .borrow()
                        .iter()
                        .map(Value::name)
                        .collect::<Result<Vec<_>, _>>()?
                };
                if names.is_empty() || names.len() > 32 {
                    return Err("rangecheck: DeviceN components".into());
                }
                Space::Tint {
                    names,
                    base: Box::new(self.color_space(&values[2])?),
                    procedure: values[3].clone(),
                }
            }
            "CIEBasedA" | "CIEBasedABC" | "CIEBasedDEF" | "CIEBasedDEFG" | "CalGray" | "CalRGB" => {
                let Value::Dict(dict) = values.get(1).ok_or("rangecheck: color parameters")? else {
                    return Err("typecheck: color parameters".into());
                };
                self.vm.read(&Value::Dict(dict.clone()))?;
                let dict = dict.borrow().clone();
                if !dict.contains_key("WhitePoint") {
                    return Err("undefined: WhitePoint".into());
                }
                let white = parameter(&dict, "WhitePoint", &[0.9505, 1., 1.089])?;
                if white[0] <= 0. || white[1] != 1. || white[2] <= 0. {
                    return Err("rangecheck: WhitePoint".into());
                }
                if name.starts_with("Cal") {
                    let count = if name == "CalGray" { 1 } else { 3 };
                    let gamma = if count == 1 {
                        vec![dict
                            .get("Gamma")
                            .map(Value::number)
                            .transpose()?
                            .unwrap_or(1.)]
                    } else {
                        parameter(&dict, "Gamma", &[1.; 3])?
                    };
                    Space::Cal {
                        count,
                        white,
                        black: parameter(&dict, "BlackPoint", &[0.; 3])?,
                        gamma,
                        matrix: parameter(&dict, "Matrix", &[1., 0., 0., 0., 1., 0., 0., 0., 1.])?,
                    }
                } else {
                    let count = match name.as_str() {
                        "CIEBasedA" => 1,
                        "CIEBasedDEFG" => 4,
                        _ => 3,
                    };
                    let key = match name.as_str() {
                        "CIEBasedA" => "RangeA",
                        "CIEBasedABC" => "RangeABC",
                        "CIEBasedDEF" => "RangeDEF",
                        _ => "RangeDEFG",
                    };
                    let ranges = parameter(&dict, key, &range(count))?;
                    if ranges.chunks(2).any(|r| r[0] >= r[1]) {
                        return Err("rangecheck: CIE component ranges".into());
                    }
                    Space::Cie {
                        kind: name,
                        parameters: dict,
                        count,
                        ranges,
                    }
                }
            }
            _ => return Err(format!("undefined: color space {name}")),
        })
    }
    fn cie_xyz(
        &mut self,
        kind: &str,
        dict: &BTreeMap<String, Value>,
        input: &[f64],
    ) -> Result<Vec<f64>, String> {
        let mut values = input.to_vec();
        if kind == "CIEBasedDEF" || kind == "CIEBasedDEFG" {
            let n = values.len();
            let procedures = vector(dict, if n == 3 { "DecodeDEF" } else { "DecodeDEFG" })?;
            let limits = parameter(
                dict,
                if n == 3 { "RangeHIJ" } else { "RangeHIJK" },
                &range(n),
            )?;
            for (i, value) in values.iter_mut().enumerate() {
                if let Some(proc) = procedures.get(i) {
                    *value = self.color_call(proc, &[*value], 1)?[0];
                }
                *value = value.clamp(limits[i * 2], limits[i * 2 + 1]);
            }
            let table = vector(dict, "Table")?;
            if table.len() != n + 1 {
                return Err("rangecheck: CIE Table".into());
            }
            let sizes = table[..n]
                .iter()
                .map(|v| Ok(v.number()? as usize))
                .collect::<Result<Vec<_>, String>>()?;
            if sizes.iter().any(|v| *v < 2) {
                return Err("rangecheck: CIE table size".into());
            }
            fn flatten(value: &Value, out: &mut Vec<u8>) -> Result<(), String> {
                match value {
                    Value::String(s) => {
                        s.read()?;
                        out.extend(s.borrow().iter());
                        Ok(())
                    }
                    Value::Array(a) => {
                        a.read()?;
                        for item in a.borrow().iter() {
                            flatten(item, out)?;
                        }
                        Ok(())
                    }
                    _ => Err("typecheck: CIE table data".into()),
                }
            }
            let mut bytes = Vec::new();
            flatten(&table[n], &mut bytes)?;
            let size = sizes
                .iter()
                .try_fold(3usize, |a, b| a.checked_mul(*b))
                .ok_or("limitcheck: CIE Table")?;
            if bytes.len() != size {
                return Err("rangecheck: CIE table data".into());
            }
            let position = values
                .iter()
                .enumerate()
                .map(|(i, v)| {
                    (v - limits[2 * i]) / (limits[2 * i + 1] - limits[2 * i])
                        * (sizes[i] - 1) as f64
                })
                .collect::<Vec<_>>();
            let mut abc = vec![0.; 3];
            for corner in 0..(1usize << n) {
                let (mut weight, mut index) = (1., 0usize);
                for axis in 0..n {
                    let lo = position[axis].floor() as usize;
                    let upper = (corner >> axis) & 1;
                    let coordinate = (lo + upper).min(sizes[axis] - 1);
                    weight *= if upper == 0 {
                        1. - position[axis].fract()
                    } else {
                        position[axis].fract()
                    };
                    index = index * sizes[axis] + coordinate;
                }
                for c in 0..3 {
                    abc[c] += weight * bytes[index * 3 + c] as f64 / 255.;
                }
            }
            let limits = parameter(dict, "RangeABC", &range(3))?;
            values = abc
                .iter()
                .enumerate()
                .map(|(i, v)| limits[2 * i] + v * (limits[2 * i + 1] - limits[2 * i]))
                .collect();
        }
        let mut lmn = if kind == "CIEBasedA" {
            let a = if let Some(proc) = dict.get("DecodeA") {
                self.color_call(proc, &values, 1)?[0]
            } else {
                values[0]
            };
            parameter(dict, "MatrixA", &[1.; 3])?
                .iter()
                .map(|v| v * a)
                .collect::<Vec<_>>()
        } else {
            let procedures = vector(dict, "DecodeABC")?;
            for (i, value) in values.iter_mut().enumerate() {
                if let Some(proc) = procedures.get(i) {
                    *value = self.color_call(proc, &[*value], 1)?[0];
                }
            }
            product(
                &parameter(dict, "MatrixABC", &[1., 0., 0., 0., 1., 0., 0., 0., 1.])?,
                &values,
            )
        };
        let ranges = parameter(dict, "RangeLMN", &range(3))?;
        let procedures = vector(dict, "DecodeLMN")?;
        for (i, value) in lmn.iter_mut().enumerate() {
            *value = value.clamp(ranges[2 * i], ranges[2 * i + 1]);
            if let Some(proc) = procedures.get(i) {
                *value = self.color_call(proc, &[*value], 1)?[0];
            }
        }
        Ok(product(
            &parameter(dict, "MatrixLMN", &[1., 0., 0., 0., 1., 0., 0., 0., 1.])?,
            &lmn,
        ))
    }
    fn space_values(&mut self, space: &Space, input: &[f64]) -> Result<(Space, Vec<f64>), String> {
        let values = space.clip(input);
        match space {
            Space::Indexed { base, high, table } => {
                let index = values[0].round().clamp(0., *high as f64) as usize;
                let n = base.components();
                self.space_values(base, &table[index * n..(index + 1) * n])
            }
            Space::Tint {
                base, procedure, ..
            } => {
                let values = self.color_call(procedure, &values, base.components())?;
                self.space_values(base, &values)
            }
            Space::Cie {
                kind, parameters, ..
            } => {
                let xyz = self.cie_xyz(kind, parameters, &values)?;
                let white = parameter(parameters, "WhitePoint", &[0.9505, 1., 1.089])?;
                Ok((
                    Space::Cal {
                        count: 3,
                        white,
                        black: parameter(parameters, "BlackPoint", &[0.; 3])?,
                        gamma: vec![1.; 3],
                        matrix: vec![4., 0., 0., 0., 4., 0., 0., 0., 4.],
                    },
                    xyz.iter().map(|v| v / 4.).collect(),
                ))
            }
            _ => Ok((space.clone(), values)),
        }
    }
    pub(super) fn color_pdf(
        &mut self,
        space: &Space,
        objects: &mut Vec<shading::Object>,
    ) -> Result<String, String> {
        Ok(match space {
            Space::Gray => "/DeviceGray".into(),
            Space::Rgb => "/DeviceRGB".into(),
            Space::Cmyk => "/DeviceCMYK".into(),
            Space::Cal {
                count,
                white,
                black,
                gamma,
                matrix,
            } => format!(
                "[/{} << /WhitePoint {} /BlackPoint {} /Gamma {} {} >>]",
                if *count == 1 { "CalGray" } else { "CalRGB" },
                list(white),
                list(black),
                if *count == 1 {
                    gamma[0].to_string()
                } else {
                    list(gamma)
                },
                if *count == 3 {
                    format!("/Matrix {}", list(matrix))
                } else {
                    String::new()
                }
            ),
            Space::Indexed { base, high, table } => {
                let base_pdf = self.color_pdf(base, objects)?;
                if *high > 255 {
                    let n = base.components();
                    let mut functions = Vec::new();
                    for values in table.chunks(n) {
                        let normalized = base.normalized(values);
                        let id = objects.len();
                        objects.push(shading::Object {
                            dictionary: format!(
                                "/FunctionType 2 /Domain [0 1] /C0 {} /C1 {} /N 1",
                                list(&normalized),
                                list(&normalized)
                            ),
                            data: None,
                        });
                        functions.push(format!("@@{id}@@"));
                    }
                    let id = objects.len();
                    objects.push(shading::Object{dictionary:format!("/FunctionType 3 /Domain [0 1] /Functions [{}] /Bounds [{}] /Encode [{}]",functions.join(" "),(0..*high).map(|i|((i as f64+0.5)/(*high as f64)).to_string()).collect::<Vec<_>>().join(" "),vec!["0 1";*high+1].join(" ")),data:None});
                    return Ok(format!("[/Separation /IndexedEntry {base_pdf} @@{id}@@]"));
                }
                let ranges = base.ranges();
                let n = base.components();
                let bytes = table
                    .iter()
                    .enumerate()
                    .map(|(i, v)| {
                        ((v - ranges[2 * (i % n)])
                            / (ranges[2 * (i % n) + 1] - ranges[2 * (i % n)])
                            * 255.)
                            .round()
                            .clamp(0., 255.) as u8
                    })
                    .map(|b| format!("{b:02X}"))
                    .collect::<String>();
                format!("[/Indexed {base_pdf} {high} <{bytes}>]")
            }
            Space::Cie { .. } | Space::Tint { .. } => {
                let count = space.components();
                if count > 16 {
                    return Err("limitcheck: sampled tint dimensions".into());
                }
                let steps: usize = match count {
                    1 => 257,
                    2 => 33,
                    3 => 17,
                    4 => 9,
                    _ => 2,
                };
                let mut samples = Vec::new();
                let limits = space.ranges();
                let alternate = match space {
                    Space::Tint { base, .. } => base.as_ref().clone(),
                    Space::Cie { .. } => Space::Rgb,
                    _ => unreachable!(),
                };
                let n = alternate.components();
                for index in 0..steps.pow(count as u32) {
                    let mut remainder = index;
                    let mut values = Vec::new();
                    for axis in 0..count {
                        let position = remainder % steps;
                        remainder /= steps;
                        values.push(
                            limits[axis * 2]
                                + position as f64 / (steps - 1) as f64
                                    * (limits[axis * 2 + 1] - limits[axis * 2]),
                        );
                    }
                    let decoded = match space {
                        Space::Tint { procedure, .. } => self.color_call(procedure, &values, n)?,
                        Space::Cie { .. } => {
                            let (physical, values) = self.space_values(space, &values)?;
                            self.physical_rgb(&physical, &values)?
                        }
                        _ => unreachable!(),
                    };
                    let normalized = alternate.normalized(&decoded);
                    for value in normalized {
                        samples
                            .extend(((value.clamp(0., 1.) * 65535.).round() as u16).to_be_bytes());
                    }
                }
                let alternate_pdf = self.color_pdf(&alternate, objects)?;
                let id = objects.len();
                objects.push(shading::Object{dictionary:format!("/FunctionType 0 /Domain {} /Range {} /Size [{}] /BitsPerSample 16 /Order 1 /Encode [{}] /Decode {}",list(&range(count)),list(&range(n)),vec![steps.to_string();count].join(" "),(0..count).flat_map(|_|["0".to_string(),(steps-1).to_string()]).collect::<Vec<_>>().join(" "),list(&range(n))),data:Some(samples)});
                let names = match space {
                    Space::Tint { names, .. } => {
                        names.iter().map(|s| pdf_name(s)).collect::<Vec<_>>()
                    }
                    _ => (0..count).map(|i| format!("/CIE{}", i + 1)).collect(),
                };
                if names.len() == 1 {
                    format!("[/Separation {} {alternate_pdf} @@{id}@@]", names[0])
                } else {
                    format!("[/DeviceN [{}] {alternate_pdf} @@{id}@@]", names.join(" "))
                }
            }
        })
    }
    pub(super) fn color_resource(&mut self, space: &Space) -> Result<String, String> {
        match space {
            Space::Gray => return Ok("/DeviceGray".into()),
            Space::Rgb => return Ok("/DeviceRGB".into()),
            Space::Cmyk => return Ok("/DeviceCMYK".into()),
            _ => {}
        }
        let mut objects = Vec::new();
        let body = self.color_pdf(space, &mut objects)?;
        let root = objects.len();
        objects.push(shading::Object {
            dictionary: body,
            data: None,
        });
        let name = format!("PSTcolor{}", self.shadings.len());
        self.shadings.push(shading::Shading {
            name: name.clone(),
            objects,
            root,
        });
        Ok(format!("/{name}"))
    }
    pub(super) fn set_selected_color(&mut self, values: Vec<f64>) -> Result<(), String> {
        let space = self.graphics.color_space.clone();
        let values = space.clip(&values);
        let pdf_values = space.normalized(&values);
        let resource = self.graphics.color_resource.clone();
        self.graphics.color_values = values;
        self.graphics.color = format!(
            "{resource} cs {resource} CS {} sc {} SC",
            pdf_values
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(" "),
            pdf_values
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(" ")
        );
        Ok(())
    }
    pub(super) fn current_color_rgb(&mut self) -> Result<Vec<f64>, String> {
        let selected = self.graphics.color_space.clone();
        let input = self.graphics.color_values.clone();
        let (space, values) = self.space_values(&selected, &input)?;
        self.physical_rgb(&space, &values)
    }
    fn physical_rgb(&self, space: &Space, values: &[f64]) -> Result<Vec<f64>, String> {
        Ok(match space {
            Space::Gray => vec![values[0]; 3],
            Space::Rgb => values.to_vec(),
            Space::Cmyk => (0..3)
                .map(|i| 1. - (values[i] + values[3]).min(1.))
                .collect(),
            Space::Cal {
                count,
                white,
                gamma,
                matrix,
                black,
            } => {
                let xyz = if *count == 1 {
                    white
                        .iter()
                        .map(|v| v * values[0].powf(gamma[0]))
                        .collect::<Vec<_>>()
                } else {
                    product(
                        &matrix,
                        &values
                            .iter()
                            .enumerate()
                            .map(|(i, v)| v.powf(gamma[i]))
                            .collect::<Vec<_>>(),
                    )
                };
                let xyz = xyz
                    .iter()
                    .enumerate()
                    .map(|(i, v)| {
                        if black[i] > 0. && white[i] > black[i] {
                            (v - black[i]) * white[i] / (white[i] - black[i])
                        } else {
                            *v
                        }
                    })
                    .collect::<Vec<_>>();
                // CAT02 chromatic adaptation to the ICC D50 PCS, followed by
                // the RGB destination's Bradford D50-to-D65 adaptation.
                let cat02 = [
                    0.7328, -0.7036, 0.003, 0.4296, 1.6975, 0.0136, -0.1624, 0.0061, 0.9834,
                ];
                let cone = product(&cat02, &xyz);
                let source = product(&cat02, &white);
                let target = product(&cat02, &[0.9642, 1., 0.8249]);
                let adapted = cone
                    .iter()
                    .enumerate()
                    .map(|(i, v)| v * target[i] / source[i])
                    .collect::<Vec<_>>();
                let xyz = product(
                    &[
                        1.096123820835514,
                        0.454369041975359,
                        -0.009627608738429,
                        -0.278869000218287,
                        0.473533154307412,
                        -0.005698031216114,
                        0.182745179382773,
                        0.072097803717229,
                        1.015325639954543,
                    ],
                    &adapted,
                );
                let bradford = [
                    0.8951, -0.7502, 0.0389, 0.2664, 1.7135, -0.0685, -0.1614, 0.0367, 1.0296,
                ];
                let cone = product(&bradford, &xyz);
                let source = product(&bradford, &[0.9642, 1., 0.8249]);
                let target = product(&bradford, &[0.95047, 1., 1.08883]);
                let cone = cone
                    .iter()
                    .enumerate()
                    .map(|(i, v)| v * target[i] / source[i])
                    .collect::<Vec<_>>();
                let xyz = product(
                    &[
                        0.9869929, 0.4323053, -0.0085287, -0.1470543, 0.5183603, 0.0400428,
                        0.1599627, 0.0492912, 0.9684867,
                    ],
                    &cone,
                );
                product(
                    &[
                        3.2404542, -0.969266, 0.0556434, -1.5371385, 1.8760108, -0.2040259,
                        -0.4985314, 0.041556, 1.0572252,
                    ],
                    &xyz,
                )
                .iter()
                .map(|v| {
                    if *v <= 0.0031308 {
                        12.92 * v
                    } else {
                        1.055 * v.powf(1. / 2.4) - 0.055
                    }
                })
                .map(|v| v.clamp(0., 1.))
                .collect()
            }
            _ => unreachable!(),
        })
    }
    pub(super) fn select_space(&mut self, value: Value) -> Result<(), String> {
        let space = self.color_space(&value)?;
        let resource = self.color_resource(&space)?;
        let initial = space.initial();
        self.graphics.image_components = space.components();
        self.graphics.color_space = space;
        self.graphics.color_definition = value;
        self.graphics.color_resource = resource;
        self.set_selected_color(initial)
    }
}

#[derive(Clone)]
struct Function {
    domain: Vec<f64>,
    range: Option<Vec<f64>>,
    kind: FunctionKind,
}
#[derive(Clone)]
enum FunctionKind {
    Many(Vec<Function>),
    Power(Vec<f64>, Vec<f64>, f64),
    Sampled {
        sizes: Vec<usize>,
        bits: usize,
        encode: Vec<f64>,
        decode: Vec<f64>,
        data: Vec<u8>,
    },
    Stitched {
        bounds: Vec<f64>,
        encode: Vec<f64>,
        children: Vec<Function>,
    },
    Calculator(Value, usize),
}
impl Interpreter {
    fn color_function(&mut self, value: &Value) -> Result<Function, String> {
        self.vm.read(value)?;
        if let Value::Array(array) = value {
            array.read()?;
            let mut children = Vec::new();
            for child in array.borrow().iter() {
                children.push(self.color_function(child)?);
            }
            let domain = children
                .first()
                .ok_or("rangecheck: function array")?
                .domain
                .clone();
            return Ok(Function {
                domain,
                range: None,
                kind: FunctionKind::Many(children),
            });
        }
        let Value::Dict(dict) = value else {
            return Err("typecheck: color function".into());
        };
        let dict = dict.borrow();
        let get = |key: &str| {
            dict.get(key)
                .ok_or_else(|| format!("undefined: Function {key}"))
        };
        let domain = color::numbers(get("Domain")?)?;
        let range = dict.get("Range").map(color::numbers).transpose()?;
        let kind = get("FunctionType")?.number()? as i32;
        let data = |value: &Value| -> Result<Vec<u8>, String> {
            match value {
                Value::String(bytes) => {
                    bytes.read()?;
                    Ok(bytes.borrow().to_vec())
                }
                Value::File(file) => {
                    self.vm.read(value)?;
                    let file = file.borrow();
                    Ok(file.0[file.1..].to_vec())
                }
                _ => Err("typecheck: Function DataSource".into()),
            }
        };
        let kind = match kind {
            2 => {
                let a = dict
                    .get("C0")
                    .map(color::numbers)
                    .transpose()?
                    .unwrap_or(vec![0.]);
                let b = dict
                    .get("C1")
                    .map(color::numbers)
                    .transpose()?
                    .unwrap_or(vec![1.]);
                if domain.len() != 2 || a.len() != b.len() {
                    return Err("rangecheck: exponential function".into());
                }
                FunctionKind::Power(a, b, get("N")?.number()?)
            }
            3 => {
                let Value::Array(functions) = get("Functions")? else {
                    return Err("typecheck: stitching Functions".into());
                };
                functions.read()?;
                let mut children = Vec::new();
                for child in functions.borrow().iter() {
                    children.push(self.color_function(child)?);
                }
                let bounds = color::numbers(get("Bounds")?)?;
                let encode = color::numbers(get("Encode")?)?;
                if domain.len() != 2
                    || bounds.len() + 1 != children.len()
                    || encode.len() != children.len() * 2
                {
                    return Err("rangecheck: stitching function".into());
                }
                FunctionKind::Stitched {
                    bounds,
                    encode,
                    children,
                }
            }
            0 => {
                let sizes = color::numbers(get("Size")?)?
                    .iter()
                    .map(|v| *v as usize)
                    .collect::<Vec<_>>();
                let bits = get("BitsPerSample")?.number()? as usize;
                let output = range
                    .as_ref()
                    .ok_or("undefined: sampled function Range")?
                    .len()
                    / 2;
                if sizes.is_empty()
                    || sizes.len() * 2 != domain.len()
                    || sizes.iter().any(|v| *v == 0)
                    || !matches!(bits, 1 | 2 | 4 | 8 | 12 | 16 | 24 | 32)
                {
                    return Err("rangecheck: sampled function".into());
                }
                if dict
                    .get("Order")
                    .map(Value::number)
                    .transpose()?
                    .unwrap_or(1.)
                    != 1.
                {
                    return Err(
                        "rangecheck: normalized CIE function supports linear sample interpolation"
                            .into(),
                    );
                }
                let bytes = data(get("DataSource")?)?;
                let samples = sizes
                    .iter()
                    .try_fold(output, |a, b| a.checked_mul(*b))
                    .ok_or("limitcheck: function samples")?;
                if bytes.len() < (samples * bits + 7) / 8 {
                    return Err("rangecheck: function sample data".into());
                }
                let encode = dict
                    .get("Encode")
                    .map(color::numbers)
                    .transpose()?
                    .unwrap_or_else(|| sizes.iter().flat_map(|n| [0., (*n - 1) as f64]).collect());
                let decode = dict
                    .get("Decode")
                    .map(color::numbers)
                    .transpose()?
                    .unwrap_or_else(|| range.clone().unwrap());
                FunctionKind::Sampled {
                    sizes,
                    bits,
                    encode,
                    decode,
                    data: bytes,
                }
            }
            4 => {
                let bytes = data(get("DataSource")?)?;
                let tokens = Reader {
                    bytes: &bytes,
                    pos: 0,
                    one: false,
                }
                .values(None)?;
                let procedure = if tokens.len() == 1 && matches!(tokens[0], Value::Proc(_)) {
                    tokens[0].clone()
                } else {
                    Value::Proc(tokens.into())
                };
                FunctionKind::Calculator(
                    procedure,
                    range.as_ref().ok_or("undefined: calculator Range")?.len() / 2,
                )
            }
            _ => return Err("rangecheck: FunctionType".into()),
        };
        Ok(Function {
            domain,
            range,
            kind,
        })
    }
    fn color_function_value(
        &mut self,
        function: &Function,
        input: &[f64],
    ) -> Result<Vec<f64>, String> {
        if input.len() * 2 != function.domain.len() {
            return Err("rangecheck: function inputs".into());
        }
        let input = input
            .iter()
            .zip(function.domain.chunks(2))
            .map(|(v, r)| v.clamp(r[0], r[1]))
            .collect::<Vec<_>>();
        let result = match &function.kind {
            FunctionKind::Many(children) => {
                let mut values = Vec::new();
                for child in children {
                    values.extend(self.color_function_value(child, &input)?);
                }
                values
            }
            FunctionKind::Power(a, b, n) => a
                .iter()
                .zip(b)
                .map(|(a, b)| a + input[0].powf(*n) * (b - a))
                .collect(),
            FunctionKind::Calculator(procedure, n) => self.color_call(procedure, &input, *n)?,
            FunctionKind::Stitched {
                bounds,
                encode,
                children,
            } => {
                let index = bounds
                    .iter()
                    .position(|b| input[0] < *b)
                    .unwrap_or(bounds.len());
                let lo = if index == 0 {
                    function.domain[0]
                } else {
                    bounds[index - 1]
                };
                let hi = if index == bounds.len() {
                    function.domain[1]
                } else {
                    bounds[index]
                };
                let value = encode[2 * index]
                    + (input[0] - lo) / (hi - lo) * (encode[2 * index + 1] - encode[2 * index]);
                self.color_function_value(&children[index], &[value])?
            }
            FunctionKind::Sampled {
                sizes,
                bits,
                encode,
                decode,
                data,
            } => {
                let n = decode.len() / 2;
                let positions = input
                    .iter()
                    .enumerate()
                    .map(|(i, v)| {
                        (encode[2 * i]
                            + (v - function.domain[2 * i])
                                / (function.domain[2 * i + 1] - function.domain[2 * i])
                                * (encode[2 * i + 1] - encode[2 * i]))
                            .clamp(0., (sizes[i] - 1) as f64)
                    })
                    .collect::<Vec<_>>();
                let mut values = vec![0.; n];
                for corner in 0..(1usize << sizes.len()) {
                    let (mut index, mut stride, mut weight) = (0usize, 1usize, 1.);
                    for axis in 0..sizes.len() {
                        let upper = (corner >> axis) & 1;
                        index += ((positions[axis].floor() as usize + upper).min(sizes[axis] - 1))
                            * stride;
                        stride *= sizes[axis];
                        weight *= if upper == 0 {
                            1. - positions[axis].fract()
                        } else {
                            positions[axis].fract()
                        };
                    }
                    for c in 0..n {
                        let value = images::sample(data, (index * n + c) * bits, *bits) as f64
                            / ((1u64 << bits) - 1) as f64;
                        values[c] +=
                            weight * (decode[2 * c] + value * (decode[2 * c + 1] - decode[2 * c]));
                    }
                }
                values
            }
        };
        Ok(if let Some(range) = &function.range {
            if result.len() * 2 != range.len() {
                return Err("rangecheck: function results".into());
            }
            result
                .iter()
                .zip(range.chunks(2))
                .map(|(v, r)| v.clamp(r[0], r[1]))
                .collect()
        } else {
            result
        })
    }
    pub(super) fn normalized_color_function(
        &mut self,
        value: &Value,
        space: &Space,
        objects: &mut Vec<shading::Object>,
    ) -> Result<Option<String>, String> {
        if !matches!(space, Space::Cie { .. }) || space.ranges() == range(space.components()) {
            return Ok(None);
        }
        if let Value::Dict(dict)=value {
            self.vm.read(value)?;let dict=dict.borrow();
            if dict.get("FunctionType").map(Value::number).transpose()?==Some(0.) {
                let mut transformed=dict.clone();let get=|key:&str|dict.get(key).ok_or_else(||format!("undefined: Function {key}"));
                let original_range=numbers(get("Range")?)?;let original_decode=dict.get("Decode").map(numbers).transpose()?.unwrap_or_else(||original_range.clone());let cie_ranges=space.ranges();
                let normalize=|values:&[f64]|->Result<Vec<f64>,String>{if values.len()!=cie_ranges.len(){return Err("rangecheck: sampled shading color outputs".into());}Ok(values.iter().enumerate().map(|(i,v)|(v-cie_ranges[(i/2)*2])/(cie_ranges[(i/2)*2+1]-cie_ranges[(i/2)*2])).collect())};
                transformed.insert("Range".into(),Value::Array(normalize(&original_range)?.into_iter().map(Value::Number).collect::<Vec<_>>().into()));
                transformed.insert("Decode".into(),Value::Array(normalize(&original_decode)?.into_iter().map(Value::Number).collect::<Vec<_>>().into()));
                // Affine output conversion commutes with sample interpolation and
                // clipping. Preserve the original bytes, bit depth and Order,
                // including cubic interpolation, for the native PDF consumer.
                let value=Value::Dict(Rc::new(RefCell::new(transformed)));
                return Ok(Some(shading::function(&value,objects,&self.vm)?));
            }
        }
        let function = self.color_function(value)?;
        let count = function.domain.len() / 2;
        if count == 0 || count > 2 {
            return Err("rangecheck: shading function dimensions".into());
        }
        let size: usize = if count == 1 { 257 } else { 33 };
        let mut data = Vec::new();
        for index in 0..size.pow(count as u32) {
            let mut rest = index;
            let input = (0..count)
                .map(|i| {
                    let position = rest % size;
                    rest /= size;
                    function.domain[2 * i]
                        + position as f64 / (size - 1) as f64
                            * (function.domain[2 * i + 1] - function.domain[2 * i])
                })
                .collect::<Vec<_>>();
            let values = self.color_function_value(&function, &input)?;
            if values.len() != space.components() {
                return Err("rangecheck: shading color components".into());
            }
            for value in space.normalized(&values) {
                data.extend(((value.clamp(0., 1.) * 65535.).round() as u16).to_be_bytes());
            }
        }
        let id = objects.len();
        objects.push(shading::Object{dictionary:format!("/FunctionType 0 /Domain {} /Range {} /Size [{}] /BitsPerSample 16 /Order 1 /Encode [{}] /Decode {}",list(&function.domain),list(&range(space.components())),vec![size.to_string();count].join(" "),(0..count).flat_map(|_|["0".to_string(),(size-1).to_string()]).collect::<Vec<_>>().join(" "),list(&range(space.components()))),data:Some(data)});
        Ok(Some(format!("@@{id}@@")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn current_colorspace_resets_and_dictionary_first_value(){
        let mut i=Interpreter::new();i.budget=100000;
        i.execute_bytes(b"1 0 0 setrgbcolor /DeviceGray setcolorspace currentcolor << /Audit 1 /Audit 2 >> /Audit get /DeviceCMYK setcolorspace currentcolor").unwrap();
        let values=i.stack.iter().map(|v|v.number().unwrap()).collect::<Vec<_>>();assert_eq!(values,vec![0.,1.,0.,0.,0.,1.]);
        let output=std::process::Command::new("gs").args(["-q","-dNODISPLAY","-dBATCH","-c","1 0 0 setrgbcolor /DeviceGray setcolorspace currentcolor == << /Audit 1 /Audit 2 >> /Audit get == /DeviceCMYK setcolorspace currentcolor 4 array astore =="]).output().unwrap();assert!(output.status.success());let text=String::from_utf8(output.stdout).unwrap();assert!(text.starts_with("0.0\n1\n[0.0 0.0 0.0 1.0]"),"{text}");
    }
    #[test]
    fn full_cie_pipeline_preserves_clipping_and_nonlinear_stages(){
        let mut i=Interpreter::new();i.budget=100000;i.execute_bytes(b"<< /DecodeABC [{dup mul} {} {}] /MatrixABC [.5 0 .5 0 1 0 .5 0 .5] /RangeLMN [0 .4 0 1 0 1] /DecodeLMN [{dup mul} {} {}] >>").unwrap();let Value::Dict(parameters)=i.pop().unwrap()else{panic!()};let parameters=parameters.borrow().clone();let value=i.cie_xyz("CIEBasedABC",&parameters,&[0.5,0.25,0.75]).unwrap();for(a,b)in value.iter().zip([0.16,0.25,0.5]){assert!((a-b).abs()<1e-12);}
    }
    #[test]
    fn cubic_nonunit_cie_normalization_preserves_native_samples(){
        let mut i=Interpreter::new();i.budget=100000;i.execute_bytes(b"<< /FunctionType 0 /Domain [0 1] /Range [20 80] /Size [4] /BitsPerSample 8 /Order 3 /Encode [0 3] /Decode [-20 120] /DataSource <0055AAFF> >>").unwrap();let value=i.pop().unwrap();let space=Space::Cie{kind:"CIEBasedA".into(),parameters:BTreeMap::new(),count:1,ranges:vec![0.,100.]};let mut objects=Vec::new();i.normalized_color_function(&value,&space,&mut objects).unwrap();let object=objects.last().unwrap();assert_eq!(object.data.as_deref(),Some(&[0,0x55,0xaa,0xff][..]));assert!(object.dictionary.contains("/Order 3"));assert!(object.dictionary.contains("/BitsPerSample 8"));assert!(object.dictionary.contains("/Range [0.2 0.8]"),"{}",object.dictionary);assert!(object.dictionary.contains("/Decode [-0.2 1.2]"),"{}",object.dictionary);
    }

}
