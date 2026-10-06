/* Pitex embedded preview engine — reference-counted byte buffer.
 * Pitex-authored (AGPL-3.0-or-later). */
// Translated from driver/tbuf.c with C2Rust 0.22.1.
extern "C" {
    pub type __sFILEX;
    fn fclose(_: *mut FILE) -> ::core::ffi::c_int;
    fn ferror(_: *mut FILE) -> ::core::ffi::c_int;
    fn fopen(
        __filename: *const ::core::ffi::c_char,
        __mode: *const ::core::ffi::c_char,
    ) -> *mut FILE;
    fn fread(
        __ptr: *mut ::core::ffi::c_void,
        __size: size_t,
        __nitems: size_t,
        __stream: *mut FILE,
    ) -> ::core::ffi::c_ulong;
    fn malloc(__size: size_t) -> *mut ::core::ffi::c_void;
    fn calloc(__count: size_t, __size: size_t) -> *mut ::core::ffi::c_void;
    fn free(_: *mut ::core::ffi::c_void);
    fn realloc(__ptr: *mut ::core::ffi::c_void, __size: size_t) -> *mut ::core::ffi::c_void;
    fn abort() -> !;
    fn memcpy(
        __dst: *mut ::core::ffi::c_void,
        __src: *const ::core::ffi::c_void,
        __n: size_t,
    ) -> *mut ::core::ffi::c_void;
}
pub type __int64_t = i64;
pub type __darwin_size_t = usize;
pub type __darwin_off_t = __int64_t;
pub type size_t = __darwin_size_t;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct tbuf {
    pub data: *mut ::core::ffi::c_uchar,
    pub len: size_t,
    pub cap: size_t,
    pub refs: ::core::ffi::c_int,
}
pub type FILE = __sFILE;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct __sFILE {
    pub _p: *mut ::core::ffi::c_uchar,
    pub _r: ::core::ffi::c_int,
    pub _w: ::core::ffi::c_int,
    pub _flags: ::core::ffi::c_short,
    pub _file: ::core::ffi::c_short,
    pub _bf: __sbuf,
    pub _lbfsize: ::core::ffi::c_int,
    pub _cookie: *mut ::core::ffi::c_void,
    pub _close: Option<unsafe extern "C" fn(*mut ::core::ffi::c_void) -> ::core::ffi::c_int>,
    pub _read: Option<
        unsafe extern "C" fn(
            *mut ::core::ffi::c_void,
            *mut ::core::ffi::c_char,
            ::core::ffi::c_int,
        ) -> ::core::ffi::c_int,
    >,
    pub _seek: Option<
        unsafe extern "C" fn(*mut ::core::ffi::c_void, fpos_t, ::core::ffi::c_int) -> fpos_t,
    >,
    pub _write: Option<
        unsafe extern "C" fn(
            *mut ::core::ffi::c_void,
            *const ::core::ffi::c_char,
            ::core::ffi::c_int,
        ) -> ::core::ffi::c_int,
    >,
    pub _ub: __sbuf,
    pub _extra: *mut __sFILEX,
    pub _ur: ::core::ffi::c_int,
    pub _ubuf: [::core::ffi::c_uchar; 3],
    pub _nbuf: [::core::ffi::c_uchar; 1],
    pub _lb: __sbuf,
    pub _blksize: ::core::ffi::c_int,
    pub _offset: fpos_t,
}
pub type fpos_t = __darwin_off_t;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct __sbuf {
    pub _base: *mut ::core::ffi::c_uchar,
    pub _size: ::core::ffi::c_int,
}
pub const __DARWIN_NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
pub const NULL: *mut ::core::ffi::c_void = __DARWIN_NULL;
#[no_mangle]
pub unsafe extern "C" fn tbuf_new(mut cap: size_t) -> *mut tbuf {
    let mut b: *mut tbuf =
        calloc(1 as size_t, ::core::mem::size_of::<tbuf>() as size_t) as *mut tbuf;
    if b.is_null() {
        abort();
    }
    (*b).cap = if cap != 0 { cap } else { 16 as size_t };
    (*b).data = malloc((*b).cap) as *mut ::core::ffi::c_uchar;
    if (*b).data.is_null() {
        abort();
    }
    (*b).refs = 1 as ::core::ffi::c_int;
    return b;
}
#[no_mangle]
pub unsafe extern "C" fn tbuf_from_copy(
    mut data: *const ::core::ffi::c_void,
    mut len: size_t,
) -> *mut tbuf {
    let mut b: *mut tbuf = tbuf_new(len.wrapping_add(1 as size_t));
    if len != 0 {
        memcpy((*b).data as *mut ::core::ffi::c_void, data, len);
    }
    (*b).len = len;
    return b;
}
#[no_mangle]
pub unsafe extern "C" fn tbuf_read_file(mut path: *const ::core::ffi::c_char) -> *mut tbuf {
    let mut f: *mut FILE =
        fopen(path, b"rb\0" as *const u8 as *const ::core::ffi::c_char) as *mut FILE;
    if f.is_null() {
        return ::core::ptr::null_mut::<tbuf>();
    }
    let mut b: *mut tbuf = tbuf_new(4096 as size_t);
    loop {
        if (*b).len == (*b).cap {
            (*b).cap = (*b).cap.wrapping_mul(2 as size_t);
            (*b).data = realloc((*b).data as *mut ::core::ffi::c_void, (*b).cap)
                as *mut ::core::ffi::c_uchar;
            if (*b).data.is_null() {
                abort();
            }
        }
        let mut n: size_t = fread(
            (*b).data.offset((*b).len as isize) as *mut ::core::ffi::c_void,
            1 as size_t,
            (*b).cap.wrapping_sub((*b).len),
            f,
        ) as size_t;
        if n == 0 as size_t {
            break;
        }
        (*b).len = (*b).len.wrapping_add(n);
    }
    let mut err: ::core::ffi::c_int = ferror(f);
    fclose(f);
    if err != 0 {
        tbuf_drop(b);
        return ::core::ptr::null_mut::<tbuf>();
    }
    return b;
}
#[no_mangle]
pub unsafe extern "C" fn tbuf_keep(mut b: *mut tbuf) -> *mut tbuf {
    if !b.is_null() {
        (*b).refs += 1;
    }
    return b;
}
#[no_mangle]
pub unsafe extern "C" fn tbuf_drop(mut b: *mut tbuf) {
    if b.is_null() || {
        (*b).refs -= 1;
        (*b).refs > 0 as ::core::ffi::c_int
    } {
        return;
    }
    free((*b).data as *mut ::core::ffi::c_void);
    free(b as *mut ::core::ffi::c_void);
}
#[no_mangle]
pub unsafe extern "C" fn tbuf_append(
    mut b: *mut tbuf,
    mut data: *const ::core::ffi::c_void,
    mut len: size_t,
) {
    if (*b).len.wrapping_add(len) > (*b).cap {
        let mut cap: size_t = (*b).cap;
        while cap < (*b).len.wrapping_add(len) {
            cap = cap.wrapping_mul(2 as size_t);
        }
        (*b).data =
            realloc((*b).data as *mut ::core::ffi::c_void, cap) as *mut ::core::ffi::c_uchar;
        if (*b).data.is_null() {
            abort();
        }
        (*b).cap = cap;
    }
    memcpy(
        (*b).data.offset((*b).len as isize) as *mut ::core::ffi::c_void,
        data,
        len,
    );
    (*b).len = (*b).len.wrapping_add(len);
}
