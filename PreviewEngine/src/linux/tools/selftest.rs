/* Pitex embedded preview engine — boundary regressions (B7–B9). Builds a
 * crafted XDV/corrupt font and drives the real xdv2pdf/synctex/parser code
 * paths. Run by `make selftest`. Pitex-authored (AGPL-3.0-or-later). */
// Translated from tools/selftest.c with C2Rust 0.22.1.
extern "C" {
    pub type _IO_wide_data;
    pub type _IO_codecvt;
    pub type _IO_marker;
    pub type internal_state;
    pub type xdv_index;
    pub type xdv2pdf;
    pub type font_cache;
    static mut stderr: *mut FILE;
    fn fprintf(
        __stream: *mut FILE,
        __format: *const ::core::ffi::c_char,
        ...
    ) -> ::core::ffi::c_int;
    fn strtod(
        __nptr: *const ::core::ffi::c_char,
        __endptr: *mut *mut ::core::ffi::c_char,
    ) -> ::core::ffi::c_double;
    fn calloc(__nmemb: size_t, __size: size_t) -> *mut ::core::ffi::c_void;
    fn realloc(
        __ptr: *mut ::core::ffi::c_void,
        __size: size_t,
    ) -> *mut ::core::ffi::c_void;
    fn free(__ptr: *mut ::core::ffi::c_void);
    fn abort() -> !;
    fn memcpy(
        __dest: *mut ::core::ffi::c_void,
        __src: *const ::core::ffi::c_void,
        __n: size_t,
    ) -> *mut ::core::ffi::c_void;
    fn memset(
        __s: *mut ::core::ffi::c_void,
        __c: ::core::ffi::c_int,
        __n: size_t,
    ) -> *mut ::core::ffi::c_void;
    fn memcmp(
        __s1: *const ::core::ffi::c_void,
        __s2: *const ::core::ffi::c_void,
        __n: size_t,
    ) -> ::core::ffi::c_int;
    fn memmem(
        __haystack: *const ::core::ffi::c_void,
        __haystacklen: size_t,
        __needle: *const ::core::ffi::c_void,
        __needlelen: size_t,
    ) -> *mut ::core::ffi::c_void;
    fn strlen(__s: *const ::core::ffi::c_char) -> size_t;
    fn tbuf_new(cap: size_t) -> *mut tbuf;
    fn tbuf_from_copy(data: *const ::core::ffi::c_void, len: size_t) -> *mut tbuf;
    fn tbuf_drop(b: *mut tbuf);
    fn tbuf_append(b: *mut tbuf, data: *const ::core::ffi::c_void, len: size_t);
    fn inflate(strm: z_streamp, flush: ::core::ffi::c_int) -> ::core::ffi::c_int;
    fn inflateEnd(strm: z_streamp) -> ::core::ffi::c_int;
    fn inflateInit_(
        strm: z_streamp,
        version: *const ::core::ffi::c_char,
        stream_size: ::core::ffi::c_int,
    ) -> ::core::ffi::c_int;
    fn xdv_index_new() -> *mut xdv_index;
    fn xdv_index_free(x: *mut xdv_index);
    fn xdv_index_update(
        x: *mut xdv_index,
        data: *const ::core::ffi::c_uchar,
        len: size_t,
    );
    fn xdv_index_page_count(x: *const xdv_index) -> ::core::ffi::c_int;
    fn xdv2pdf_new(resolver: xdv_resolver) -> *mut xdv2pdf;
    fn xdv2pdf_free(w: *mut xdv2pdf);
    fn xdv2pdf_write(
        w: *mut xdv2pdf,
        ranges: *const xdv_range,
        nranges: ::core::ffi::c_int,
        pdf: *mut pbuf,
        warnings: *mut pbuf,
    ) -> ::core::ffi::c_int;
    fn font_cache_new(resolver: xdv_resolver) -> *mut font_cache;
    fn font_cache_free(fc: *mut font_cache);
    fn font_native(
        fc: *mut font_cache,
        name: *const ::core::ffi::c_char,
        index: ::core::ffi::c_int,
        warnings: *mut pbuf,
    ) -> *mut native_face;
}
pub type size_t = usize;
pub type __uint32_t = u32;
pub type __off_t = ::core::ffi::c_long;
pub type __off64_t = ::core::ffi::c_long;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _IO_FILE {
    pub _flags: ::core::ffi::c_int,
    pub _IO_read_ptr: *mut ::core::ffi::c_char,
    pub _IO_read_end: *mut ::core::ffi::c_char,
    pub _IO_read_base: *mut ::core::ffi::c_char,
    pub _IO_write_base: *mut ::core::ffi::c_char,
    pub _IO_write_ptr: *mut ::core::ffi::c_char,
    pub _IO_write_end: *mut ::core::ffi::c_char,
    pub _IO_buf_base: *mut ::core::ffi::c_char,
    pub _IO_buf_end: *mut ::core::ffi::c_char,
    pub _IO_save_base: *mut ::core::ffi::c_char,
    pub _IO_backup_base: *mut ::core::ffi::c_char,
    pub _IO_save_end: *mut ::core::ffi::c_char,
    pub _markers: *mut _IO_marker,
    pub _chain: *mut _IO_FILE,
    pub _fileno: ::core::ffi::c_int,
    pub _flags2: ::core::ffi::c_int,
    pub _old_offset: __off_t,
    pub _cur_column: ::core::ffi::c_ushort,
    pub _vtable_offset: ::core::ffi::c_schar,
    pub _shortbuf: [::core::ffi::c_char; 1],
    pub _lock: *mut ::core::ffi::c_void,
    pub _offset: __off64_t,
    pub _codecvt: *mut _IO_codecvt,
    pub _wide_data: *mut _IO_wide_data,
    pub _freeres_list: *mut _IO_FILE,
    pub _freeres_buf: *mut ::core::ffi::c_void,
    pub __pad5: size_t,
    pub _mode: ::core::ffi::c_int,
    pub _unused2: [::core::ffi::c_char; 20],
}
pub type _IO_lock_t = ();
pub type FILE = _IO_FILE;
pub type Byte = ::core::ffi::c_uchar;
pub type uInt = ::core::ffi::c_uint;
pub type uLong = ::core::ffi::c_ulong;
pub type Bytef = Byte;
pub type voidpf = *mut ::core::ffi::c_void;
pub type alloc_func = Option<unsafe extern "C" fn(voidpf, uInt, uInt) -> voidpf>;
pub type free_func = Option<unsafe extern "C" fn(voidpf, voidpf) -> ()>;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct z_stream_s {
    pub next_in: *mut Bytef,
    pub avail_in: uInt,
    pub total_in: uLong,
    pub next_out: *mut Bytef,
    pub avail_out: uInt,
    pub total_out: uLong,
    pub msg: *mut ::core::ffi::c_char,
    pub state: *mut internal_state,
    pub zalloc: alloc_func,
    pub zfree: free_func,
    pub opaque: voidpf,
    pub data_type: ::core::ffi::c_int,
    pub adler: uLong,
    pub reserved: uLong,
}
pub type z_stream = z_stream_s;
pub type z_streamp = *mut z_stream;
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
pub type uint32_t = __uint32_t;
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
#[derive(Copy, Clone)]
#[repr(C)]
pub struct bb {
    pub d: *mut ::core::ffi::c_uchar,
    pub len: size_t,
    pub cap: size_t,
}
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<
    ::core::ffi::c_void,
>();
#[inline]
unsafe extern "C" fn pbuf_reserve(mut b: *mut pbuf, mut extra: size_t) {
    if (*b).len.wrapping_add(extra) <= (*b).cap {
        return;
    }
    let mut cap: size_t = if (*b).cap != 0 { (*b).cap } else { 256 as size_t };
    while cap < (*b).len.wrapping_add(extra) {
        cap = cap.wrapping_mul(2 as size_t);
    }
    let mut p: *mut ::core::ffi::c_uchar = realloc(
        (*b).data as *mut ::core::ffi::c_void,
        cap,
    ) as *mut ::core::ffi::c_uchar;
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
    memcpy((*b).data.offset((*b).len as isize) as *mut ::core::ffi::c_void, data, len);
    (*b).len = (*b).len.wrapping_add(len);
}
#[inline]
unsafe extern "C" fn pbuf_free(mut b: *mut pbuf) {
    free((*b).data as *mut ::core::ffi::c_void);
    (*b).data = ::core::ptr::null_mut::<::core::ffi::c_uchar>();
    (*b).cap = 0 as size_t;
    (*b).len = (*b).cap;
}
pub const ZLIB_VERSION: [::core::ffi::c_char; 4] = unsafe {
    ::core::mem::transmute::<[u8; 4], [::core::ffi::c_char; 4]>(*b"1.3\0")
};
pub const Z_OK: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
static mut failures: ::core::ffi::c_int = 0;
unsafe extern "C" fn put(
    mut b: *mut bb,
    mut p: *const ::core::ffi::c_void,
    mut n: size_t,
) {
    if (*b).len.wrapping_add(n) > (*b).cap {
        (*b).cap = (*b)
            .len
            .wrapping_add(n)
            .wrapping_mul(2 as size_t)
            .wrapping_add(64 as size_t);
        (*b).d = realloc((*b).d as *mut ::core::ffi::c_void, (*b).cap)
            as *mut ::core::ffi::c_uchar;
        if (*b).d.is_null() {
            abort();
        }
    }
    memcpy((*b).d.offset((*b).len as isize) as *mut ::core::ffi::c_void, p, n);
    (*b).len = (*b).len.wrapping_add(n);
}
unsafe extern "C" fn u8(mut b: *mut bb, mut v: ::core::ffi::c_int) {
    let mut c: ::core::ffi::c_uchar = v as ::core::ffi::c_uchar;
    put(b, &raw mut c as *const ::core::ffi::c_void, 1 as size_t);
}
unsafe extern "C" fn u16(mut b: *mut bb, mut v: ::core::ffi::c_uint) {
    u8(b, (v >> 8 as ::core::ffi::c_int) as ::core::ffi::c_int);
    u8(b, v as ::core::ffi::c_int);
}
unsafe extern "C" fn u32(mut b: *mut bb, mut v: ::core::ffi::c_uint) {
    u8(b, (v >> 24 as ::core::ffi::c_int) as ::core::ffi::c_int);
    u8(b, (v >> 16 as ::core::ffi::c_int) as ::core::ffi::c_int);
    u8(b, (v >> 8 as ::core::ffi::c_int) as ::core::ffi::c_int);
    u8(b, v as ::core::ffi::c_int);
}
unsafe extern "C" fn i32(mut b: *mut bb, mut v: ::core::ffi::c_int) {
    u32(b, v as ::core::ffi::c_uint);
}
unsafe extern "C" fn xxx(mut b: *mut bb, mut s: *const ::core::ffi::c_char) {
    u8(b, 242 as ::core::ffi::c_int);
    u32(b, strlen(s) as ::core::ffi::c_uint);
    put(b, s as *const ::core::ffi::c_void, strlen(s));
}
unsafe extern "C" fn xxx_truncated(
    mut b: *mut bb,
    mut s: *const ::core::ffi::c_char,
    mut real_len: size_t,
) {
    u8(b, 242 as ::core::ffi::c_int);
    u32(b, (real_len as ::core::ffi::c_uint).wrapping_add(64 as ::core::ffi::c_uint));
    put(b, s as *const ::core::ffi::c_void, real_len);
}
unsafe extern "C" fn pre(mut b: *mut bb) -> size_t {
    u8(b, 247 as ::core::ffi::c_int);
    u8(b, 7 as ::core::ffi::c_int);
    u32(b, 25400000 as ::core::ffi::c_int as ::core::ffi::c_uint);
    u32(b, 473628672 as ::core::ffi::c_int as ::core::ffi::c_uint);
    u32(b, 1000 as ::core::ffi::c_uint);
    let mut c: *const ::core::ffi::c_char = b"selftest\0" as *const u8
        as *const ::core::ffi::c_char;
    u8(b, strlen(c) as ::core::ffi::c_int);
    put(b, c as *const ::core::ffi::c_void, strlen(c));
    return (*b).len;
}
unsafe extern "C" fn bop(mut b: *mut bb) {
    u8(b, 139 as ::core::ffi::c_int);
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < 10 as ::core::ffi::c_int {
        u32(b, 0 as ::core::ffi::c_uint);
        i += 1;
    }
    i32(b, -(1 as ::core::ffi::c_int));
}
unsafe extern "C" fn eop(mut b: *mut bb) {
    u8(b, 140 as ::core::ffi::c_int);
}
unsafe extern "C" fn new_writer() -> *mut xdv2pdf {
    return xdv2pdf_new(xdv_resolver {
        env: NULL,
        load: None,
    });
}
unsafe extern "C" fn pdf_of(
    mut b: *mut bb,
    mut pdf: *mut pbuf,
    mut warn: *mut pbuf,
) -> ::core::ffi::c_int {
    let mut x: *mut xdv_index = xdv_index_new();
    xdv_index_update(x, (*b).d, (*b).len);
    let mut w: *mut xdv2pdf = new_writer();
    let mut r: xdv_range = xdv_range {
        data: (*b).d,
        len: (*b).len,
        index: x,
        first: 0 as ::core::ffi::c_int,
        count: xdv_index_page_count(x),
    };
    let mut err: ::core::ffi::c_int = xdv2pdf_write(
        w,
        &raw mut r,
        1 as ::core::ffi::c_int,
        pdf,
        warn,
    );
    xdv2pdf_free(w);
    xdv_index_free(x);
    return err;
}
unsafe extern "C" fn pdf_inflate_all(mut pdf: *mut pbuf, mut out: *mut pbuf) {
    let mut i: size_t = 0 as size_t;
    while i.wrapping_add(8 as size_t) < (*pdf).len {
        if !(memcmp(
            (*pdf).data.offset(i as isize) as *const ::core::ffi::c_void,
            b"stream\n\0" as *const u8 as *const ::core::ffi::c_char
                as *const ::core::ffi::c_void,
            7 as size_t,
        ) != 0)
        {
            let mut s: *const ::core::ffi::c_uchar = (*pdf)
                .data
                .offset(i as isize)
                .offset(7 as ::core::ffi::c_int as isize);
            let mut avail: size_t = (*pdf).len.wrapping_sub(i).wrapping_sub(7 as size_t);
            let mut es: *const ::core::ffi::c_uchar = memmem(
                s as *const ::core::ffi::c_void,
                avail,
                b"endstream\0" as *const u8 as *const ::core::ffi::c_char
                    as *const ::core::ffi::c_void,
                9 as size_t,
            ) as *const ::core::ffi::c_uchar;
            if es.is_null() {
                break;
            }
            avail = es.offset_from(s) as ::core::ffi::c_long as size_t;
            let mut tmp: [::core::ffi::c_uchar; 65536] = [0; 65536];
            let mut z: z_stream = z_stream_s {
                next_in: ::core::ptr::null_mut::<Bytef>(),
                avail_in: 0,
                total_in: 0,
                next_out: ::core::ptr::null_mut::<Bytef>(),
                avail_out: 0,
                total_out: 0,
                msg: ::core::ptr::null_mut::<::core::ffi::c_char>(),
                state: ::core::ptr::null_mut::<internal_state>(),
                zalloc: None,
                zfree: None,
                opaque: ::core::ptr::null_mut::<::core::ffi::c_void>(),
                data_type: 0,
                adler: 0,
                reserved: 0,
            };
            inflateInit_(
                &raw mut z,
                ZLIB_VERSION.as_ptr(),
                ::core::mem::size_of::<z_stream>() as ::core::ffi::c_int,
            );
            z.next_in = s as *mut ::core::ffi::c_uchar as *mut Bytef;
            z.avail_in = avail as ::core::ffi::c_uint as uInt;
            let mut rc: ::core::ffi::c_int = 0;
            loop {
                z.next_out = &raw mut tmp as *mut ::core::ffi::c_uchar as *mut Bytef;
                z.avail_out = ::core::mem::size_of::<[::core::ffi::c_uchar; 65536]>()
                    as uInt;
                rc = inflate(&raw mut z, 0 as ::core::ffi::c_int);
                pbuf_append(
                    out,
                    &raw mut tmp as *mut ::core::ffi::c_uchar
                        as *const ::core::ffi::c_void,
                    (::core::mem::size_of::<[::core::ffi::c_uchar; 65536]>() as size_t)
                        .wrapping_sub(z.avail_out as size_t),
                );
                if !(rc == Z_OK) {
                    break;
                }
            }
            inflateEnd(&raw mut z);
            i = i.wrapping_add(7 as size_t);
        }
        i = i.wrapping_add(1);
    }
}
unsafe extern "C" fn pdf_has(
    mut pdf: *mut pbuf,
    mut tok: *const ::core::ffi::c_char,
) -> ::core::ffi::c_int {
    let mut flat: pbuf = pbuf {
        data: ::core::ptr::null_mut::<::core::ffi::c_uchar>(),
        len: 0,
        cap: 0,
    };
    pbuf_append(&raw mut flat, (*pdf).data as *const ::core::ffi::c_void, (*pdf).len);
    pdf_inflate_all(pdf, &raw mut flat);
    let mut n: size_t = strlen(tok);
    let mut found: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut i: size_t = 0 as size_t;
    while i.wrapping_add(n) <= flat.len {
        if memcmp(
            flat.data.offset(i as isize) as *const ::core::ffi::c_void,
            tok as *const ::core::ffi::c_void,
            n,
        ) == 0
        {
            found = 1 as ::core::ffi::c_int;
        }
        i = i.wrapping_add(1);
    }
    pbuf_free(&raw mut flat);
    return found;
}
unsafe extern "C" fn pdf_mediabox(
    mut pdf: *mut pbuf,
    mut w: *mut ::core::ffi::c_double,
    mut h: *mut ::core::ffi::c_double,
) -> ::core::ffi::c_int {
    let mut flat: pbuf = pbuf {
        data: ::core::ptr::null_mut::<::core::ffi::c_uchar>(),
        len: 0,
        cap: 0,
    };
    pbuf_append(&raw mut flat, (*pdf).data as *const ::core::ffi::c_void, (*pdf).len);
    pdf_inflate_all(pdf, &raw mut flat);
    let mut key: *const ::core::ffi::c_char = b"/MediaBox[0 0 \0" as *const u8
        as *const ::core::ffi::c_char;
    let mut n: size_t = strlen(key);
    let mut ok: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut i: size_t = 0 as size_t;
    while i.wrapping_add(n) <= flat.len {
        if memcmp(
            flat.data.offset(i as isize) as *const ::core::ffi::c_void,
            key as *const ::core::ffi::c_void,
            n,
        ) == 0
        {
            let mut q: *mut ::core::ffi::c_char = ::core::ptr::null_mut::<
                ::core::ffi::c_char,
            >();
            let mut a: ::core::ffi::c_double = strtod(
                (flat.data as *mut ::core::ffi::c_char)
                    .offset(i as isize)
                    .offset(n as isize),
                &raw mut q,
            );
            if q
                != (flat.data as *mut ::core::ffi::c_char)
                    .offset(i as isize)
                    .offset(n as isize)
            {
                let mut b: ::core::ffi::c_double = strtod(q, &raw mut q);
                if q
                    != (flat.data as *mut ::core::ffi::c_char)
                        .offset(i as isize)
                        .offset(n as isize)
                {
                    *w = a;
                    *h = b;
                    ok = 1 as ::core::ffi::c_int;
                }
            }
            break;
        } else {
            i = i.wrapping_add(1);
        }
    }
    pbuf_free(&raw mut flat);
    return ok;
}
unsafe extern "C" fn test_special_no_bleed() {
    let mut b: bb = bb {
        d: ::core::ptr::null_mut::<::core::ffi::c_uchar>(),
        len: 0,
        cap: 0,
    };
    pre(&raw mut b);
    bop(&raw mut b);
    xxx(&raw mut b, b"x:scale 2 2\0" as *const u8 as *const ::core::ffi::c_char);
    u8(&raw mut b, 101 as ::core::ffi::c_int);
    u8(&raw mut b, 50 as ::core::ffi::c_int);
    u8(&raw mut b, 137 as ::core::ffi::c_int);
    i32(&raw mut b, 50000 as ::core::ffi::c_int);
    i32(&raw mut b, 50000 as ::core::ffi::c_int);
    eop(&raw mut b);
    let mut pdf: pbuf = pbuf {
        data: ::core::ptr::null_mut::<::core::ffi::c_uchar>(),
        len: 0,
        cap: 0,
    };
    let mut warn: pbuf = pbuf {
        data: ::core::ptr::null_mut::<::core::ffi::c_uchar>(),
        len: 0,
        cap: 0,
    };
    let mut err: ::core::ffi::c_int = pdf_of(&raw mut b, &raw mut pdf, &raw mut warn);
    if !(err == 0 as ::core::ffi::c_int && pdf.len > 200 as size_t) {
        fprintf(
            stderr,
            b"FAIL: %s\n\0" as *const u8 as *const ::core::ffi::c_char,
            b"scale-special next-opcode bleed renders\0" as *const u8
                as *const ::core::ffi::c_char,
        );
        failures += 1;
    } else {
        fprintf(
            stderr,
            b"ok: %s\n\0" as *const u8 as *const ::core::ffi::c_char,
            b"scale-special next-opcode bleed renders\0" as *const u8
                as *const ::core::ffi::c_char,
        );
    }
    if pdf_has(&raw mut pdf, b"2 0 0 2 \0" as *const u8 as *const ::core::ffi::c_char)
        == 0
    {
        fprintf(
            stderr,
            b"FAIL: %s\n\0" as *const u8 as *const ::core::ffi::c_char,
            b"x:scale emitted ~2 transform\0" as *const u8 as *const ::core::ffi::c_char,
        );
        failures += 1;
    } else {
        fprintf(
            stderr,
            b"ok: %s\n\0" as *const u8 as *const ::core::ffi::c_char,
            b"x:scale emitted ~2 transform\0" as *const u8 as *const ::core::ffi::c_char,
        );
    }
    if pdf_has(&raw mut pdf, b"200.00000\0" as *const u8 as *const ::core::ffi::c_char)
        != 0
    {
        fprintf(
            stderr,
            b"FAIL: %s\n\0" as *const u8 as *const ::core::ffi::c_char,
            b"x:scale did not consume next opcode\0" as *const u8
                as *const ::core::ffi::c_char,
        );
        failures += 1;
    } else {
        fprintf(
            stderr,
            b"ok: %s\n\0" as *const u8 as *const ::core::ffi::c_char,
            b"x:scale did not consume next opcode\0" as *const u8
                as *const ::core::ffi::c_char,
        );
    }
    pbuf_free(&raw mut pdf);
    pbuf_free(&raw mut warn);
    free(b.d as *mut ::core::ffi::c_void);
}
unsafe extern "C" fn test_truncated_special() {
    let mut b: bb = bb {
        d: ::core::ptr::null_mut::<::core::ffi::c_uchar>(),
        len: 0,
        cap: 0,
    };
    pre(&raw mut b);
    bop(&raw mut b);
    u8(&raw mut b, DVI_RIGHT1 as ::core::ffi::c_int);
    u8(&raw mut b, 10 as ::core::ffi::c_int);
    xxx_truncated(
        &raw mut b,
        b"x:gsave\0" as *const u8 as *const ::core::ffi::c_char,
        7 as size_t,
    );
    eop(&raw mut b);
    let mut pdf: pbuf = pbuf {
        data: ::core::ptr::null_mut::<::core::ffi::c_uchar>(),
        len: 0,
        cap: 0,
    };
    let mut warn: pbuf = pbuf {
        data: ::core::ptr::null_mut::<::core::ffi::c_uchar>(),
        len: 0,
        cap: 0,
    };
    let mut err: ::core::ffi::c_int = pdf_of(&raw mut b, &raw mut pdf, &raw mut warn);
    if !(err == 0 as ::core::ffi::c_int || err != 0 as ::core::ffi::c_int) {
        fprintf(
            stderr,
            b"FAIL: %s\n\0" as *const u8 as *const ::core::ffi::c_char,
            b"truncated XXX did not crash\0" as *const u8 as *const ::core::ffi::c_char,
        );
        failures += 1;
    } else {
        fprintf(
            stderr,
            b"ok: %s\n\0" as *const u8 as *const ::core::ffi::c_char,
            b"truncated XXX did not crash\0" as *const u8 as *const ::core::ffi::c_char,
        );
    }
    pbuf_free(&raw mut pdf);
    pbuf_free(&raw mut warn);
    free(b.d as *mut ::core::ffi::c_void);
}
unsafe extern "C" fn test_pagesize_boundary() {
    let mut b: bb = bb {
        d: ::core::ptr::null_mut::<::core::ffi::c_uchar>(),
        len: 0,
        cap: 0,
    };
    pre(&raw mut b);
    bop(&raw mut b);
    xxx(&raw mut b, b"pdf:pagesize wid\0" as *const u8 as *const ::core::ffi::c_char);
    eop(&raw mut b);
    let mut pdf: pbuf = pbuf {
        data: ::core::ptr::null_mut::<::core::ffi::c_uchar>(),
        len: 0,
        cap: 0,
    };
    let mut warn: pbuf = pbuf {
        data: ::core::ptr::null_mut::<::core::ffi::c_uchar>(),
        len: 0,
        cap: 0,
    };
    let mut err: ::core::ffi::c_int = pdf_of(&raw mut b, &raw mut pdf, &raw mut warn);
    if !(err == 0 as ::core::ffi::c_int) {
        fprintf(
            stderr,
            b"FAIL: %s\n\0" as *const u8 as *const ::core::ffi::c_char,
            b"boundary pdf:pagesize renders\0" as *const u8 as *const ::core::ffi::c_char,
        );
        failures += 1;
    } else {
        fprintf(
            stderr,
            b"ok: %s\n\0" as *const u8 as *const ::core::ffi::c_char,
            b"boundary pdf:pagesize renders\0" as *const u8 as *const ::core::ffi::c_char,
        );
    }
    let mut w: ::core::ffi::c_double = 0 as ::core::ffi::c_int as ::core::ffi::c_double;
    let mut h: ::core::ffi::c_double = 0 as ::core::ffi::c_int as ::core::ffi::c_double;
    if !(pdf_mediabox(&raw mut pdf, &raw mut w, &raw mut h) != 0
        && w > 590 as ::core::ffi::c_int as ::core::ffi::c_double
        && w < 600 as ::core::ffi::c_int as ::core::ffi::c_double)
    {
        fprintf(
            stderr,
            b"FAIL: %s\n\0" as *const u8 as *const ::core::ffi::c_char,
            b"boundary pagesize kept default size\0" as *const u8
                as *const ::core::ffi::c_char,
        );
        failures += 1;
    } else {
        fprintf(
            stderr,
            b"ok: %s\n\0" as *const u8 as *const ::core::ffi::c_char,
            b"boundary pagesize kept default size\0" as *const u8
                as *const ::core::ffi::c_char,
        );
    }
    pbuf_free(&raw mut pdf);
    pbuf_free(&raw mut warn);
    free(b.d as *mut ::core::ffi::c_void);
    b = bb {
        d: ::core::ptr::null_mut::<::core::ffi::c_uchar>(),
        len: 0,
        cap: 0,
    };
    pre(&raw mut b);
    bop(&raw mut b);
    xxx(
        &raw mut b,
        b"pdf:pagesize width 100mm height 50mm\0" as *const u8
            as *const ::core::ffi::c_char,
    );
    eop(&raw mut b);
    memset(
        &raw mut pdf as *mut ::core::ffi::c_void,
        0 as ::core::ffi::c_int,
        ::core::mem::size_of::<pbuf>() as size_t,
    );
    memset(
        &raw mut warn as *mut ::core::ffi::c_void,
        0 as ::core::ffi::c_int,
        ::core::mem::size_of::<pbuf>() as size_t,
    );
    err = pdf_of(&raw mut b, &raw mut pdf, &raw mut warn);
    h = 0 as ::core::ffi::c_int as ::core::ffi::c_double;
    w = h;
    if !(err == 0 as ::core::ffi::c_int
        && pdf_mediabox(&raw mut pdf, &raw mut w, &raw mut h) != 0 && w > 283.0f64
        && w < 283.9f64 && h > 141.3f64 && h < 142.2f64)
    {
        fprintf(
            stderr,
            b"FAIL: %s\n\0" as *const u8 as *const ::core::ffi::c_char,
            b"explicit width/height honored (100mm\xE2\x86\x92283.46bp, 50mm\xE2\x86\x92141.73bp)\0"
                as *const u8 as *const ::core::ffi::c_char,
        );
        failures += 1;
    } else {
        fprintf(
            stderr,
            b"ok: %s\n\0" as *const u8 as *const ::core::ffi::c_char,
            b"explicit width/height honored (100mm\xE2\x86\x92283.46bp, 50mm\xE2\x86\x92141.73bp)\0"
                as *const u8 as *const ::core::ffi::c_char,
        );
    }
    pbuf_free(&raw mut pdf);
    pbuf_free(&raw mut warn);
    free(b.d as *mut ::core::ffi::c_void);
}
unsafe extern "C" fn load_font(
    mut env: *mut ::core::ffi::c_void,
    mut name: *const ::core::ffi::c_char,
    mut kind: xdv_res_kind,
) -> *mut tbuf {
    static mut CMAP_LEN: ::core::ffi::c_int = 12 as ::core::ffi::c_int;
    let mut total: size_t = ((12 as ::core::ffi::c_int + 16 as ::core::ffi::c_int)
        as size_t)
        .wrapping_add(CMAP_LEN as size_t);
    let mut d: *mut ::core::ffi::c_uchar = calloc(1 as size_t, total)
        as *mut ::core::ffi::c_uchar;
    if d.is_null() {
        abort();
    }
    *d.offset(1 as ::core::ffi::c_int as isize) = 1 as ::core::ffi::c_uchar;
    *d.offset(5 as ::core::ffi::c_int as isize) = 1 as ::core::ffi::c_uchar;
    let mut r: *mut ::core::ffi::c_uchar = d.offset(12 as ::core::ffi::c_int as isize);
    memcpy(
        r as *mut ::core::ffi::c_void,
        b"cmap\0" as *const u8 as *const ::core::ffi::c_char
            as *const ::core::ffi::c_void,
        4 as size_t,
    );
    *r.offset(11 as ::core::ffi::c_int as isize) = 28 as ::core::ffi::c_uchar;
    *r.offset(15 as ::core::ffi::c_int as isize) = CMAP_LEN as ::core::ffi::c_uchar;
    let mut c: *mut ::core::ffi::c_uchar = d.offset(28 as ::core::ffi::c_int as isize);
    *c.offset(2 as ::core::ffi::c_int as isize) = 0 as ::core::ffi::c_uchar;
    *c.offset(3 as ::core::ffi::c_int as isize) = 1 as ::core::ffi::c_uchar;
    *c.offset(5 as ::core::ffi::c_int as isize) = 3 as ::core::ffi::c_uchar;
    *c.offset(7 as ::core::ffi::c_int as isize) = 1 as ::core::ffi::c_uchar;
    *c.offset(6 as ::core::ffi::c_int as isize) = 0xff as ::core::ffi::c_uchar;
    *c.offset(7 as ::core::ffi::c_int as isize) = 0xff as ::core::ffi::c_uchar;
    *c.offset(8 as ::core::ffi::c_int as isize) = 0xff as ::core::ffi::c_uchar;
    *c.offset(9 as ::core::ffi::c_int as isize) = 0xfc as ::core::ffi::c_uchar;
    let mut t: *mut tbuf = tbuf_new(total as ::core::ffi::c_int as size_t);
    tbuf_append(t, d as *const ::core::ffi::c_void, total);
    free(d as *mut ::core::ffi::c_void);
    return t;
}
unsafe extern "C" fn test_corrupt_cmap() {
    let mut fc: *mut font_cache = font_cache_new(xdv_resolver {
        env: NULL,
        load: Some(
            load_font
                as unsafe extern "C" fn(
                    *mut ::core::ffi::c_void,
                    *const ::core::ffi::c_char,
                    xdv_res_kind,
                ) -> *mut tbuf,
        ),
    });
    let mut warn: pbuf = pbuf {
        data: ::core::ptr::null_mut::<::core::ffi::c_uchar>(),
        len: 0,
        cap: 0,
    };
    let mut f: *mut native_face = font_native(
        fc,
        b"corrupt.ttf\0" as *const u8 as *const ::core::ffi::c_char,
        -(1 as ::core::ffi::c_int),
        &raw mut warn,
    );
    if !(!f.is_null() && (*f).ok as ::core::ffi::c_int != 0) {
        fprintf(
            stderr,
            b"FAIL: %s\n\0" as *const u8 as *const ::core::ffi::c_char,
            b"corrupt cmap tolerated, face usable\0" as *const u8
                as *const ::core::ffi::c_char,
        );
        failures += 1;
    } else {
        fprintf(
            stderr,
            b"ok: %s\n\0" as *const u8 as *const ::core::ffi::c_char,
            b"corrupt cmap tolerated, face usable\0" as *const u8
                as *const ::core::ffi::c_char,
        );
    }
    if !(!f.is_null() && (*f).num_glyphs > 0 as ::core::ffi::c_int) {
        fprintf(
            stderr,
            b"FAIL: %s\n\0" as *const u8 as *const ::core::ffi::c_char,
            b"face still has glyphs\0" as *const u8 as *const ::core::ffi::c_char,
        );
        failures += 1;
    } else {
        fprintf(
            stderr,
            b"ok: %s\n\0" as *const u8 as *const ::core::ffi::c_char,
            b"face still has glyphs\0" as *const u8 as *const ::core::ffi::c_char,
        );
    }
    pbuf_free(&raw mut warn);
    font_cache_free(fc);
}
unsafe extern "C" fn load_font_ok(
    mut env: *mut ::core::ffi::c_void,
    mut name: *const ::core::ffi::c_char,
    mut kind: xdv_res_kind,
) -> *mut tbuf {
    let cmap_len: ::core::ffi::c_int = 4 as ::core::ffi::c_int + 8 as ::core::ffi::c_int
        + 32 as ::core::ffi::c_int;
    let mut d: *mut ::core::ffi::c_uchar = calloc(
        1 as size_t,
        ((12 as ::core::ffi::c_int + 16 as ::core::ffi::c_int) as size_t)
            .wrapping_add(cmap_len as size_t)
            .wrapping_add(8 as size_t),
    ) as *mut ::core::ffi::c_uchar;
    if d.is_null() {
        abort();
    }
    *d.offset(1 as ::core::ffi::c_int as isize) = 1 as ::core::ffi::c_uchar;
    *d.offset(5 as ::core::ffi::c_int as isize) = 1 as ::core::ffi::c_uchar;
    let mut r: *mut ::core::ffi::c_uchar = d.offset(12 as ::core::ffi::c_int as isize);
    memcpy(
        r as *mut ::core::ffi::c_void,
        b"cmap\0" as *const u8 as *const ::core::ffi::c_char
            as *const ::core::ffi::c_void,
        4 as size_t,
    );
    *r.offset(11 as ::core::ffi::c_int as isize) = 28 as ::core::ffi::c_uchar;
    let mut cl: ::core::ffi::c_uint = cmap_len as ::core::ffi::c_uint;
    *r.offset(12 as ::core::ffi::c_int as isize) = (cl >> 24 as ::core::ffi::c_int
        & 255 as ::core::ffi::c_uint) as ::core::ffi::c_uchar;
    *r.offset(13 as ::core::ffi::c_int as isize) = (cl >> 16 as ::core::ffi::c_int
        & 255 as ::core::ffi::c_uint) as ::core::ffi::c_uchar;
    *r.offset(14 as ::core::ffi::c_int as isize) = (cl >> 8 as ::core::ffi::c_int
        & 255 as ::core::ffi::c_uint) as ::core::ffi::c_uchar;
    *r.offset(15 as ::core::ffi::c_int as isize) = (cl & 255 as ::core::ffi::c_uint)
        as ::core::ffi::c_uchar;
    let mut c: *mut ::core::ffi::c_uchar = d.offset(28 as ::core::ffi::c_int as isize);
    *c.offset(2 as ::core::ffi::c_int as isize) = 0 as ::core::ffi::c_uchar;
    *c.offset(3 as ::core::ffi::c_int as isize) = 1 as ::core::ffi::c_uchar;
    *c.offset(5 as ::core::ffi::c_int as isize) = 3 as ::core::ffi::c_uchar;
    *c.offset(7 as ::core::ffi::c_int as isize) = 1 as ::core::ffi::c_uchar;
    *c.offset(8 as ::core::ffi::c_int as isize) = 0 as ::core::ffi::c_uchar;
    *c.offset(9 as ::core::ffi::c_int as isize) = 0 as ::core::ffi::c_uchar;
    *c.offset(10 as ::core::ffi::c_int as isize) = 0 as ::core::ffi::c_uchar;
    *c.offset(11 as ::core::ffi::c_int as isize) = 12 as ::core::ffi::c_uchar;
    let mut s4: *mut ::core::ffi::c_uchar = c.offset(12 as ::core::ffi::c_int as isize);
    *s4.offset(0 as ::core::ffi::c_int as isize) = 0 as ::core::ffi::c_uchar;
    *s4.offset(1 as ::core::ffi::c_int as isize) = 4 as ::core::ffi::c_uchar;
    let mut s4len: ::core::ffi::c_uint = 32 as ::core::ffi::c_uint;
    *s4.offset(2 as ::core::ffi::c_int as isize) = (s4len >> 8 as ::core::ffi::c_int
        & 255 as ::core::ffi::c_uint) as ::core::ffi::c_uchar;
    *s4.offset(3 as ::core::ffi::c_int as isize) = (s4len & 255 as ::core::ffi::c_uint)
        as ::core::ffi::c_uchar;
    *s4.offset(4 as ::core::ffi::c_int as isize) = 0 as ::core::ffi::c_uchar;
    *s4.offset(5 as ::core::ffi::c_int as isize) = 0 as ::core::ffi::c_uchar;
    *s4.offset(6 as ::core::ffi::c_int as isize) = 0 as ::core::ffi::c_uchar;
    *s4.offset(7 as ::core::ffi::c_int as isize) = 4 as ::core::ffi::c_uchar;
    let mut ends: *mut ::core::ffi::c_uchar = s4
        .offset(14 as ::core::ffi::c_int as isize);
    let mut starts: *mut ::core::ffi::c_uchar = ends
        .offset(4 as ::core::ffi::c_int as isize)
        .offset(2 as ::core::ffi::c_int as isize);
    let mut deltas: *mut ::core::ffi::c_uchar = starts
        .offset(4 as ::core::ffi::c_int as isize);
    let mut ranges: *mut ::core::ffi::c_uchar = deltas
        .offset(4 as ::core::ffi::c_int as isize);
    *ends.offset(0 as ::core::ffi::c_int as isize) = 0 as ::core::ffi::c_uchar;
    *ends.offset(1 as ::core::ffi::c_int as isize) = 32 as ::core::ffi::c_uchar;
    *ends.offset(2 as ::core::ffi::c_int as isize) = 0xff as ::core::ffi::c_uchar;
    *ends.offset(3 as ::core::ffi::c_int as isize) = 0xff as ::core::ffi::c_uchar;
    *starts.offset(0 as ::core::ffi::c_int as isize) = 0 as ::core::ffi::c_uchar;
    *starts.offset(1 as ::core::ffi::c_int as isize) = 32 as ::core::ffi::c_uchar;
    *starts.offset(2 as ::core::ffi::c_int as isize) = 0xff as ::core::ffi::c_uchar;
    *starts.offset(3 as ::core::ffi::c_int as isize) = 0xff as ::core::ffi::c_uchar;
    let mut d0: ::core::ffi::c_int = -(31 as ::core::ffi::c_int)
        & 0xffff as ::core::ffi::c_int;
    *deltas.offset(0 as ::core::ffi::c_int as isize) = (d0 >> 8 as ::core::ffi::c_int
        & 255 as ::core::ffi::c_int) as ::core::ffi::c_uchar;
    *deltas.offset(1 as ::core::ffi::c_int as isize) = (d0 & 255 as ::core::ffi::c_int)
        as ::core::ffi::c_uchar;
    *deltas.offset(2 as ::core::ffi::c_int as isize) = 0 as ::core::ffi::c_uchar;
    *deltas.offset(3 as ::core::ffi::c_int as isize) = 1 as ::core::ffi::c_uchar;
    let mut t: *mut tbuf = tbuf_new(64 as size_t);
    tbuf_append(
        t,
        d as *const ::core::ffi::c_void,
        ((12 as ::core::ffi::c_int + 16 as ::core::ffi::c_int) as size_t)
            .wrapping_add(cmap_len as size_t)
            .wrapping_add(8 as size_t),
    );
    free(d as *mut ::core::ffi::c_void);
    return t;
}
unsafe extern "C" fn test_valid_cmap() {
    let mut fc: *mut font_cache = font_cache_new(xdv_resolver {
        env: NULL,
        load: Some(
            load_font_ok
                as unsafe extern "C" fn(
                    *mut ::core::ffi::c_void,
                    *const ::core::ffi::c_char,
                    xdv_res_kind,
                ) -> *mut tbuf,
        ),
    });
    let mut warn: pbuf = pbuf {
        data: ::core::ptr::null_mut::<::core::ffi::c_uchar>(),
        len: 0,
        cap: 0,
    };
    let mut f: *mut native_face = font_native(
        fc,
        b"ok.ttf\0" as *const u8 as *const ::core::ffi::c_char,
        -(1 as ::core::ffi::c_int),
        &raw mut warn,
    );
    if warn.len != 0 {
        fprintf(
            stderr,
            b"warn: %.*s\0" as *const u8 as *const ::core::ffi::c_char,
            warn.len as ::core::ffi::c_int,
            warn.data,
        );
    }
    if !(!f.is_null() && (*f).ok as ::core::ffi::c_int != 0) {
        fprintf(
            stderr,
            b"FAIL: %s\n\0" as *const u8 as *const ::core::ffi::c_char,
            b"valid font parses\0" as *const u8 as *const ::core::ffi::c_char,
        );
        failures += 1;
    } else {
        fprintf(
            stderr,
            b"ok: %s\n\0" as *const u8 as *const ::core::ffi::c_char,
            b"valid font parses\0" as *const u8 as *const ::core::ffi::c_char,
        );
    }
    if !(!f.is_null() && (*f).num_glyphs > 0 as ::core::ffi::c_int) {
        fprintf(
            stderr,
            b"FAIL: %s\n\0" as *const u8 as *const ::core::ffi::c_char,
            b"valid font has glyphs\0" as *const u8 as *const ::core::ffi::c_char,
        );
        failures += 1;
    } else {
        fprintf(
            stderr,
            b"ok: %s\n\0" as *const u8 as *const ::core::ffi::c_char,
            b"valid font has glyphs\0" as *const u8 as *const ::core::ffi::c_char,
        );
    }
    if !(!f.is_null()
        && *(*f).to_unicode.offset(1 as ::core::ffi::c_int as isize) == 32 as uint32_t)
    {
        fprintf(
            stderr,
            b"FAIL: %s\n\0" as *const u8 as *const ::core::ffi::c_char,
            b"valid cmap maps space to gid 1\0" as *const u8
                as *const ::core::ffi::c_char,
        );
        failures += 1;
    } else {
        fprintf(
            stderr,
            b"ok: %s\n\0" as *const u8 as *const ::core::ffi::c_char,
            b"valid cmap maps space to gid 1\0" as *const u8
                as *const ::core::ffi::c_char,
        );
    }
    pbuf_free(&raw mut warn);
    font_cache_free(fc);
}
unsafe extern "C" fn load_script_font(
    mut env: *mut ::core::ffi::c_void,
    mut name: *const ::core::ffi::c_char,
    mut kind: xdv_res_kind,
) -> *mut tbuf {
    let mut base: *mut tbuf = load_font_ok(NULL, name, kind);
    let mut variant: ::core::ffi::c_int = *(env as *mut ::core::ffi::c_int);
    let mut g: bb = bb {
        d: ::core::ptr::null_mut::<::core::ffi::c_uchar>(),
        len: 0,
        cap: 0,
    };
    u16(&raw mut g, 1 as ::core::ffi::c_uint);
    u16(&raw mut g, 0 as ::core::ffi::c_uint);
    u16(&raw mut g, 0 as ::core::ffi::c_uint);
    u16(&raw mut g, 10 as ::core::ffi::c_uint);
    u16(&raw mut g, 24 as ::core::ffi::c_uint);
    u16(&raw mut g, 1 as ::core::ffi::c_uint);
    put(
        &raw mut g,
        b"ssty\0" as *const u8 as *const ::core::ffi::c_char
            as *const ::core::ffi::c_void,
        4 as size_t,
    );
    u16(&raw mut g, 8 as ::core::ffi::c_uint);
    u16(&raw mut g, 0 as ::core::ffi::c_uint);
    u16(&raw mut g, 1 as ::core::ffi::c_uint);
    u16(&raw mut g, 0 as ::core::ffi::c_uint);
    u16(&raw mut g, 1 as ::core::ffi::c_uint);
    u16(&raw mut g, 4 as ::core::ffi::c_uint);
    let mut type_0: ::core::ffi::c_uint = (if variant == 4 as ::core::ffi::c_int
        || variant == 5 as ::core::ffi::c_int
    {
        1 as ::core::ffi::c_int
    } else {
        3 as ::core::ffi::c_int
    }) as ::core::ffi::c_uint;
    let mut extension: ::core::ffi::c_int = (variant == 2 as ::core::ffi::c_int
        || variant == 6 as ::core::ffi::c_int) as ::core::ffi::c_int;
    u16(&raw mut g, if extension != 0 { 7 as ::core::ffi::c_uint } else { type_0 });
    u16(&raw mut g, 0 as ::core::ffi::c_uint);
    u16(&raw mut g, 1 as ::core::ffi::c_uint);
    u16(&raw mut g, 8 as ::core::ffi::c_uint);
    if extension != 0 {
        u16(&raw mut g, 1 as ::core::ffi::c_uint);
        u16(&raw mut g, type_0);
        u32(
            &raw mut g,
            if variant == 6 as ::core::ffi::c_int {
                0xfffffffc as ::core::ffi::c_uint
            } else {
                8 as ::core::ffi::c_uint
            },
        );
    }
    u16(
        &raw mut g,
        (if variant == 5 as ::core::ffi::c_int {
            2 as ::core::ffi::c_int
        } else {
            1 as ::core::ffi::c_int
        }) as ::core::ffi::c_uint,
    );
    u16(
        &raw mut g,
        (if variant == 4 as ::core::ffi::c_int {
            6 as ::core::ffi::c_int
        } else if variant == 5 as ::core::ffi::c_int {
            8 as ::core::ffi::c_int
        } else {
            12 as ::core::ffi::c_int
        }) as ::core::ffi::c_uint,
    );
    u16(&raw mut g, 1 as ::core::ffi::c_uint);
    if variant == 5 as ::core::ffi::c_int {
        u16(&raw mut g, 2 as ::core::ffi::c_uint);
    } else if variant != 4 as ::core::ffi::c_int {
        u16(
            &raw mut g,
            (if variant == 7 as ::core::ffi::c_int {
                0xffff as ::core::ffi::c_int
            } else {
                8 as ::core::ffi::c_int
            }) as ::core::ffi::c_uint,
        );
        u16(&raw mut g, 1 as ::core::ffi::c_uint);
        u16(&raw mut g, 2 as ::core::ffi::c_uint);
    }
    u16(
        &raw mut g,
        (if variant == 3 as ::core::ffi::c_int {
            2 as ::core::ffi::c_int
        } else {
            1 as ::core::ffi::c_int
        }) as ::core::ffi::c_uint,
    );
    u16(
        &raw mut g,
        (if variant == 8 as ::core::ffi::c_int {
            0xffff as ::core::ffi::c_int
        } else {
            1 as ::core::ffi::c_int
        }) as ::core::ffi::c_uint,
    );
    u16(&raw mut g, 1 as ::core::ffi::c_uint);
    if variant == 3 as ::core::ffi::c_int {
        u16(&raw mut g, 1 as ::core::ffi::c_uint);
        u16(&raw mut g, 0 as ::core::ffi::c_uint);
    }
    if variant == 1 as ::core::ffi::c_int {
        let ref mut fresh0 = *g.d.offset(7 as ::core::ffi::c_int as isize);
        *fresh0 = 0xff as ::core::ffi::c_uchar;
        *g.d.offset(6 as ::core::ffi::c_int as isize) = *fresh0;
    }
    let mut b: bb = bb {
        d: ::core::ptr::null_mut::<::core::ffi::c_uchar>(),
        len: 0,
        cap: 0,
    };
    put(&raw mut b, (*base).data as *const ::core::ffi::c_void, 12 as size_t);
    *b.d.offset(5 as ::core::ffi::c_int as isize) = 2 as ::core::ffi::c_uchar;
    put(
        &raw mut b,
        b"cmap\0" as *const u8 as *const ::core::ffi::c_char
            as *const ::core::ffi::c_void,
        4 as size_t,
    );
    u32(&raw mut b, 0 as ::core::ffi::c_uint);
    u32(&raw mut b, 44 as ::core::ffi::c_uint);
    u32(&raw mut b, 44 as ::core::ffi::c_uint);
    put(
        &raw mut b,
        b"GSUB\0" as *const u8 as *const ::core::ffi::c_char
            as *const ::core::ffi::c_void,
        4 as size_t,
    );
    u32(&raw mut b, 0 as ::core::ffi::c_uint);
    u32(&raw mut b, 88 as ::core::ffi::c_uint);
    u32(&raw mut b, g.len as ::core::ffi::c_uint);
    put(
        &raw mut b,
        (*base).data.offset(28 as ::core::ffi::c_int as isize)
            as *const ::core::ffi::c_void,
        44 as size_t,
    );
    put(&raw mut b, g.d as *const ::core::ffi::c_void, g.len);
    let mut result: *mut tbuf = tbuf_from_copy(b.d as *const ::core::ffi::c_void, b.len);
    tbuf_drop(base);
    free(b.d as *mut ::core::ffi::c_void);
    free(g.d as *mut ::core::ffi::c_void);
    return result;
}
unsafe extern "C" fn test_script_unicode() {
    let names: [*const ::core::ffi::c_char; 9] = [
        b"alternate substitution\0" as *const u8 as *const ::core::ffi::c_char,
        b"corrupt feature offset\0" as *const u8 as *const ::core::ffi::c_char,
        b"extension substitution\0" as *const u8 as *const ::core::ffi::c_char,
        b"coverage range\0" as *const u8 as *const ::core::ffi::c_char,
        b"single delta substitution\0" as *const u8 as *const ::core::ffi::c_char,
        b"single array substitution\0" as *const u8 as *const ::core::ffi::c_char,
        b"corrupt extension offset\0" as *const u8 as *const ::core::ffi::c_char,
        b"corrupt alternate offset\0" as *const u8 as *const ::core::ffi::c_char,
        b"corrupt coverage count\0" as *const u8 as *const ::core::ffi::c_char,
    ];
    let mut variant: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while variant < 9 as ::core::ffi::c_int {
        let mut fc: *mut font_cache = font_cache_new(xdv_resolver {
            env: &raw mut variant as *mut ::core::ffi::c_void,
            load: Some(
                load_script_font
                    as unsafe extern "C" fn(
                        *mut ::core::ffi::c_void,
                        *const ::core::ffi::c_char,
                        xdv_res_kind,
                    ) -> *mut tbuf,
            ),
        });
        let mut warn: pbuf = pbuf {
            data: ::core::ptr::null_mut::<::core::ffi::c_uchar>(),
            len: 0,
            cap: 0,
        };
        let mut f: *mut native_face = font_native(
            fc,
            b"script.ttf\0" as *const u8 as *const ::core::ffi::c_char,
            -(1 as ::core::ffi::c_int),
            &raw mut warn,
        );
        let mut valid: ::core::ffi::c_int = (variant != 1 as ::core::ffi::c_int
            && variant < 6 as ::core::ffi::c_int) as ::core::ffi::c_int;
        if !(!f.is_null() && (*f).ok as ::core::ffi::c_int != 0
            && *(*f).to_unicode.offset(1 as ::core::ffi::c_int as isize)
                == 32 as uint32_t)
        {
            fprintf(
                stderr,
                b"FAIL: %s\n\0" as *const u8 as *const ::core::ffi::c_char,
                b"script font retains base cmap\0" as *const u8
                    as *const ::core::ffi::c_char,
            );
            failures += 1;
        } else {
            fprintf(
                stderr,
                b"ok: %s\n\0" as *const u8 as *const ::core::ffi::c_char,
                b"script font retains base cmap\0" as *const u8
                    as *const ::core::ffi::c_char,
            );
        }
        if !(!f.is_null()
            && *(*f).to_unicode.offset(2 as ::core::ffi::c_int as isize)
                == (if valid != 0 {
                    32 as ::core::ffi::c_int
                } else {
                    0 as ::core::ffi::c_int
                }) as uint32_t)
        {
            fprintf(
                stderr,
                b"FAIL: %s\n\0" as *const u8 as *const ::core::ffi::c_char,
                names[variant as usize],
            );
            failures += 1;
        } else {
            fprintf(
                stderr,
                b"ok: %s\n\0" as *const u8 as *const ::core::ffi::c_char,
                names[variant as usize],
            );
        }
        pbuf_free(&raw mut warn);
        font_cache_free(fc);
        variant += 1;
    }
}
unsafe fn main_0() -> ::core::ffi::c_int {
    test_special_no_bleed();
    test_truncated_special();
    test_pagesize_boundary();
    test_corrupt_cmap();
    test_valid_cmap();
    test_script_unicode();
    if failures != 0 {
        fprintf(
            stderr,
            b"%d failures\n\0" as *const u8 as *const ::core::ffi::c_char,
            failures,
        );
    } else {
        fprintf(
            stderr,
            b"all boundary regressions pass\n\0" as *const u8
                as *const ::core::ffi::c_char,
        );
    }
    return if failures != 0 { 1 as ::core::ffi::c_int } else { 0 as ::core::ffi::c_int };
}
pub fn main() {
    unsafe { ::std::process::exit(main_0() as i32) }
}
