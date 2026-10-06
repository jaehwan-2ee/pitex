/****************************************************************************\
 Part of the XeTeX typesetting system
 Copyright (c) 1994-2008 by SIL International
 Copyright (c) 2009-2012 by Jonathan Kew
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
\****************************************************************************/

// Manual Rust translation of the layout algorithms from
// xetex/layout/xetex-XeTeXLayoutInterface.cpp. The opaque FontInst and
// FontMgr class access remains in a small C++ bridge.
// Original SHA-256: 0887e7bf3ce77c9aee272976f5c7d98ae87309f2e1d6332b9efdad32ad575239
use std::{
    collections::BTreeMap,
    ffi::{c_char, c_long, c_void, CStr},
    ptr,
};
type Handle = *mut c_void;
#[repr(C)]
#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct GlyphBox {
    pub x_min: f32,
    pub y_min: f32,
    pub x_max: f32,
    pub y_max: f32,
}
#[repr(C)]
#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Feature {
    tag: u32,
    value: u32,
    start: u32,
    end: u32,
}
#[repr(C)]
pub struct Layout {
    font: Handle,
    font_ref: Handle,
    script: u32,
    language: Handle,
    features: *mut Feature,
    shapers: *mut *mut c_char,
    free_shapers: bool,
    shaper: *mut c_char,
    n_features: i32,
    rgb: u32,
    extend: f32,
    slant: f32,
    embolden: f32,
    buffer: Handle,
}
#[repr(C)]
#[derive(Default)]
struct Properties {
    direction: u32,
    script: u32,
    language: Handle,
    reserved1: Handle,
    reserved2: Handle,
}
#[repr(C)]
struct GlyphInfo {
    glyph: u32,
    mask: u32,
    cluster: u32,
    var1: u32,
    var2: u32,
}
#[repr(C)]
struct GlyphPosition {
    x_advance: i32,
    y_advance: i32,
    x_offset: i32,
    y_offset: i32,
    var: u32,
}
extern "C" {
    fn free(pointer: Handle);
    fn calloc(count: usize, size: usize) -> Handle;
    fn realloc(pointer: Handle, size: usize) -> Handle;
    fn strdup(value: *const c_char) -> *mut c_char;
    fn abort() -> !;
    fn getReqEngine() -> c_char;
    fn deleteFont(font: Handle);
    fn getGlyphWidth(font: Handle, glyph: u32) -> f32;
    fn ttxl_font_units_to_points(font: Handle, units: f32) -> f32;
    fn ttxl_font_get_point_size(font: Handle) -> f32;
    fn pitex_font_units_to_points_double(font: Handle, units: f64) -> f32;
    fn pitex_font_hb(font: Handle) -> Handle;
    fn pitex_font_vertical(font: Handle) -> bool;
    fn pitex_font_metric(font: Handle, which: i32) -> f32;
    fn pitex_font_filename(font: Handle, index: *mut u32) -> *mut c_char;
    fn pitex_font_bounds(font: Handle, glyph: u32, bounds: *mut GlyphBox);
    fn pitex_font_height_depth(font: Handle, glyph: u32, height: *mut f32, depth: *mut f32);
    fn pitex_font_sidebearings(font: Handle, glyph: u32, left: *mut f32, right: *mut f32);
    fn pitex_font_italic_correction(font: Handle, glyph: u32) -> f32;
    fn pitex_font_map_character(font: Handle, character: u32) -> u32;
    fn pitex_font_map_glyph(font: Handle, name: *const c_char) -> i32;
    fn pitex_font_character_range(font: Handle, first: bool) -> i32;
    fn D2Fix(value: f64) -> i32;
    fn hb_font_get_face(font: Handle) -> Handle;
    fn hb_font_get_ptem(font: Handle) -> f32;
    fn hb_ot_layout_table_get_script_tags(
        face: Handle,
        table: u32,
        start: u32,
        count: *mut u32,
        tags: *mut u32,
    ) -> u32;
    fn hb_ot_layout_script_get_language_tags(
        face: Handle,
        table: u32,
        script: u32,
        start: u32,
        count: *mut u32,
        tags: *mut u32,
    ) -> u32;
    fn hb_ot_layout_table_find_script(
        face: Handle,
        table: u32,
        script: u32,
        index: *mut u32,
    ) -> i32;
    fn hb_ot_layout_script_select_language(
        face: Handle,
        table: u32,
        script: u32,
        count: u32,
        languages: *const u32,
        index: *mut u32,
    ) -> i32;
    fn hb_ot_layout_language_get_feature_tags(
        face: Handle,
        table: u32,
        script: u32,
        language: u32,
        start: u32,
        count: *mut u32,
        tags: *mut u32,
    ) -> u32;
    fn hb_language_from_string(language: *const c_char, length: i32) -> Handle;
    fn hb_language_to_string(language: Handle) -> *const c_char;
    fn hb_ot_tag_to_language(tag: u32) -> Handle;
    fn hb_tag_from_string(tag: *const c_char, length: i32) -> u32;
    fn hb_ot_tag_to_script(tag: u32) -> u32;
    fn hb_script_get_horizontal_direction(script: u32) -> u32;
    fn hb_buffer_create() -> Handle;
    fn hb_buffer_destroy(buffer: Handle);
    fn hb_buffer_reset(buffer: Handle);
    fn hb_buffer_add_utf16(buffer: Handle, text: *const u16, length: i32, offset: u32, count: i32);
    fn hb_buffer_set_direction(buffer: Handle, direction: u32);
    fn hb_buffer_set_script(buffer: Handle, script: u32);
    fn hb_buffer_get_script(buffer: Handle) -> u32;
    fn hb_buffer_set_language(buffer: Handle, language: Handle);
    fn hb_buffer_guess_segment_properties(buffer: Handle);
    fn hb_buffer_get_segment_properties(buffer: Handle, properties: *mut Properties);
    fn hb_buffer_set_content_type(buffer: Handle, content: u32);
    fn hb_buffer_get_length(buffer: Handle) -> u32;
    fn hb_buffer_get_glyph_infos(buffer: Handle, length: *mut u32) -> *mut GlyphInfo;
    fn hb_buffer_get_glyph_positions(buffer: Handle, length: *mut u32) -> *mut GlyphPosition;
    fn hb_shape_plan_create_cached(
        face: Handle,
        properties: *const Properties,
        features: *const Feature,
        count: u32,
        shapers: *const *mut c_char,
    ) -> Handle;
    fn hb_shape_plan_create(
        face: Handle,
        properties: *const Properties,
        features: *const Feature,
        count: u32,
        shapers: *const *mut c_char,
    ) -> Handle;
    fn hb_shape_plan_execute(
        plan: Handle,
        font: Handle,
        buffer: Handle,
        features: *const Feature,
        count: u32,
    ) -> i32;
    fn hb_shape_plan_get_shaper(plan: Handle) -> *const c_char;
    fn hb_shape_plan_destroy(plan: Handle);
    fn hb_ot_math_has_data(face: Handle) -> i32;
    fn hb_graphite2_face_get_gr_face(face: Handle) -> Handle;
    fn gr_face_n_fref(face: Handle) -> u16;
    fn gr_face_fref(face: Handle, index: u16) -> Handle;
    fn gr_face_find_fref(face: Handle, id: u32) -> Handle;
    fn gr_face_featureval_for_lang(face: Handle, language: u32) -> Handle;
    fn gr_fref_id(feature: Handle) -> u32;
    fn gr_fref_n_values(feature: Handle) -> u16;
    fn gr_fref_value(feature: Handle, index: u16) -> i16;
    fn gr_fref_feature_value(feature: Handle, values: Handle) -> u16;
    fn gr_fref_set_feature_value(feature: Handle, value: u16, values: Handle) -> i32;
    fn gr_fref_label(
        feature: Handle,
        language: *mut u16,
        encoding: u32,
        length: *mut u32,
    ) -> Handle;
    fn gr_fref_value_label(
        feature: Handle,
        index: u16,
        language: *mut u16,
        encoding: u32,
        length: *mut u32,
    ) -> Handle;
    fn gr_label_destroy(label: Handle);
    fn gr_featureval_destroy(values: Handle);
    fn gr_make_font(size: f32, face: Handle) -> Handle;
    fn gr_font_destroy(font: Handle);
    fn gr_make_seg(
        font: Handle,
        face: Handle,
        script: u32,
        features: Handle,
        encoding: u32,
        text: *const u16,
        count: usize,
        direction: i32,
    ) -> Handle;
    fn gr_seg_destroy(segment: Handle);
    fn gr_seg_first_slot(segment: Handle) -> Handle;
    fn gr_seg_last_slot(segment: Handle) -> Handle;
    fn gr_slot_next_in_segment(slot: Handle) -> Handle;
    fn gr_slot_index(slot: Handle) -> u32;
    fn gr_seg_cinfo(segment: Handle, index: u32) -> Handle;
    fn gr_cinfo_break_weight(info: Handle) -> i32;
    fn gr_cinfo_base(info: Handle) -> usize;
}
static mut BOXES: Option<BTreeMap<u32, GlyphBox>> = None;
static mut PROTRUSION: Option<BTreeMap<(i32, u32, i32), i32>> = None;
#[no_mangle]
pub unsafe extern "C" fn getCachedGlyphBBox(font: u16, glyph: u16, bounds: *mut GlyphBox) -> i32 {
    if let Some(value) = BOXES
        .as_ref()
        .and_then(|boxes| boxes.get(&((font as u32) << 16 | glyph as u32)))
    {
        *bounds = *value;
        1
    } else {
        0
    }
}
#[no_mangle]
pub unsafe extern "C" fn cacheGlyphBBox(font: u16, glyph: u16, bounds: *const GlyphBox) {
    BOXES
        .get_or_insert_with(BTreeMap::new)
        .insert((font as u32) << 16 | glyph as u32, *bounds);
}
#[no_mangle]
pub unsafe extern "C" fn set_cp_code(font: i32, code: u32, side: i32, value: i32) {
    if !matches!(side, 0 | 1) {
        abort();
    }
    PROTRUSION
        .get_or_insert_with(BTreeMap::new)
        .insert((font, code, side), value);
}
#[no_mangle]
pub unsafe extern "C" fn get_cp_code(font: i32, code: u32, side: i32) -> i32 {
    if !matches!(side, 0 | 1) {
        abort();
    }
    PROTRUSION
        .as_ref()
        .and_then(|codes| codes.get(&(font, code, side)))
        .copied()
        .unwrap_or(0)
}
#[no_mangle]
pub unsafe extern "C" fn getSlant(font: Handle) -> i32 {
    D2Fix((-pitex_font_metric(font, 4) as f64 * std::f64::consts::PI / 180.).tan())
}
const GSUB: u32 = u32::from_be_bytes(*b"GSUB");
const GPOS: u32 = u32::from_be_bytes(*b"GPOS");
unsafe fn face(font: Handle) -> Handle {
    hb_font_get_face(pitex_font_hb(font))
}
unsafe fn scripts(font: Handle) -> Vec<u32> {
    // Preserve the original selection: both arrays were populated from GSUB,
    // including the one whose initial capacity came from GPOS. The full GSUB
    // array therefore always wins (or is equal); RAII avoids those old leaks.
    let face = face(font);
    let mut count =
        hb_ot_layout_table_get_script_tags(face, GSUB, 0, ptr::null_mut(), ptr::null_mut());
    let mut tags = vec![0; count as usize];
    hb_ot_layout_table_get_script_tags(face, GSUB, 0, &mut count, tags.as_mut_ptr());
    tags.truncate(count as usize);
    tags
}
#[no_mangle]
pub unsafe extern "C" fn countScripts(font: Handle) -> u32 {
    scripts(font).len() as u32
}
#[no_mangle]
pub unsafe extern "C" fn getIndScript(font: Handle, index: u32) -> u32 {
    scripts(font).get(index as usize).copied().unwrap_or(0)
}
#[no_mangle]
pub unsafe extern "C" fn countLanguages(font: Handle, script: u32) -> u32 {
    let face = face(font);
    if let Some(index) = scripts(font).iter().position(|tag| *tag == script) {
        [GSUB, GPOS]
            .iter()
            .map(|table| {
                hb_ot_layout_script_get_language_tags(
                    face,
                    *table,
                    index as u32,
                    0,
                    ptr::null_mut(),
                    ptr::null_mut(),
                )
            })
            .sum()
    } else {
        0
    }
}
#[no_mangle]
pub unsafe extern "C" fn getIndLanguage(font: Handle, script: u32, index: u32) -> u32 {
    let face = face(font);
    if let Some(script) = scripts(font).iter().position(|tag| *tag == script) {
        for table in [GSUB, GPOS] {
            let mut count = hb_ot_layout_script_get_language_tags(
                face,
                table,
                script as u32,
                0,
                ptr::null_mut(),
                ptr::null_mut(),
            );
            let mut tags = vec![0; count as usize];
            hb_ot_layout_script_get_language_tags(
                face,
                table,
                script as u32,
                0,
                &mut count,
                tags.as_mut_ptr(),
            );
            if let Some(value) = tags.get(index as usize) {
                return *value;
            }
        }
    }
    0
}
unsafe fn features(font: Handle, script: u32, language: u32) -> Vec<u32> {
    let face = face(font);
    let mut result = Vec::new();
    for table in [GSUB, GPOS] {
        let (mut script_index, mut language_index) = (0, 0);
        if hb_ot_layout_table_find_script(face, table, script, &mut script_index) != 0
            && (hb_ot_layout_script_select_language(
                face,
                table,
                script_index,
                1,
                &language,
                &mut language_index,
            ) != 0
                || language == 0)
        {
            let mut count = hb_ot_layout_language_get_feature_tags(
                face,
                table,
                script_index,
                language_index,
                0,
                ptr::null_mut(),
                ptr::null_mut(),
            );
            let start = result.len();
            result.resize(start + count as usize, 0);
            hb_ot_layout_language_get_feature_tags(
                face,
                table,
                script_index,
                language_index,
                0,
                &mut count,
                result.as_mut_ptr().add(start),
            );
            result.truncate(start + count as usize);
        }
    }
    result
}
#[no_mangle]
pub unsafe extern "C" fn countFeatures(font: Handle, script: u32, language: u32) -> u32 {
    features(font, script, language).len() as u32
}
#[no_mangle]
pub unsafe extern "C" fn getIndFeature(
    font: Handle,
    script: u32,
    language: u32,
    index: u32,
) -> u32 {
    features(font, script, language)
        .get(index as usize)
        .copied()
        .unwrap_or(0)
}
#[no_mangle]
pub unsafe extern "C" fn getFont(engine: *mut Layout) -> Handle {
    (*engine).font
}
#[no_mangle]
pub unsafe extern "C" fn getFontRef(engine: *mut Layout) -> Handle {
    (*engine).font_ref
}
#[no_mangle]
pub unsafe extern "C" fn getFontFilename(engine: *mut Layout, index: *mut u32) -> *mut c_char {
    pitex_font_filename((*engine).font, index)
}
#[no_mangle]
pub unsafe extern "C" fn getExtendFactor(engine: *mut Layout) -> f32 {
    (*engine).extend
}
#[no_mangle]
pub unsafe extern "C" fn getSlantFactor(engine: *mut Layout) -> f32 {
    (*engine).slant
}
#[no_mangle]
pub unsafe extern "C" fn getEmboldenFactor(engine: *mut Layout) -> f32 {
    (*engine).embolden
}
#[no_mangle]
pub unsafe extern "C" fn ttxl_get_hb_font(engine: *mut Layout) -> Handle {
    pitex_font_hb((*engine).font)
}
#[no_mangle]
pub unsafe extern "C" fn createLayoutEngine(
    font_ref: Handle,
    font: Handle,
    script: u32,
    language: *mut c_char,
    features: *mut Feature,
    count: i32,
    shapers: *mut *mut c_char,
    rgb: u32,
    extend: f32,
    slant: f32,
    embolden: f32,
) -> *mut Layout {
    let selected_language = if getReqEngine() == b'G' as c_char {
        hb_language_from_string(language, -1)
    } else {
        hb_ot_tag_to_language(hb_tag_from_string(language, -1))
    };
    free(language.cast());
    Box::into_raw(Box::new(Layout {
        font,
        font_ref,
        script,
        language: selected_language,
        features,
        shapers,
        free_shapers: false,
        shaper: ptr::null_mut(),
        n_features: count,
        rgb,
        extend,
        slant,
        embolden,
        buffer: hb_buffer_create(),
    }))
}
#[no_mangle]
pub unsafe extern "C" fn deleteLayoutEngine(engine: *mut Layout) {
    let engine = Box::from_raw(engine);
    hb_buffer_destroy(engine.buffer);
    deleteFont(engine.font);
    free(engine.shaper.cast());
    if engine.free_shapers {
        free(engine.shapers.cast());
    }
}
// Original Pitex feature selection, preserved from the compatibility hook:
// explicit zero overrides follow the user's fontspec feature selections.
#[no_mangle]
pub unsafe extern "C" fn pitex_disable_native_ligatures(engine: *mut Layout) {
    let count = (*engine).n_features;
    let array = realloc(
        (*engine).features.cast(),
        (count as usize + 5) * std::mem::size_of::<Feature>(),
    )
    .cast::<Feature>();
    if array.is_null() {
        abort();
    }
    (*engine).features = array;
    for (index, tag) in [*b"liga", *b"clig", *b"dlig", *b"hlig", *b"kern"]
        .iter()
        .enumerate()
    {
        *array.add(count as usize + index) = Feature {
            tag: u32::from_be_bytes(*tag),
            value: 0,
            start: 0,
            end: u32::MAX,
        };
    }
    (*engine).n_features = count + 5;
}
#[no_mangle]
pub unsafe extern "C" fn layoutChars(
    engine: *mut Layout,
    text: *mut u16,
    offset: i32,
    count: i32,
    max: i32,
    right_to_left: bool,
) -> i32 {
    let engine = &mut *engine;
    let font = pitex_font_hb(engine.font);
    let face = hb_font_get_face(font);
    let direction = if pitex_font_vertical(engine.font) {
        6
    } else if right_to_left {
        5
    } else {
        4
    };
    hb_buffer_reset(engine.buffer);
    hb_buffer_add_utf16(engine.buffer, text, max, offset as u32, count);
    hb_buffer_set_direction(engine.buffer, direction);
    hb_buffer_set_script(engine.buffer, hb_ot_tag_to_script(engine.script));
    hb_buffer_set_language(engine.buffer, engine.language);
    hb_buffer_guess_segment_properties(engine.buffer);
    let mut properties = Properties::default();
    hb_buffer_get_segment_properties(engine.buffer, &mut properties);
    if engine.shapers.is_null() {
        engine.shapers = calloc(2, std::mem::size_of::<*mut c_char>()).cast();
        if engine.shapers.is_null() {
            abort();
        }
        *engine.shapers = b"ot\0".as_ptr().cast_mut().cast();
        engine.free_shapers = true;
    }
    let mut plan = hb_shape_plan_create_cached(
        face,
        &properties,
        engine.features,
        engine.n_features as u32,
        engine.shapers,
    );
    let mut shaped = hb_shape_plan_execute(
        plan,
        font,
        engine.buffer,
        engine.features,
        engine.n_features as u32,
    ) != 0;
    if !engine.shaper.is_null() {
        free(engine.shaper.cast());
        engine.shaper = ptr::null_mut();
    }
    if !shaped {
        hb_shape_plan_destroy(plan);
        plan = hb_shape_plan_create(
            face,
            &properties,
            engine.features,
            engine.n_features as u32,
            ptr::null(),
        );
        shaped = hb_shape_plan_execute(
            plan,
            font,
            engine.buffer,
            engine.features,
            engine.n_features as u32,
        ) != 0;
    }
    if !shaped {
        abort();
    }
    engine.shaper = strdup(hb_shape_plan_get_shaper(plan));
    hb_buffer_set_content_type(engine.buffer, 2);
    hb_shape_plan_destroy(plan);
    hb_buffer_get_length(engine.buffer) as i32
}
#[no_mangle]
pub unsafe extern "C" fn getGlyphs(engine: *mut Layout, glyphs: *mut u32) {
    let count = hb_buffer_get_length((*engine).buffer);
    let info = hb_buffer_get_glyph_infos((*engine).buffer, ptr::null_mut());
    for index in 0..count as usize {
        *glyphs.add(index) = (*info.add(index)).glyph;
    }
}
#[no_mangle]
pub unsafe extern "C" fn getGlyphAdvances(engine: *mut Layout, advances: *mut f32) {
    let font = (*engine).font;
    let vertical = pitex_font_vertical(font);
    let count = hb_buffer_get_length((*engine).buffer);
    let positions = hb_buffer_get_glyph_positions((*engine).buffer, ptr::null_mut());
    for index in 0..count as usize {
        let position = &*positions.add(index);
        *advances.add(index) = pitex_font_units_to_points_double(
            font,
            if vertical {
                position.y_advance
            } else {
                position.x_advance
            } as f64,
        );
    }
}
#[no_mangle]
pub unsafe extern "C" fn getGlyphPositions(engine: *mut Layout, positions: *mut Point) {
    let engine = &*engine;
    let count = hb_buffer_get_length(engine.buffer) as usize;
    let raw = hb_buffer_get_glyph_positions(engine.buffer, ptr::null_mut());
    let vertical = pitex_font_vertical(engine.font);
    let (mut x, mut y) = (0f32, 0f32);
    for index in 0..count {
        let glyph = &*raw.add(index);
        let out = &mut *positions.add(index);
        if vertical {
            out.x = -ttxl_font_units_to_points(engine.font, x + glyph.y_offset as f32);
            out.y = ttxl_font_units_to_points(engine.font, y - glyph.x_offset as f32);
            x += glyph.y_advance as f32;
            y += glyph.x_advance as f32;
        } else {
            out.x = ttxl_font_units_to_points(engine.font, x + glyph.x_offset as f32);
            out.y = -ttxl_font_units_to_points(engine.font, y + glyph.y_offset as f32);
            x += glyph.x_advance as f32;
            y += glyph.y_advance as f32;
        }
    }
    *positions.add(count) = Point {
        x: if vertical {
            -ttxl_font_units_to_points(engine.font, x)
        } else {
            ttxl_font_units_to_points(engine.font, x)
        },
        y: if vertical {
            ttxl_font_units_to_points(engine.font, y)
        } else {
            -ttxl_font_units_to_points(engine.font, y)
        },
    };
    if engine.extend != 1. || engine.slant != 0. {
        for index in 0..=count {
            let position = &mut *positions.add(index);
            position.x = position.x * engine.extend - position.y * engine.slant;
        }
    }
}
#[no_mangle]
pub unsafe extern "C" fn getPointSize(engine: *mut Layout) -> f32 {
    ttxl_font_get_point_size((*engine).font)
}
#[no_mangle]
pub unsafe extern "C" fn getAscentAndDescent(
    engine: *mut Layout,
    ascent: *mut f32,
    descent: *mut f32,
) {
    *ascent = pitex_font_metric((*engine).font, 0);
    *descent = pitex_font_metric((*engine).font, 1);
}
#[no_mangle]
pub unsafe extern "C" fn getCapAndXHeight(engine: *mut Layout, cap: *mut f32, x: *mut f32) {
    *cap = pitex_font_metric((*engine).font, 2);
    *x = pitex_font_metric((*engine).font, 3);
}
#[no_mangle]
pub unsafe extern "C" fn getDefaultDirection(engine: *mut Layout) -> i32 {
    if hb_script_get_horizontal_direction(hb_buffer_get_script((*engine).buffer)) == 5 {
        0xff
    } else {
        0xfe
    }
}
#[no_mangle]
pub unsafe extern "C" fn getRgbValue(engine: *mut Layout) -> u32 {
    (*engine).rgb
}
#[no_mangle]
pub unsafe extern "C" fn getGlyphBounds(engine: *mut Layout, glyph: u32, bounds: *mut GlyphBox) {
    pitex_font_bounds((*engine).font, glyph, bounds);
    if (*engine).extend != 0. {
        (*bounds).x_min *= (*engine).extend;
        (*bounds).x_max *= (*engine).extend;
    }
}
#[no_mangle]
pub unsafe extern "C" fn getGlyphWidthFromEngine(engine: *mut Layout, glyph: u32) -> f32 {
    (*engine).extend * getGlyphWidth((*engine).font, glyph)
}
#[no_mangle]
pub unsafe extern "C" fn getGlyphHeightDepth(
    engine: *mut Layout,
    glyph: u32,
    height: *mut f32,
    depth: *mut f32,
) {
    pitex_font_height_depth((*engine).font, glyph, height, depth)
}
#[no_mangle]
pub unsafe extern "C" fn getGlyphSidebearings(
    engine: *mut Layout,
    glyph: u32,
    left: *mut f32,
    right: *mut f32,
) {
    pitex_font_sidebearings((*engine).font, glyph, left, right);
    if (*engine).extend != 0. {
        *left *= (*engine).extend;
        *right *= (*engine).extend;
    }
}
#[no_mangle]
pub unsafe extern "C" fn getGlyphItalCorr(engine: *mut Layout, glyph: u32) -> f32 {
    (*engine).extend * pitex_font_italic_correction((*engine).font, glyph)
}
#[no_mangle]
pub unsafe extern "C" fn mapCharToGlyph(engine: *mut Layout, character: u32) -> u32 {
    pitex_font_map_character((*engine).font, character)
}
#[no_mangle]
pub unsafe extern "C" fn mapGlyphToIndex(engine: *mut Layout, name: *const c_char) -> i32 {
    pitex_font_map_glyph((*engine).font, name)
}
#[no_mangle]
pub unsafe extern "C" fn getFontCharRange(engine: *mut Layout, first: i32) -> i32 {
    pitex_font_character_range((*engine).font, first != 0)
}
#[no_mangle]
pub unsafe extern "C" fn usingGraphite(engine: *mut Layout) -> bool {
    !(*engine).shaper.is_null() && CStr::from_ptr((*engine).shaper).to_bytes() == b"graphite2"
}
#[no_mangle]
pub unsafe extern "C" fn usingOpenType(engine: *mut Layout) -> bool {
    (*engine).shaper.is_null() || CStr::from_ptr((*engine).shaper).to_bytes() == b"ot"
}
#[no_mangle]
pub unsafe extern "C" fn isOpenTypeMathFont(engine: *mut Layout) -> bool {
    hb_ot_math_has_data(face((*engine).font)) != 0
}
unsafe fn graphite(engine: *mut Layout) -> Handle {
    hb_graphite2_face_get_gr_face(face((*engine).font))
}
unsafe fn language_tag(language: Handle) -> u32 {
    let name = hb_language_to_string(language);
    if name.is_null() {
        0
    } else {
        hb_tag_from_string(name, CStr::from_ptr(name).to_bytes().len() as i32)
    }
}
#[no_mangle]
pub unsafe extern "C" fn countGraphiteFeatures(engine: *mut Layout) -> u32 {
    let face = graphite(engine);
    if face.is_null() {
        0
    } else {
        gr_face_n_fref(face) as u32
    }
}
#[no_mangle]
pub unsafe extern "C" fn getGraphiteFeatureCode(engine: *mut Layout, index: u32) -> u32 {
    let face = graphite(engine);
    if face.is_null() {
        0
    } else {
        gr_fref_id(gr_face_fref(face, index as u16))
    }
}
#[no_mangle]
pub unsafe extern "C" fn countGraphiteFeatureSettings(engine: *mut Layout, id: u32) -> u32 {
    let face = graphite(engine);
    if face.is_null() {
        0
    } else {
        gr_fref_n_values(gr_face_find_fref(face, id)) as u32
    }
}
#[no_mangle]
pub unsafe extern "C" fn getGraphiteFeatureSettingCode(
    engine: *mut Layout,
    id: u32,
    index: u32,
) -> u32 {
    let face = graphite(engine);
    if face.is_null() {
        0
    } else {
        gr_fref_value(gr_face_find_fref(face, id), index as u16) as i32 as u32
    }
}
#[no_mangle]
pub unsafe extern "C" fn getGraphiteFeatureDefaultSetting(engine: *mut Layout, id: u32) -> u32 {
    let face = graphite(engine);
    if face.is_null() {
        0
    } else {
        let values = gr_face_featureval_for_lang(face, language_tag((*engine).language));
        let result = gr_fref_feature_value(gr_face_find_fref(face, id), values) as u32;
        gr_featureval_destroy(values);
        result
    }
}
#[no_mangle]
pub unsafe extern "C" fn getGraphiteFeatureLabel(engine: *mut Layout, id: u32) -> *mut c_char {
    let face = graphite(engine);
    if face.is_null() {
        ptr::null_mut()
    } else {
        let (mut language, mut length) = (0x409, 0);
        gr_fref_label(gr_face_find_fref(face, id), &mut language, 1, &mut length).cast()
    }
}
#[no_mangle]
pub unsafe extern "C" fn getGraphiteFeatureSettingLabel(
    engine: *mut Layout,
    id: u32,
    setting: u32,
) -> *mut c_char {
    let face = graphite(engine);
    if !face.is_null() {
        let feature = gr_face_find_fref(face, id);
        for index in 0..gr_fref_n_values(feature) {
            if gr_fref_value(feature, index) as i32 == setting as i32 {
                let (mut language, mut length) = (0x409, 0);
                return gr_fref_value_label(feature, index, &mut language, 1, &mut length).cast();
            }
        }
    }
    ptr::null_mut()
}
unsafe fn prefix(label: *const c_char, name: *const c_char, length: i32) -> bool {
    if label.is_null() {
        return false;
    }
    let label = CStr::from_ptr(label).to_bytes();
    let name = std::slice::from_raw_parts(name.cast::<u8>(), length.max(0) as usize);
    label.starts_with(name)
}
#[no_mangle]
pub unsafe extern "C" fn findGraphiteFeatureNamed(
    engine: *mut Layout,
    name: *const c_char,
    length: i32,
) -> c_long {
    let face = graphite(engine);
    if !face.is_null() {
        let tag = hb_tag_from_string(name, length);
        for index in 0..gr_face_n_fref(face) {
            let feature = gr_face_fref(face, index);
            let (mut language, mut bytes) = (0x409, 0);
            let label = gr_fref_label(feature, &mut language, 1, &mut bytes);
            let matches = gr_fref_id(feature) == tag || prefix(label.cast(), name, length);
            gr_label_destroy(label);
            if matches {
                return gr_fref_id(feature) as c_long;
            }
        }
    }
    -1
}
#[no_mangle]
pub unsafe extern "C" fn findGraphiteFeatureSettingNamed(
    engine: *mut Layout,
    id: u32,
    name: *const c_char,
    length: i32,
) -> c_long {
    let face = graphite(engine);
    if !face.is_null() {
        let tag = hb_tag_from_string(name, length);
        let feature = gr_face_find_fref(face, id);
        for index in 0..gr_fref_n_values(feature) {
            let (mut language, mut bytes) = (0x409, 0);
            let label = gr_fref_value_label(feature, index, &mut language, 1, &mut bytes);
            let matches = gr_fref_id(feature) == tag || prefix(label.cast(), name, length);
            gr_label_destroy(label);
            if matches {
                return gr_fref_value(feature, index) as c_long;
            }
        }
    }
    -1
}
#[no_mangle]
pub unsafe extern "C" fn findGraphiteFeature(
    engine: *mut Layout,
    start: *const c_char,
    end: *const c_char,
    feature: *mut u32,
    value: *mut i32,
) -> bool {
    *feature = 0;
    *value = 0;
    let mut start = start;
    while start < end && matches!(*start as u8, b' ' | b'\t') {
        start = start.add(1);
    }
    let mut setting = start;
    while setting < end && *setting != b'=' as c_char {
        setting = setting.add(1);
    }
    let id = findGraphiteFeatureNamed(engine, start, setting.offset_from(start) as i32);
    *feature = id as u32;
    if id == -1 || setting == end {
        return false;
    }
    setting = setting.add(1);
    while setting < end && matches!(*setting as u8, b' ' | b'\t') {
        setting = setting.add(1);
    }
    if setting == end {
        return false;
    }
    *value =
        findGraphiteFeatureSettingNamed(engine, *feature, setting, end.offset_from(setting) as i32)
            as i32;
    *value != -1
}
static mut GR_SEGMENT: Handle = ptr::null_mut();
static mut GR_PREVIOUS: Handle = ptr::null_mut();
static mut GR_TEXT_LENGTH: i32 = 0;
#[no_mangle]
pub unsafe extern "C" fn initGraphiteBreaking(
    engine: *mut Layout,
    text: *const u16,
    length: i32,
) -> bool {
    let face = graphite(engine);
    if face.is_null() {
        return false;
    }
    let font = gr_make_font(hb_font_get_ptem(ttxl_get_hb_font(engine)), face);
    if font.is_null() {
        return false;
    }
    if !GR_SEGMENT.is_null() {
        gr_seg_destroy(GR_SEGMENT);
        GR_SEGMENT = ptr::null_mut();
        GR_PREVIOUS = ptr::null_mut();
    }
    let values = gr_face_featureval_for_lang(face, language_tag((*engine).language));
    for index in 0..(*engine).n_features.max(0) as usize {
        let feature = &*(*engine).features.add(index);
        let reference = gr_face_find_fref(face, feature.tag);
        if !reference.is_null() {
            gr_fref_set_feature_value(reference, feature.value as u16, values);
        }
    }
    GR_SEGMENT = gr_make_seg(
        font,
        face,
        (*engine).script,
        values,
        2,
        text,
        length.max(0) as usize,
        0,
    );
    gr_featureval_destroy(values);
    gr_font_destroy(font);
    if GR_SEGMENT.is_null() {
        return false;
    }
    GR_PREVIOUS = gr_seg_first_slot(GR_SEGMENT);
    GR_TEXT_LENGTH = length;
    true
}
#[no_mangle]
pub unsafe extern "C" fn findNextGraphiteBreak() -> i32 {
    if GR_SEGMENT.is_null() || GR_PREVIOUS.is_null() || GR_PREVIOUS == gr_seg_last_slot(GR_SEGMENT)
    {
        return -1;
    }
    let mut slot = gr_slot_next_in_segment(GR_PREVIOUS);
    while !slot.is_null() {
        let info = gr_seg_cinfo(GR_SEGMENT, gr_slot_index(slot));
        let weight = gr_cinfo_break_weight(info);
        if (-15..0).contains(&weight) {
            GR_PREVIOUS = slot;
            return gr_cinfo_base(info) as i32;
        } else if (1..=15).contains(&weight) {
            GR_PREVIOUS = gr_slot_next_in_segment(slot);
            return gr_cinfo_base(info) as i32 + 1;
        }
        slot = gr_slot_next_in_segment(slot);
    }
    GR_PREVIOUS = gr_seg_last_slot(GR_SEGMENT);
    GR_TEXT_LENGTH
}
