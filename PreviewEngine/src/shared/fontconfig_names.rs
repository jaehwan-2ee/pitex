/****************************************************************************\
 Part of the XeTeX typesetting system
 Copyright (c) 1994-2008 by SIL International
 Copyright (c) 2009, 2011 by Jonathan Kew

 SIL Author(s): Jonathan Kew

Permission is hereby granted, free of charge, to any person obtaining
a copy of this software and associated documentation files (the
"Software"), to deal in the Software without restriction, including
without limitation the rights to use, copy, modify, merge, publish,
distribute, sublicense, and/or sell copies of the Software, and to
permit persons to whom the Software is furnished to do so, subject to
the following conditions:

The above copyright notice and this permission notice shall be
included in all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND
NONINFRINGEMENT. IN NO EVENT SHALL THE COPYRIGHT HOLDERS BE LIABLE
FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF
CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION
WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

Except as contained in this notice, the name of the copyright holders
shall not be used in advertising or otherwise to promote the sale,
use or other dealings in this Software without prior written
authorization from the copyright holders.
\****************************************************************************/

// Manual Rust translation of Fontconfig discovery and name-table algorithms from
// xetex/layout/xetex-XeTeXFontMgr_FC.cpp. Native FontMgr containers stay opaque.
// Original SHA-256: bffffa93891c0f81fe5760de09e4a8fb224dddba42bf84137ef276108bc94cd9
use std::{
    ffi::{c_char, c_long, c_void, CStr, CString},
    ptr,
};
type Handle = *mut c_void;
#[repr(C)]
struct FontSet {
    count: i32,
    capacity: i32,
    fonts: *mut Handle,
}
#[repr(C)]
struct SfntName {
    platform: u16,
    encoding: u16,
    language: u16,
    name: u16,
    bytes: *mut u8,
    length: u32,
}
#[repr(C)]
struct NameList {
    values: *const *const c_char,
    count: usize,
}
#[repr(C)]
struct NamesView {
    postscript: *const c_char,
    full: NameList,
    family: NameList,
    style: NameList,
}
type ReceiveNames = unsafe extern "C" fn(Handle, *const NamesView);
type IsCached = unsafe extern "C" fn(Handle, Handle) -> bool;
type CacheNames = unsafe extern "C" fn(Handle, Handle, *const NamesView);
extern "C" {
    static mut gFreeTypeLibrary: Handle;
    fn FcPatternGetString(
        pattern: Handle,
        key: *const c_char,
        index: i32,
        value: *mut *mut u8,
    ) -> i32;
    fn FcPatternGetInteger(pattern: Handle, key: *const c_char, index: i32, value: *mut i32)
        -> i32;
    fn FT_New_Face(library: Handle, path: *const c_char, index: c_long, face: *mut Handle) -> i32;
    fn FT_Done_Face(face: Handle) -> i32;
    fn FT_Get_Postscript_Name(face: Handle) -> *const c_char;
    fn FT_Get_Sfnt_Name_Count(face: Handle) -> u32;
    fn FT_Get_Sfnt_Name(face: Handle, index: u32, name: *mut SfntName) -> i32;
    fn pitex_fc_face_is_sfnt(face: Handle) -> bool;
}
// Fontconfig object names are ASCII and nul terminated; its result value 0 is Match.
unsafe fn strings(pattern: Handle, key: &'static [u8]) -> Vec<Vec<u8>> {
    let mut result = Vec::new();
    let mut index = 0;
    let mut value = ptr::null_mut();
    while FcPatternGetString(pattern, key.as_ptr().cast(), index, &mut value) == 0 {
        result.push(CStr::from_ptr(value.cast()).to_bytes().to_vec());
        index += 1;
    }
    result
}
fn add_name(list: &mut Vec<CString>, name: CString, preferred: bool) {
    if let Some(index) = list.iter().position(|value| value == &name) {
        if !preferred {
            return;
        }
        list.remove(index);
    }
    if preferred {
        list.insert(0, name);
    } else {
        list.push(name);
    }
}
fn c_name(mut bytes: Vec<u8>) -> CString {
    // Upstream list operations consumed C strings, truncating embedded nul bytes.
    if let Some(end) = bytes.iter().position(|value| *value == 0) {
        bytes.truncate(end);
    }
    CString::new(bytes).unwrap()
}
const MAC_ROMAN: [u16; 128] = [
    0x00c4, 0x00c5, 0x00c7, 0x00c9, 0x00d1, 0x00d6, 0x00dc, 0x00e1, 0x00e0, 0x00e2, 0x00e4, 0x00e3,
    0x00e5, 0x00e7, 0x00e9, 0x00e8, 0x00ea, 0x00eb, 0x00ed, 0x00ec, 0x00ee, 0x00ef, 0x00f1, 0x00f3,
    0x00f2, 0x00f4, 0x00f6, 0x00f5, 0x00fa, 0x00f9, 0x00fb, 0x00fc, 0x2020, 0x00b0, 0x00a2, 0x00a3,
    0x00a7, 0x2022, 0x00b6, 0x00df, 0x00ae, 0x00a9, 0x2122, 0x00b4, 0x00a8, 0x2260, 0x00c6, 0x00d8,
    0x221e, 0x00b1, 0x2264, 0x2265, 0x00a5, 0x00b5, 0x2202, 0x2211, 0x220f, 0x03c0, 0x222b, 0x00aa,
    0x00ba, 0x03a9, 0x00e6, 0x00f8, 0x00bf, 0x00a1, 0x00ac, 0x221a, 0x0192, 0x2248, 0x2206, 0x00ab,
    0x00bb, 0x2026, 0x00a0, 0x00c0, 0x00c3, 0x00d5, 0x0152, 0x0153, 0x2013, 0x2014, 0x201c, 0x201d,
    0x2018, 0x2019, 0x00f7, 0x25ca, 0x00ff, 0x0178, 0x2044, 0x20ac, 0x2039, 0x203a, 0xfb01, 0xfb02,
    0x2021, 0x00b7, 0x201a, 0x201e, 0x2030, 0x00c2, 0x00ca, 0x00c1, 0x00cb, 0x00c8, 0x00cd, 0x00ce,
    0x00cf, 0x00cc, 0x00d3, 0x00d4, 0xf8ff, 0x00d2, 0x00da, 0x00db, 0x00d9, 0x0131, 0x02c6, 0x02dc,
    0x00af, 0x02d8, 0x02d9, 0x02da, 0x00b8, 0x02dd, 0x02db, 0x02c7,
];
fn decode_name(
    bytes: &[u8],
    platform: u16,
    encoding: u16,
    language: u16,
    mac_roman_available: bool,
) -> Option<(CString, bool)> {
    let preferred = mac_roman_available && platform == 1 && encoding == 0 && language == 0;
    let decoded = if preferred {
        bytes
            .iter()
            .map(|value| {
                char::from_u32(if *value < 128 {
                    *value as u32
                } else {
                    MAC_ROMAN[(*value - 128) as usize] as u32
                })
                .unwrap()
            })
            .collect::<String>()
    } else if platform == 0 || platform == 3 {
        let mut text = String::new();
        let mut offset = 0;
        while offset < bytes.len() {
            if offset + 1 == bytes.len() {
                text.push('\u{fffd}');
                break;
            }
            let first = u16::from_be_bytes([bytes[offset], bytes[offset + 1]]);
            offset += 2;
            let value = if (0xd800..=0xdbff).contains(&first) {
                if offset + 1 < bytes.len() {
                    let second = u16::from_be_bytes([bytes[offset], bytes[offset + 1]]);
                    if (0xdc00..=0xdfff).contains(&second) {
                        offset += 2;
                        0x10000 + ((first as u32 - 0xd800) << 10) + second as u32 - 0xdc00
                    } else {
                        0xfffd
                    }
                } else {
                    // ICU consumes a trailing incomplete low word with the high
                    // surrogate as a single malformed sequence.
                    offset = bytes.len();
                    0xfffd
                }
            } else if (0xdc00..=0xdfff).contains(&first) {
                0xfffd
            } else {
                first as u32
            };
            text.push(char::from_u32(value).unwrap());
        }
        text
    } else {
        return None;
    };
    Some((c_name(decoded.into_bytes()), preferred))
}
#[derive(Default)]
struct Names {
    postscript: CString,
    full: Vec<CString>,
    family: Vec<CString>,
    style: Vec<CString>,
}
impl Names {
    fn with_view<R>(&self, call: impl FnOnce(*const NamesView) -> R) -> R {
        let full = self
            .full
            .iter()
            .map(|name| name.as_ptr())
            .collect::<Vec<_>>();
        let family = self
            .family
            .iter()
            .map(|name| name.as_ptr())
            .collect::<Vec<_>>();
        let style = self
            .style
            .iter()
            .map(|name| name.as_ptr())
            .collect::<Vec<_>>();
        let view = NamesView {
            postscript: self.postscript.as_ptr(),
            full: NameList {
                values: full.as_ptr(),
                count: full.len(),
            },
            family: NameList {
                values: family.as_ptr(),
                count: family.len(),
            },
            style: NameList {
                values: style.as_ptr(),
                count: style.len(),
            },
        };
        call(&view)
    }
}
struct Face(Handle);
impl Drop for Face {
    fn drop(&mut self) {
        unsafe {
            FT_Done_Face(self.0);
        }
    }
}
unsafe fn read_names(pattern: Handle, mac_roman_available: bool) -> Names {
    let mut result = Names::default();
    let mut path = ptr::null_mut();
    let mut index = 0;
    if FcPatternGetString(pattern, b"file\0".as_ptr().cast(), 0, &mut path) != 0
        || FcPatternGetInteger(pattern, b"index\0".as_ptr().cast(), 0, &mut index) != 0
    {
        return result;
    }
    let mut face = ptr::null_mut();
    if FT_New_Face(gFreeTypeLibrary, path.cast(), index as c_long, &mut face) != 0 {
        return result;
    }
    let face = Face(face);
    let postscript = FT_Get_Postscript_Name(face.0);
    if postscript.is_null() {
        return result;
    }
    result.postscript = CStr::from_ptr(postscript).to_owned();
    if pitex_fc_face_is_sfnt(face.0) {
        let mut preferred_family = Vec::new();
        let mut preferred_style = Vec::new();
        for index in 0..FT_Get_Sfnt_Name_Count(face.0) {
            let mut record = std::mem::zeroed::<SfntName>();
            if FT_Get_Sfnt_Name(face.0, index, &mut record) != 0
                || !matches!(record.name, 1 | 2 | 4 | 16 | 17)
            {
                continue;
            }
            let bytes = if record.length == 0 {
                &[]
            } else {
                std::slice::from_raw_parts(record.bytes, record.length as usize)
            };
            if let Some((name, preferred)) = decode_name(
                bytes,
                record.platform,
                record.encoding,
                record.language,
                mac_roman_available,
            ) {
                let list = match record.name {
                    1 => &mut result.family,
                    2 => &mut result.style,
                    4 => &mut result.full,
                    16 => &mut preferred_family,
                    17 => &mut preferred_style,
                    _ => unreachable!(),
                };
                add_name(list, name, preferred);
            }
        }
        if !preferred_family.is_empty() {
            result.family = preferred_family;
        }
        if !preferred_style.is_empty() {
            result.style = preferred_style;
        }
    } else {
        for (list, key) in [
            (&mut result.full, b"fullname\0".as_slice()),
            (&mut result.family, b"family\0".as_slice()),
            (&mut result.style, b"style\0".as_slice()),
        ] {
            for name in strings(pattern, key) {
                add_name(list, c_name(name), false);
            }
        }
        if result.full.is_empty() {
            // Broken non-SFNT patterns may have no family; avoid the former front() UB.
            if let Some(family) = result.family.first() {
                let mut full = family.as_bytes().to_vec();
                if let Some(style) = result.style.first() {
                    full.push(b' ');
                    full.extend_from_slice(style.as_bytes());
                }
                result.full.push(c_name(full));
            }
        }
    }
    result
}
#[no_mangle]
unsafe extern "C" fn pitex_fc_read_names(
    pattern: Handle,
    mac_roman_available: bool,
    output: Handle,
    receive: ReceiveNames,
) {
    read_names(pattern, mac_roman_available).with_view(|view| receive(output, view));
}
unsafe fn cache_families(
    manager: Handle,
    patterns: &[Handle],
    families: &[CString],
    mac_roman_available: bool,
    is_cached: IsCached,
    cache: CacheNames,
) {
    if families.is_empty() {
        return;
    }
    for &pattern in patterns {
        if is_cached(manager, pattern) {
            continue;
        }
        if strings(pattern, b"family\0")
            .iter()
            .any(|family| families.iter().any(|name| name.as_bytes() == family))
        {
            let names = read_names(pattern, mac_roman_available);
            names.with_view(|view| cache(manager, pattern, view));
        }
    }
}
fn matches_pattern(
    name: &[u8],
    family_prefix: Option<&[u8]>,
    full: &[Vec<u8>],
    families: &[Vec<u8>],
    styles: &[Vec<u8>],
) -> bool {
    if full.iter().any(|value| value == name) {
        return true;
    }
    families.iter().any(|family| {
        if family == name || family_prefix == Some(family.as_slice()) {
            return true;
        }
        styles.iter().any(|style| {
            let mut joined = family.clone();
            joined.push(b' ');
            joined.extend_from_slice(style);
            joined == name
        })
    })
}
#[no_mangle]
unsafe extern "C" fn pitex_fc_search(
    manager: Handle,
    fonts: *const FontSet,
    cached_all: *mut bool,
    name: *const c_char,
    mac_roman_available: bool,
    is_cached: IsCached,
    cache: CacheNames,
) {
    if *cached_all {
        return;
    }
    let name = CStr::from_ptr(name).to_bytes();
    let prefix = name
        .iter()
        .position(|value| *value == b'-')
        .filter(|index| *index > 0 && *index + 1 < name.len())
        .map(|index| &name[..index]);
    let patterns = if (*fonts).count == 0 {
        &[]
    } else {
        std::slice::from_raw_parts((*fonts).fonts, (*fonts).count as usize)
    };
    let mut found = false;
    loop {
        for &pattern in patterns {
            if is_cached(manager, pattern) {
                continue;
            }
            if *cached_all {
                let names = read_names(pattern, mac_roman_available);
                names.with_view(|view| cache(manager, pattern, view));
                continue;
            }
            if matches_pattern(
                name,
                prefix,
                &strings(pattern, b"fullname\0"),
                &strings(pattern, b"family\0"),
                &strings(pattern, b"style\0"),
            ) {
                let names = read_names(pattern, mac_roman_available);
                names.with_view(|view| cache(manager, pattern, view));
                cache_families(
                    manager,
                    patterns,
                    &names.family,
                    mac_roman_available,
                    is_cached,
                    cache,
                );
                found = true;
            }
        }
        if found || *cached_all {
            break;
        }
        *cached_all = true;
    }
}
#[no_mangle]
unsafe extern "C" fn pitex_fc_style(
    pattern: Handle,
    weight: *mut u16,
    width: *mut u16,
    slant: *mut i16,
) {
    if *weight != 0 || *width != 0 {
        return;
    }
    let mut value = 0;
    if FcPatternGetInteger(pattern, b"weight\0".as_ptr().cast(), 0, &mut value) == 0 {
        *weight = value as u16;
    }
    if FcPatternGetInteger(pattern, b"width\0".as_ptr().cast(), 0, &mut value) == 0 {
        *width = value as u16;
    }
    if FcPatternGetInteger(pattern, b"slant\0".as_ptr().cast(), 0, &mut value) == 0 {
        *slant = value as i16;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unicode_names_include_surrogates_and_icu_malformed_consumption() {
        for (input, expected) in [
            (&[0x00, 0x41, 0xd8, 0x3d, 0xde, 0x00][..], "A😀"),
            (&[0xfe, 0xff, 0x00, 0x41][..], "\u{feff}A"),
            (&[0xd8, 0x00, 0x00][..], "�"),
            (&[0xdc, 0x00, 0x00, 0x41][..], "�A"),
            (&[0x00, 0x41, 0x00, 0x00, 0x00, 0x42][..], "A"),
        ] {
            let (name, preferred) = decode_name(input, 3, 1, 0x409, true).unwrap();
            assert_eq!(name.to_str().unwrap(), expected);
            assert!(!preferred);
        }
    }
    #[test]
    fn english_mac_roman_names_move_to_front_without_duplicates() {
        let (name, preferred) = decode_name(&[b'C', b'a', b'f', 0x8e], 1, 0, 0, true).unwrap();
        assert_eq!(name.to_str().unwrap(), "Café");
        assert!(preferred);
        assert!(decode_name(b"Ignored", 1, 0, 1, true).is_none());
        assert!(decode_name(b"Ignored", 1, 0, 0, false).is_none());
        let mut names = vec![CString::new("Other").unwrap(), name.clone()];
        add_name(&mut names, name.clone(), true);
        assert_eq!(names.len(), 2);
        assert_eq!(names[0], name);
        add_name(&mut names, name, false);
        assert_eq!(names.len(), 2);
    }
}
