/*
 Part of the XeTeX typesetting system
 Copyright (c) 1994-2008 by SIL International
 Copyright (c) 2009 by Jonathan Kew
 Copyright (c) 2012-2015 by Khaled Hosny

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
*/
// Manual Rust translation of xetex/engine/xetex-XeTeXOTMath.cpp.
// Original SHA-256: a58bf996f4f23b14d2d52dd1e735d9c0785dab782aef59d6f7bd335c32cdf915
// The original C ABI, HarfBuzz units, f32 intermediate arithmetic and fixed
// point rounding are preserved, including XeTeX's maximum-of-two-kerns rule.
use std::{ffi::c_void, ptr};
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct GlyphPart {
    pub glyph: u32,
    pub start_connector_length: i32,
    pub end_connector_length: i32,
    pub full_advance: i32,
    pub flags: u32,
}
#[repr(C)]
pub struct GlyphAssembly {
    pub count: u32,
    pub parts: *mut GlyphPart,
}
#[repr(C)]
#[derive(Default)]
struct GlyphVariant {
    glyph: u32,
    advance: i32,
}
extern "C" {
    static mut font_area: *mut i32;
    static mut font_size: *mut i32;
    static mut font_layout_engine: *mut *mut c_void;
    fn getFont(engine: *mut c_void) -> *mut c_void;
    fn ttxl_get_hb_font(engine: *mut c_void) -> *mut c_void;
    fn ttxl_font_units_to_points(font: *mut c_void, units: f32) -> f32;
    fn ttxl_font_points_to_units(font: *mut c_void, points: f32) -> f32;
    fn ttxl_font_get_point_size(font: *mut c_void) -> f32;
    fn getGlyphHeightDepth(engine: *mut c_void, glyph: u32, height: *mut f32, depth: *mut f32);
    fn D2Fix(points: f64) -> i32;
    fn Fix2D(points: i32) -> f64;
    fn malloc(size: usize) -> *mut c_void;
    fn free(pointer: *mut c_void);
    fn abort() -> !;
    fn hb_ot_math_get_constant(font: *mut c_void, constant: i32) -> i32;
    fn hb_ot_math_get_glyph_variants(
        font: *mut c_void,
        glyph: u32,
        direction: u32,
        start: u32,
        count: *mut u32,
        variants: *mut GlyphVariant,
    ) -> u32;
    fn hb_ot_math_get_glyph_assembly(
        font: *mut c_void,
        glyph: u32,
        direction: u32,
        start: u32,
        count: *mut u32,
        parts: *mut GlyphPart,
        italic: *mut i32,
    ) -> u32;
    fn hb_ot_math_get_glyph_italics_correction(font: *mut c_void, glyph: u32) -> i32;
    fn hb_ot_math_get_glyph_top_accent_attachment(font: *mut c_void, glyph: u32) -> i32;
    fn hb_ot_math_get_min_connector_overlap(font: *mut c_void, direction: u32) -> i32;
    fn hb_ot_math_get_glyph_kerning(font: *mut c_void, glyph: u32, side: i32, height: i32) -> i32;
}
const OPEN_TYPE: i32 = 0xfffe;
const SYMBOL_CONSTANTS: [i32; 23] = [
    -1, -1, -1, -1, -1, 6, -1, -1, 33, 32, 22, 35, 34, 11, 11, 12, 8, 8, 14, 10, 2, -1, 5,
];
const EXTENSION_CONSTANTS: [i32; 14] = [-1, -1, -1, -1, -1, 6, -1, -1, 38, 18, 20, 19, 21, 26];
unsafe fn native(f: i32) -> bool {
    *font_area.offset(f as isize) == OPEN_TYPE
}
unsafe fn engine(f: i32) -> *mut c_void {
    *font_layout_engine.offset(f as isize)
}
unsafe fn fixed_units(f: i32, units: i32) -> i32 {
    D2Fix(ttxl_font_units_to_points(getFont(engine(f)), units as f32) as f64)
}
fn direction(horizontal: i32) -> u32 {
    if horizontal != 0 {
        5
    } else {
        6
    }
}
#[no_mangle]
pub unsafe extern "C" fn get_ot_math_constant(f: i32, n: i32) -> i32 {
    if !native(f) {
        return 0;
    }
    let value = hb_ot_math_get_constant(ttxl_get_hb_font(engine(f)), n);
    if matches!(n, 0 | 1 | 55) {
        value
    } else {
        fixed_units(f, value)
    }
}
#[no_mangle]
pub unsafe extern "C" fn get_native_mathsy_param(f: i32, n: i32) -> i32 {
    if n == 6 {
        return *font_size.offset(f as isize);
    }
    if n == 21 {
        return ((*font_size.offset(f as isize) as f64 * 1.5) as i32)
            .min(get_native_mathsy_param(f, 20));
    }
    SYMBOL_CONSTANTS
        .get(n as usize)
        .filter(|n| **n >= 0)
        .map_or(0, |n| get_ot_math_constant(f, *n))
}
#[no_mangle]
pub unsafe extern "C" fn get_native_mathex_param(f: i32, n: i32) -> i32 {
    if n == 6 {
        return *font_size.offset(f as isize);
    }
    EXTENSION_CONSTANTS
        .get(n as usize)
        .filter(|n| **n >= 0)
        .map_or(0, |n| get_ot_math_constant(f, *n))
}
#[no_mangle]
pub unsafe extern "C" fn get_ot_math_variant(
    f: i32,
    g: i32,
    v: i32,
    advance: *mut i32,
    horizontal: i32,
) -> i32 {
    *advance = -1;
    if !native(f) {
        return g;
    }
    let mut count = 1;
    let mut variant = GlyphVariant::default();
    hb_ot_math_get_glyph_variants(
        ttxl_get_hb_font(engine(f)),
        g as u32,
        direction(horizontal),
        v as u32,
        &mut count,
        &mut variant,
    );
    if count == 0 {
        return g;
    }
    *advance = fixed_units(f, variant.advance);
    variant.glyph as i32
}
#[no_mangle]
pub unsafe extern "C" fn get_ot_assembly_ptr(f: i32, g: i32, horizontal: i32) -> *mut c_void {
    if !native(f) {
        return ptr::null_mut();
    }
    let hb = ttxl_get_hb_font(engine(f));
    let count = hb_ot_math_get_glyph_assembly(
        hb,
        g as u32,
        direction(horizontal),
        0,
        ptr::null_mut(),
        ptr::null_mut(),
        ptr::null_mut(),
    );
    if count == 0 {
        return ptr::null_mut();
    }
    let assembly = malloc(std::mem::size_of::<GlyphAssembly>()).cast::<GlyphAssembly>();
    let parts = malloc(count as usize * std::mem::size_of::<GlyphPart>()).cast::<GlyphPart>();
    if assembly.is_null() || parts.is_null() {
        abort();
    }
    *assembly = GlyphAssembly { count, parts };
    hb_ot_math_get_glyph_assembly(
        hb,
        g as u32,
        direction(horizontal),
        0,
        &mut (*assembly).count,
        parts,
        ptr::null_mut(),
    );
    assembly.cast()
}
#[no_mangle]
pub unsafe extern "C" fn free_ot_assembly(assembly: *mut GlyphAssembly) {
    if !assembly.is_null() {
        free((*assembly).parts.cast());
        free(assembly.cast());
    }
}
#[no_mangle]
pub unsafe extern "C" fn get_ot_math_ital_corr(f: i32, g: i32) -> i32 {
    if !native(f) {
        return 0;
    }
    fixed_units(
        f,
        hb_ot_math_get_glyph_italics_correction(ttxl_get_hb_font(engine(f)), g as u32),
    )
}
#[no_mangle]
pub unsafe extern "C" fn get_ot_math_accent_pos(f: i32, g: i32) -> i32 {
    if !native(f) {
        return i32::MAX;
    }
    fixed_units(
        f,
        hb_ot_math_get_glyph_top_accent_attachment(ttxl_get_hb_font(engine(f)), g as u32),
    )
}
#[no_mangle]
pub unsafe extern "C" fn ot_min_connector_overlap(f: i32) -> i32 {
    if !native(f) {
        return 0;
    }
    fixed_units(
        f,
        hb_ot_math_get_min_connector_overlap(ttxl_get_hb_font(engine(f)), 5),
    )
}
unsafe fn kern(f: i32, g: i32, height: i32, side: i32) -> i32 {
    if !native(f) {
        return 0;
    }
    hb_ot_math_get_glyph_kerning(ttxl_get_hb_font(engine(f)), g as u32, side, height)
}
unsafe fn glyph_metrics(f: i32, g: i32) -> (f32, f32) {
    let (mut height, mut depth) = (0., 0.);
    if native(f) {
        getGlyphHeightDepth(engine(f), g as u32, &mut height, &mut depth);
    }
    (height, depth)
}
#[no_mangle]
pub unsafe extern "C" fn get_ot_math_kern(
    f: i32,
    g: i32,
    sf: i32,
    sg: i32,
    command: i32,
    shift_scaled: i32,
) -> i32 {
    if !native(f) || !native(sf) {
        return 0;
    }
    let font = getFont(engine(f));
    let script = getFont(engine(sf));
    let (height, depth) = glyph_metrics(f, g);
    let (script_height, script_depth) = glyph_metrics(sf, sg);
    let height = ttxl_font_points_to_units(font, height) as i32;
    let depth = ttxl_font_points_to_units(font, depth) as i32;
    let script_height = ttxl_font_points_to_units(script, script_height) as i32;
    let script_depth = ttxl_font_points_to_units(script, script_depth) as i32;
    let shift = ttxl_font_points_to_units(font, Fix2D(shift_scaled) as f32) as i32;
    let size = ttxl_font_get_point_size(font);
    if size == 0. {
        abort();
    }
    let scale = ttxl_font_get_point_size(script) / size;
    let (first, second) = if command == 0 {
        let first = (kern(f, g, (shift as f32 - scale * script_depth as f32) as i32, 0) as f32
            + scale * kern(sf, sg, -script_depth, 3) as f32) as i32;
        let second = (kern(f, g, height, 0) as f32
            + scale * kern(sf, sg, ((height - shift) as f32 / scale) as i32, 3) as f32)
            as i32;
        (first, second)
    } else if command == 1 {
        let first = (kern(
            f,
            g,
            (scale * script_height as f32 - shift as f32) as i32,
            2,
        ) as f32
            + scale * kern(sf, sg, script_height, 1) as f32) as i32;
        let second = (kern(f, g, -depth, 2) as f32
            + scale * kern(sf, sg, ((shift - depth) as f32 / scale) as i32, 1) as f32)
            as i32;
        (first, second)
    } else {
        abort();
    };
    fixed_units(f, first.max(second))
}
#[no_mangle]
pub unsafe extern "C" fn ot_part_count(assembly: *const GlyphAssembly) -> i32 {
    (*assembly).count as i32
}
#[no_mangle]
pub unsafe extern "C" fn ot_part_glyph(assembly: *const GlyphAssembly, index: i32) -> i32 {
    (*(*assembly).parts.offset(index as isize)).glyph as i32
}
#[no_mangle]
pub unsafe extern "C" fn ot_part_is_extender(assembly: *const GlyphAssembly, index: i32) -> bool {
    (*(*assembly).parts.offset(index as isize)).flags & 1 != 0
}
#[no_mangle]
pub unsafe extern "C" fn ot_part_start_connector(
    f: i32,
    assembly: *const GlyphAssembly,
    index: i32,
) -> i32 {
    if !native(f) {
        return 0;
    }
    fixed_units(
        f,
        (*(*assembly).parts.offset(index as isize)).start_connector_length,
    )
}
#[no_mangle]
pub unsafe extern "C" fn ot_part_end_connector(
    f: i32,
    assembly: *const GlyphAssembly,
    index: i32,
) -> i32 {
    if !native(f) {
        return 0;
    }
    fixed_units(
        f,
        (*(*assembly).parts.offset(index as isize)).end_connector_length,
    )
}
#[no_mangle]
pub unsafe extern "C" fn ot_part_full_advance(
    f: i32,
    assembly: *const GlyphAssembly,
    index: i32,
) -> i32 {
    if !native(f) {
        return 0;
    }
    fixed_units(f, (*(*assembly).parts.offset(index as isize)).full_advance)
}
#[cfg(all(test, math_standalone))]
#[path = "opentype_math_tests.rs"]
mod tests;
