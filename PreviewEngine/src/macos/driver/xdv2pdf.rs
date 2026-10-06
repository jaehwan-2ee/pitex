#[path = "../../shared/pdf_inclusion.rs"]
mod pdf_inclusion;

#[path = "../../shared/pdf_document.rs"]
mod pdf_document;
#[path = "../../shared/backend_options.rs"]
mod backend_options;
#[path = "../../shared/postscript_pdf.rs"]
mod postscript_pdf;
#[path = "../../shared/bitmap_fonts.rs"]
mod bitmap_fonts;
#[path = "../../shared/font_output.rs"]
mod font_output;

/* Pitex embedded preview engine — XDV to PDF conversion.
 *
 * Interprets DVI/XDV pages (Knuth's DVI format, XeTeX native-font opcodes
 * 252-254, virtual fonts) and the dvipdfmx-style specials emitted by the
 * LaTeX xetex drivers (graphics-def xetex.def, pgfsys-dvipdfmx/xetex.def),
 * producing a self-contained PDF. Independently written from the published
 * format descriptions and driver sources' documented special syntax; no
 * xdvipdfmx/dvipdfmx or MuPDF code. Pitex-authored (AGPL-3.0-or-later).
 *
 * ponytail: fonts are embedded whole (no subsetting) and compressed once
 * per session; subset them if very large CJK fonts make publishing slow. */
// Translated from driver/xdv2pdf.c with C2Rust 0.22.1.
extern "C" {
    fn font_map_reset(fc:*mut font_cache);
    fn font_map_line(fc:*mut font_cache,spec:*const ::core::ffi::c_char);
    fn font_map_file(fc:*mut font_cache,spec:*const ::core::ffi::c_char)->bool;
    pub type xdv_index;
    pub type pr_doc;
    pub type font_cache;
    pub type pdfw;
    pub type pdfw_import;
    fn malloc(__size: size_t) -> *mut ::core::ffi::c_void;
    fn calloc(__count: size_t, __size: size_t) -> *mut ::core::ffi::c_void;
    fn free(_: *mut ::core::ffi::c_void);
    fn realloc(__ptr: *mut ::core::ffi::c_void, __size: size_t) -> *mut ::core::ffi::c_void;
    fn abort() -> !;
    fn strtod(
        _: *const ::core::ffi::c_char,
        _: *mut *mut ::core::ffi::c_char,
    ) -> ::core::ffi::c_double;
    fn strtol(
        __str: *const ::core::ffi::c_char,
        __endptr: *mut *mut ::core::ffi::c_char,
        __base: ::core::ffi::c_int,
    ) -> ::core::ffi::c_long;
    fn memchr(
        __s: *const ::core::ffi::c_void,
        __c: ::core::ffi::c_int,
        __n: size_t,
    ) -> *mut ::core::ffi::c_void;
    fn memcmp(
        __s1: *const ::core::ffi::c_void,
        __s2: *const ::core::ffi::c_void,
        __n: size_t,
    ) -> ::core::ffi::c_int;
    fn memcpy(
        __dst: *mut ::core::ffi::c_void,
        __src: *const ::core::ffi::c_void,
        __n: size_t,
    ) -> *mut ::core::ffi::c_void;
    fn memset(
        __b: *mut ::core::ffi::c_void,
        __c: ::core::ffi::c_int,
        __len: size_t,
    ) -> *mut ::core::ffi::c_void;
    fn strchr(__s: *const ::core::ffi::c_char, __c: ::core::ffi::c_int)
        -> *mut ::core::ffi::c_char;
    fn strcmp(
        __s1: *const ::core::ffi::c_char,
        __s2: *const ::core::ffi::c_char,
    ) -> ::core::ffi::c_int;
    fn strlen(__s: *const ::core::ffi::c_char) -> size_t;
    fn strncmp(
        __s1: *const ::core::ffi::c_char,
        __s2: *const ::core::ffi::c_char,
        __n: size_t,
    ) -> ::core::ffi::c_int;
    fn strstr(
        __big: *const ::core::ffi::c_char,
        __little: *const ::core::ffi::c_char,
    ) -> *mut ::core::ffi::c_char;
    fn strdup(__s1: *const ::core::ffi::c_char) -> *mut ::core::ffi::c_char;
    fn pbuf_printf(b: *mut pbuf, fmt: *const ::core::ffi::c_char, ...);
    fn pbuf_real(b: *mut pbuf, v: ::core::ffi::c_double);
    fn pbuf_deflate(
        out: *mut pbuf,
        data: *const ::core::ffi::c_uchar,
        len: size_t,
        level: ::core::ffi::c_int,
    ) -> ::core::ffi::c_int;
    fn tbuf_drop(b: *mut tbuf);
    fn xdv_insn_length(
        p: *const ::core::ffi::c_uchar,
        end: *const ::core::ffi::c_uchar,
    ) -> ::core::ffi::c_long;
    fn xdv_index_page_count(x: *const xdv_index) -> ::core::ffi::c_int;
    fn xdv_index_page(x: *const xdv_index, i: ::core::ffi::c_int) -> *const xdv_page;
    fn xdv_index_preamble(
        x: *const xdv_index,
        num: *mut uint32_t,
        den: *mut uint32_t,
        mag: *mut uint32_t,
    ) -> bool;
    static mut _DefaultRuneLocale: _RuneLocale;
    fn __maskrune(_: __darwin_ct_rune_t, _: ::core::ffi::c_ulong) -> ::core::ffi::c_int;
    fn cos(_: ::core::ffi::c_double) -> ::core::ffi::c_double;
    fn sin(_: ::core::ffi::c_double) -> ::core::ffi::c_double;
    fn fabs(_: ::core::ffi::c_double) -> ::core::ffi::c_double;
    fn floor(_: ::core::ffi::c_double) -> ::core::ffi::c_double;
    fn sscanf(
        _: *const ::core::ffi::c_char,
        _: *const ::core::ffi::c_char,
        ...
    ) -> ::core::ffi::c_int;
    fn snprintf(
        __str: *mut ::core::ffi::c_char,
        __size: size_t,
        __format: *const ::core::ffi::c_char,
        ...
    ) -> ::core::ffi::c_int;
    fn font_cache_new(resolver: xdv_resolver) -> *mut font_cache;
    fn font_cache_free(fc: *mut font_cache);
    fn font_native(
        fc: *mut font_cache,
        name: *const ::core::ffi::c_char,
        index: ::core::ffi::c_int,
        warnings: *mut pbuf,
    ) -> *mut native_face;
    fn native_advance(f: *const native_face, gid: ::core::ffi::c_int) -> ::core::ffi::c_int;
    fn font_tfm(fc: *mut font_cache, name: *const ::core::ffi::c_char) -> *mut tfm_font;
    fn font_map_duplicate_count(fc:*mut font_cache)->i32;
    fn font_map_ps_lookup(fc:*mut font_cache,name:*const ::core::ffi::c_char)->*const map_entry;
    fn font_map_lookup(fc: *mut font_cache, tfm: *const ::core::ffi::c_char) -> *const map_entry;
    fn font_type1(
        fc: *mut font_cache,
        file: *const ::core::ffi::c_char,
        warnings: *mut pbuf,
    ) -> *mut type1_font;
    fn font_enc(fc: *mut font_cache, file: *const ::core::ffi::c_char) -> *mut enc_vector;
    fn font_vf(fc: *mut font_cache, name: *const ::core::ffi::c_char) -> *mut vf_font;
    fn image_encode(
        data: *const ::core::ffi::c_uchar,
        len: size_t,
        name: *const ::core::ffi::c_char,
        out: *mut pdf_image,
        warnings: *mut pbuf,
    ) -> ::core::ffi::c_int;
    fn image_free(img: *mut pdf_image);
    fn pr_open(data: *const ::core::ffi::c_uchar, len: size_t) -> *mut pr_doc;
    fn pr_close(doc: *mut pr_doc);
    fn pr_document_info(doc:*mut pr_doc)->*mut pr_obj;
    fn pr_version(doc:*mut pr_doc)->i32;
    fn pr_is_encrypted(doc: *mut pr_doc) -> bool;
    fn pr_page(
        doc: *mut pr_doc,
        index0: ::core::ffi::c_int,
        box_kind: ::core::ffi::c_int,
        out: *mut pr_page_info,
    ) -> bool;
    fn pr_normalize_page(doc: *mut pr_doc, page_arg: ::core::ffi::c_int) -> ::core::ffi::c_int;
    fn pr_resolve(doc: *mut pr_doc, obj: *mut pr_obj) -> *mut pr_obj;
    fn pr_get(doc: *mut pr_doc, dict: *mut pr_obj, key: *const ::core::ffi::c_char) -> *mut pr_obj;
    fn pr_stream_raw(
        doc: *mut pr_doc,
        stream: *mut pr_obj,
        len: *mut size_t,
    ) -> *const ::core::ffi::c_uchar;
    fn pr_stream_decode(doc: *mut pr_doc, stream: *mut pr_obj, out: *mut pbuf) -> bool;
    fn pr_rotation_matrix(rotate: ::core::ffi::c_int, m: *mut ::core::ffi::c_double);
    fn pr_transform_box(
        m: *const ::core::ffi::c_double,
        box_0: *const ::core::ffi::c_double,
        out: *mut ::core::ffi::c_double,
    );
    fn pdfw_new(out: *mut pbuf) -> *mut pdfw;
    fn pdfw_free(w: *mut pdfw);
    fn pdfw_out(w: *mut pdfw) -> *mut pbuf;
    fn pdfw_alloc(w: *mut pdfw) -> ::core::ffi::c_int;
    fn pdfw_begin(w: *mut pdfw, num: ::core::ffi::c_int);
    fn pdfw_end(w: *mut pdfw);
    fn pdfw_stream(
        w: *mut pdfw,
        num: ::core::ffi::c_int,
        dict_body: *const ::core::ffi::c_char,
        data: *const ::core::ffi::c_void,
        len: size_t,
        compress: bool,
    );
    fn pdfw_stream_deflated(
        w: *mut pdfw,
        num: ::core::ffi::c_int,
        dict_body: *const ::core::ffi::c_char,
        data: *const ::core::ffi::c_void,
        len: size_t,
    );
    fn pdfw_finish(w: *mut pdfw, root: ::core::ffi::c_int, info: ::core::ffi::c_int);
    fn pdfw_object_compression(w:*mut pdfw,level:i32);
    fn pdfw_configuration(w: *mut pdfw,major:i32,minor:i32,compression:i32);
    fn pdfw_trailer(w: *mut pdfw,body:*const ::core::ffi::c_char,length:usize);
    fn pdfw_minimum_version(w:*mut pdfw,major:i32,minor:i32);
    fn pdfw_effective_version(w:*mut pdfw)->i32;
    fn pdfw_name(b: *mut pbuf, name: *const ::core::ffi::c_char);
    fn pdfw_hexstring(b:*mut pbuf,s:*const ::core::ffi::c_uchar,len:size_t);
    fn pdfw_import_begin(w: *mut pdfw, src: *mut pr_doc) -> *mut pdfw_import;
    fn pdfw_import_redirect(imp:*mut pdfw_import,source:i32,destination:i32);
    fn pdfw_import_value(imp: *mut pdfw_import, b: *mut pbuf, obj: *mut pr_obj);
    fn pdfw_import_end(imp: *mut pdfw_import);
}
pub type __uint32_t = u32;
pub type __darwin_ct_rune_t = ::core::ffi::c_int;
pub type __darwin_size_t = usize;
pub type __darwin_wchar_t = ::libc::wchar_t;
pub type __darwin_rune_t = __darwin_wchar_t;
pub type int32_t = i32;
pub type size_t = __darwin_size_t;
pub type uint32_t = u32;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct pbuf {
    pub data: *mut ::core::ffi::c_uchar,
    pub len: size_t,
    pub cap: size_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct tbuf {
    pub data: *mut ::core::ffi::c_uchar,
    pub len: size_t,
    pub cap: size_t,
    pub refs: ::core::ffi::c_int,
}
pub type C2RustUnnamed = ::core::ffi::c_uint;
pub const XDV_TEXT_AND_GLYPHS: C2RustUnnamed = 254;
pub const XDV_GLYPHS: C2RustUnnamed = 253;
pub const XDV_NATIVE_FONT_DEF: C2RustUnnamed = 252;
pub const DVI_POST_POST: C2RustUnnamed = 249;
pub const DVI_POST: C2RustUnnamed = 248;
pub const DVI_PRE: C2RustUnnamed = 247;
pub const DVI_FNT_DEF1: C2RustUnnamed = 243;
pub const DVI_XXX1: C2RustUnnamed = 239;
pub const DVI_FNT1: C2RustUnnamed = 235;
pub const DVI_FNT_NUM_0: C2RustUnnamed = 171;
pub const DVI_Z1: C2RustUnnamed = 167;
pub const DVI_Z0: C2RustUnnamed = 166;
pub const DVI_Y1: C2RustUnnamed = 162;
pub const DVI_Y0: C2RustUnnamed = 161;
pub const DVI_DOWN1: C2RustUnnamed = 157;
pub const DVI_X1: C2RustUnnamed = 153;
pub const DVI_X0: C2RustUnnamed = 152;
pub const DVI_W1: C2RustUnnamed = 148;
pub const DVI_W0: C2RustUnnamed = 147;
pub const DVI_RIGHT1: C2RustUnnamed = 143;
pub const DVI_POP: C2RustUnnamed = 142;
pub const DVI_PUSH: C2RustUnnamed = 141;
pub const DVI_EOP: C2RustUnnamed = 140;
pub const DVI_BOP: C2RustUnnamed = 139;
pub const DVI_NOP: C2RustUnnamed = 138;
pub const DVI_PUT_RULE: C2RustUnnamed = 137;
pub const DVI_PUT1: C2RustUnnamed = 133;
pub const DVI_SET_RULE: C2RustUnnamed = 132;
pub const DVI_SET1: C2RustUnnamed = 128;
pub const DVI_SET_CHAR_0: C2RustUnnamed = 0;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct xdv_page {
    pub bop: size_t,
    pub eop_end: size_t,
}
pub type xdv_res_kind = ::core::ffi::c_uint;
pub const RES_IMAGE: xdv_res_kind = 6;
pub const RES_TYPE1: xdv_res_kind = 5;
pub const RES_MAP: xdv_res_kind = 4;
pub const RES_ENC: xdv_res_kind = 3;
pub const RES_VF: xdv_res_kind = 2;
pub const RES_TFM: xdv_res_kind = 1;
pub const RES_NATIVE_FONT: xdv_res_kind = 0;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct xdv_resolver {
    pub env: *mut ::core::ffi::c_void,
    pub load: Option<
        unsafe extern "C" fn(
            *mut ::core::ffi::c_void,
            *const ::core::ffi::c_char,
            xdv_res_kind,
        ) -> *mut tbuf,
    >,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct xdv_range {
    pub data: *const ::core::ffi::c_uchar,
    pub len: size_t,
    pub index: *const xdv_index,
    pub first: ::core::ffi::c_int,
    pub count: ::core::ffi::c_int,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct xdv2pdf {
    pub res: xdv_resolver,
    pub fonts: *mut font_cache,
    pub images: *mut cached_image,
    pub epoch: ::core::ffi::c_int,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct cached_image {
    pub next: *mut cached_image,
    pub file: *mut tbuf,
    pub config_key: u64,
    pub filename: *mut ::core::ffi::c_char,
    pub img: pdf_image,
    pub pdf: bool,
    pub doc: *mut pr_doc,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct pdf_image {
    pub ok: bool,
    pub width: ::core::ffi::c_int,
    pub height: ::core::ffi::c_int,
    pub xdpi: ::core::ffi::c_double,
    pub ydpi: ::core::ffi::c_double,
    pub dict: [::core::ffi::c_char; 192],
    pub dct: bool,
    pub data: pbuf,
    pub smask: pbuf,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct conv_ctx {
    pub font_output: *mut font_output::State,
    pub document: *mut pdf_document::Document,
    pub this_page: i32,
    pub expansion: f64,
    pub postscript: *mut crate::shared_postscript::Interpreter,
    pub w: *mut xdv2pdf,
    pub pw: *mut pdfw,
    pub warnings: *mut pbuf,
    pub pfonts: *mut pdf_font,
    pub npfonts: ::core::ffi::c_int,
    pub cappfonts: ::core::ffi::c_int,
    pub xobjs: *mut xobj_use,
    pub nxobjs: ::core::ffi::c_int,
    pub capxobjs: ::core::ffi::c_int,
    pub named: *mut named_obj,
    pub nnamed: ::core::ffi::c_int,
    pub capnamed: ::core::ffi::c_int,
    pub page_res: resources,
    pub font_dict_obj: ::core::ffi::c_int,
    pub xobject_dict_obj: ::core::ffi::c_int,
    pub resources_obj: ::core::ffi::c_int,
    pub conv: ::core::ffi::c_double,
    pub page_w: ::core::ffi::c_double,
    pub page_h: ::core::ffi::c_double,
    pub default_w: ::core::ffi::c_double,
    pub default_h: ::core::ffi::c_double,
    pub ox: ::core::ffi::c_double,
    pub oy: ::core::ffi::c_double,
    pub off_stack: [[::core::ffi::c_double; 2]; 64],
    pub off_depth: ::core::ffi::c_int,
    pub targets: [target; 8],
    pub ntargets: ::core::ffi::c_int,
    pub q_depth: ::core::ffi::c_int,
    pub render: bool,
    pub cs: [colorstack; 16],
    pub old_colors: [*mut ::core::ffi::c_char; 64],
    pub old_depth: ::core::ffi::c_int,
    pub background: [::core::ffi::c_double; 3],
    pub has_background: bool,
    pub in_bt: bool,
    pub in_tj: bool,
    pub pos_valid: bool,
    pub t_font: ::core::ffi::c_int,
    pub t_size: ::core::ffi::c_double,
    pub t_a: ::core::ffi::c_double,
    pub t_c: ::core::ffi::c_double,
    pub t_x: ::core::ffi::c_double,
    pub t_y: ::core::ffi::c_double,
    pub warned: [[::core::ffi::c_char; 48]; 64],
    pub nwarned: ::core::ffi::c_int,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct colorstack {
    pub ops: [*mut ::core::ffi::c_char; 64],
    pub depth: ::core::ffi::c_int,
    pub init: *mut ::core::ffi::c_char,
    pub page: bool,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct target {
    pub content: pbuf,
    pub res: resources,
    pub form_index: usize,
    pub bbox: [::core::ffi::c_double; 4],
    pub ox: ::core::ffi::c_double,
    pub oy: ::core::ffi::c_double,
    pub q_base: ::core::ffi::c_int,
    pub saved_render: bool,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct named_obj {
    pub name: *mut ::core::ffi::c_char,
    pub obj: ::core::ffi::c_int,
    pub kind: named_kind,
    pub dict: kvlist,
    pub array: pbuf,
    pub written: bool,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct kvlist {
    pub items: *mut kv,
    pub n: ::core::ffi::c_int,
    pub cap: ::core::ffi::c_int,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct kv {
    pub key: *mut ::core::ffi::c_char,
    pub value: *mut ::core::ffi::c_char,
}
pub type named_kind = ::core::ffi::c_uint;
pub const NO_FORM: named_kind = 5;
pub const NO_STREAM: named_kind = 4;
pub const NO_RAW: named_kind = 3;
pub const NO_ARRAY: named_kind = 2;
pub const NO_DICT: named_kind = 1;
pub const NO_UNDEF: named_kind = 0;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct resources {
    pub cat: [kvlist; 7],
    pub cat_ref: [*mut ::core::ffi::c_char; 7],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct xobj_use {
    pub ci: *mut cached_image,
    pub page: ::core::ffi::c_int,
    pub box_0: ::core::ffi::c_int,
    pub obj: ::core::ffi::c_int,
    pub name: [::core::ffi::c_char; 96],
    pub w: ::core::ffi::c_double,
    pub h: ::core::ffi::c_double,
    pub bbox: [::core::ffi::c_double; 4],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct pdf_font {
    pub bitmap: bool,
    pub font_id: i32,
    pub vertical: bool,
    pub native: bool,
    pub nf: *mut native_face,
    pub tfm: *mut tfm_font,
    pub t1: *mut type1_font,
    pub enc: *mut enc_vector,
    pub obj: ::core::ffi::c_int,
    pub used: *mut ::core::ffi::c_uchar,
    pub nused: ::core::ffi::c_int,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct enc_vector {
    pub ok: bool,
    pub glyph: [*mut ::core::ffi::c_char; 256],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct type1_font {
    pub ok: bool,
    pub data: *mut ::core::ffi::c_uchar,
    pub len1: size_t,
    pub len2: size_t,
    pub len3: size_t,
    pub bbox: [::core::ffi::c_double; 4],
    pub fontname: [::core::ffi::c_char; 128],
    pub deflated: pbuf,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct tfm_font {
    pub ok: bool,
    pub bc: ::core::ffi::c_int,
    pub ec: ::core::ffi::c_int,
    pub checksum: uint32_t,
    pub design: int32_t,
    pub width: [int32_t; 256],
    pub exists: [bool; 256],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct native_face {
    pub ok: bool,
    pub cff: bool,
    pub sfnt: *mut ::core::ffi::c_uchar,
    pub sfnt_len: size_t,
    pub units_per_em: ::core::ffi::c_int,
    pub num_glyphs: ::core::ffi::c_int,
    pub num_hmetrics: ::core::ffi::c_int,
    pub hmtx: *const ::core::ffi::c_uchar,
    pub hmtx_len: size_t,
    pub to_unicode: *mut uint32_t,
    pub bbox: [::core::ffi::c_double; 4],
    pub ascent: ::core::ffi::c_double,
    pub descent: ::core::ffi::c_double,
    pub cap_height: ::core::ffi::c_double,
    pub italic_angle: ::core::ffi::c_double,
    pub psname: [::core::ffi::c_char; 128],
    pub deflated: pbuf,
}
pub const NCAT: C2RustUnnamed_8 = 7;
pub const CAT_XOBJECT: C2RustUnnamed_8 = 4;
pub const CAT_FONT: C2RustUnnamed_8 = 5;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct pr_obj {
    pub type_0: pr_type,
    pub u: C2RustUnnamed_0,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union C2RustUnnamed_0 {
    pub b: ::core::ffi::c_int,
    pub i: ::core::ffi::c_longlong,
    pub r: ::core::ffi::c_double,
    pub str_0: C2RustUnnamed_5,
    pub arr: C2RustUnnamed_4,
    pub dict: C2RustUnnamed_3,
    pub ref_0: C2RustUnnamed_2,
    pub stream: C2RustUnnamed_1,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_1 {
    pub dict: *mut pr_obj,
    pub offset: size_t,
    pub length: size_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_2 {
    pub num: ::core::ffi::c_int,
    pub gen: ::core::ffi::c_int,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_3 {
    pub keys: *mut *mut ::core::ffi::c_char,
    pub vals: *mut *mut pr_obj,
    pub n: ::core::ffi::c_int,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_4 {
    pub items: *mut *mut pr_obj,
    pub n: ::core::ffi::c_int,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_5 {
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
#[derive(Copy, Clone)]
#[repr(C)]
pub struct pr_page_info {
    pub box_0: [::core::ffi::c_double; 4],
    pub rotate: ::core::ffi::c_int,
    pub page: *mut pr_obj,
    pub resources: *mut pr_obj,
}
pub const PR_BOX_CROP: C2RustUnnamed_7 = 1;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct font_table {
    pub n: ::core::ffi::c_int,
    pub cap: ::core::ffi::c_int,
    pub slots: *mut *mut font_slot,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct font_slot {
    pub letterspace: f64,
    pub is_virtual: bool,
    pub vertical: bool,
    pub k: int32_t,
    pub kind: fs_kind,
    pub size: ::core::ffi::c_double,
    pub name: *mut ::core::ffi::c_char,
    pub tfm: *mut tfm_font,
    pub map: *const map_entry,
    pub t1: *mut type1_font,
    pub enc: *mut enc_vector,
    pub vf: *mut vf_font,
    pub vf_local: *mut font_table,
    pub vf_default: ::core::ffi::c_int,
    pub nf: *mut native_face,
    pub colored: bool,
    pub rgba: [::core::ffi::c_double; 4],
    pub extend: ::core::ffi::c_double,
    pub slant: ::core::ffi::c_double,
    pub embolden: ::core::ffi::c_double,
    pub pdf_font: ::core::ffi::c_int,
    pub publish_epoch: ::core::ffi::c_int,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct vf_font {
    pub ok: bool,
    pub nfonts: ::core::ffi::c_int,
    pub fonts: *mut vf_fontdef,
    pub chars: [vf_char; 256],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct vf_char {
    pub packet: *const ::core::ffi::c_uchar,
    pub len: uint32_t,
    pub width_fix: int32_t,
    pub exists: bool,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct vf_fontdef {
    pub k: int32_t,
    pub scaled_fix: int32_t,
    pub name: *mut ::core::ffi::c_char,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct map_entry {
    pub tfm: *mut ::core::ffi::c_char,
    pub psname: *mut ::core::ffi::c_char,
    pub fontfile: *mut ::core::ffi::c_char,
    pub encfile: *mut ::core::ffi::c_char,
    pub slant: ::core::ffi::c_double,
    pub extend: ::core::ffi::c_double,
}
pub type fs_kind = ::core::ffi::c_uint;
pub const FS_BITMAP: fs_kind = 4;
pub const FS_VF: fs_kind = 3;
pub const FS_TYPE1: fs_kind = 2;
pub const FS_NATIVE: fs_kind = 1;
pub const FS_MISSING: fs_kind = 0;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct dvi_state {
    pub fonts: *mut font_table,
    pub font: *mut font_slot,
    pub scale: ::core::ffi::c_double,
    pub r: dvi_regs,
    pub stack: [dvi_regs; 1024],
    pub sp: ::core::ffi::c_int,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct dvi_regs {
    pub h: ::core::ffi::c_double,
    pub v: ::core::ffi::c_double,
    pub w: ::core::ffi::c_double,
    pub x: ::core::ffi::c_double,
    pub y: ::core::ffi::c_double,
    pub z: ::core::ffi::c_double,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _RuneLocale {
    pub __magic: [::core::ffi::c_char; 8],
    pub __encoding: [::core::ffi::c_char; 32],
    pub __sgetrune: Option<
        unsafe extern "C" fn(
            *const ::core::ffi::c_char,
            __darwin_size_t,
            *mut *const ::core::ffi::c_char,
        ) -> __darwin_rune_t,
    >,
    pub __sputrune: Option<
        unsafe extern "C" fn(
            __darwin_rune_t,
            *mut ::core::ffi::c_char,
            __darwin_size_t,
            *mut *mut ::core::ffi::c_char,
        ) -> ::core::ffi::c_int,
    >,
    pub __invalid_rune: __darwin_rune_t,
    pub __runetype: [__uint32_t; 256],
    pub __maplower: [__darwin_rune_t; 256],
    pub __mapupper: [__darwin_rune_t; 256],
    pub __runetype_ext: _RuneRange,
    pub __maplower_ext: _RuneRange,
    pub __mapupper_ext: _RuneRange,
    pub __variable: *mut ::core::ffi::c_void,
    pub __variable_len: ::core::ffi::c_int,
    pub __ncharclasses: ::core::ffi::c_int,
    pub __charclasses: *mut _RuneCharClass,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _RuneCharClass {
    pub __name: [::core::ffi::c_char; 14],
    pub __mask: __uint32_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _RuneRange {
    pub __nranges: ::core::ffi::c_int,
    pub __ranges: *mut _RuneEntry,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _RuneEntry {
    pub __min: __darwin_rune_t,
    pub __max: __darwin_rune_t,
    pub __map: __darwin_rune_t,
    pub __types: *mut __uint32_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mat {
    pub a: ::core::ffi::c_double,
    pub b: ::core::ffi::c_double,
    pub c: ::core::ffi::c_double,
    pub d: ::core::ffi::c_double,
    pub e: ::core::ffi::c_double,
    pub f: ::core::ffi::c_double,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_6 {
    pub u: *const ::core::ffi::c_char,
    pub f: ::core::ffi::c_double,
}
pub type C2RustUnnamed_7 = ::core::ffi::c_uint;
pub const PR_BOX_ART: C2RustUnnamed_7 = 5;
pub const PR_BOX_TRIM: C2RustUnnamed_7 = 4;
pub const PR_BOX_BLEED: C2RustUnnamed_7 = 3;
pub const PR_BOX_MEDIA: C2RustUnnamed_7 = 2;
pub type C2RustUnnamed_8 = ::core::ffi::c_uint;
pub const CAT_PROPERTIES: C2RustUnnamed_8 = 6;
pub const CAT_SHADING: C2RustUnnamed_8 = 3;
pub const CAT_PATTERN: C2RustUnnamed_8 = 2;
pub const CAT_COLORSPACE: C2RustUnnamed_8 = 1;
pub const CAT_EXTGSTATE: C2RustUnnamed_8 = 0;
pub const __DARWIN_NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
pub const true_0: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const false_0: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
#[inline]
unsafe extern "C" fn pbuf_reserve(mut b: *mut pbuf, mut extra: size_t) {
    if (*b).len.wrapping_add(extra) <= (*b).cap {
        return;
    }
    let mut cap: size_t = if (*b).cap != 0 {
        (*b).cap
    } else {
        256 as size_t
    };
    while cap < (*b).len.wrapping_add(extra) {
        cap = cap.wrapping_mul(2 as size_t);
    }
    let mut p: *mut ::core::ffi::c_uchar =
        realloc((*b).data as *mut ::core::ffi::c_void, cap) as *mut ::core::ffi::c_uchar;
    if p.is_null() {
        abort();
    }
    (*b).data = p;
    (*b).cap = cap;
}
#[inline]
unsafe extern "C" fn pbuf_append(
    mut b: *mut pbuf,
    mut data: *const ::core::ffi::c_void,
    mut len: size_t,
) {
    if len == 0 {
        return;
    }
    pbuf_reserve(b, len);
    memcpy(
        (*b).data.offset((*b).len as isize) as *mut ::core::ffi::c_void,
        data,
        len,
    );
    (*b).len = (*b).len.wrapping_add(len);
}
#[inline]
unsafe extern "C" fn pbuf_putc(mut b: *mut pbuf, mut c: ::core::ffi::c_int) {
    pbuf_reserve(b, 1 as size_t);
    let fresh0 = (*b).len;
    (*b).len = (*b).len.wrapping_add(1);
    *(*b).data.offset(fresh0 as isize) = c as ::core::ffi::c_uchar;
}
#[inline]
unsafe extern "C" fn pbuf_puts(mut b: *mut pbuf, mut s: *const ::core::ffi::c_char) {
    pbuf_append(b, s as *const ::core::ffi::c_void, strlen(s));
}
#[inline]
unsafe extern "C" fn pbuf_free(mut b: *mut pbuf) {
    free((*b).data as *mut ::core::ffi::c_void);
    (*b).data = ::core::ptr::null_mut::<::core::ffi::c_uchar>();
    (*b).cap = 0 as size_t;
    (*b).len = (*b).cap;
}
#[inline]
unsafe extern "C" fn pbuf_clear(mut b: *mut pbuf) {
    (*b).len = 0 as size_t;
}
pub const _CACHED_RUNES: ::core::ffi::c_int = (1 as ::core::ffi::c_int) << 8 as ::core::ffi::c_int;
pub const _CTYPE_A: ::core::ffi::c_long = 0x100 as ::core::ffi::c_long;
pub const _CTYPE_D: ::core::ffi::c_long = 0x400 as ::core::ffi::c_long;
pub const _CTYPE_S: ::core::ffi::c_long = 0x4000 as ::core::ffi::c_long;
#[inline]
unsafe extern "C" fn isascii(mut _c: ::core::ffi::c_int) -> ::core::ffi::c_int {
    return (_c & !(0x7f as ::core::ffi::c_int) == 0 as ::core::ffi::c_int) as ::core::ffi::c_int;
}
#[inline]
unsafe extern "C" fn __istype(
    mut _c: __darwin_ct_rune_t,
    mut _f: ::core::ffi::c_ulong,
) -> ::core::ffi::c_int {
    return if isascii(_c as ::core::ffi::c_int) != 0 {
        (_DefaultRuneLocale.__runetype[_c as usize] as ::core::ffi::c_ulong & _f != 0)
            as ::core::ffi::c_int
    } else {
        (__maskrune(_c, _f) != 0) as ::core::ffi::c_int
    };
}
#[inline]
unsafe extern "C" fn __isctype(
    mut _c: __darwin_ct_rune_t,
    mut _f: ::core::ffi::c_ulong,
) -> __darwin_ct_rune_t {
    return if _c < 0 as ::core::ffi::c_int || _c >= _CACHED_RUNES {
        0 as __darwin_ct_rune_t
    } else {
        (_DefaultRuneLocale.__runetype[_c as usize] as ::core::ffi::c_ulong & _f != 0)
            as ::core::ffi::c_int
    };
}
#[inline]
pub unsafe extern "C" fn isalnum(mut _c: ::core::ffi::c_int) -> ::core::ffi::c_int {
    return __istype(
        _c as __darwin_ct_rune_t,
        (_CTYPE_A | _CTYPE_D) as ::core::ffi::c_ulong,
    );
}
#[inline]
pub unsafe extern "C" fn isalpha(mut _c: ::core::ffi::c_int) -> ::core::ffi::c_int {
    return __istype(_c as __darwin_ct_rune_t, _CTYPE_A as ::core::ffi::c_ulong);
}
#[inline]
pub unsafe extern "C" fn isdigit(mut _c: ::core::ffi::c_int) -> ::core::ffi::c_int {
    return __isctype(_c as __darwin_ct_rune_t, _CTYPE_D as ::core::ffi::c_ulong)
        as ::core::ffi::c_int;
}
#[inline]
pub unsafe extern "C" fn isspace(mut _c: ::core::ffi::c_int) -> ::core::ffi::c_int {
    return __istype(_c as __darwin_ct_rune_t, _CTYPE_S as ::core::ffi::c_ulong);
}
pub const M_PI: ::core::ffi::c_double = 3.14159265358979323846264338327950288f64;
pub const MAX_VF_DEPTH: ::core::ffi::c_int = 8 as ::core::ffi::c_int;
pub const MAX_STACK: ::core::ffi::c_int = 1024 as ::core::ffi::c_int;
pub const MAX_COLORSTACKS: ::core::ffi::c_int = 16 as ::core::ffi::c_int;
unsafe extern "C" fn be(mut p: *const ::core::ffi::c_uchar, mut n: ::core::ffi::c_int) -> uint32_t {
    let mut v: uint32_t = 0 as uint32_t;
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < n {
        v = v << 8 as ::core::ffi::c_int | *p.offset(i as isize) as uint32_t;
        i += 1;
    }
    return v;
}
unsafe extern "C" fn sbe(mut p: *const ::core::ffi::c_uchar, mut n: ::core::ffi::c_int) -> int32_t {
    let mut v: uint32_t = be(p, n);
    if n < 4 as ::core::ffi::c_int
        && v & (1 as uint32_t) << 8 as ::core::ffi::c_int * n - 1 as ::core::ffi::c_int != 0
    {
        v = (v as ::core::ffi::c_uint | !(0 as ::core::ffi::c_uint) << 8 as ::core::ffi::c_int * n)
            as uint32_t;
    }
    return v as int32_t;
}
unsafe extern "C" fn xstrndup(
    mut s: *const ::core::ffi::c_char,
    mut n: size_t,
) -> *mut ::core::ffi::c_char {
    let mut r: *mut ::core::ffi::c_char =
        malloc(n.wrapping_add(1 as size_t)) as *mut ::core::ffi::c_char;
    if r.is_null() {
        abort();
    }
    memcpy(
        r as *mut ::core::ffi::c_void,
        s as *const ::core::ffi::c_void,
        n,
    );
    *r.offset(n as isize) = 0 as ::core::ffi::c_char;
    return r;
}
unsafe extern "C" fn table_find(mut t: *mut font_table, mut k: int32_t) -> *mut font_slot {
    let mut i: ::core::ffi::c_int = (*t).n - 1 as ::core::ffi::c_int;
    while i >= 0 as ::core::ffi::c_int {
        if (**(*t).slots.offset(i as isize)).k == k {
            return *(*t).slots.offset(i as isize);
        }
        i -= 1;
    }
    return ::core::ptr::null_mut::<font_slot>();
}
unsafe extern "C" fn table_add(mut t: *mut font_table, mut s: *mut font_slot) {
    if (*t).n == (*t).cap {
        (*t).cap = if (*t).cap != 0 {
            (*t).cap * 2 as ::core::ffi::c_int
        } else {
            16 as ::core::ffi::c_int
        };
        (*t).slots = realloc(
            (*t).slots as *mut ::core::ffi::c_void,
            (::core::mem::size_of::<*mut font_slot>() as size_t).wrapping_mul((*t).cap as size_t),
        ) as *mut *mut font_slot;
        if (*t).slots.is_null() {
            abort();
        }
    }
    let fresh8 = (*t).n;
    (*t).n = (*t).n + 1;
    let ref mut fresh9 = *(*t).slots.offset(fresh8 as isize);
    *fresh9 = s;
}
unsafe extern "C" fn table_free(mut t: *mut font_table) {
    if t.is_null() {
        return;
    }
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < (*t).n {
        slot_free(*(*t).slots.offset(i as isize));
        i += 1;
    }
    free((*t).slots as *mut ::core::ffi::c_void);
    free(t as *mut ::core::ffi::c_void);
}
unsafe extern "C" fn slot_free(mut s: *mut font_slot) {
    free((*s).name as *mut ::core::ffi::c_void);
    table_free((*s).vf_local);
    free(s as *mut ::core::ffi::c_void);
}
unsafe extern "C" fn kv_set(
    mut l: *mut kvlist,
    mut key: *const ::core::ffi::c_char,
    mut klen: size_t,
    mut val: *const ::core::ffi::c_char,
    mut vlen: size_t,
) {
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < (*l).n {
        if strlen((*(*l).items.offset(i as isize)).key) == klen
            && memcmp(
                (*(*l).items.offset(i as isize)).key as *const ::core::ffi::c_void,
                key as *const ::core::ffi::c_void,
                klen,
            ) == 0 as ::core::ffi::c_int
        {
            free((*(*l).items.offset(i as isize)).value as *mut ::core::ffi::c_void);
            let ref mut fresh3 = (*(*l).items.offset(i as isize)).value;
            *fresh3 = xstrndup(val, vlen);
            return;
        }
        i += 1;
    }
    if (*l).n == (*l).cap {
        (*l).cap = if (*l).cap != 0 {
            (*l).cap * 2 as ::core::ffi::c_int
        } else {
            8 as ::core::ffi::c_int
        };
        (*l).items = realloc(
            (*l).items as *mut ::core::ffi::c_void,
            (::core::mem::size_of::<kv>() as size_t).wrapping_mul((*l).cap as size_t),
        ) as *mut kv;
        if (*l).items.is_null() {
            abort();
        }
    }
    let ref mut fresh4 = (*(*l).items.offset((*l).n as isize)).key;
    *fresh4 = xstrndup(key, klen);
    let ref mut fresh5 = (*(*l).items.offset((*l).n as isize)).value;
    *fresh5 = xstrndup(val, vlen);
    (*l).n += 1;
}
unsafe extern "C" fn kv_free(mut l: *mut kvlist) {
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < (*l).n {
        free((*(*l).items.offset(i as isize)).key as *mut ::core::ffi::c_void);
        free((*(*l).items.offset(i as isize)).value as *mut ::core::ffi::c_void);
        i += 1;
    }
    free((*l).items as *mut ::core::ffi::c_void);
    memset(
        l as *mut ::core::ffi::c_void,
        0 as ::core::ffi::c_int,
        ::core::mem::size_of::<kvlist>() as size_t,
    );
}
static mut cat_names: [*const ::core::ffi::c_char; 7] = [
    b"ExtGState\0" as *const u8 as *const ::core::ffi::c_char,
    b"ColorSpace\0" as *const u8 as *const ::core::ffi::c_char,
    b"Pattern\0" as *const u8 as *const ::core::ffi::c_char,
    b"Shading\0" as *const u8 as *const ::core::ffi::c_char,
    b"XObject\0" as *const u8 as *const ::core::ffi::c_char,
    b"Font\0" as *const u8 as *const ::core::ffi::c_char,
    b"Properties\0" as *const u8 as *const ::core::ffi::c_char,
];
unsafe extern "C" fn resources_free(mut r: *mut resources) {
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < NCAT as ::core::ffi::c_int {
        kv_free((&raw mut (*r).cat as *mut kvlist).offset(i as isize) as *mut kvlist);
        free((*r).cat_ref[i as usize] as *mut ::core::ffi::c_void);
        i += 1;
    }
    memset(
        r as *mut ::core::ffi::c_void,
        0 as ::core::ffi::c_int,
        ::core::mem::size_of::<resources>() as size_t,
    );
}
unsafe extern "C" fn cur(mut c: *mut conv_ctx) -> *mut target {
    return (&raw mut (*c).targets as *mut target)
        .offset(((*c).ntargets - 1 as ::core::ffi::c_int) as isize) as *mut target;
}
unsafe extern "C" fn warn_once(
    mut c: *mut conv_ctx,
    mut key: *const ::core::ffi::c_char,
    mut fmt: *const ::core::ffi::c_char,
    mut arg: *const ::core::ffi::c_char,
) {
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < (*c).nwarned {
        if strncmp(
            &raw mut *(&raw mut (*c).warned as *mut [::core::ffi::c_char; 48]).offset(i as isize)
                as *mut ::core::ffi::c_char,
            key,
            (::core::mem::size_of::<[::core::ffi::c_char; 48]>() as size_t)
                .wrapping_sub(1 as size_t),
        ) == 0 as ::core::ffi::c_int
        {
            return;
        }
        i += 1;
    }
    if (*c).nwarned < 64 as ::core::ffi::c_int {
        let fresh2 = (*c).nwarned;
        (*c).nwarned = (*c).nwarned + 1;
        snprintf(
            &raw mut *(&raw mut (*c).warned as *mut [::core::ffi::c_char; 48])
                .offset(fresh2 as isize) as *mut ::core::ffi::c_char,
            ::core::mem::size_of::<[::core::ffi::c_char; 48]>() as size_t,
            b"%s\0" as *const u8 as *const ::core::ffi::c_char,
            key,
        );
    }
    pbuf_printf((*c).warnings, fmt, arg);
    pbuf_putc((*c).warnings, '\n' as i32);
}
unsafe extern "C" fn emit_num(mut b: *mut pbuf, mut v: ::core::ffi::c_double) {
    pbuf_real(b, v);
    pbuf_putc(b, ' ' as i32);
}
unsafe extern "C" fn text_end(mut c: *mut conv_ctx) {
    let mut o: *mut pbuf =
        &raw mut (*(cur as unsafe extern "C" fn(*mut conv_ctx) -> *mut target)(c)).content;
    if (*c).in_tj {
        pbuf_puts(o, b"]TJ\n\0" as *const u8 as *const ::core::ffi::c_char);
    }
    if (*c).in_bt {
        pbuf_puts(o, b"ET\n\0" as *const u8 as *const ::core::ffi::c_char);
    }
    (*c).pos_valid = false_0 != 0;
    (*c).in_bt = (*c).pos_valid;
    (*c).in_tj = (*c).in_bt;
    (*c).t_font = -(1 as ::core::ffi::c_int);
}
unsafe extern "C" fn emit_raw(
    mut c: *mut conv_ctx,
    mut s: *const ::core::ffi::c_char,
    mut n: size_t,
) {
    if !(*c).render {
        return;
    }
    text_end(c);
    let mut o: *mut pbuf =
        &raw mut (*(cur as unsafe extern "C" fn(*mut conv_ctx) -> *mut target)(c)).content;
    pbuf_append(o, s as *const ::core::ffi::c_void, n);
    pdf_document::Document::track_raw(c,s,n);
    pbuf_putc(o, '\n' as i32);
}
unsafe extern "C" fn emit_q(mut c: *mut conv_ctx) {
    if !(*c).render {
        return;
    }
    text_end(c);
    pbuf_puts(
        &raw mut (*(cur as unsafe extern "C" fn(*mut conv_ctx) -> *mut target)(c)).content,
        b"q\n\0" as *const u8 as *const ::core::ffi::c_char,
    );
    (*c).q_depth += 1;
    pdf_document::Document::save(c);
}
unsafe extern "C" fn emit_Q(mut c: *mut conv_ctx) {
    if !(*c).render {
        return;
    }
    text_end(c);
    if (*c).q_depth > (*cur(c)).q_base {
        pbuf_puts(
            &raw mut (*(cur as unsafe extern "C" fn(*mut conv_ctx) -> *mut target)(c)).content,
            b"Q\n\0" as *const u8 as *const ::core::ffi::c_char,
        );
        (*c).q_depth -= 1;
        pdf_document::Document::restore(c);
    }
}
unsafe extern "C" fn emit_cm(mut c: *mut conv_ctx, mut m: mat) {
    if !(*c).render {
        return;
    }
    text_end(c);
    let mut o: *mut pbuf =
        &raw mut (*(cur as unsafe extern "C" fn(*mut conv_ctx) -> *mut target)(c)).content;
    pdf_document::Document::concat(c,[m.a,m.b,m.c,m.d,m.e,m.f]);
    emit_num(o, m.a);
    emit_num(o, m.b);
    emit_num(o, m.c);
    emit_num(o, m.d);
    backend_options::emit_coordinate(c,o, m.e);
    backend_options::emit_coordinate(c,o, m.f);
    pbuf_puts(o, b"cm\n\0" as *const u8 as *const ::core::ffi::c_char);
}
unsafe extern "C" fn mat_mul(mut x: mat, mut y: mat) -> mat {
    return mat {
        a: x.a * y.a + x.b * y.c,
        b: x.a * y.b + x.b * y.d,
        c: x.c * y.a + x.d * y.c,
        d: x.c * y.b + x.d * y.d,
        e: x.e * y.a + x.f * y.c + y.e,
        f: x.e * y.b + x.f * y.d + y.f,
    };
}
unsafe extern "C" fn mat_translate(
    mut x: ::core::ffi::c_double,
    mut y: ::core::ffi::c_double,
) -> mat {
    return mat {
        a: 1 as ::core::ffi::c_int as ::core::ffi::c_double,
        b: 0 as ::core::ffi::c_int as ::core::ffi::c_double,
        c: 0 as ::core::ffi::c_int as ::core::ffi::c_double,
        d: 1 as ::core::ffi::c_int as ::core::ffi::c_double,
        e: x,
        f: y,
    };
}
unsafe extern "C" fn mat_about(
    mut m: mat,
    mut x: ::core::ffi::c_double,
    mut y: ::core::ffi::c_double,
) -> mat {
    return mat_mul(mat_mul(mat_translate(-x, -y), m), mat_translate(x, y));
}
unsafe extern "C" fn X(
    mut c: *mut conv_ctx,
    mut h: ::core::ffi::c_double,
) -> ::core::ffi::c_double {
    return (*(*c).document).origin.unwrap_or([72.,72.])[0] + h * (*c).conv - (*c).ox;
}
unsafe extern "C" fn Y(
    mut c: *mut conv_ctx,
    mut v: ::core::ffi::c_double,
) -> ::core::ffi::c_double {
    return (*c).page_h - (*(*c).document).origin.unwrap_or([72.,72.])[1] - v * (*c).conv - (*c).oy;
}
unsafe extern "C" fn emit_rule(
    mut c: *mut conv_ctx,
    mut h: ::core::ffi::c_double,
    mut v: ::core::ffi::c_double,
    mut width: ::core::ffi::c_double,
    mut height: ::core::ffi::c_double,
) {
    if !(*c).render
        || width <= 0 as ::core::ffi::c_int as ::core::ffi::c_double
        || height <= 0 as ::core::ffi::c_int as ::core::ffi::c_double
    {
        return;
    }
    pdf_document::Document::touch(c,X(c,h),Y(c,v),width*(*c).conv,height*(*c).conv,0.);
    text_end(c);
    let mut o: *mut pbuf =
        &raw mut (*(cur as unsafe extern "C" fn(*mut conv_ctx) -> *mut target)(c)).content;
    backend_options::emit_coordinate(c,o, X(c, h));
    backend_options::emit_coordinate(c,o, Y(c, v));
    backend_options::emit_coordinate(c,o, width * (*c).conv);
    backend_options::emit_coordinate(c,o, height * (*c).conv);
    pbuf_puts(o, b"re f\n\0" as *const u8 as *const ::core::ffi::c_char);
}
unsafe extern "C" fn emit_glyph(
    mut c: *mut conv_ctx,
    mut pf: ::core::ffi::c_int,
    mut size: ::core::ffi::c_double,
    mut a: ::core::ffi::c_double,
    mut sl: ::core::ffi::c_double,
    mut x: ::core::ffi::c_double,
    mut y: ::core::ffi::c_double,
    mut code: *const ::core::ffi::c_uchar,
    mut nbytes: ::core::ffi::c_int,
    mut width_units: ::core::ffi::c_double,
) {
    font_output::before_glyph(c,x,y,size,width_units/1000.*size*a);
    pdf_document::Document::touch(c,x,y,width_units / 1000. * size * a,size*0.8,size*0.2);
    let mut o: *mut pbuf =
        &raw mut (*(cur as unsafe extern "C" fn(*mut conv_ctx) -> *mut target)(c)).content;
    if !(*c).in_bt {
        pbuf_puts(o, b"BT\n\0" as *const u8 as *const ::core::ffi::c_char);
        (*c).in_bt = true_0 != 0;
        (*c).t_font = -(1 as ::core::ffi::c_int);
        (*c).pos_valid = false_0 != 0;
    }
    if (*c).t_font != pf || fabs((*c).t_size - size) > 1e-6f64 {
        if (*c).in_tj {
            pbuf_puts(o, b"]TJ\n\0" as *const u8 as *const ::core::ffi::c_char);
            (*c).in_tj = false_0 != 0;
        }
        pbuf_printf(
            o,
            b"/%s \0" as *const u8 as *const ::core::ffi::c_char,
            std::ffi::CString::new(font_output::resource(c,pf)).unwrap().as_ptr(),
        );
        backend_options::emit_coordinate(c,o, size);
        pbuf_puts(o, b"Tf\n\0" as *const u8 as *const ::core::ffi::c_char);
        (*c).t_font = pf;
        (*c).t_size = size;
        (*c).pos_valid = false_0 != 0;
    }
    if fabs((*c).t_a - a) > 1e-9f64 || fabs((*c).t_c - sl) > 1e-9f64 {
        (*c).pos_valid = false_0 != 0;
    }
    if (*c).pos_valid as ::core::ffi::c_int != 0
        && (*c).in_tj as ::core::ffi::c_int != 0
        && fabs(y - (*c).t_y) < 1e-4f64
    {
        let mut adj: ::core::ffi::c_double = -(x - (*c).t_x) * 1000.0f64 / (size * a);
        if fabs(adj) > 0.01f64 {
            pbuf_real(o, adj);
        }
    } else {
        if (*c).in_tj {
            pbuf_puts(o, b"]TJ\n\0" as *const u8 as *const ::core::ffi::c_char);
        }
        emit_num(o, a);
        pbuf_puts(o, b"0 \0" as *const u8 as *const ::core::ffi::c_char);
        emit_num(o, sl);
        pbuf_puts(o, b"1 \0" as *const u8 as *const ::core::ffi::c_char);
        backend_options::emit_coordinate(c,o, x);
        backend_options::emit_coordinate(c,o, y);
        pbuf_puts(o, b"Tm[\0" as *const u8 as *const ::core::ffi::c_char);
        (*c).in_tj = true_0 != 0;
        (*c).t_a = a;
        (*c).t_c = sl;
    }
    static mut hex: [::core::ffi::c_char; 17] = unsafe {
        ::core::mem::transmute::<[u8; 17], [::core::ffi::c_char; 17]>(*b"0123456789ABCDEF\0")
    };
    pbuf_putc(o, '<' as i32);
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < nbytes {
        pbuf_putc(
            o,
            hex[(*code.offset(i as isize) as ::core::ffi::c_int >> 4 as ::core::ffi::c_int)
                as usize] as ::core::ffi::c_int,
        );
        pbuf_putc(
            o,
            hex[(*code.offset(i as isize) as ::core::ffi::c_int & 15 as ::core::ffi::c_int)
                as usize] as ::core::ffi::c_int,
        );
        i += 1;
    }
    pbuf_putc(o, '>' as i32);
    (*c).t_x = x + width_units / 1000.0f64 * size * a;
    (*c).t_y = y;
    (*c).pos_valid = true_0 != 0;
}
unsafe extern "C" fn pdf_font_for(
    mut c: *mut conv_ctx,
    mut s: *mut font_slot,
) -> ::core::ffi::c_int {
    if (*s).publish_epoch == (*(*c).w).epoch && (*s).pdf_font >= 0 as ::core::ffi::c_int {
        return (*s).pdf_font;
    }
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < (*c).npfonts {
        let mut p: *mut pdf_font = (*c).pfonts.offset(i as isize) as *mut pdf_font;
        if !(*s).is_virtual && (*p).font_id == (*s).k && if (*s).kind as ::core::ffi::c_uint
            == FS_NATIVE as ::core::ffi::c_int as ::core::ffi::c_uint
        {
            ((*p).native as ::core::ffi::c_int != 0 && (*p).nf == (*s).nf && (*p).vertical == (*s).vertical) as ::core::ffi::c_int
        } else {
            (!(*p).native && (*p).bitmap == ((*s).kind == FS_BITMAP) && (*p).t1 == (*s).t1 && (*p).enc == (*s).enc && (*p).tfm == (*s).tfm)
                as ::core::ffi::c_int
        } != 0
        {
            (*s).pdf_font = i;
            (*s).publish_epoch = (*(*c).w).epoch;
            return i;
        }
        i += 1;
    }
    if (*c).npfonts == (*c).cappfonts {
        (*c).cappfonts = if (*c).cappfonts != 0 {
            (*c).cappfonts * 2 as ::core::ffi::c_int
        } else {
            16 as ::core::ffi::c_int
        };
        (*c).pfonts = realloc(
            (*c).pfonts as *mut ::core::ffi::c_void,
            (::core::mem::size_of::<pdf_font>() as size_t).wrapping_mul((*c).cappfonts as size_t),
        ) as *mut pdf_font;
        if (*c).pfonts.is_null() {
            abort();
        }
    }
    let mut p_0: *mut pdf_font = (*c).pfonts.offset((*c).npfonts as isize) as *mut pdf_font;
    memset(
        p_0 as *mut ::core::ffi::c_void,
        0 as ::core::ffi::c_int,
        ::core::mem::size_of::<pdf_font>() as size_t,
    );
    (*p_0).bitmap = (*s).kind == FS_BITMAP;
    if (*p_0).bitmap { bitmap_fonts::attach(c,s,(*c).npfonts); }
    (*p_0).font_id = if (*s).is_virtual { -(*c).npfonts - 1 } else { (*s).k };
    (*p_0).native =
        (*s).kind as ::core::ffi::c_uint == FS_NATIVE as ::core::ffi::c_int as ::core::ffi::c_uint;
    (*p_0).nf = (*s).nf;
    (*p_0).vertical = (*s).vertical;
    (*p_0).t1 = (*s).t1;
    (*p_0).enc = (*s).enc;
    (*p_0).tfm = (*s).tfm;
    (*p_0).obj = if (*s).is_virtual { pdfw_alloc((*c).pw) } else { font_output::object(c,(*s).k) };
    (*p_0).nused = if (*p_0).native as ::core::ffi::c_int != 0 {
        (*(*s).nf).num_glyphs
    } else {
        256 as ::core::ffi::c_int
    };
    (*p_0).used = calloc(
        ((*p_0).nused as size_t).wrapping_add(1 as size_t),
        1 as size_t,
    ) as *mut ::core::ffi::c_uchar;
    if (*p_0).used.is_null() {
        abort();
    }
    font_output::mark_included(c,s,p_0);
    (*s).pdf_font = (*c).npfonts;
    (*s).publish_epoch = (*(*c).w).epoch;
    let fresh7 = (*c).npfonts;
    (*c).npfonts = (*c).npfonts + 1;
    return fresh7;
}
unsafe extern "C" fn new_slot(mut k: int32_t) -> *mut font_slot {
    let mut s: *mut font_slot =
        calloc(1 as size_t, ::core::mem::size_of::<font_slot>() as size_t) as *mut font_slot;
    if s.is_null() {
        abort();
    }
    (*s).k = k;
    (*s).extend = 1 as ::core::ffi::c_int as ::core::ffi::c_double;
    (*s).pdf_font = -(1 as ::core::ffi::c_int);
    return s;
}
unsafe extern "C" fn setup_tfm_slot(mut c: *mut conv_ctx, mut s: *mut font_slot) {
    static mut vf_depth: ::core::ffi::c_int = 0;
    let mut vf: *mut vf_font = if vf_depth < MAX_VF_DEPTH {
        font_vf((*(*c).w).fonts, (*s).name)
    } else {
        ::core::ptr::null_mut::<vf_font>()
    };
    (*s).tfm = font_tfm((*(*c).w).fonts, (*s).name);
    if !vf.is_null() && (*vf).ok as ::core::ffi::c_int != 0 {
        (*s).kind = FS_VF;
        (*s).vf = vf;
        (*s).vf_local =
            calloc(1 as size_t, ::core::mem::size_of::<font_table>() as size_t) as *mut font_table;
        if (*s).vf_local.is_null() {
            abort();
        }
        vf_depth += 1;
        let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
        while i < (*vf).nfonts {
            let mut l: *mut font_slot = new_slot((*(*vf).fonts.offset(i as isize)).k);
            (*l).is_virtual = true;
            (*l).name = strdup((*(*vf).fonts.offset(i as isize)).name);
            (*l).size = (*(*vf).fonts.offset(i as isize)).scaled_fix as ::core::ffi::c_double
                * (*s).size
                / 1048576.0f64;
            setup_tfm_slot(c, l);
            table_add((*s).vf_local, l);
            i += 1;
        }
        vf_depth -= 1;
        return;
    }
    let mut m: *const map_entry = font_map_lookup((*(*c).w).fonts, (*s).name);
    if m.is_null() || (*m).fontfile.is_null() {
        if bitmap_fonts::setup(c,s) { return; }
        (*s).kind = FS_MISSING;
        warn_once(
            c,
            (*s).name,
            b"no Type 1 or prebuilt PK font found for TeX font %s (glyphs omitted)\0" as *const u8
                as *const ::core::ffi::c_char,
            (*s).name,
        );
        return;
    }
    (*s).map = m;
    (*s).t1 = font_type1((*(*c).w).fonts, (*m).fontfile, (*c).warnings);
    if !(*(*s).t1).ok || !(*(*s).tfm).ok {
        if !(*m).encfile.is_null() {
            let enc=font_enc((*(*c).w).fonts,(*m).encfile);
            if !enc.is_null() && (*enc).ok { (*s).enc=enc; }
        }
        if bitmap_fonts::setup(c,s) { return; }
        (*s).kind = FS_MISSING;
        if !(*(*s).tfm).ok {
            warn_once(
                c,
                (*s).name,
                b"TFM not found: %s\0" as *const u8 as *const ::core::ffi::c_char,
                (*s).name,
            );
        }
        return;
    }
    (*s).enc = if !(*m).encfile.is_null() {
        font_enc((*(*c).w).fonts, (*m).encfile)
    } else {
        ::core::ptr::null_mut::<enc_vector>()
    };
    if !(*s).enc.is_null() && !(*(*s).enc).ok {
        (*s).enc = ::core::ptr::null_mut::<enc_vector>();
    }
    (*s).extend = if (*m).extend > 0 as ::core::ffi::c_int as ::core::ffi::c_double {
        (*m).extend
    } else {
        1 as ::core::ffi::c_int as ::core::ffi::c_double
    };
    (*s).slant = (*m).slant;
    (*s).kind = FS_TYPE1;
}
unsafe extern "C" fn define_tfm_font(
    mut c: *mut conv_ctx,
    mut t: *mut font_table,
    mut p: *const ::core::ffi::c_uchar,
    mut k_bytes: ::core::ffi::c_int,
) {
    let mut k: int32_t = sbe(p.offset(1 as ::core::ffi::c_int as isize), k_bytes);
    let mut q: *const ::core::ffi::c_uchar = p
        .offset(1 as ::core::ffi::c_int as isize)
        .offset(k_bytes as isize);
    let mut s: int32_t = be(
        q.offset(4 as ::core::ffi::c_int as isize),
        4 as ::core::ffi::c_int,
    ) as int32_t;
    let mut a: ::core::ffi::c_int =
        *q.offset(12 as ::core::ffi::c_int as isize) as ::core::ffi::c_int;
    let mut l: ::core::ffi::c_int =
        *q.offset(13 as ::core::ffi::c_int as isize) as ::core::ffi::c_int;
    if !table_find(t, k).is_null() {
        return;
    }
    let mut slot: *mut font_slot = new_slot(k);
    (*slot).name = xstrndup(
        (q as *const ::core::ffi::c_char)
            .offset(14 as ::core::ffi::c_int as isize)
            .offset(a as isize),
        l as size_t,
    );
    (*slot).size = s as ::core::ffi::c_double;
    setup_tfm_slot(c, slot);
    (*slot).letterspace=(*(*c).font_output).letterspace.get(&k).copied().unwrap_or(0.);
    table_add(t, slot);
}
unsafe extern "C" fn define_native_font(
    mut c: *mut conv_ctx,
    mut t: *mut font_table,
    mut p: *const ::core::ffi::c_uchar,
) {
    let mut k: int32_t = be(
        p.offset(1 as ::core::ffi::c_int as isize),
        4 as ::core::ffi::c_int,
    ) as int32_t;
    if !table_find(t, k).is_null() {
        return;
    }
    let mut size: int32_t = be(
        p.offset(5 as ::core::ffi::c_int as isize),
        4 as ::core::ffi::c_int,
    ) as int32_t;
    let mut flags: ::core::ffi::c_uint = be(
        p.offset(9 as ::core::ffi::c_int as isize),
        2 as ::core::ffi::c_int,
    ) as ::core::ffi::c_uint;
    let mut nl: ::core::ffi::c_int =
        *p.offset(11 as ::core::ffi::c_int as isize) as ::core::ffi::c_int;
    let mut q: *const ::core::ffi::c_uchar = p
        .offset(12 as ::core::ffi::c_int as isize)
        .offset(nl as isize);
    let mut index: uint32_t = be(q, 4 as ::core::ffi::c_int);
    q = q.offset(4 as ::core::ffi::c_int as isize);
    let mut s: *mut font_slot = new_slot(k);
    (*s).name = xstrndup(
        (p as *const ::core::ffi::c_char).offset(12 as ::core::ffi::c_int as isize),
        nl as size_t,
    );
    (*s).size = size as ::core::ffi::c_double;
    if flags & 0x200 as ::core::ffi::c_uint != 0 {
        let mut rgba: uint32_t = be(q, 4 as ::core::ffi::c_int);
        q = q.offset(4 as ::core::ffi::c_int as isize);
        (*s).colored = true_0 != 0;
        (*s).rgba[0 as ::core::ffi::c_int as usize] =
            (rgba >> 24 as ::core::ffi::c_int) as ::core::ffi::c_double / 255.0f64;
        (*s).rgba[1 as ::core::ffi::c_int as usize] =
            (rgba >> 16 as ::core::ffi::c_int & 255 as uint32_t) as ::core::ffi::c_double
                / 255.0f64;
        (*s).rgba[2 as ::core::ffi::c_int as usize] =
            (rgba >> 8 as ::core::ffi::c_int & 255 as uint32_t) as ::core::ffi::c_double / 255.0f64;
        (*s).rgba[3 as ::core::ffi::c_int as usize] =
            (rgba & 255 as uint32_t) as ::core::ffi::c_double / 255.0f64;
    }
    if flags & 0x1000 as ::core::ffi::c_uint != 0 {
        (*s).extend =
            be(q, 4 as ::core::ffi::c_int) as int32_t as ::core::ffi::c_double / 65536.0f64;
        q = q.offset(4 as ::core::ffi::c_int as isize);
    }
    if flags & 0x2000 as ::core::ffi::c_uint != 0 {
        (*s).slant =
            be(q, 4 as ::core::ffi::c_int) as int32_t as ::core::ffi::c_double / 65536.0f64;
        q = q.offset(4 as ::core::ffi::c_int as isize);
    }
    if flags & 0x4000 as ::core::ffi::c_uint != 0 {
        (*s).embolden =
            be(q, 4 as ::core::ffi::c_int) as int32_t as ::core::ffi::c_double / 65536.0f64;
        q = q.offset(4 as ::core::ffi::c_int as isize);
    }
    (*s).vertical = flags & 0x100 != 0;
    (*s).nf = font_native(
        (*(*c).w).fonts,
        (*s).name,
        index as ::core::ffi::c_int,
        (*c).warnings,
    );
    (*s).kind = (if (*(*s).nf).ok as ::core::ffi::c_int != 0 {
        FS_NATIVE as ::core::ffi::c_int
    } else {
        FS_MISSING as ::core::ffi::c_int
    }) as fs_kind;
    table_add(t, s);
}
unsafe extern "C" fn draw_char(
    mut c: *mut conv_ctx,
    mut s: *mut font_slot,
    mut code: ::core::ffi::c_uint,
    mut h: ::core::ffi::c_double,
    mut v: ::core::ffi::c_double,
    mut depth: ::core::ffi::c_int,
) {
    if !(*c).render || code > 255 as ::core::ffi::c_uint {
        return;
    }
    match (*s).kind as ::core::ffi::c_uint {
        2 | 4 => {
            if (*s).kind == FS_BITMAP && !bitmap_fonts::exists(c,s,code) { return; }
            if !(*(*s).tfm).exists[code as usize] {
                return;
            }
            let mut pf: ::core::ffi::c_int = pdf_font_for(c, s);
            *(*(*c).pfonts.offset(pf as isize))
                .used
                .offset(code as isize) = 1 as ::core::ffi::c_uchar;
            let mut size: ::core::ffi::c_double = (*s).size * (*c).conv;
            let mut wunits: ::core::ffi::c_double =
                (*(*s).tfm).width[code as usize] as ::core::ffi::c_double / 1048.576f64;
            let mut b: ::core::ffi::c_uchar = code as ::core::ffi::c_uchar;
            emit_glyph(
                c,
                pf,
                size,
                (*s).extend * (*c).expansion,
                (*s).slant,
                X(c, h + (*s).letterspace/2.),
                Y(c, v),
                &raw mut b,
                1 as ::core::ffi::c_int,
                wunits,
            );
        }
        3 => {
            if depth >= MAX_VF_DEPTH || !(*(*s).vf).chars[code as usize].exists {
                return;
            }
            run_packet(
                c,
                s,
                (*(*s).vf).chars[code as usize].packet,
                (*(*s).vf).chars[code as usize].len as size_t,
                h,
                v,
                depth + 1 as ::core::ffi::c_int,
            );
        }
        _ => {}
    };
}
unsafe extern "C" fn char_width(
    mut s: *mut font_slot,
    mut code: ::core::ffi::c_uint,
) -> ::core::ffi::c_double {
    if code > 255 as ::core::ffi::c_uint {
        return 0 as ::core::ffi::c_int as ::core::ffi::c_double;
    }
    if !(*s).tfm.is_null()
        && (*(*s).tfm).ok as ::core::ffi::c_int != 0
        && (*(*s).tfm).exists[code as usize] as ::core::ffi::c_int != 0
    {
        return (*(*s).tfm).width[code as usize] as ::core::ffi::c_double * (*s).size
            / 1048576.0f64 + (*s).letterspace;
    }
    if (*s).kind as ::core::ffi::c_uint == FS_VF as ::core::ffi::c_int as ::core::ffi::c_uint
        && (*(*s).vf).chars[code as usize].exists as ::core::ffi::c_int != 0
    {
        return (*(*s).vf).chars[code as usize].width_fix as ::core::ffi::c_double * (*s).size
            / 1048576.0f64;
    }
    return 0 as ::core::ffi::c_int as ::core::ffi::c_double;
}
unsafe extern "C" fn begin_native_style(mut c: *mut conv_ctx, mut s: *mut font_slot) {
    if !(*s).colored && (*s).embolden == 0 as ::core::ffi::c_int as ::core::ffi::c_double {
        return;
    }
    text_end(c);
    let mut o: *mut pbuf =
        &raw mut (*(cur as unsafe extern "C" fn(*mut conv_ctx) -> *mut target)(c)).content;
    pbuf_puts(o, b"q \0" as *const u8 as *const ::core::ffi::c_char);
    if (*s).colored {
        emit_num(o, (*s).rgba[0 as ::core::ffi::c_int as usize]);
        emit_num(o, (*s).rgba[1 as ::core::ffi::c_int as usize]);
        emit_num(o, (*s).rgba[2 as ::core::ffi::c_int as usize]);
        pbuf_puts(o, b"rg \0" as *const u8 as *const ::core::ffi::c_char);
        emit_num(o, (*s).rgba[0 as ::core::ffi::c_int as usize]);
        emit_num(o, (*s).rgba[1 as ::core::ffi::c_int as usize]);
        emit_num(o, (*s).rgba[2 as ::core::ffi::c_int as usize]);
        pbuf_puts(o, b"RG \0" as *const u8 as *const ::core::ffi::c_char);
    }
    if (*s).embolden != 0 as ::core::ffi::c_int as ::core::ffi::c_double {
        emit_num(o, fabs((*s).embolden));
        pbuf_puts(o, b"w 2 Tr\0" as *const u8 as *const ::core::ffi::c_char);
    }
    pbuf_putc(o, '\n' as i32);
}
unsafe extern "C" fn end_native_style(mut c: *mut conv_ctx, mut s: *mut font_slot) {
    if !(*s).colored && (*s).embolden == 0 as ::core::ffi::c_int as ::core::ffi::c_double {
        return;
    }
    text_end(c);
    pbuf_puts(
        &raw mut (*(cur as unsafe extern "C" fn(*mut conv_ctx) -> *mut target)(c)).content,
        b"0 Tr Q\n\0" as *const u8 as *const ::core::ffi::c_char,
    );
}
unsafe extern "C" fn draw_native_glyphs(
    mut c: *mut conv_ctx,
    mut s: *mut font_slot,
    mut h: ::core::ffi::c_double,
    mut v: ::core::ffi::c_double,
    mut xy: *const ::core::ffi::c_uchar,
    mut ids: *const ::core::ffi::c_uchar,
    mut n: ::core::ffi::c_int,
) {
    if !(*c).render
        || (*s).kind as ::core::ffi::c_uint
            != FS_NATIVE as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        return;
    }
    let mut pf: ::core::ffi::c_int = pdf_font_for(c, s);
    let mut p: *mut pdf_font = (*c).pfonts.offset(pf as isize) as *mut pdf_font;
    let mut size: ::core::ffi::c_double = (*s).size * (*c).conv;
    begin_native_style(c, s);
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < n {
        let mut gx: int32_t = be(
            xy.offset((8 as ::core::ffi::c_int * i) as isize),
            4 as ::core::ffi::c_int,
        ) as int32_t;
        let mut gy: int32_t = be(
            xy.offset((8 as ::core::ffi::c_int * i) as isize)
                .offset(4 as ::core::ffi::c_int as isize),
            4 as ::core::ffi::c_int,
        ) as int32_t;
        let mut gid: ::core::ffi::c_uint = be(
            ids.offset((2 as ::core::ffi::c_int * i) as isize),
            2 as ::core::ffi::c_int,
        ) as ::core::ffi::c_uint;
        if (gid as ::core::ffi::c_int) < (*p).nused {
            *(*p).used.offset(gid as isize) = 1 as ::core::ffi::c_uchar;
        }
        let mut wunits: ::core::ffi::c_double =
            native_advance((*s).nf, gid as ::core::ffi::c_int) as ::core::ffi::c_double * 1000.0f64
                / (*(*s).nf).units_per_em as ::core::ffi::c_double;
        let mut code: [::core::ffi::c_uchar; 2] = [
            (gid >> 8 as ::core::ffi::c_int) as ::core::ffi::c_uchar,
            gid as ::core::ffi::c_uchar,
        ];
        if (*s).vertical {
            text_end(c);
            let o = &raw mut (*cur(c)).content;
            pbuf_printf(o, b"BT /%s \0".as_ptr().cast(), std::ffi::CString::new(font_output::resource(c,pf)).unwrap().as_ptr());
            emit_num(o,size); pbuf_puts(o,b"Tf 0 \0".as_ptr().cast());
            emit_num(o,(*s).extend * (*c).expansion);
            emit_num(o,-1.); emit_num(o,(*s).slant);
            backend_options::emit_coordinate(c,o,X(c,h+gx as f64)); backend_options::emit_coordinate(c,o,Y(c,v+gy as f64));
            pbuf_puts(o,b"Tm \0".as_ptr().cast());
            pdfw_hexstring(o,code.as_ptr(),2);
            pbuf_puts(o,b" Tj ET\n\0".as_ptr().cast());
            pdf_document::Document::touch(c,X(c,h+gx as f64),Y(c,v+gy as f64),size,size,0.);
        } else {
        emit_glyph(
            c,
            pf,
            size,
            (*s).extend * (*c).expansion,
            (*s).slant,
            X(c, h + gx as ::core::ffi::c_double),
            Y(c, v + gy as ::core::ffi::c_double),
            &raw mut code as *mut ::core::ffi::c_uchar,
            2 as ::core::ffi::c_int,
            wunits,
        );
        }
        i += 1;
    }
    end_native_style(c, s);
}
unsafe extern "C" fn named_get(
    mut c: *mut conv_ctx,
    mut name: *const ::core::ffi::c_char,
    mut len: size_t,
    mut create: bool,
) -> *mut named_obj {
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < (*c).nnamed {
        if strlen((*(*c).named.offset(i as isize)).name) == len
            && memcmp(
                (*(*c).named.offset(i as isize)).name as *const ::core::ffi::c_void,
                name as *const ::core::ffi::c_void,
                len,
            ) == 0 as ::core::ffi::c_int
        {
            return (*c).named.offset(i as isize) as *mut named_obj;
        }
        i += 1;
    }
    if !create {
        return ::core::ptr::null_mut::<named_obj>();
    }
    if (*c).nnamed == (*c).capnamed {
        (*c).capnamed = if (*c).capnamed != 0 {
            (*c).capnamed * 2 as ::core::ffi::c_int
        } else {
            16 as ::core::ffi::c_int
        };
        (*c).named = realloc(
            (*c).named as *mut ::core::ffi::c_void,
            (::core::mem::size_of::<named_obj>() as size_t).wrapping_mul((*c).capnamed as size_t),
        ) as *mut named_obj;
        if (*c).named.is_null() {
            abort();
        }
    }
    let fresh11 = (*c).nnamed;
    (*c).nnamed = (*c).nnamed + 1;
    let mut o: *mut named_obj = (*c).named.offset(fresh11 as isize) as *mut named_obj;
    memset(
        o as *mut ::core::ffi::c_void,
        0 as ::core::ffi::c_int,
        ::core::mem::size_of::<named_obj>() as size_t,
    );
    (*o).name = xstrndup(name, len);
    (*o).obj = pdfw_alloc((*c).pw);
    return o;
}
unsafe extern "C" fn skip_ws(
    mut p: *const ::core::ffi::c_char,
    mut end: *const ::core::ffi::c_char,
) -> *const ::core::ffi::c_char {
    while p < end && isspace(*p as ::core::ffi::c_uchar as ::core::ffi::c_int) != 0 {
        p = p.offset(1);
    }
    return p;
}
unsafe extern "C" fn skip_value(
    mut p: *const ::core::ffi::c_char,
    mut end: *const ::core::ffi::c_char,
) -> *const ::core::ffi::c_char {
    p = skip_ws(p, end);
    if p >= end {
        return p;
    }
    if *p as ::core::ffi::c_int == '(' as i32 {
        let mut depth: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
        while p < end {
            if *p as ::core::ffi::c_int == '\\' as i32 {
                p = p.offset(1);
            } else if *p as ::core::ffi::c_int == '(' as i32 {
                depth += 1;
            } else if *p as ::core::ffi::c_int == ')' as i32 && {
                depth -= 1;
                depth == 0 as ::core::ffi::c_int
            } {
                return p.offset(1 as ::core::ffi::c_int as isize);
            }
            p = p.offset(1);
        }
        return end;
    }
    if *p as ::core::ffi::c_int == '<' as i32
        && p.offset(1 as ::core::ffi::c_int as isize) < end
        && *p.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int == '<' as i32
    {
        p = p.offset(2 as ::core::ffi::c_int as isize);
        loop {
            p = skip_ws(p, end);
            if p >= end {
                return end;
            }
            if *p as ::core::ffi::c_int == '>' as i32
                && p.offset(1 as ::core::ffi::c_int as isize) < end
                && *p.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int == '>' as i32
            {
                return p.offset(2 as ::core::ffi::c_int as isize);
            }
            let mut q: *const ::core::ffi::c_char = skip_value(p, end);
            if q == p {
                return end;
            }
            p = q;
        }
    }
    if *p as ::core::ffi::c_int == '<' as i32 {
        let mut q_0: *const ::core::ffi::c_char = memchr(
            p as *const ::core::ffi::c_void,
            '>' as i32,
            end.offset_from(p) as ::core::ffi::c_long as size_t,
        ) as *const ::core::ffi::c_char;
        return if !q_0.is_null() {
            q_0.offset(1 as ::core::ffi::c_int as isize)
        } else {
            end
        };
    }
    if *p as ::core::ffi::c_int == '[' as i32 {
        p = p.offset(1);
        loop {
            p = skip_ws(p, end);
            if p >= end {
                return end;
            }
            if *p as ::core::ffi::c_int == ']' as i32 {
                return p.offset(1 as ::core::ffi::c_int as isize);
            }
            let mut q_1: *const ::core::ffi::c_char = skip_value(p, end);
            if q_1 == p {
                return end;
            }
            p = q_1;
        }
    }
    if *p as ::core::ffi::c_int == '/' as i32 {
        p = p.offset(1);
    }
    let mut s: *const ::core::ffi::c_char = p;
    while p < end
        && isspace(*p as ::core::ffi::c_uchar as ::core::ffi::c_int) == 0
        && strchr(
            b"()<>[]{}/%\0" as *const u8 as *const ::core::ffi::c_char,
            *p as ::core::ffi::c_int,
        )
        .is_null()
    {
        p = p.offset(1);
    }
    if p == s && p < end && *p as ::core::ffi::c_int != '/' as i32 {
        p = p.offset(1);
    }
    let mut t: *const ::core::ffi::c_char = skip_ws(p, end);
    if isdigit(*s as ::core::ffi::c_uchar as ::core::ffi::c_int) != 0 {
        let mut u: *const ::core::ffi::c_char = t;
        while u < end && isdigit(*u as ::core::ffi::c_uchar as ::core::ffi::c_int) != 0 {
            u = u.offset(1);
        }
        if u > t {
            let mut r: *const ::core::ffi::c_char = skip_ws(u, end);
            if r < end
                && *r as ::core::ffi::c_int == 'R' as i32
                && (r.offset(1 as ::core::ffi::c_int as isize) == end
                    || isalnum(
                        *r.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_uchar
                            as ::core::ffi::c_int,
                    ) == 0)
            {
                return r.offset(1 as ::core::ffi::c_int as isize);
            }
        }
    }
    return p;
}
unsafe extern "C" fn subst_names(
    mut c: *mut conv_ctx,
    mut s: *const ::core::ffi::c_char,
    mut n: size_t,
    mut cpx: ::core::ffi::c_double,
    mut cpy: ::core::ffi::c_double,
) -> *mut ::core::ffi::c_char {
    let mut b: pbuf = pbuf {
        data: ::core::ptr::null_mut::<::core::ffi::c_uchar>(),
        len: 0,
        cap: 0,
    };
    let mut end: *const ::core::ffi::c_char = s.offset(n as isize);
    let mut p: *const ::core::ffi::c_char = s;
    while p < end {
        if *p as ::core::ffi::c_int == '(' as i32 {
            let mut q: *const ::core::ffi::c_char = skip_value(p, end);
            pbuf_append(
                &raw mut b,
                p as *const ::core::ffi::c_void,
                q.offset_from(p) as ::core::ffi::c_long as size_t,
            );
            p = q;
        } else {
            let mut prev: ::core::ffi::c_char = (if p > s {
                *p.offset(-(1 as ::core::ffi::c_int) as isize) as ::core::ffi::c_int
            } else {
                ' ' as i32
            }) as ::core::ffi::c_char;
            if *p as ::core::ffi::c_int == '@' as i32
                && (isspace(prev as ::core::ffi::c_uchar as ::core::ffi::c_int) != 0
                    || !strchr(
                        b"[]<>{}\0" as *const u8 as *const ::core::ffi::c_char,
                        prev as ::core::ffi::c_int,
                    )
                    .is_null())
            {
                let mut q_0: *const ::core::ffi::c_char =
                    p.offset(1 as ::core::ffi::c_int as isize);
                while q_0 < end
                    && (isalnum(*q_0 as ::core::ffi::c_uchar as ::core::ffi::c_int) != 0
                        || !strchr(
                            b"_.-@:\0" as *const u8 as *const ::core::ffi::c_char,
                            *q_0 as ::core::ffi::c_int,
                        )
                        .is_null())
                {
                    q_0 = q_0.offset(1);
                }
                let mut len: size_t = q_0.offset_from(p) as ::core::ffi::c_long as size_t;
                if len == 5 as size_t
                    && memcmp(
                        p as *const ::core::ffi::c_void,
                        b"@xpos\0" as *const u8 as *const ::core::ffi::c_char
                            as *const ::core::ffi::c_void,
                        5 as size_t,
                    ) == 0
                {
                    pbuf_real(&raw mut b, cpx);
                } else if len == 5 as size_t
                    && memcmp(
                        p as *const ::core::ffi::c_void,
                        b"@ypos\0" as *const u8 as *const ::core::ffi::c_char
                            as *const ::core::ffi::c_void,
                        5 as size_t,
                    ) == 0
                {
                    pbuf_real(&raw mut b, cpy);
                } else if let Some(page) = pdf_document::Document::named_reference(c, &String::from_utf8_lossy(std::slice::from_raw_parts(p.cast::<u8>(),len))) {
                    pbuf_printf(&raw mut b,b"%d 0 R\0".as_ptr().cast(),page);
                } else if len > 1 as size_t {
                    pbuf_printf(
                        &raw mut b,
                        b"%d 0 R\0" as *const u8 as *const ::core::ffi::c_char,
                        (*named_get(c, p, len, true_0 != 0)).obj,
                    );
                }
                p = q_0;
            } else {
                let fresh12 = p;
                p = p.offset(1);
                pbuf_putc(&raw mut b, *fresh12 as ::core::ffi::c_int);
            }
        }
    }
    pbuf_putc(&raw mut b, 0 as ::core::ffi::c_int);
    return b.data as *mut ::core::ffi::c_char;
}
unsafe extern "C" fn merge_dict(mut l: *mut kvlist, mut s: *const ::core::ffi::c_char) {
    let mut end: *const ::core::ffi::c_char = s.offset(strlen(s) as isize);
    let mut p: *const ::core::ffi::c_char = skip_ws(s, end);
    if p.offset(1 as ::core::ffi::c_int as isize) >= end
        || *p.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int != '<' as i32
        || *p.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int != '<' as i32
    {
        return;
    }
    p = p.offset(2 as ::core::ffi::c_int as isize);
    loop {
        p = skip_ws(p, end);
        if p >= end
            || *p as ::core::ffi::c_int == '>' as i32
                && p.offset(1 as ::core::ffi::c_int as isize) < end
                && *p.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int == '>' as i32
        {
            break;
        }
        if *p as ::core::ffi::c_int != '/' as i32 {
            let mut q: *const ::core::ffi::c_char = skip_value(p, end);
            if q == p {
                break;
            }
            p = q;
        } else {
            let mut k: *const ::core::ffi::c_char = p.offset(1 as ::core::ffi::c_int as isize);
            let mut ke: *const ::core::ffi::c_char = k;
            while ke < end
                && isspace(*ke as ::core::ffi::c_uchar as ::core::ffi::c_int) == 0
                && strchr(
                    b"()<>[]{}/%\0" as *const u8 as *const ::core::ffi::c_char,
                    *ke as ::core::ffi::c_int,
                )
                .is_null()
            {
                ke = ke.offset(1);
            }
            let mut vs: *const ::core::ffi::c_char = skip_ws(ke, end);
            let mut ve: *const ::core::ffi::c_char = skip_value(vs, end);
            kv_set(
                l,
                k,
                ke.offset_from(k) as ::core::ffi::c_long as size_t,
                vs,
                ve.offset_from(vs) as ::core::ffi::c_long as size_t,
            );
            p = ve;
        }
    }
}
unsafe extern "C" fn merge_resources(mut r: *mut resources, mut dict: *const ::core::ffi::c_char) {
    let mut tmp: kvlist = kvlist {
        items: ::core::ptr::null_mut::<kv>(),
        n: 0,
        cap: 0,
    };
    merge_dict(&raw mut tmp, dict);
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < tmp.n {
        let mut cat: ::core::ffi::c_int = -(1 as ::core::ffi::c_int);
        let mut j: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
        while j < NCAT as ::core::ffi::c_int {
            if strcmp((*tmp.items.offset(i as isize)).key, cat_names[j as usize]) == 0 {
                cat = j;
            }
            j += 1;
        }
        if !(cat < 0 as ::core::ffi::c_int) {
            let mut v: *const ::core::ffi::c_char = (*tmp.items.offset(i as isize)).value;
            if *v.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int == '<' as i32 {
                merge_dict(
                    (&raw mut (*r).cat as *mut kvlist).offset(cat as isize) as *mut kvlist,
                    v,
                );
            } else {
                free((*r).cat_ref[cat as usize] as *mut ::core::ffi::c_void);
                (*r).cat_ref[cat as usize] = strdup(v);
            }
        }
        i += 1;
    }
    kv_free(&raw mut tmp);
}
unsafe extern "C" fn parse_dimen(
    mut pp: *mut *const ::core::ffi::c_char,
    mut end: *const ::core::ffi::c_char,
    mut ok: *mut bool,
) -> ::core::ffi::c_double {
    let mut p: *const ::core::ffi::c_char = skip_ws(*pp, end);
    let mut q: *mut ::core::ffi::c_char = ::core::ptr::null_mut::<::core::ffi::c_char>();
    let mut v: ::core::ffi::c_double = strtod(p, &raw mut q);
    *ok = q != p as *mut ::core::ffi::c_char;
    p = skip_ws(q, end);
    static mut units: [C2RustUnnamed_6; 10] = [
        C2RustUnnamed_6 {
            u: b"truept\0" as *const u8 as *const ::core::ffi::c_char,
            f: 72.0f64 / 72.27f64,
        },
        C2RustUnnamed_6 {
            u: b"pt\0" as *const u8 as *const ::core::ffi::c_char,
            f: 72.0f64 / 72.27f64,
        },
        C2RustUnnamed_6 {
            u: b"bp\0" as *const u8 as *const ::core::ffi::c_char,
            f: 1 as ::core::ffi::c_int as ::core::ffi::c_double,
        },
        C2RustUnnamed_6 {
            u: b"in\0" as *const u8 as *const ::core::ffi::c_char,
            f: 72 as ::core::ffi::c_int as ::core::ffi::c_double,
        },
        C2RustUnnamed_6 {
            u: b"cm\0" as *const u8 as *const ::core::ffi::c_char,
            f: 72 as ::core::ffi::c_int as ::core::ffi::c_double / 2.54f64,
        },
        C2RustUnnamed_6 {
            u: b"mm\0" as *const u8 as *const ::core::ffi::c_char,
            f: 72 as ::core::ffi::c_int as ::core::ffi::c_double / 25.4f64,
        },
        C2RustUnnamed_6 {
            u: b"pc\0" as *const u8 as *const ::core::ffi::c_char,
            f: 12 as ::core::ffi::c_int as ::core::ffi::c_double * 72.0f64 / 72.27f64,
        },
        C2RustUnnamed_6 {
            u: b"dd\0" as *const u8 as *const ::core::ffi::c_char,
            f: 1238.0f64 / 1157 as ::core::ffi::c_int as ::core::ffi::c_double * 72.0f64 / 72.27f64,
        },
        C2RustUnnamed_6 {
            u: b"cc\0" as *const u8 as *const ::core::ffi::c_char,
            f: 12 as ::core::ffi::c_int as ::core::ffi::c_double * 1238.0f64
                / 1157 as ::core::ffi::c_int as ::core::ffi::c_double
                * 72.0f64
                / 72.27f64,
        },
        C2RustUnnamed_6 {
            u: b"sp\0" as *const u8 as *const ::core::ffi::c_char,
            f: 72.0f64 / 72.27f64 / 65536 as ::core::ffi::c_int as ::core::ffi::c_double,
        },
    ];
    let mut i: size_t = 0 as size_t;
    while i
        < (::core::mem::size_of::<[C2RustUnnamed_6; 10]>() as usize)
            .wrapping_div(::core::mem::size_of::<C2RustUnnamed_6>() as usize)
    {
        let mut n: size_t = strlen(units[i as usize].u);
        if end.offset_from(p) as ::core::ffi::c_long as size_t >= n
            && strncmp(p, units[i as usize].u, n) == 0
        {
            *pp = p.offset(n as isize);
            return v * units[i as usize].f;
        }
        i = i.wrapping_add(1);
    }
    *pp = q;
    return v;
}
unsafe extern "C" fn parse_paper(
    mut s: *const ::core::ffi::c_char,
    mut end: *const ::core::ffi::c_char,
    mut w: *mut ::core::ffi::c_double,
    mut h: *mut ::core::ffi::c_double,
) -> bool {
    let mut ok1: bool = false;
    let mut ok2: bool = false;
    let mut p: *const ::core::ffi::c_char = s;
    *w = parse_dimen(&raw mut p, end, &raw mut ok1);
    p = skip_ws(p, end);
    if p < end && *p as ::core::ffi::c_int == ',' as i32 {
        p = p.offset(1);
    }
    *h = parse_dimen(&raw mut p, end, &raw mut ok2);
    return ok1 as ::core::ffi::c_int != 0
        && ok2 as ::core::ffi::c_int != 0
        && *w > 0 as ::core::ffi::c_int as ::core::ffi::c_double
        && *h > 0 as ::core::ffi::c_int as ::core::ffi::c_double;
}
unsafe extern "C" fn page_size_prescan(
    mut c: *mut conv_ctx,
    mut p: *const ::core::ffi::c_uchar,
    mut end: *const ::core::ffi::c_uchar,
) {
    let mut w: ::core::ffi::c_double = (*c).default_w;
    let mut h: ::core::ffi::c_double = (*c).default_h;
    while p < end {
        let mut n: ::core::ffi::c_long = xdv_insn_length(p, end);
        if n <= 0 as ::core::ffi::c_long {
            break;
        }
        if *p.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
            >= DVI_XXX1 as ::core::ffi::c_int
            && (*p.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int)
                < DVI_XXX1 as ::core::ffi::c_int + 4 as ::core::ffi::c_int
        {
            let mut k: ::core::ffi::c_int = *p.offset(0 as ::core::ffi::c_int as isize)
                as ::core::ffi::c_int
                - DVI_XXX1 as ::core::ffi::c_int
                + 1 as ::core::ffi::c_int;
            let mut s: *const ::core::ffi::c_char = (p as *const ::core::ffi::c_char)
                .offset(1 as ::core::ffi::c_int as isize)
                .offset(k as isize);
            let mut se: *const ::core::ffi::c_char =
                (p as *const ::core::ffi::c_char).offset(n as isize);
            if se.offset_from(s) as ::core::ffi::c_long as size_t > 10 as size_t
                && strncmp(
                    s,
                    b"papersize=\0" as *const u8 as *const ::core::ffi::c_char,
                    10 as size_t,
                ) == 0
            {
                let mut pw: ::core::ffi::c_double = 0.;
                let mut ph: ::core::ffi::c_double = 0.;
                if parse_paper(
                    s.offset(10 as ::core::ffi::c_int as isize),
                    se,
                    &raw mut pw,
                    &raw mut ph,
                ) {
                    w = pw;
                    (*c).default_w = w;
                    h = ph;
                    (*c).default_h = h;
                }
            } else if se.offset_from(s) as ::core::ffi::c_long as size_t > 13 as size_t
                && strncmp(
                    s,
                    b"pdf:pagesize \0" as *const u8 as *const ::core::ffi::c_char,
                    13 as size_t,
                ) == 0
            {
                let mut q: *const ::core::ffi::c_char = s.offset(13 as ::core::ffi::c_int as isize);
                q = skip_ws(q, se);
                if se.offset_from(q) as ::core::ffi::c_long as size_t >= 7 as size_t
                    && strncmp(
                        q,
                        b"default\0" as *const u8 as *const ::core::ffi::c_char,
                        7 as size_t,
                    ) == 0
                {
                    w = (*c).default_w;
                    h = (*c).default_h;
                } else {
                    let mut pw_0: ::core::ffi::c_double =
                        0 as ::core::ffi::c_int as ::core::ffi::c_double;
                    let mut ph_0: ::core::ffi::c_double =
                        0 as ::core::ffi::c_int as ::core::ffi::c_double;
                    while q < se {
                        q = skip_ws(q, se);
                        let mut ok: bool = false;
                        if se.offset_from(q) as ::core::ffi::c_long as size_t >= 5 as size_t
                            && strncmp(
                                q,
                                b"width\0" as *const u8 as *const ::core::ffi::c_char,
                                5 as size_t,
                            ) == 0
                        {
                            q = q.offset(5 as ::core::ffi::c_int as isize);
                            pw_0 = parse_dimen(&raw mut q, se, &raw mut ok);
                        } else if se.offset_from(q) as ::core::ffi::c_long as size_t >= 6 as size_t
                            && strncmp(
                                q,
                                b"height\0" as *const u8 as *const ::core::ffi::c_char,
                                6 as size_t,
                            ) == 0
                        {
                            q = q.offset(6 as ::core::ffi::c_int as isize);
                            ph_0 = parse_dimen(&raw mut q, se, &raw mut ok);
                        } else {
                            while q < se
                                && isspace(*q as ::core::ffi::c_uchar as ::core::ffi::c_int) == 0
                            {
                                q = q.offset(1);
                            }
                        }
                    }
                    if pw_0 > 0 as ::core::ffi::c_int as ::core::ffi::c_double
                        && ph_0 > 0 as ::core::ffi::c_int as ::core::ffi::c_double
                    {
                        w = pw_0;
                        h = ph_0;
                        (*c).default_w = w;
                        (*c).default_h = h;
                    }
                }
            }
        }
        p = p.offset(n as isize);
    }
    (*c).page_w = w;
    (*c).page_h = h;
}
unsafe extern "C" fn set_colorstack_ops(mut c: *mut conv_ctx, mut ops: *const ::core::ffi::c_char) {
    if !ops.is_null() && *ops as ::core::ffi::c_int != 0 {
        emit_raw(c, ops, strlen(ops));
    }
}
unsafe extern "C" fn paren_content(
    mut p: *const ::core::ffi::c_char,
    mut end: *const ::core::ffi::c_char,
) -> *mut ::core::ffi::c_char {
    p = skip_ws(p, end);
    if p >= end || *p as ::core::ffi::c_int != '(' as i32 {
        return strdup(b"\0" as *const u8 as *const ::core::ffi::c_char);
    }
    let mut q: *const ::core::ffi::c_char = skip_value(p, end);
    if (q.offset_from(p) as ::core::ffi::c_long) < 2 as ::core::ffi::c_long {
        return strdup(b"\0" as *const u8 as *const ::core::ffi::c_char);
    }
    return xstrndup(
        p.offset(1 as ::core::ffi::c_int as isize),
        (q.offset_from(p) as ::core::ffi::c_long - 2 as ::core::ffi::c_long) as size_t,
    );
}
unsafe extern "C" fn color_spec_ops(
    mut s: *const ::core::ffi::c_char,
    mut end: *const ::core::ffi::c_char,
) -> *mut ::core::ffi::c_char {
    s = skip_ws(s, end);
    let mut v: [::core::ffi::c_double; 4] =
        [0 as ::core::ffi::c_int as ::core::ffi::c_double, 0., 0., 0.];
    let mut n: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut model: *const ::core::ffi::c_char = s;
    while s < end && isalpha(*s as ::core::ffi::c_uchar as ::core::ffi::c_int) != 0 {
        s = s.offset(1);
    }
    let mut mlen: size_t = s.offset_from(model) as ::core::ffi::c_long as size_t;
    while n < 4 as ::core::ffi::c_int {
        s = skip_ws(s, end);
        if s < end
            && (*s as ::core::ffi::c_int == '[' as i32 || *s as ::core::ffi::c_int == ']' as i32)
        {
            s = s.offset(1);
        } else {
            let mut q: *mut ::core::ffi::c_char = ::core::ptr::null_mut::<::core::ffi::c_char>();
            let mut x: ::core::ffi::c_double = strtod(s, &raw mut q);
            if q == s as *mut ::core::ffi::c_char {
                break;
            }
            let fresh14 = n;
            n = n + 1;
            v[fresh14 as usize] = x;
            s = q;
        }
    }
    let mut buf: [::core::ffi::c_char; 160] = [0; 160];
    if mlen == 3 as size_t
        && strncmp(
            model,
            b"rgb\0" as *const u8 as *const ::core::ffi::c_char,
            3 as size_t,
        ) == 0
        || mlen == 0 as size_t && n == 3 as ::core::ffi::c_int
    {
        snprintf(
            &raw mut buf as *mut ::core::ffi::c_char,
            ::core::mem::size_of::<[::core::ffi::c_char; 160]>() as size_t,
            b"%g %g %g rg %g %g %g RG\0" as *const u8 as *const ::core::ffi::c_char,
            v[0 as ::core::ffi::c_int as usize],
            v[1 as ::core::ffi::c_int as usize],
            v[2 as ::core::ffi::c_int as usize],
            v[0 as ::core::ffi::c_int as usize],
            v[1 as ::core::ffi::c_int as usize],
            v[2 as ::core::ffi::c_int as usize],
        );
    } else if mlen == 4 as size_t
        && strncmp(
            model,
            b"cmyk\0" as *const u8 as *const ::core::ffi::c_char,
            4 as size_t,
        ) == 0
        || mlen == 0 as size_t && n == 4 as ::core::ffi::c_int
    {
        snprintf(
            &raw mut buf as *mut ::core::ffi::c_char,
            ::core::mem::size_of::<[::core::ffi::c_char; 160]>() as size_t,
            b"%g %g %g %g k %g %g %g %g K\0" as *const u8 as *const ::core::ffi::c_char,
            v[0 as ::core::ffi::c_int as usize],
            v[1 as ::core::ffi::c_int as usize],
            v[2 as ::core::ffi::c_int as usize],
            v[3 as ::core::ffi::c_int as usize],
            v[0 as ::core::ffi::c_int as usize],
            v[1 as ::core::ffi::c_int as usize],
            v[2 as ::core::ffi::c_int as usize],
            v[3 as ::core::ffi::c_int as usize],
        );
    } else if mlen == 4 as size_t
        && strncmp(
            model,
            b"gray\0" as *const u8 as *const ::core::ffi::c_char,
            4 as size_t,
        ) == 0
        || mlen == 0 as size_t && n == 1 as ::core::ffi::c_int
    {
        snprintf(
            &raw mut buf as *mut ::core::ffi::c_char,
            ::core::mem::size_of::<[::core::ffi::c_char; 160]>() as size_t,
            b"%g g %g G\0" as *const u8 as *const ::core::ffi::c_char,
            v[0 as ::core::ffi::c_int as usize],
            v[0 as ::core::ffi::c_int as usize],
        );
    } else if mlen == 3 as size_t
        && strncmp(
            model,
            b"hsb\0" as *const u8 as *const ::core::ffi::c_char,
            3 as size_t,
        ) == 0
    {
        let mut h: ::core::ffi::c_double =
            v[0 as ::core::ffi::c_int as usize] * 6 as ::core::ffi::c_int as ::core::ffi::c_double;
        let mut sat: ::core::ffi::c_double = v[1 as ::core::ffi::c_int as usize];
        let mut br: ::core::ffi::c_double = v[2 as ::core::ffi::c_int as usize];
        let mut f: ::core::ffi::c_double = h - floor(h);
        let mut r: ::core::ffi::c_double = 0.;
        let mut g: ::core::ffi::c_double = 0.;
        let mut b: ::core::ffi::c_double = 0.;
        let mut p: ::core::ffi::c_double =
            br * (1 as ::core::ffi::c_int as ::core::ffi::c_double - sat);
        let mut q_0: ::core::ffi::c_double =
            br * (1 as ::core::ffi::c_int as ::core::ffi::c_double - sat * f);
        let mut t: ::core::ffi::c_double = br
            * (1 as ::core::ffi::c_int as ::core::ffi::c_double
                - sat * (1 as ::core::ffi::c_int as ::core::ffi::c_double - f));
        match floor(h) as ::core::ffi::c_int % 6 as ::core::ffi::c_int {
            0 => {
                r = br;
                g = t;
                b = p;
            }
            1 => {
                r = q_0;
                g = br;
                b = p;
            }
            2 => {
                r = p;
                g = br;
                b = t;
            }
            3 => {
                r = p;
                g = q_0;
                b = br;
            }
            4 => {
                r = t;
                g = p;
                b = br;
            }
            _ => {
                r = br;
                g = p;
                b = q_0;
            }
        }
        snprintf(
            &raw mut buf as *mut ::core::ffi::c_char,
            ::core::mem::size_of::<[::core::ffi::c_char; 160]>() as size_t,
            b"%g %g %g rg %g %g %g RG\0" as *const u8 as *const ::core::ffi::c_char,
            r,
            g,
            b,
            r,
            g,
            b,
        );
    } else if mlen == 5 as size_t
        && strncmp(
            model,
            b"Black\0" as *const u8 as *const ::core::ffi::c_char,
            5 as size_t,
        ) == 0
    {
        snprintf(
            &raw mut buf as *mut ::core::ffi::c_char,
            ::core::mem::size_of::<[::core::ffi::c_char; 160]>() as size_t,
            b"0 g 0 G\0" as *const u8 as *const ::core::ffi::c_char,
        );
    } else {
        return ::core::ptr::null_mut::<::core::ffi::c_char>();
    }
    return strdup(&raw mut buf as *mut ::core::ffi::c_char);
}
unsafe extern "C" fn do_pdfcolorstack(
    mut c: *mut conv_ctx,
    mut s: *const ::core::ffi::c_char,
    mut end: *const ::core::ffi::c_char,
    mut init: bool,
) {
    let mut q: *mut ::core::ffi::c_char = ::core::ptr::null_mut::<::core::ffi::c_char>();
    let mut n: ::core::ffi::c_long = strtol(s, &raw mut q, 10 as ::core::ffi::c_int);
    if q == s as *mut ::core::ffi::c_char
        || n < 0 as ::core::ffi::c_long
        || n >= MAX_COLORSTACKS as ::core::ffi::c_long
    {
        return;
    }
    let mut st: *mut colorstack =
        (&raw mut (*c).cs as *mut colorstack).offset(n as isize) as *mut colorstack;
    let mut p: *const ::core::ffi::c_char = skip_ws(q, end);
    if init {
        (*st).page = false_0 != 0;
        loop {
            p = skip_ws(p, end);
            if strncmp(
                p,
                b"page\0" as *const u8 as *const ::core::ffi::c_char,
                4 as size_t,
            ) == 0
            {
                (*st).page = true_0 != 0;
                p = p.offset(4 as ::core::ffi::c_int as isize);
            } else {
                if !(strncmp(
                    p,
                    b"direct\0" as *const u8 as *const ::core::ffi::c_char,
                    6 as size_t,
                ) == 0)
                {
                    break;
                }
                p = p.offset(6 as ::core::ffi::c_int as isize);
            }
        }
        free((*st).init as *mut ::core::ffi::c_void);
        (*st).init = paren_content(p, end);
        let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
        while i < (*st).depth {
            free((*st).ops[i as usize] as *mut ::core::ffi::c_void);
            i += 1;
        }
        (*st).depth = 0 as ::core::ffi::c_int;
        return;
    }
    if strncmp(
        p,
        b"push\0" as *const u8 as *const ::core::ffi::c_char,
        4 as size_t,
    ) == 0
    {
        let mut ops: *mut ::core::ffi::c_char =
            paren_content(p.offset(4 as ::core::ffi::c_int as isize), end);
        if (*st).depth < 64 as ::core::ffi::c_int {
            let fresh16 = (*st).depth;
            (*st).depth = (*st).depth + 1;
            (*st).ops[fresh16 as usize] = ops;
        } else {
            free(ops as *mut ::core::ffi::c_void);
        }
        set_colorstack_ops(
            c,
            if (*st).depth != 0 {
                (*st).ops[((*st).depth - 1 as ::core::ffi::c_int) as usize]
            } else {
                ::core::ptr::null_mut::<::core::ffi::c_char>()
            },
        );
    } else if strncmp(
        p,
        b"pop\0" as *const u8 as *const ::core::ffi::c_char,
        3 as size_t,
    ) == 0
    {
        if (*st).depth > 0 as ::core::ffi::c_int {
            (*st).depth -= 1;
            free((*st).ops[(*st).depth as usize] as *mut ::core::ffi::c_void);
        }
        set_colorstack_ops(
            c,
            if (*st).depth != 0 {
                (*st).ops[((*st).depth - 1 as ::core::ffi::c_int) as usize]
            } else {
                (*st).init
            },
        );
    } else if strncmp(
        p,
        b"set\0" as *const u8 as *const ::core::ffi::c_char,
        3 as size_t,
    ) == 0
    {
        let mut ops_0: *mut ::core::ffi::c_char =
            paren_content(p.offset(3 as ::core::ffi::c_int as isize), end);
        if (*st).depth > 0 as ::core::ffi::c_int {
            free(
                (*st).ops[((*st).depth - 1 as ::core::ffi::c_int) as usize]
                    as *mut ::core::ffi::c_void,
            );
            (*st).ops[((*st).depth - 1 as ::core::ffi::c_int) as usize] = ops_0;
        } else {
            free((*st).init as *mut ::core::ffi::c_void);
            (*st).init = ops_0;
        }
        set_colorstack_ops(
            c,
            if (*st).depth != 0 {
                (*st).ops[((*st).depth - 1 as ::core::ffi::c_int) as usize]
            } else {
                (*st).init
            },
        );
    } else if strncmp(
        p,
        b"current\0" as *const u8 as *const ::core::ffi::c_char,
        7 as size_t,
    ) == 0
    {
        set_colorstack_ops(
            c,
            if (*st).depth != 0 {
                (*st).ops[((*st).depth - 1 as ::core::ffi::c_int) as usize]
            } else {
                (*st).init
            },
        );
    }
}
unsafe extern "C" fn old_color_push(mut c: *mut conv_ctx, mut ops: *mut ::core::ffi::c_char) {
    if ops.is_null() {
        return;
    }
    if (*c).old_depth < 64 as ::core::ffi::c_int {
        let fresh15 = (*c).old_depth;
        (*c).old_depth = (*c).old_depth + 1;
        (*c).old_colors[fresh15 as usize] = ops;
    } else {
        free(ops as *mut ::core::ffi::c_void);
    }
    set_colorstack_ops(c, ops);
}
unsafe extern "C" fn old_color_pop(mut c: *mut conv_ctx) {
    if (*c).old_depth > 0 as ::core::ffi::c_int {
        (*c).old_depth -= 1;
        free((*c).old_colors[(*c).old_depth as usize] as *mut ::core::ffi::c_void);
    }
    set_colorstack_ops(
        c,
        if (*c).old_depth != 0 {
            (*c).old_colors[((*c).old_depth - 1 as ::core::ffi::c_int) as usize]
                as *const ::core::ffi::c_char
        } else {
            b"0 g 0 G\0" as *const u8 as *const ::core::ffi::c_char
        },
    );
}
unsafe extern "C" fn get_image(
    mut c: *mut conv_ctx,
    mut file: *const ::core::ffi::c_char,
) -> *mut cached_image {
    let mut data: *mut tbuf =
        (*(*c).w).res.load.expect("non-null function pointer")((*(*c).w).res.env, file, RES_IMAGE);
    if data.is_null() {
        warn_once(
            c,
            file,
            b"image not found: %s\0" as *const u8 as *const ::core::ffi::c_char,
            file,
        );
        return ::core::ptr::null_mut::<cached_image>();
    }
    let mut ci: *mut cached_image = (*(*c).w).images;
    while !ci.is_null() {
        if (*ci).file == data && (*ci).config_key == (*(*c).document).backend.image_key() {
            tbuf_drop(data);
            if (*ci).pdf&&!(*ci).doc.is_null()&&!pdf_inclusion::version(c,(*ci).doc,file){return std::ptr::null_mut();}
            return ci;
        }
        ci = (*ci).next as *mut cached_image;
    }
    let mut ci_0: *mut cached_image = calloc(
        1 as size_t,
        ::core::mem::size_of::<cached_image>() as size_t,
    ) as *mut cached_image;
    if ci_0.is_null() {
        abort();
    }
    (*ci_0).file = data;
    (*ci_0).filename = strdup(file);
    (*ci_0).config_key = (*(*c).document).backend.image_key();
    if (*data).len >= 5 as size_t
        && memcmp(
            (*data).data as *const ::core::ffi::c_void,
            b"%PDF-\0" as *const u8 as *const ::core::ffi::c_char as *const ::core::ffi::c_void,
            5 as size_t,
        ) == 0
    {
        (*ci_0).pdf = true_0 != 0;
        (*ci_0).doc = pr_open((*data).data, (*data).len);
        if (*ci_0).doc.is_null() {
            warn_once(
                c,
                file,
                b"unreadable PDF figure: %s\0" as *const u8 as *const ::core::ffi::c_char,
                file,
            );
        } else if pr_is_encrypted((*ci_0).doc) {
            warn_once(
                c,
                file,
                b"encrypted PDF figures are not previewed: %s\0" as *const u8
                    as *const ::core::ffi::c_char,
                file,
            );
            pr_close((*ci_0).doc);
            (*ci_0).doc = ::core::ptr::null_mut::<pr_doc>();
        }
    } else {
        image_encode(
            (*data).data,
            (*data).len,
            file,
            &raw mut (*ci_0).img,
            (*c).warnings,
        );
    }
    (*ci_0).next = (*(*c).w).images as *mut cached_image;
    (*(*c).w).images = ci_0;
    if (*ci_0).pdf&&!(*ci_0).doc.is_null()&&!pdf_inclusion::version(c,(*ci_0).doc,file){return std::ptr::null_mut();}
    return ci_0;
}
unsafe extern "C" fn use_xobject(
    mut c: *mut conv_ctx,
    mut ci: *mut cached_image,
    mut page: ::core::ffi::c_int,
    mut box_0: ::core::ffi::c_int,
) -> *mut xobj_use {
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < (*c).nxobjs {
        if (*(*c).xobjs.offset(i as isize)).ci == ci
            && (*(*c).xobjs.offset(i as isize)).page == page
            && (*(*c).xobjs.offset(i as isize)).box_0 == box_0
        {
            return (*c).xobjs.offset(i as isize) as *mut xobj_use;
        }
        i += 1;
    }
    if if (*ci).pdf as ::core::ffi::c_int != 0 {
        (*ci).doc.is_null() as ::core::ffi::c_int
    } else {
        !(*ci).img.ok as ::core::ffi::c_int
    } != 0
    {
        return ::core::ptr::null_mut::<xobj_use>();
    }
    let mut u: xobj_use = xobj_use {
        ci: ci,
        page: page,
        box_0: box_0,
        obj: 0 as ::core::ffi::c_int,
        name: [0; 96],
        w: 0 as ::core::ffi::c_int as ::core::ffi::c_double,
        h: 0 as ::core::ffi::c_int as ::core::ffi::c_double,
        bbox: [0 as ::core::ffi::c_int as ::core::ffi::c_double, 0., 0., 0.],
    };
    if (*ci).pdf {
        let mut info: pr_page_info = pr_page_info {
            box_0: [0.; 4],
            rotate: 0,
            page: ::core::ptr::null_mut::<pr_obj>(),
            resources: ::core::ptr::null_mut::<pr_obj>(),
        };
        if !pr_page(
            (*ci).doc,
            pr_normalize_page((*ci).doc, page) - 1 as ::core::ffi::c_int,
            if box_0 != 0 {
                box_0
            } else {
                PR_BOX_CROP as ::core::ffi::c_int
            },
            &raw mut info,
        ) {
            return ::core::ptr::null_mut::<xobj_use>();
        }
        let mut m: [::core::ffi::c_double; 6] = [0.; 6];
        pr_rotation_matrix(info.rotate, &raw mut m as *mut ::core::ffi::c_double);
        pr_transform_box(
            &raw mut m as *mut ::core::ffi::c_double as *const ::core::ffi::c_double,
            &raw mut info.box_0 as *mut ::core::ffi::c_double as *const ::core::ffi::c_double,
            &raw mut u.bbox as *mut ::core::ffi::c_double,
        );
    } else {
        u.w = (*ci).img.width as ::core::ffi::c_double * 72.0f64
            / (if (*ci).img.xdpi > 0 as ::core::ffi::c_int as ::core::ffi::c_double {
                (*ci).img.xdpi
            } else {
                72 as ::core::ffi::c_int as ::core::ffi::c_double
            });
        u.h = (*ci).img.height as ::core::ffi::c_double * 72.0f64
            / (if (*ci).img.ydpi > 0 as ::core::ffi::c_int as ::core::ffi::c_double {
                (*ci).img.ydpi
            } else {
                72 as ::core::ffi::c_int as ::core::ffi::c_double
            });
    }
    u.obj = pdfw_alloc((*c).pw);
    let mut name = format!("Im{}", (*c).nxobjs + 1);
    if (*(*c).document).backend.integer("pdfuniqueresname") > 0 {
        name.push_str(&font_output::namespace(c));
    }
    for (to, from) in u.name.iter_mut().zip(name.bytes()) { *to = from as _; }
    if (*c).nxobjs == (*c).capxobjs {
        (*c).capxobjs = if (*c).capxobjs != 0 {
            (*c).capxobjs * 2 as ::core::ffi::c_int
        } else {
            8 as ::core::ffi::c_int
        };
        (*c).xobjs = realloc(
            (*c).xobjs as *mut ::core::ffi::c_void,
            (::core::mem::size_of::<xobj_use>() as size_t).wrapping_mul((*c).capxobjs as size_t),
        ) as *mut xobj_use;
        if (*c).xobjs.is_null() {
            abort();
        }
    }
    *(*c).xobjs.offset((*c).nxobjs as isize) = u;
    let fresh13 = (*c).nxobjs;
    (*c).nxobjs = (*c).nxobjs + 1;
    return (*c).xobjs.offset(fresh13 as isize) as *mut xobj_use;
}
unsafe extern "C" fn do_image(
    mut c: *mut conv_ctx,
    mut s: *const ::core::ffi::c_char,
    mut end: *const ::core::ffi::c_char,
    mut cx: ::core::ffi::c_double,
    mut cy: ::core::ffi::c_double,
) {
    let mut m: mat = mat {
        a: 1 as ::core::ffi::c_int as ::core::ffi::c_double,
        b: 0 as ::core::ffi::c_int as ::core::ffi::c_double,
        c: 0 as ::core::ffi::c_int as ::core::ffi::c_double,
        d: 1 as ::core::ffi::c_int as ::core::ffi::c_double,
        e: 0 as ::core::ffi::c_int as ::core::ffi::c_double,
        f: 0 as ::core::ffi::c_int as ::core::ffi::c_double,
    };
    let mut have_matrix: bool = false_0 != 0;
    let mut hide: bool = false_0 != 0;
    let mut width: ::core::ffi::c_double = 0 as ::core::ffi::c_int as ::core::ffi::c_double;
    let mut height: ::core::ffi::c_double = 0 as ::core::ffi::c_int as ::core::ffi::c_double;
    let mut page: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
    let mut box_0: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut file: *mut ::core::ffi::c_char = ::core::ptr::null_mut::<::core::ffi::c_char>();
    let mut p: *const ::core::ffi::c_char = s;
    while p < end {
        p = skip_ws(p, end);
        if p >= end {
            break;
        }
        let mut ok: bool = false;
        if *p as ::core::ffi::c_int == '(' as i32 {
            let mut q: *const ::core::ffi::c_char = skip_value(p, end);
            file = xstrndup(
                p.offset(1 as ::core::ffi::c_int as isize),
                (if q.offset_from(p) as ::core::ffi::c_long >= 2 as ::core::ffi::c_long {
                    q.offset_from(p) as ::core::ffi::c_long - 2 as ::core::ffi::c_long
                } else {
                    0 as ::core::ffi::c_long
                }) as size_t,
            );
            p = q;
            break;
        } else if *p as ::core::ffi::c_int == '@' as i32 {
            while p < end && isspace(*p as ::core::ffi::c_uchar as ::core::ffi::c_int) == 0 {
                p = p.offset(1);
            }
        } else if strncmp(
            p,
            b"matrix\0" as *const u8 as *const ::core::ffi::c_char,
            6 as size_t,
        ) == 0
        {
            p = p.offset(6 as ::core::ffi::c_int as isize);
            let mut v: [::core::ffi::c_double; 6] = [0.; 6];
            let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
            while i < 6 as ::core::ffi::c_int {
                let mut q_0: *mut ::core::ffi::c_char =
                    ::core::ptr::null_mut::<::core::ffi::c_char>();
                v[i as usize] = strtod(p, &raw mut q_0);
                p = q_0;
                i += 1;
            }
            m = mat {
                a: v[0 as ::core::ffi::c_int as usize],
                b: v[1 as ::core::ffi::c_int as usize],
                c: v[2 as ::core::ffi::c_int as usize],
                d: v[3 as ::core::ffi::c_int as usize],
                e: v[4 as ::core::ffi::c_int as usize],
                f: v[5 as ::core::ffi::c_int as usize],
            };
            have_matrix = true_0 != 0;
        } else if strncmp(
            p,
            b"page\0" as *const u8 as *const ::core::ffi::c_char,
            4 as size_t,
        ) == 0
            && strncmp(
                p,
                b"pagebox\0" as *const u8 as *const ::core::ffi::c_char,
                7 as size_t,
            ) == 0
        {
            p = skip_ws(p.offset(7 as ::core::ffi::c_int as isize), end);
            static mut boxes: [*const ::core::ffi::c_char; 6] = [
                ::core::ptr::null::<::core::ffi::c_char>(),
                b"cropbox\0" as *const u8 as *const ::core::ffi::c_char,
                b"mediabox\0" as *const u8 as *const ::core::ffi::c_char,
                b"bleedbox\0" as *const u8 as *const ::core::ffi::c_char,
                b"trimbox\0" as *const u8 as *const ::core::ffi::c_char,
                b"artbox\0" as *const u8 as *const ::core::ffi::c_char,
            ];
            let mut i_0: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
            while i_0 <= 5 as ::core::ffi::c_int {
                if strncmp(p, boxes[i_0 as usize], strlen(boxes[i_0 as usize])) == 0 {
                    box_0 = i_0;
                }
                i_0 += 1;
            }
            while p < end && isspace(*p as ::core::ffi::c_uchar as ::core::ffi::c_int) == 0 {
                p = p.offset(1);
            }
        } else if strncmp(
            p,
            b"page\0" as *const u8 as *const ::core::ffi::c_char,
            4 as size_t,
        ) == 0
        {
            let mut q_1: *mut ::core::ffi::c_char = ::core::ptr::null_mut::<::core::ffi::c_char>();
            page = strtol(
                p.offset(4 as ::core::ffi::c_int as isize),
                &raw mut q_1,
                10 as ::core::ffi::c_int,
            ) as ::core::ffi::c_int;
            p = q_1;
        } else if strncmp(
            p,
            b"width\0" as *const u8 as *const ::core::ffi::c_char,
            5 as size_t,
        ) == 0
        {
            p = p.offset(5 as ::core::ffi::c_int as isize);
            width = parse_dimen(&raw mut p, end, &raw mut ok);
        } else if strncmp(
            p,
            b"height\0" as *const u8 as *const ::core::ffi::c_char,
            6 as size_t,
        ) == 0
        {
            p = p.offset(6 as ::core::ffi::c_int as isize);
            height = parse_dimen(&raw mut p, end, &raw mut ok);
        } else if strncmp(
            p,
            b"hide\0" as *const u8 as *const ::core::ffi::c_char,
            4 as size_t,
        ) == 0
        {
            hide = true_0 != 0;
            p = p.offset(4 as ::core::ffi::c_int as isize);
        } else {
            while p < end && isspace(*p as ::core::ffi::c_uchar as ::core::ffi::c_int) == 0 {
                p = p.offset(1);
            }
        }
    }
    if file.is_null() || hide as ::core::ffi::c_int != 0 || !(*c).render {
        free(file as *mut ::core::ffi::c_void);
        return;
    }
    let mut ci: *mut cached_image = get_image(c, file);
    let mut u: *mut xobj_use = if !ci.is_null() {
        use_xobject(c, ci, page, box_0)
    } else {
        ::core::ptr::null_mut::<xobj_use>()
    };
    if u.is_null() {
        free(file as *mut ::core::ffi::c_void);
        return;
    }
    pdf_inclusion::used(c,ci,page,box_0);
    free(file as *mut ::core::ffi::c_void);
    let mut place: mat = mat {
        a: 0.,
        b: 0.,
        c: 0.,
        d: 0.,
        e: 0.,
        f: 0.,
    };
    let mut bw: ::core::ffi::c_double = if (*ci).pdf as ::core::ffi::c_int != 0 {
        (*u).bbox[2 as ::core::ffi::c_int as usize] - (*u).bbox[0 as ::core::ffi::c_int as usize]
    } else {
        (*u).w
    };
    let mut bh: ::core::ffi::c_double = if (*ci).pdf as ::core::ffi::c_int != 0 {
        (*u).bbox[3 as ::core::ffi::c_int as usize] - (*u).bbox[1 as ::core::ffi::c_int as usize]
    } else {
        (*u).h
    };
    if have_matrix {
        place = m;
    } else {
        let mut sx: ::core::ffi::c_double = 1 as ::core::ffi::c_int as ::core::ffi::c_double;
        let mut sy: ::core::ffi::c_double = 1 as ::core::ffi::c_int as ::core::ffi::c_double;
        if width > 0 as ::core::ffi::c_int as ::core::ffi::c_double
            && height > 0 as ::core::ffi::c_int as ::core::ffi::c_double
        {
            sx = width / bw;
            sy = height / bh;
        } else if width > 0 as ::core::ffi::c_int as ::core::ffi::c_double {
            sy = width / bw;
            sx = sy;
        } else if height > 0 as ::core::ffi::c_int as ::core::ffi::c_double {
            sy = height / bh;
            sx = sy;
        }
        place = mat {
            a: sx,
            b: 0 as ::core::ffi::c_int as ::core::ffi::c_double,
            c: 0 as ::core::ffi::c_int as ::core::ffi::c_double,
            d: sy,
            e: 0 as ::core::ffi::c_int as ::core::ffi::c_double,
            f: 0 as ::core::ffi::c_int as ::core::ffi::c_double,
        };
        if (*ci).pdf {
            place = mat_mul(
                mat_translate(
                    -(*u).bbox[0 as ::core::ffi::c_int as usize],
                    -(*u).bbox[1 as ::core::ffi::c_int as usize],
                ),
                place,
            );
        }
    }
    if !(*ci).pdf {
        place = mat_mul(
            mat {
                a: (*u).w,
                b: 0 as ::core::ffi::c_int as ::core::ffi::c_double,
                c: 0 as ::core::ffi::c_int as ::core::ffi::c_double,
                d: (*u).h,
                e: 0 as ::core::ffi::c_int as ::core::ffi::c_double,
                f: 0 as ::core::ffi::c_int as ::core::ffi::c_double,
            },
            place,
        );
    }
    emit_q(c);
    emit_cm(c, mat_mul(place, mat_translate(cx, cy)));
    let bbox=if (*ci).pdf {(*u).bbox}else{[0.,0.,1.,1.]};
    let rectangle=pdf_document::Document::rectangle(c,bbox);
    pdf_document::Document::touch_rectangle(c,rectangle);
    pbuf_printf(
        &raw mut (*(cur as unsafe extern "C" fn(*mut conv_ctx) -> *mut target)(c)).content,
        b"/%s Do\n\0" as *const u8 as *const ::core::ffi::c_char,
        &raw mut (*u).name as *mut ::core::ffi::c_char,
    );
    emit_Q(c);
}
unsafe extern "C" fn parse_transform(
    mut p: *const ::core::ffi::c_char,
    mut end: *const ::core::ffi::c_char,
    mut m: *mut mat,
) -> bool {
    *m = mat {
        a: 1 as ::core::ffi::c_int as ::core::ffi::c_double,
        b: 0 as ::core::ffi::c_int as ::core::ffi::c_double,
        c: 0 as ::core::ffi::c_int as ::core::ffi::c_double,
        d: 1 as ::core::ffi::c_int as ::core::ffi::c_double,
        e: 0 as ::core::ffi::c_int as ::core::ffi::c_double,
        f: 0 as ::core::ffi::c_int as ::core::ffi::c_double,
    };
    let mut any: bool = false_0 != 0;
    while p < end {
        p = skip_ws(p, end);
        if p >= end {
            break;
        }
        let mut q: *mut ::core::ffi::c_char = ::core::ptr::null_mut::<::core::ffi::c_char>();
        if strncmp(
            p,
            b"rotate\0" as *const u8 as *const ::core::ffi::c_char,
            6 as size_t,
        ) == 0
        {
            let mut a: ::core::ffi::c_double =
                strtod(p.offset(6 as ::core::ffi::c_int as isize), &raw mut q) * M_PI / 180.0f64;
            *m = mat_mul(
                *m,
                mat {
                    a: cos(a),
                    b: sin(a),
                    c: -sin(a),
                    d: cos(a),
                    e: 0 as ::core::ffi::c_int as ::core::ffi::c_double,
                    f: 0 as ::core::ffi::c_int as ::core::ffi::c_double,
                },
            );
            p = q;
            any = true_0 != 0;
        } else if strncmp(
            p,
            b"xscale\0" as *const u8 as *const ::core::ffi::c_char,
            6 as size_t,
        ) == 0
        {
            let mut s: ::core::ffi::c_double =
                strtod(p.offset(6 as ::core::ffi::c_int as isize), &raw mut q);
            *m = mat_mul(
                *m,
                mat {
                    a: s,
                    b: 0 as ::core::ffi::c_int as ::core::ffi::c_double,
                    c: 0 as ::core::ffi::c_int as ::core::ffi::c_double,
                    d: 1 as ::core::ffi::c_int as ::core::ffi::c_double,
                    e: 0 as ::core::ffi::c_int as ::core::ffi::c_double,
                    f: 0 as ::core::ffi::c_int as ::core::ffi::c_double,
                },
            );
            p = q;
            any = true_0 != 0;
        } else if strncmp(
            p,
            b"yscale\0" as *const u8 as *const ::core::ffi::c_char,
            6 as size_t,
        ) == 0
        {
            let mut s_0: ::core::ffi::c_double =
                strtod(p.offset(6 as ::core::ffi::c_int as isize), &raw mut q);
            *m = mat_mul(
                *m,
                mat {
                    a: 1 as ::core::ffi::c_int as ::core::ffi::c_double,
                    b: 0 as ::core::ffi::c_int as ::core::ffi::c_double,
                    c: 0 as ::core::ffi::c_int as ::core::ffi::c_double,
                    d: s_0,
                    e: 0 as ::core::ffi::c_int as ::core::ffi::c_double,
                    f: 0 as ::core::ffi::c_int as ::core::ffi::c_double,
                },
            );
            p = q;
            any = true_0 != 0;
        } else if strncmp(
            p,
            b"scale\0" as *const u8 as *const ::core::ffi::c_char,
            5 as size_t,
        ) == 0
        {
            let mut sx: ::core::ffi::c_double =
                strtod(p.offset(5 as ::core::ffi::c_int as isize), &raw mut q);
            let mut r: *const ::core::ffi::c_char = q;
            let mut sy: ::core::ffi::c_double = strtod(r, &raw mut q);
            if q == r as *mut ::core::ffi::c_char {
                sy = sx;
            }
            *m = mat_mul(
                *m,
                mat {
                    a: sx,
                    b: 0 as ::core::ffi::c_int as ::core::ffi::c_double,
                    c: 0 as ::core::ffi::c_int as ::core::ffi::c_double,
                    d: sy,
                    e: 0 as ::core::ffi::c_int as ::core::ffi::c_double,
                    f: 0 as ::core::ffi::c_int as ::core::ffi::c_double,
                },
            );
            p = q;
            any = true_0 != 0;
        } else if strncmp(
            p,
            b"matrix\0" as *const u8 as *const ::core::ffi::c_char,
            6 as size_t,
        ) == 0
        {
            let mut v: [::core::ffi::c_double; 6] = [0.; 6];
            p = p.offset(6 as ::core::ffi::c_int as isize);
            let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
            while i < 6 as ::core::ffi::c_int {
                v[i as usize] = strtod(p, &raw mut q);
                p = q;
                i += 1;
            }
            *m = mat_mul(
                *m,
                mat {
                    a: v[0 as ::core::ffi::c_int as usize],
                    b: v[1 as ::core::ffi::c_int as usize],
                    c: v[2 as ::core::ffi::c_int as usize],
                    d: v[3 as ::core::ffi::c_int as usize],
                    e: v[4 as ::core::ffi::c_int as usize],
                    f: v[5 as ::core::ffi::c_int as usize],
                },
            );
            any = true_0 != 0;
        } else {
            while p < end && isspace(*p as ::core::ffi::c_uchar as ::core::ffi::c_int) == 0 {
                p = p.offset(1);
            }
        }
    }
    return any;
}
unsafe extern "C" fn push_offset(
    mut c: *mut conv_ctx,
    mut x: ::core::ffi::c_double,
    mut y: ::core::ffi::c_double,
) {
    if (*c).off_depth < 64 as ::core::ffi::c_int {
        (*c).off_stack[(*c).off_depth as usize][0 as ::core::ffi::c_int as usize] = (*c).ox;
        (*c).off_stack[(*c).off_depth as usize][1 as ::core::ffi::c_int as usize] = (*c).oy;
        (*c).off_depth += 1;
    }
    (*c).ox += x;
    (*c).oy += y;
}
unsafe extern "C" fn pop_offset(mut c: *mut conv_ctx) {
    if (*c).off_depth > 0 as ::core::ffi::c_int {
        (*c).off_depth -= 1;
        (*c).ox = (*c).off_stack[(*c).off_depth as usize][0 as ::core::ffi::c_int as usize];
        (*c).oy = (*c).off_stack[(*c).off_depth as usize][1 as ::core::ffi::c_int as usize];
    }
}
unsafe extern "C" fn do_special_bounded(
    mut c: *mut conv_ctx,
    mut s: *const ::core::ffi::c_char,
    mut n: size_t,
    mut h: ::core::ffi::c_double,
    mut v: ::core::ffi::c_double,
) {
    let mut copy: *mut ::core::ffi::c_char =
        malloc(n.wrapping_add(16 as size_t)) as *mut ::core::ffi::c_char;
    if copy.is_null() {
        abort();
    }
    memcpy(
        copy as *mut ::core::ffi::c_void,
        s as *const ::core::ffi::c_void,
        n,
    );
    memset(
        copy.offset(n as isize) as *mut ::core::ffi::c_void,
        0 as ::core::ffi::c_int,
        16 as size_t,
    );
    do_special(c, copy, n, h, v);
    free(copy as *mut ::core::ffi::c_void);
}
unsafe extern "C" fn do_special(
    mut c: *mut conv_ctx,
    mut s: *const ::core::ffi::c_char,
    mut n: size_t,
    mut h: ::core::ffi::c_double,
    mut v: ::core::ffi::c_double,
) {
    let mut end: *const ::core::ffi::c_char = s.offset(n as isize);
    let mut cx: ::core::ffi::c_double = X(c, h);
    let mut cy: ::core::ffi::c_double = Y(c, v);
    s = skip_ws(s, end);
    let special_text = String::from_utf8_lossy(std::slice::from_raw_parts(s.cast::<u8>(),end.offset_from(s) as usize));
    if font_output::special(c,&special_text,cx,cy) { return; }
    if let Some(value) = special_text.strip_prefix("pitex:font-expansion ") {
        if let Ok(ratio) = value.trim().parse::<f64>() {
            if ratio.is_finite() && ratio>0. && ratio<10. { (*c).expansion=ratio; }
        }
        return;
    }
    {
        let ps_result = (*(*c).postscript).special(&special_text,cx,cy,|name| {
            let name=std::ffi::CString::new(name).ok()?;
            let b=((*(*c).w).res.load? )((*(*c).w).res.env,name.as_ptr(),RES_IMAGE);
            if b.is_null() {return None;}
            let bytes=std::slice::from_raw_parts((*b).data,(*b).len).to_vec();
            tbuf_drop(b);Some(bytes)
        });
        match ps_result {
            Ok(Some(ops))=> {
                if !(*c).render {return;}
                for (name,fill,stroke) in (*(*c).postscript).opacity_resources() {
                    let resources=&raw mut (*cur(c)).res.cat[CAT_EXTGSTATE as usize];
                    let key=std::ffi::CString::new(name).unwrap();
                    // Reuse resource names across specials and pages.
                    let mut present=false;
                    for i in 0..(*resources).n {if strcmp((*(*resources).items.offset(i as isize)).key,key.as_ptr())==0 {present=true;break;}}
                    if !present {
                        let object=pdfw_alloc((*c).pw);let out=pdfw_out((*c).pw);
                        pdfw_begin((*c).pw,object);pbuf_printf(out,b"<</Type/ExtGState/ca %g/CA %g>>\0".as_ptr().cast(),fill,stroke);pdfw_end((*c).pw);
                        let reference=format!("{object} 0 R");kv_set(resources,key.as_ptr(),key.as_bytes().len(),reference.as_ptr().cast(),reference.len());
                    }
                }
                postscript_pdf::resources(c);
                emit_raw(c,ops.as_ptr().cast(),ops.len());return;
            },
            Err(error)=> {
                let error=std::ffi::CString::new(error.replace('\0'," ")).unwrap();
                warn_once(c,b"postscript\0".as_ptr().cast(),b"PostScript special: %s\0".as_ptr().cast(),error.as_ptr());return;
            },
            Ok(None)=>{},
        }
    }
    if end.offset_from(s) as ::core::ffi::c_long as size_t
        >= (::core::mem::size_of::<[::core::ffi::c_char; 18]>() as usize).wrapping_sub(1 as usize)
        && strncmp(
            s,
            b"pdfcolorstackinit\0" as *const u8 as *const ::core::ffi::c_char,
            (::core::mem::size_of::<[::core::ffi::c_char; 18]>() as size_t)
                .wrapping_sub(1 as size_t),
        ) == 0
    {
        pdf_document::Document::colorstack(
            c,
            s.offset(17 as ::core::ffi::c_int as isize),
            end,
            true_0 != 0, cx, cy,
        );
    } else if end.offset_from(s) as ::core::ffi::c_long as size_t
        >= (::core::mem::size_of::<[::core::ffi::c_char; 14]>() as usize).wrapping_sub(1 as usize)
        && strncmp(
            s,
            b"pdfcolorstack\0" as *const u8 as *const ::core::ffi::c_char,
            (::core::mem::size_of::<[::core::ffi::c_char; 14]>() as size_t)
                .wrapping_sub(1 as size_t),
        ) == 0
    {
        pdf_document::Document::colorstack(
            c,
            skip_ws(s.offset(13 as ::core::ffi::c_int as isize), end),
            end,
            false_0 != 0, cx, cy,
        );
    } else if end.offset_from(s) as ::core::ffi::c_long as size_t
        >= (::core::mem::size_of::<[::core::ffi::c_char; 11]>() as usize).wrapping_sub(1 as usize)
        && strncmp(
            s,
            b"color push\0" as *const u8 as *const ::core::ffi::c_char,
            (::core::mem::size_of::<[::core::ffi::c_char; 11]>() as size_t)
                .wrapping_sub(1 as size_t),
        ) == 0
    {
        old_color_push(
            c,
            color_spec_ops(s.offset(10 as ::core::ffi::c_int as isize), end),
        );
    } else if end.offset_from(s) as ::core::ffi::c_long as size_t
        >= (::core::mem::size_of::<[::core::ffi::c_char; 10]>() as usize).wrapping_sub(1 as usize)
        && strncmp(
            s,
            b"color pop\0" as *const u8 as *const ::core::ffi::c_char,
            (::core::mem::size_of::<[::core::ffi::c_char; 10]>() as size_t)
                .wrapping_sub(1 as size_t),
        ) == 0
    {
        old_color_pop(c);
    } else if end.offset_from(s) as ::core::ffi::c_long as size_t
        >= (::core::mem::size_of::<[::core::ffi::c_char; 7]>() as usize).wrapping_sub(1 as usize)
        && strncmp(
            s,
            b"color \0" as *const u8 as *const ::core::ffi::c_char,
            (::core::mem::size_of::<[::core::ffi::c_char; 7]>() as size_t)
                .wrapping_sub(1 as size_t),
        ) == 0
    {
        let mut ops: *mut ::core::ffi::c_char =
            color_spec_ops(s.offset(6 as ::core::ffi::c_int as isize), end);
        if !ops.is_null() {
            set_colorstack_ops(c, ops);
            free(ops as *mut ::core::ffi::c_void);
        }
    } else if end.offset_from(s) as ::core::ffi::c_long as size_t
        >= (::core::mem::size_of::<[::core::ffi::c_char; 11]>() as usize).wrapping_sub(1 as usize)
        && strncmp(
            s,
            b"background\0" as *const u8 as *const ::core::ffi::c_char,
            (::core::mem::size_of::<[::core::ffi::c_char; 11]>() as size_t)
                .wrapping_sub(1 as size_t),
        ) == 0
    {
        let mut ops_0: *mut ::core::ffi::c_char =
            color_spec_ops(s.offset(10 as ::core::ffi::c_int as isize), end);
        if !ops_0.is_null() {
            let mut r: ::core::ffi::c_double = 1 as ::core::ffi::c_int as ::core::ffi::c_double;
            let mut g: ::core::ffi::c_double = 1 as ::core::ffi::c_int as ::core::ffi::c_double;
            let mut b: ::core::ffi::c_double = 1 as ::core::ffi::c_int as ::core::ffi::c_double;
            if sscanf(
                ops_0,
                b"%lf %lf %lf rg\0" as *const u8 as *const ::core::ffi::c_char,
                &raw mut r,
                &raw mut g,
                &raw mut b,
            ) == 3 as ::core::ffi::c_int
                || sscanf(
                    ops_0,
                    b"%lf g\0" as *const u8 as *const ::core::ffi::c_char,
                    &raw mut r,
                ) == 1 as ::core::ffi::c_int
            {
                if strstr(ops_0, b"rg\0" as *const u8 as *const ::core::ffi::c_char).is_null() {
                    b = r;
                    g = b;
                }
                (*c).background[0 as ::core::ffi::c_int as usize] = r;
                (*c).background[1 as ::core::ffi::c_int as usize] = g;
                (*c).background[2 as ::core::ffi::c_int as usize] = b;
                (*c).has_background = !(r == 1 as ::core::ffi::c_int as ::core::ffi::c_double
                    && g == 1 as ::core::ffi::c_int as ::core::ffi::c_double
                    && b == 1 as ::core::ffi::c_int as ::core::ffi::c_double);
            }
            free(ops_0 as *mut ::core::ffi::c_void);
        }
    } else if end.offset_from(s) as ::core::ffi::c_long as size_t
        >= (::core::mem::size_of::<[::core::ffi::c_char; 8]>() as usize).wrapping_sub(1 as usize)
        && strncmp(
            s,
            b"x:gsave\0" as *const u8 as *const ::core::ffi::c_char,
            (::core::mem::size_of::<[::core::ffi::c_char; 8]>() as size_t)
                .wrapping_sub(1 as size_t),
        ) == 0
    {
        emit_q(c);
    } else if end.offset_from(s) as ::core::ffi::c_long as size_t
        >= (::core::mem::size_of::<[::core::ffi::c_char; 11]>() as usize).wrapping_sub(1 as usize)
        && strncmp(
            s,
            b"x:grestore\0" as *const u8 as *const ::core::ffi::c_char,
            (::core::mem::size_of::<[::core::ffi::c_char; 11]>() as size_t)
                .wrapping_sub(1 as size_t),
        ) == 0
    {
        emit_Q(c);
    } else if end.offset_from(s) as ::core::ffi::c_long as size_t
        >= (::core::mem::size_of::<[::core::ffi::c_char; 9]>() as usize).wrapping_sub(1 as usize)
        && strncmp(
            s,
            b"x:rotate\0" as *const u8 as *const ::core::ffi::c_char,
            (::core::mem::size_of::<[::core::ffi::c_char; 9]>() as size_t)
                .wrapping_sub(1 as size_t),
        ) == 0
    {
        let mut a: ::core::ffi::c_double = strtod(
            s.offset(8 as ::core::ffi::c_int as isize),
            ::core::ptr::null_mut::<*mut ::core::ffi::c_char>(),
        ) * M_PI
            / 180.0f64;
        emit_cm(
            c,
            mat_about(
                mat {
                    a: cos(a),
                    b: sin(a),
                    c: -sin(a),
                    d: cos(a),
                    e: 0 as ::core::ffi::c_int as ::core::ffi::c_double,
                    f: 0 as ::core::ffi::c_int as ::core::ffi::c_double,
                },
                cx,
                cy,
            ),
        );
    } else if end.offset_from(s) as ::core::ffi::c_long as size_t
        >= (::core::mem::size_of::<[::core::ffi::c_char; 8]>() as usize).wrapping_sub(1 as usize)
        && strncmp(
            s,
            b"x:scale\0" as *const u8 as *const ::core::ffi::c_char,
            (::core::mem::size_of::<[::core::ffi::c_char; 8]>() as size_t)
                .wrapping_sub(1 as size_t),
        ) == 0
    {
        let mut q: *mut ::core::ffi::c_char = ::core::ptr::null_mut::<::core::ffi::c_char>();
        let mut sx: ::core::ffi::c_double =
            strtod(s.offset(7 as ::core::ffi::c_int as isize), &raw mut q);
        let mut sy: ::core::ffi::c_double =
            strtod(q, ::core::ptr::null_mut::<*mut ::core::ffi::c_char>());
        emit_cm(
            c,
            mat_about(
                mat {
                    a: sx,
                    b: 0 as ::core::ffi::c_int as ::core::ffi::c_double,
                    c: 0 as ::core::ffi::c_int as ::core::ffi::c_double,
                    d: sy,
                    e: 0 as ::core::ffi::c_int as ::core::ffi::c_double,
                    f: 0 as ::core::ffi::c_int as ::core::ffi::c_double,
                },
                cx,
                cy,
            ),
        );
    } else if end.offset_from(s) as ::core::ffi::c_long as size_t
        >= (::core::mem::size_of::<[::core::ffi::c_char; 5]>() as usize).wrapping_sub(1 as usize)
        && strncmp(
            s,
            b"pdf:\0" as *const u8 as *const ::core::ffi::c_char,
            (::core::mem::size_of::<[::core::ffi::c_char; 5]>() as size_t)
                .wrapping_sub(1 as size_t),
        ) == 0
        || end.offset_from(s) as ::core::ffi::c_long as size_t
            >= (::core::mem::size_of::<[::core::ffi::c_char; 3]>() as usize)
                .wrapping_sub(1 as usize)
            && strncmp(
                s,
                b"x:\0" as *const u8 as *const ::core::ffi::c_char,
                (::core::mem::size_of::<[::core::ffi::c_char; 3]>() as size_t)
                    .wrapping_sub(1 as size_t),
            ) == 0
    {
        let mut p: *const ::core::ffi::c_char = skip_ws(
            s.offset(
                (if end.offset_from(s) as ::core::ffi::c_long as size_t
                    >= (::core::mem::size_of::<[::core::ffi::c_char; 5]>() as usize)
                        .wrapping_sub(1 as usize)
                    && strncmp(
                        s,
                        b"pdf:\0" as *const u8 as *const ::core::ffi::c_char,
                        (::core::mem::size_of::<[::core::ffi::c_char; 5]>() as size_t)
                            .wrapping_sub(1 as size_t),
                    ) == 0
                {
                    4 as ::core::ffi::c_int
                } else {
                    2 as ::core::ffi::c_int
                }) as isize,
            ),
            end,
        );
        let mut cmd: *const ::core::ffi::c_char = p;
        while p < end && isalpha(*p as ::core::ffi::c_uchar as ::core::ffi::c_int) != 0 {
            p = p.offset(1);
        }
        let mut cl: size_t = p.offset_from(cmd) as ::core::ffi::c_long as size_t;
        let mut arg: *const ::core::ffi::c_char = skip_ws(p, end);
        if pdf_document::Document::special(c,cmd,cl,arg,end,cx,cy) { return; }

        if !(cl
            == (::core::mem::size_of::<[::core::ffi::c_char; 9]>() as usize)
                .wrapping_sub(1 as usize)
            && strncmp(
                cmd,
                b"pagesize\0" as *const u8 as *const ::core::ffi::c_char,
                cl,
            ) == 0)
        {
            if cl
                == (::core::mem::size_of::<[::core::ffi::c_char; 7]>() as usize)
                    .wrapping_sub(1 as usize)
                && strncmp(
                    cmd,
                    b"btrans\0" as *const u8 as *const ::core::ffi::c_char,
                    cl,
                ) == 0
                || cl
                    == (::core::mem::size_of::<[::core::ffi::c_char; 3]>() as usize)
                        .wrapping_sub(1 as usize)
                    && strncmp(cmd, b"bt\0" as *const u8 as *const ::core::ffi::c_char, cl) == 0
            {
                emit_q(c);
                let mut m: mat = mat {
                    a: 0.,
                    b: 0.,
                    c: 0.,
                    d: 0.,
                    e: 0.,
                    f: 0.,
                };
                if parse_transform(arg, end, &raw mut m) {
                    emit_cm(c, mat_about(m, cx, cy));
                }
            } else if cl
                == (::core::mem::size_of::<[::core::ffi::c_char; 7]>() as usize)
                    .wrapping_sub(1 as usize)
                && strncmp(
                    cmd,
                    b"etrans\0" as *const u8 as *const ::core::ffi::c_char,
                    cl,
                ) == 0
                || cl
                    == (::core::mem::size_of::<[::core::ffi::c_char; 3]>() as usize)
                        .wrapping_sub(1 as usize)
                    && strncmp(cmd, b"et\0" as *const u8 as *const ::core::ffi::c_char, cl) == 0
            {
                emit_Q(c);
            } else if cl
                == (::core::mem::size_of::<[::core::ffi::c_char; 9]>() as usize)
                    .wrapping_sub(1 as usize)
                && strncmp(
                    cmd,
                    b"bcontent\0" as *const u8 as *const ::core::ffi::c_char,
                    cl,
                ) == 0
            {
                emit_q(c);
                emit_cm(c, mat_translate(cx, cy));
                push_offset(c, cx, cy);
            } else if cl
                == (::core::mem::size_of::<[::core::ffi::c_char; 9]>() as usize)
                    .wrapping_sub(1 as usize)
                && strncmp(
                    cmd,
                    b"econtent\0" as *const u8 as *const ::core::ffi::c_char,
                    cl,
                ) == 0
            {
                pop_offset(c);
                emit_Q(c);
            } else if cl
                == (::core::mem::size_of::<[::core::ffi::c_char; 5]>() as usize)
                    .wrapping_sub(1 as usize)
                && strncmp(
                    cmd,
                    b"code\0" as *const u8 as *const ::core::ffi::c_char,
                    cl,
                ) == 0
                || cl
                    == (::core::mem::size_of::<[::core::ffi::c_char; 7]>() as usize)
                        .wrapping_sub(1 as usize)
                    && strncmp(
                        cmd,
                        b"direct\0" as *const u8 as *const ::core::ffi::c_char,
                        cl,
                    ) == 0
            {
                emit_raw(
                    c,
                    arg,
                    end.offset_from(arg) as ::core::ffi::c_long as size_t,
                );
            } else if cl
                == (::core::mem::size_of::<[::core::ffi::c_char; 8]>() as usize)
                    .wrapping_sub(1 as usize)
                && strncmp(
                    cmd,
                    b"literal\0" as *const u8 as *const ::core::ffi::c_char,
                    cl,
                ) == 0
            {
                if strncmp(
                    arg,
                    b"direct\0" as *const u8 as *const ::core::ffi::c_char,
                    6 as size_t,
                ) == 0
                {
                    emit_raw(
                        c,
                        arg.offset(6 as ::core::ffi::c_int as isize),
                        (end.offset_from(arg) as ::core::ffi::c_long - 6 as ::core::ffi::c_long)
                            as size_t,
                    );
                } else {
                    emit_cm(c, mat_translate(cx, cy));
                    emit_raw(
                        c,
                        arg,
                        end.offset_from(arg) as ::core::ffi::c_long as size_t,
                    );
                    emit_cm(c, mat_translate(-cx, -cy));
                }
            } else if cl
                == (::core::mem::size_of::<[::core::ffi::c_char; 8]>() as usize)
                    .wrapping_sub(1 as usize)
                && strncmp(
                    cmd,
                    b"content\0" as *const u8 as *const ::core::ffi::c_char,
                    cl,
                ) == 0
            {
                emit_q(c);
                emit_cm(c, mat_translate(cx, cy));
                emit_raw(
                    c,
                    arg,
                    end.offset_from(arg) as ::core::ffi::c_long as size_t,
                );
                emit_Q(c);
            } else if cl
                == (::core::mem::size_of::<[::core::ffi::c_char; 7]>() as usize)
                    .wrapping_sub(1 as usize)
                && strncmp(
                    cmd,
                    b"bcolor\0" as *const u8 as *const ::core::ffi::c_char,
                    cl,
                ) == 0
                || cl
                    == (::core::mem::size_of::<[::core::ffi::c_char; 3]>() as usize)
                        .wrapping_sub(1 as usize)
                    && strncmp(cmd, b"bc\0" as *const u8 as *const ::core::ffi::c_char, cl) == 0
            {
                old_color_push(c, color_spec_ops(arg, end));
            } else if cl
                == (::core::mem::size_of::<[::core::ffi::c_char; 7]>() as usize)
                    .wrapping_sub(1 as usize)
                && strncmp(
                    cmd,
                    b"ecolor\0" as *const u8 as *const ::core::ffi::c_char,
                    cl,
                ) == 0
                || cl
                    == (::core::mem::size_of::<[::core::ffi::c_char; 3]>() as usize)
                        .wrapping_sub(1 as usize)
                    && strncmp(cmd, b"ec\0" as *const u8 as *const ::core::ffi::c_char, cl) == 0
            {
                old_color_pop(c);
            } else if cl
                == (::core::mem::size_of::<[::core::ffi::c_char; 7]>() as usize)
                    .wrapping_sub(1 as usize)
                && strncmp(
                    cmd,
                    b"scolor\0" as *const u8 as *const ::core::ffi::c_char,
                    cl,
                ) == 0
                || cl
                    == (::core::mem::size_of::<[::core::ffi::c_char; 3]>() as usize)
                        .wrapping_sub(1 as usize)
                    && strncmp(cmd, b"sc\0" as *const u8 as *const ::core::ffi::c_char, cl) == 0
            {
                let mut ops_1: *mut ::core::ffi::c_char = color_spec_ops(arg, end);
                if !ops_1.is_null() {
                    set_colorstack_ops(c, ops_1);
                    free(ops_1 as *mut ::core::ffi::c_void);
                }
            } else if cl
                == (::core::mem::size_of::<[::core::ffi::c_char; 6]>() as usize)
                    .wrapping_sub(1 as usize)
                && strncmp(
                    cmd,
                    b"image\0" as *const u8 as *const ::core::ffi::c_char,
                    cl,
                ) == 0
            {
                do_image(c, arg, end, cx, cy);
            } else if cl
                == (::core::mem::size_of::<[::core::ffi::c_char; 4]>() as usize)
                    .wrapping_sub(1 as usize)
                && strncmp(cmd, b"obj\0" as *const u8 as *const ::core::ffi::c_char, cl) == 0
            {
                let mut q_0: *const ::core::ffi::c_char = arg;
                if q_0 < end && *q_0 as ::core::ffi::c_int == '@' as i32 {
                    let mut ne: *const ::core::ffi::c_char = q_0;
                    while ne < end
                        && isspace(*ne as ::core::ffi::c_uchar as ::core::ffi::c_int) == 0
                    {
                        if b"()<>[]{}/%".contains(&(*ne as u8)) { break; }
                    ne = ne.offset(1);
                    }
                    let mut val: *mut ::core::ffi::c_char = if std::slice::from_raw_parts(q_0.cast::<u8>(),ne.offset_from(q_0) as usize).starts_with(b"@pitexobj") {
                        pdf_document::substitute_pdftex(c,ne,end.offset_from(ne) as usize,cx,cy)
                    } else { subst_names(c,ne,end.offset_from(ne) as usize,cx,cy) };
                    // Resolve references before retaining a named-object pointer.
                    let mut o: *mut named_obj = named_get(
                        c,
                        q_0,
                        ne.offset_from(q_0) as ::core::ffi::c_long as size_t,
                        true_0 != 0,
                    );
                    let mut vs: *const ::core::ffi::c_char =
                        skip_ws(val, val.offset(strlen(val) as isize));
                    if *vs.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                        == '<' as i32
                        && *vs.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                            == '<' as i32
                    {
                        if (*o).kind as ::core::ffi::c_uint
                            != NO_DICT as ::core::ffi::c_int as ::core::ffi::c_uint
                        {
                            kv_free(&raw mut (*o).dict);
                            (*o).kind = NO_DICT;
                        }
                        merge_dict(&raw mut (*o).dict, vs);
                    } else if *vs.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                        == '[' as i32
                    {
                        (*o).kind = NO_ARRAY;
                        pbuf_clear(&raw mut (*o).array);
                        pbuf_append(
                            &raw mut (*o).array,
                            vs.offset(1 as ::core::ffi::c_int as isize)
                                as *const ::core::ffi::c_void,
                            (if skip_value(vs, vs.offset(strlen(vs) as isize)).offset_from(vs)
                                as ::core::ffi::c_long
                                - 2 as ::core::ffi::c_long
                                > 0 as ::core::ffi::c_long
                            {
                                skip_value(vs, vs.offset(strlen(vs) as isize)).offset_from(vs)
                                    as ::core::ffi::c_long
                                    - 2 as ::core::ffi::c_long
                            } else {
                                0 as ::core::ffi::c_long
                            }) as size_t,
                        );
                    } else {
                        (*o).kind = NO_RAW;
                        pbuf_clear(&raw mut (*o).array);
                        pbuf_puts(&raw mut (*o).array, vs);
                    }
                    free(val as *mut ::core::ffi::c_void);
                }
            } else if cl
                == (::core::mem::size_of::<[::core::ffi::c_char; 4]>() as usize)
                    .wrapping_sub(1 as usize)
                && strncmp(cmd, b"put\0" as *const u8 as *const ::core::ffi::c_char, cl) == 0
            {
                let mut q_1: *const ::core::ffi::c_char = arg;
                let mut ne_0: *const ::core::ffi::c_char = q_1;
                while ne_0 < end
                    && isspace(*ne_0 as ::core::ffi::c_uchar as ::core::ffi::c_int) == 0
                {
                    if b"()<>[]{}/%".contains(&(*ne_0 as u8)) { break; }
                    ne_0 = ne_0.offset(1);
                }
                let mut val_0: *mut ::core::ffi::c_char = subst_names(
                    c,
                    ne_0,
                    end.offset_from(ne_0) as ::core::ffi::c_long as size_t,
                    cx,
                    cy,
                );
                if ne_0.offset_from(q_1) as ::core::ffi::c_long as size_t == 10 as size_t
                    && strncmp(
                        q_1,
                        b"@resources\0" as *const u8 as *const ::core::ffi::c_char,
                        10 as size_t,
                    ) == 0
                {
                    merge_resources(
                        &raw mut (*(cur as unsafe extern "C" fn(*mut conv_ctx) -> *mut target)(c))
                            .res,
                        val_0,
                    );
                } else if q_1 < end
                    && *q_1 as ::core::ffi::c_int == '@' as i32
                    && strncmp(
                        q_1,
                        b"@thispage\0" as *const u8 as *const ::core::ffi::c_char,
                        9 as size_t,
                    ) != 0
                    && strncmp(
                        q_1,
                        b"@prevpage\0" as *const u8 as *const ::core::ffi::c_char,
                        9 as size_t,
                    ) != 0
                    && strncmp(
                        q_1,
                        b"@nextpage\0" as *const u8 as *const ::core::ffi::c_char,
                        9 as size_t,
                    ) != 0
                {
                    let mut o_0: *mut named_obj = named_get(
                        c,
                        q_1,
                        ne_0.offset_from(q_1) as ::core::ffi::c_long as size_t,
                        true_0 != 0,
                    );
                    let mut vs_0: *const ::core::ffi::c_char =
                        skip_ws(val_0, val_0.offset(strlen(val_0) as isize));
                    if (*o_0).kind as ::core::ffi::c_uint
                        == NO_UNDEF as ::core::ffi::c_int as ::core::ffi::c_uint
                    {
                        (*o_0).kind = (if *vs_0.offset(0 as ::core::ffi::c_int as isize)
                            as ::core::ffi::c_int
                            == '[' as i32
                        {
                            NO_ARRAY as ::core::ffi::c_int
                        } else {
                            NO_DICT as ::core::ffi::c_int
                        }) as named_kind;
                    }
                    if (*o_0).kind as ::core::ffi::c_uint
                        == NO_DICT as ::core::ffi::c_int as ::core::ffi::c_uint
                        || (*o_0).kind as ::core::ffi::c_uint
                            == NO_STREAM as ::core::ffi::c_int as ::core::ffi::c_uint
                        || (*o_0).kind as ::core::ffi::c_uint
                            == NO_FORM as ::core::ffi::c_int as ::core::ffi::c_uint
                    {
                        merge_dict(&raw mut (*o_0).dict, vs_0);
                    } else if (*o_0).kind as ::core::ffi::c_uint
                        == NO_ARRAY as ::core::ffi::c_int as ::core::ffi::c_uint
                        && *vs_0.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                            == '[' as i32
                    {
                        let mut ve: *const ::core::ffi::c_char =
                            skip_value(vs_0, vs_0.offset(strlen(vs_0) as isize));
                        if (*o_0).array.len != 0 {
                            pbuf_putc(&raw mut (*o_0).array, ' ' as i32);
                        }
                        if ve.offset_from(vs_0) as ::core::ffi::c_long >= 2 as ::core::ffi::c_long {
                            pbuf_append(
                                &raw mut (*o_0).array,
                                vs_0.offset(1 as ::core::ffi::c_int as isize)
                                    as *const ::core::ffi::c_void,
                                (ve.offset_from(vs_0) as ::core::ffi::c_long
                                    - 2 as ::core::ffi::c_long)
                                    as size_t,
                            );
                        }
                    } else if (*o_0).kind == NO_ARRAY {
                        if (*o_0).array.len != 0 { pbuf_putc(&raw mut (*o_0).array,' ' as i32); }
                        pbuf_puts(&raw mut (*o_0).array,vs_0);
                    }
                }
                free(val_0 as *mut ::core::ffi::c_void);
            } else if cl
                == (::core::mem::size_of::<[::core::ffi::c_char; 7]>() as usize)
                    .wrapping_sub(1 as usize)
                && strncmp(
                    cmd,
                    b"stream\0" as *const u8 as *const ::core::ffi::c_char,
                    cl,
                ) == 0
                || cl
                    == (::core::mem::size_of::<[::core::ffi::c_char; 8]>() as usize)
                        .wrapping_sub(1 as usize)
                    && strncmp(
                        cmd,
                        b"fstream\0" as *const u8 as *const ::core::ffi::c_char,
                        cl,
                    ) == 0
            {
                let mut q_2: *const ::core::ffi::c_char = arg;
                let mut ne_1: *const ::core::ffi::c_char = q_2;
                while ne_1 < end
                    && isspace(*ne_1 as ::core::ffi::c_uchar as ::core::ffi::c_int) == 0
                {
                    if b"()<>[]{}/%".contains(&(*ne_1 as u8)) { break; }
                    ne_1 = ne_1.offset(1);
                }
                if q_2 < end
                    && *q_2 as ::core::ffi::c_int == '@' as i32
                    && (cl
                        == (::core::mem::size_of::<[::core::ffi::c_char; 7]>() as usize)
                            .wrapping_sub(1 as usize)
                        && strncmp(
                            cmd,
                            b"stream\0" as *const u8 as *const ::core::ffi::c_char,
                            cl,
                        ) == 0)
                {
                    let mut o_1: *mut named_obj = named_get(
                        c,
                        q_2,
                        ne_1.offset_from(q_2) as ::core::ffi::c_long as size_t,
                        true_0 != 0,
                    );
                    let mut ds: *const ::core::ffi::c_char = skip_ws(ne_1, end);
                    let mut de: *const ::core::ffi::c_char = skip_value(ds, end);
                    (*o_1).kind = NO_STREAM;
                    pbuf_clear(&raw mut (*o_1).array);
                    let mut r_0: *const ::core::ffi::c_char =
                        ds.offset(1 as ::core::ffi::c_int as isize);
                    while r_0 < de.offset(-(1 as ::core::ffi::c_int as isize)) {
                        if *r_0 as ::core::ffi::c_int == '\\' as i32
                            && r_0.offset(1 as ::core::ffi::c_int as isize)
                                < de.offset(-(1 as ::core::ffi::c_int as isize))
                        {
                            r_0 = r_0.offset(1);
                            let mut e: ::core::ffi::c_char = *r_0;
                            pbuf_putc(
                                &raw mut (*o_1).array,
                                if e as ::core::ffi::c_int == 'n' as i32 {
                                    '\n' as i32
                                } else if e as ::core::ffi::c_int == 'r' as i32 {
                                    '\r' as i32
                                } else if e as ::core::ffi::c_int == 't' as i32 {
                                    '\t' as i32
                                } else {
                                    e as ::core::ffi::c_int
                                },
                            );
                        } else {
                            pbuf_putc(&raw mut (*o_1).array, *r_0 as ::core::ffi::c_int);
                        }
                        r_0 = r_0.offset(1);
                    }
                    let mut dict: *mut ::core::ffi::c_char = subst_names(
                        c,
                        de,
                        end.offset_from(de) as ::core::ffi::c_long as size_t,
                        cx,
                        cy,
                    );
                    merge_dict(&raw mut (*o_1).dict, dict);
                    free(dict as *mut ::core::ffi::c_void);
                }
            } else if cl
                == (::core::mem::size_of::<[::core::ffi::c_char; 6]>() as usize)
                    .wrapping_sub(1 as usize)
                && strncmp(
                    cmd,
                    b"bxobj\0" as *const u8 as *const ::core::ffi::c_char,
                    cl,
                ) == 0
                || cl
                    == (::core::mem::size_of::<[::core::ffi::c_char; 10]>() as usize)
                        .wrapping_sub(1 as usize)
                    && strncmp(
                        cmd,
                        b"beginxobj\0" as *const u8 as *const ::core::ffi::c_char,
                        cl,
                    ) == 0
            {
                if (*c).ntargets >= 8 as ::core::ffi::c_int {
                    return;
                }
                let mut q_3: *const ::core::ffi::c_char = arg;
                let mut ne_2: *const ::core::ffi::c_char = q_3;
                while ne_2 < end
                    && isspace(*ne_2 as ::core::ffi::c_uchar as ::core::ffi::c_int) == 0
                {
                    if b"()<>[]{}/%".contains(&(*ne_2 as u8)) { break; }
                    ne_2 = ne_2.offset(1);
                }
                if q_3 >= end || *q_3 as ::core::ffi::c_int != '@' as i32 {
                    return;
                }
                let mut o_2: *mut named_obj = named_get(
                    c,
                    q_3,
                    ne_2.offset_from(q_3) as ::core::ffi::c_long as size_t,
                    true_0 != 0,
                );
                (*o_2).kind = NO_FORM;
                let mut w: ::core::ffi::c_double = 0 as ::core::ffi::c_int as ::core::ffi::c_double;
                let mut hh: ::core::ffi::c_double =
                    0 as ::core::ffi::c_int as ::core::ffi::c_double;
                let mut d: ::core::ffi::c_double = 0 as ::core::ffi::c_int as ::core::ffi::c_double;
                let mut bbox: [::core::ffi::c_double; 4] = [
                    0 as ::core::ffi::c_int as ::core::ffi::c_double,
                    0 as ::core::ffi::c_int as ::core::ffi::c_double,
                    0 as ::core::ffi::c_int as ::core::ffi::c_double,
                    0 as ::core::ffi::c_int as ::core::ffi::c_double,
                ];
                let mut have_bbox: bool = false_0 != 0;
                let mut r_1: *const ::core::ffi::c_char = ne_2;
                while r_1 < end {
                    r_1 = skip_ws(r_1, end);
                    let mut ok: bool = false;
                    if strncmp(
                        r_1,
                        b"width\0" as *const u8 as *const ::core::ffi::c_char,
                        5 as size_t,
                    ) == 0
                    {
                        r_1 = r_1.offset(5 as ::core::ffi::c_int as isize);
                        w = parse_dimen(&raw mut r_1, end, &raw mut ok);
                    } else if strncmp(
                        r_1,
                        b"height\0" as *const u8 as *const ::core::ffi::c_char,
                        6 as size_t,
                    ) == 0
                    {
                        r_1 = r_1.offset(6 as ::core::ffi::c_int as isize);
                        hh = parse_dimen(&raw mut r_1, end, &raw mut ok);
                    } else if strncmp(
                        r_1,
                        b"depth\0" as *const u8 as *const ::core::ffi::c_char,
                        5 as size_t,
                    ) == 0
                    {
                        r_1 = r_1.offset(5 as ::core::ffi::c_int as isize);
                        d = parse_dimen(&raw mut r_1, end, &raw mut ok);
                    } else if strncmp(
                        r_1,
                        b"bbox\0" as *const u8 as *const ::core::ffi::c_char,
                        4 as size_t,
                    ) == 0
                    {
                        r_1 = r_1.offset(4 as ::core::ffi::c_int as isize);
                        let mut z: *mut ::core::ffi::c_char =
                            ::core::ptr::null_mut::<::core::ffi::c_char>();
                        let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
                        while i < 4 as ::core::ffi::c_int {
                            bbox[i as usize] = strtod(r_1, &raw mut z);
                            r_1 = z;
                            i += 1;
                        }
                        have_bbox = true_0 != 0;
                    } else {
                        while r_1 < end
                            && isspace(*r_1 as ::core::ffi::c_uchar as ::core::ffi::c_int) == 0
                        {
                            r_1 = r_1.offset(1);
                        }
                    }
                }
                if !have_bbox {
                    bbox[0 as ::core::ffi::c_int as usize] =
                        0 as ::core::ffi::c_int as ::core::ffi::c_double;
                    bbox[1 as ::core::ffi::c_int as usize] = -d;
                    bbox[2 as ::core::ffi::c_int as usize] = w;
                    bbox[3 as ::core::ffi::c_int as usize] = hh;
                }
                text_end(c);
                let fresh10 = (*c).ntargets;
                (*c).ntargets = (*c).ntargets + 1;
                let mut t: *mut target =
                    (&raw mut (*c).targets as *mut target).offset(fresh10 as isize) as *mut target;
                memset(
                    t as *mut ::core::ffi::c_void,
                    0 as ::core::ffi::c_int,
                    ::core::mem::size_of::<target>() as size_t,
                );
                // Named objects can grow during form construction; retain an index.
                (*t).form_index = o_2.offset_from((*c).named) as usize + 1;
                memcpy(
                    &raw mut (*t).bbox as *mut ::core::ffi::c_double as *mut ::core::ffi::c_void,
                    &raw mut bbox as *mut ::core::ffi::c_double as *const ::core::ffi::c_void,
                    ::core::mem::size_of::<[::core::ffi::c_double; 4]>() as size_t,
                );
                (*t).ox = cx;
                (*t).oy = cy;
                (*t).q_base = (*c).q_depth;
                (*t).saved_render = (*c).render;
                (*c).render = true_0 != 0;
                push_offset(c, cx, cy);
            } else if cl
                == (::core::mem::size_of::<[::core::ffi::c_char; 6]>() as usize)
                    .wrapping_sub(1 as usize)
                && strncmp(
                    cmd,
                    b"exobj\0" as *const u8 as *const ::core::ffi::c_char,
                    cl,
                ) == 0
                || cl
                    == (::core::mem::size_of::<[::core::ffi::c_char; 8]>() as usize)
                        .wrapping_sub(1 as usize)
                    && strncmp(
                        cmd,
                        b"endxobj\0" as *const u8 as *const ::core::ffi::c_char,
                        cl,
                    ) == 0
            {
                if (*c).ntargets <= 1 as ::core::ffi::c_int {
                    return;
                }
                text_end(c);
                while (*c).q_depth > (*cur(c)).q_base {
                    emit_Q(c);
                }
                pop_offset(c);
                let primitive_form = std::ffi::CStr::from_ptr((*target_form(c,cur(c))).name).to_bytes().starts_with(b"@pitexform");
                let raw_extra = String::from_utf8_lossy(std::slice::from_raw_parts(arg.cast::<u8>(),end.offset_from(arg) as usize));
                let extra_text = if primitive_form { pdf_document::pdftex_references(&raw_extra) } else { raw_extra.into_owned() };
                let mut extra: *mut ::core::ffi::c_char = subst_names(
                    c,
                    extra_text.as_ptr().cast(),
                    extra_text.len(),
                    cx,
                    cy,
                );
                write_form(c, cur(c), extra);
                free(extra as *mut ::core::ffi::c_void);
                pbuf_free(
                    &raw mut (*(cur as unsafe extern "C" fn(*mut conv_ctx) -> *mut target)(c))
                        .content,
                );
                resources_free(
                    &raw mut (*(cur as unsafe extern "C" fn(*mut conv_ctx) -> *mut target)(c)).res,
                );
                (*c).render = (*cur(c)).saved_render;
                (*c).ntargets -= 1;
            } else if cl
                == (::core::mem::size_of::<[::core::ffi::c_char; 6]>() as usize)
                    .wrapping_sub(1 as usize)
                && strncmp(
                    cmd,
                    b"uxobj\0" as *const u8 as *const ::core::ffi::c_char,
                    cl,
                ) == 0
                || cl
                    == (::core::mem::size_of::<[::core::ffi::c_char; 8]>() as usize)
                        .wrapping_sub(1 as usize)
                    && strncmp(
                        cmd,
                        b"usexobj\0" as *const u8 as *const ::core::ffi::c_char,
                        cl,
                    ) == 0
            {
                let mut q_4: *const ::core::ffi::c_char = arg;
                let mut ne_3: *const ::core::ffi::c_char = q_4;
                while ne_3 < end
                    && isspace(*ne_3 as ::core::ffi::c_uchar as ::core::ffi::c_int) == 0
                {
                    if b"()<>[]{}/%".contains(&(*ne_3 as u8)) { break; }
                    ne_3 = ne_3.offset(1);
                }
                let mut o_3: *mut named_obj = named_get(
                    c,
                    q_4,
                    ne_3.offset_from(q_4) as ::core::ffi::c_long as size_t,
                    false_0 != 0,
                );
                if o_3.is_null()
                    || (*o_3).kind as ::core::ffi::c_uint
                        != NO_FORM as ::core::ffi::c_int as ::core::ffi::c_uint
                    || !(*c).render
                {
                    if (*c).render {
                        warn_once(
                            c,
                            b"uxobj\0" as *const u8 as *const ::core::ffi::c_char,
                            b"form XObject used before definition%s\0" as *const u8
                                as *const ::core::ffi::c_char,
                            b"\0" as *const u8 as *const ::core::ffi::c_char,
                        );
                    }
                    return;
                }
                let name_string=pdf_document::Document::form_resource(c,(*o_3).obj);
                let name=std::ffi::CString::new(name_string).unwrap();
                let mut ref_0: [::core::ffi::c_char; 32] = [0; 32];
                snprintf(
                    &raw mut ref_0 as *mut ::core::ffi::c_char,
                    ::core::mem::size_of::<[::core::ffi::c_char; 32]>() as size_t,
                    b"%d 0 R\0" as *const u8 as *const ::core::ffi::c_char,
                    (*o_3).obj,
                );
                kv_set(
                    (&raw mut (*(cur as unsafe extern "C" fn(*mut conv_ctx) -> *mut target)(c))
                        .res
                        .cat as *mut kvlist)
                        .offset(CAT_XOBJECT as ::core::ffi::c_int as isize)
                        as *mut kvlist,
                    name.as_ptr(),
                    strlen(name.as_ptr()),
                    &raw mut ref_0 as *mut ::core::ffi::c_char,
                    strlen(&raw mut ref_0 as *mut ::core::ffi::c_char),
                );
                emit_q(c);
                emit_cm(c, mat_translate(cx, cy));
                if let Some(bbox)=(*(*c).document).form_boxes.get(&(*o_3).obj).copied(){
                    let rectangle=pdf_document::Document::rectangle(c,bbox);
                    pdf_document::Document::touch_rectangle(c,rectangle);
                }
                pbuf_printf(
                    &raw mut (*(cur as unsafe extern "C" fn(*mut conv_ctx) -> *mut target)(c))
                        .content,
                    b"/%s Do\n\0" as *const u8 as *const ::core::ffi::c_char,
                    name.as_ptr(),
                );
                emit_Q(c);
            } else if !(cl
                == (::core::mem::size_of::<[::core::ffi::c_char; 4]>() as usize)
                    .wrapping_sub(1 as usize)
                && strncmp(cmd, b"ann\0" as *const u8 as *const ::core::ffi::c_char, cl) == 0
                || cl
                    == (::core::mem::size_of::<[::core::ffi::c_char; 5]>() as usize)
                        .wrapping_sub(1 as usize)
                    && strncmp(
                        cmd,
                        b"bann\0" as *const u8 as *const ::core::ffi::c_char,
                        cl,
                    ) == 0
                || cl
                    == (::core::mem::size_of::<[::core::ffi::c_char; 5]>() as usize)
                        .wrapping_sub(1 as usize)
                    && strncmp(
                        cmd,
                        b"eann\0" as *const u8 as *const ::core::ffi::c_char,
                        cl,
                    ) == 0
                || cl
                    == (::core::mem::size_of::<[::core::ffi::c_char; 5]>() as usize)
                        .wrapping_sub(1 as usize)
                    && strncmp(
                        cmd,
                        b"dest\0" as *const u8 as *const ::core::ffi::c_char,
                        cl,
                    ) == 0
                || cl
                    == (::core::mem::size_of::<[::core::ffi::c_char; 8]>() as usize)
                        .wrapping_sub(1 as usize)
                    && strncmp(
                        cmd,
                        b"outline\0" as *const u8 as *const ::core::ffi::c_char,
                        cl,
                    ) == 0
                || cl
                    == (::core::mem::size_of::<[::core::ffi::c_char; 4]>() as usize)
                        .wrapping_sub(1 as usize)
                    && strncmp(cmd, b"out\0" as *const u8 as *const ::core::ffi::c_char, cl) == 0
                || cl
                    == (::core::mem::size_of::<[::core::ffi::c_char; 8]>() as usize)
                        .wrapping_sub(1 as usize)
                    && strncmp(
                        cmd,
                        b"docinfo\0" as *const u8 as *const ::core::ffi::c_char,
                        cl,
                    ) == 0
                || cl
                    == (::core::mem::size_of::<[::core::ffi::c_char; 8]>() as usize)
                        .wrapping_sub(1 as usize)
                    && strncmp(
                        cmd,
                        b"docview\0" as *const u8 as *const ::core::ffi::c_char,
                        cl,
                    ) == 0
                || cl
                    == (::core::mem::size_of::<[::core::ffi::c_char; 6]>() as usize)
                        .wrapping_sub(1 as usize)
                    && strncmp(
                        cmd,
                        b"close\0" as *const u8 as *const ::core::ffi::c_char,
                        cl,
                    ) == 0
                || cl
                    == (::core::mem::size_of::<[::core::ffi::c_char; 6]>() as usize)
                        .wrapping_sub(1 as usize)
                    && strncmp(
                        cmd,
                        b"names\0" as *const u8 as *const ::core::ffi::c_char,
                        cl,
                    ) == 0
                || cl
                    == (::core::mem::size_of::<[::core::ffi::c_char; 10]>() as usize)
                        .wrapping_sub(1 as usize)
                    && strncmp(
                        cmd,
                        b"tounicode\0" as *const u8 as *const ::core::ffi::c_char,
                        cl,
                    ) == 0
                || cl
                    == (::core::mem::size_of::<[::core::ffi::c_char; 8]>() as usize)
                        .wrapping_sub(1 as usize)
                    && strncmp(
                        cmd,
                        b"mapline\0" as *const u8 as *const ::core::ffi::c_char,
                        cl,
                    ) == 0
                || cl
                    == (::core::mem::size_of::<[::core::ffi::c_char; 8]>() as usize)
                        .wrapping_sub(1 as usize)
                    && strncmp(
                        cmd,
                        b"mapfile\0" as *const u8 as *const ::core::ffi::c_char,
                        cl,
                    ) == 0
                || cl
                    == (::core::mem::size_of::<[::core::ffi::c_char; 12]>() as usize)
                        .wrapping_sub(1 as usize)
                    && strncmp(
                        cmd,
                        b"fontmapline\0" as *const u8 as *const ::core::ffi::c_char,
                        cl,
                    ) == 0
                || cl
                    == (::core::mem::size_of::<[::core::ffi::c_char; 12]>() as usize)
                        .wrapping_sub(1 as usize)
                    && strncmp(
                        cmd,
                        b"fontmapfile\0" as *const u8 as *const ::core::ffi::c_char,
                        cl,
                    ) == 0
                || cl
                    == (::core::mem::size_of::<[::core::ffi::c_char; 13]>() as usize)
                        .wrapping_sub(1 as usize)
                    && strncmp(
                        cmd,
                        b"minorversion\0" as *const u8 as *const ::core::ffi::c_char,
                        cl,
                    ) == 0
                || cl
                    == (::core::mem::size_of::<[::core::ffi::c_char; 13]>() as usize)
                        .wrapping_sub(1 as usize)
                    && strncmp(
                        cmd,
                        b"majorversion\0" as *const u8 as *const ::core::ffi::c_char,
                        cl,
                    ) == 0
                || cl
                    == (::core::mem::size_of::<[::core::ffi::c_char; 8]>() as usize)
                        .wrapping_sub(1 as usize)
                    && strncmp(
                        cmd,
                        b"encrypt\0" as *const u8 as *const ::core::ffi::c_char,
                        cl,
                    ) == 0
                || cl
                    == (::core::mem::size_of::<[::core::ffi::c_char; 5]>() as usize)
                        .wrapping_sub(1 as usize)
                    && strncmp(
                        cmd,
                        b"link\0" as *const u8 as *const ::core::ffi::c_char,
                        cl,
                    ) == 0
                || cl
                    == (::core::mem::size_of::<[::core::ffi::c_char; 7]>() as usize)
                        .wrapping_sub(1 as usize)
                    && strncmp(
                        cmd,
                        b"nolink\0" as *const u8 as *const ::core::ffi::c_char,
                        cl,
                    ) == 0
                || cl
                    == (::core::mem::size_of::<[::core::ffi::c_char; 7]>() as usize)
                        .wrapping_sub(1 as usize)
                    && strncmp(
                        cmd,
                        b"thread\0" as *const u8 as *const ::core::ffi::c_char,
                        cl,
                    ) == 0
                || cl
                    == (::core::mem::size_of::<[::core::ffi::c_char; 8]>() as usize)
                        .wrapping_sub(1 as usize)
                    && strncmp(
                        cmd,
                        b"article\0" as *const u8 as *const ::core::ffi::c_char,
                        cl,
                    ) == 0
                || cl
                    == (::core::mem::size_of::<[::core::ffi::c_char; 5]>() as usize)
                        .wrapping_sub(1 as usize)
                    && strncmp(
                        cmd,
                        b"bead\0" as *const u8 as *const ::core::ffi::c_char,
                        cl,
                    ) == 0
                || cl
                    == (::core::mem::size_of::<[::core::ffi::c_char; 4]>() as usize)
                        .wrapping_sub(1 as usize)
                    && strncmp(cmd, b"bop\0" as *const u8 as *const ::core::ffi::c_char, cl) == 0
                || cl
                    == (::core::mem::size_of::<[::core::ffi::c_char; 4]>() as usize)
                        .wrapping_sub(1 as usize)
                    && strncmp(cmd, b"eop\0" as *const u8 as *const ::core::ffi::c_char, cl) == 0
                || cl
                    == (::core::mem::size_of::<[::core::ffi::c_char; 5]>() as usize)
                        .wrapping_sub(1 as usize)
                    && strncmp(
                        cmd,
                        b"font\0" as *const u8 as *const ::core::ffi::c_char,
                        cl,
                    ) == 0
                || cl
                    == (::core::mem::size_of::<[::core::ffi::c_char; 14]>() as usize)
                        .wrapping_sub(1 as usize)
                    && strncmp(
                        cmd,
                        b"pageresources\0" as *const u8 as *const ::core::ffi::c_char,
                        cl,
                    ) == 0
                || cl
                    == (::core::mem::size_of::<[::core::ffi::c_char; 11]>() as usize)
                        .wrapping_sub(1 as usize)
                    && strncmp(
                        cmd,
                        b"backupfont\0" as *const u8 as *const ::core::ffi::c_char,
                        cl,
                    ) == 0
                || cl
                    == (::core::mem::size_of::<[::core::ffi::c_char; 13]>() as usize)
                        .wrapping_sub(1 as usize)
                    && strncmp(
                        cmd,
                        b"setfillcolor\0" as *const u8 as *const ::core::ffi::c_char,
                        cl,
                    ) == 0
                || cl
                    == (::core::mem::size_of::<[::core::ffi::c_char; 15]>() as usize)
                        .wrapping_sub(1 as usize)
                    && strncmp(
                        cmd,
                        b"setstrokecolor\0" as *const u8 as *const ::core::ffi::c_char,
                        cl,
                    ) == 0)
            {
                if (*c).render {
                    let mut tmp: [::core::ffi::c_char; 48] = [0; 48];
                    snprintf(
                        &raw mut tmp as *mut ::core::ffi::c_char,
                        ::core::mem::size_of::<[::core::ffi::c_char; 48]>() as size_t,
                        b"%.*s\0" as *const u8 as *const ::core::ffi::c_char,
                        (if cl < 40 as size_t { cl } else { 40 as size_t }) as ::core::ffi::c_int,
                        cmd,
                    );
                    warn_once(
                        c,
                        &raw mut tmp as *mut ::core::ffi::c_char,
                        b"unsupported special ignored: %s\0" as *const u8
                            as *const ::core::ffi::c_char,
                        &raw mut tmp as *mut ::core::ffi::c_char,
                    );
                }
            }
        }
    } else if !(end.offset_from(s) as ::core::ffi::c_long as size_t
        >= (::core::mem::size_of::<[::core::ffi::c_char; 11]>() as usize).wrapping_sub(1 as usize)
        && strncmp(
            s,
            b"papersize=\0" as *const u8 as *const ::core::ffi::c_char,
            (::core::mem::size_of::<[::core::ffi::c_char; 11]>() as size_t)
                .wrapping_sub(1 as size_t),
        ) == 0
        || end.offset_from(s) as ::core::ffi::c_long as size_t
            >= (::core::mem::size_of::<[::core::ffi::c_char; 5]>() as usize)
                .wrapping_sub(1 as usize)
            && strncmp(
                s,
                b"src:\0" as *const u8 as *const ::core::ffi::c_char,
                (::core::mem::size_of::<[::core::ffi::c_char; 5]>() as size_t)
                    .wrapping_sub(1 as size_t),
            ) == 0
        || end.offset_from(s) as ::core::ffi::c_long as size_t
            >= (::core::mem::size_of::<[::core::ffi::c_char; 8]>() as usize)
                .wrapping_sub(1 as usize)
            && strncmp(
                s,
                b"header=\0" as *const u8 as *const ::core::ffi::c_char,
                (::core::mem::size_of::<[::core::ffi::c_char; 8]>() as size_t)
                    .wrapping_sub(1 as size_t),
            ) == 0
        || end.offset_from(s) as ::core::ffi::c_long as size_t
            >= (::core::mem::size_of::<[::core::ffi::c_char; 10]>() as usize)
                .wrapping_sub(1 as usize)
            && strncmp(
                s,
                b"landscape\0" as *const u8 as *const ::core::ffi::c_char,
                (::core::mem::size_of::<[::core::ffi::c_char; 10]>() as size_t)
                    .wrapping_sub(1 as size_t),
            ) == 0
        || end.offset_from(s) as ::core::ffi::c_long as size_t
            >= (::core::mem::size_of::<[::core::ffi::c_char; 2]>() as usize)
                .wrapping_sub(1 as usize)
            && strncmp(
                s,
                b"!\0" as *const u8 as *const ::core::ffi::c_char,
                (::core::mem::size_of::<[::core::ffi::c_char; 2]>() as size_t)
                    .wrapping_sub(1 as size_t),
            ) == 0
        || end.offset_from(s) as ::core::ffi::c_long as size_t
            >= (::core::mem::size_of::<[::core::ffi::c_char; 4]>() as usize)
                .wrapping_sub(1 as usize)
            && strncmp(
                s,
                b"em:\0" as *const u8 as *const ::core::ffi::c_char,
                (::core::mem::size_of::<[::core::ffi::c_char; 4]>() as size_t)
                    .wrapping_sub(1 as size_t),
            ) == 0
        || end.offset_from(s) as ::core::ffi::c_long as size_t
            >= (::core::mem::size_of::<[::core::ffi::c_char; 6]>() as usize)
                .wrapping_sub(1 as usize)
            && strncmp(
                s,
                b"line:\0" as *const u8 as *const ::core::ffi::c_char,
                (::core::mem::size_of::<[::core::ffi::c_char; 6]>() as size_t)
                    .wrapping_sub(1 as size_t),
            ) == 0
        || end.offset_from(s) as ::core::ffi::c_long as size_t
            >= (::core::mem::size_of::<[::core::ffi::c_char; 6]>() as usize)
                .wrapping_sub(1 as usize)
            && strncmp(
                s,
                b"html:\0" as *const u8 as *const ::core::ffi::c_char,
                (::core::mem::size_of::<[::core::ffi::c_char; 6]>() as size_t)
                    .wrapping_sub(1 as size_t),
            ) == 0
        || end.offset_from(s) as ::core::ffi::c_long as size_t
            >= (::core::mem::size_of::<[::core::ffi::c_char; 10]>() as usize)
                .wrapping_sub(1 as usize)
            && strncmp(
                s,
                b"dvipdfmx:\0" as *const u8 as *const ::core::ffi::c_char,
                (::core::mem::size_of::<[::core::ffi::c_char; 10]>() as size_t)
                    .wrapping_sub(1 as size_t),
            ) == 0
        || end.offset_from(s) as ::core::ffi::c_long as size_t
            >= (::core::mem::size_of::<[::core::ffi::c_char; 7]>() as usize)
                .wrapping_sub(1 as usize)
            && strncmp(
                s,
                b"dvips:\0" as *const u8 as *const ::core::ffi::c_char,
                (::core::mem::size_of::<[::core::ffi::c_char; 7]>() as size_t)
                    .wrapping_sub(1 as size_t),
            ) == 0)
    {
        if end.offset_from(s) as ::core::ffi::c_long as size_t
            >= (::core::mem::size_of::<[::core::ffi::c_char; 4]>() as usize)
                .wrapping_sub(1 as usize)
            && strncmp(
                s,
                b"ps:\0" as *const u8 as *const ::core::ffi::c_char,
                (::core::mem::size_of::<[::core::ffi::c_char; 4]>() as size_t)
                    .wrapping_sub(1 as size_t),
            ) == 0
            || end.offset_from(s) as ::core::ffi::c_long as size_t
                >= (::core::mem::size_of::<[::core::ffi::c_char; 7]>() as usize)
                    .wrapping_sub(1 as usize)
                && strncmp(
                    s,
                    b"PSfile\0" as *const u8 as *const ::core::ffi::c_char,
                    (::core::mem::size_of::<[::core::ffi::c_char; 7]>() as size_t)
                        .wrapping_sub(1 as size_t),
                ) == 0
            || end.offset_from(s) as ::core::ffi::c_long as size_t
                >= (::core::mem::size_of::<[::core::ffi::c_char; 3]>() as usize)
                    .wrapping_sub(1 as usize)
                && strncmp(
                    s,
                    b"\" \0" as *const u8 as *const ::core::ffi::c_char,
                    (::core::mem::size_of::<[::core::ffi::c_char; 3]>() as size_t)
                        .wrapping_sub(1 as size_t),
                ) == 0
            || end.offset_from(s) as ::core::ffi::c_long as size_t
                >= (::core::mem::size_of::<[::core::ffi::c_char; 2]>() as usize)
                    .wrapping_sub(1 as usize)
                && strncmp(
                    s,
                    b"\"\0" as *const u8 as *const ::core::ffi::c_char,
                    (::core::mem::size_of::<[::core::ffi::c_char; 2]>() as size_t)
                        .wrapping_sub(1 as size_t),
                ) == 0
            || end.offset_from(s) as ::core::ffi::c_long as size_t
                >= (::core::mem::size_of::<[::core::ffi::c_char; 7]>() as usize)
                    .wrapping_sub(1 as usize)
                && strncmp(
                    s,
                    b"psfile\0" as *const u8 as *const ::core::ffi::c_char,
                    (::core::mem::size_of::<[::core::ffi::c_char; 7]>() as size_t)
                        .wrapping_sub(1 as size_t),
                ) == 0
        {
            if (*c).render {
                warn_once(
                    c,
                    b"postscript\0" as *const u8 as *const ::core::ffi::c_char,
                    b"PostScript specials cannot be previewed%s\0" as *const u8
                        as *const ::core::ffi::c_char,
                    b"\0" as *const u8 as *const ::core::ffi::c_char,
                );
            }
        }
    }
}
unsafe extern "C" fn run_packet(
    mut c: *mut conv_ctx,
    mut vfslot: *mut font_slot,
    mut p: *const ::core::ffi::c_uchar,
    mut len: size_t,
    mut h: ::core::ffi::c_double,
    mut v: ::core::ffi::c_double,
    mut depth: ::core::ffi::c_int,
) {
    let mut st: *mut dvi_state =
        calloc(1 as size_t, ::core::mem::size_of::<dvi_state>() as size_t) as *mut dvi_state;
    if st.is_null() {
        abort();
    }
    (*st).fonts = (*vfslot).vf_local;
    (*st).font = if !(*vfslot).vf_local.is_null() && (*(*vfslot).vf_local).n != 0 {
        *(*(*vfslot).vf_local)
            .slots
            .offset(0 as ::core::ffi::c_int as isize)
    } else {
        ::core::ptr::null_mut::<font_slot>()
    };
    (*st).scale = (*vfslot).size / 1048576.0f64;
    (*st).r.h = h;
    (*st).r.v = v;
    interpret(c, st, p, p.offset(len as isize), depth);
    free(st as *mut ::core::ffi::c_void);
}
unsafe extern "C" fn interpret(
    mut c: *mut conv_ctx,
    mut st: *mut dvi_state,
    mut p: *const ::core::ffi::c_uchar,
    mut end: *const ::core::ffi::c_uchar,
    mut depth: ::core::ffi::c_int,
) {
    while p < end {
        let mut n: ::core::ffi::c_long = xdv_insn_length(p, end);
        if n <= 0 as ::core::ffi::c_long {
            break;
        }
        let mut op: ::core::ffi::c_int =
            *p.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int;
        let mut sc: ::core::ffi::c_double = (*st).scale;
        if op < 128 as ::core::ffi::c_int
            || op >= DVI_SET1 as ::core::ffi::c_int
                && op < DVI_SET1 as ::core::ffi::c_int + 4 as ::core::ffi::c_int
            || op >= DVI_PUT1 as ::core::ffi::c_int
                && op < DVI_PUT1 as ::core::ffi::c_int + 4 as ::core::ffi::c_int
        {
            let mut code: ::core::ffi::c_uint = 0;
            let mut set: bool = true_0 != 0;
            if op < 128 as ::core::ffi::c_int {
                code = op as ::core::ffi::c_uint;
            } else if op < DVI_SET_RULE as ::core::ffi::c_int {
                code = be(
                    p.offset(1 as ::core::ffi::c_int as isize),
                    op - DVI_SET1 as ::core::ffi::c_int + 1 as ::core::ffi::c_int,
                ) as ::core::ffi::c_uint;
            } else {
                code = be(
                    p.offset(1 as ::core::ffi::c_int as isize),
                    op - DVI_PUT1 as ::core::ffi::c_int + 1 as ::core::ffi::c_int,
                ) as ::core::ffi::c_uint;
                set = false_0 != 0;
            }
            if !(*st).font.is_null() {
                draw_char(c, (*st).font, code, (*st).r.h, (*st).r.v, depth);
                if set {
                    (*st).r.h += char_width((*st).font, code);
                }
            }
        } else if op == DVI_SET_RULE as ::core::ffi::c_int
            || op == DVI_PUT_RULE as ::core::ffi::c_int
        {
            let mut a: ::core::ffi::c_double = sbe(
                p.offset(1 as ::core::ffi::c_int as isize),
                4 as ::core::ffi::c_int,
            ) as ::core::ffi::c_double
                * sc;
            let mut b: ::core::ffi::c_double = sbe(
                p.offset(5 as ::core::ffi::c_int as isize),
                4 as ::core::ffi::c_int,
            ) as ::core::ffi::c_double
                * sc;
            emit_rule(c, (*st).r.h, (*st).r.v, b, a);
            if op == DVI_SET_RULE as ::core::ffi::c_int {
                (*st).r.h += b;
            }
        } else if op == DVI_PUSH as ::core::ffi::c_int {
            if (*st).sp < MAX_STACK {
                let fresh6 = (*st).sp;
                (*st).sp = (*st).sp + 1;
                (*st).stack[fresh6 as usize] = (*st).r;
            }
        } else if op == DVI_POP as ::core::ffi::c_int {
            if (*st).sp > 0 as ::core::ffi::c_int {
                (*st).sp -= 1;
                (*st).r = (*st).stack[(*st).sp as usize];
            }
        } else if op >= DVI_RIGHT1 as ::core::ffi::c_int
            && op < DVI_RIGHT1 as ::core::ffi::c_int + 4 as ::core::ffi::c_int
        {
            (*st).r.h += sbe(
                p.offset(1 as ::core::ffi::c_int as isize),
                op - DVI_RIGHT1 as ::core::ffi::c_int + 1 as ::core::ffi::c_int,
            ) as ::core::ffi::c_double
                * sc;
        } else if op == DVI_W0 as ::core::ffi::c_int {
            (*st).r.h += (*st).r.w;
        } else if op >= DVI_W1 as ::core::ffi::c_int
            && op < DVI_W1 as ::core::ffi::c_int + 4 as ::core::ffi::c_int
        {
            (*st).r.w = sbe(
                p.offset(1 as ::core::ffi::c_int as isize),
                op - DVI_W1 as ::core::ffi::c_int + 1 as ::core::ffi::c_int,
            ) as ::core::ffi::c_double
                * sc;
            (*st).r.h += (*st).r.w;
        } else if op == DVI_X0 as ::core::ffi::c_int {
            (*st).r.h += (*st).r.x;
        } else if op >= DVI_X1 as ::core::ffi::c_int
            && op < DVI_X1 as ::core::ffi::c_int + 4 as ::core::ffi::c_int
        {
            (*st).r.x = sbe(
                p.offset(1 as ::core::ffi::c_int as isize),
                op - DVI_X1 as ::core::ffi::c_int + 1 as ::core::ffi::c_int,
            ) as ::core::ffi::c_double
                * sc;
            (*st).r.h += (*st).r.x;
        } else if op >= DVI_DOWN1 as ::core::ffi::c_int
            && op < DVI_DOWN1 as ::core::ffi::c_int + 4 as ::core::ffi::c_int
        {
            (*st).r.v += sbe(
                p.offset(1 as ::core::ffi::c_int as isize),
                op - DVI_DOWN1 as ::core::ffi::c_int + 1 as ::core::ffi::c_int,
            ) as ::core::ffi::c_double
                * sc;
        } else if op == DVI_Y0 as ::core::ffi::c_int {
            (*st).r.v += (*st).r.y;
        } else if op >= DVI_Y1 as ::core::ffi::c_int
            && op < DVI_Y1 as ::core::ffi::c_int + 4 as ::core::ffi::c_int
        {
            (*st).r.y = sbe(
                p.offset(1 as ::core::ffi::c_int as isize),
                op - DVI_Y1 as ::core::ffi::c_int + 1 as ::core::ffi::c_int,
            ) as ::core::ffi::c_double
                * sc;
            (*st).r.v += (*st).r.y;
        } else if op == DVI_Z0 as ::core::ffi::c_int {
            (*st).r.v += (*st).r.z;
        } else if op >= DVI_Z1 as ::core::ffi::c_int
            && op < DVI_Z1 as ::core::ffi::c_int + 4 as ::core::ffi::c_int
        {
            (*st).r.z = sbe(
                p.offset(1 as ::core::ffi::c_int as isize),
                op - DVI_Z1 as ::core::ffi::c_int + 1 as ::core::ffi::c_int,
            ) as ::core::ffi::c_double
                * sc;
            (*st).r.v += (*st).r.z;
        } else if op >= DVI_FNT_NUM_0 as ::core::ffi::c_int && op < DVI_FNT1 as ::core::ffi::c_int
            || op >= DVI_FNT1 as ::core::ffi::c_int
                && op < DVI_FNT1 as ::core::ffi::c_int + 4 as ::core::ffi::c_int
        {
            let mut k: int32_t = if op < DVI_FNT1 as ::core::ffi::c_int {
                op as int32_t - DVI_FNT_NUM_0 as ::core::ffi::c_int as int32_t
            } else {
                sbe(
                    p.offset(1 as ::core::ffi::c_int as isize),
                    op - DVI_FNT1 as ::core::ffi::c_int + 1 as ::core::ffi::c_int,
                )
            };
            (*st).font = table_find((*st).fonts, k);
        } else if op >= DVI_XXX1 as ::core::ffi::c_int
            && op < DVI_XXX1 as ::core::ffi::c_int + 4 as ::core::ffi::c_int
        {
            let mut k_0: ::core::ffi::c_int =
                op - DVI_XXX1 as ::core::ffi::c_int + 1 as ::core::ffi::c_int;
            do_special_bounded(
                c,
                (p as *const ::core::ffi::c_char)
                    .offset(1 as ::core::ffi::c_int as isize)
                    .offset(k_0 as isize),
                (n - 1 as ::core::ffi::c_long - k_0 as ::core::ffi::c_long) as size_t,
                (*st).r.h,
                (*st).r.v,
            );
        } else if op >= DVI_FNT_DEF1 as ::core::ffi::c_int
            && op < DVI_FNT_DEF1 as ::core::ffi::c_int + 4 as ::core::ffi::c_int
        {
            if depth == 0 as ::core::ffi::c_int {
                define_tfm_font(
                    c,
                    (*st).fonts,
                    p,
                    op - DVI_FNT_DEF1 as ::core::ffi::c_int + 1 as ::core::ffi::c_int,
                );
            }
        } else if op == XDV_NATIVE_FONT_DEF as ::core::ffi::c_int {
            if depth == 0 as ::core::ffi::c_int {
                define_native_font(c, (*st).fonts, p);
            }
        } else if op == XDV_GLYPHS as ::core::ffi::c_int
            || op == XDV_TEXT_AND_GLYPHS as ::core::ffi::c_int
        {
            let mut q: *const ::core::ffi::c_uchar = p.offset(1 as ::core::ffi::c_int as isize);
            let mut text_len: ::core::ffi::c_uint =
                if op == XDV_TEXT_AND_GLYPHS as ::core::ffi::c_int {
                    be(
                        p.offset(1 as ::core::ffi::c_int as isize),
                        2 as ::core::ffi::c_int,
                    ) as ::core::ffi::c_uint
                } else {
                    0 as ::core::ffi::c_uint
                };
            if op == XDV_TEXT_AND_GLYPHS as ::core::ffi::c_int {
                q = q.offset(
                    (2 as ::core::ffi::c_uint)
                        .wrapping_add((2 as ::core::ffi::c_uint).wrapping_mul(text_len))
                        as isize,
                );
            }
            let mut w: int32_t = sbe(q, 4 as ::core::ffi::c_int);
            let mut cnt: ::core::ffi::c_int = be(
                q.offset(4 as ::core::ffi::c_int as isize),
                2 as ::core::ffi::c_int,
            ) as ::core::ffi::c_int;
            let mut actual_text: bool = text_len != 0
                && (*c).render as ::core::ffi::c_int != 0
                && !(*st).font.is_null()
                && (*(*st).font).kind as ::core::ffi::c_uint
                    == FS_NATIVE as ::core::ffi::c_int as ::core::ffi::c_uint;
            if actual_text {
                text_end(c);
                pbuf_puts(
                    &raw mut (*(cur as unsafe extern "C" fn(*mut conv_ctx) -> *mut target)(c))
                        .content,
                    b"/Span<</ActualText<FEFF\0" as *const u8 as *const ::core::ffi::c_char,
                );
                let mut i: ::core::ffi::c_uint = 0 as ::core::ffi::c_uint;
                while i < (2 as ::core::ffi::c_uint).wrapping_mul(text_len) {
                    pbuf_printf(
                        &raw mut (*(cur as unsafe extern "C" fn(*mut conv_ctx) -> *mut target)(c))
                            .content,
                        b"%02X\0" as *const u8 as *const ::core::ffi::c_char,
                        *p.offset((3 as ::core::ffi::c_uint).wrapping_add(i) as isize)
                            as ::core::ffi::c_int,
                    );
                    i = i.wrapping_add(1);
                }
                pbuf_puts(
                    &raw mut (*(cur as unsafe extern "C" fn(*mut conv_ctx) -> *mut target)(c))
                        .content,
                    b">>>BDC\n\0" as *const u8 as *const ::core::ffi::c_char,
                );
            }
            if !(*st).font.is_null() {
                draw_native_glyphs(
                    c,
                    (*st).font,
                    (*st).r.h,
                    (*st).r.v,
                    q.offset(6 as ::core::ffi::c_int as isize),
                    q.offset(6 as ::core::ffi::c_int as isize)
                        .offset((8 as ::core::ffi::c_int * cnt) as isize),
                    cnt,
                );
            }
            if actual_text {
                emit_raw(
                    c,
                    b"EMC\0" as *const u8 as *const ::core::ffi::c_char,
                    3 as size_t,
                );
            }
            (*st).r.h += w as ::core::ffi::c_double;
        }
        p = p.offset(n as isize);
    }
}
unsafe extern "C" fn write_resources_body(
    mut c: *mut conv_ctx,
    mut b: *mut pbuf,
    mut r: *mut resources,
) {
    pbuf_printf(
        b,
        b"/Font %d 0 R/XObject %d 0 R\0" as *const u8 as *const ::core::ffi::c_char,
        (*c).font_dict_obj,
        (*c).xobject_dict_obj,
    );
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < NCAT as ::core::ffi::c_int {
        if !(i == CAT_FONT as ::core::ffi::c_int || i == CAT_XOBJECT as ::core::ffi::c_int) {
            if !(*r).cat_ref[i as usize].is_null() {
                pbuf_printf(
                    b,
                    b"/%s %s\0" as *const u8 as *const ::core::ffi::c_char,
                    cat_names[i as usize],
                    (*r).cat_ref[i as usize],
                );
            } else if (*r).cat[i as usize].n != 0 {
                pbuf_printf(
                    b,
                    b"/%s<<\0" as *const u8 as *const ::core::ffi::c_char,
                    cat_names[i as usize],
                );
                let mut j: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
                while j < (*r).cat[i as usize].n {
                    pbuf_printf(
                        b,
                        b"/%s %s\0" as *const u8 as *const ::core::ffi::c_char,
                        (*(*r).cat[i as usize].items.offset(j as isize)).key,
                        (*(*r).cat[i as usize].items.offset(j as isize)).value,
                    );
                    j += 1;
                }
                pbuf_puts(b, b">>\0" as *const u8 as *const ::core::ffi::c_char);
            }
        }
        i += 1;
    }
}
unsafe fn target_form(c: *mut conv_ctx, t: *mut target) -> *mut named_obj {
    let index = (*t).form_index;
    if index == 0 || index > (*c).nnamed as usize { return std::ptr::null_mut(); }
    (*c).named.add(index - 1)
}
unsafe extern "C" fn write_form(
    mut c: *mut conv_ctx,
    mut t: *mut target,
    mut extra: *const ::core::ffi::c_char,
) {
    let form = target_form(c,t);
    if form.is_null() {
        return;
    }
    (*(*c).document).form_boxes.insert((*form).obj,(*t).bbox);
    let mut j: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while j < (*t).res.cat[CAT_XOBJECT as ::core::ffi::c_int as usize].n {
        kv_set(
            (&raw mut (*c).page_res.cat as *mut kvlist)
                .offset(CAT_XOBJECT as ::core::ffi::c_int as isize) as *mut kvlist,
            (*(*t).res.cat[CAT_XOBJECT as ::core::ffi::c_int as usize]
                .items
                .offset(j as isize))
            .key,
            strlen(
                (*(*t).res.cat[CAT_XOBJECT as ::core::ffi::c_int as usize]
                    .items
                    .offset(j as isize))
                .key,
            ),
            (*(*t).res.cat[CAT_XOBJECT as ::core::ffi::c_int as usize]
                .items
                .offset(j as isize))
            .value,
            strlen(
                (*(*t).res.cat[CAT_XOBJECT as ::core::ffi::c_int as usize]
                    .items
                    .offset(j as isize))
                .value,
            ),
        );
        j += 1;
    }
    let mut d: pbuf = pbuf {
        data: ::core::ptr::null_mut::<::core::ffi::c_uchar>(),
        len: 0,
        cap: 0,
    };
    pbuf_puts(
        &raw mut d,
        b"/Type/XObject/Subtype/Form/BBox[\0" as *const u8 as *const ::core::ffi::c_char,
    );
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < 4 as ::core::ffi::c_int {
        pbuf_real(&raw mut d, (*t).bbox[i as usize]);
        pbuf_putc(
            &raw mut d,
            if i < 3 as ::core::ffi::c_int {
                ' ' as i32
            } else {
                ']' as i32
            },
        );
        i += 1;
    }
    pbuf_puts(
        &raw mut d,
        b"/Resources<<\0" as *const u8 as *const ::core::ffi::c_char,
    );
    write_resources_body(c, &raw mut d, &raw mut (*t).res);
    pbuf_puts(
        &raw mut d,
        b">>\0" as *const u8 as *const ::core::ffi::c_char,
    );
    let mut ex: kvlist = kvlist {
        items: ::core::ptr::null_mut::<kv>(),
        n: 0,
        cap: 0,
    };
    if !extra.is_null() {
        merge_dict(&raw mut ex, extra);
    }
    let mut i_0: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i_0 < ex.n {
        pbuf_printf(
            &raw mut d,
            b"/%s %s\0" as *const u8 as *const ::core::ffi::c_char,
            (*ex.items.offset(i_0 as isize)).key,
            (*ex.items.offset(i_0 as isize)).value,
        );
        i_0 += 1;
    }
    kv_free(&raw mut ex);
    pbuf_putc(&raw mut d, 0 as ::core::ffi::c_int);
    pdfw_stream(
        (*c).pw,
        (*form).obj,
        d.data as *mut ::core::ffi::c_char,
        (*t).content.data as *const ::core::ffi::c_void,
        (*t).content.len,
        true_0 != 0,
    );
    (*form).written = true_0 != 0;
    pbuf_free(&raw mut d);
}
unsafe extern "C" fn write_native_font(mut c: *mut conv_ctx, mut p: *mut pdf_font) {
    let mut f: *mut native_face = (*p).nf;
    if (*f).deflated.len == 0 {
        pbuf_deflate(
            &raw mut (*f).deflated,
            (*f).sfnt,
            (*f).sfnt_len,
            6 as ::core::ffi::c_int,
        );
    }
    let mut desc: ::core::ffi::c_int = pdfw_alloc((*c).pw);
    let mut fd: ::core::ffi::c_int = pdfw_alloc((*c).pw);
    let mut ff: ::core::ffi::c_int = pdfw_alloc((*c).pw);
    let mut tu: ::core::ffi::c_int = if font_output::allow_unicode(c,p) {pdfw_alloc((*c).pw)} else {0};
    let mut o: *mut pbuf = pdfw_out((*c).pw);
    pdfw_begin((*c).pw, (*p).obj);
    pbuf_puts(
        o,
        b"<</Type/Font/Subtype/Type0/BaseFont\0" as *const u8 as *const ::core::ffi::c_char,
    );
    pdfw_name(o, &raw mut (*f).psname as *mut ::core::ffi::c_char);
    pbuf_printf(
        o,
        b"/Encoding/%s/DescendantFonts[%d 0 R]\0".as_ptr().cast(),
        if (*p).vertical { b"Identity-V\0".as_ptr().cast::<::core::ffi::c_char>() } else { b"Identity-H\0".as_ptr().cast() },
        desc,
    );
    if tu>0 {pbuf_printf(o,b"/ToUnicode %d 0 R\0".as_ptr().cast(),tu);}
    font_output::attributes(c,p,o);pbuf_puts(o,b">>\0".as_ptr().cast());
    pdfw_end((*c).pw);
    pdfw_begin((*c).pw, desc);
    pbuf_printf(
        o,
        b"<</Type/Font/Subtype/%s/BaseFont\0" as *const u8 as *const ::core::ffi::c_char,
        if (*f).cff as ::core::ffi::c_int != 0 {
            b"CIDFontType0\0" as *const u8 as *const ::core::ffi::c_char
        } else {
            b"CIDFontType2\0" as *const u8 as *const ::core::ffi::c_char
        },
    );
    pdfw_name(o, &raw mut (*f).psname as *mut ::core::ffi::c_char);
    pbuf_printf(
        o,
        b"/CIDSystemInfo<</Registry(Adobe)/Ordering(Identity)/Supplement 0>>/FontDescriptor %d 0 R/DW 1000\0"
            as *const u8 as *const ::core::ffi::c_char,
        fd,
    );
    if !(*f).cff {
        pbuf_puts(
            o,
            b"/CIDToGIDMap/Identity\0" as *const u8 as *const ::core::ffi::c_char,
        );
    }
    if (*p).vertical { pdf_document::vertical_metrics(c,p,o); }
    pbuf_puts(o, b"/W[\0" as *const u8 as *const ::core::ffi::c_char);
    let mut g: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while g < (*p).nused {
        if !(*(*p).used.offset(g as isize) == 0) {
            pbuf_printf(o, b"%d[\0" as *const u8 as *const ::core::ffi::c_char, g);
            let mut k: ::core::ffi::c_int = g;
            while k < (*p).nused && *(*p).used.offset(k as isize) as ::core::ffi::c_int != 0 {
                pbuf_real(
                    o,
                    native_advance(f, k) as ::core::ffi::c_double * 1000.0f64
                        / (*f).units_per_em as ::core::ffi::c_double,
                );
                pbuf_putc(o, ' ' as i32);
                k += 1;
            }
            pbuf_puts(o, b"]\0" as *const u8 as *const ::core::ffi::c_char);
            g = k;
        }
        g += 1;
    }
    pbuf_puts(o, b"]>>\0" as *const u8 as *const ::core::ffi::c_char);
    pdfw_end((*c).pw);
    pdfw_begin((*c).pw, fd);
    pbuf_puts(
        o,
        b"<</Type/FontDescriptor/FontName\0" as *const u8 as *const ::core::ffi::c_char,
    );
    pdfw_name(o, &raw mut (*f).psname as *mut ::core::ffi::c_char);
    pbuf_puts(
        o,
        b"/Flags 4/FontBBox[\0" as *const u8 as *const ::core::ffi::c_char,
    );
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < 4 as ::core::ffi::c_int {
        pbuf_real(o, (*f).bbox[i as usize]);
        pbuf_putc(
            o,
            if i < 3 as ::core::ffi::c_int {
                ' ' as i32
            } else {
                ']' as i32
            },
        );
        i += 1;
    }
    pbuf_puts(
        o,
        b"/ItalicAngle \0" as *const u8 as *const ::core::ffi::c_char,
    );
    pbuf_real(o, (*f).italic_angle);
    pbuf_puts(o, b"/Ascent \0" as *const u8 as *const ::core::ffi::c_char);
    pbuf_real(o, (*f).ascent);
    pbuf_puts(o, b"/Descent \0" as *const u8 as *const ::core::ffi::c_char);
    pbuf_real(o, (*f).descent);
    pbuf_puts(
        o,
        b"/CapHeight \0" as *const u8 as *const ::core::ffi::c_char,
    );
    pbuf_real(o, (*f).cap_height);
    pbuf_printf(
        o,
        b"/StemV 80/%s %d 0 R>>\0" as *const u8 as *const ::core::ffi::c_char,
        if (*f).cff as ::core::ffi::c_int != 0 {
            b"FontFile3\0" as *const u8 as *const ::core::ffi::c_char
        } else {
            b"FontFile2\0" as *const u8 as *const ::core::ffi::c_char
        },
        ff,
    );
    pdfw_end((*c).pw);
    let mut dict: [::core::ffi::c_char; 96] = [0; 96];
    if (*f).cff {
        snprintf(
            &raw mut dict as *mut ::core::ffi::c_char,
            ::core::mem::size_of::<[::core::ffi::c_char; 96]>() as size_t,
            b"/Subtype/OpenType\0" as *const u8 as *const ::core::ffi::c_char,
        );
    } else {
        snprintf(
            &raw mut dict as *mut ::core::ffi::c_char,
            ::core::mem::size_of::<[::core::ffi::c_char; 96]>() as size_t,
            b"/Length1 %zu\0" as *const u8 as *const ::core::ffi::c_char,
            (*f).sfnt_len,
        );
    }
    pdfw_stream_deflated(
        (*c).pw,
        ff,
        &raw mut dict as *mut ::core::ffi::c_char,
        (*f).deflated.data as *const ::core::ffi::c_void,
        (*f).deflated.len,
    );
    if tu>0 {
    let mut cmap: pbuf = pbuf {
        data: ::core::ptr::null_mut::<::core::ffi::c_uchar>(),
        len: 0,
        cap: 0,
    };
    pbuf_puts(
        &raw mut cmap,
        b"/CIDInit/ProcSet findresource begin\n12 dict begin\nbegincmap\n/CIDSystemInfo<</Registry(Adobe)/Ordering(UCS)/Supplement 0>>def\n/CMapName/Adobe-Identity-UCS def\n/CMapType 2 def\n1 begincodespacerange\n<0000><FFFF>\nendcodespacerange\n\0"
            as *const u8 as *const ::core::ffi::c_char,
    );
    let mut count: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut block: pbuf = pbuf {
        data: ::core::ptr::null_mut::<::core::ffi::c_uchar>(),
        len: 0,
        cap: 0,
    };
    let mut g_0: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while g_0 < (*p).nused {
        if !(*(*p).used.offset(g_0 as isize) == 0 || *(*f).to_unicode.offset(g_0 as isize) == 0) {
            let mut u: uint32_t = *(*f).to_unicode.offset(g_0 as isize);
            if u >= 0x10000 as uint32_t {
                let mut v: uint32_t = u.wrapping_sub(0x10000 as uint32_t);
                pbuf_printf(
                    &raw mut block,
                    b"<%04X><%04X%04X>\n\0" as *const u8 as *const ::core::ffi::c_char,
                    g_0,
                    (0xd800 as uint32_t).wrapping_add(v >> 10 as ::core::ffi::c_int),
                    (0xdc00 as uint32_t).wrapping_add(v & 0x3ff as uint32_t),
                );
            } else {
                pbuf_printf(
                    &raw mut block,
                    b"<%04X><%04X>\n\0" as *const u8 as *const ::core::ffi::c_char,
                    g_0,
                    u,
                );
            }
            count += 1;
            if count == 100 as ::core::ffi::c_int {
                pbuf_printf(
                    &raw mut cmap,
                    b"100 beginbfchar\n%.*sendbfchar\n\0" as *const u8
                        as *const ::core::ffi::c_char,
                    block.len as ::core::ffi::c_int,
                    block.data as *mut ::core::ffi::c_char,
                );
                pbuf_clear(&raw mut block);
                count = 0 as ::core::ffi::c_int;
            }
        }
        g_0 += 1;
    }
    if count != 0 {
        pbuf_printf(
            &raw mut cmap,
            b"%d beginbfchar\n%.*sendbfchar\n\0" as *const u8 as *const ::core::ffi::c_char,
            count,
            block.len as ::core::ffi::c_int,
            block.data as *mut ::core::ffi::c_char,
        );
    }
    pbuf_puts(
        &raw mut cmap,
        b"endcmap\nCMapName currentdict/CMap defineresource pop\nend\nend\n\0" as *const u8
            as *const ::core::ffi::c_char,
    );
    pdfw_stream(
        (*c).pw,
        tu,
        ::core::ptr::null::<::core::ffi::c_char>(),
        cmap.data as *const ::core::ffi::c_void,
        cmap.len,
        true_0 != 0,
    );
    pbuf_free(&raw mut cmap);
    pbuf_free(&raw mut block);
    }
}
unsafe extern "C" fn write_type1_font(mut c: *mut conv_ctx, mut p: *mut pdf_font) {
    let mut t: *mut type1_font = (*p).t1;
    if (*t).deflated.len == 0 {
        pbuf_deflate(
            &raw mut (*t).deflated,
            (*t).data,
            (*t).len1.wrapping_add((*t).len2).wrapping_add((*t).len3),
            6 as ::core::ffi::c_int,
        );
    }
    let mut first: ::core::ffi::c_int = 256 as ::core::ffi::c_int;
    let mut last: ::core::ffi::c_int = -(1 as ::core::ffi::c_int);
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < 256 as ::core::ffi::c_int {
        if *(*p).used.offset(i as isize) != 0 {
            if i < first {
                first = i;
            }
            last = i;
        }
        i += 1;
    }
    if last < 0 as ::core::ffi::c_int {
        last = 0 as ::core::ffi::c_int;
        first = last;
    }
    let mut fd: ::core::ffi::c_int = pdfw_alloc((*c).pw);
    let mut ff: ::core::ffi::c_int = pdfw_alloc((*c).pw);
    let mut o: *mut pbuf = pdfw_out((*c).pw);
    let unicode = backend_options::type1_unicode(c,p);
    pdfw_begin((*c).pw, (*p).obj);
    pbuf_puts(
        o,
        b"<</Type/Font/Subtype/Type1/BaseFont\0" as *const u8 as *const ::core::ffi::c_char,
    );
    pdfw_name(o, &raw mut (*t).fontname as *mut ::core::ffi::c_char);
    pbuf_printf(
        o,
        b"/FirstChar %d/LastChar %d/Widths[\0" as *const u8 as *const ::core::ffi::c_char,
        first,
        last,
    );
    let mut i_0: ::core::ffi::c_int = first;
    while i_0 <= last {
        pbuf_real(
            o,
            if !(*p).tfm.is_null() && (*(*p).tfm).exists[i_0 as usize] as ::core::ffi::c_int != 0 {
                (*(*p).tfm).width[i_0 as usize] as ::core::ffi::c_double / 1048.576f64
            } else {
                0 as ::core::ffi::c_int as ::core::ffi::c_double
            },
        );
        pbuf_putc(o, ' ' as i32);
        i_0 += 1;
    }
    pbuf_printf(
        o,
        b"]/FontDescriptor %d 0 R\0" as *const u8 as *const ::core::ffi::c_char,
        fd,
    );
    if !(*p).enc.is_null() {
        pbuf_puts(
            o,
            b"/Encoding<</Type/Encoding/Differences[\0" as *const u8 as *const ::core::ffi::c_char,
        );
        let mut gap: bool = true_0 != 0;
        let mut i_1: ::core::ffi::c_int = first;
        while i_1 <= last {
            if *(*p).used.offset(i_1 as isize) == 0 || (*(*p).enc).glyph[i_1 as usize].is_null() {
                gap = true_0 != 0;
            } else {
                if gap {
                    pbuf_printf(o, b" %d\0" as *const u8 as *const ::core::ffi::c_char, i_1);
                }
                pdfw_name(o, (*(*p).enc).glyph[i_1 as usize]);
                gap = false_0 != 0;
            }
            i_1 += 1;
        }
        pbuf_puts(o, b"]>>\0" as *const u8 as *const ::core::ffi::c_char);
    }
    if let Some(unicode)=unicode {pbuf_printf(o,b"/ToUnicode %d 0 R\0".as_ptr().cast(),unicode);}
    font_output::attributes(c,p,o);
    pbuf_puts(o, b">>\0" as *const u8 as *const ::core::ffi::c_char);
    pdfw_end((*c).pw);
    pdfw_begin((*c).pw, fd);
    pbuf_puts(
        o,
        b"<</Type/FontDescriptor/FontName\0" as *const u8 as *const ::core::ffi::c_char,
    );
    pdfw_name(o, &raw mut (*t).fontname as *mut ::core::ffi::c_char);
    pbuf_puts(
        o,
        b"/Flags 4/FontBBox[\0" as *const u8 as *const ::core::ffi::c_char,
    );
    let mut i_2: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i_2 < 4 as ::core::ffi::c_int {
        pbuf_real(o, (*t).bbox[i_2 as usize]);
        pbuf_putc(
            o,
            if i_2 < 3 as ::core::ffi::c_int {
                ' ' as i32
            } else {
                ']' as i32
            },
        );
        i_2 += 1;
    }
    pbuf_puts(
        o,
        b"/ItalicAngle 0/Ascent \0" as *const u8 as *const ::core::ffi::c_char,
    );
    pbuf_real(o, (*t).bbox[3 as ::core::ffi::c_int as usize]);
    pbuf_puts(o, b"/Descent \0" as *const u8 as *const ::core::ffi::c_char);
    pbuf_real(o, (*t).bbox[1 as ::core::ffi::c_int as usize]);
    pbuf_puts(
        o,
        b"/CapHeight \0" as *const u8 as *const ::core::ffi::c_char,
    );
    pbuf_real(o, (*t).bbox[3 as ::core::ffi::c_int as usize]);
    pbuf_printf(
        o,
        b"/StemV 80/FontFile %d 0 R\0" as *const u8 as *const ::core::ffi::c_char,
        ff,
    );
    font_output::charset(c,p,o);pbuf_puts(o,b">>\0".as_ptr().cast());
    pdfw_end((*c).pw);
    let mut dict: [::core::ffi::c_char; 96] = [0; 96];
    snprintf(
        &raw mut dict as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 96]>() as size_t,
        b"/Length1 %zu/Length2 %zu/Length3 %zu\0" as *const u8 as *const ::core::ffi::c_char,
        (*t).len1,
        (*t).len2,
        (*t).len3,
    );
    pdfw_stream_deflated(
        (*c).pw,
        ff,
        &raw mut dict as *mut ::core::ffi::c_char,
        (*t).deflated.data as *const ::core::ffi::c_void,
        (*t).deflated.len,
    );
}
unsafe extern "C" fn write_xobject(mut c: *mut conv_ctx, mut u: *mut xobj_use) {
    let mut ci: *mut cached_image = (*u).ci;
    if !(*ci).pdf {
        let mut dict: [::core::ffi::c_char; 320] = [0; 320];
        let mut smask: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
        if (*ci).img.smask.len != 0 {
            smask = pdfw_alloc((*c).pw);
            let mut sd: [::core::ffi::c_char; 128] = [0; 128];
            snprintf(
                &raw mut sd as *mut ::core::ffi::c_char,
                ::core::mem::size_of::<[::core::ffi::c_char; 128]>() as size_t,
                b"/Type/XObject/Subtype/Image/Width %d/Height %d/ColorSpace/DeviceGray/BitsPerComponent %d\0"
                    as *const u8 as *const ::core::ffi::c_char,
                (*ci).img.width,
                (*ci).img.height,
                if std::ffi::CStr::from_ptr((*ci).img.dict.as_ptr()).to_bytes().windows(20).any(|s|s==b"/BitsPerComponent 16"){16}else{8},
            );
            pdfw_stream_deflated(
                (*c).pw,
                smask,
                &raw mut sd as *mut ::core::ffi::c_char,
                (*ci).img.smask.data as *const ::core::ffi::c_void,
                (*ci).img.smask.len,
            );
        }
        if (*ci).img.dct {
            snprintf(
                &raw mut dict as *mut ::core::ffi::c_char,
                ::core::mem::size_of::<[::core::ffi::c_char; 320]>() as size_t,
                b"/Type/XObject/Subtype/Image%s/Filter/DCTDecode\0" as *const u8
                    as *const ::core::ffi::c_char,
                &raw mut (*ci).img.dict as *mut ::core::ffi::c_char,
            );
            let image_dictionary = pdf_document::Document::image_dictionary(c, ci, dict.as_ptr());
            pdfw_stream(
                (*c).pw,
                (*u).obj,
                image_dictionary.as_ptr(),
                (*ci).img.data.data as *const ::core::ffi::c_void,
                (*ci).img.data.len,
                false_0 != 0,
            );
        } else {
            if smask != 0 {
                snprintf(
                    &raw mut dict as *mut ::core::ffi::c_char,
                    ::core::mem::size_of::<[::core::ffi::c_char; 320]>() as size_t,
                    b"/Type/XObject/Subtype/Image%s/SMask %d 0 R\0" as *const u8
                        as *const ::core::ffi::c_char,
                    &raw mut (*ci).img.dict as *mut ::core::ffi::c_char,
                    smask,
                );
            } else {
                snprintf(
                    &raw mut dict as *mut ::core::ffi::c_char,
                    ::core::mem::size_of::<[::core::ffi::c_char; 320]>() as size_t,
                    b"/Type/XObject/Subtype/Image%s\0" as *const u8 as *const ::core::ffi::c_char,
                    &raw mut (*ci).img.dict as *mut ::core::ffi::c_char,
                );
            }
            let image_dictionary = pdf_document::Document::image_dictionary(c, ci, dict.as_ptr());
            pdfw_stream_deflated(
                (*c).pw,
                (*u).obj,
                image_dictionary.as_ptr(),
                (*ci).img.data.data as *const ::core::ffi::c_void,
                (*ci).img.data.len,
            );
        }
        return;
    }
    let mut info: pr_page_info = pr_page_info {
        box_0: [0.; 4],
        rotate: 0,
        page: ::core::ptr::null_mut::<pr_obj>(),
        resources: ::core::ptr::null_mut::<pr_obj>(),
    };
    pr_page(
        (*ci).doc,
        pr_normalize_page((*ci).doc, (*u).page) - 1 as ::core::ffi::c_int,
        if (*u).box_0 != 0 {
            (*u).box_0
        } else {
            PR_BOX_CROP as ::core::ffi::c_int
        },
        &raw mut info,
    );
    let mut m: [::core::ffi::c_double; 6] = [0.; 6];
    pr_rotation_matrix(info.rotate, &raw mut m as *mut ::core::ffi::c_double);
    let mut imp: *mut pdfw_import = pdfw_import_begin((*c).pw, (*ci).doc);
    pdf_inclusion::fonts(c,(*ci).doc,info.resources,imp);
    let mut d: pbuf = pbuf {
        data: ::core::ptr::null_mut::<::core::ffi::c_uchar>(),
        len: 0,
        cap: 0,
    };
    pbuf_puts(
        &raw mut d,
        b"/Type/XObject/Subtype/Form/BBox[\0" as *const u8 as *const ::core::ffi::c_char,
    );
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < 4 as ::core::ffi::c_int {
        pbuf_real(&raw mut d, info.box_0[i as usize]);
        pbuf_putc(
            &raw mut d,
            if i < 3 as ::core::ffi::c_int {
                ' ' as i32
            } else {
                ']' as i32
            },
        );
        i += 1;
    }
    pbuf_puts(
        &raw mut d,
        b"/Matrix[\0" as *const u8 as *const ::core::ffi::c_char,
    );
    let mut i_0: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i_0 < 6 as ::core::ffi::c_int {
        pbuf_real(&raw mut d, m[i_0 as usize]);
        pbuf_putc(
            &raw mut d,
            if i_0 < 5 as ::core::ffi::c_int {
                ' ' as i32
            } else {
                ']' as i32
            },
        );
        i_0 += 1;
    }
    pbuf_puts(
        &raw mut d,
        b"/Resources \0" as *const u8 as *const ::core::ffi::c_char,
    );
    pdfw_import_value(imp, &raw mut d, info.resources);
    let policy=(*(*c).document).backend.info_policy();
    if let Some(key)=policy.ptex_key("FileName") {
        pbuf_putc(&raw mut d,b'/' as i32);pbuf_append(&raw mut d,key.as_ptr().cast(),key.len());
        let file=std::ffi::CStr::from_ptr((*ci).filename).to_bytes();let literal=std::ffi::CString::new(format!("({})",String::from_utf8_lossy(file).replace('\\',"\\\\").replace('(',"\\(").replace(')',"\\)"))).unwrap();pbuf_puts(&raw mut d,literal.as_ptr());
    }
    if let Some(key)=policy.ptex_key("PageNumber") {let value=format!("/{key} {}",(*u).page);pbuf_append(&raw mut d,value.as_ptr().cast(),value.len());}
    if let Some(key)=policy.ptex_key("InfoDict") {let source=pr_document_info((*ci).doc);if !source.is_null(){let key=format!("/{key} ");pbuf_append(&raw mut d,key.as_ptr().cast(),key.len());pdfw_import_value(imp,&raw mut d,source);}}
    let mut group: *mut pr_obj = pr_get(
        (*ci).doc,
        info.page,
        b"Group\0" as *const u8 as *const ::core::ffi::c_char,
    );
    if !group.is_null() {
        pbuf_puts(
            &raw mut d,
            b"/Group \0" as *const u8 as *const ::core::ffi::c_char,
        );
        pdfw_import_value(imp, &raw mut d, group);
    }
    let mut contents: *mut pr_obj = pr_get(
        (*ci).doc,
        info.page,
        b"Contents\0" as *const u8 as *const ::core::ffi::c_char,
    );
    let mut data: pbuf = pbuf {
        data: ::core::ptr::null_mut::<::core::ffi::c_uchar>(),
        len: 0,
        cap: 0,
    };
    let mut raw_single: bool = false_0 != 0;
    if !contents.is_null()
        && (*contents).type_0 as ::core::ffi::c_uint
            == PR_STREAM as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        let mut filter: *mut pr_obj = pr_get(
            (*ci).doc,
            contents,
            b"Filter\0" as *const u8 as *const ::core::ffi::c_char,
        );
        let mut parms: *mut pr_obj = pr_get(
            (*ci).doc,
            contents,
            b"DecodeParms\0" as *const u8 as *const ::core::ffi::c_char,
        );
        let mut len: size_t = 0;
        let mut raw: *const ::core::ffi::c_uchar = pr_stream_raw((*ci).doc, contents, &raw mut len);
        if !raw.is_null() {
            pbuf_append(&raw mut data, raw as *const ::core::ffi::c_void, len);
            if !filter.is_null() {
                pbuf_puts(
                    &raw mut d,
                    b"/Filter \0" as *const u8 as *const ::core::ffi::c_char,
                );
                pdfw_import_value(imp, &raw mut d, filter);
            }
            if !parms.is_null() {
                pbuf_puts(
                    &raw mut d,
                    b"/DecodeParms \0" as *const u8 as *const ::core::ffi::c_char,
                );
                pdfw_import_value(imp, &raw mut d, parms);
            }
            raw_single = true_0 != 0;
        }
    } else if !contents.is_null()
        && (*contents).type_0 as ::core::ffi::c_uint
            == PR_ARRAY as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        let mut i_1: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
        while i_1 < (*contents).u.arr.n {
            let mut s: *mut pr_obj =
                pr_resolve((*ci).doc, *(*contents).u.arr.items.offset(i_1 as isize));
            if !(s.is_null()
                || (*s).type_0 as ::core::ffi::c_uint
                    != PR_STREAM as ::core::ffi::c_int as ::core::ffi::c_uint)
            {
                if !pr_stream_decode((*ci).doc, s, &raw mut data) {
                    warn_once(
                        c,
                        b"pdfcontent\0" as *const u8 as *const ::core::ffi::c_char,
                        b"PDF figure uses an unsupported content filter%s\0" as *const u8
                            as *const ::core::ffi::c_char,
                        b"\0" as *const u8 as *const ::core::ffi::c_char,
                    );
                    pbuf_clear(&raw mut data);
                    break;
                } else {
                    pbuf_putc(&raw mut data, '\n' as i32);
                }
            }
            i_1 += 1;
        }
    }
    pbuf_putc(&raw mut d, 0 as ::core::ffi::c_int);
    let image_dictionary = pdf_document::Document::image_dictionary(c, ci, d.data.cast());
    pbuf_clear(&raw mut d);
    pbuf_puts(&raw mut d, image_dictionary.as_ptr());
    pbuf_putc(&raw mut d, 0 as ::core::ffi::c_int);
    if raw_single {
        pdfw_stream(
            (*c).pw,
            (*u).obj,
            d.data as *mut ::core::ffi::c_char,
            data.data as *const ::core::ffi::c_void,
            data.len,
            false_0 != 0,
        );
    } else {
        pdfw_stream(
            (*c).pw,
            (*u).obj,
            d.data as *mut ::core::ffi::c_char,
            data.data as *const ::core::ffi::c_void,
            data.len,
            true_0 != 0,
        );
    }
    pbuf_free(&raw mut d);
    pbuf_free(&raw mut data);
    pdfw_import_end(imp);
}
unsafe extern "C" fn write_named(mut c: *mut conv_ctx, mut o: *mut named_obj) {
    if (*o).written {
        return;
    }
    (*o).written = true_0 != 0;
    let mut out: *mut pbuf = pdfw_out((*c).pw);
    match (*o).kind as ::core::ffi::c_uint {
        1 => {
            pdfw_begin((*c).pw, (*o).obj);
            pbuf_puts(out, b"<<\0" as *const u8 as *const ::core::ffi::c_char);
            let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
            while i < (*o).dict.n {
                pbuf_printf(
                    out,
                    b"/%s %s\0" as *const u8 as *const ::core::ffi::c_char,
                    (*(*o).dict.items.offset(i as isize)).key,
                    (*(*o).dict.items.offset(i as isize)).value,
                );
                i += 1;
            }
            pbuf_puts(out, b">>\0" as *const u8 as *const ::core::ffi::c_char);
            pdfw_end((*c).pw);
        }
        2 => {
            pdfw_begin((*c).pw, (*o).obj);
            pbuf_putc(out, '[' as i32);
            pbuf_append(
                out,
                (*o).array.data as *const ::core::ffi::c_void,
                (*o).array.len,
            );
            pbuf_putc(out, ']' as i32);
            pdfw_end((*c).pw);
        }
        3 => {
            pdfw_begin((*c).pw, (*o).obj);
            pbuf_append(
                out,
                (*o).array.data as *const ::core::ffi::c_void,
                (*o).array.len,
            );
            pdfw_end((*c).pw);
        }
        4 => {
            let mut d: pbuf = pbuf {
                data: ::core::ptr::null_mut::<::core::ffi::c_uchar>(),
                len: 0,
                cap: 0,
            };
            let mut has_filter=false;
            let mut i_0: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
            while i_0 < (*o).dict.n {
                if strcmp(
                    (*(*o).dict.items.offset(i_0 as isize)).key,
                    b"Length\0" as *const u8 as *const ::core::ffi::c_char,
                ) != 0

                {
                    if strcmp((*(*o).dict.items.offset(i_0 as isize)).key,b"Filter\0".as_ptr().cast())==0 {has_filter=true;}
                    pbuf_printf(
                        &raw mut d,
                        b"/%s %s\0" as *const u8 as *const ::core::ffi::c_char,
                        (*(*o).dict.items.offset(i_0 as isize)).key,
                        (*(*o).dict.items.offset(i_0 as isize)).value,
                    );
                }
                i_0 += 1;
            }
            pbuf_putc(&raw mut d, 0 as ::core::ffi::c_int);
            pdfw_stream(
                (*c).pw,
                (*o).obj,
                d.data as *mut ::core::ffi::c_char,
                (*o).array.data as *const ::core::ffi::c_void,
                (*o).array.len,
                !has_filter,
            );
            pbuf_free(&raw mut d);
        }
        _ => {
            pdfw_begin((*c).pw, (*o).obj);
            pbuf_puts(out, b"null\0" as *const u8 as *const ::core::ffi::c_char);
            pdfw_end((*c).pw);
        }
    };
}
#[no_mangle]
pub unsafe extern "C" fn xdv2pdf_new(mut resolver: xdv_resolver) -> *mut xdv2pdf {
    let mut w: *mut xdv2pdf =
        calloc(1 as size_t, ::core::mem::size_of::<xdv2pdf>() as size_t) as *mut xdv2pdf;
    if w.is_null() {
        abort();
    }
    (*w).res = resolver;
    (*w).fonts = font_cache_new(resolver);
    return w;
}
#[no_mangle]
pub unsafe extern "C" fn xdv2pdf_free(mut w: *mut xdv2pdf) {
    if w.is_null() {
        return;
    }
    let mut ci: *mut cached_image = (*w).images;
    while !ci.is_null() {
        let mut n: *mut cached_image = (*ci).next as *mut cached_image;
        tbuf_drop((*ci).file);
        free((*ci).filename.cast());
        image_free(&raw mut (*ci).img);
        pr_close((*ci).doc);
        free(ci as *mut ::core::ffi::c_void);
        ci = n;
    }
    font_cache_free((*w).fonts);
    free(w as *mut ::core::ffi::c_void);
}
unsafe extern "C" fn prune_images(mut w: *mut xdv2pdf) {
    let mut pp: *mut *mut cached_image = &raw mut (*w).images;
    while !(*pp).is_null() {
        let mut ci: *mut cached_image = *pp;
        if (*(*ci).file).refs <= 1 as ::core::ffi::c_int {
            *pp = (*ci).next as *mut cached_image;
            tbuf_drop((*ci).file);
            free((*ci).filename.cast());
            image_free(&raw mut (*ci).img);
            pr_close((*ci).doc);
            free(ci as *mut ::core::ffi::c_void);
        } else {
            pp = &raw mut (*ci).next as *mut *mut cached_image;
        }
    }
}
unsafe extern "C" fn reset_page_state(mut c: *mut conv_ctx) {
    pdf_document::Document::begin_page(c);
    (*(*c).postscript).begin_page();
    (*c).oy = 0 as ::core::ffi::c_int as ::core::ffi::c_double;
    (*c).ox = (*c).oy;
    (*c).off_depth = 0 as ::core::ffi::c_int;
    (*c).q_depth = 0 as ::core::ffi::c_int;
    (*c).pos_valid = false_0 != 0;
    (*c).in_tj = (*c).pos_valid;
    (*c).in_bt = (*c).in_tj;
    (*c).t_font = -(1 as ::core::ffi::c_int);
    (*c).has_background = false_0 != 0;
}
#[no_mangle]
pub unsafe extern "C" fn xdv2pdf_write(
    mut w: *mut xdv2pdf,
    mut ranges: *const xdv_range,
    mut nranges: ::core::ffi::c_int,
    mut pdf: *mut pbuf,
    mut warnings: *mut pbuf,
) -> ::core::ffi::c_int {
    (*w).epoch += 1;
    font_map_reset((*w).fonts);
    let mut c: *mut conv_ctx =
        calloc(1 as size_t, ::core::mem::size_of::<conv_ctx>() as size_t) as *mut conv_ctx;
    if c.is_null() {
        abort();
    }
    (*c).w = w;
    (*c).warnings = warnings;
    (*c).pw = pdfw_new(pdf);
    let mut catalog: ::core::ffi::c_int = pdfw_alloc((*c).pw);
    let mut pages: ::core::ffi::c_int = pdfw_alloc((*c).pw);
    let mut info: ::core::ffi::c_int = pdfw_alloc((*c).pw);
    (*c).font_dict_obj = pdfw_alloc((*c).pw);
    (*c).xobject_dict_obj = pdfw_alloc((*c).pw);
    (*c).resources_obj = pdfw_alloc((*c).pw);
    (*c).document = Box::into_raw(Box::new(pdf_document::Document::default()));
    (*c).font_output = Box::into_raw(Box::new(font_output::State::default()));
    (*(*c).document).catalog_object = catalog;
    (*(*c).document).pages_object = pages;
    (*(*c).document).info_object = info;
    (*(*c).document).names_object = pdfw_alloc((*c).pw);
    backend_options::load_builtin_mappings(c);
    (*c).postscript = Box::into_raw(Box::new(crate::shared_postscript::Interpreter::new()));
    (*c).expansion = 1.;
    let mut reserved_pages = 0usize;
    for range in std::slice::from_raw_parts(ranges,nranges.max(0) as usize) {
        reserved_pages += range.count.max(0).min((xdv_index_page_count(range.index)-range.first).max(0)) as usize;
    }
    pdf_document::Document::reserve_pages(c,reserved_pages);

    let mut page_objs: *mut ::core::ffi::c_int = ::core::ptr::null_mut::<::core::ffi::c_int>();
    let mut npages: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut cappages: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    (*c).default_w = 595.2756;
    (*c).default_h = 841.8898;
    let mut r: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while r < nranges {
        let mut rg: *const xdv_range = ranges.offset(r as isize) as *const xdv_range;
        let mut num: uint32_t = 0;
        let mut den: uint32_t = 0;
        let mut mag: uint32_t = 0;
        if !(!xdv_index_preamble((*rg).index, &raw mut num, &raw mut den, &raw mut mag)
            || den == 0 as uint32_t)
        {
            if mag == 0 as uint32_t {
                mag = 1000 as uint32_t;
            }
            (*c).conv = num as ::core::ffi::c_double / den as ::core::ffi::c_double
                * mag as ::core::ffi::c_double
                / 1000.0f64
                * 72.0f64
                / 254000.0f64;

            let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
            while i < MAX_COLORSTACKS {
                let mut k: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
                while k < (*c).cs[i as usize].depth {
                    free((*c).cs[i as usize].ops[k as usize] as *mut ::core::ffi::c_void);
                    k += 1;
                }
                free((*c).cs[i as usize].init as *mut ::core::ffi::c_void);
                memset(
                    (&raw mut (*c).cs as *mut colorstack).offset(i as isize) as *mut colorstack
                        as *mut ::core::ffi::c_void,
                    0 as ::core::ffi::c_int,
                    ::core::mem::size_of::<colorstack>() as size_t,
                );
                i += 1;
            }
            let mut i_0: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
            while i_0 < (*c).old_depth {
                free((*c).old_colors[i_0 as usize] as *mut ::core::ffi::c_void);
                i_0 += 1;
            }
            (*c).old_depth = 0 as ::core::ffi::c_int;
            pdf_document::Document::reset_colors(c);
            let mut fonts: *mut font_table =
                calloc(1 as size_t, ::core::mem::size_of::<font_table>() as size_t)
                    as *mut font_table;
            if fonts.is_null() {
                abort();
            }
            let mut last: ::core::ffi::c_int = (*rg).first + (*rg).count;
            if last > xdv_index_page_count((*rg).index) {
                last = xdv_index_page_count((*rg).index);
            }
            let mut st: *mut dvi_state =
                calloc(1 as size_t, ::core::mem::size_of::<dvi_state>() as size_t)
                    as *mut dvi_state;
            if st.is_null() {
                abort();
            }
            (*st).fonts = fonts;
            (*(*c).font_output).table = fonts;
            (*st).scale = 1 as ::core::ffi::c_int as ::core::ffi::c_double;
            (*c).ntargets = 1 as ::core::ffi::c_int;
            memset(
                (&raw mut (*c).targets as *mut target).offset(0 as ::core::ffi::c_int as isize)
                    as *mut target as *mut ::core::ffi::c_void,
                0 as ::core::ffi::c_int,
                ::core::mem::size_of::<target>() as size_t,
            );
            let mut pg: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
            while pg < last {
                let mut xp: *const xdv_page = xdv_index_page((*rg).index, pg);
                let mut pb: *const ::core::ffi::c_uchar = (*rg)
                    .data
                    .offset((*xp).bop as isize)
                    .offset(45 as ::core::ffi::c_int as isize);
                let mut pe: *const ::core::ffi::c_uchar = (*rg)
                    .data
                    .offset((*xp).eop_end as isize)
                    .offset(-(1 as ::core::ffi::c_int as isize));
                let mut render: bool = pg >= (*rg).first;
                (*c).render = render;
                (*c).this_page = if render { (*(*c).document).pages[npages as usize] } else {0};
                (*c).expansion = 1.;

                page_size_prescan(c, pb, pe);
                reset_page_state(c);
                font_output::new_page(c);
                memset(
                    &raw mut (*st).r as *mut ::core::ffi::c_void,
                    0 as ::core::ffi::c_int,
                    ::core::mem::size_of::<dvi_regs>() as size_t,
                );
                (*st).sp = 0 as ::core::ffi::c_int;
                (*st).font = ::core::ptr::null_mut::<font_slot>();
                let mut t: *mut target = (&raw mut (*c).targets as *mut target)
                    .offset(0 as ::core::ffi::c_int as isize)
                    as *mut target;
                pbuf_clear(&raw mut (*t).content);
                if render {
                    pdf_document::Document::restore_colors(c);
                    let mut i_1: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
                    while i_1 < MAX_COLORSTACKS {
                        if (*c).cs[i_1 as usize].page as ::core::ffi::c_int != 0
                            || !(*c).cs[i_1 as usize].init.is_null()
                        {
                            let mut ops: *const ::core::ffi::c_char =
                                if (*c).cs[i_1 as usize].depth != 0 {
                                    (*c).cs[i_1 as usize].ops[((*c).cs[i_1 as usize].depth
                                        - 1 as ::core::ffi::c_int)
                                        as usize]
                                } else {
                                    (*c).cs[i_1 as usize].init
                                };
                            if !ops.is_null() && *ops as ::core::ffi::c_int != 0 {
                                pbuf_puts(&raw mut (*t).content, ops);
                                pbuf_putc(&raw mut (*t).content, '\n' as i32);
                            }
                        }
                        i_1 += 1;
                    }
                    if (*c).old_depth != 0 {
                        pbuf_puts(
                            &raw mut (*t).content,
                            (*c).old_colors[((*c).old_depth - 1 as ::core::ffi::c_int) as usize],
                        );
                        pbuf_putc(&raw mut (*t).content, '\n' as i32);
                    }
                }
                interpret(c, st, pb, pe, 0 as ::core::ffi::c_int);
                while (*c).ntargets > 1 as ::core::ffi::c_int {
                    pbuf_free(
                        &raw mut (*(cur as unsafe extern "C" fn(*mut conv_ctx) -> *mut target)(c))
                            .content,
                    );
                    resources_free(
                        &raw mut (*(cur as unsafe extern "C" fn(*mut conv_ctx) -> *mut target)(c))
                            .res,
                    );
                    (*c).render = (*cur(c)).saved_render;
                    (*c).ntargets -= 1;
                }
                if render {
                    text_end(c);
                    while (*c).q_depth > 0 as ::core::ffi::c_int {
                        pbuf_puts(
                            &raw mut (*t).content,
                            b"Q\n\0" as *const u8 as *const ::core::ffi::c_char,
                        );
                        (*c).q_depth -= 1;
                    }
                    let mut content: pbuf = pbuf {
                        data: ::core::ptr::null_mut::<::core::ffi::c_uchar>(),
                        len: 0,
                        cap: 0,
                    };
                    if (*c).has_background {
                        pbuf_printf(
                            &raw mut content,
                            b"q \0" as *const u8 as *const ::core::ffi::c_char,
                        );
                        emit_num(
                            &raw mut content,
                            (*c).background[0 as ::core::ffi::c_int as usize],
                        );
                        emit_num(
                            &raw mut content,
                            (*c).background[1 as ::core::ffi::c_int as usize],
                        );
                        emit_num(
                            &raw mut content,
                            (*c).background[2 as ::core::ffi::c_int as usize],
                        );
                        pbuf_printf(
                            &raw mut content,
                            b"rg 0 0 \0" as *const u8 as *const ::core::ffi::c_char,
                        );
                        emit_num(&raw mut content, (*c).page_w);
                        emit_num(&raw mut content, (*c).page_h);
                        pbuf_puts(
                            &raw mut content,
                            b"re f Q\n\0" as *const u8 as *const ::core::ffi::c_char,
                        );
                    }
                    pbuf_append(
                        &raw mut content,
                        (*t).content.data as *const ::core::ffi::c_void,
                        (*t).content.len,
                    );
                    let mut cobj: ::core::ffi::c_int = pdfw_alloc((*c).pw);
                    let mut pobj: ::core::ffi::c_int = (*c).this_page;
                    pdfw_stream(
                        (*c).pw,
                        cobj,
                        ::core::ptr::null::<::core::ffi::c_char>(),
                        content.data as *const ::core::ffi::c_void,
                        content.len,
                        true_0 != 0,
                    );
                    pbuf_free(&raw mut content);
                    let mut o: *mut pbuf = pdfw_out((*c).pw);
                    pdfw_begin((*c).pw, pobj);
                    pbuf_printf(
                        o,
                        b"<</Type/Page/Parent %d 0 R/MediaBox[0 0 \0" as *const u8
                            as *const ::core::ffi::c_char,
                        pages,
                    );
                    pbuf_real(o, (*c).page_w);
                    pbuf_putc(o, ' ' as i32);
                    pbuf_real(o, (*c).page_h);
                    pbuf_printf(
                        o,
                        b"]/Resources %d 0 R/Contents %d 0 R\0" as *const u8
                            as *const ::core::ffi::c_char,
                        (*c).resources_obj,
                        cobj,
                    );
                    pdf_document::Document::page_body(c,o,pobj);
                    pbuf_puts(o,b">>\0".as_ptr().cast());
                    pdfw_end((*c).pw);
                    if npages == cappages {
                        cappages = if cappages != 0 {
                            cappages * 2 as ::core::ffi::c_int
                        } else {
                            64 as ::core::ffi::c_int
                        };
                        page_objs = realloc(
                            page_objs as *mut ::core::ffi::c_void,
                            (::core::mem::size_of::<::core::ffi::c_int>() as size_t)
                                .wrapping_mul(cappages as size_t),
                        ) as *mut ::core::ffi::c_int;
                        if page_objs.is_null() {
                            abort();
                        }
                    }
                    let fresh1 = npages;
                    npages = npages + 1;
                    *page_objs.offset(fresh1 as isize) = pobj;
                }
                pg += 1;
            }
            let mut i_2: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
            while i_2 < NCAT as ::core::ffi::c_int {
                let mut j: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
                while j < (*c).targets[0 as ::core::ffi::c_int as usize].res.cat[i_2 as usize].n {
                    kv_set(
                        (&raw mut (*c).page_res.cat as *mut kvlist).offset(i_2 as isize)
                            as *mut kvlist,
                        (*(*c).targets[0 as ::core::ffi::c_int as usize].res.cat[i_2 as usize]
                            .items
                            .offset(j as isize))
                        .key,
                        strlen(
                            (*(*c).targets[0 as ::core::ffi::c_int as usize].res.cat[i_2 as usize]
                                .items
                                .offset(j as isize))
                            .key,
                        ),
                        (*(*c).targets[0 as ::core::ffi::c_int as usize].res.cat[i_2 as usize]
                            .items
                            .offset(j as isize))
                        .value,
                        strlen(
                            (*(*c).targets[0 as ::core::ffi::c_int as usize].res.cat[i_2 as usize]
                                .items
                                .offset(j as isize))
                            .value,
                        ),
                    );
                    j += 1;
                }
                if !(*c).targets[0 as ::core::ffi::c_int as usize].res.cat_ref[i_2 as usize]
                    .is_null()
                {
                    free((*c).page_res.cat_ref[i_2 as usize] as *mut ::core::ffi::c_void);
                    (*c).page_res.cat_ref[i_2 as usize] = strdup(
                        (*c).targets[0 as ::core::ffi::c_int as usize].res.cat_ref[i_2 as usize],
                    );
                }
                i_2 += 1;
            }
            pbuf_free(
                &raw mut (*(&raw mut (*c).targets as *mut target)
                    .offset(0 as ::core::ffi::c_int as isize))
                .content,
            );
            resources_free(
                &raw mut (*(&raw mut (*c).targets as *mut target)
                    .offset(0 as ::core::ffi::c_int as isize))
                .res,
            );
            free(st as *mut ::core::ffi::c_void);
            table_free(fonts);
        }
        r += 1;
    }
    let mut o_0: *mut pbuf = pdfw_out((*c).pw);
    let mut i_3: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i_3 < (*c).npfonts {
        if (*(*c).pfonts.offset(i_3 as isize)).native {
            write_native_font(c, (*c).pfonts.offset(i_3 as isize) as *mut pdf_font);
        } else if (*(*c).pfonts.offset(i_3 as isize)).bitmap {
            bitmap_fonts::write(c,(*c).pfonts.offset(i_3 as isize),i_3);
        } else {
            write_type1_font(c, (*c).pfonts.offset(i_3 as isize) as *mut pdf_font);
        }
        i_3 += 1;
    }
    pdfw_begin((*c).pw, (*c).font_dict_obj);
    pbuf_puts(o_0, b"<<\0" as *const u8 as *const ::core::ffi::c_char);
    let mut i_4: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i_4 < (*c).npfonts {
        pbuf_printf(
            o_0,
            b"/%s %d 0 R\0" as *const u8 as *const ::core::ffi::c_char,
            std::ffi::CString::new(font_output::resource(c,i_4)).unwrap().as_ptr(),
            (*(*c).pfonts.offset(i_4 as isize)).obj,
        );
        i_4 += 1;
    }
    font_output::write_extra_resources(c,o_0);
    pbuf_puts(o_0, b">>\0" as *const u8 as *const ::core::ffi::c_char);
    pdfw_end((*c).pw);
    let mut i_5: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i_5 < (*c).nxobjs {
        write_xobject(c, (*c).xobjs.offset(i_5 as isize) as *mut xobj_use);
        i_5 += 1;
    }
    pdfw_begin((*c).pw, (*c).xobject_dict_obj);
    pbuf_puts(o_0, b"<<\0" as *const u8 as *const ::core::ffi::c_char);
    let mut i_6: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i_6 < (*c).nxobjs {
        pbuf_printf(
            o_0,
            b"/%s %d 0 R\0" as *const u8 as *const ::core::ffi::c_char,
            &raw mut (*(*c).xobjs.offset(i_6 as isize)).name as *mut ::core::ffi::c_char,
            (*(*c).xobjs.offset(i_6 as isize)).obj,
        );
        i_6 += 1;
    }
    let mut j_0: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while j_0 < (*c).page_res.cat[CAT_XOBJECT as ::core::ffi::c_int as usize].n {
        pbuf_printf(
            o_0,
            b"/%s %s\0" as *const u8 as *const ::core::ffi::c_char,
            (*(*c).page_res.cat[CAT_XOBJECT as ::core::ffi::c_int as usize]
                .items
                .offset(j_0 as isize))
            .key,
            (*(*c).page_res.cat[CAT_XOBJECT as ::core::ffi::c_int as usize]
                .items
                .offset(j_0 as isize))
            .value,
        );
        j_0 += 1;
    }
    pbuf_puts(o_0, b">>\0" as *const u8 as *const ::core::ffi::c_char);
    pdfw_end((*c).pw);
    pdfw_begin((*c).pw, (*c).resources_obj);
    pbuf_puts(o_0, b"<<\0" as *const u8 as *const ::core::ffi::c_char);
    write_resources_body(c, o_0, &raw mut (*c).page_res);
    pbuf_puts(
        o_0,
        if (*(*c).document).backend.integer("pdfomitprocset")!=0 {b">>\0".as_ptr()} else {b"/ProcSet[/PDF/Text/ImageB/ImageC]>>\0".as_ptr()} as *const ::core::ffi::c_char,
    );
    pdfw_end((*c).pw);
    pdf_document::Document::finish(c);
    let mut i_7: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i_7 < (*c).nnamed {
        write_named(c, (*c).named.offset(i_7 as isize) as *mut named_obj);
        i_7 += 1;
    }
    pdfw_begin((*c).pw, pages);
    pbuf_puts(
        o_0,
        b"<</Type/Pages/Kids[\0" as *const u8 as *const ::core::ffi::c_char,
    );
    let mut i_8: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i_8 < npages {
        pbuf_printf(
            o_0,
            b"%d 0 R \0" as *const u8 as *const ::core::ffi::c_char,
            *page_objs.offset(i_8 as isize),
        );
        i_8 += 1;
    }
    pbuf_printf(
        o_0,
        b"]/Count %d\0" as *const u8 as *const ::core::ffi::c_char,
        npages,
    );
    pdf_document::Document::pages_body(c,o_0);
    pbuf_puts(o_0,b">>\0".as_ptr().cast());
    pdfw_end((*c).pw);
    pdfw_begin((*c).pw, catalog);
    pbuf_printf(
        o_0,
        b"<</Type/Catalog/Pages %d 0 R\0" as *const u8 as *const ::core::ffi::c_char,
        pages,
    );
    pdf_document::Document::catalog_body(c,o_0);
    pbuf_puts(o_0,b">>\0".as_ptr().cast());
    pdfw_end((*c).pw);
    let omit_info = (*(*c).document).backend.integer("pdfomitinfodict")!=0;
    if !omit_info {
    pdfw_begin((*c).pw, info);
    pbuf_puts(
        o_0,
        b"<<\0"
            as *const u8 as *const ::core::ffi::c_char,
    );
    pdf_document::Document::info_body(c,o_0);
    pbuf_puts(o_0,b">>\0".as_ptr().cast());
    pdfw_end((*c).pw);
    pdf_document::Document::finish_version(c);
    }
    pdfw_finish((*c).pw, catalog, if omit_info {0}else{info});
    // Draft output typesets but leaves the displayed PDF unchanged.
    let draft = (*(*c).document).backend.integer("pdfdraftmode") > 0;
    if draft { pbuf_clear(o_0); }
    let mut i_9: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i_9 < (*c).npfonts {
        free((*(*c).pfonts.offset(i_9 as isize)).used as *mut ::core::ffi::c_void);
        i_9 += 1;
    }
    free((*c).pfonts as *mut ::core::ffi::c_void);
    free((*c).xobjs as *mut ::core::ffi::c_void);
    let mut i_10: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i_10 < (*c).nnamed {
        free((*(*c).named.offset(i_10 as isize)).name as *mut ::core::ffi::c_void);
        kv_free(&raw mut (*(*c).named.offset(i_10 as isize)).dict);
        pbuf_free(&raw mut (*(*c).named.offset(i_10 as isize)).array);
        i_10 += 1;
    }
    free((*c).named as *mut ::core::ffi::c_void);
    resources_free(&raw mut (*c).page_res);
    let mut i_11: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i_11 < MAX_COLORSTACKS {
        let mut k_0: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
        while k_0 < (*c).cs[i_11 as usize].depth {
            free((*c).cs[i_11 as usize].ops[k_0 as usize] as *mut ::core::ffi::c_void);
            k_0 += 1;
        }
        free((*c).cs[i_11 as usize].init as *mut ::core::ffi::c_void);
        i_11 += 1;
    }
    let mut i_12: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i_12 < (*c).old_depth {
        free((*c).old_colors[i_12 as usize] as *mut ::core::ffi::c_void);
        i_12 += 1;
    }
    free(page_objs as *mut ::core::ffi::c_void);
    drop(Box::from_raw((*c).font_output));
    let inclusion_error=(*(*c).document).inclusion.fatal;
    drop(Box::from_raw((*c).document));
    drop(Box::from_raw((*c).postscript));
    pdfw_free((*c).pw);
    free(c as *mut ::core::ffi::c_void);
    prune_images(w);
    return if inclusion_error {3} else if draft { 2 } else if npages > 0 as ::core::ffi::c_int {
        0 as ::core::ffi::c_int
    } else {
        1 as ::core::ffi::c_int
    };
}
pub const NULL: *mut ::core::ffi::c_void = __DARWIN_NULL;
