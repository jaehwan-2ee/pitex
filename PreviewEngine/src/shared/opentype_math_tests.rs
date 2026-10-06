// Standalone C ABI and real OpenType MATH table checks for the retained MIT
// translation. Run with rustc --test --cfg math_standalone ... -lharfbuzz.
use super::*;
use std::{
    ffi::{c_char, CString},
    process::Command,
};
#[repr(C)]
struct Font {
    hb: *mut c_void,
    size: f32,
    units: f32,
}
#[repr(C)]
#[derive(Default)]
struct Extents {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
}
extern "C" {
    fn hb_blob_create_from_file_or_fail(file: *const c_char) -> *mut c_void;
    fn hb_blob_destroy(blob: *mut c_void);
    fn hb_face_create(blob: *mut c_void, index: u32) -> *mut c_void;
    fn hb_face_destroy(face: *mut c_void);
    fn hb_face_get_upem(face: *mut c_void) -> u32;
    fn hb_font_create(face: *mut c_void) -> *mut c_void;
    fn hb_font_destroy(font: *mut c_void);
    fn hb_font_set_scale(font: *mut c_void, x: i32, y: i32);
    fn hb_ot_font_set_funcs(font: *mut c_void);
    fn hb_font_get_nominal_glyph(font: *mut c_void, character: u32, glyph: *mut u32) -> i32;
    fn hb_font_get_glyph_extents(font: *mut c_void, glyph: u32, extents: *mut Extents) -> i32;
}
#[no_mangle]
static mut font_area: *mut i32 = ptr::null_mut();
#[no_mangle]
static mut font_size: *mut i32 = ptr::null_mut();
#[no_mangle]
static mut font_layout_engine: *mut *mut c_void = ptr::null_mut();
#[no_mangle]
unsafe extern "C" fn getFont(engine: *mut c_void) -> *mut c_void {
    engine
}
#[no_mangle]
unsafe extern "C" fn ttxl_get_hb_font(engine: *mut c_void) -> *mut c_void {
    (*engine.cast::<Font>()).hb
}
#[no_mangle]
unsafe extern "C" fn ttxl_font_units_to_points(font: *mut c_void, units: f32) -> f32 {
    let font = &*font.cast::<Font>();
    units * font.size / font.units
}
#[no_mangle]
unsafe extern "C" fn ttxl_font_points_to_units(font: *mut c_void, points: f32) -> f32 {
    let font = &*font.cast::<Font>();
    points * font.units / font.size
}
#[no_mangle]
unsafe extern "C" fn ttxl_font_get_point_size(font: *mut c_void) -> f32 {
    (*font.cast::<Font>()).size
}
#[no_mangle]
unsafe extern "C" fn getGlyphHeightDepth(
    engine: *mut c_void,
    glyph: u32,
    height: *mut f32,
    depth: *mut f32,
) {
    let mut extents = Extents::default();
    hb_font_get_glyph_extents(ttxl_get_hb_font(engine), glyph, &mut extents);
    if !height.is_null() {
        *height = ttxl_font_units_to_points(engine, extents.y as f32);
    }
    if !depth.is_null() {
        *depth = ttxl_font_units_to_points(engine, -(extents.y + extents.height) as f32);
    }
}
#[no_mangle]
unsafe extern "C" fn D2Fix(value: f64) -> i32 {
    (value * 65536. + 0.5) as i32
}
#[no_mangle]
unsafe extern "C" fn Fix2D(value: i32) -> f64 {
    value as f64 / 65536.
}

#[cfg(math_reference)]
extern "C" {
    fn reference_get_ot_math_constant(f: i32, n: i32) -> i32;
    fn reference_get_native_mathsy_param(f: i32, n: i32) -> i32;
    fn reference_get_native_mathex_param(f: i32, n: i32) -> i32;
    fn reference_get_ot_math_variant(
        f: i32,
        g: i32,
        v: i32,
        advance: *mut i32,
        horizontal: i32,
    ) -> i32;
    fn reference_get_ot_assembly_ptr(f: i32, g: i32, horizontal: i32) -> *mut c_void;
    fn reference_free_ot_assembly(assembly: *mut GlyphAssembly);
    fn reference_get_ot_math_ital_corr(f: i32, g: i32) -> i32;
    fn reference_get_ot_math_accent_pos(f: i32, g: i32) -> i32;
    fn reference_ot_min_connector_overlap(f: i32) -> i32;
    fn reference_get_ot_math_kern(
        f: i32,
        g: i32,
        sf: i32,
        sg: i32,
        command: i32,
        shift: i32,
    ) -> i32;
    fn reference_ot_part_count(assembly: *const GlyphAssembly) -> i32;
    fn reference_ot_part_glyph(assembly: *const GlyphAssembly, index: i32) -> i32;
    fn reference_ot_part_is_extender(assembly: *const GlyphAssembly, index: i32) -> bool;
    fn reference_ot_part_start_connector(f: i32, assembly: *const GlyphAssembly, index: i32)
        -> i32;
    fn reference_ot_part_end_connector(f: i32, assembly: *const GlyphAssembly, index: i32) -> i32;
    fn reference_ot_part_full_advance(f: i32, assembly: *const GlyphAssembly, index: i32) -> i32;
}
#[test]
fn mathematical_boundaries_and_c_abi() {
    unsafe {
        let path = std::env::var("PITEX_MATH_FIXTURE").unwrap_or_else(|_| {
            String::from_utf8(
                Command::new("kpsewhich")
                    .arg("latinmodern-math.otf")
                    .output()
                    .unwrap()
                    .stdout,
            )
            .unwrap()
            .trim()
            .into()
        });
        let file = CString::new(path).unwrap();
        let blob = hb_blob_create_from_file_or_fail(file.as_ptr());
        assert!(!blob.is_null());
        let face = hb_face_create(blob, 0);
        let units = hb_face_get_upem(face) as f32;
        assert!(units > 0.);
        let mut fonts = [0., 10., 7., 5., 17.3125, 0.01].map(|size| {
            let hb = hb_font_create(face);
            hb_font_set_scale(hb, units as i32, units as i32);
            hb_ot_font_set_funcs(hb);
            Box::new(Font { hb, size, units })
        });
        let mut areas = [0, OPEN_TYPE, OPEN_TYPE, OPEN_TYPE, OPEN_TYPE, OPEN_TYPE];
        let mut sizes = [
            0,
            10 * 65536,
            7 * 65536,
            5 * 65536,
            D2Fix(17.3125),
            D2Fix(0.01),
        ];
        let mut engines = fonts
            .iter_mut()
            .map(|font| (&mut **font as *mut Font).cast::<c_void>())
            .collect::<Vec<_>>();
        font_area = areas.as_mut_ptr();
        font_size = sizes.as_mut_ptr();
        font_layout_engine = engines.as_mut_ptr();
        // Non-MATH fonts preserve the accent sentinel and absent-variant marker.
        assert_eq!(get_ot_math_accent_pos(0, 0), i32::MAX);
        assert_eq!(get_ot_math_ital_corr(0, 0), 0);
        assert!(get_ot_assembly_ptr(0, 0, 0).is_null());
        let mut advance = 0;
        assert_eq!(get_ot_math_variant(0, 17, 0, &mut advance, 0), 17);
        assert_eq!(advance, -1);
        assert_eq!(get_native_mathsy_param(1, -1), 0);
        assert_eq!(get_native_mathex_param(1, 100), 0);
        let mut glyphs = Vec::new();
        for character in ['(', ')', '√', '∫', '∑', '→', '̂', 'x', 'f', 'j', '𝑓', '𝑥']
        {
            let mut glyph = 0;
            assert_ne!(
                hb_font_get_nominal_glyph(fonts[1].hb, character as u32, &mut glyph),
                0
            );
            glyphs.push(glyph as i32);
        }
        let mut assemblies = 0;
        for f in 1..fonts.len() as i32 {
            for constant in 0..56 {
                let raw = hb_ot_math_get_constant(fonts[f as usize].hb, constant);
                let expected = if matches!(constant, 0 | 1 | 55) {
                    raw
                } else {
                    D2Fix(ttxl_font_units_to_points(engines[f as usize], raw as f32) as f64)
                };
                assert_eq!(get_ot_math_constant(f, constant), expected);
                #[cfg(math_reference)]
                assert_eq!(
                    get_ot_math_constant(f, constant),
                    reference_get_ot_math_constant(f, constant)
                );
            }
            assert_eq!(get_ot_math_constant(f, 0), get_ot_math_constant(1, 0));
            assert_eq!(get_native_mathsy_param(f, 6), sizes[f as usize]);
            assert!(get_native_mathsy_param(f, 21) <= get_native_mathsy_param(f, 20));
            #[cfg(math_reference)]
            for parameter in 0..32 {
                assert_eq!(
                    get_native_mathsy_param(f, parameter),
                    reference_get_native_mathsy_param(f, parameter)
                );
                assert_eq!(
                    get_native_mathex_param(f, parameter),
                    reference_get_native_mathex_param(f, parameter)
                );
            }
            for &glyph in &glyphs {
                #[cfg(math_reference)]
                {
                    assert_eq!(
                        get_ot_math_ital_corr(f, glyph),
                        reference_get_ot_math_ital_corr(f, glyph)
                    );
                    assert_eq!(
                        get_ot_math_accent_pos(f, glyph),
                        reference_get_ot_math_accent_pos(f, glyph)
                    );
                    assert_eq!(
                        ot_min_connector_overlap(f),
                        reference_ot_min_connector_overlap(f)
                    );
                }
                for horizontal in [0, 1] {
                    for variant in [0, 1, 2, 5, 100000] {
                        let mut advance = 0;
                        let selected =
                            get_ot_math_variant(f, glyph, variant, &mut advance, horizontal);
                        if variant == 100000 {
                            assert_eq!((selected, advance), (glyph, -1));
                        }
                        #[cfg(math_reference)]
                        {
                            let mut original_advance = 0;
                            let original = reference_get_ot_math_variant(
                                f,
                                glyph,
                                variant,
                                &mut original_advance,
                                horizontal,
                            );
                            assert_eq!((selected, advance), (original, original_advance));
                        }
                    }
                    let assembly =
                        get_ot_assembly_ptr(f, glyph, horizontal).cast::<GlyphAssembly>();
                    #[cfg(math_reference)]
                    let original =
                        reference_get_ot_assembly_ptr(f, glyph, horizontal).cast::<GlyphAssembly>();
                    #[cfg(math_reference)]
                    assert_eq!(assembly.is_null(), original.is_null());
                    if !assembly.is_null() {
                        assemblies += 1;
                        assert!(ot_part_count(assembly) > 0);
                        let mut extenders = 0;
                        for part in 0..ot_part_count(assembly) {
                            let full = ot_part_full_advance(f, assembly, part);
                            assert!(ot_part_start_connector(f, assembly, part) <= full);
                            assert!(ot_part_end_connector(f, assembly, part) <= full);
                            extenders += ot_part_is_extender(assembly, part) as i32;
                            #[cfg(math_reference)]
                            {
                                assert_eq!(
                                    ot_part_count(assembly),
                                    reference_ot_part_count(original)
                                );
                                assert_eq!(
                                    ot_part_glyph(assembly, part),
                                    reference_ot_part_glyph(original, part)
                                );
                                assert_eq!(
                                    ot_part_is_extender(assembly, part),
                                    reference_ot_part_is_extender(original, part)
                                );
                                assert_eq!(
                                    ot_part_start_connector(f, assembly, part),
                                    reference_ot_part_start_connector(f, original, part)
                                );
                                assert_eq!(
                                    ot_part_end_connector(f, assembly, part),
                                    reference_ot_part_end_connector(f, original, part)
                                );
                                assert_eq!(full, reference_ot_part_full_advance(f, original, part));
                            }
                        }
                        assert!(extenders > 0);
                        free_ot_assembly(assembly);
                    }
                    #[cfg(math_reference)]
                    reference_free_ot_assembly(original);
                }
                for sf in 1..fonts.len() as i32 {
                    for &script in &glyphs {
                        for command in [0, 1] {
                            for shift in [-10 * 65536, -1, 0, 1, 2 * 65536, 10 * 65536] {
                                let value = get_ot_math_kern(f, glyph, sf, script, command, shift);
                                #[cfg(math_reference)]
                                assert_eq!(
                                    value,
                                    reference_get_ot_math_kern(
                                        f, glyph, sf, script, command, shift
                                    )
                                );
                                assert!(value.abs() < 100 * 65536);
                            }
                        }
                    }
                }
            }
        }
        assert!(assemblies > 0);
        free_ot_assembly(ptr::null_mut());
        font_area = ptr::null_mut();
        font_size = ptr::null_mut();
        font_layout_engine = ptr::null_mut();
        for font in fonts {
            hb_font_destroy(font.hb);
        }
        hb_face_destroy(face);
        hb_blob_destroy(blob);
    }
}
