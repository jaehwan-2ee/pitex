/****************************************************************************\
 Part of the XeTeX typesetting system
 Copyright (c) 1994-2008 by SIL International
 Copyright (c) 2009 by Jonathan Kew

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

// Manual Rust translation of xetex/layout/xetex-XeTeXFontInst.cpp.
// Original SHA-256: d6487df22bb4e62e3cbe97e004c539e6abad09f00eb3f320bfbfccdca0d8a315
// Only class lifetime and SDK record field access remain in C++.
use std::{
    ffi::{c_char, c_long, c_ulong, c_void, CStr, CString},
    ptr,
};
pub type Handle = *mut c_void;
#[repr(C)]
#[derive(Default)]
pub struct FaceInfo {
    pub flags: c_long,
    pub num_faces: c_long,
    pub num_glyphs: c_long,
    pub units_per_em: u32,
    pub ascender: i32,
    pub descender: i32,
    pub post_present: i32,
    pub italic_angle: i32,
    pub os2_present: i32,
    pub cap_height: i32,
    pub x_height: i32,
}
#[repr(C)]
#[derive(Default)]
struct GlyphInfo {
    slot: Handle,
    x_bearing: c_long,
    y_bearing: c_long,
    width: c_long,
    height: c_long,
    format: u32,
    point_count: i32,
    points: *const Vector,
}
#[repr(C)]
#[derive(Default)]
pub struct FontInit {
    face: Handle,
    data: Handle,
    afm_data: Handle,
    hb_font: Handle,
    filename: *mut c_char,
    index: u32,
    units_per_em: u32,
    ascent: f32,
    descent: f32,
    cap_height: f32,
    x_height: f32,
    italic_angle: f32,
}
#[repr(C)]
#[derive(Default)]
struct Vector {
    x: c_long,
    y: c_long,
}
#[repr(C)]
#[derive(Default)]
struct Bounds {
    x_min: c_long,
    y_min: c_long,
    x_max: c_long,
    y_max: c_long,
}
#[repr(C)]
#[derive(Default)]
pub struct GlyphBox {
    x_min: f32,
    y_min: f32,
    x_max: f32,
    y_max: f32,
}
#[repr(C)]
struct Extents {
    x_bearing: i32,
    y_bearing: i32,
    width: i32,
    height: i32,
}
type Destroy = Option<unsafe extern "C" fn(Handle)>;
type Nominal = Option<unsafe extern "C" fn(Handle, Handle, u32, *mut u32, Handle) -> i32>;
type Variation = Option<unsafe extern "C" fn(Handle, Handle, u32, u32, *mut u32, Handle) -> i32>;
type Advance = Option<unsafe extern "C" fn(Handle, Handle, u32, Handle) -> i32>;
type Origin = Option<unsafe extern "C" fn(Handle, Handle, u32, *mut i32, *mut i32, Handle) -> i32>;
type Kerning = Option<unsafe extern "C" fn(Handle, Handle, u32, u32, Handle) -> i32>;
type GlyphExtents = Option<unsafe extern "C" fn(Handle, Handle, u32, *mut Extents, Handle) -> i32>;
type Contour =
    Option<unsafe extern "C" fn(Handle, Handle, u32, u32, *mut i32, *mut i32, Handle) -> i32>;
type GlyphName = Option<unsafe extern "C" fn(Handle, Handle, u32, *mut c_char, u32, Handle) -> i32>;
extern "C" {
    fn malloc(size: usize) -> Handle;
    fn free(value: Handle);
    fn strdup(value: *const c_char) -> *mut c_char;
    fn strlen(value: *const c_char) -> usize;
    fn _tt_abort(message: *const c_char, ...) -> !;
    fn ttstub_input_open(path: *const c_char, format: u32, gzip: i32) -> Handle;
    fn ttstub_input_get_size(input: Handle) -> usize;
    fn ttstub_input_read(input: Handle, data: *mut c_char, length: usize) -> isize;
    fn ttstub_input_close(input: Handle) -> i32;
    fn FT_Init_FreeType(library: *mut Handle) -> i32;
    fn FT_New_Memory_Face(
        library: Handle,
        data: *const u8,
        length: c_long,
        index: c_long,
        face: *mut Handle,
    ) -> i32;
    fn FT_New_Face(library: Handle, path: *const c_char, index: c_long, face: *mut Handle) -> i32;
    fn FT_Done_Face(face: Handle) -> i32;
    fn FT_Get_Postscript_Name(face: Handle) -> *const c_char;
    fn FT_Load_Sfnt_Table(
        face: Handle,
        tag: c_ulong,
        offset: c_long,
        data: *mut u8,
        length: *mut c_ulong,
    ) -> i32;
    fn FT_Get_Char_Index(face: Handle, character: c_ulong) -> u32;
    fn FT_Face_GetCharVariantIndex(face: Handle, character: c_ulong, selector: c_ulong) -> u32;
    fn FT_Get_Advance(face: Handle, glyph: u32, flags: i32, advance: *mut c_long) -> i32;
    fn FT_Get_Kerning(face: Handle, left: u32, right: u32, mode: u32, vector: *mut Vector) -> i32;
    fn FT_Load_Glyph(face: Handle, glyph: u32, flags: i32) -> i32;
    fn FT_Get_Glyph(slot: Handle, glyph: *mut Handle) -> i32;
    fn FT_Glyph_Get_CBox(glyph: Handle, mode: u32, bounds: *mut Bounds);
    fn FT_Done_Glyph(glyph: Handle);
    fn FT_Get_Glyph_Name(face: Handle, glyph: u32, name: *mut c_char, size: u32) -> i32;
    pub fn FT_Get_First_Char(face: Handle, glyph: *mut u32) -> c_ulong;
    pub fn FT_Get_Next_Char(face: Handle, character: c_ulong, glyph: *mut u32) -> c_ulong;
    pub fn pitex_ft_face_info(face: Handle, info: *mut FaceInfo);
    fn pitex_ft_loaded_glyph(face: Handle, info: *mut GlyphInfo);
    fn pitex_ft_attach_memory(face: Handle, data: Handle, length: c_ulong);
    fn hb_blob_create(
        data: *const c_char,
        length: u32,
        mode: u32,
        user: Handle,
        destroy: Destroy,
    ) -> Handle;
    fn hb_face_create_for_tables(
        callback: Option<unsafe extern "C" fn(Handle, u32, Handle) -> Handle>,
        data: Handle,
        destroy: Destroy,
    ) -> Handle;
    fn hb_face_set_index(face: Handle, index: u32);
    fn hb_face_set_upem(face: Handle, units: u32);
    fn hb_face_destroy(face: Handle);
    fn hb_font_create(face: Handle) -> Handle;
    fn hb_font_funcs_create() -> Handle;
    fn hb_font_set_funcs(font: Handle, funcs: Handle, data: Handle, destroy: Destroy);
    fn hb_font_set_scale(font: Handle, x: i32, y: i32);
    fn hb_font_set_ppem(font: Handle, x: u32, y: u32);
    fn hb_font_funcs_set_nominal_glyph_func(
        funcs: Handle,
        callback: Nominal,
        data: Handle,
        destroy: Destroy,
    );
    fn hb_font_funcs_set_variation_glyph_func(
        funcs: Handle,
        callback: Variation,
        data: Handle,
        destroy: Destroy,
    );
    fn hb_font_funcs_set_glyph_h_advance_func(
        funcs: Handle,
        callback: Advance,
        data: Handle,
        destroy: Destroy,
    );
    fn hb_font_funcs_set_glyph_v_advance_func(
        funcs: Handle,
        callback: Advance,
        data: Handle,
        destroy: Destroy,
    );
    fn hb_font_funcs_set_glyph_h_origin_func(
        funcs: Handle,
        callback: Origin,
        data: Handle,
        destroy: Destroy,
    );
    fn hb_font_funcs_set_glyph_v_origin_func(
        funcs: Handle,
        callback: Origin,
        data: Handle,
        destroy: Destroy,
    );
    fn hb_font_funcs_set_glyph_h_kerning_func(
        funcs: Handle,
        callback: Kerning,
        data: Handle,
        destroy: Destroy,
    );
    fn hb_font_funcs_set_glyph_v_kerning_func(
        funcs: Handle,
        callback: Kerning,
        data: Handle,
        destroy: Destroy,
    );
    fn hb_font_funcs_set_glyph_extents_func(
        funcs: Handle,
        callback: GlyphExtents,
        data: Handle,
        destroy: Destroy,
    );
    fn hb_font_funcs_set_glyph_contour_point_func(
        funcs: Handle,
        callback: Contour,
        data: Handle,
        destroy: Destroy,
    );
    fn hb_font_funcs_set_glyph_name_func(
        funcs: Handle,
        callback: GlyphName,
        data: Handle,
        destroy: Destroy,
    );
}
#[no_mangle]
pub static mut gFreeTypeLibrary: Handle = ptr::null_mut();
static mut FONT_FUNCTIONS: Handle = ptr::null_mut();
pub unsafe fn ensure_library() -> Handle {
    if gFreeTypeLibrary.is_null() {
        let error = FT_Init_FreeType(ptr::addr_of_mut!(gFreeTypeLibrary));
        if error != 0 {
            _tt_abort(
                b"FreeType initialization failed, error %d\0"
                    .as_ptr()
                    .cast(),
                error,
            );
        }
    }
    gFreeTypeLibrary
}
pub unsafe fn collection_index(path: *const c_char, postscript: *const c_char) -> Option<u32> {
    let library = ensure_library();
    let mut face = ptr::null_mut();
    if FT_New_Face(library, path, 0, &mut face) != 0 {
        return Some(0);
    }
    let mut info = FaceInfo::default();
    pitex_ft_face_info(face, &mut info);
    FT_Done_Face(face);
    if info.num_faces <= 1 {
        return Some(0);
    }
    if postscript.is_null() {
        return None;
    }
    let target = CStr::from_ptr(postscript).to_bytes();
    for index in 0..info.num_faces {
        let mut face = ptr::null_mut();
        if FT_New_Face(library, path, index, &mut face) != 0 {
            continue;
        }
        let name = FT_Get_Postscript_Name(face);
        let matches = !name.is_null() && CStr::from_ptr(name).to_bytes() == target;
        FT_Done_Face(face);
        if matches {
            return Some(index as u32);
        }
    }
    None
}
fn points(size: f32, units: u32, value: c_long) -> f32 {
    (value as f64 * size as f64 / units as f64) as f32
}
#[no_mangle]
pub unsafe extern "C" fn pitex_ft_advance(face: Handle, glyph: u32, vertical: bool) -> c_long {
    let mut advance = 0;
    let flags = 1 | if vertical { 16 } else { 0 };
    if FT_Get_Advance(face, glyph, flags, &mut advance) != 0 {
        return 0;
    }
    if vertical {
        -advance
    } else {
        advance
    }
}
unsafe extern "C" fn nominal(
    _: Handle,
    face: Handle,
    character: u32,
    glyph: *mut u32,
    _: Handle,
) -> i32 {
    *glyph = FT_Get_Char_Index(face, character as c_ulong);
    (*glyph != 0) as i32
}
unsafe extern "C" fn variation(
    _: Handle,
    face: Handle,
    character: u32,
    selector: u32,
    glyph: *mut u32,
    _: Handle,
) -> i32 {
    *glyph = FT_Face_GetCharVariantIndex(face, character as c_ulong, selector as c_ulong);
    (*glyph != 0) as i32
}
unsafe extern "C" fn horizontal_advance(_: Handle, face: Handle, glyph: u32, _: Handle) -> i32 {
    pitex_ft_advance(face, glyph, false) as i32
}
unsafe extern "C" fn vertical_advance(_: Handle, face: Handle, glyph: u32, _: Handle) -> i32 {
    pitex_ft_advance(face, glyph, true) as i32
}
// The original XeTeX origin callbacks deliberately preserve (0,0), including
// vertical text; changing that model belongs to a separate behavior change.
unsafe extern "C" fn origin(
    _: Handle,
    _: Handle,
    _: u32,
    _: *mut i32,
    _: *mut i32,
    _: Handle,
) -> i32 {
    1
}
unsafe extern "C" fn horizontal_kerning(
    _: Handle,
    face: Handle,
    left: u32,
    right: u32,
    _: Handle,
) -> i32 {
    let mut vector = Vector::default();
    if FT_Get_Kerning(face, left, right, 2, &mut vector) != 0 {
        0
    } else {
        vector.x as i32
    }
}
unsafe extern "C" fn vertical_kerning(_: Handle, _: Handle, _: u32, _: u32, _: Handle) -> i32 {
    0
}
unsafe extern "C" fn glyph_extents(
    _: Handle,
    face: Handle,
    glyph: u32,
    extents: *mut Extents,
    _: Handle,
) -> i32 {
    if FT_Load_Glyph(face, glyph, 1) != 0 {
        return 0;
    }
    let mut metrics = GlyphInfo::default();
    pitex_ft_loaded_glyph(face, &mut metrics);
    *extents = Extents {
        x_bearing: metrics.x_bearing as i32,
        y_bearing: metrics.y_bearing as i32,
        width: metrics.width as i32,
        height: -metrics.height as i32,
    };
    1
}
unsafe extern "C" fn contour_point(
    _: Handle,
    face: Handle,
    glyph: u32,
    index: u32,
    x: *mut i32,
    y: *mut i32,
    _: Handle,
) -> i32 {
    if FT_Load_Glyph(face, glyph, 1) != 0 {
        return 0;
    }
    let mut info = GlyphInfo::default();
    pitex_ft_loaded_glyph(face, &mut info);
    if info.format != u32::from_be_bytes(*b"outl") || index >= info.point_count as u32 {
        return 0;
    }
    let point = &*info.points.add(index as usize);
    *x = point.x as i32;
    *y = point.y as i32;
    1
}
unsafe extern "C" fn glyph_name(
    _: Handle,
    face: Handle,
    glyph: u32,
    name: *mut c_char,
    size: u32,
    _: Handle,
) -> i32 {
    if FT_Get_Glyph_Name(face, glyph, name, size) != 0 {
        return 0;
    }
    if size != 0 && *name == 0 {
        0
    } else {
        1
    }
}
unsafe fn font_functions() -> Handle {
    if !FONT_FUNCTIONS.is_null() {
        return FONT_FUNCTIONS;
    }
    let funcs = hb_font_funcs_create();
    hb_font_funcs_set_nominal_glyph_func(funcs, Some(nominal), ptr::null_mut(), None);
    hb_font_funcs_set_variation_glyph_func(funcs, Some(variation), ptr::null_mut(), None);
    hb_font_funcs_set_glyph_h_advance_func(funcs, Some(horizontal_advance), ptr::null_mut(), None);
    hb_font_funcs_set_glyph_v_advance_func(funcs, Some(vertical_advance), ptr::null_mut(), None);
    hb_font_funcs_set_glyph_h_origin_func(funcs, Some(origin), ptr::null_mut(), None);
    hb_font_funcs_set_glyph_v_origin_func(funcs, Some(origin), ptr::null_mut(), None);
    hb_font_funcs_set_glyph_h_kerning_func(funcs, Some(horizontal_kerning), ptr::null_mut(), None);
    hb_font_funcs_set_glyph_v_kerning_func(funcs, Some(vertical_kerning), ptr::null_mut(), None);
    hb_font_funcs_set_glyph_extents_func(funcs, Some(glyph_extents), ptr::null_mut(), None);
    hb_font_funcs_set_glyph_contour_point_func(funcs, Some(contour_point), ptr::null_mut(), None);
    hb_font_funcs_set_glyph_name_func(funcs, Some(glyph_name), ptr::null_mut(), None);
    FONT_FUNCTIONS = funcs;
    funcs
}
unsafe fn table(face: Handle, tag: u32) -> (Handle, c_ulong) {
    let mut length = 0;
    if FT_Load_Sfnt_Table(face, tag as c_ulong, 0, ptr::null_mut(), &mut length) != 0 {
        return (ptr::null_mut(), 0);
    }
    let data = malloc(length as usize);
    if data.is_null() {
        return (data, 0);
    }
    if FT_Load_Sfnt_Table(face, tag as c_ulong, 0, data.cast(), &mut length) != 0 {
        free(data);
        return (ptr::null_mut(), 0);
    }
    (data, length)
}
unsafe extern "C" fn font_table(_: Handle, tag: u32, face: Handle) -> Handle {
    let (data, length) = table(face, tag);
    if data.is_null() {
        ptr::null_mut()
    } else {
        hb_blob_create(data.cast(), length as u32, 2, data, Some(free))
    }
}
#[no_mangle]
pub unsafe extern "C" fn pitex_ft_table(face: Handle, tag: u32) -> Handle {
    table(face, tag).0
}
unsafe fn read(input: Handle) -> (Handle, usize) {
    let length = ttstub_input_get_size(input);
    let data = malloc(length);
    if data.is_null() && length != 0 {
        _tt_abort(b"font allocation failed\0".as_ptr().cast());
    }
    let count = ttstub_input_read(input, data.cast(), length);
    if count < 0 || count as usize != length {
        _tt_abort(b"failed to read font file\0".as_ptr().cast());
    }
    ttstub_input_close(input);
    (data, length)
}
fn afm_name(path: &[u8]) -> Vec<u8> {
    let base = path.rsplit(|b| *b == b'/').next().unwrap_or(path);
    let mut name = base.to_vec();
    if let Some(dot) = name.iter().rposition(|b| *b == b'.') {
        if name.len() - dot == 4
            && name[dot + 1].to_ascii_lowercase() == b'p'
            && name[dot + 2].to_ascii_lowercase() == b'f'
        {
            name.truncate(dot);
            name.extend_from_slice(b".afm");
        }
    }
    name
}
#[no_mangle]
pub unsafe extern "C" fn pitex_ft_initialize(
    path: *const c_char,
    index: i32,
    size: f32,
    result: *mut FontInit,
    status: *mut i32,
) {
    *result = FontInit::default();
    if path.is_null() {
        *status = 1;
        return;
    }
    let library = ensure_library();
    let mut input = ptr::null_mut();
    for format in [47, 36, 32] {
        input = ttstub_input_open(path, format, 0);
        if !input.is_null() {
            break;
        }
    }
    if input.is_null() {
        *status = 1;
        return;
    }
    let (data, length) = read(input);
    (*result).data = data;
    let error = FT_New_Memory_Face(
        library,
        data.cast(),
        length as c_long,
        index as c_long,
        &mut (*result).face,
    );
    if error != 0 {
        *status = 1;
        return;
    }
    let mut info = FaceInfo::default();
    pitex_ft_face_info((*result).face, &mut info);
    if info.flags & 1 == 0 {
        *status = 1;
        return;
    }
    if index == 0 && info.flags & 8 == 0 {
        let filename = CString::new(afm_name(CStr::from_ptr(path).to_bytes())).unwrap();
        let input = ttstub_input_open(filename.as_ptr(), 4, 0);
        if !input.is_null() {
            let (data, length) = read(input);
            (*result).afm_data = data;
            pitex_ft_attach_memory((*result).face, data, length as c_ulong);
        }
    }
    // Attached AFM data can replace Type 1 global ascender/descender metrics.
    // The original instance reads them after FT_Attach_Stream.
    pitex_ft_face_info((*result).face, &mut info);
    let result = &mut *result;
    result.filename = strdup(path);
    result.index = index as u32;
    result.units_per_em = info.units_per_em;
    result.ascent = points(size, info.units_per_em, info.ascender as c_long);
    result.descent = points(size, info.units_per_em, info.descender as c_long);
    if info.post_present != 0 {
        result.italic_angle = (info.italic_angle as f64 / 65536.) as f32;
    }
    if info.os2_present != 0 {
        result.cap_height = points(size, info.units_per_em, info.cap_height as c_long);
        result.x_height = points(size, info.units_per_em, info.x_height as c_long);
    }
    let face = hb_face_create_for_tables(Some(font_table), result.face, None);
    hb_face_set_index(face, index as u32);
    hb_face_set_upem(face, info.units_per_em);
    result.hb_font = hb_font_create(face);
    hb_face_destroy(face);
    hb_font_set_funcs(result.hb_font, font_functions(), result.face, None);
    hb_font_set_scale(
        result.hb_font,
        info.units_per_em as i32,
        info.units_per_em as i32,
    );
    hb_font_set_ppem(result.hb_font, 0, 0);
}
#[no_mangle]
pub unsafe extern "C" fn pitex_ft_bounds(
    face: Handle,
    size: f32,
    units: u32,
    glyph: u32,
    bounds: *mut GlyphBox,
) {
    *bounds = GlyphBox::default();
    if FT_Load_Glyph(face, glyph, 1) != 0 {
        return;
    }
    let mut info = GlyphInfo::default();
    pitex_ft_loaded_glyph(face, &mut info);
    let mut loaded = ptr::null_mut();
    if FT_Get_Glyph(info.slot, &mut loaded) != 0 {
        return;
    }
    let mut bbox = Bounds::default();
    FT_Glyph_Get_CBox(loaded, 0, &mut bbox);
    FT_Done_Glyph(loaded);
    *bounds = GlyphBox {
        x_min: points(size, units, bbox.x_min),
        y_min: points(size, units, bbox.y_min),
        x_max: points(size, units, bbox.x_max),
        y_max: points(size, units, bbox.y_max),
    };
}
#[no_mangle]
pub unsafe extern "C" fn pitex_ft_height_depth(
    face: Handle,
    size: f32,
    units: u32,
    glyph: u32,
    height: *mut f32,
    depth: *mut f32,
) {
    let mut bbox = GlyphBox::default();
    pitex_ft_bounds(face, size, units, glyph, &mut bbox);
    if !height.is_null() {
        *height = bbox.y_max;
    }
    if !depth.is_null() {
        *depth = -bbox.y_min;
    }
}
#[no_mangle]
pub unsafe extern "C" fn pitex_ft_sidebearings(
    face: Handle,
    size: f32,
    units: u32,
    glyph: u32,
    left: *mut f32,
    right: *mut f32,
) {
    let width = points(size, units, pitex_ft_advance(face, glyph, false));
    let mut bbox = GlyphBox::default();
    pitex_ft_bounds(face, size, units, glyph, &mut bbox);
    if !left.is_null() {
        *left = bbox.x_min;
    }
    if !right.is_null() {
        *right = width - bbox.x_max;
    }
}
#[no_mangle]
pub unsafe extern "C" fn pitex_ft_italic_correction(
    face: Handle,
    size: f32,
    units: u32,
    glyph: u32,
) -> f32 {
    let width = points(size, units, pitex_ft_advance(face, glyph, false));
    let mut bbox = GlyphBox::default();
    pitex_ft_bounds(face, size, units, glyph, &mut bbox);
    if bbox.x_max > width {
        bbox.x_max - width
    } else {
        0.
    }
}
static mut NAME: [c_char; 256] = [0; 256];
#[no_mangle]
pub unsafe extern "C" fn pitex_ft_glyph_name(
    face: Handle,
    glyph: u32,
    length: *mut i32,
) -> *const c_char {
    let mut info = FaceInfo::default();
    pitex_ft_face_info(face, &mut info);
    if info.flags & 512 == 0 {
        *length = 0;
        return ptr::null();
    }
    let buffer = ptr::addr_of_mut!(NAME).cast::<c_char>();
    FT_Get_Glyph_Name(face, glyph, buffer, 256);
    *length = strlen(buffer) as i32;
    buffer
}
#[no_mangle]
pub unsafe extern "C" fn pitex_ft_last_character(face: Handle) -> i32 {
    let mut glyph = 0;
    let mut character = FT_Get_First_Char(face, &mut glyph);
    let mut previous = character;
    while glyph != 0 {
        previous = character;
        character = FT_Get_Next_Char(face, character, &mut glyph);
    }
    previous as i32
}
#[cfg(test)]
mod tests {
    use super::*;
    static FONT_TESTS: std::sync::Mutex<()> = std::sync::Mutex::new(());
    #[test]
    fn afm_candidate_is_local_basename() {
        assert_eq!(afm_name(b"/fonts/MyFace.PFB"), b"MyFace.afm");
        assert_eq!(afm_name(b"Face.otf"), b"Face.otf");
        assert_eq!(afm_name(b"/fonts/Face.pfa"), b"Face.afm");
    }
    #[test]
    fn collection_face_selection_when_requested() {
        let Some(path) = std::env::var_os("PITEX_TTC_FIXTURE") else {
            return;
        };
        let _guard = FONT_TESTS.lock().unwrap();
        unsafe {
            let path = CString::new(path.to_string_lossy().as_bytes()).unwrap();
            let library = ensure_library();
            let mut face = ptr::null_mut();
            assert_eq!(FT_New_Face(library, path.as_ptr(), 0, &mut face), 0);
            let mut info = FaceInfo::default();
            pitex_ft_face_info(face, &mut info);
            assert!(info.num_faces > 1);
            FT_Done_Face(face);
            assert_eq!(FT_New_Face(library, path.as_ptr(), 1, &mut face), 0);
            let name = CStr::from_ptr(FT_Get_Postscript_Name(face)).to_owned();
            FT_Done_Face(face);
            assert_eq!(collection_index(path.as_ptr(), name.as_ptr()), Some(1));
            assert_eq!(
                collection_index(
                    path.as_ptr(),
                    b"PitexMissingCollectionFace\0".as_ptr().cast()
                ),
                None
            );
        }
    }
    #[test]
    fn attached_type1_metrics_when_requested() {
        let (Some(font), Some(afm)) = (
            std::env::var_os("PITEX_AFM_FONT"),
            std::env::var_os("PITEX_AFM_FIXTURE"),
        ) else {
            return;
        };
        let _guard = FONT_TESTS.lock().unwrap();
        unsafe {
            let font = CString::new(font.to_string_lossy().as_bytes()).unwrap();
            let mut face = ptr::null_mut();
            assert_eq!(
                FT_New_Face(ensure_library(), font.as_ptr(), 0, &mut face),
                0
            );
            let mut before = FaceInfo::default();
            pitex_ft_face_info(face, &mut before);
            assert_eq!(before.flags & 8, 0);
            let bytes = std::fs::read(afm).unwrap();
            pitex_ft_attach_memory(
                face,
                bytes.as_ptr().cast_mut().cast(),
                bytes.len() as c_ulong,
            );
            let mut after = FaceInfo::default();
            pitex_ft_face_info(face, &mut after);
            FT_Done_Face(face);
            assert_ne!(
                (before.ascender, before.descender),
                (after.ascender, after.descender)
            );
            assert_eq!((after.ascender, after.descender), (694, -194));
        }
    }
}
