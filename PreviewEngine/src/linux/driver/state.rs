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
    pub type _IO_wide_data;
    pub type _IO_codecvt;
    pub type _IO_marker;
    fn calloc(__nmemb: size_t, __size: size_t) -> *mut ::core::ffi::c_void;
    fn free(__ptr: *mut ::core::ffi::c_void);
    fn abort() -> !;
    static mut stderr: *mut FILE;
    fn fprintf(
        __stream: *mut FILE,
        __format: *const ::core::ffi::c_char,
        ...
    ) -> ::core::ffi::c_int;
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
    fn tbuf_new(cap: size_t) -> *mut tbuf;
    fn tbuf_keep(b: *mut tbuf) -> *mut tbuf;
    fn tbuf_drop(b: *mut tbuf);
    fn tbuf_append(b: *mut tbuf, data: *const ::core::ffi::c_void, len: size_t);
}
pub type size_t = usize;
pub type __uint8_t = u8;
pub type __dev_t = ::core::ffi::c_ulong;
pub type __uid_t = ::core::ffi::c_uint;
pub type __gid_t = ::core::ffi::c_uint;
pub type __ino_t = ::core::ffi::c_ulong;
pub type __mode_t = ::core::ffi::c_uint;
pub type __nlink_t = ::core::ffi::c_ulong;
pub type __off_t = ::core::ffi::c_long;
pub type __off64_t = ::core::ffi::c_long;
pub type __time_t = ::core::ffi::c_long;
pub type __blksize_t = ::core::ffi::c_long;
pub type __blkcnt_t = ::core::ffi::c_long;
pub type __syscall_slong_t = ::core::ffi::c_long;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct timespec {
    pub tv_sec: __time_t,
    pub tv_nsec: __syscall_slong_t,
}
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
pub type uint8_t = __uint8_t;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct stat {
    pub st_dev: __dev_t,
    pub st_ino: __ino_t,
    pub st_nlink: __nlink_t,
    pub st_mode: __mode_t,
    pub st_uid: __uid_t,
    pub st_gid: __gid_t,
    pub __pad0: ::core::ffi::c_int,
    pub st_rdev: __dev_t,
    pub st_size: __off_t,
    pub st_blksize: __blksize_t,
    pub st_blocks: __blkcnt_t,
    pub st_atim: timespec,
    pub st_mtim: timespec,
    pub st_ctim: timespec,
    pub __glibc_reserved: [__syscall_slong_t; 3],
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
    pub stdout: filecell_t,
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
    let mut log: *mut log_t = calloc(
        1 as size_t,
        ::core::mem::size_of::<log_t>() as size_t,
    ) as *mut log_t;
    if log.is_null() {
        abort();
    }
    (*log).data = tbuf_new(512 as size_t);
    (*log).snap = 1 as ::core::ffi::c_int as mark_t;
    let mut zero: uint8_t = 0 as uint8_t;
    tbuf_append((*log).data, &raw mut zero as *const ::core::ffi::c_void, 1 as size_t);
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
pub unsafe extern "C" fn log_fileentry(
    mut log: *mut log_t,
    mut entry: *mut fileentry_t,
) {
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
            stderr,
            b"[fatal] rollback: mark=%d len =%d\n\0" as *const u8
                as *const ::core::ffi::c_char,
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
    return (*st1).st_dev == (*st2).st_dev && (*st1).st_ino == (*st2).st_ino
        && (*st1).st_mode == (*st2).st_mode && (*st1).st_nlink == (*st2).st_nlink
        && (*st1).st_uid == (*st2).st_uid && (*st1).st_gid == (*st2).st_gid
        && (*st1).st_rdev == (*st2).st_rdev && (*st1).st_size == (*st2).st_size
        && (*st1).st_blksize == (*st2).st_blksize && (*st1).st_blocks == (*st2).st_blocks
        && same_time((*st1).st_atim, (*st2).st_atim) as ::core::ffi::c_int != 0
        && same_time((*st1).st_mtim, (*st2).st_mtim) as ::core::ffi::c_int != 0
        && same_time((*st1).st_ctim, (*st2).st_ctim) as ::core::ffi::c_int != 0;
}
