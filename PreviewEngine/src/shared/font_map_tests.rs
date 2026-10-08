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
