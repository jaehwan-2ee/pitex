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
// Translated from xetex/engine/xetex-pic.c with C2Rust 0.22.1.
extern "C" {
    pub type ttbc_input_handle_t;
    pub type ttbc_diagnostic_t;
    pub type pr_doc;
    fn cos(_: ::core::ffi::c_double) -> ::core::ffi::c_double;
    fn sin(_: ::core::ffi::c_double) -> ::core::ffi::c_double;
    fn malloc(__size: size_t) -> *mut ::core::ffi::c_void;
    fn free(_: *mut ::core::ffi::c_void);
    fn memcpy(
        __dst: *mut ::core::ffi::c_void,
        __src: *const ::core::ffi::c_void,
        __n: size_t,
    ) -> *mut ::core::ffi::c_void;
    fn strlen(__s: *const ::core::ffi::c_char) -> size_t;
    fn strdup(__s1: *const ::core::ffi::c_char) -> *mut ::core::ffi::c_char;
    fn ttstub_input_open(
        path: *const ::core::ffi::c_char,
        format: ttbc_file_format,
        is_gz: ::core::ffi::c_int,
    ) -> rust_input_handle_t;
    fn ttstub_input_get_size(handle: rust_input_handle_t) -> size_t;
    fn ttstub_input_seek(
        handle: rust_input_handle_t,
        offset: ssize_t,
        whence: ::core::ffi::c_int,
    ) -> size_t;
    fn ttstub_input_read(
        handle: rust_input_handle_t,
        data: *mut ::core::ffi::c_char,
        len: size_t,
    ) -> ssize_t;
    fn ttstub_input_close(handle: rust_input_handle_t) -> ::core::ffi::c_int;
    fn ttstub_pic_get_cached_bounds(
        name: *const ::core::ffi::c_char,
        type_0: ::core::ffi::c_int,
        page: ::core::ffi::c_int,
        bounds: *mut ::core::ffi::c_float,
    ) -> ::core::ffi::c_int;
    fn ttstub_pic_set_cached_bounds(
        name: *const ::core::ffi::c_char,
        type_0: ::core::ffi::c_int,
        page: ::core::ffi::c_int,
        bounds: *const ::core::ffi::c_float,
    );
    fn Fix2D(f: Fixed) -> ::core::ffi::c_double;
    fn D2Fix(d: ::core::ffi::c_double) -> Fixed;
    static mut name_of_file: *mut ::core::ffi::c_char;
    static mut help_line: [*const ::core::ffi::c_char; 6];
    static mut help_ptr: ::core::ffi::c_uchar;
    static mut mem: *mut memory_word;
    static mut cur_list: list_state_record;
    static mut cur_val: int32_t;
    static mut cur_name: str_number;
    static mut cur_area: str_number;
    static mut cur_ext: str_number;
    fn scan_keyword(s: *const ::core::ffi::c_char) -> bool;
    fn scan_int();
    fn scan_dimen(mu: bool, inf: bool, shortcut: bool);
    fn scan_decimal();
    fn pack_file_name(n: str_number, a: str_number, e: str_number);
    fn scan_file_name();
    fn new_whatsit(s: small_number, w: small_number);
    fn error();
    fn capture_to_diagnostic(diagnostic: *mut ttbc_diagnostic_t);
    fn error_here_with_diagnostic(message: *const ::core::ffi::c_char) -> *mut ttbc_diagnostic_t;
    fn print(s: int32_t);
    fn print_cstr(s: *const ::core::ffi::c_char);
    fn print_file_name(n: int32_t, a: int32_t, e: int32_t);
    fn print_scaled(s: scaled_t);
    fn pr_open(data: *const ::core::ffi::c_uchar, len: size_t) -> *mut pr_doc;
    fn pr_close(doc: *mut pr_doc);
    fn pr_page_count(doc: *mut pr_doc) -> ::core::ffi::c_int;
    fn pr_page(
        doc: *mut pr_doc,
        index0: ::core::ffi::c_int,
        box_kind: ::core::ffi::c_int,
        out: *mut pr_page_info,
    ) -> bool;
    fn pr_normalize_page(doc: *mut pr_doc, page_arg: ::core::ffi::c_int) -> ::core::ffi::c_int;
    fn pr_rotation_matrix(rotate: ::core::ffi::c_int, m: *mut ::core::ffi::c_double);
    fn pr_transform_box(
        m: *const ::core::ffi::c_double,
        box_0: *const ::core::ffi::c_double,
        out: *mut ::core::ffi::c_double,
    );
    fn pitex_img_info_read(
        data: *const ::core::ffi::c_uchar,
        len: size_t,
        out: *mut pitex_img_info,
    ) -> ::core::ffi::c_int;
}
pub type __darwin_size_t = usize;
pub type __darwin_ssize_t = isize;
pub type size_t = __darwin_size_t;
pub type int32_t = i32;
pub type uint16_t = u16;
pub type ssize_t = __darwin_ssize_t;
pub type rust_input_handle_t = *mut ttbc_input_handle_t;
pub type ttbc_file_format = ::core::ffi::c_uint;
pub const TTBC_FILE_FORMAT_VF: ttbc_file_format = 33;
pub const TTBC_FILE_FORMAT_TYPE1: ttbc_file_format = 32;
pub const TTBC_FILE_FORMAT_TRUE_TYPE: ttbc_file_format = 36;
pub const TTBC_FILE_FORMAT_TFM: ttbc_file_format = 3;
pub const TTBC_FILE_FORMAT_TEX_PS_HEADER: ttbc_file_format = 30;
pub const TTBC_FILE_FORMAT_TEX: ttbc_file_format = 26;
pub const TTBC_FILE_FORMAT_TECTONIC_PRIMARY: ttbc_file_format = 59;
pub const TTBC_FILE_FORMAT_SFD: ttbc_file_format = 46;
pub const TTBC_FILE_FORMAT_PROGRAM_DATA: ttbc_file_format = 39;
pub const TTBC_FILE_FORMAT_PK: ttbc_file_format = 1;
pub const TTBC_FILE_FORMAT_PICT: ttbc_file_format = 25;
pub const TTBC_FILE_FORMAT_OVF: ttbc_file_format = 23;
pub const TTBC_FILE_FORMAT_OPEN_TYPE: ttbc_file_format = 47;
pub const TTBC_FILE_FORMAT_OFM: ttbc_file_format = 20;
pub const TTBC_FILE_FORMAT_MISC_FONTS: ttbc_file_format = 41;
pub const TTBC_FILE_FORMAT_FONT_MAP: ttbc_file_format = 11;
pub const TTBC_FILE_FORMAT_FORMAT: ttbc_file_format = 10;
pub const TTBC_FILE_FORMAT_ENC: ttbc_file_format = 44;
pub const TTBC_FILE_FORMAT_CNF: ttbc_file_format = 8;
pub const TTBC_FILE_FORMAT_CMAP: ttbc_file_format = 45;
pub const TTBC_FILE_FORMAT_BST: ttbc_file_format = 7;
pub const TTBC_FILE_FORMAT_BIB: ttbc_file_format = 6;
pub const TTBC_FILE_FORMAT_AFM: ttbc_file_format = 4;
pub type scaled_t = int32_t;
pub type SInt32 = ::core::ffi::c_int;
pub type Fixed = SInt32;
pub type str_number = int32_t;
pub type small_number = ::core::ffi::c_short;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct b32x2_le_t {
    pub s0: int32_t,
    pub s1: int32_t,
}
pub type b32x2 = b32x2_le_t;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct b16x4_le_t {
    pub s0: uint16_t,
    pub s1: uint16_t,
    pub s2: uint16_t,
    pub s3: uint16_t,
}
pub type b16x4 = b16x4_le_t;
#[derive(Copy, Clone)]
#[repr(C)]
pub union memory_word {
    pub b32: b32x2,
    pub b16: b16x4,
    pub gr: ::core::ffi::c_double,
    pub ptr: *mut ::core::ffi::c_void,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct list_state_record {
    pub mode: ::core::ffi::c_short,
    pub head: int32_t,
    pub tail: int32_t,
    pub eTeX_aux: int32_t,
    pub prev_graf: int32_t,
    pub mode_line: int32_t,
    pub aux: memory_word,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct transform_t {
    pub a: ::core::ffi::c_double,
    pub b: ::core::ffi::c_double,
    pub c: ::core::ffi::c_double,
    pub d: ::core::ffi::c_double,
    pub x: ::core::ffi::c_double,
    pub y: ::core::ffi::c_double,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct real_point {
    pub x: ::core::ffi::c_float,
    pub y: ::core::ffi::c_float,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct real_rect {
    pub x: ::core::ffi::c_float,
    pub y: ::core::ffi::c_float,
    pub wd: ::core::ffi::c_float,
    pub ht: ::core::ffi::c_float,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct pitex_img_info {
    pub kind: pitex_img_kind,
    pub width: ::core::ffi::c_uint,
    pub height: ::core::ffi::c_uint,
    pub xdpi: ::core::ffi::c_double,
    pub ydpi: ::core::ffi::c_double,
}
pub type pitex_img_kind = ::core::ffi::c_uint;
pub const PITEX_IMG_BMP: pitex_img_kind = 3;
pub const PITEX_IMG_JPEG: pitex_img_kind = 2;
pub const PITEX_IMG_PNG: pitex_img_kind = 1;
pub const PITEX_IMG_NONE: pitex_img_kind = 0;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct pr_page_info {
    pub box_0: [::core::ffi::c_double; 4],
    pub rotate: ::core::ffi::c_int,
    pub page: *mut pr_obj,
    pub resources: *mut pr_obj,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct pr_obj {
    pub type_0: pr_type,
    pub u: C2RustUnnamed,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union C2RustUnnamed {
    pub b: ::core::ffi::c_int,
    pub i: ::core::ffi::c_longlong,
    pub r: ::core::ffi::c_double,
    pub str_0: C2RustUnnamed_4,
    pub arr: C2RustUnnamed_3,
    pub dict: C2RustUnnamed_2,
    pub ref_0: C2RustUnnamed_1,
    pub stream: C2RustUnnamed_0,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_0 {
    pub dict: *mut pr_obj,
    pub offset: size_t,
    pub length: size_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_1 {
    pub num: ::core::ffi::c_int,
    pub gen: ::core::ffi::c_int,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_2 {
    pub keys: *mut *mut ::core::ffi::c_char,
    pub vals: *mut *mut pr_obj,
    pub n: ::core::ffi::c_int,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_3 {
    pub items: *mut *mut pr_obj,
    pub n: ::core::ffi::c_int,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_4 {
    pub s: *mut ::core::ffi::c_char,
    pub len: size_t,
}
pub type pr_type = ::core::ffi::c_uint;
pub const PR_STREAM: pr_type = 9;
pub const PR_REF: pr_type = 8;
pub const PR_DICT: pr_type = 7;
pub const PR_ARRAY: pr_type = 6;
pub const PR_STRING: pr_type = 5;
pub const PR_NAME: pr_type = 4;
pub const PR_REAL: pr_type = 3;
pub const PR_INT: pr_type = 2;
pub const PR_BOOL: pr_type = 1;
pub const PR_NULL: pr_type = 0;
pub const __DARWIN_NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
pub const M_PI: ::core::ffi::c_double = 3.14159265358979323846264338327950288f64;
pub const NULL: *mut ::core::ffi::c_void = __DARWIN_NULL;
pub const PIC_NODE_SIZE: ::core::ffi::c_int = 9 as ::core::ffi::c_int;
pub const pdfbox_crop: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const pdfbox_media: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const pdfbox_bleed: ::core::ffi::c_int = 3 as ::core::ffi::c_int;
pub const pdfbox_trim: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
pub const pdfbox_art: ::core::ffi::c_int = 5 as ::core::ffi::c_int;
pub const pdfbox_none: ::core::ffi::c_int = 6 as ::core::ffi::c_int;
pub const PIC_NODE: ::core::ffi::c_int = 43 as ::core::ffi::c_int;
pub const PDF_NODE: ::core::ffi::c_int = 44 as ::core::ffi::c_int;
pub const SEEK_SET: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
unsafe extern "C" fn read_whole_handle(
    mut handle: rust_input_handle_t,
    mut len: *mut size_t,
) -> *mut ::core::ffi::c_uchar {
    let mut size: size_t = ttstub_input_get_size(handle);
    let mut data: *mut ::core::ffi::c_uchar =
        malloc(size.wrapping_add(1 as size_t)) as *mut ::core::ffi::c_uchar;
    let mut got: size_t = 0 as size_t;
    ttstub_input_seek(handle, 0 as ssize_t, SEEK_SET);
    while got < size {
        let mut n: ssize_t = ttstub_input_read(
            handle,
            (data as *mut ::core::ffi::c_char).offset(got as isize),
            size.wrapping_sub(got),
        );
        if n <= 0 as ssize_t {
            break;
        }
        got = got.wrapping_add(n as size_t);
    }
    *len = got;
    return data;
}
#[no_mangle]
pub unsafe extern "C" fn count_pdf_file_pages() -> ::core::ffi::c_int {
    let mut pages: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut handle: rust_input_handle_t = ::core::ptr::null_mut::<ttbc_input_handle_t>();
    let mut data: *mut ::core::ffi::c_uchar = ::core::ptr::null_mut::<::core::ffi::c_uchar>();
    let mut len: size_t = 0;
    let mut doc: *mut pr_doc = ::core::ptr::null_mut::<pr_doc>();
    handle = ttstub_input_open(name_of_file, TTBC_FILE_FORMAT_PICT, 0 as ::core::ffi::c_int);
    if handle.is_null() {
        return 0 as ::core::ffi::c_int;
    }
    data = read_whole_handle(handle, &raw mut len);
    ttstub_input_close(handle);
    doc = pr_open(data, len);
    if !doc.is_null() {
        pages = pr_page_count(doc);
        pr_close(doc);
    }
    free(data as *mut ::core::ffi::c_void);
    return pages;
}
unsafe extern "C" fn pdf_get_rect(
    mut handle: rust_input_handle_t,
    mut page_num: ::core::ffi::c_int,
    mut pdf_box: ::core::ffi::c_int,
    mut box_0: *mut real_rect,
) -> ::core::ffi::c_int {
    let mut data: *mut ::core::ffi::c_uchar = ::core::ptr::null_mut::<::core::ffi::c_uchar>();
    let mut len: size_t = 0;
    let mut doc: *mut pr_doc = ::core::ptr::null_mut::<pr_doc>();
    let mut info: pr_page_info = pr_page_info {
        box_0: [0.; 4],
        rotate: 0,
        page: ::core::ptr::null_mut::<pr_obj>(),
        resources: ::core::ptr::null_mut::<pr_obj>(),
    };
    let mut m: [::core::ffi::c_double; 6] = [0.; 6];
    let mut bbox: [::core::ffi::c_double; 4] = [0.; 4];
    let mut ok: ::core::ffi::c_int = 0;
    data = read_whole_handle(handle, &raw mut len);
    doc = pr_open(data, len);
    if doc.is_null() {
        free(data as *mut ::core::ffi::c_void);
        return -(1 as ::core::ffi::c_int);
    }
    page_num = pr_normalize_page(doc, page_num);
    ok = pr_page(
        doc,
        page_num - 1 as ::core::ffi::c_int,
        pdf_box,
        &raw mut info,
    ) as ::core::ffi::c_int;
    pr_close(doc);
    free(data as *mut ::core::ffi::c_void);
    if ok == 0 {
        return -(1 as ::core::ffi::c_int);
    }
    pr_rotation_matrix(info.rotate, &raw mut m as *mut ::core::ffi::c_double);
    pr_transform_box(
        &raw mut m as *mut ::core::ffi::c_double as *const ::core::ffi::c_double,
        &raw mut info.box_0 as *mut ::core::ffi::c_double as *const ::core::ffi::c_double,
        &raw mut bbox as *mut ::core::ffi::c_double,
    );
    (*box_0).x = (72.27f64 / 72 as ::core::ffi::c_int as ::core::ffi::c_double
        * bbox[0 as ::core::ffi::c_int as usize]) as ::core::ffi::c_float;
    (*box_0).y = (72.27f64 / 72 as ::core::ffi::c_int as ::core::ffi::c_double
        * bbox[1 as ::core::ffi::c_int as usize]) as ::core::ffi::c_float;
    (*box_0).wd = (72.27f64 / 72 as ::core::ffi::c_int as ::core::ffi::c_double
        * (bbox[2 as ::core::ffi::c_int as usize] - bbox[0 as ::core::ffi::c_int as usize]))
        as ::core::ffi::c_float;
    (*box_0).ht = (72.27f64 / 72 as ::core::ffi::c_int as ::core::ffi::c_double
        * (bbox[3 as ::core::ffi::c_int as usize] - bbox[1 as ::core::ffi::c_int as usize]))
        as ::core::ffi::c_float;
    return 0 as ::core::ffi::c_int;
}
unsafe extern "C" fn get_image_size_in_inches(
    mut handle: rust_input_handle_t,
    mut width: *mut ::core::ffi::c_double,
    mut height: *mut ::core::ffi::c_double,
) -> ::core::ffi::c_int {
    let mut data: *mut ::core::ffi::c_uchar = ::core::ptr::null_mut::<::core::ffi::c_uchar>();
    let mut len: size_t = 0;
    let mut info: pitex_img_info = pitex_img_info {
        kind: PITEX_IMG_NONE,
        width: 0,
        height: 0,
        xdpi: 0.,
        ydpi: 0.,
    };
    let mut err: ::core::ffi::c_int = 0;
    data = read_whole_handle(handle, &raw mut len);
    err = pitex_img_info_read(data, len, &raw mut info);
    free(data as *mut ::core::ffi::c_void);
    if err != 0 {
        *width = -(1 as ::core::ffi::c_int) as ::core::ffi::c_double;
        *height = -(1 as ::core::ffi::c_int) as ::core::ffi::c_double;
        return err;
    }
    *width = info.width as ::core::ffi::c_double / info.xdpi;
    *height = info.height as ::core::ffi::c_double / info.ydpi;
    return 0 as ::core::ffi::c_int;
}
unsafe extern "C" fn find_pic_file(
    mut path: *mut *mut ::core::ffi::c_char,
    mut bounds: *mut real_rect,
    mut pdfBoxType: ::core::ffi::c_int,
    mut page: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut err: ::core::ffi::c_int = -(1 as ::core::ffi::c_int);
    let mut handle: rust_input_handle_t = ::core::ptr::null_mut::<ttbc_input_handle_t>();
    if ttstub_pic_get_cached_bounds(
        name_of_file,
        pdfBoxType,
        page,
        bounds as *mut ::core::ffi::c_void as *mut ::core::ffi::c_float,
    ) == 1 as ::core::ffi::c_int
    {
        *path = strdup(name_of_file);
        return 0 as ::core::ffi::c_int;
    }
    handle = ttstub_input_open(name_of_file, TTBC_FILE_FORMAT_PICT, 0 as ::core::ffi::c_int);
    (*bounds).ht = 0.0f32;
    (*bounds).wd = (*bounds).ht;
    (*bounds).y = (*bounds).wd;
    (*bounds).x = (*bounds).y;
    if handle.is_null() {
        return 1 as ::core::ffi::c_int;
    }
    if pdfBoxType != 0 as ::core::ffi::c_int {
        err = pdf_get_rect(handle, page, pdfBoxType, bounds);
    } else {
        let mut wd: ::core::ffi::c_double = 0.;
        let mut ht: ::core::ffi::c_double = 0.;
        err = get_image_size_in_inches(handle, &raw mut wd, &raw mut ht);
        (*bounds).wd = (wd * 72.27f64) as ::core::ffi::c_float;
        (*bounds).ht = (ht * 72.27f64) as ::core::ffi::c_float;
    }
    ttstub_input_close(handle);
    if err == 0 as ::core::ffi::c_int {
        *path = strdup(name_of_file);
        ttstub_pic_set_cached_bounds(
            name_of_file,
            pdfBoxType,
            page,
            bounds as *mut ::core::ffi::c_void as *const ::core::ffi::c_float,
        );
    }
    return err;
}
unsafe extern "C" fn transform_point(mut p: *mut real_point, mut t: *const transform_t) {
    let mut r: real_point = real_point { x: 0., y: 0. };
    r.x = ((*t).a * (*p).x as ::core::ffi::c_double
        + (*t).c * (*p).y as ::core::ffi::c_double
        + (*t).x) as ::core::ffi::c_float;
    r.y = ((*t).b * (*p).x as ::core::ffi::c_double
        + (*t).d * (*p).y as ::core::ffi::c_double
        + (*t).y) as ::core::ffi::c_float;
    *p = r;
}
unsafe extern "C" fn make_identity(mut t: *mut transform_t) {
    (*t).a = 1.0f64;
    (*t).b = 0.0f64;
    (*t).c = 0.0f64;
    (*t).d = 1.0f64;
    (*t).x = 0.0f64;
    (*t).y = 0.0f64;
}
unsafe extern "C" fn make_scale(
    mut t: *mut transform_t,
    mut xscale: ::core::ffi::c_double,
    mut yscale: ::core::ffi::c_double,
) {
    (*t).a = xscale;
    (*t).b = 0.0f64;
    (*t).c = 0.0f64;
    (*t).d = yscale;
    (*t).x = 0.0f64;
    (*t).y = 0.0f64;
}
unsafe extern "C" fn make_translation(
    mut t: *mut transform_t,
    mut dx: ::core::ffi::c_double,
    mut dy: ::core::ffi::c_double,
) {
    (*t).a = 1.0f64;
    (*t).b = 0.0f64;
    (*t).c = 0.0f64;
    (*t).d = 1.0f64;
    (*t).x = dx;
    (*t).y = dy;
}
unsafe extern "C" fn make_rotation(mut t: *mut transform_t, mut a: ::core::ffi::c_double) {
    (*t).a = cos(a);
    (*t).b = sin(a);
    (*t).c = -sin(a);
    (*t).d = cos(a);
    (*t).x = 0.0f64;
    (*t).y = 0.0f64;
}
unsafe extern "C" fn transform_concat(mut t1: *mut transform_t, mut t2: *const transform_t) {
    let mut r: transform_t = transform_t {
        a: 0.,
        b: 0.,
        c: 0.,
        d: 0.,
        x: 0.,
        y: 0.,
    };
    r.a = (*t1).a * (*t2).a + (*t1).b * (*t2).c + 0.0f64 * (*t2).x;
    r.b = (*t1).a * (*t2).b + (*t1).b * (*t2).d + 0.0f64 * (*t2).y;
    r.c = (*t1).c * (*t2).a + (*t1).d * (*t2).c + 0.0f64 * (*t2).x;
    r.d = (*t1).c * (*t2).b + (*t1).d * (*t2).d + 0.0f64 * (*t2).y;
    r.x = (*t1).x * (*t2).a + (*t1).y * (*t2).c + 1.0f64 * (*t2).x;
    r.y = (*t1).x * (*t2).b + (*t1).y * (*t2).d + 1.0f64 * (*t2).y;
    *t1 = r;
}
#[no_mangle]
pub unsafe extern "C" fn load_picture(mut is_pdf: bool) {
    let mut pic_path: *mut ::core::ffi::c_char = ::core::ptr::null_mut::<::core::ffi::c_char>();
    let mut bounds: real_rect = real_rect {
        x: 0.,
        y: 0.,
        wd: 0.,
        ht: 0.,
    };
    let mut t: transform_t = transform_t {
        a: 0.,
        b: 0.,
        c: 0.,
        d: 0.,
        x: 0.,
        y: 0.,
    };
    let mut t2: transform_t = transform_t {
        a: 0.,
        b: 0.,
        c: 0.,
        d: 0.,
        x: 0.,
        y: 0.,
    };
    let mut corners: [real_point; 4] = [real_point { x: 0., y: 0. }; 4];
    let mut x_size_req: ::core::ffi::c_double = 0.;
    let mut y_size_req: ::core::ffi::c_double = 0.;
    let mut check_keywords: bool = false;
    let mut xmin: ::core::ffi::c_double = 0.;
    let mut xmax: ::core::ffi::c_double = 0.;
    let mut ymin: ::core::ffi::c_double = 0.;
    let mut ymax: ::core::ffi::c_double = 0.;
    let mut i: small_number = 0;
    let mut page: int32_t = 0;
    let mut pdf_box_type: int32_t = 0;
    let mut result: int32_t = 0;
    scan_file_name();
    pack_file_name(cur_name, cur_area, cur_ext);
    pdf_box_type = 0 as ::core::ffi::c_int as int32_t;
    page = 0 as ::core::ffi::c_int as int32_t;
    if is_pdf {
        if scan_keyword(b"page\0" as *const u8 as *const ::core::ffi::c_char) {
            scan_int();
            page = cur_val;
        }
        pdf_box_type = pdfbox_none as int32_t;
        if scan_keyword(b"crop\0" as *const u8 as *const ::core::ffi::c_char) {
            pdf_box_type = pdfbox_crop as int32_t;
        } else if scan_keyword(b"media\0" as *const u8 as *const ::core::ffi::c_char) {
            pdf_box_type = pdfbox_media as int32_t;
        } else if scan_keyword(b"bleed\0" as *const u8 as *const ::core::ffi::c_char) {
            pdf_box_type = pdfbox_bleed as int32_t;
        } else if scan_keyword(b"trim\0" as *const u8 as *const ::core::ffi::c_char) {
            pdf_box_type = pdfbox_trim as int32_t;
        } else if scan_keyword(b"art\0" as *const u8 as *const ::core::ffi::c_char) {
            pdf_box_type = pdfbox_art as int32_t;
        }
    }
    if pdf_box_type == pdfbox_none as int32_t {
        result = find_pic_file(
            &raw mut pic_path,
            &raw mut bounds,
            pdfbox_crop,
            page as ::core::ffi::c_int,
        ) as int32_t;
    } else {
        result = find_pic_file(
            &raw mut pic_path,
            &raw mut bounds,
            pdf_box_type as ::core::ffi::c_int,
            page as ::core::ffi::c_int,
        ) as int32_t;
    }
    corners[0 as ::core::ffi::c_int as usize].x = bounds.x;
    corners[0 as ::core::ffi::c_int as usize].y = bounds.y;
    corners[1 as ::core::ffi::c_int as usize].x = corners[0 as ::core::ffi::c_int as usize].x;
    corners[1 as ::core::ffi::c_int as usize].y = bounds.y + bounds.ht;
    corners[2 as ::core::ffi::c_int as usize].x = bounds.x + bounds.wd;
    corners[2 as ::core::ffi::c_int as usize].y = corners[1 as ::core::ffi::c_int as usize].y;
    corners[3 as ::core::ffi::c_int as usize].x = corners[2 as ::core::ffi::c_int as usize].x;
    corners[3 as ::core::ffi::c_int as usize].y = corners[0 as ::core::ffi::c_int as usize].y;
    x_size_req = 0.0f64;
    y_size_req = 0.0f64;
    make_identity(&raw mut t);
    check_keywords = true_0 != 0;
    while check_keywords {
        if scan_keyword(b"scaled\0" as *const u8 as *const ::core::ffi::c_char) {
            scan_int();
            if x_size_req == 0.0f64 && y_size_req == 0.0f64 {
                make_scale(
                    &raw mut t2,
                    cur_val as ::core::ffi::c_double / 1000.0f64,
                    cur_val as ::core::ffi::c_double / 1000.0f64,
                );
                let mut for_end: int32_t = 0;
                i = 0 as small_number;
                for_end = 3 as ::core::ffi::c_int as int32_t;
                if i as int32_t <= for_end {
                    loop {
                        transform_point(
                            (&raw mut corners as *mut real_point).offset(i as isize)
                                as *mut real_point,
                            &raw mut t2,
                        );
                        let fresh0 = i;
                        i = i + 1;
                        if !((fresh0 as int32_t) < for_end) {
                            break;
                        }
                    }
                }
                transform_concat(&raw mut t, &raw mut t2);
            }
        } else if scan_keyword(b"xscaled\0" as *const u8 as *const ::core::ffi::c_char) {
            scan_int();
            if x_size_req == 0.0f64 && y_size_req == 0.0f64 {
                make_scale(
                    &raw mut t2,
                    cur_val as ::core::ffi::c_double / 1000.0f64,
                    1.0f64,
                );
                let mut for_end_0: int32_t = 0;
                i = 0 as small_number;
                for_end_0 = 3 as ::core::ffi::c_int as int32_t;
                if i as int32_t <= for_end_0 {
                    loop {
                        transform_point(
                            (&raw mut corners as *mut real_point).offset(i as isize)
                                as *mut real_point,
                            &raw mut t2,
                        );
                        let fresh1 = i;
                        i = i + 1;
                        if !((fresh1 as int32_t) < for_end_0) {
                            break;
                        }
                    }
                }
                transform_concat(&raw mut t, &raw mut t2);
            }
        } else if scan_keyword(b"yscaled\0" as *const u8 as *const ::core::ffi::c_char) {
            scan_int();
            if x_size_req == 0.0f64 && y_size_req == 0.0f64 {
                make_scale(
                    &raw mut t2,
                    1.0f64,
                    cur_val as ::core::ffi::c_double / 1000.0f64,
                );
                let mut for_end_1: int32_t = 0;
                i = 0 as small_number;
                for_end_1 = 3 as ::core::ffi::c_int as int32_t;
                if i as int32_t <= for_end_1 {
                    loop {
                        transform_point(
                            (&raw mut corners as *mut real_point).offset(i as isize)
                                as *mut real_point,
                            &raw mut t2,
                        );
                        let fresh2 = i;
                        i = i + 1;
                        if !((fresh2 as int32_t) < for_end_1) {
                            break;
                        }
                    }
                }
                transform_concat(&raw mut t, &raw mut t2);
            }
        } else if scan_keyword(b"width\0" as *const u8 as *const ::core::ffi::c_char) {
            scan_dimen(false_0 != 0, false_0 != 0, false_0 != 0);
            if cur_val <= 0 as int32_t {
                error_here_with_diagnostic(
                    b"Improper image \0" as *const u8 as *const ::core::ffi::c_char,
                );
                print_cstr(b"size (\0" as *const u8 as *const ::core::ffi::c_char);
                print_scaled(cur_val as scaled_t);
                print_cstr(b"pt) will be ignored\0" as *const u8 as *const ::core::ffi::c_char);
                capture_to_diagnostic(::core::ptr::null_mut::<ttbc_diagnostic_t>());
                help_ptr = 2 as ::core::ffi::c_uchar;
                help_line[1 as ::core::ffi::c_int as usize] =
                    b"I can't scale images to zero or negative sizes,\0" as *const u8
                        as *const ::core::ffi::c_char;
                help_line[0 as ::core::ffi::c_int as usize] =
                    b"so I'm ignoring this.\0" as *const u8 as *const ::core::ffi::c_char;
                error();
            } else {
                x_size_req = Fix2D(cur_val as Fixed);
            }
        } else if scan_keyword(b"height\0" as *const u8 as *const ::core::ffi::c_char) {
            scan_dimen(false_0 != 0, false_0 != 0, false_0 != 0);
            if cur_val <= 0 as int32_t {
                error_here_with_diagnostic(
                    b"Improper image \0" as *const u8 as *const ::core::ffi::c_char,
                );
                print_cstr(b"size (\0" as *const u8 as *const ::core::ffi::c_char);
                print_scaled(cur_val as scaled_t);
                print_cstr(b"pt) will be ignored\0" as *const u8 as *const ::core::ffi::c_char);
                capture_to_diagnostic(::core::ptr::null_mut::<ttbc_diagnostic_t>());
                help_ptr = 2 as ::core::ffi::c_uchar;
                help_line[1 as ::core::ffi::c_int as usize] =
                    b"I can't scale images to zero or negative sizes,\0" as *const u8
                        as *const ::core::ffi::c_char;
                help_line[0 as ::core::ffi::c_int as usize] =
                    b"so I'm ignoring this.\0" as *const u8 as *const ::core::ffi::c_char;
                error();
            } else {
                y_size_req = Fix2D(cur_val as Fixed);
            }
        } else if scan_keyword(b"rotated\0" as *const u8 as *const ::core::ffi::c_char) {
            scan_decimal();
            if x_size_req != 0.0f64 || y_size_req != 0.0f64 {
                xmin = 1000000.0f64;
                xmax = -(xmin as int32_t) as ::core::ffi::c_double;
                ymin = xmin;
                ymax = xmax;
                let mut for_end_2: int32_t = 0;
                i = 0 as small_number;
                for_end_2 = 3 as ::core::ffi::c_int as int32_t;
                if i as int32_t <= for_end_2 {
                    loop {
                        if (corners[i as usize].x as ::core::ffi::c_double) < xmin {
                            xmin = corners[i as usize].x as ::core::ffi::c_double;
                        }
                        if corners[i as usize].x as ::core::ffi::c_double > xmax {
                            xmax = corners[i as usize].x as ::core::ffi::c_double;
                        }
                        if (corners[i as usize].y as ::core::ffi::c_double) < ymin {
                            ymin = corners[i as usize].y as ::core::ffi::c_double;
                        }
                        if corners[i as usize].y as ::core::ffi::c_double > ymax {
                            ymax = corners[i as usize].y as ::core::ffi::c_double;
                        }
                        let fresh3 = i;
                        i = i + 1;
                        if !((fresh3 as int32_t) < for_end_2) {
                            break;
                        }
                    }
                }
                if x_size_req == 0.0f64 {
                    make_scale(
                        &raw mut t2,
                        y_size_req / (ymax - ymin),
                        y_size_req / (ymax - ymin),
                    );
                } else if y_size_req == 0.0f64 {
                    make_scale(
                        &raw mut t2,
                        x_size_req / (xmax - xmin),
                        x_size_req / (xmax - xmin),
                    );
                } else {
                    make_scale(
                        &raw mut t2,
                        x_size_req / (xmax - xmin),
                        y_size_req / (ymax - ymin),
                    );
                }
                let mut for_end_3: int32_t = 0;
                i = 0 as small_number;
                for_end_3 = 3 as ::core::ffi::c_int as int32_t;
                if i as int32_t <= for_end_3 {
                    loop {
                        transform_point(
                            (&raw mut corners as *mut real_point).offset(i as isize)
                                as *mut real_point,
                            &raw mut t2,
                        );
                        let fresh4 = i;
                        i = i + 1;
                        if !((fresh4 as int32_t) < for_end_3) {
                            break;
                        }
                    }
                }
                x_size_req = 0.0f64;
                y_size_req = 0.0f64;
                transform_concat(&raw mut t, &raw mut t2);
            }
            make_rotation(&raw mut t2, Fix2D(cur_val as Fixed) * M_PI / 180.0f64);
            let mut for_end_4: int32_t = 0;
            i = 0 as small_number;
            for_end_4 = 3 as ::core::ffi::c_int as int32_t;
            if i as int32_t <= for_end_4 {
                loop {
                    transform_point(
                        (&raw mut corners as *mut real_point).offset(i as isize) as *mut real_point,
                        &raw mut t2,
                    );
                    let fresh5 = i;
                    i = i + 1;
                    if !((fresh5 as int32_t) < for_end_4) {
                        break;
                    }
                }
            }
            xmin = 1000000.0f64;
            xmax = -(xmin as int32_t) as ::core::ffi::c_double;
            ymin = xmin;
            ymax = xmax;
            let mut for_end_5: int32_t = 0;
            i = 0 as small_number;
            for_end_5 = 3 as ::core::ffi::c_int as int32_t;
            if i as int32_t <= for_end_5 {
                loop {
                    if (corners[i as usize].x as ::core::ffi::c_double) < xmin {
                        xmin = corners[i as usize].x as ::core::ffi::c_double;
                    }
                    if corners[i as usize].x as ::core::ffi::c_double > xmax {
                        xmax = corners[i as usize].x as ::core::ffi::c_double;
                    }
                    if (corners[i as usize].y as ::core::ffi::c_double) < ymin {
                        ymin = corners[i as usize].y as ::core::ffi::c_double;
                    }
                    if corners[i as usize].y as ::core::ffi::c_double > ymax {
                        ymax = corners[i as usize].y as ::core::ffi::c_double;
                    }
                    let fresh6 = i;
                    i = i + 1;
                    if !((fresh6 as int32_t) < for_end_5) {
                        break;
                    }
                }
            }
            corners[0 as ::core::ffi::c_int as usize].x = xmin as ::core::ffi::c_float;
            corners[0 as ::core::ffi::c_int as usize].y = ymin as ::core::ffi::c_float;
            corners[1 as ::core::ffi::c_int as usize].x = xmin as ::core::ffi::c_float;
            corners[1 as ::core::ffi::c_int as usize].y = ymax as ::core::ffi::c_float;
            corners[2 as ::core::ffi::c_int as usize].x = xmax as ::core::ffi::c_float;
            corners[2 as ::core::ffi::c_int as usize].y = ymax as ::core::ffi::c_float;
            corners[3 as ::core::ffi::c_int as usize].x = xmax as ::core::ffi::c_float;
            corners[3 as ::core::ffi::c_int as usize].y = ymin as ::core::ffi::c_float;
            transform_concat(&raw mut t, &raw mut t2);
        } else {
            check_keywords = false_0 != 0;
        }
    }
    if x_size_req != 0.0f64 || y_size_req != 0.0f64 {
        xmin = 1000000.0f64;
        xmax = -(xmin as int32_t) as ::core::ffi::c_double;
        ymin = xmin;
        ymax = xmax;
        let mut for_end_6: int32_t = 0;
        i = 0 as small_number;
        for_end_6 = 3 as ::core::ffi::c_int as int32_t;
        if i as int32_t <= for_end_6 {
            loop {
                if (corners[i as usize].x as ::core::ffi::c_double) < xmin {
                    xmin = corners[i as usize].x as ::core::ffi::c_double;
                }
                if corners[i as usize].x as ::core::ffi::c_double > xmax {
                    xmax = corners[i as usize].x as ::core::ffi::c_double;
                }
                if (corners[i as usize].y as ::core::ffi::c_double) < ymin {
                    ymin = corners[i as usize].y as ::core::ffi::c_double;
                }
                if corners[i as usize].y as ::core::ffi::c_double > ymax {
                    ymax = corners[i as usize].y as ::core::ffi::c_double;
                }
                let fresh7 = i;
                i = i + 1;
                if !((fresh7 as int32_t) < for_end_6) {
                    break;
                }
            }
        }
        if x_size_req == 0.0f64 {
            make_scale(
                &raw mut t2,
                y_size_req / (ymax - ymin),
                y_size_req / (ymax - ymin),
            );
        } else if y_size_req == 0.0f64 {
            make_scale(
                &raw mut t2,
                x_size_req / (xmax - xmin),
                x_size_req / (xmax - xmin),
            );
        } else {
            make_scale(
                &raw mut t2,
                x_size_req / (xmax - xmin),
                y_size_req / (ymax - ymin),
            );
        }
        let mut for_end_7: int32_t = 0;
        i = 0 as small_number;
        for_end_7 = 3 as ::core::ffi::c_int as int32_t;
        if i as int32_t <= for_end_7 {
            loop {
                transform_point(
                    (&raw mut corners as *mut real_point).offset(i as isize) as *mut real_point,
                    &raw mut t2,
                );
                let fresh8 = i;
                i = i + 1;
                if !((fresh8 as int32_t) < for_end_7) {
                    break;
                }
            }
        }
        x_size_req = 0.0f64;
        y_size_req = 0.0f64;
        transform_concat(&raw mut t, &raw mut t2);
    }
    xmin = 1000000.0f64;
    xmax = -(xmin as int32_t) as ::core::ffi::c_double;
    ymin = xmin;
    ymax = xmax;
    let mut for_end_8: int32_t = 0;
    i = 0 as small_number;
    for_end_8 = 3 as ::core::ffi::c_int as int32_t;
    if i as int32_t <= for_end_8 {
        loop {
            if (corners[i as usize].x as ::core::ffi::c_double) < xmin {
                xmin = corners[i as usize].x as ::core::ffi::c_double;
            }
            if corners[i as usize].x as ::core::ffi::c_double > xmax {
                xmax = corners[i as usize].x as ::core::ffi::c_double;
            }
            if (corners[i as usize].y as ::core::ffi::c_double) < ymin {
                ymin = corners[i as usize].y as ::core::ffi::c_double;
            }
            if corners[i as usize].y as ::core::ffi::c_double > ymax {
                ymax = corners[i as usize].y as ::core::ffi::c_double;
            }
            let fresh9 = i;
            i = i + 1;
            if !((fresh9 as int32_t) < for_end_8) {
                break;
            }
        }
    }
    make_translation(
        &raw mut t2,
        (-(xmin as int32_t) * 72 as int32_t) as ::core::ffi::c_double / 72.27f64,
        (-(ymin as int32_t) * 72 as int32_t) as ::core::ffi::c_double / 72.27f64,
    );
    transform_concat(&raw mut t, &raw mut t2);
    if result == 0 as int32_t {
        new_whatsit(
            PIC_NODE as small_number,
            (PIC_NODE_SIZE as size_t).wrapping_add(
                strlen(pic_path)
                    .wrapping_add(::core::mem::size_of::<memory_word>() as size_t)
                    .wrapping_sub(1 as size_t)
                    .wrapping_div(::core::mem::size_of::<memory_word>() as size_t),
            ) as small_number,
        );
        if is_pdf {
            (*mem.offset(cur_list.tail as isize)).b16.s0 = PDF_NODE as uint16_t;
        }
        (*mem.offset((cur_list.tail + 4 as int32_t) as isize))
            .b16
            .s1 = strlen(pic_path) as uint16_t;
        (*mem.offset((cur_list.tail + 4 as int32_t) as isize))
            .b16
            .s0 = page as uint16_t;
        (*mem.offset((cur_list.tail + 8 as int32_t) as isize))
            .b16
            .s1 = pdf_box_type as uint16_t;
        (*mem.offset((cur_list.tail + 1 as int32_t) as isize))
            .b32
            .s1 = D2Fix(xmax - xmin) as int32_t;
        (*mem.offset((cur_list.tail + 3 as int32_t) as isize))
            .b32
            .s1 = D2Fix(ymax - ymin) as int32_t;
        (*mem.offset((cur_list.tail + 2 as int32_t) as isize))
            .b32
            .s1 = 0 as ::core::ffi::c_int as int32_t;
        (*mem.offset((cur_list.tail + 5 as int32_t) as isize))
            .b32
            .s0 = D2Fix(t.a) as int32_t;
        (*mem.offset((cur_list.tail + 5 as int32_t) as isize))
            .b32
            .s1 = D2Fix(t.b) as int32_t;
        (*mem.offset((cur_list.tail + 6 as int32_t) as isize))
            .b32
            .s0 = D2Fix(t.c) as int32_t;
        (*mem.offset((cur_list.tail + 6 as int32_t) as isize))
            .b32
            .s1 = D2Fix(t.d) as int32_t;
        (*mem.offset((cur_list.tail + 7 as int32_t) as isize))
            .b32
            .s0 = D2Fix(t.x) as int32_t;
        (*mem.offset((cur_list.tail + 7 as int32_t) as isize))
            .b32
            .s1 = D2Fix(t.y) as int32_t;
        memcpy(
            mem.offset((cur_list.tail + PIC_NODE_SIZE as int32_t) as isize) as *mut memory_word
                as *mut ::core::ffi::c_uchar as *mut ::core::ffi::c_void,
            pic_path as *const ::core::ffi::c_void,
            strlen(pic_path),
        );
        free(pic_path as *mut ::core::ffi::c_void);
    } else {
        error_here_with_diagnostic(
            b"Unable to load picture or PDF file '\0" as *const u8 as *const ::core::ffi::c_char,
        );
        print_file_name(cur_name as int32_t, cur_area as int32_t, cur_ext as int32_t);
        print('\'' as i32);
        capture_to_diagnostic(::core::ptr::null_mut::<ttbc_diagnostic_t>());
        if result == -(43 as int32_t) {
            help_ptr = 2 as ::core::ffi::c_uchar;
            help_line[1 as ::core::ffi::c_int as usize] =
                b"The requested image couldn't be read because\0" as *const u8
                    as *const ::core::ffi::c_char;
            help_line[0 as ::core::ffi::c_int as usize] =
                b"the file was not found.\0" as *const u8 as *const ::core::ffi::c_char;
        } else {
            help_ptr = 2 as ::core::ffi::c_uchar;
            help_line[1 as ::core::ffi::c_int as usize] =
                b"The requested image couldn't be read because\0" as *const u8
                    as *const ::core::ffi::c_char;
            help_line[0 as ::core::ffi::c_int as usize] = b"it was not a recognized image format.\0"
                as *const u8
                as *const ::core::ffi::c_char;
        }
        error();
    };
}
pub const true_0: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const false_0: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
