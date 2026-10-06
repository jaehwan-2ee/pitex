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
// Translated from driver/fs.c with C2Rust 0.22.1.
extern "C" {
    fn calloc(__nmemb: size_t, __size: size_t) -> *mut ::core::ffi::c_void;
    fn free(__ptr: *mut ::core::ffi::c_void);
    fn abort() -> !;
    fn strcmp(
        __s1: *const ::core::ffi::c_char,
        __s2: *const ::core::ffi::c_char,
    ) -> ::core::ffi::c_int;
    fn strdup(__s: *const ::core::ffi::c_char) -> *mut ::core::ffi::c_char;
    fn tbuf_drop(b: *mut tbuf);
}
pub type size_t = usize;
pub type __dev_t = ::core::ffi::c_ulong;
pub type __uid_t = ::core::ffi::c_uint;
pub type __gid_t = ::core::ffi::c_uint;
pub type __ino_t = ::core::ffi::c_ulong;
pub type __mode_t = ::core::ffi::c_uint;
pub type __nlink_t = ::core::ffi::c_ulong;
pub type __off_t = ::core::ffi::c_long;
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
pub struct filesystem_s {
    pub count: ::core::ffi::c_int,
    pub cap: ::core::ffi::c_int,
    pub table: *mut tablecell,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct tablecell {
    pub hash: ::core::ffi::c_ulong,
    pub entry: *mut fileentry_t,
}
pub type filesystem_t = filesystem_s;
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<
    ::core::ffi::c_void,
>();
unsafe extern "C" fn sdbm_hash(
    mut str: *const ::core::ffi::c_uchar,
) -> ::core::ffi::c_ulong {
    let mut hash: ::core::ffi::c_ulong = 0 as ::core::ffi::c_ulong;
    let mut c: ::core::ffi::c_int = 0;
    loop {
        let fresh0 = str;
        str = str.offset(1);
        c = *fresh0 as ::core::ffi::c_int;
        if !(c != 0) {
            break;
        }
        hash = (c as ::core::ffi::c_ulong)
            .wrapping_add(hash << 6 as ::core::ffi::c_int)
            .wrapping_add(hash << 16 as ::core::ffi::c_int)
            .wrapping_sub(hash);
    }
    return hash.wrapping_mul(2654435761 as ::core::ffi::c_ulong);
}
unsafe extern "C" fn xcalloc(
    mut n: size_t,
    mut size: size_t,
) -> *mut ::core::ffi::c_void {
    let mut p: *mut ::core::ffi::c_void = calloc(n, size);
    if p.is_null() {
        abort();
    }
    return p;
}
unsafe extern "C" fn normalize_path(
    mut path: *const ::core::ffi::c_char,
) -> *const ::core::ffi::c_char {
    if *path.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int == '.' as i32
        && *path.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
            == '/' as i32
    {
        path = path.offset(2 as ::core::ffi::c_int as isize);
        while *path as ::core::ffi::c_int == '/' as i32 {
            path = path.offset(1 as ::core::ffi::c_int as isize);
        }
    }
    return path;
}
#[no_mangle]
pub unsafe extern "C" fn filesystem_new() -> *mut filesystem_t {
    let mut fs: *mut filesystem_t = xcalloc(
        1 as size_t,
        ::core::mem::size_of::<filesystem_t>() as size_t,
    ) as *mut filesystem_t;
    (*fs).cap = 64 as ::core::ffi::c_int;
    (*fs).table = xcalloc(64 as size_t, ::core::mem::size_of::<tablecell>() as size_t)
        as *mut tablecell;
    return fs;
}
#[no_mangle]
pub unsafe extern "C" fn filesystem_free(mut fs: *mut filesystem_t) {
    let mut cap: ::core::ffi::c_int = (*fs).cap;
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < cap {
        let mut e: *mut fileentry_t = (*(*fs).table.offset(i as isize)).entry;
        if !e.is_null() {
            tbuf_drop((*e).fs_data);
            tbuf_drop((*e).edit_data);
            tbuf_drop((*e).saved.data);
            free((*e).path as *mut ::core::ffi::c_void);
            free((*(*fs).table.offset(i as isize)).entry as *mut ::core::ffi::c_void);
        }
        i += 1;
    }
    free((*fs).table as *mut ::core::ffi::c_void);
    free(fs as *mut ::core::ffi::c_void);
}
unsafe extern "C" fn table_get(
    mut cap: ::core::ffi::c_int,
    mut table: *mut tablecell,
    mut path: *const ::core::ffi::c_char,
) -> *mut tablecell {
    let mut mask: ::core::ffi::c_ulong = (cap - 1 as ::core::ffi::c_int)
        as ::core::ffi::c_ulong;
    let mut hash: ::core::ffi::c_ulong = sdbm_hash(path as *const ::core::ffi::c_uchar);
    let mut index: ::core::ffi::c_int = (hash & mask) as ::core::ffi::c_int;
    while !(*table.offset(index as isize)).entry.is_null() {
        if (*table.offset(index as isize)).hash == hash
            && strcmp((*(*table.offset(index as isize)).entry).path, path)
                == 0 as ::core::ffi::c_int
        {
            break;
        }
        index = ((index + 1 as ::core::ffi::c_int) as ::core::ffi::c_ulong & mask)
            as ::core::ffi::c_int;
    }
    (*table.offset(index as isize)).hash = hash;
    return table.offset(index as isize) as *mut tablecell;
}
unsafe extern "C" fn table_resize(
    mut oldcap: ::core::ffi::c_int,
    mut oldtab: *mut tablecell,
    mut newcap: ::core::ffi::c_int,
) -> *mut tablecell {
    let mut newtab: *mut tablecell = xcalloc(
        newcap as size_t,
        ::core::mem::size_of::<tablecell>() as size_t,
    ) as *mut tablecell;
    let mut mask: ::core::ffi::c_int = newcap - 1 as ::core::ffi::c_int;
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < oldcap {
        if !(*oldtab.offset(i as isize)).entry.is_null() {
            let mut cell: tablecell = *oldtab.offset(i as isize);
            let mut index: ::core::ffi::c_int = (cell.hash
                & mask as ::core::ffi::c_ulong) as ::core::ffi::c_int;
            while !(*newtab.offset(index as isize)).entry.is_null() {
                if (cell.hash & mask as ::core::ffi::c_ulong)
                    < (*newtab.offset(index as isize)).hash
                        & mask as ::core::ffi::c_ulong
                {
                    let mut tmp: tablecell = *newtab.offset(index as isize);
                    *newtab.offset(index as isize) = cell;
                    cell = tmp;
                }
                index = index + 1 as ::core::ffi::c_int & mask;
            }
            *newtab.offset(index as isize) = cell;
        }
        i += 1;
    }
    return newtab;
}
#[no_mangle]
pub unsafe extern "C" fn filesystem_lookup(
    mut fs: *mut filesystem_t,
    mut path: *const ::core::ffi::c_char,
) -> *mut fileentry_t {
    return (*table_get((*fs).cap, (*fs).table, normalize_path(path))).entry;
}
#[no_mangle]
pub unsafe extern "C" fn filesystem_lookup_or_create(
    mut fs: *mut filesystem_t,
    mut path: *const ::core::ffi::c_char,
) -> *mut fileentry_t {
    path = normalize_path(path);
    let mut cell: *mut tablecell = table_get((*fs).cap, (*fs).table, path);
    let mut entry: *mut fileentry_t = (*cell).entry;
    if !entry.is_null() {
        return entry;
    }
    entry = xcalloc(1 as size_t, ::core::mem::size_of::<fileentry_t>() as size_t)
        as *mut fileentry_t;
    (*entry).path = strdup(path);
    if (*entry).path.is_null() {
        abort();
    }
    (*entry).saved.level = FILE_NONE;
    (*entry).seen = -(1 as ::core::ffi::c_int);
    (*entry).pic_cache.type_0 = -(1 as ::core::ffi::c_int);
    (*entry).fs_stat.st_ino = 0 as __ino_t;
    (*entry).debug_rollback_invalidation = -(1 as ::core::ffi::c_int);
    (*cell).entry = entry;
    (*fs).count += 1 as ::core::ffi::c_int;
    if (*fs).count * 4 as ::core::ffi::c_int >= (*fs).cap * 3 as ::core::ffi::c_int {
        let mut newcap: ::core::ffi::c_int = (*fs).cap * 2 as ::core::ffi::c_int;
        let mut newtab: *mut tablecell = table_resize((*fs).cap, (*fs).table, newcap);
        free((*fs).table as *mut ::core::ffi::c_void);
        (*fs).cap = newcap;
        (*fs).table = newtab;
    }
    return entry;
}
#[no_mangle]
pub unsafe extern "C" fn filesystem_scan(
    mut fs: *mut filesystem_t,
    mut index: *mut ::core::ffi::c_int,
) -> *mut fileentry_t {
    while *index < (*fs).cap {
        let mut i: ::core::ffi::c_int = *index;
        *index += 1 as ::core::ffi::c_int;
        if !(*(*fs).table.offset(i as isize)).entry.is_null() {
            return (*(*fs).table.offset(i as isize)).entry;
        }
    }
    return ::core::ptr::null_mut::<fileentry_t>();
}
