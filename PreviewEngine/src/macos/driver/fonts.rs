/* Pitex embedded preview engine — font file readers for PDF embedding.
 * Independently written from the OpenType/TrueType, TFM/VF, Type 1 and
 * pdfTeX font map format descriptions. Pitex-authored (AGPL-3.0-or-later). */
// Translated from driver/fonts.c with C2Rust 0.22.1.
extern "C" {
    fn malloc(__size: size_t) -> *mut ::core::ffi::c_void;
    fn calloc(__count: size_t, __size: size_t) -> *mut ::core::ffi::c_void;
    fn free(_: *mut ::core::ffi::c_void);
    fn realloc(__ptr: *mut ::core::ffi::c_void, __size: size_t) -> *mut ::core::ffi::c_void;
    fn abort() -> !;
    fn strtod(
        _: *const ::core::ffi::c_char,
        _: *mut *mut ::core::ffi::c_char,
    ) -> ::core::ffi::c_double;
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
    fn strchr(__s: *const ::core::ffi::c_char, __c: ::core::ffi::c_int)
        -> *mut ::core::ffi::c_char;
    fn strcmp(
        __s1: *const ::core::ffi::c_char,
        __s2: *const ::core::ffi::c_char,
    ) -> ::core::ffi::c_int;
    fn strlen(__s: *const ::core::ffi::c_char) -> size_t;
    fn strrchr(
        __s: *const ::core::ffi::c_char,
        __c: ::core::ffi::c_int,
    ) -> *mut ::core::ffi::c_char;
    fn strstr(
        __big: *const ::core::ffi::c_char,
        __little: *const ::core::ffi::c_char,
    ) -> *mut ::core::ffi::c_char;
    fn strdup(__s1: *const ::core::ffi::c_char) -> *mut ::core::ffi::c_char;
    fn strndup(__s1: *const ::core::ffi::c_char, __n: size_t) -> *mut ::core::ffi::c_char;
    fn strcasecmp(
        _: *const ::core::ffi::c_char,
        _: *const ::core::ffi::c_char,
    ) -> ::core::ffi::c_int;
    fn pbuf_printf(b: *mut pbuf, fmt: *const ::core::ffi::c_char, ...);
    fn tbuf_drop(b: *mut tbuf);
    static mut _DefaultRuneLocale: _RuneLocale;
    fn __maskrune(_: __darwin_ct_rune_t, _: ::core::ffi::c_ulong) -> ::core::ffi::c_int;
    fn snprintf(
        __str: *mut ::core::ffi::c_char,
        __size: size_t,
        __format: *const ::core::ffi::c_char,
        ...
    ) -> ::core::ffi::c_int;
}
pub type int16_t = i16;
pub type int32_t = i32;
pub type uint32_t = u32;
pub type __uint32_t = u32;
pub type __darwin_ct_rune_t = ::core::ffi::c_int;
pub type __darwin_size_t = usize;
pub type __darwin_wchar_t = ::libc::wchar_t;
pub type __darwin_rune_t = __darwin_wchar_t;
pub type size_t = __darwin_size_t;
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
pub struct font_cache {
    pub duplicate_maps: i32,
    pub res: xdv_resolver,
    pub entries: *mut fc_entry,
    pub map_loaded: bool,
    pub map: *mut map_entry,
    pub nmap: ::core::ffi::c_int,
    pub capmap: ::core::ffi::c_int,
    pub anon_counter: ::core::ffi::c_int,
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
#[derive(Copy, Clone)]
#[repr(C)]
pub struct fc_entry {
    pub next: *mut fc_entry,
    pub kind: entry_kind,
    pub name: *mut ::core::ffi::c_char,
    pub index: ::core::ffi::c_int,
    pub file: *mut tbuf,
    pub value: *mut ::core::ffi::c_void,
}
pub type entry_kind = ::core::ffi::c_uint;
pub const K_VF: entry_kind = 4;
pub const K_ENC: entry_kind = 3;
pub const K_TYPE1: entry_kind = 2;
pub const K_TFM: entry_kind = 1;
pub const K_NATIVE: entry_kind = 0;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct vf_fontdef {
    pub k: int32_t,
    pub scaled_fix: int32_t,
    pub name: *mut ::core::ffi::c_char,
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
pub const NULL: *mut ::core::ffi::c_void = __DARWIN_NULL;
pub const __DARWIN_NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
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
unsafe extern "C" fn pbuf_free(mut b: *mut pbuf) {
    free((*b).data as *mut ::core::ffi::c_void);
    (*b).data = ::core::ptr::null_mut::<::core::ffi::c_uchar>();
    (*b).cap = 0 as size_t;
    (*b).len = (*b).cap;
}
pub const _CACHED_RUNES: ::core::ffi::c_int = (1 as ::core::ffi::c_int) << 8 as ::core::ffi::c_int;
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
pub unsafe extern "C" fn isdigit(mut _c: ::core::ffi::c_int) -> ::core::ffi::c_int {
    return __isctype(_c as __darwin_ct_rune_t, _CTYPE_D as ::core::ffi::c_ulong)
        as ::core::ffi::c_int;
}
#[inline]
pub unsafe extern "C" fn isspace(mut _c: ::core::ffi::c_int) -> ::core::ffi::c_int {
    return __istype(_c as __darwin_ct_rune_t, _CTYPE_S as ::core::ffi::c_ulong);
}
#[no_mangle]
pub unsafe extern "C" fn font_cache_new(mut resolver: xdv_resolver) -> *mut font_cache {
    let mut fc: *mut font_cache =
        calloc(1 as size_t, ::core::mem::size_of::<font_cache>() as size_t) as *mut font_cache;
    if fc.is_null() {
        abort();
    }
    (*fc).res = resolver;
    return fc;
}
unsafe extern "C" fn free_value(mut e: *mut fc_entry) {
    if (*e).value.is_null() {
        return;
    }
    match (*e).kind as ::core::ffi::c_uint {
        0 => {
            let mut f: *mut native_face = (*e).value as *mut native_face;
            free((*f).sfnt as *mut ::core::ffi::c_void);
            free((*f).to_unicode as *mut ::core::ffi::c_void);
            pbuf_free(&raw mut (*f).deflated);
        }
        2 => {
            let mut t: *mut type1_font = (*e).value as *mut type1_font;
            free((*t).data as *mut ::core::ffi::c_void);
            pbuf_free(&raw mut (*t).deflated);
        }
        3 => {
            let mut v: *mut enc_vector = (*e).value as *mut enc_vector;
            let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
            while i < 256 as ::core::ffi::c_int {
                free((*v).glyph[i as usize] as *mut ::core::ffi::c_void);
                i += 1;
            }
        }
        4 => {
            let mut v_0: *mut vf_font = (*e).value as *mut vf_font;
            let mut i_0: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
            while i_0 < (*v_0).nfonts {
                free((*(*v_0).fonts.offset(i_0 as isize)).name as *mut ::core::ffi::c_void);
                i_0 += 1;
            }
            free((*v_0).fonts as *mut ::core::ffi::c_void);
        }
        _ => {}
    }
    free((*e).value);
}
#[no_mangle]
pub unsafe extern "C" fn font_cache_free(mut fc: *mut font_cache) {
    if fc.is_null() {
        return;
    }
    let mut e: *mut fc_entry = (*fc).entries;
    while !e.is_null() {
        let mut n: *mut fc_entry = (*e).next as *mut fc_entry;
        free_value(e);
        tbuf_drop((*e).file);
        free((*e).name as *mut ::core::ffi::c_void);
        free(e as *mut ::core::ffi::c_void);
        e = n;
    }
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < (*fc).nmap {
        free((*(*fc).map.offset(i as isize)).tfm as *mut ::core::ffi::c_void);
        free((*(*fc).map.offset(i as isize)).psname as *mut ::core::ffi::c_void);
        free((*(*fc).map.offset(i as isize)).fontfile as *mut ::core::ffi::c_void);
        free((*(*fc).map.offset(i as isize)).encfile as *mut ::core::ffi::c_void);
        i += 1;
    }
    free((*fc).map as *mut ::core::ffi::c_void);
    free(fc as *mut ::core::ffi::c_void);
}
unsafe extern "C" fn lookup(
    mut fc: *mut font_cache,
    mut k: entry_kind,
    mut name: *const ::core::ffi::c_char,
    mut index: ::core::ffi::c_int,
) -> *mut fc_entry {
    let mut e: *mut fc_entry = (*fc).entries;
    while !e.is_null() {
        if (*e).kind as ::core::ffi::c_uint == k as ::core::ffi::c_uint
            && (*e).index == index
            && strcmp((*e).name, name) == 0 as ::core::ffi::c_int
        {
            return e;
        }
        e = (*e).next as *mut fc_entry;
    }
    return ::core::ptr::null_mut::<fc_entry>();
}
unsafe extern "C" fn insert(
    mut fc: *mut font_cache,
    mut k: entry_kind,
    mut name: *const ::core::ffi::c_char,
    mut index: ::core::ffi::c_int,
    mut file: *mut tbuf,
    mut value: *mut ::core::ffi::c_void,
) -> *mut fc_entry {
    let mut e: *mut fc_entry =
        calloc(1 as size_t, ::core::mem::size_of::<fc_entry>() as size_t) as *mut fc_entry;
    if e.is_null() {
        abort();
    }
    (*e).kind = k;
    (*e).name = strdup(name);
    (*e).index = index;
    (*e).file = file;
    (*e).value = value;
    (*e).next = (*fc).entries as *mut fc_entry;
    (*fc).entries = e;
    return e;
}
unsafe extern "C" fn xcalloc(mut n: size_t, mut s: size_t) -> *mut ::core::ffi::c_void {
    let mut p: *mut ::core::ffi::c_void = calloc(if n != 0 { n } else { 1 as size_t }, s);
    if p.is_null() {
        abort();
    }
    return p;
}
unsafe extern "C" fn rd32(mut p: *const ::core::ffi::c_uchar) -> uint32_t {
    return (*p.offset(0 as ::core::ffi::c_int as isize) as uint32_t) << 24 as ::core::ffi::c_int
        | (*p.offset(1 as ::core::ffi::c_int as isize) as uint32_t) << 16 as ::core::ffi::c_int
        | (*p.offset(2 as ::core::ffi::c_int as isize) as uint32_t) << 8 as ::core::ffi::c_int
        | *p.offset(3 as ::core::ffi::c_int as isize) as uint32_t;
}
unsafe extern "C" fn rd16(mut p: *const ::core::ffi::c_uchar) -> ::core::ffi::c_uint {
    return (*p.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_uint)
        << 8 as ::core::ffi::c_int
        | *p.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_uint;
}
unsafe extern "C" fn rds16(mut p: *const ::core::ffi::c_uchar) -> ::core::ffi::c_int {
    return rd16(p) as int16_t as ::core::ffi::c_int;
}
unsafe extern "C" fn find_table(
    mut s: *const ::core::ffi::c_uchar,
    mut len: size_t,
    mut tag: *const ::core::ffi::c_char,
    mut out: *mut *const ::core::ffi::c_uchar,
    mut olen: *mut size_t,
) -> bool {
    if len < 12 as size_t {
        return false_0 != 0;
    }
    let mut n: ::core::ffi::c_uint = rd16(s.offset(4 as ::core::ffi::c_int as isize));
    if (12 as size_t).wrapping_add((n as size_t).wrapping_mul(16 as size_t)) > len {
        return false_0 != 0;
    }
    let mut i: ::core::ffi::c_uint = 0 as ::core::ffi::c_uint;
    while i < n {
        let mut r: *const ::core::ffi::c_uchar = s
            .offset(12 as ::core::ffi::c_int as isize)
            .offset(i.wrapping_mul(16 as ::core::ffi::c_uint) as isize);
        if memcmp(
            r as *const ::core::ffi::c_void,
            tag as *const ::core::ffi::c_void,
            4 as size_t,
        ) == 0 as ::core::ffi::c_int
        {
            let mut off: uint32_t = rd32(r.offset(8 as ::core::ffi::c_int as isize));
            let mut l: uint32_t = rd32(r.offset(12 as ::core::ffi::c_int as isize));
            if off as size_t > len || l as size_t > len.wrapping_sub(off as size_t) {
                return false_0 != 0;
            }
            *out = s.offset(off as isize);
            *olen = l as size_t;
            return true_0 != 0;
        }
        i = i.wrapping_add(1);
    }
    return false_0 != 0;
}
unsafe extern "C" fn extract_face(
    mut file: *const ::core::ffi::c_uchar,
    mut flen: size_t,
    mut face_off: size_t,
    mut out_len: *mut size_t,
) -> *mut ::core::ffi::c_uchar {
    if face_off.wrapping_add(12 as size_t) > flen {
        return ::core::ptr::null_mut::<::core::ffi::c_uchar>();
    }
    let mut h: *const ::core::ffi::c_uchar = file.offset(face_off as isize);
    let mut n: ::core::ffi::c_uint = rd16(h.offset(4 as ::core::ffi::c_int as isize));
    if face_off
        .wrapping_add(12 as size_t)
        .wrapping_add((n as size_t).wrapping_mul(16 as size_t))
        > flen
    {
        return ::core::ptr::null_mut::<::core::ffi::c_uchar>();
    }
    let mut total: size_t = (12 as size_t).wrapping_add((n as size_t).wrapping_mul(16 as size_t));
    let mut i: ::core::ffi::c_uint = 0 as ::core::ffi::c_uint;
    while i < n {
        let mut r: *const ::core::ffi::c_uchar = h
            .offset(12 as ::core::ffi::c_int as isize)
            .offset(i.wrapping_mul(16 as ::core::ffi::c_uint) as isize);
        let mut off: uint32_t = rd32(r.offset(8 as ::core::ffi::c_int as isize));
        let mut l: uint32_t = rd32(r.offset(12 as ::core::ffi::c_int as isize));
        if off as size_t > flen || l as size_t > flen.wrapping_sub(off as size_t) {
            return ::core::ptr::null_mut::<::core::ffi::c_uchar>();
        }
        total = total.wrapping_add(
            (l as size_t).wrapping_add(3 as size_t) & !(3 as ::core::ffi::c_int as size_t),
        );
        i = i.wrapping_add(1);
    }
    let mut o: *mut ::core::ffi::c_uchar = xcalloc(total, 1 as size_t) as *mut ::core::ffi::c_uchar;
    memcpy(
        o as *mut ::core::ffi::c_void,
        h as *const ::core::ffi::c_void,
        12 as size_t,
    );
    let mut pos: size_t = (12 as size_t).wrapping_add((n as size_t).wrapping_mul(16 as size_t));
    let mut i_0: ::core::ffi::c_uint = 0 as ::core::ffi::c_uint;
    while i_0 < n {
        let mut r_0: *const ::core::ffi::c_uchar = h
            .offset(12 as ::core::ffi::c_int as isize)
            .offset(i_0.wrapping_mul(16 as ::core::ffi::c_uint) as isize);
        let mut off_0: uint32_t = rd32(r_0.offset(8 as ::core::ffi::c_int as isize));
        let mut l_0: uint32_t = rd32(r_0.offset(12 as ::core::ffi::c_int as isize));
        let mut rec: *mut ::core::ffi::c_uchar = o
            .offset(12 as ::core::ffi::c_int as isize)
            .offset(i_0.wrapping_mul(16 as ::core::ffi::c_uint) as isize);
        memcpy(
            rec as *mut ::core::ffi::c_void,
            r_0 as *const ::core::ffi::c_void,
            8 as size_t,
        );
        *rec.offset(8 as ::core::ffi::c_int as isize) =
            (pos >> 24 as ::core::ffi::c_int) as ::core::ffi::c_uchar;
        *rec.offset(9 as ::core::ffi::c_int as isize) =
            (pos >> 16 as ::core::ffi::c_int) as ::core::ffi::c_uchar;
        *rec.offset(10 as ::core::ffi::c_int as isize) =
            (pos >> 8 as ::core::ffi::c_int) as ::core::ffi::c_uchar;
        *rec.offset(11 as ::core::ffi::c_int as isize) = pos as ::core::ffi::c_uchar;
        memcpy(
            rec.offset(12 as ::core::ffi::c_int as isize) as *mut ::core::ffi::c_void,
            r_0.offset(12 as ::core::ffi::c_int as isize) as *const ::core::ffi::c_void,
            4 as size_t,
        );
        memcpy(
            o.offset(pos as isize) as *mut ::core::ffi::c_void,
            file.offset(off_0 as isize) as *const ::core::ffi::c_void,
            l_0 as size_t,
        );
        pos = pos.wrapping_add(
            (l_0 as size_t).wrapping_add(3 as size_t) & !(3 as ::core::ffi::c_int as size_t),
        );
        i_0 = i_0.wrapping_add(1);
    }
    *out_len = total;
    return o;
}
unsafe extern "C" fn set_unicode(
    mut f: *mut native_face,
    mut gid: ::core::ffi::c_uint,
    mut cp: uint32_t,
) {
    if gid != 0
        && gid < (*f).num_glyphs as ::core::ffi::c_uint
        && *(*f).to_unicode.offset(gid as isize) == 0
    {
        *(*f).to_unicode.offset(gid as isize) = cp;
    }
}
unsafe extern "C" fn parse_cmap(mut f: *mut native_face) {
    let mut c: *const ::core::ffi::c_uchar = ::core::ptr::null::<::core::ffi::c_uchar>();
    let mut len: size_t = 0;
    if !find_table(
        (*f).sfnt,
        (*f).sfnt_len,
        b"cmap\0" as *const u8 as *const ::core::ffi::c_char,
        &raw mut c,
        &raw mut len,
    ) || len < 4 as size_t
    {
        return;
    }
    let mut n: ::core::ffi::c_uint = rd16(c.offset(2 as ::core::ffi::c_int as isize));
    let mut best: *const ::core::ffi::c_uchar = ::core::ptr::null::<::core::ffi::c_uchar>();
    let mut best_rank: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut best_len: size_t = 0 as size_t;
    let mut symbol: bool = false_0 != 0;
    let mut i: ::core::ffi::c_uint = 0 as ::core::ffi::c_uint;
    while i < n
        && (4 as ::core::ffi::c_uint).wrapping_add(
            i.wrapping_add(1 as ::core::ffi::c_uint)
                .wrapping_mul(8 as ::core::ffi::c_uint),
        ) as size_t
            <= len
    {
        let mut r: *const ::core::ffi::c_uchar = c
            .offset(4 as ::core::ffi::c_int as isize)
            .offset(i.wrapping_mul(8 as ::core::ffi::c_uint) as isize);
        let mut pid: ::core::ffi::c_uint = rd16(r);
        let mut eid: ::core::ffi::c_uint = rd16(r.offset(2 as ::core::ffi::c_int as isize));
        let mut off: uint32_t = rd32(r.offset(4 as ::core::ffi::c_int as isize));
        if !(off as size_t > len.wrapping_sub(4 as size_t)) {
            let mut fmt: ::core::ffi::c_uint = rd16(c.offset(off as isize));
            let mut rank: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
            if fmt == 12 as ::core::ffi::c_uint
                && (pid == 3 as ::core::ffi::c_uint && eid == 10 as ::core::ffi::c_uint
                    || pid == 0 as ::core::ffi::c_uint)
            {
                rank = 4 as ::core::ffi::c_int;
            } else if fmt == 4 as ::core::ffi::c_uint
                && (pid == 3 as ::core::ffi::c_uint && eid == 1 as ::core::ffi::c_uint
                    || pid == 0 as ::core::ffi::c_uint)
            {
                rank = 3 as ::core::ffi::c_int;
            } else if fmt == 4 as ::core::ffi::c_uint
                && pid == 3 as ::core::ffi::c_uint
                && eid == 0 as ::core::ffi::c_uint
            {
                rank = 1 as ::core::ffi::c_int;
            }
            if rank > best_rank {
                best_rank = rank;
                best = c.offset(off as isize);
                best_len = len.wrapping_sub(off as size_t);
                symbol = rank == 1 as ::core::ffi::c_int;
            }
        }
        i = i.wrapping_add(1);
    }
    if best.is_null() {
        return;
    }
    let mut fmt_0: ::core::ffi::c_uint = rd16(best);
    if fmt_0 == 12 as ::core::ffi::c_uint && best_len >= 16 as size_t {
        let mut groups: uint32_t = rd32(best.offset(12 as ::core::ffi::c_int as isize));
        let mut g: uint32_t = 0 as uint32_t;
        while g < groups
            && (16 as size_t)
                .wrapping_add((g.wrapping_add(1 as uint32_t) as size_t).wrapping_mul(12 as size_t))
                <= best_len
        {
            let mut p: *const ::core::ffi::c_uchar = best
                .offset(16 as ::core::ffi::c_int as isize)
                .offset(g.wrapping_mul(12 as uint32_t) as isize);
            let mut s: uint32_t = rd32(p);
            let mut e: uint32_t = rd32(p.offset(4 as ::core::ffi::c_int as isize));
            let mut gid: uint32_t = rd32(p.offset(8 as ::core::ffi::c_int as isize));
            if !(e < s || e.wrapping_sub(s) > 0x110000 as uint32_t) {
                let mut cp: uint32_t = s;
                while cp <= e && cp < 0x110000 as uint32_t {
                    set_unicode(
                        f,
                        (gid as ::core::ffi::c_uint).wrapping_add(
                            (cp as ::core::ffi::c_uint).wrapping_sub(s as ::core::ffi::c_uint),
                        ),
                        cp,
                    );
                    cp = cp.wrapping_add(1);
                }
            }
            g = g.wrapping_add(1);
        }
    } else if fmt_0 == 4 as ::core::ffi::c_uint && best_len >= 14 as size_t {
        let mut segx2: ::core::ffi::c_uint = rd16(best.offset(6 as ::core::ffi::c_int as isize));
        let mut need: size_t =
            (16 as size_t).wrapping_add((segx2 as size_t).wrapping_mul(4 as size_t));
        if need > best_len {
            return;
        }
        let mut ends: *const ::core::ffi::c_uchar = best.offset(14 as ::core::ffi::c_int as isize);
        let mut starts: *const ::core::ffi::c_uchar = ends
            .offset(segx2 as isize)
            .offset(2 as ::core::ffi::c_int as isize);
        let mut deltas: *const ::core::ffi::c_uchar = starts.offset(segx2 as isize);
        let mut ranges: *const ::core::ffi::c_uchar = deltas.offset(segx2 as isize);
        let mut s_0: ::core::ffi::c_uint = 0 as ::core::ffi::c_uint;
        while s_0 < segx2.wrapping_div(2 as ::core::ffi::c_uint) {
            let mut end: ::core::ffi::c_uint =
                rd16(ends.offset((2 as ::core::ffi::c_uint).wrapping_mul(s_0) as isize));
            let mut start: ::core::ffi::c_uint =
                rd16(starts.offset((2 as ::core::ffi::c_uint).wrapping_mul(s_0) as isize));
            let mut delta: ::core::ffi::c_int =
                rds16(deltas.offset((2 as ::core::ffi::c_uint).wrapping_mul(s_0) as isize));
            let mut ro: ::core::ffi::c_uint =
                rd16(ranges.offset((2 as ::core::ffi::c_uint).wrapping_mul(s_0) as isize));
            let mut cp_0: ::core::ffi::c_uint = start;
            while cp_0 <= end && cp_0 != 0xffff as ::core::ffi::c_uint {
                let mut gid_0: ::core::ffi::c_uint = 0;
                if ro == 0 as ::core::ffi::c_uint {
                    gid_0 = cp_0.wrapping_add(delta as ::core::ffi::c_uint)
                        & 0xffff as ::core::ffi::c_uint;
                } else {
                    let mut g_0: *const ::core::ffi::c_uchar = ranges
                        .offset((2 as ::core::ffi::c_uint).wrapping_mul(s_0) as isize)
                        .offset(ro as isize)
                        .offset(
                            (2 as ::core::ffi::c_uint).wrapping_mul(cp_0.wrapping_sub(start))
                                as isize,
                        );
                    if g_0.offset(2 as ::core::ffi::c_int as isize) > best.offset(best_len as isize)
                    {
                        break;
                    }
                    gid_0 = rd16(g_0);
                    if gid_0 != 0 {
                        gid_0 = gid_0.wrapping_add(delta as ::core::ffi::c_uint)
                            & 0xffff as ::core::ffi::c_uint;
                    }
                }
                let mut u: uint32_t = cp_0 as uint32_t;
                if symbol as ::core::ffi::c_int != 0
                    && cp_0 >= 0xf000 as ::core::ffi::c_uint
                    && cp_0 <= 0xf0ff as ::core::ffi::c_uint
                {
                    u = cp_0.wrapping_sub(0xf000 as ::core::ffi::c_uint) as uint32_t;
                }
                set_unicode(f, gid_0, u);
                cp_0 = cp_0.wrapping_add(1);
            }
            s_0 = s_0.wrapping_add(1);
        }
    }
}
unsafe extern "C" fn script_substitution(
    mut f: *mut native_face,
    mut s: *const ::core::ffi::c_uchar,
    mut len: size_t,
    mut type_0: ::core::ffi::c_uint,
) {
    if type_0 == 7 as ::core::ffi::c_uint
        && len >= 8 as size_t
        && rd16(s) == 1 as ::core::ffi::c_uint
    {
        let mut off: ::core::ffi::c_uint =
            rd32(s.offset(4 as ::core::ffi::c_int as isize)) as ::core::ffi::c_uint;
        if off as size_t > len || len.wrapping_sub(off as size_t) < 6 as size_t {
            return;
        }
        type_0 = rd16(s.offset(2 as ::core::ffi::c_int as isize));
        s = s.offset(off as isize);
        len = len.wrapping_sub(off as size_t);
    }
    if type_0 != 1 as ::core::ffi::c_uint && type_0 != 3 as ::core::ffi::c_uint || len < 6 as size_t
    {
        return;
    }
    let mut format: ::core::ffi::c_uint = rd16(s);
    let mut off_0: ::core::ffi::c_uint = rd16(s.offset(2 as ::core::ffi::c_int as isize));
    if type_0 == 1 as ::core::ffi::c_uint
        && format != 1 as ::core::ffi::c_uint
        && format != 2 as ::core::ffi::c_uint
        || type_0 == 3 as ::core::ffi::c_uint && format != 1 as ::core::ffi::c_uint
        || off_0 as size_t > len
        || len.wrapping_sub(off_0 as size_t) < 4 as size_t
    {
        return;
    }
    let mut cov: *const ::core::ffi::c_uchar = s.offset(off_0 as isize);
    let mut clen: size_t = len.wrapping_sub(off_0 as size_t);
    let mut cformat: ::core::ffi::c_uint = rd16(cov);
    let mut count: ::core::ffi::c_uint = rd16(cov.offset(2 as ::core::ffi::c_int as isize));
    if cformat != 1 as ::core::ffi::c_uint && cformat != 2 as ::core::ffi::c_uint {
        return;
    }
    let mut stride: ::core::ffi::c_uint = (if cformat == 1 as ::core::ffi::c_uint {
        2 as ::core::ffi::c_int
    } else {
        6 as ::core::ffi::c_int
    }) as ::core::ffi::c_uint;
    if count as size_t
        > clen
            .wrapping_sub(4 as size_t)
            .wrapping_div(stride as size_t)
    {
        return;
    }
    let mut i: ::core::ffi::c_uint = 0 as ::core::ffi::c_uint;
    while i < count {
        let mut r: *const ::core::ffi::c_uchar = cov
            .offset(4 as ::core::ffi::c_int as isize)
            .offset(i.wrapping_mul(stride) as isize);
        let mut first: ::core::ffi::c_uint = rd16(r);
        let mut last: ::core::ffi::c_uint = if cformat == 1 as ::core::ffi::c_uint {
            first
        } else {
            rd16(r.offset(2 as ::core::ffi::c_int as isize))
        };
        let mut index: ::core::ffi::c_uint = if cformat == 1 as ::core::ffi::c_uint {
            i
        } else {
            rd16(r.offset(4 as ::core::ffi::c_int as isize))
        };
        let mut gid: ::core::ffi::c_uint = first;
        while gid <= last && gid < (*f).num_glyphs as ::core::ffi::c_uint {
            let mut cp: uint32_t = *(*f).to_unicode.offset(gid as isize);
            if !(cp == 0) {
                if type_0 == 1 as ::core::ffi::c_uint && format == 1 as ::core::ffi::c_uint {
                    set_unicode(
                        f,
                        gid.wrapping_add(rd16(s.offset(4 as ::core::ffi::c_int as isize)))
                            & 0xffff as ::core::ffi::c_uint,
                        cp,
                    );
                } else if index < rd16(s.offset(4 as ::core::ffi::c_int as isize))
                    && (index as size_t) < len.wrapping_sub(6 as size_t).wrapping_div(2 as size_t)
                {
                    let mut value: ::core::ffi::c_uint = rd16(
                        s.offset(6 as ::core::ffi::c_int as isize)
                            .offset(index.wrapping_mul(2 as ::core::ffi::c_uint) as isize),
                    );
                    if type_0 == 1 as ::core::ffi::c_uint {
                        set_unicode(f, value, cp);
                    } else if value as size_t <= len
                        && len.wrapping_sub(value as size_t) >= 2 as size_t
                    {
                        let mut n: ::core::ffi::c_uint = rd16(s.offset(value as isize));
                        if !(n as size_t
                            > len
                                .wrapping_sub(value as size_t)
                                .wrapping_sub(2 as size_t)
                                .wrapping_div(2 as size_t))
                        {
                            let mut k: ::core::ffi::c_uint = 0 as ::core::ffi::c_uint;
                            while k < n {
                                set_unicode(
                                    f,
                                    rd16(
                                        s.offset(value as isize)
                                            .offset(2 as ::core::ffi::c_int as isize)
                                            .offset(
                                                k.wrapping_mul(2 as ::core::ffi::c_uint) as isize
                                            ),
                                    ),
                                    cp,
                                );
                                k = k.wrapping_add(1);
                            }
                        }
                    }
                }
            }
            gid = gid.wrapping_add(1);
            index = index.wrapping_add(1);
        }
        i = i.wrapping_add(1);
    }
}
unsafe extern "C" fn parse_script_unicode(mut f: *mut native_face) {
    let mut s: *const ::core::ffi::c_uchar = ::core::ptr::null::<::core::ffi::c_uchar>();
    let mut len: size_t = 0;
    if !find_table(
        (*f).sfnt,
        (*f).sfnt_len,
        b"GSUB\0" as *const u8 as *const ::core::ffi::c_char,
        &raw mut s,
        &raw mut len,
    ) || len < 10 as size_t
        || rd16(s) != 1 as ::core::ffi::c_uint
    {
        return;
    }
    let mut fo: ::core::ffi::c_uint = rd16(s.offset(6 as ::core::ffi::c_int as isize));
    let mut lo: ::core::ffi::c_uint = rd16(s.offset(8 as ::core::ffi::c_int as isize));
    if fo as size_t > len
        || len.wrapping_sub(fo as size_t) < 2 as size_t
        || lo as size_t > len
        || len.wrapping_sub(lo as size_t) < 2 as size_t
    {
        return;
    }
    let mut nf: ::core::ffi::c_uint = rd16(s.offset(fo as isize));
    let mut nl: ::core::ffi::c_uint = rd16(s.offset(lo as isize));
    if nf as size_t
        > len
            .wrapping_sub(fo as size_t)
            .wrapping_sub(2 as size_t)
            .wrapping_div(6 as size_t)
        || nl as size_t
            > len
                .wrapping_sub(lo as size_t)
                .wrapping_sub(2 as size_t)
                .wrapping_div(2 as size_t)
    {
        return;
    }
    let mut i: ::core::ffi::c_uint = 0 as ::core::ffi::c_uint;
    while i < nf {
        let mut rec: *const ::core::ffi::c_uchar = s
            .offset(fo as isize)
            .offset(2 as ::core::ffi::c_int as isize)
            .offset(i.wrapping_mul(6 as ::core::ffi::c_uint) as isize);
        if !(memcmp(
            rec as *const ::core::ffi::c_void,
            b"ssty\0" as *const u8 as *const ::core::ffi::c_char as *const ::core::ffi::c_void,
            4 as size_t,
        ) != 0)
        {
            let mut feat: size_t = (fo as size_t)
                .wrapping_add(rd16(rec.offset(4 as ::core::ffi::c_int as isize)) as size_t);
            if !(feat > len || len.wrapping_sub(feat) < 4 as size_t) {
                let mut n: ::core::ffi::c_uint = rd16(
                    s.offset(feat as isize)
                        .offset(2 as ::core::ffi::c_int as isize),
                );
                if !(n as size_t
                    > len
                        .wrapping_sub(feat)
                        .wrapping_sub(4 as size_t)
                        .wrapping_div(2 as size_t))
                {
                    let mut j: ::core::ffi::c_uint = 0 as ::core::ffi::c_uint;
                    while j < n {
                        let mut idx: ::core::ffi::c_uint = rd16(
                            s.offset(feat as isize)
                                .offset(4 as ::core::ffi::c_int as isize)
                                .offset(j.wrapping_mul(2 as ::core::ffi::c_uint) as isize),
                        );
                        if !(idx >= nl) {
                            let mut lookup_0: size_t = (lo as size_t).wrapping_add(rd16(
                                s.offset(lo as isize)
                                    .offset(2 as ::core::ffi::c_int as isize)
                                    .offset(idx.wrapping_mul(2 as ::core::ffi::c_uint) as isize),
                            )
                                as size_t);
                            if !(lookup_0 > len || len.wrapping_sub(lookup_0) < 6 as size_t) {
                                let mut type_0: ::core::ffi::c_uint =
                                    rd16(s.offset(lookup_0 as isize));
                                let mut ns: ::core::ffi::c_uint = rd16(
                                    s.offset(lookup_0 as isize)
                                        .offset(4 as ::core::ffi::c_int as isize),
                                );
                                if !(ns as size_t
                                    > len
                                        .wrapping_sub(lookup_0)
                                        .wrapping_sub(6 as size_t)
                                        .wrapping_div(2 as size_t))
                                {
                                    let mut k: ::core::ffi::c_uint = 0 as ::core::ffi::c_uint;
                                    while k < ns {
                                        let mut sub: size_t = lookup_0.wrapping_add(rd16(
                                            s.offset(lookup_0 as isize)
                                                .offset(6 as ::core::ffi::c_int as isize)
                                                .offset(k.wrapping_mul(2 as ::core::ffi::c_uint)
                                                    as isize),
                                        )
                                            as size_t);
                                        if sub <= len {
                                            script_substitution(
                                                f,
                                                s.offset(sub as isize),
                                                len.wrapping_sub(sub),
                                                type_0,
                                            );
                                        }
                                        k = k.wrapping_add(1);
                                    }
                                }
                            }
                        }
                        j = j.wrapping_add(1);
                    }
                }
            }
        }
        i = i.wrapping_add(1);
    }
}
unsafe extern "C" fn read_psname(mut f: *mut native_face, mut fc: *mut font_cache) {
    let mut t: *const ::core::ffi::c_uchar = ::core::ptr::null::<::core::ffi::c_uchar>();
    let mut len: size_t = 0;
    (*f).psname[0 as ::core::ffi::c_int as usize] = 0 as ::core::ffi::c_char;
    if find_table(
        (*f).sfnt,
        (*f).sfnt_len,
        b"name\0" as *const u8 as *const ::core::ffi::c_char,
        &raw mut t,
        &raw mut len,
    ) as ::core::ffi::c_int
        != 0
        && len >= 6 as size_t
    {
        let mut count: ::core::ffi::c_uint = rd16(t.offset(2 as ::core::ffi::c_int as isize));
        let mut soff: ::core::ffi::c_uint = rd16(t.offset(4 as ::core::ffi::c_int as isize));
        let mut pass: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
        while pass < 2 as ::core::ffi::c_int && (*f).psname[0 as ::core::ffi::c_int as usize] == 0 {
            let mut i: ::core::ffi::c_uint = 0 as ::core::ffi::c_uint;
            while i < count
                && (6 as size_t).wrapping_add(
                    (i.wrapping_add(1 as ::core::ffi::c_uint) as size_t).wrapping_mul(12 as size_t),
                ) <= len
            {
                let mut r: *const ::core::ffi::c_uchar = t
                    .offset(6 as ::core::ffi::c_int as isize)
                    .offset(i.wrapping_mul(12 as ::core::ffi::c_uint) as isize);
                let mut pid: ::core::ffi::c_uint = rd16(r);
                let mut nid: ::core::ffi::c_uint = rd16(r.offset(6 as ::core::ffi::c_int as isize));
                let mut l: ::core::ffi::c_uint = rd16(r.offset(8 as ::core::ffi::c_int as isize));
                let mut o: ::core::ffi::c_uint = rd16(r.offset(10 as ::core::ffi::c_int as isize));
                if !(nid != 6 as ::core::ffi::c_uint
                    || (soff as size_t)
                        .wrapping_add(o as size_t)
                        .wrapping_add(l as size_t)
                        > len)
                {
                    let mut s: *const ::core::ffi::c_uchar =
                        t.offset(soff as isize).offset(o as isize);
                    let mut k: size_t = 0 as size_t;
                    if pass == 0 as ::core::ffi::c_int && pid == 3 as ::core::ffi::c_uint {
                        let mut j: ::core::ffi::c_uint = 0 as ::core::ffi::c_uint;
                        while j.wrapping_add(1 as ::core::ffi::c_uint) < l
                            && k < (::core::mem::size_of::<[::core::ffi::c_char; 128]>() as usize)
                                .wrapping_sub(1 as usize)
                        {
                            if *s.offset(j as isize) as ::core::ffi::c_int
                                == 0 as ::core::ffi::c_int
                                && *s.offset(j.wrapping_add(1 as ::core::ffi::c_uint) as isize)
                                    as ::core::ffi::c_int
                                    > 32 as ::core::ffi::c_int
                                && (*s.offset(j.wrapping_add(1 as ::core::ffi::c_uint) as isize)
                                    as ::core::ffi::c_int)
                                    < 127 as ::core::ffi::c_int
                            {
                                let fresh1 = k;
                                k = k.wrapping_add(1);
                                (*f).psname[fresh1 as usize] = *s
                                    .offset(j.wrapping_add(1 as ::core::ffi::c_uint) as isize)
                                    as ::core::ffi::c_char;
                            }
                            j = j.wrapping_add(2 as ::core::ffi::c_uint);
                        }
                    } else if pass == 1 as ::core::ffi::c_int && pid == 1 as ::core::ffi::c_uint {
                        let mut j_0: ::core::ffi::c_uint = 0 as ::core::ffi::c_uint;
                        while j_0 < l
                            && k < (::core::mem::size_of::<[::core::ffi::c_char; 128]>() as usize)
                                .wrapping_sub(1 as usize)
                        {
                            if *s.offset(j_0 as isize) as ::core::ffi::c_int
                                > 32 as ::core::ffi::c_int
                                && (*s.offset(j_0 as isize) as ::core::ffi::c_int)
                                    < 127 as ::core::ffi::c_int
                            {
                                let fresh2 = k;
                                k = k.wrapping_add(1);
                                (*f).psname[fresh2 as usize] =
                                    *s.offset(j_0 as isize) as ::core::ffi::c_char;
                            }
                            j_0 = j_0.wrapping_add(1);
                        }
                    }
                    (*f).psname[k as usize] = 0 as ::core::ffi::c_char;
                    if k != 0 {
                        break;
                    }
                }
                i = i.wrapping_add(1);
            }
            pass += 1;
        }
    }
    let mut p: *mut ::core::ffi::c_char = &raw mut (*f).psname as *mut ::core::ffi::c_char;
    while *p != 0 {
        if !strchr(
            b"()<>[]{}/%#\0" as *const u8 as *const ::core::ffi::c_char,
            *p as ::core::ffi::c_int,
        )
        .is_null()
        {
            *p = '-' as i32 as ::core::ffi::c_char;
        }
        p = p.offset(1);
    }
    if (*f).psname[0 as ::core::ffi::c_int as usize] == 0 {
        (*fc).anon_counter += 1;
        snprintf(
            &raw mut (*f).psname as *mut ::core::ffi::c_char,
            ::core::mem::size_of::<[::core::ffi::c_char; 128]>() as size_t,
            b"PitexFont%d\0" as *const u8 as *const ::core::ffi::c_char,
            (*fc).anon_counter,
        );
    }
}
#[no_mangle]
pub unsafe extern "C" fn font_native(
    mut fc: *mut font_cache,
    mut name: *const ::core::ffi::c_char,
    mut index: ::core::ffi::c_int,
    mut warnings: *mut pbuf,
) -> *mut native_face {
    let mut e: *mut fc_entry = lookup(fc, K_NATIVE, name, index);
    if !e.is_null() {
        return (*e).value as *mut native_face;
    }
    let mut f: *mut native_face =
        xcalloc(1 as size_t, ::core::mem::size_of::<native_face>() as size_t) as *mut native_face;
    let mut file: *mut tbuf =
        (*fc).res.load.expect("non-null function pointer")((*fc).res.env, name, RES_NATIVE_FONT);
    insert(
        fc,
        K_NATIVE,
        name,
        index,
        file,
        f as *mut ::core::ffi::c_void,
    );
    if file.is_null() {
        pbuf_printf(
            warnings,
            b"font file not found: %s\n\0" as *const u8 as *const ::core::ffi::c_char,
            name,
        );
        return f;
    }
    let mut d: *const ::core::ffi::c_uchar = (*file).data;
    let mut n: size_t = (*file).len;
    let mut face_off: size_t = 0 as size_t;
    if n >= 12 as size_t
        && memcmp(
            d as *const ::core::ffi::c_void,
            b"ttcf\0" as *const u8 as *const ::core::ffi::c_char as *const ::core::ffi::c_void,
            4 as size_t,
        ) == 0 as ::core::ffi::c_int
    {
        let mut count: uint32_t = rd32(d.offset(8 as ::core::ffi::c_int as isize));
        if index < 0 as ::core::ffi::c_int
            || index as uint32_t >= count
            || (12 as size_t).wrapping_add(
                ((index + 1 as ::core::ffi::c_int) as size_t).wrapping_mul(4 as size_t),
            ) > n
        {
            pbuf_printf(
                warnings,
                b"font collection index %d out of range: %s\n\0" as *const u8
                    as *const ::core::ffi::c_char,
                index,
                name,
            );
            return f;
        }
        face_off = rd32(
            d.offset(12 as ::core::ffi::c_int as isize)
                .offset((index * 4 as ::core::ffi::c_int) as isize),
        ) as size_t;
    }
    if face_off.wrapping_add(4 as size_t) > n {
        return f;
    }
    let mut ver: uint32_t = rd32(d.offset(face_off as isize));
    if ver != 0x10000 as uint32_t && ver != 0x4f54544f as uint32_t && ver != 0x74727565 as uint32_t
    {
        pbuf_printf(
            warnings,
            b"unsupported native font format (not OpenType/TrueType): %s\n\0" as *const u8
                as *const ::core::ffi::c_char,
            name,
        );
        return f;
    }
    (*f).sfnt = extract_face(d, n, face_off, &raw mut (*f).sfnt_len);
    if (*f).sfnt.is_null() {
        pbuf_printf(
            warnings,
            b"corrupt font file: %s\n\0" as *const u8 as *const ::core::ffi::c_char,
            name,
        );
        return f;
    }
    (*f).cff = ver == 0x4f54544f as uint32_t;
    let mut t: *const ::core::ffi::c_uchar = ::core::ptr::null::<::core::ffi::c_uchar>();
    let mut len: size_t = 0;
    (*f).units_per_em = 1000 as ::core::ffi::c_int;
    let mut bb: [::core::ffi::c_double; 4] = [
        0 as ::core::ffi::c_int as ::core::ffi::c_double,
        -(250 as ::core::ffi::c_int) as ::core::ffi::c_double,
        1000 as ::core::ffi::c_int as ::core::ffi::c_double,
        750 as ::core::ffi::c_int as ::core::ffi::c_double,
    ];
    if find_table(
        (*f).sfnt,
        (*f).sfnt_len,
        b"head\0" as *const u8 as *const ::core::ffi::c_char,
        &raw mut t,
        &raw mut len,
    ) as ::core::ffi::c_int
        != 0
        && len >= 54 as size_t
    {
        (*f).units_per_em = rd16(t.offset(18 as ::core::ffi::c_int as isize)) as ::core::ffi::c_int;
        if (*f).units_per_em < 16 as ::core::ffi::c_int {
            (*f).units_per_em = 1000 as ::core::ffi::c_int;
        }
        bb[0 as ::core::ffi::c_int as usize] =
            rds16(t.offset(36 as ::core::ffi::c_int as isize)) as ::core::ffi::c_double;
        bb[1 as ::core::ffi::c_int as usize] =
            rds16(t.offset(38 as ::core::ffi::c_int as isize)) as ::core::ffi::c_double;
        bb[2 as ::core::ffi::c_int as usize] =
            rds16(t.offset(40 as ::core::ffi::c_int as isize)) as ::core::ffi::c_double;
        bb[3 as ::core::ffi::c_int as usize] =
            rds16(t.offset(42 as ::core::ffi::c_int as isize)) as ::core::ffi::c_double;
    }
    let mut k: ::core::ffi::c_double = 1000.0f64 / (*f).units_per_em as ::core::ffi::c_double;
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < 4 as ::core::ffi::c_int {
        (*f).bbox[i as usize] = bb[i as usize] * k;
        i += 1;
    }
    if find_table(
        (*f).sfnt,
        (*f).sfnt_len,
        b"maxp\0" as *const u8 as *const ::core::ffi::c_char,
        &raw mut t,
        &raw mut len,
    ) as ::core::ffi::c_int
        != 0
        && len >= 6 as size_t
    {
        (*f).num_glyphs = rd16(t.offset(4 as ::core::ffi::c_int as isize)) as ::core::ffi::c_int;
    }
    (*f).ascent = (*f).bbox[3 as ::core::ffi::c_int as usize];
    (*f).descent = (*f).bbox[1 as ::core::ffi::c_int as usize];
    if find_table(
        (*f).sfnt,
        (*f).sfnt_len,
        b"hhea\0" as *const u8 as *const ::core::ffi::c_char,
        &raw mut t,
        &raw mut len,
    ) as ::core::ffi::c_int
        != 0
        && len >= 36 as size_t
    {
        (*f).ascent =
            rds16(t.offset(4 as ::core::ffi::c_int as isize)) as ::core::ffi::c_double * k;
        (*f).descent =
            rds16(t.offset(6 as ::core::ffi::c_int as isize)) as ::core::ffi::c_double * k;
        (*f).num_hmetrics = rd16(t.offset(34 as ::core::ffi::c_int as isize)) as ::core::ffi::c_int;
    }
    if find_table(
        (*f).sfnt,
        (*f).sfnt_len,
        b"hmtx\0" as *const u8 as *const ::core::ffi::c_char,
        &raw mut t,
        &raw mut len,
    ) {
        (*f).hmtx = t;
        (*f).hmtx_len = len;
        if ((*f).num_hmetrics as size_t).wrapping_mul(4 as size_t) > len {
            (*f).num_hmetrics = len.wrapping_div(4 as size_t) as ::core::ffi::c_int;
        }
    }
    (*f).cap_height = (*f).ascent;
    if find_table(
        (*f).sfnt,
        (*f).sfnt_len,
        b"OS/2\0" as *const u8 as *const ::core::ffi::c_char,
        &raw mut t,
        &raw mut len,
    ) as ::core::ffi::c_int
        != 0
        && len >= 90 as size_t
        && rd16(t) >= 2 as ::core::ffi::c_uint
    {
        (*f).cap_height =
            rds16(t.offset(88 as ::core::ffi::c_int as isize)) as ::core::ffi::c_double * k;
    }
    if find_table(
        (*f).sfnt,
        (*f).sfnt_len,
        b"post\0" as *const u8 as *const ::core::ffi::c_char,
        &raw mut t,
        &raw mut len,
    ) as ::core::ffi::c_int
        != 0
        && len >= 8 as size_t
    {
        (*f).italic_angle = rd32(t.offset(4 as ::core::ffi::c_int as isize)) as int32_t
            as ::core::ffi::c_double
            / 65536.0f64;
    }
    if (*f).num_glyphs <= 0 as ::core::ffi::c_int {
        (*f).num_glyphs = 65535 as ::core::ffi::c_int;
    }
    (*f).to_unicode = xcalloc(
        (*f).num_glyphs as size_t,
        ::core::mem::size_of::<uint32_t>() as size_t,
    ) as *mut uint32_t;
    parse_cmap(f);
    parse_script_unicode(f);
    read_psname(f, fc);
    (*f).ok = true_0 != 0;
    return f;
}
#[no_mangle]
pub unsafe extern "C" fn native_advance(
    mut f: *const native_face,
    mut gid: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    if (*f).hmtx.is_null()
        || (*f).num_hmetrics <= 0 as ::core::ffi::c_int
        || gid < 0 as ::core::ffi::c_int
    {
        return 0 as ::core::ffi::c_int;
    }
    let mut i: ::core::ffi::c_int = if gid < (*f).num_hmetrics {
        gid
    } else {
        (*f).num_hmetrics - 1 as ::core::ffi::c_int
    };
    return rd16((*f).hmtx.offset((4 as ::core::ffi::c_int * i) as isize)) as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn font_tfm(
    mut fc: *mut font_cache,
    mut name: *const ::core::ffi::c_char,
) -> *mut tfm_font {
    let mut e: *mut fc_entry = lookup(fc, K_TFM, name, 0 as ::core::ffi::c_int);
    if !e.is_null() {
        return (*e).value as *mut tfm_font;
    }
    let mut t: *mut tfm_font =
        xcalloc(1 as size_t, ::core::mem::size_of::<tfm_font>() as size_t) as *mut tfm_font;
    let mut file: *mut tbuf =
        (*fc).res.load.expect("non-null function pointer")((*fc).res.env, name, RES_TFM);
    insert(
        fc,
        K_TFM,
        name,
        0 as ::core::ffi::c_int,
        file,
        t as *mut ::core::ffi::c_void,
    );
    if file.is_null() || (*file).len < 24 as size_t {
        return t;
    }
    let mut d: *const ::core::ffi::c_uchar = (*file).data;
    let mut lf: ::core::ffi::c_uint = rd16(d);
    let mut lh: ::core::ffi::c_uint = rd16(d.offset(2 as ::core::ffi::c_int as isize));
    let mut bc: ::core::ffi::c_uint = rd16(d.offset(4 as ::core::ffi::c_int as isize));
    let mut ec: ::core::ffi::c_uint = rd16(d.offset(6 as ::core::ffi::c_int as isize));
    let mut nw: ::core::ffi::c_uint = rd16(d.offset(8 as ::core::ffi::c_int as isize));
    if (lf as size_t).wrapping_mul(4 as size_t) > (*file).len
        || lh < 2 as ::core::ffi::c_uint
        || bc > ec.wrapping_add(1 as ::core::ffi::c_uint)
        || ec > 255 as ::core::ffi::c_uint
    {
        return t;
    }
    let mut header: size_t = 24 as size_t;
    let mut ci: size_t = header.wrapping_add((lh as size_t).wrapping_mul(4 as size_t));
    let mut wt: size_t = ci.wrapping_add(
        (ec.wrapping_sub(bc).wrapping_add(1 as ::core::ffi::c_uint) as size_t)
            .wrapping_mul(4 as size_t),
    );
    if wt.wrapping_add((nw as size_t).wrapping_mul(4 as size_t)) > (*file).len {
        return t;
    }
    (*t).checksum = rd32(d.offset(header as isize));
    (*t).design = rd32(
        d.offset(header as isize)
            .offset(4 as ::core::ffi::c_int as isize),
    ) as int32_t;
    (*t).bc = bc as ::core::ffi::c_int;
    (*t).ec = ec as ::core::ffi::c_int;
    let mut c: ::core::ffi::c_uint = bc;
    while c <= ec {
        let mut wi: ::core::ffi::c_uint = *d.offset(
            ci.wrapping_add(c.wrapping_sub(bc).wrapping_mul(4 as ::core::ffi::c_uint) as size_t)
                as isize,
        ) as ::core::ffi::c_uint;
        if !(wi == 0 as ::core::ffi::c_uint || wi >= nw) {
            (*t).width[c as usize] = rd32(
                d.offset(wt as isize)
                    .offset((wi as size_t).wrapping_mul(4 as size_t) as isize),
            ) as int32_t;
            (*t).exists[c as usize] = true_0 != 0;
        }
        c = c.wrapping_add(1);
    }
    (*t).ok = true_0 != 0;
    return t;
}
unsafe extern "C" fn next_token(
    mut p: *mut *mut ::core::ffi::c_char,
    mut quoted: *mut bool,
) -> *mut ::core::ffi::c_char {
    let mut s: *mut ::core::ffi::c_char = *p;
    while *s as ::core::ffi::c_int == ' ' as i32 || *s as ::core::ffi::c_int == '\t' as i32 {
        s = s.offset(1);
    }
    if *s == 0 {
        return ::core::ptr::null_mut::<::core::ffi::c_char>();
    }
    *quoted = false_0 != 0;
    let mut start: *mut ::core::ffi::c_char = ::core::ptr::null_mut::<::core::ffi::c_char>();
    if *s as ::core::ffi::c_int == '"' as i32 {
        *quoted = true_0 != 0;
        s = s.offset(1);
        start = s;
        while *s as ::core::ffi::c_int != 0 && *s as ::core::ffi::c_int != '"' as i32 {
            s = s.offset(1);
        }
    } else {
        start = s;
        while *s as ::core::ffi::c_int != 0
            && *s as ::core::ffi::c_int != ' ' as i32
            && *s as ::core::ffi::c_int != '\t' as i32
        {
            s = s.offset(1);
        }
    }
    if *s != 0 {
        let fresh4 = s;
        s = s.offset(1);
        *fresh4 = 0 as ::core::ffi::c_char;
    }
    *p = s;
    return start;
}
unsafe extern "C" fn ends_with(
    mut s: *const ::core::ffi::c_char,
    mut suffix: *const ::core::ffi::c_char,
) -> bool {
    let mut a: size_t = strlen(s);
    let mut b: size_t = strlen(suffix);
    return a >= b
        && strcasecmp(s.offset(a as isize).offset(-(b as isize)), suffix)
            == 0 as ::core::ffi::c_int;
}
unsafe extern "C" fn load_map(mut fc: *mut font_cache) {
    (*fc).map_loaded = true_0 != 0;
    let mut file: *mut tbuf = (*fc).res.load.expect("non-null function pointer")(
        (*fc).res.env,
        b"pdftex.map\0" as *const u8 as *const ::core::ffi::c_char,
        RES_MAP,
    );
    if file.is_null() {
        return;
    }
    let mut text: *mut ::core::ffi::c_char =
        malloc((*file).len.wrapping_add(1 as size_t)) as *mut ::core::ffi::c_char;
    if text.is_null() {
        abort();
    }
    memcpy(
        text as *mut ::core::ffi::c_void,
        (*file).data as *const ::core::ffi::c_void,
        (*file).len,
    );
    *text.offset((*file).len as isize) = 0 as ::core::ffi::c_char;
    tbuf_drop(file);
    pitex_parse_map_text(fc,text,b'+');
}
unsafe fn pitex_map_entry_free(entry:map_entry) {free(entry.tfm.cast());free(entry.psname.cast());free(entry.fontfile.cast());free(entry.encfile.cast());}
unsafe fn pitex_parse_map_text(fc:*mut font_cache,text:*mut ::core::ffi::c_char,mode:u8) {
    let mut line: *mut ::core::ffi::c_char = text;
    while !line.is_null() && *line as ::core::ffi::c_int != 0 {
        let mut nl: *mut ::core::ffi::c_char = strchr(line, '\n' as i32);
        if !nl.is_null() {
            *nl = 0 as ::core::ffi::c_char;
        }
        let mut p: *mut ::core::ffi::c_char = line;
        while *p as ::core::ffi::c_int == ' ' as i32 || *p as ::core::ffi::c_int == '\t' as i32 {
            p = p.offset(1);
        }
        if *p as ::core::ffi::c_int != 0
            && strchr(
                b"%#*;\0" as *const u8 as *const ::core::ffi::c_char,
                *p as ::core::ffi::c_int,
            )
            .is_null()
        {
            let mut m: map_entry = map_entry {
                tfm: ::core::ptr::null_mut::<::core::ffi::c_char>(),
                psname: ::core::ptr::null_mut::<::core::ffi::c_char>(),
                fontfile: ::core::ptr::null_mut::<::core::ffi::c_char>(),
                encfile: ::core::ptr::null_mut::<::core::ffi::c_char>(),
                slant: 0.,
                extend: 0.,
            };
            m.extend = 1 as ::core::ffi::c_int as ::core::ffi::c_double;
            let mut q: bool = false;
            let mut tok: *mut ::core::ffi::c_char = next_token(&raw mut p, &raw mut q);
            m.tfm = strdup(tok);
            loop {
                tok = next_token(&raw mut p, &raw mut q);
                if tok.is_null() {
                    break;
                }
                if q {
                    let mut s: *mut ::core::ffi::c_char =
                        ::core::ptr::null_mut::<::core::ffi::c_char>();
                    s = strstr(
                        tok,
                        b"SlantFont\0" as *const u8 as *const ::core::ffi::c_char,
                    );
                    if !s.is_null() {
                        *s = 0 as ::core::ffi::c_char;
                        let mut num: *mut ::core::ffi::c_char = strrchr(tok, ' ' as i32);
                        m.slant = std::ffi::CStr::from_ptr(tok).to_string_lossy().split_whitespace().last().and_then(|value| value.parse::<f64>().ok()).unwrap_or(0.0);
                        *s = 'S' as i32 as ::core::ffi::c_char;
                    }
                    s = strstr(
                        tok,
                        b"ExtendFont\0" as *const u8 as *const ::core::ffi::c_char,
                    );
                    if !s.is_null() {
                        *s = 0 as ::core::ffi::c_char;
                        let mut num_0: *mut ::core::ffi::c_char = strrchr(tok, ' ' as i32);
                        m.extend = std::ffi::CStr::from_ptr(tok).to_string_lossy().split_whitespace().last().and_then(|value| value.parse::<f64>().ok()).unwrap_or(1.0);
                    }
                } else if *tok.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                    == '<' as i32
                {
                    let mut f: *mut ::core::ffi::c_char =
                        tok.offset(1 as ::core::ffi::c_int as isize);
                    if *f as ::core::ffi::c_int == '<' as i32
                        || *f as ::core::ffi::c_int == '[' as i32
                    {
                        f = f.offset(1);
                    }
                    if *f == 0 {
                        f = next_token(&raw mut p, &raw mut q);
                        if f.is_null() {
                            break;
                        }
                    }
                    if ends_with(f, b".enc\0" as *const u8 as *const ::core::ffi::c_char) {
                        free(m.encfile as *mut ::core::ffi::c_void);
                        m.encfile = strdup(f);
                    } else {
                        free(m.fontfile as *mut ::core::ffi::c_void);
                        m.fontfile = strdup(f);
                    }
                } else if m.psname.is_null()
                    && isdigit(
                        *tok.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_uchar
                            as ::core::ffi::c_int,
                    ) == 0
                {
                    m.psname = strdup(tok);
                }
            }
            let existing=(0..(*fc).nmap).find(|i|strcmp((*(*fc).map.offset(*i as isize)).tfm,m.tfm)==0);
            if let Some(index)=existing {
                if mode!=b'=' && mode!=b'-' {(*fc).duplicate_maps=(*fc).duplicate_maps.saturating_add(1);}
                if mode==b'=' || mode==b'-' {
                    let previous=(*fc).map.offset(index as isize);
                    free((*previous).tfm.cast());free((*previous).psname.cast());free((*previous).fontfile.cast());free((*previous).encfile.cast());
                    if mode==b'=' { *previous=m; }
                    else { std::ptr::copy(previous.add(1),previous,((*fc).nmap-index-1) as usize);(*fc).nmap-=1;pitex_map_entry_free(m); }
                } else { pitex_map_entry_free(m); }
                line=if !nl.is_null() {nl.add(1)} else {std::ptr::null_mut()};
                continue;
            }
            if mode==b'-' {pitex_map_entry_free(m);line=if !nl.is_null() {nl.add(1)} else {std::ptr::null_mut()};continue;}
            if (*fc).nmap == (*fc).capmap {
                (*fc).capmap = if (*fc).capmap != 0 {
                    (*fc).capmap * 2 as ::core::ffi::c_int
                } else {
                    1024 as ::core::ffi::c_int
                };
                (*fc).map = realloc(
                    (*fc).map as *mut ::core::ffi::c_void,
                    (::core::mem::size_of::<map_entry>() as size_t)
                        .wrapping_mul((*fc).capmap as size_t),
                ) as *mut map_entry;
                if (*fc).map.is_null() {
                    abort();
                }
            }
            let fresh3 = (*fc).nmap;
            (*fc).nmap = (*fc).nmap + 1;
            *(*fc).map.offset(fresh3 as isize) = m;
        }
        line = if !nl.is_null() {
            nl.offset(1 as ::core::ffi::c_int as isize)
        } else {
            ::core::ptr::null_mut::<::core::ffi::c_char>()
        };
    }
    free(text as *mut ::core::ffi::c_void);
}
#[no_mangle]
pub unsafe extern "C" fn font_map_reset(fc:*mut font_cache) {
    for i in 0..(*fc).nmap {pitex_map_entry_free(*(*fc).map.offset(i as isize));}
    (*fc).nmap=0;(*fc).map_loaded=false;(*fc).duplicate_maps=0;
}
unsafe fn pitex_map_spec(fc:*mut font_cache,spec:*const ::core::ffi::c_char)->(u8,String) {
    let text=std::ffi::CStr::from_ptr(spec).to_string_lossy();let text=text.trim();
    let has_modifier=text.as_bytes().first().is_some_and(|b|b"+=-".contains(b));
    let (mode,text)=if has_modifier {(text.as_bytes()[0],text[1..].trim())}else{(b'+',text)};
    if has_modifier {
        // A modifier requests the default font map before applying the update.
        if !(*fc).map_loaded {load_map(fc);}
    } else if !(*fc).map_loaded {(*fc).map_loaded=true;}
    (mode,text.to_owned())
}
#[no_mangle]
pub unsafe extern "C" fn font_map_line(fc:*mut font_cache,spec:*const ::core::ffi::c_char) {
    let (mode,text)=pitex_map_spec(fc,spec);if text.is_empty(){return;}
    let text=std::ffi::CString::new(text).unwrap();pitex_parse_map_text(fc,strdup(text.as_ptr()),mode);
}
#[no_mangle]
pub unsafe extern "C" fn font_map_file(fc:*mut font_cache,spec:*const ::core::ffi::c_char)->bool {
    let (mode,text)=pitex_map_spec(fc,spec);if text.is_empty(){return true;}
    let text=std::ffi::CString::new(text).unwrap();
    let file=(*fc).res.load.expect("font map resolver")((*fc).res.env,text.as_ptr(),RES_MAP);
    if file.is_null(){return false;}
    let bytes=malloc((*file).len+1).cast::<::core::ffi::c_char>();
    if bytes.is_null(){abort();}
    memcpy(bytes.cast(),(*file).data.cast(),(*file).len);*bytes.add((*file).len)=0;tbuf_drop(file);
    pitex_parse_map_text(fc,bytes,mode);true
}
#[no_mangle]
pub unsafe extern "C" fn font_map_lookup(
    mut fc: *mut font_cache,
    mut tfm: *const ::core::ffi::c_char,
) -> *const map_entry {
    if !(*fc).map_loaded {
        load_map(fc);
    }
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < (*fc).nmap {
        if strcmp((*(*fc).map.offset(i as isize)).tfm, tfm) == 0 as ::core::ffi::c_int {
            return (*fc).map.offset(i as isize) as *mut map_entry;
        }
        i += 1;
    }
    return ::core::ptr::null::<map_entry>();
}
unsafe extern "C" fn memfind(
    mut hay: *const ::core::ffi::c_char,
    mut n: size_t,
    mut needle: *const ::core::ffi::c_char,
) -> *const ::core::ffi::c_char {
    let mut k: size_t = strlen(needle);
    let mut i: size_t = 0 as size_t;
    while i.wrapping_add(k) <= n {
        if *hay.offset(i as isize) as ::core::ffi::c_int
            == *needle.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
            && memcmp(
                hay.offset(i as isize) as *const ::core::ffi::c_void,
                needle as *const ::core::ffi::c_void,
                k,
            ) == 0 as ::core::ffi::c_int
        {
            return hay.offset(i as isize);
        }
        i = i.wrapping_add(1);
    }
    return ::core::ptr::null::<::core::ffi::c_char>();
}
unsafe extern "C" fn parse_type1_header(mut t: *mut type1_font) {
    let mut s: *const ::core::ffi::c_char = (*t).data as *const ::core::ffi::c_char;
    let mut n: size_t = (*t).len1;
    let mut p: *const ::core::ffi::c_char = memfind(
        s,
        n,
        b"/FontBBox\0" as *const u8 as *const ::core::ffi::c_char,
    );
    if !p.is_null() {
        p = p.offset(9 as ::core::ffi::c_int as isize);
        let mut end: *const ::core::ffi::c_char = s.offset(n as isize);
        while p < end
            && (*p as ::core::ffi::c_int == ' ' as i32
                || *p as ::core::ffi::c_int == '{' as i32
                || *p as ::core::ffi::c_int == '[' as i32)
        {
            p = p.offset(1);
        }
        let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
        while i < 4 as ::core::ffi::c_int && p < end {
            let mut q: *mut ::core::ffi::c_char = ::core::ptr::null_mut::<::core::ffi::c_char>();
            (*t).bbox[i as usize] = strtod(p, &raw mut q);
            if q == p as *mut ::core::ffi::c_char {
                break;
            }
            p = q;
            i += 1;
        }
    }
    p = memfind(
        s,
        n,
        b"/FontName\0" as *const u8 as *const ::core::ffi::c_char,
    );
    if !p.is_null() {
        p = p.offset(9 as ::core::ffi::c_int as isize);
        while p < s.offset(n as isize)
            && (*p as ::core::ffi::c_int == ' ' as i32 || *p as ::core::ffi::c_int == '/' as i32)
        {
            p = p.offset(1);
        }
        let mut k: size_t = 0 as size_t;
        while p < s.offset(n as isize)
            && k < (::core::mem::size_of::<[::core::ffi::c_char; 128]>() as usize)
                .wrapping_sub(1 as usize)
            && *p as ::core::ffi::c_int > 32 as ::core::ffi::c_int
            && (*p as ::core::ffi::c_int) < 127 as ::core::ffi::c_int
            && strchr(
                b"/()<>[]{}%\0" as *const u8 as *const ::core::ffi::c_char,
                *p as ::core::ffi::c_int,
            )
            .is_null()
        {
            let fresh6 = p;
            p = p.offset(1);
            let fresh7 = k;
            k = k.wrapping_add(1);
            (*t).fontname[fresh7 as usize] = *fresh6;
        }
        (*t).fontname[k as usize] = 0 as ::core::ffi::c_char;
    }
}
#[no_mangle]
pub unsafe extern "C" fn font_type1(
    mut fc: *mut font_cache,
    mut file: *const ::core::ffi::c_char,
    mut warnings: *mut pbuf,
) -> *mut type1_font {
    let mut e: *mut fc_entry = lookup(fc, K_TYPE1, file, 0 as ::core::ffi::c_int);
    if !e.is_null() {
        return (*e).value as *mut type1_font;
    }
    let mut t: *mut type1_font =
        xcalloc(1 as size_t, ::core::mem::size_of::<type1_font>() as size_t) as *mut type1_font;
    let mut b: *mut tbuf =
        (*fc).res.load.expect("non-null function pointer")((*fc).res.env, file, RES_TYPE1);
    insert(
        fc,
        K_TYPE1,
        file,
        0 as ::core::ffi::c_int,
        b,
        t as *mut ::core::ffi::c_void,
    );
    if b.is_null() {
        pbuf_printf(
            warnings,
            b"Type 1 font not found: %s\n\0" as *const u8 as *const ::core::ffi::c_char,
            file,
        );
        return t;
    }
    let mut d: *const ::core::ffi::c_uchar = (*b).data;
    let mut n: size_t = (*b).len;
    let mut clear: pbuf = pbuf {
        data: ::core::ptr::null_mut::<::core::ffi::c_uchar>(),
        len: 0,
        cap: 0,
    };
    let mut bin: pbuf = pbuf {
        data: ::core::ptr::null_mut::<::core::ffi::c_uchar>(),
        len: 0,
        cap: 0,
    };
    let mut trail: pbuf = pbuf {
        data: ::core::ptr::null_mut::<::core::ffi::c_uchar>(),
        len: 0,
        cap: 0,
    };
    if n > 6 as size_t
        && *d.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
            == 0x80 as ::core::ffi::c_int
    {
        let mut pos: size_t = 0 as size_t;
        while pos.wrapping_add(2 as size_t) <= n
            && *d.offset(pos as isize) as ::core::ffi::c_int == 0x80 as ::core::ffi::c_int
        {
            let mut type_0: ::core::ffi::c_int =
                *d.offset(pos.wrapping_add(1 as size_t) as isize) as ::core::ffi::c_int;
            if type_0 == 3 as ::core::ffi::c_int {
                break;
            }
            if pos.wrapping_add(6 as size_t) > n {
                break;
            }
            let mut l: uint32_t = *d.offset(pos.wrapping_add(2 as size_t) as isize) as uint32_t
                | (*d.offset(pos.wrapping_add(3 as size_t) as isize) as uint32_t)
                    << 8 as ::core::ffi::c_int
                | (*d.offset(pos.wrapping_add(4 as size_t) as isize) as uint32_t)
                    << 16 as ::core::ffi::c_int
                | (*d.offset(pos.wrapping_add(5 as size_t) as isize) as uint32_t)
                    << 24 as ::core::ffi::c_int;
            pos = pos.wrapping_add(6 as size_t);
            if l as size_t > n.wrapping_sub(pos) {
                break;
            }
            if type_0 == 1 as ::core::ffi::c_int {
                pbuf_append(
                    if bin.len != 0 {
                        &raw mut trail
                    } else {
                        &raw mut clear
                    },
                    d.offset(pos as isize) as *const ::core::ffi::c_void,
                    l as size_t,
                );
            } else if type_0 == 2 as ::core::ffi::c_int {
                pbuf_append(
                    &raw mut bin,
                    d.offset(pos as isize) as *const ::core::ffi::c_void,
                    l as size_t,
                );
            }
            pos = pos.wrapping_add(l as size_t);
        }
    } else {
        let mut s: *const ::core::ffi::c_char = d as *const ::core::ffi::c_char;
        let mut ex: *const ::core::ffi::c_char =
            memfind(s, n, b"eexec\0" as *const u8 as *const ::core::ffi::c_char);
        if !ex.is_null() {
            let mut cl: size_t =
                (ex.offset_from(s) as ::core::ffi::c_long as size_t).wrapping_add(5 as size_t);
            while cl < n
                && (*s.offset(cl as isize) as ::core::ffi::c_int == '\r' as i32
                    || *s.offset(cl as isize) as ::core::ffi::c_int == '\n' as i32)
            {
                cl = cl.wrapping_add(1);
            }
            pbuf_append(&raw mut clear, s as *const ::core::ffi::c_void, cl);
            let mut i: size_t = cl;
            let mut zeros: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
            let mut hi: ::core::ffi::c_int = -(1 as ::core::ffi::c_int);
            let mut zstart: size_t = n;
            while i < n {
                let mut c: ::core::ffi::c_int = *s.offset(i as isize) as ::core::ffi::c_int;
                if c == '0' as i32 {
                    let fresh5 = zeros;
                    zeros = zeros + 1;
                    if fresh5 == 0 as ::core::ffi::c_int {
                        zstart = i;
                    }
                    if zeros >= 64 as ::core::ffi::c_int {
                        break;
                    }
                } else if isspace(c) == 0 {
                    zeros = 0 as ::core::ffi::c_int;
                }
                let mut v: ::core::ffi::c_int = if isdigit(c) != 0 {
                    c - '0' as i32
                } else if c >= 'a' as i32 && c <= 'f' as i32 {
                    c - 'a' as i32 + 10 as ::core::ffi::c_int
                } else if c >= 'A' as i32 && c <= 'F' as i32 {
                    c - 'A' as i32 + 10 as ::core::ffi::c_int
                } else {
                    -(1 as ::core::ffi::c_int)
                };
                if !(v < 0 as ::core::ffi::c_int) {
                    if hi < 0 as ::core::ffi::c_int {
                        hi = v;
                    } else {
                        pbuf_putc(&raw mut bin, hi * 16 as ::core::ffi::c_int + v);
                        hi = -(1 as ::core::ffi::c_int);
                    }
                }
                i = i.wrapping_add(1);
            }
            let mut zbytes: size_t = 0 as size_t;
            let mut k: size_t = zstart;
            while k < i {
                zbytes = zbytes.wrapping_add(
                    (*s.offset(k as isize) as ::core::ffi::c_int == '0' as i32)
                        as ::core::ffi::c_int as size_t,
                );
                k = k.wrapping_add(1);
            }
            if bin.len >= zbytes.wrapping_div(2 as size_t) {
                bin.len = bin.len.wrapping_sub(zbytes.wrapping_div(2 as size_t));
            }
            if zstart < n {
                pbuf_append(
                    &raw mut trail,
                    s.offset(zstart as isize) as *const ::core::ffi::c_void,
                    n.wrapping_sub(zstart),
                );
            }
        }
    }
    if clear.len == 0 || bin.len == 0 {
        pbuf_printf(
            warnings,
            b"unrecognized Type 1 font file: %s\n\0" as *const u8 as *const ::core::ffi::c_char,
            file,
        );
        pbuf_free(&raw mut clear);
        pbuf_free(&raw mut bin);
        pbuf_free(&raw mut trail);
        return t;
    }
    (*t).len1 = clear.len;
    (*t).len2 = bin.len;
    (*t).len3 = trail.len;
    (*t).data = malloc(
        (*t).len1
            .wrapping_add((*t).len2)
            .wrapping_add((*t).len3)
            .wrapping_add(1 as size_t),
    ) as *mut ::core::ffi::c_uchar;
    if (*t).data.is_null() {
        abort();
    }
    memcpy(
        (*t).data as *mut ::core::ffi::c_void,
        clear.data as *const ::core::ffi::c_void,
        (*t).len1,
    );
    memcpy(
        (*t).data.offset((*t).len1 as isize) as *mut ::core::ffi::c_void,
        bin.data as *const ::core::ffi::c_void,
        (*t).len2,
    );
    if (*t).len3 != 0 {
        memcpy(
            (*t).data
                .offset((*t).len1 as isize)
                .offset((*t).len2 as isize) as *mut ::core::ffi::c_void,
            trail.data as *const ::core::ffi::c_void,
            (*t).len3,
        );
    }
    pbuf_free(&raw mut clear);
    pbuf_free(&raw mut bin);
    pbuf_free(&raw mut trail);
    (*t).bbox[0 as ::core::ffi::c_int as usize] = 0 as ::core::ffi::c_int as ::core::ffi::c_double;
    (*t).bbox[1 as ::core::ffi::c_int as usize] =
        -(250 as ::core::ffi::c_int) as ::core::ffi::c_double;
    (*t).bbox[2 as ::core::ffi::c_int as usize] =
        1000 as ::core::ffi::c_int as ::core::ffi::c_double;
    (*t).bbox[3 as ::core::ffi::c_int as usize] =
        750 as ::core::ffi::c_int as ::core::ffi::c_double;
    parse_type1_header(t);
    if (*t).fontname[0 as ::core::ffi::c_int as usize] == 0 {
        (*fc).anon_counter += 1;
        snprintf(
            &raw mut (*t).fontname as *mut ::core::ffi::c_char,
            ::core::mem::size_of::<[::core::ffi::c_char; 128]>() as size_t,
            b"PitexType1-%d\0" as *const u8 as *const ::core::ffi::c_char,
            (*fc).anon_counter,
        );
    }
    (*t).ok = true_0 != 0;
    return t;
}
#[no_mangle]
pub unsafe extern "C" fn font_enc(
    mut fc: *mut font_cache,
    mut file: *const ::core::ffi::c_char,
) -> *mut enc_vector {
    let mut e: *mut fc_entry = lookup(fc, K_ENC, file, 0 as ::core::ffi::c_int);
    if !e.is_null() {
        return (*e).value as *mut enc_vector;
    }
    let mut v: *mut enc_vector =
        xcalloc(1 as size_t, ::core::mem::size_of::<enc_vector>() as size_t) as *mut enc_vector;
    let mut b: *mut tbuf =
        (*fc).res.load.expect("non-null function pointer")((*fc).res.env, file, RES_ENC);
    insert(
        fc,
        K_ENC,
        file,
        0 as ::core::ffi::c_int,
        ::core::ptr::null_mut::<tbuf>(),
        v as *mut ::core::ffi::c_void,
    );
    if b.is_null() {
        return v;
    }
    let mut s: *const ::core::ffi::c_char = (*b).data as *const ::core::ffi::c_char;
    let mut end: *const ::core::ffi::c_char = s.offset((*b).len as isize);
    let mut in_array: bool = false_0 != 0;
    let mut idx: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while s < end && idx < 256 as ::core::ffi::c_int {
        let mut c: ::core::ffi::c_char = *s;
        if c as ::core::ffi::c_int == '%' as i32 {
            while s < end
                && *s as ::core::ffi::c_int != '\n' as i32
                && *s as ::core::ffi::c_int != '\r' as i32
            {
                s = s.offset(1);
            }
        } else if c as ::core::ffi::c_int == '[' as i32 {
            in_array = true_0 != 0;
            s = s.offset(1);
        } else {
            if c as ::core::ffi::c_int == ']' as i32 {
                break;
            }
            if c as ::core::ffi::c_int == '/' as i32 && in_array as ::core::ffi::c_int != 0 {
                s = s.offset(1);
                let mut st: *const ::core::ffi::c_char = s;
                while s < end
                    && *s as ::core::ffi::c_int > 32 as ::core::ffi::c_int
                    && strchr(
                        b"/[]{}()<>%\0" as *const u8 as *const ::core::ffi::c_char,
                        *s as ::core::ffi::c_int,
                    )
                    .is_null()
                {
                    s = s.offset(1);
                }
                let fresh8 = idx;
                idx = idx + 1;
                (*v).glyph[fresh8 as usize] =
                    strndup(st, s.offset_from(st) as ::core::ffi::c_long as size_t);
            } else if c as ::core::ffi::c_int == '/' as i32 && !in_array {
                s = s.offset(1);
                while s < end
                    && *s as ::core::ffi::c_int > 32 as ::core::ffi::c_int
                    && strchr(
                        b"/[]{}()<>%\0" as *const u8 as *const ::core::ffi::c_char,
                        *s as ::core::ffi::c_int,
                    )
                    .is_null()
                {
                    s = s.offset(1);
                }
            } else {
                s = s.offset(1);
            }
        }
    }
    tbuf_drop(b);
    (*v).ok = idx > 0 as ::core::ffi::c_int;
    return v;
}
#[no_mangle]
pub unsafe extern "C" fn font_vf(
    mut fc: *mut font_cache,
    mut name: *const ::core::ffi::c_char,
) -> *mut vf_font {
    let mut e: *mut fc_entry = lookup(fc, K_VF, name, 0 as ::core::ffi::c_int);
    if !e.is_null() {
        return (*e).value as *mut vf_font;
    }
    let mut b: *mut tbuf =
        (*fc).res.load.expect("non-null function pointer")((*fc).res.env, name, RES_VF);
    if b.is_null()
        || (*b).len < 11 as size_t
        || *(*b).data.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
            != 247 as ::core::ffi::c_int
        || *(*b).data.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
            != 202 as ::core::ffi::c_int
    {
        tbuf_drop(b);
        insert(
            fc,
            K_VF,
            name,
            0 as ::core::ffi::c_int,
            ::core::ptr::null_mut::<tbuf>(),
            NULL,
        );
        return ::core::ptr::null_mut::<vf_font>();
    }
    let mut v: *mut vf_font =
        xcalloc(1 as size_t, ::core::mem::size_of::<vf_font>() as size_t) as *mut vf_font;
    insert(
        fc,
        K_VF,
        name,
        0 as ::core::ffi::c_int,
        b,
        v as *mut ::core::ffi::c_void,
    );
    let mut d: *const ::core::ffi::c_uchar = (*b).data;
    let mut end: *const ::core::ffi::c_uchar = d.offset((*b).len as isize);
    let mut p: *const ::core::ffi::c_uchar = d
        .offset(3 as ::core::ffi::c_int as isize)
        .offset(*d.offset(2 as ::core::ffi::c_int as isize) as ::core::ffi::c_int as isize)
        .offset(8 as ::core::ffi::c_int as isize);
    let mut capf: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while p < end {
        let mut op: ::core::ffi::c_int = *p as ::core::ffi::c_int;
        if op >= 243 as ::core::ffi::c_int && op <= 246 as ::core::ffi::c_int {
            let mut k: ::core::ffi::c_int = op - 242 as ::core::ffi::c_int;
            if p.offset(1 as ::core::ffi::c_int as isize)
                .offset(k as isize)
                .offset(14 as ::core::ffi::c_int as isize)
                > end
            {
                break;
            }
            let mut num: int32_t = 0 as int32_t;
            let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
            while i < k {
                num = num << 8 as ::core::ffi::c_int
                    | *p.offset((1 as ::core::ffi::c_int + i) as isize) as int32_t;
                i += 1;
            }
            let mut q: *const ::core::ffi::c_uchar = p
                .offset(1 as ::core::ffi::c_int as isize)
                .offset(k as isize);
            let mut s: int32_t = rd32(q.offset(4 as ::core::ffi::c_int as isize)) as int32_t;
            let mut a: ::core::ffi::c_int =
                *q.offset(12 as ::core::ffi::c_int as isize) as ::core::ffi::c_int;
            let mut l: ::core::ffi::c_int =
                *q.offset(13 as ::core::ffi::c_int as isize) as ::core::ffi::c_int;
            if q.offset(14 as ::core::ffi::c_int as isize)
                .offset(a as isize)
                .offset(l as isize)
                > end
            {
                break;
            }
            if (*v).nfonts == capf {
                capf = if capf != 0 {
                    capf * 2 as ::core::ffi::c_int
                } else {
                    4 as ::core::ffi::c_int
                };
                (*v).fonts = realloc(
                    (*v).fonts as *mut ::core::ffi::c_void,
                    (::core::mem::size_of::<vf_fontdef>() as size_t).wrapping_mul(capf as size_t),
                ) as *mut vf_fontdef;
                if (*v).fonts.is_null() {
                    abort();
                }
            }
            (*(*v).fonts.offset((*v).nfonts as isize)).k = num;
            (*(*v).fonts.offset((*v).nfonts as isize)).scaled_fix = s;
            let ref mut fresh9 = (*(*v).fonts.offset((*v).nfonts as isize)).name;
            *fresh9 = strndup(
                (q as *const ::core::ffi::c_char)
                    .offset(14 as ::core::ffi::c_int as isize)
                    .offset(a as isize),
                l as size_t,
            );
            (*v).nfonts += 1;
            p = q
                .offset(14 as ::core::ffi::c_int as isize)
                .offset(a as isize)
                .offset(l as isize);
        } else if op < 242 as ::core::ffi::c_int {
            if p.offset(5 as ::core::ffi::c_int as isize)
                .offset(op as isize)
                > end
            {
                break;
            }
            let mut cc: ::core::ffi::c_int =
                *p.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int;
            let mut w: int32_t = ((*p.offset(2 as ::core::ffi::c_int as isize) as uint32_t)
                << 16 as ::core::ffi::c_int
                | (*p.offset(3 as ::core::ffi::c_int as isize) as uint32_t)
                    << 8 as ::core::ffi::c_int
                | *p.offset(4 as ::core::ffi::c_int as isize) as uint32_t)
                as int32_t;
            if w & 0x800000 as int32_t != 0 {
                w = (w as ::core::ffi::c_int - 0x1000000 as ::core::ffi::c_int) as int32_t;
            }
            (*v).chars[cc as usize] = vf_char {
                packet: p.offset(5 as ::core::ffi::c_int as isize),
                len: op as uint32_t,
                width_fix: w,
                exists: true_0 != 0,
            };
            p = p.offset((5 as ::core::ffi::c_int + op) as isize);
        } else {
            if !(op == 242 as ::core::ffi::c_int) {
                break;
            }
            if p.offset(13 as ::core::ffi::c_int as isize) > end {
                break;
            }
            let mut pl: uint32_t = rd32(p.offset(1 as ::core::ffi::c_int as isize));
            let mut cc_0: uint32_t = rd32(p.offset(5 as ::core::ffi::c_int as isize));
            let mut w_0: int32_t = rd32(p.offset(9 as ::core::ffi::c_int as isize)) as int32_t;
            if pl as size_t
                > (end.offset_from(p) as ::core::ffi::c_long - 13 as ::core::ffi::c_long) as size_t
            {
                break;
            }
            if cc_0 < 256 as uint32_t {
                (*v).chars[cc_0 as usize] = vf_char {
                    packet: p.offset(13 as ::core::ffi::c_int as isize),
                    len: pl,
                    width_fix: w_0,
                    exists: true_0 != 0,
                };
            }
            p = p.offset((13 as uint32_t).wrapping_add(pl) as isize);
        }
    }
    (*v).ok = true_0 != 0;
    return v;
}
pub const true_0: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const false_0: ::core::ffi::c_int = 0 as ::core::ffi::c_int;

#[no_mangle]
pub unsafe extern "C" fn font_map_duplicate_count(fc:*mut font_cache)->i32 {(*fc).duplicate_maps}
#[no_mangle]
pub unsafe extern "C" fn font_map_ps_lookup(fc:*mut font_cache,psname:*const ::core::ffi::c_char)->*const map_entry {
    if !(*fc).map_loaded {load_map(fc);}
    for index in 0..(*fc).nmap {let entry=(*fc).map.add(index as usize);if !(*entry).psname.is_null()&&strcmp((*entry).psname,psname)==0{return entry;}}
    std::ptr::null()
}
