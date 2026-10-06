// Pitex-authored native backend policy. Copyright (c) 2026 Pitex contributors.
// SPDX-License-Identifier: AGPL-3.0-or-later
use super::*;
use std::collections::BTreeMap;

#[derive(Clone)]
pub struct Options {
    pub tokens: BTreeMap<String, String>,
    pub values: BTreeMap<String, i32>,
    pub glyph_unicode: BTreeMap<String, String>,
    pub requested_version: i32,
    pub builtin_glyph_unicode: BTreeMap<String, String>,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            tokens: BTreeMap::new(),
            values: crate::backend_definitions::PARAMETERS
                .iter()
                .filter(|p| p.storage != crate::backend_definitions::Storage::Tokens)
                .map(|p| (p.name.to_string(), p.initial))
                .collect(),
            glyph_unicode: BTreeMap::new(),
            builtin_glyph_unicode: BTreeMap::new(),
            requested_version: 17,
        }
    }
}
impl Options {
    pub fn integer(&self, name: &str) -> i32 {
        self.values.get(name).copied().unwrap_or(0)
    }
    pub fn dimension(&self, name: &str) -> f64 {
        self.integer(name) as f64 * 72. / 72.27 / 65536.
    }
    pub fn image_key(&self) -> u64 {
        let mut hash = 0xcbf29ce484222325u64;
        for name in [
            "pdfgamma",
            "pdfimageapplygamma",
            "pdfimagegamma",
            "pdfimagehicolor",
            "pdfimageresolution",
        ] {
            for byte in self.integer(name).to_le_bytes() {
                hash ^= byte as u64;
                hash = hash.wrapping_mul(0x100000001b3)
            }
        }
        hash ^ (self.requested_version as u64)
    }
    pub fn info_policy(&self) -> crate::pdf_metadata::MetadataPolicy {
        crate::pdf_metadata::MetadataPolicy {
            omit_info_dictionary: self.integer("pdfomitinfodict") != 0,
            omit_automatic_dates: self.integer("pdfinfoomitdate") != 0,
            suppress_ptex: self.integer("pdfsuppressptexinfo"),
            use_ptex_underscore: self.integer("pdfuseptexunderscore"),
            pdf_major_version: self.requested_version / 10,
        }
    }
}
pub fn coordinate(value: f64, digits: i32) -> String {
    let value = if value.is_finite() { value } else { 0. };
    let precision = digits.clamp(0, 4) as usize;
    let mut result = format!("{value:.precision$}");
    if result.contains('.') {
        while result.ends_with('0') {
            result.pop();
        }
        if result.ends_with('.') {
            result.pop();
        }
    }
    if result == "-0" {
        result = "0".into()
    }
    result
}
pub unsafe fn emit_coordinate(c: *mut conv_ctx, out: *mut pbuf, value: f64) {
    let text = coordinate(value, (*(*c).document).backend.integer("pdfdecimaldigits"));
    pbuf_append(out, text.as_ptr().cast(), text.len());
    pbuf_putc(out, b' ' as i32);
}
pub unsafe fn configure(c: *mut conv_ctx, values: &BTreeMap<String, String>) {
    let options = &mut (*(*c).document).backend;
    for (name, value) in values {
        if let Ok(value) = value.parse::<i32>() {
            if options.values.contains_key(name) {
                options.values.insert(name.clone(), value);
            }
        }
    }
    // Codec options belong to this conversion, not a process-wide persistent
    // cache. A key also distinguishes already decoded images across edits.
    crate::driver_images::pitex_image_configuration(
        options.integer("pdfimageapplygamma"),
        options.integer("pdfgamma"),
        options.integer("pdfimagegamma"),
        if options.requested_version >= 15 {
            options.integer("pdfimagehicolor")
        } else {
            0
        },
        options.integer("pdfimageresolution"),
    );
}
pub unsafe fn load_builtin_mappings(c: *mut conv_ctx) {
    let file = crate::xetex_common_texlive_provider::texlive_file_path(
        b"glyphtounicode.tex\0".as_ptr().cast(),
        std::ptr::null_mut(),
    );
    if file.is_null() {
        return;
    }
    let Ok(source) =
        std::fs::read_to_string(std::ffi::CStr::from_ptr(file).to_string_lossy().as_ref())
    else {
        return;
    };
    let map = &mut (*(*c).document).backend.builtin_glyph_unicode;
    for line in source.lines() {
        if let Some(tail) = line.trim().strip_prefix("\\pdfglyphtounicode{") {
            if let Some((name, rest)) = tail.split_once("}{") {
                if let Some(end) = rest.find('}') {
                    if let Some(value) = unicode_hex(&rest[..end]) {
                        map.insert(name.into(), value);
                    }
                }
            }
        }
    }
}
pub fn unicode_hex(value: &str) -> Option<String> {
    let pieces = value.split_whitespace().collect::<Vec<_>>();
    if pieces.is_empty() {
        return None;
    }
    let mut result = String::new();
    for piece in pieces {
        if !piece.bytes().all(|b| b.is_ascii_hexdigit()) {
            return None;
        }
        if piece.len() <= 4 {
            result += &format!("{:04X}", u16::from_str_radix(piece, 16).ok()?);
        } else if piece.len() % 4 == 0 {
            result += &piece.to_ascii_uppercase();
        } else {
            let c = char::from_u32(u32::from_str_radix(piece, 16).ok()?)?;
            result += &c
                .encode_utf16(&mut [0; 2])
                .iter()
                .map(|v| format!("{v:04X}"))
                .collect::<String>();
        }
    }
    Some(result)
}
pub fn glyph_unicode(name: &str, options: &Options) -> Option<String> {
    if let Some(value) = options.glyph_unicode.get(name) {
        return Some(value.clone());
    }
    if let Some(value) = options.builtin_glyph_unicode.get(name) {
        return Some(value.clone());
    }
    let base = name.split('.').next()?;
    if let Some(value) = options.glyph_unicode.get(base).or_else(|| options.builtin_glyph_unicode.get(base)) {
        return Some(value.clone());
    }
    if base.contains('_') {
        return base
            .split('_')
            .map(|s| glyph_unicode(s, options))
            .collect::<Option<Vec<_>>>()
            .map(|v| v.join(""));
    }
    if let Some(value) = base.strip_prefix("uni") {
        if value.len() % 4 == 0 && value.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Some(value.to_ascii_uppercase());
        }
    }
    if let Some(value) = base.strip_prefix('u') {
        if let Ok(n) = u32::from_str_radix(value, 16) {
            if let Some(c) = char::from_u32(n) {
                return Some(
                    c.encode_utf16(&mut [0; 2])
                        .iter()
                        .map(|v| format!("{v:04X}"))
                        .collect(),
                );
            }
        }
    }
    let text = if base.len() == 1 && base.is_ascii() {
        base.to_string()
    } else {
        match base {
            "space" => " ",
            "exclam" => "!",
            "quotedbl" => "\"",
            "numbersign" => "#",
            "dollar" => "$",
            "percent" => "%",
            "ampersand" => "&",
            "quotesingle" => "'",
            "parenleft" => "(",
            "parenright" => ")",
            "asterisk" => "*",
            "plus" => "+",
            "comma" => ",",
            "hyphen" | "minus" => "-",
            "period" => ".",
            "slash" => "/",
            "colon" => ":",
            "semicolon" => ";",
            "less" => "<",
            "equal" => "=",
            "greater" => ">",
            "question" => "?",
            "at" => "@",
            "bracketleft" => "[",
            "backslash" => "\\",
            "bracketright" => "]",
            "asciicircum" => "^",
            "underscore" => "_",
            "grave" => "`",
            "braceleft" => "{",
            "bar" => "|",
            "braceright" => "}",
            "asciitilde" => "~",
            "zero" => "0",
            "one" => "1",
            "two" => "2",
            "three" => "3",
            "four" => "4",
            "five" => "5",
            "six" => "6",
            "seven" => "7",
            "eight" => "8",
            "nine" => "9",
            "fi" => "fi",
            "fl" => "fl",
            "ff" => "ff",
            "ffi" => "ffi",
            "ffl" => "ffl",
            "endash" => "–",
            "emdash" => "—",
            "ellipsis" => "…",
            "quoteleft" => "‘",
            "quoteright" => "’",
            "quotedblleft" => "“",
            "quotedblright" => "”",
            _ => return None,
        }
        .to_string()
    };
    Some(text.encode_utf16().map(|v| format!("{v:04X}")).collect())
}
pub unsafe fn type1_unicode(c: *mut conv_ctx, font: *mut pdf_font) -> Option<i32> {
    let options = &(*(*c).document).backend;
    if !font_output::allow_unicode(c,font) {
        return None;
    }
    let mut names = BTreeMap::new();
    if !(*font).enc.is_null() {
        for code in 0..256usize {
            let name = (*(*font).enc).glyph[code];
            if !name.is_null() {
                names.insert(
                    code,
                    std::ffi::CStr::from_ptr(name)
                        .to_string_lossy()
                        .into_owned(),
                );
            }
        }
    } else {
        let data = std::slice::from_raw_parts((*(*font).t1).data, (*(*font).t1).len1);
        let text = String::from_utf8_lossy(data);
        let tokens = text.split_ascii_whitespace().collect::<Vec<_>>();
        for window in tokens.windows(4) {
            if window[0] == "dup" && window[3] == "put" {
                if let Ok(code) = window[1].parse::<usize>() {
                    if code < 256 {
                        if let Some(name) = window[2].strip_prefix('/') {
                            names.insert(code, name.to_string());
                        }
                    }
                }
            }
        }
    }
    let mappings = names
        .iter()
        .filter_map(|(code, name)| {
            if *(*font).used.add(*code) == 0 {
                None
            } else {
                glyph_unicode(name, options).map(|value| (*code, value))
            }
        })
        .collect::<Vec<_>>();
    if mappings.is_empty() {
        return None;
    }
    let object = pdfw_alloc((*c).pw);
    let mut content=String::from("/CIDInit/ProcSet findresource begin 12 dict begin begincmap /CIDSystemInfo<</Registry(Adobe)/Ordering(UCS)/Supplement 0>>def /CMapName/PitexType1Unicode def /CMapType 2 def 1 begincodespacerange <00><FF> endcodespacerange\n");
    for chunk in mappings.chunks(100) {
        content += &format!("{} beginbfchar\n", chunk.len());
        for (code, value) in chunk {
            content += &format!("<{code:02X}><{value}>\n")
        }
        content += "endbfchar\n"
    }
    content += "endcmap CMapName currentdict/CMap defineresource pop end end\n";
    pdfw_stream(
        (*c).pw,
        object,
        std::ptr::null(),
        content.as_ptr().cast(),
        content.len(),
        true,
    );
    Some(object)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn coordinate_precision_keeps_integer_zeroes() {
        assert_eq!(coordinate(100., 0), "100");
        assert_eq!(coordinate(-0.00001, 2), "0");
        assert_eq!(coordinate(1.23456, 3), "1.235");
    }
    #[test]
    fn mapping_overrides_and_composite_names() {
        let mut o = Options::default();
        assert_eq!(glyph_unicode("f_f_i", &o).as_deref(), Some("006600660069"));
        o.glyph_unicode.insert("a".into(), "03B1".into());
        assert_eq!(glyph_unicode("a", &o).as_deref(), Some("03B1"));
        assert_eq!(glyph_unicode("b", &o).as_deref(), Some("0062"));
        o.builtin_glyph_unicode.insert("aacute".into(), "00E1".into());
        o.builtin_glyph_unicode.insert("Alpha".into(), "0391".into());
        assert_eq!(glyph_unicode("aacute.sc", &o).as_deref(), Some("00E1"));
        assert_eq!(glyph_unicode("Alpha.alt", &o).as_deref(), Some("0391"));
        assert_eq!(glyph_unicode("a.alt", &o).as_deref(), Some("03B1"));
    }
}
