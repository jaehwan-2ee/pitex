/* Pitex embedded preview engine — growable byte buffer.
 * Pitex-authored (AGPL-3.0-or-later, see repository LICENSE). */
// Translated from shared/pitex_buf.c with C2Rust 0.22.1.
extern "C" {
    pub type internal_state;
    fn realloc(__ptr: *mut ::core::ffi::c_void, __size: size_t) -> *mut ::core::ffi::c_void;
    fn abort() -> !;
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
    fn fabs(_: ::core::ffi::c_double) -> ::core::ffi::c_double;
    fn snprintf(
        __str: *mut ::core::ffi::c_char,
        __size: size_t,
        __format: *const ::core::ffi::c_char,
        ...
    ) -> ::core::ffi::c_int;
    fn vsnprintf(
        __str: *mut ::core::ffi::c_char,
        __size: size_t,
        __format: *const ::core::ffi::c_char,
        _: ::core::ffi::VaList,
    ) -> ::core::ffi::c_int;
    fn deflate(strm: z_streamp, flush: ::core::ffi::c_int) -> ::core::ffi::c_int;
    fn deflateEnd(strm: z_streamp) -> ::core::ffi::c_int;
    fn inflate(strm: z_streamp, flush: ::core::ffi::c_int) -> ::core::ffi::c_int;
    fn inflateEnd(strm: z_streamp) -> ::core::ffi::c_int;
    fn deflateBound(strm: z_streamp, sourceLen: uLong) -> uLong;
    fn deflateInit_(
        strm: z_streamp,
        level: ::core::ffi::c_int,
        version: *const ::core::ffi::c_char,
        stream_size: ::core::ffi::c_int,
    ) -> ::core::ffi::c_int;
    fn inflateInit_(
        strm: z_streamp,
        version: *const ::core::ffi::c_char,
        stream_size: ::core::ffi::c_int,
    ) -> ::core::ffi::c_int;
}
pub type __builtin_va_list = *mut ::core::ffi::c_char;
pub type va_list = __builtin_va_list;
pub type __darwin_size_t = usize;
pub type size_t = __darwin_size_t;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct pbuf {
    pub data: *mut ::core::ffi::c_uchar,
    pub len: size_t,
    pub cap: size_t,
}
pub type z_stream = z_stream_s;
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
pub type uLong = ::core::ffi::c_ulong;
pub type voidpf = *mut ::core::ffi::c_void;
pub type free_func = Option<unsafe extern "C" fn(voidpf, voidpf) -> ()>;
pub type alloc_func = Option<unsafe extern "C" fn(voidpf, uInt, uInt) -> voidpf>;
pub type uInt = ::core::ffi::c_uint;
pub type Bytef = Byte;
pub type Byte = ::core::ffi::c_uchar;
pub type z_streamp = *mut z_stream;
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
#[inline(always)]
unsafe extern "C" fn __inline_isfinitef(mut __x: ::core::ffi::c_float) -> ::core::ffi::c_int {
    return (__x == __x && __x.abs() != ::core::f32::INFINITY) as ::core::ffi::c_int;
}
#[inline(always)]
unsafe extern "C" fn __inline_isfinited(mut __x: ::core::ffi::c_double) -> ::core::ffi::c_int {
    return (__x == __x && __x.abs() != ::core::f64::INFINITY) as ::core::ffi::c_int;
}
#[inline(always)]
unsafe extern "C" fn __inline_isfinitel(mut __x: ::core::ffi::c_double) -> ::core::ffi::c_int {
    return (__x == __x && __x.abs() != ::core::f64::INFINITY) as ::core::ffi::c_int;
}
pub const ZLIB_VERSION: [::core::ffi::c_char; 7] =
    unsafe { ::core::mem::transmute::<[u8; 7], [::core::ffi::c_char; 7]>(*b"1.2.12\0") };
pub const Z_NO_FLUSH: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const Z_FINISH: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
pub const Z_OK: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const Z_STREAM_END: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const Z_BUF_ERROR: ::core::ffi::c_int = -(5 as ::core::ffi::c_int);
#[no_mangle]
pub unsafe extern "C" fn pbuf_printf(
    mut b: *mut pbuf,
    mut fmt: *const ::core::ffi::c_char,
    mut args: ...
) {
    let mut ap: ::core::ffi::VaList<'_>;
    ap = args.clone();
    let mut small: [::core::ffi::c_char; 256] = [0; 256];
    let mut n: ::core::ffi::c_int = vsnprintf(
        &raw mut small as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 256]>() as size_t,
        fmt,
        ap.clone(),
    );
    if n < 0 as ::core::ffi::c_int {
        return;
    }
    if (n as size_t) < ::core::mem::size_of::<[::core::ffi::c_char; 256]>() as usize {
        pbuf_append(
            b,
            &raw mut small as *mut ::core::ffi::c_char as *const ::core::ffi::c_void,
            n as size_t,
        );
        return;
    }
    pbuf_reserve(b, (n as size_t).wrapping_add(1 as size_t));
    ap = args.clone();
    vsnprintf(
        ((*b).data as *mut ::core::ffi::c_char).offset((*b).len as isize),
        (n as size_t).wrapping_add(1 as size_t),
        fmt,
        ap.clone(),
    );
    (*b).len = (*b).len.wrapping_add(n as size_t);
}
#[no_mangle]
pub unsafe extern "C" fn pbuf_real(mut b: *mut pbuf, mut v: ::core::ffi::c_double) {
    if if ::core::mem::size_of::<::core::ffi::c_double>() as usize
        == ::core::mem::size_of::<::core::ffi::c_float>() as usize
    {
        __inline_isfinitef(v as ::core::ffi::c_float)
    } else if ::core::mem::size_of::<::core::ffi::c_double>() as usize
        == ::core::mem::size_of::<::core::ffi::c_double>() as usize
    {
        __inline_isfinited(v)
    } else {
        __inline_isfinitel(v)
    } == 0
    {
        v = 0 as ::core::ffi::c_int as ::core::ffi::c_double;
    }
    if fabs(v) < 5e-7f64 {
        v = 0 as ::core::ffi::c_int as ::core::ffi::c_double;
    }
    let mut tmp: [::core::ffi::c_char; 64] = [0; 64];
    let mut n: ::core::ffi::c_int = snprintf(
        &raw mut tmp as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 64]>() as size_t,
        b"%.6f\0" as *const u8 as *const ::core::ffi::c_char,
        v,
    );
    while n > 0 as ::core::ffi::c_int
        && tmp[(n - 1 as ::core::ffi::c_int) as usize] as ::core::ffi::c_int == '0' as i32
    {
        n -= 1;
    }
    if n > 0 as ::core::ffi::c_int
        && tmp[(n - 1 as ::core::ffi::c_int) as usize] as ::core::ffi::c_int == '.' as i32
    {
        n -= 1;
    }
    if n == 0 as ::core::ffi::c_int
        || n == 1 as ::core::ffi::c_int
            && tmp[0 as ::core::ffi::c_int as usize] as ::core::ffi::c_int == '-' as i32
    {
        tmp[0 as ::core::ffi::c_int as usize] = '0' as i32 as ::core::ffi::c_char;
        n = 1 as ::core::ffi::c_int;
    }
    if n == 2 as ::core::ffi::c_int
        && tmp[0 as ::core::ffi::c_int as usize] as ::core::ffi::c_int == '-' as i32
        && tmp[1 as ::core::ffi::c_int as usize] as ::core::ffi::c_int == '0' as i32
    {
        tmp[0 as ::core::ffi::c_int as usize] = '0' as i32 as ::core::ffi::c_char;
        n = 1 as ::core::ffi::c_int;
    }
    pbuf_append(
        b,
        &raw mut tmp as *mut ::core::ffi::c_char as *const ::core::ffi::c_void,
        n as size_t,
    );
}
#[no_mangle]
pub unsafe extern "C" fn pbuf_deflate(
    mut out: *mut pbuf,
    mut data: *const ::core::ffi::c_uchar,
    mut len: size_t,
    mut level: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut zs: z_stream = z_stream_s {
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
    memset(
        &raw mut zs as *mut ::core::ffi::c_void,
        0 as ::core::ffi::c_int,
        ::core::mem::size_of::<z_stream>() as size_t,
    );
    if deflateInit_(
        &raw mut zs,
        level,
        ZLIB_VERSION.as_ptr(),
        ::core::mem::size_of::<z_stream>() as ::core::ffi::c_int,
    ) != Z_OK
    {
        return -(1 as ::core::ffi::c_int);
    }
    pbuf_reserve(out, deflateBound(&raw mut zs, len as uLong) as size_t);
    zs.next_in = data as *mut Bytef;
    zs.avail_in = len as uInt;
    zs.next_out = (*out).data.offset((*out).len as isize) as *mut Bytef;
    zs.avail_out = (*out).cap.wrapping_sub((*out).len) as uInt;
    let mut r: ::core::ffi::c_int = deflate(&raw mut zs, Z_FINISH);
    (*out).len = ((*out).len as ::core::ffi::c_ulong)
        .wrapping_add(zs.total_out as ::core::ffi::c_ulong) as size_t as size_t;
    deflateEnd(&raw mut zs);
    return if r == Z_STREAM_END {
        0 as ::core::ffi::c_int
    } else {
        -(1 as ::core::ffi::c_int)
    };
}
#[no_mangle]
pub unsafe extern "C" fn pbuf_inflate(
    mut out: *mut pbuf,
    mut data: *const ::core::ffi::c_uchar,
    mut len: size_t,
) -> ::core::ffi::c_int {
    let mut zs: z_stream = z_stream_s {
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
    memset(
        &raw mut zs as *mut ::core::ffi::c_void,
        0 as ::core::ffi::c_int,
        ::core::mem::size_of::<z_stream>() as size_t,
    );
    if inflateInit_(
        &raw mut zs,
        ZLIB_VERSION.as_ptr(),
        ::core::mem::size_of::<z_stream>() as ::core::ffi::c_int,
    ) != Z_OK
    {
        return -(1 as ::core::ffi::c_int);
    }
    zs.next_in = data as *mut Bytef;
    zs.avail_in = len as uInt;
    let mut r: ::core::ffi::c_int = Z_OK;
    let mut start: size_t = (*out).len;
    while r == Z_OK {
        if (*out).len.wrapping_sub(start)
            > (1 as ::core::ffi::c_int as size_t) << 30 as ::core::ffi::c_int
        {
            break;
        }
        pbuf_reserve(
            out,
            if len > 4096 as size_t {
                len
            } else {
                4096 as size_t
            },
        );
        zs.next_out = (*out).data.offset((*out).len as isize) as *mut Bytef;
        zs.avail_out = (*out).cap.wrapping_sub((*out).len) as uInt;
        let mut before: size_t = zs.total_out as size_t;
        r = inflate(&raw mut zs, Z_NO_FLUSH);
        (*out).len = ((*out).len as ::core::ffi::c_ulong)
            .wrapping_add((zs.total_out as size_t).wrapping_sub(before) as ::core::ffi::c_ulong)
            as size_t as size_t;
        if r == Z_BUF_ERROR && zs.avail_in == 0 as uInt {
            break;
        }
    }
    inflateEnd(&raw mut zs);
    return if r == Z_STREAM_END || (*out).len > start {
        0 as ::core::ffi::c_int
    } else {
        -(1 as ::core::ffi::c_int)
    };
}
