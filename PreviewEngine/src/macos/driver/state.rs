/*
 * MIT License
 *
 * Copyright (c) 2023 Frédéric Bour <frederic.bour@lakaban.net>
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to
 * deal in the Software without restriction, including without limitation the
 * rights to use, copy, modify, merge, publish, distribute, sublicense, and/or
 * sell copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in
 * all copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING
 * FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS
 * IN THE SOFTWARE.
 */
// Translated from driver/state.c with C2Rust 0.22.1.
extern "C" {
    pub type __sFILEX;
    fn calloc(__count: size_t, __size: size_t) -> *mut ::core::ffi::c_void;
    fn free(_: *mut ::core::ffi::c_void);
    fn abort() -> !;
    static mut __stderrp: *mut FILE;
    fn fprintf(_: *mut FILE, _: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
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
    fn tbuf_new(cap: size_t) -> *mut tbuf;
    fn tbuf_keep(b: *mut tbuf) -> *mut tbuf;
    fn tbuf_drop(b: *mut tbuf);
    fn tbuf_append(b: *mut tbuf, data: *const ::core::ffi::c_void, len: size_t);
}
pub type __uint16_t = u16;
pub type __int32_t = i32;
pub type __uint32_t = u32;
pub type __int64_t = i64;
pub type __uint64_t = u64;
pub type __darwin_size_t = usize;
pub type __darwin_time_t = ::core::ffi::c_long;
pub type __darwin_blkcnt_t = __int64_t;
pub type __darwin_blksize_t = __int32_t;
pub type __darwin_dev_t = __int32_t;
pub type __darwin_gid_t = __uint32_t;
pub type __darwin_ino64_t = __uint64_t;
pub type __darwin_mode_t = __uint16_t;
pub type __darwin_off_t = __int64_t;
pub type __darwin_uid_t = __uint32_t;
pub type size_t = __darwin_size_t;
pub type uid_t = __darwin_uid_t;
pub type uint8_t = u8;
pub type dev_t = __darwin_dev_t;
pub type mode_t = __darwin_mode_t;
pub type fpos_t = __darwin_off_t;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct __sbuf {
    pub _base: *mut ::core::ffi::c_uchar,
    pub _size: ::core::ffi::c_int,
}
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
pub type FILE = __sFILE;
pub type off_t = __darwin_off_t;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct timespec {
    pub tv_sec: __darwin_time_t,
    pub tv_nsec: ::core::ffi::c_long,
}
pub type blkcnt_t = __darwin_blkcnt_t;
pub type blksize_t = __darwin_blksize_t;
pub type nlink_t = __uint16_t;
pub type gid_t = __darwin_gid_t;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct stat {
    pub st_dev: dev_t,
    pub st_mode: mode_t,
    pub st_nlink: nlink_t,
    pub st_ino: __darwin_ino64_t,
    pub st_uid: uid_t,
    pub st_gid: gid_t,
    pub st_rdev: dev_t,
    pub st_atimespec: timespec,
    pub st_mtimespec: timespec,
    pub st_ctimespec: timespec,
    pub st_birthtimespec: timespec,
    pub st_size: off_t,
    pub st_blocks: blkcnt_t,
    pub st_blksize: blksize_t,
    pub st_flags: __uint32_t,
    pub st_gen: __uint32_t,
    pub st_lspare: __int32_t,
    pub st_qspare: [__int64_t; 2],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct pic_cache {
    pub type_0: ::core::ffi::c_int,
    pub page: ::core::ffi::c_int,
    pub bounds: [::core::ffi::c_float; 4],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct tbuf {
    pub data: *mut ::core::ffi::c_uchar,
    pub len: size_t,
    pub cap: size_t,
    pub refs: ::core::ffi::c_int,
}
pub type accesslevel = ::core::ffi::c_uint;
pub const FILE_WRITE: accesslevel = 2;
pub const FILE_READ: accesslevel = 1;
pub const FILE_NONE: accesslevel = 0;
pub type mark_t = ::core::ffi::c_int;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct fileentry_s {
    pub path: *const ::core::ffi::c_char,
    pub fs_stat: stat,
    pub fs_data: *mut tbuf,
    pub pic_cache: pic_cache,
    pub edit_data: *mut tbuf,
    pub promised: bool,
    pub edit_data_from_convergence: bool,
    pub saved: C2RustUnnamed,
    pub seen: ::core::ffi::c_int,
    pub debug_rollback_invalidation: ::core::ffi::c_int,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed {
    pub data: *mut tbuf,
    pub level: accesslevel,
    pub snap: mark_t,
}
pub type fileentry_t = fileentry_s;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct filecell_s {
    pub snap: mark_t,
    pub entry: *mut fileentry_t,
}
pub type filecell_t = filecell_s;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct state_t {
    pub table: [filecell_t; 1024],
    pub __stdoutp: filecell_t,
    pub document: filecell_t,
    pub synctex: filecell_t,
    pub log: filecell_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct log_s {
    pub snap: mark_t,
    pub data: *mut tbuf,
}
pub type log_t = log_s;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct overwrite_data {
    pub buf: *mut tbuf,
    pub start: ::core::ffi::c_int,
    pub len: ::core::ffi::c_int,
}
pub const LOG_OVERWRITE: log_action = 68;
pub const LOG_CELL: log_action = 67;
pub const LOG_ENTRY: log_action = 66;
pub type log_action = ::core::ffi::c_uint;
pub const LOG: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
#[no_mangle]
pub unsafe extern "C" fn log_new() -> *mut log_t {
    let mut log: *mut log_t =
        calloc(1 as size_t, ::core::mem::size_of::<log_t>() as size_t) as *mut log_t;
    if log.is_null() {
        abort();
    }
    (*log).data = tbuf_new(512 as size_t);
    (*log).snap = 1 as ::core::ffi::c_int as mark_t;
    let mut zero: uint8_t = 0 as uint8_t;
    tbuf_append(
        (*log).data,
        &raw mut zero as *const ::core::ffi::c_void,
        1 as size_t,
    );
    return log;
}
#[no_mangle]
pub unsafe extern "C" fn log_free(mut log: *mut log_t) {
    tbuf_drop((*log).data);
    free(log as *mut ::core::ffi::c_void);
}
unsafe extern "C" fn pop_value(
    mut buf: *mut tbuf,
    mut val: *mut ::core::ffi::c_void,
    mut len: size_t,
) {
    if (*buf).len < len {
        abort();
    }
    (*buf).len = (*buf).len.wrapping_sub(len);
    memcpy(
        val,
        (*buf).data.offset((*buf).len as isize) as *const ::core::ffi::c_void,
        len,
    );
}
unsafe extern "C" fn push_action(mut buf: *mut tbuf, mut action: log_action) {
    let mut b: uint8_t = action as uint8_t;
    tbuf_append(
        buf,
        &raw mut b as *const ::core::ffi::c_void,
        ::core::mem::size_of::<uint8_t>() as size_t,
    );
}
#[no_mangle]
pub unsafe extern "C" fn log_fileentry(mut log: *mut log_t, mut entry: *mut fileentry_t) {
    if (*entry).saved.snap != (*log).snap {
        if !(*entry).saved.data.is_null() {
            tbuf_keep((*entry).saved.data);
            tbuf_append(
                (*log).data,
                &raw mut (*(*entry).saved.data).len as *const ::core::ffi::c_void,
                ::core::mem::size_of::<size_t>() as size_t,
            );
        }
        tbuf_append(
            (*log).data,
            &raw mut (*entry).saved as *const ::core::ffi::c_void,
            ::core::mem::size_of::<C2RustUnnamed>() as size_t,
        );
        tbuf_append(
            (*log).data,
            &raw mut entry as *const ::core::ffi::c_void,
            ::core::mem::size_of::<*mut fileentry_t>() as size_t,
        );
        push_action((*log).data, LOG_ENTRY);
        (*entry).saved.snap = (*log).snap;
    }
}
#[no_mangle]
pub unsafe extern "C" fn log_filecell(mut log: *mut log_t, mut cell: *mut filecell_t) {
    if (*cell).snap != (*log).snap {
        tbuf_append(
            (*log).data,
            cell as *const ::core::ffi::c_void,
            ::core::mem::size_of::<filecell_t>() as size_t,
        );
        tbuf_append(
            (*log).data,
            &raw mut cell as *const ::core::ffi::c_void,
            ::core::mem::size_of::<*mut filecell_t>() as size_t,
        );
        push_action((*log).data, LOG_CELL);
        (*cell).snap = (*log).snap;
    }
}
#[no_mangle]
pub unsafe extern "C" fn log_overwrite(
    mut log: *mut log_t,
    mut buf: *mut tbuf,
    mut start: ::core::ffi::c_int,
    mut len: ::core::ffi::c_int,
) {
    tbuf_keep(buf);
    tbuf_append(
        (*log).data,
        (*buf).data.offset(start as isize) as *const ::core::ffi::c_void,
        len as size_t,
    );
    let mut data: overwrite_data = overwrite_data {
        buf: buf,
        start: start,
        len: len,
    };
    tbuf_append(
        (*log).data,
        &raw mut data as *const ::core::ffi::c_void,
        ::core::mem::size_of::<overwrite_data>() as size_t,
    );
    push_action((*log).data, LOG_OVERWRITE);
}
unsafe extern "C" fn pop_action(mut buf: *mut tbuf) -> log_action {
    let mut b: uint8_t = 0;
    pop_value(
        buf,
        &raw mut b as *mut ::core::ffi::c_void,
        ::core::mem::size_of::<uint8_t>() as size_t,
    );
    return b as log_action;
}
unsafe extern "C" fn log_pop(mut log: *mut log_t) {
    match pop_action((*log).data) as ::core::ffi::c_uint {
        66 => {
            let mut entry: *mut fileentry_t = ::core::ptr::null_mut::<fileentry_t>();
            pop_value(
                (*log).data,
                &raw mut entry as *mut ::core::ffi::c_void,
                ::core::mem::size_of::<*mut fileentry_t>() as size_t,
            );
            if !(*entry).saved.data.is_null() {
                tbuf_drop((*entry).saved.data);
            }
            pop_value(
                (*log).data,
                &raw mut (*entry).saved as *mut ::core::ffi::c_void,
                ::core::mem::size_of::<C2RustUnnamed>() as size_t,
            );
            if !(*entry).saved.data.is_null() {
                pop_value(
                    (*log).data,
                    &raw mut (*(*entry).saved.data).len as *mut ::core::ffi::c_void,
                    ::core::mem::size_of::<size_t>() as size_t,
                );
            }
        }
        67 => {
            let mut cell: *mut filecell_t = ::core::ptr::null_mut::<filecell_t>();
            pop_value(
                (*log).data,
                &raw mut cell as *mut ::core::ffi::c_void,
                ::core::mem::size_of::<*mut filecell_t>() as size_t,
            );
            pop_value(
                (*log).data,
                cell as *mut ::core::ffi::c_void,
                ::core::mem::size_of::<filecell_t>() as size_t,
            );
        }
        68 => {
            let mut data: overwrite_data = overwrite_data {
                buf: ::core::ptr::null_mut::<tbuf>(),
                start: 0,
                len: 0,
            };
            pop_value(
                (*log).data,
                &raw mut data as *mut ::core::ffi::c_void,
                ::core::mem::size_of::<overwrite_data>() as size_t,
            );
            pop_value(
                (*log).data,
                (*data.buf).data.offset(data.start as isize) as *mut ::core::ffi::c_void,
                data.len as size_t,
            );
            tbuf_drop(data.buf);
        }
        _ => {
            abort();
        }
    };
}
#[no_mangle]
pub unsafe extern "C" fn log_snapshot(mut log: *mut log_t) -> mark_t {
    (*log).snap = (*(*log).data).len as mark_t;
    return (*log).snap;
}
#[no_mangle]
pub unsafe extern "C" fn log_rollback(mut log: *mut log_t, mut mark: mark_t) {
    if mark > (*log).snap {
        abort();
    }
    while (*(*log).data).len > mark as size_t {
        log_pop(log);
    }
    if mark as size_t != (*(*log).data).len {
        fprintf(
            __stderrp,
            b"[fatal] rollback: mark=%d len =%d\n\0" as *const u8 as *const ::core::ffi::c_char,
            mark,
            (*(*log).data).len as ::core::ffi::c_int,
        );
        abort();
    }
    (*log).snap = mark;
}
#[no_mangle]
pub unsafe extern "C" fn state_init(mut st: *mut state_t) {
    memset(
        st as *mut ::core::ffi::c_void,
        0 as ::core::ffi::c_int,
        ::core::mem::size_of::<state_t>() as size_t,
    );
}
unsafe extern "C" fn same_time(mut a: timespec, mut b: timespec) -> bool {
    return a.tv_sec == b.tv_sec && a.tv_nsec == b.tv_nsec;
}
#[no_mangle]
pub unsafe extern "C" fn stat_same(mut st1: *mut stat, mut st2: *mut stat) -> bool {
    return (*st1).st_dev == (*st2).st_dev
        && (*st1).st_ino == (*st2).st_ino
        && (*st1).st_mode as ::core::ffi::c_int == (*st2).st_mode as ::core::ffi::c_int
        && (*st1).st_nlink as ::core::ffi::c_int == (*st2).st_nlink as ::core::ffi::c_int
        && (*st1).st_uid == (*st2).st_uid
        && (*st1).st_gid == (*st2).st_gid
        && (*st1).st_rdev == (*st2).st_rdev
        && (*st1).st_size == (*st2).st_size
        && (*st1).st_blksize == (*st2).st_blksize
        && (*st1).st_blocks == (*st2).st_blocks
        && same_time((*st1).st_atimespec, (*st2).st_atimespec) as ::core::ffi::c_int != 0
        && same_time((*st1).st_mtimespec, (*st2).st_mtimespec) as ::core::ffi::c_int != 0
        && same_time((*st1).st_ctimespec, (*st2).st_ctimespec) as ::core::ffi::c_int != 0;
}
