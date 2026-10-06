/* Pitex embedded preview engine — reference-counted byte buffer.
 * Pitex-authored (AGPL-3.0-or-later). */
// Translated from driver/tbuf.c with C2Rust 0.22.1.
extern "C" {
    pub type _IO_wide_data;
    pub type _IO_codecvt;
    pub type _IO_marker;
    fn fclose(__stream: *mut FILE) -> ::core::ffi::c_int;
    fn fopen(
        __filename: *const ::core::ffi::c_char,
        __modes: *const ::core::ffi::c_char,
    ) -> *mut FILE;
    fn fread(
        __ptr: *mut ::core::ffi::c_void,
        __size: size_t,
        __n: size_t,
        __stream: *mut FILE,
    ) -> ::core::ffi::c_ulong;
    fn ferror(__stream: *mut FILE) -> ::core::ffi::c_int;
    fn malloc(__size: size_t) -> *mut ::core::ffi::c_void;
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
}
pub type size_t = usize;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct tbuf {
    pub data: *mut ::core::ffi::c_uchar,
    pub len: size_t,
    pub cap: size_t,
    pub refs: ::core::ffi::c_int,
}
pub type FILE = _IO_FILE;
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
pub type __off64_t = ::core::ffi::c_long;
pub type _IO_lock_t = ();
pub type __off_t = ::core::ffi::c_long;
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<
    ::core::ffi::c_void,
>();
#[no_mangle]
pub unsafe extern "C" fn tbuf_new(mut cap: size_t) -> *mut tbuf {
    let mut b: *mut tbuf = calloc(1 as size_t, ::core::mem::size_of::<tbuf>() as size_t)
        as *mut tbuf;
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
pub unsafe extern "C" fn tbuf_read_file(
    mut path: *const ::core::ffi::c_char,
) -> *mut tbuf {
    let mut f: *mut FILE = fopen(
        path,
        b"rb\0" as *const u8 as *const ::core::ffi::c_char,
    ) as *mut FILE;
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
    if b.is_null()
        || {
            (*b).refs -= 1;
            (*b).refs > 0 as ::core::ffi::c_int
        }
    {
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
        (*b).data = realloc((*b).data as *mut ::core::ffi::c_void, cap)
            as *mut ::core::ffi::c_uchar;
        if (*b).data.is_null() {
            abort();
        }
        (*b).cap = cap;
    }
    memcpy((*b).data.offset((*b).len as isize) as *mut ::core::ffi::c_void, data, len);
    (*b).len = (*b).len.wrapping_add(len);
}
