// Pitex-authored. Copyright (c) 2026 Pitex contributors. SPDX-License-Identifier: AGPL-3.0-or-later
// Original FreeType client. Fonts are read from the user's TeX installation;
// no font programs, metrics, or third-party converter implementations are vendored.
use std::{
    ffi::{c_char, c_int, c_long, c_uint, c_void, CString},
    rc::Rc,
};
#[repr(C)]
#[derive(Clone, Copy)]
struct Vector {
    x: c_long,
    y: c_long,
}
#[repr(C)]
struct Generic {
    data: *mut c_void,
    finalizer: *mut c_void,
}
#[repr(C)]
struct BBox {
    x_min: c_long,
    y_min: c_long,
    x_max: c_long,
    y_max: c_long,
}
#[repr(C)]
struct FaceHeader {
    num_faces: c_long,
    face_index: c_long,
    face_flags: c_long,
    style_flags: c_long,
    num_glyphs: c_long,
    family_name: *mut c_char,
    style_name: *mut c_char,
    num_fixed_sizes: c_int,
    available_sizes: *mut c_void,
    num_charmaps: c_int,
    charmaps: *mut c_void,
    generic: Generic,
    bbox: BBox,
    units: u16,
    ascender: i16,
    descender: i16,
    height: i16,
    max_advance_width: i16,
    max_advance_height: i16,
    underline_position: i16,
    underline_thickness: i16,
    glyph: *mut c_void,
}
#[repr(C)]
struct GlyphHeader {
    library: *mut c_void,
    class: *mut c_void,
    format: c_uint,
    advance: Vector,
}
#[repr(C)]
struct Outline {
    contours: i16,
    points_count: i16,
    points: *mut Vector,
    tags: *mut u8,
    ends: *mut i16,
    flags: c_int,
}
#[repr(C)]
struct OutlineGlyph {
    root: GlyphHeader,
    outline: Outline,
}
#[repr(C)]
struct OutlineFuncs {
    move_to: unsafe extern "C" fn(*const Vector, *mut c_void) -> c_int,
    line_to: unsafe extern "C" fn(*const Vector, *mut c_void) -> c_int,
    conic_to: unsafe extern "C" fn(*const Vector, *const Vector, *mut c_void) -> c_int,
    cubic_to:
        unsafe extern "C" fn(*const Vector, *const Vector, *const Vector, *mut c_void) -> c_int,
    shift: c_int,
    delta: c_long,
}
#[link(name = "freetype")]
extern "C" {
    fn FT_Init_FreeType(library: *mut *mut c_void) -> c_int;
    fn FT_Done_FreeType(library: *mut c_void) -> c_int;
    fn FT_New_Memory_Face(
        library: *mut c_void,
        data: *const u8,
        len: c_long,
        index: c_long,
        face: *mut *mut FaceHeader,
    ) -> c_int;
    fn FT_Done_Face(face: *mut FaceHeader) -> c_int;
    fn FT_Get_Char_Index(face: *mut FaceHeader, code: usize) -> c_uint;
    fn FT_Get_Name_Index(face: *mut FaceHeader, name: *const c_char) -> c_uint;
    fn FT_Select_Charmap(face: *mut FaceHeader, encoding: c_uint) -> c_int;
    fn FT_Load_Glyph(face: *mut FaceHeader, glyph: c_uint, flags: i32) -> c_int;
    fn FT_Get_Advance(
        face: *mut FaceHeader,
        glyph: c_uint,
        flags: i32,
        advance: *mut c_long,
    ) -> c_int;
    fn FT_Get_Glyph(slot: *mut c_void, glyph: *mut *mut OutlineGlyph) -> c_int;
    fn FT_Done_Glyph(glyph: *mut OutlineGlyph);
    fn FT_Outline_Decompose(
        outline: *const Outline,
        funcs: *const OutlineFuncs,
        user: *mut c_void,
    ) -> c_int;
}
#[derive(Clone, Debug)]
pub enum Command {
    Move(f64, f64),
    Line(f64, f64),
    Curve([f64; 6]),
    Close,
}
struct Collector {
    path: Vec<Command>,
    point: (f64, f64),
}
unsafe extern "C" fn mv(p: *const Vector, u: *mut c_void) -> c_int {
    let c = &mut *u.cast::<Collector>();
    if !c.path.is_empty() {
        c.path.push(Command::Close);
    }
    c.point = ((*p).x as f64, (*p).y as f64);
    c.path.push(Command::Move(c.point.0, c.point.1));
    0
}
unsafe extern "C" fn line(p: *const Vector, u: *mut c_void) -> c_int {
    let c = &mut *u.cast::<Collector>();
    c.point = ((*p).x as f64, (*p).y as f64);
    c.path.push(Command::Line(c.point.0, c.point.1));
    0
}
unsafe extern "C" fn conic(p: *const Vector, q: *const Vector, u: *mut c_void) -> c_int {
    let c = &mut *u.cast::<Collector>();
    let p = ((*p).x as f64, (*p).y as f64);
    let q = ((*q).x as f64, (*q).y as f64);
    c.path.push(Command::Curve([
        c.point.0 + 2. / 3. * (p.0 - c.point.0),
        c.point.1 + 2. / 3. * (p.1 - c.point.1),
        q.0 + 2. / 3. * (p.0 - q.0),
        q.1 + 2. / 3. * (p.1 - q.1),
        q.0,
        q.1,
    ]));
    c.point = q;
    0
}
unsafe extern "C" fn cubic(
    p: *const Vector,
    q: *const Vector,
    r: *const Vector,
    u: *mut c_void,
) -> c_int {
    let c = &mut *u.cast::<Collector>();
    c.path.push(Command::Curve([
        (*p).x as f64,
        (*p).y as f64,
        (*q).x as f64,
        (*q).y as f64,
        (*r).x as f64,
        (*r).y as f64,
    ]));
    c.point = ((*r).x as f64, (*r).y as f64);
    0
}
struct Library(*mut c_void);
impl Drop for Library {
    fn drop(&mut self) {
        unsafe {
            FT_Done_FreeType(self.0);
        }
    }
}
pub struct Font {
    face: *mut FaceHeader,
    _library: Rc<Library>,
    _data: Vec<u8>,
    pub units: f64,
}
impl Font {
    pub fn new(data: Vec<u8>) -> Result<Self, String> {
        unsafe {
            let mut library = std::ptr::null_mut();
            if FT_Init_FreeType(&mut library) != 0 {
                return Err("FreeType initialization failed".into());
            }
            let library = Rc::new(Library(library));
            let mut face = std::ptr::null_mut();
            let error =
                FT_New_Memory_Face(library.0, data.as_ptr(), data.len() as c_long, 0, &mut face);
            if error != 0 {
                return Err(format!(
                    "PostScript font format unsupported (FreeType {error})"
                ));
            }
            // Type 1 strings address the font's encoding, not Unicode.
            // Explicit PostScript Encoding arrays still select glyph names.
            // OpenType fallback fonts retain their Unicode charmap when the
            // installed face does not provide an Adobe standard charmap.
            FT_Select_Charmap(face, u32::from_be_bytes(*b"ADOB"));
            Ok(Self {
                face,
                _library: library,
                _data: data,
                units: (*face).units.max(1) as f64,
            })
        }
    }
    pub fn glyph(&self, code: u8, name: Option<&str>) -> Result<(f64, Vec<Command>), String> {
        unsafe {
            let index = if let Some(name) = name {
                let name = CString::new(name).map_err(|_| "invalid glyph name")?;
                FT_Get_Name_Index(self.face, name.as_ptr())
            } else {
                FT_Get_Char_Index(self.face, code as usize)
            };
            let mut advance = 0;
            let flags = 1 | 2 | 8;
            if FT_Get_Advance(self.face, index, flags, &mut advance) != 0
                || FT_Load_Glyph(self.face, index, flags) != 0
            {
                return Err("PostScript font glyph could not be loaded".into());
            }
            let mut glyph = std::ptr::null_mut();
            if FT_Get_Glyph((*self.face).glyph, &mut glyph) != 0 {
                return Err("PostScript glyph outline could not be loaded".into());
            }
            if (*glyph).root.format != u32::from_be_bytes(*b"outl") {
                FT_Done_Glyph(glyph);
                return Err("PostScript bitmap glyph is unsupported".into());
            }
            let mut c = Collector {
                path: vec![],
                point: (0., 0.),
            };
            let funcs = OutlineFuncs {
                move_to: mv,
                line_to: line,
                conic_to: conic,
                cubic_to: cubic,
                shift: 0,
                delta: 0,
            };
            let error =
                FT_Outline_Decompose(&(*glyph).outline, &funcs, (&mut c as *mut Collector).cast());
            FT_Done_Glyph(glyph);
            if error != 0 {
                return Err("PostScript outline decomposition failed".into());
            }
            if !c.path.is_empty() {
                c.path.push(Command::Close)
            }
            Ok((advance as f64, c.path))
        }
    }
}
impl Drop for Font {
    fn drop(&mut self) {
        unsafe {
            FT_Done_Face(self.face);
        }
    }
}
