// Pitex-authored font-map regression checks (AGPL-3.0-or-later).
use super::*;
use std::ffi::{CStr, CString};

#[test]
fn map_updates_preserve_duplicates_replacements_removals_and_reset() {
    unsafe {
        let fc = font_cache_new(xdv_resolver { env: std::ptr::null_mut(), load: None });
        (*fc).map_loaded = true;
        let parse = |text: &str, mode| {
            let text = CString::new(text).unwrap();
            pitex_parse_map_text(fc, strdup(text.as_ptr()), mode);
        };
        let name = |index| CStr::from_ptr((*(*fc).map.offset(index)).psname).to_bytes();

        parse("a First\nb Second\nc Third\na Ignored\n", b'+');
        parse("b AlsoIgnored\nd Fourth\n", b'+');
        assert_eq!((*fc).nmap, 4);
        assert_eq!((*fc).duplicate_maps, 2);
        assert_eq!(name(0), b"First");
        assert_eq!(name(1), b"Second");

        parse("b Replaced\ne Fifth\ne ReplacedAgain\n", b'=');
        assert_eq!((*fc).nmap, 5);
        assert_eq!(name(1), b"Replaced");
        assert_eq!(name(4), b"ReplacedAgain");
        parse("b\nd\nmissing\nc\n", b'-');
        assert_eq!((*fc).nmap, 2);
        assert_eq!(name(0), b"First");
        assert_eq!(name(1), b"ReplacedAgain");

        font_map_reset(fc);
        assert_eq!((*fc).nmap, 0);
        assert_eq!((*fc).duplicate_maps, 0);
        assert!(!(*fc).map_loaded);
        parse("a NewGeneration\na Ignored\n", b'+');
        assert_eq!((*fc).nmap, 1);
        assert_eq!((*fc).duplicate_maps, 1);
        assert_eq!(name(0), b"NewGeneration");
        font_cache_free(fc);
    }
}

#[test]
fn math_size_variants_copy_as_their_base_character() {
    extern "C" {
        fn tbuf_from_copy(data: *const ::core::ffi::c_void, len: size_t) -> *mut tbuf;
    }
    unsafe extern "C" fn load(
        _: *mut ::core::ffi::c_void,
        name: *const ::core::ffi::c_char,
        _: xdv_res_kind,
    ) -> *mut tbuf {
        let bytes = std::fs::read(CStr::from_ptr(name).to_str().unwrap()).unwrap();
        tbuf_from_copy(bytes.as_ptr().cast(), bytes.len() as size_t)
    }
    let Ok(found) = std::process::Command::new("kpsewhich").arg("latinmodern-math.otf").output() else {
        return eprintln!("skipped: kpsewhich not found");
    };
    let path = String::from_utf8(found.stdout).unwrap();
    if path.trim().is_empty() {
        return eprintln!("skipped: latinmodern-math.otf not installed");
    }
    unsafe {
        let fc = font_cache_new(xdv_resolver { env: std::ptr::null_mut(), load: Some(load) });
        let name = CString::new(path.trim()).unwrap();
        let mut warnings: pbuf = std::mem::zeroed();
        let f = font_native(fc, name.as_ptr(), 0, &raw mut warnings);
        assert!((*f).ok);
        let to_unicode = std::slice::from_raw_parts((*f).to_unicode, (*f).num_glyphs as usize);
        for character in ['∑', '∫', '('] {
            let glyphs = to_unicode.iter().filter(|&&cp| cp == character as u32).count();
            assert!(glyphs >= 2, "{character} has {glyphs} glyphs with a ToUnicode entry");
        }
        font_cache_free(fc);
    }
}
