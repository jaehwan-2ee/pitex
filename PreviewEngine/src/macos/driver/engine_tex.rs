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
// Translated from driver/engine_tex.c with C2Rust 0.22.1.
extern "C" {
    pub type __sFILEX;
    pub type channel_s;
    pub type filesystem_s;
    pub type log_s;
    pub type xdv_index;
    fn waitpid(_: pid_t, _: *mut ::core::ffi::c_int, _: ::core::ffi::c_int) -> pid_t;
    fn calloc(__count: size_t, __size: size_t) -> *mut ::core::ffi::c_void;
    fn free(_: *mut ::core::ffi::c_void);
    fn abort() -> !;
    fn setenv(
        __name: *const ::core::ffi::c_char,
        __value: *const ::core::ffi::c_char,
        __overwrite: ::core::ffi::c_int,
    ) -> ::core::ffi::c_int;
    fn _exit(_: ::core::ffi::c_int) -> !;
    fn close(_: ::core::ffi::c_int) -> ::core::ffi::c_int;
    fn dup2(_: ::core::ffi::c_int, _: ::core::ffi::c_int) -> ::core::ffi::c_int;
    fn execv(
        __path: *const ::core::ffi::c_char,
        __argv: *const *mut ::core::ffi::c_char,
    ) -> ::core::ffi::c_int;
    fn fork() -> pid_t;
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
    fn memmove(
        __dst: *mut ::core::ffi::c_void,
        __src: *const ::core::ffi::c_void,
        __len: size_t,
    ) -> *mut ::core::ffi::c_void;
    fn memset(
        __b: *mut ::core::ffi::c_void,
        __c: ::core::ffi::c_int,
        __len: size_t,
    ) -> *mut ::core::ffi::c_void;
    fn strcmp(
        __s1: *const ::core::ffi::c_char,
        __s2: *const ::core::ffi::c_char,
    ) -> ::core::ffi::c_int;
    fn strlen(__s: *const ::core::ffi::c_char) -> size_t;
    fn strrchr(
        __s: *const ::core::ffi::c_char,
        __c: ::core::ffi::c_int,
    ) -> *mut ::core::ffi::c_char;
    fn strdup(__s1: *const ::core::ffi::c_char) -> *mut ::core::ffi::c_char;
    fn fcntl(_: ::core::ffi::c_int, _: ::core::ffi::c_int, ...) -> ::core::ffi::c_int;
    fn __error() -> *mut ::core::ffi::c_int;
    fn clock_gettime(__clock_id: clockid_t, __tp: *mut timespec) -> ::core::ffi::c_int;
    fn stat(_: *const ::core::ffi::c_char, _: *mut stat) -> ::core::ffi::c_int;
    fn socketpair(
        _: ::core::ffi::c_int,
        _: ::core::ffi::c_int,
        _: ::core::ffi::c_int,
        _: *mut ::core::ffi::c_int,
    ) -> ::core::ffi::c_int;
    fn kill(_: pid_t, _: ::core::ffi::c_int) -> ::core::ffi::c_int;
    static mut __stderrp: *mut FILE;
    fn fprintf(_: *mut FILE, _: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
    fn perror(_: *const ::core::ffi::c_char);
    fn snprintf(
        __str: *mut ::core::ffi::c_char,
        __size: size_t,
        __format: *const ::core::ffi::c_char,
        ...
    ) -> ::core::ffi::c_int;
    fn print_backtrace();
    fn channel_new() -> *mut channel_t;
    fn channel_free(c: *mut channel_t);
    fn channel_handshake(c: *mut channel_t, fd: ::core::ffi::c_int) -> bool;
    fn channel_has_pending_query(
        t: *mut channel_t,
        fd: ::core::ffi::c_int,
        timeout: ::core::ffi::c_int,
    ) -> bool;
    fn channel_peek_query(t: *mut channel_t, fd: ::core::ffi::c_int) -> query;
    fn channel_read_query(t: *mut channel_t, fd: ::core::ffi::c_int, r: *mut query_t) -> bool;
    fn channel_write_ask(t: *mut channel_t, fd: ::core::ffi::c_int, a: *mut ask_t);
    fn channel_write_answer(t: *mut channel_t, fd: ::core::ffi::c_int, a: *mut answer_t);
    fn channel_get_buffer(t: *mut channel_t, n: size_t) -> *mut ::core::ffi::c_void;
    fn channel_flush(t: *mut channel_t, fd: ::core::ffi::c_int);
    fn channel_reset(t: *mut channel_t);
    fn tbuf_new(cap: size_t) -> *mut tbuf;
    fn tbuf_from_copy(data: *const ::core::ffi::c_void, len: size_t) -> *mut tbuf;
    fn tbuf_read_file(path: *const ::core::ffi::c_char) -> *mut tbuf;
    fn tbuf_drop(b: *mut tbuf);
    fn tbuf_append(b: *mut tbuf, data: *const ::core::ffi::c_void, len: size_t);
    fn state_init(st: *mut state_t);
    fn filesystem_new() -> *mut filesystem_t;
    fn filesystem_free(fs: *mut filesystem_t);
    fn filesystem_lookup_or_create(
        fs: *mut filesystem_t,
        path: *const ::core::ffi::c_char,
    ) -> *mut fileentry_t;
    fn filesystem_lookup(
        fs: *mut filesystem_t,
        path: *const ::core::ffi::c_char,
    ) -> *mut fileentry_t;
    fn filesystem_scan(fs: *mut filesystem_t, index: *mut ::core::ffi::c_int) -> *mut fileentry_t;
    fn log_new() -> *mut log_t;
    fn log_free(log: *mut log_t);
    fn log_snapshot(log: *mut log_t) -> mark_t;
    fn log_rollback(log: *mut log_t, snapshot: mark_t);
    fn log_fileentry(log: *mut log_t, entry: *mut fileentry_t);
    fn log_filecell(log: *mut log_t, cell: *mut filecell_t);
    fn stat_same(st1: *mut stat, st2: *mut stat) -> bool;
    fn xdv_index_new() -> *mut xdv_index;
    fn xdv_index_free(x: *mut xdv_index);
    fn xdv_index_reset(x: *mut xdv_index);
    fn xdv_index_update(x: *mut xdv_index, data: *const ::core::ffi::c_uchar, len: size_t);
    fn xdv_index_output_started(x: *const xdv_index) -> bool;
}
pub type __uint16_t = u16;
pub type __int32_t = i32;
pub type __uint32_t = u32;
pub type __int64_t = i64;
pub type __uint64_t = u64;
pub type __darwin_size_t = usize;
pub type __darwin_ssize_t = isize;
pub type __darwin_time_t = ::core::ffi::c_long;
pub type __darwin_blkcnt_t = __int64_t;
pub type __darwin_blksize_t = __int32_t;
pub type __darwin_dev_t = __int32_t;
pub type __darwin_gid_t = __uint32_t;
pub type __darwin_ino64_t = __uint64_t;
pub type __darwin_mode_t = __uint16_t;
pub type __darwin_off_t = __int64_t;
pub type __darwin_pid_t = __int32_t;
pub type __darwin_uid_t = __uint32_t;
pub type pid_t = __darwin_pid_t;
pub type size_t = __darwin_size_t;
pub type uid_t = __darwin_uid_t;
pub type uint32_t = u32;
pub type dev_t = __darwin_dev_t;
pub type mode_t = __darwin_mode_t;
pub type ssize_t = __darwin_ssize_t;
pub type gid_t = __darwin_gid_t;
pub type off_t = __darwin_off_t;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct timespec {
    pub tv_sec: __darwin_time_t,
    pub tv_nsec: ::core::ffi::c_long,
}
pub type clockid_t = ::core::ffi::c_uint;
pub const _CLOCK_THREAD_CPUTIME_ID: clockid_t = 16;
pub const _CLOCK_PROCESS_CPUTIME_ID: clockid_t = 12;
pub const _CLOCK_UPTIME_RAW_APPROX: clockid_t = 9;
pub const _CLOCK_UPTIME_RAW: clockid_t = 8;
pub const _CLOCK_MONOTONIC_RAW_APPROX: clockid_t = 5;
pub const _CLOCK_MONOTONIC_RAW: clockid_t = 4;
pub const _CLOCK_MONOTONIC: clockid_t = 6;
pub const _CLOCK_REALTIME: clockid_t = 0;
pub type blkcnt_t = __darwin_blkcnt_t;
pub type blksize_t = __darwin_blksize_t;
pub type nlink_t = __uint16_t;
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
pub type channel_t = channel_s;
pub type file_id = ::core::ffi::c_int;
pub type query = ::core::ffi::c_uint;
pub const Q_FNTB: query = 1112821318;
pub const Q_CHLD: query = 1145849923;
pub const Q_SPIC: query = 1128878163;
pub const Q_GPIC: query = 1128878151;
pub const Q_SEEN: query = 1313162579;
pub const Q_MTIM: query = 1296651341;
pub const Q_SIZE: query = 1163544915;
pub const Q_CLOS: query = 1397705795;
pub const Q_APND: query = 1145983041;
pub const Q_READ: query = 1145128274;
pub const Q_OPWR: query = 1381453903;
pub const Q_OPRD: query = 1146245199;
pub type txp_file_kind = ::core::ffi::c_uint;
pub const TXP_KIND_OTHER: txp_file_kind = 1380471887;
pub const TXP_KIND_VF: txp_file_kind = 18006;
pub const TXP_KIND_TYPE1: txp_file_kind = 827349332;
pub const TXP_KIND_TRUE_TYPE: txp_file_kind = 4609108;
pub const TXP_KIND_TFM: txp_file_kind = 5064276;
pub const TXP_KIND_TEX_PS_HEADER: txp_file_kind = 1213419604;
pub const TXP_KIND_TEX: txp_file_kind = 5784916;
pub const TXP_KIND_PRIMARY: txp_file_kind = 1296650832;
pub const TXP_KIND_SFD: txp_file_kind = 4474451;
pub const TXP_KIND_PROGRAM_DATA: txp_file_kind = 1413563472;
pub const TXP_KIND_PK: txp_file_kind = 19280;
pub const TXP_KIND_PICT: txp_file_kind = 1413695824;
pub const TXP_KIND_OVF: txp_file_kind = 4609615;
pub const TXP_KIND_OPEN_TYPE: txp_file_kind = 4609103;
pub const TXP_KIND_OFM: txp_file_kind = 5064271;
pub const TXP_KIND_MISC_FONTS: txp_file_kind = 1414415949;
pub const TXP_KIND_FONT_MAP: txp_file_kind = 1346456902;
pub const TXP_KIND_FORMAT: txp_file_kind = 1414353478;
pub const TXP_KIND_ENC: txp_file_kind = 4410949;
pub const TXP_KIND_CNF: txp_file_kind = 4607555;
pub const TXP_KIND_CMAP: txp_file_kind = 1346456899;
pub const TXP_KIND_BST: txp_file_kind = 5526338;
pub const TXP_KIND_BIB: txp_file_kind = 4344130;
pub const TXP_KIND_AFM: txp_file_kind = 5064257;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct pic_cache {
    pub type_0: ::core::ffi::c_int,
    pub page: ::core::ffi::c_int,
    pub bounds: [::core::ffi::c_float; 4],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct query_t {
    pub time: ::core::ffi::c_int,
    pub tag: query,
    pub c2rust_unnamed: C2RustUnnamed,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union C2RustUnnamed {
    pub open: C2RustUnnamed_9,
    pub read: C2RustUnnamed_8,
    pub apnd: C2RustUnnamed_7,
    pub clos: C2RustUnnamed_6,
    pub size: C2RustUnnamed_5,
    pub mtim: C2RustUnnamed_4,
    pub seen: C2RustUnnamed_3,
    pub chld: C2RustUnnamed_2,
    pub gpic: C2RustUnnamed_1,
    pub spic: C2RustUnnamed_0,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_0 {
    pub path: *mut ::core::ffi::c_char,
    pub cache: pic_cache,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_1 {
    pub path: *mut ::core::ffi::c_char,
    pub type_0: ::core::ffi::c_int,
    pub page: ::core::ffi::c_int,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_2 {
    pub fd: ::core::ffi::c_int,
    pub pid: ::core::ffi::c_int,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_3 {
    pub fid: file_id,
    pub pos: ::core::ffi::c_int,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_4 {
    pub fid: file_id,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_5 {
    pub fid: file_id,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_6 {
    pub fid: file_id,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_7 {
    pub fid: file_id,
    pub size: ::core::ffi::c_int,
    pub buf: *mut ::core::ffi::c_char,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_8 {
    pub fid: file_id,
    pub pos: ::core::ffi::c_int,
    pub size: ::core::ffi::c_int,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_9 {
    pub fid: file_id,
    pub path: *mut ::core::ffi::c_char,
    pub kind: txp_file_kind,
}
pub type answer = ::core::ffi::c_uint;
pub const A_GPIC: answer = 1128878151;
pub const A_OPEN: answer = 1313165391;
pub const A_FORK: answer = 1263685446;
pub const A_READ: answer = 1145128274;
pub const A_MTIM: answer = 1296651341;
pub const A_SIZE: answer = 1163544915;
pub const A_PASS: answer = 1397965136;
pub const A_DONE: answer = 1162760004;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct answer_t {
    pub tag: answer,
    pub c2rust_unnamed: C2RustUnnamed_10,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union C2RustUnnamed_10 {
    pub size: C2RustUnnamed_15,
    pub mtim: C2RustUnnamed_14,
    pub read: C2RustUnnamed_13,
    pub open: C2RustUnnamed_12,
    pub gpic: C2RustUnnamed_11,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_11 {
    pub bounds: [::core::ffi::c_float; 4],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_12 {
    pub path_len: ::core::ffi::c_int,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_13 {
    pub size: ::core::ffi::c_int,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_14 {
    pub mtime: uint32_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_15 {
    pub size: uint32_t,
}
pub type ask = ::core::ffi::c_uint;
pub const C_FLSH: ask = 1213418566;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ask_t {
    pub tag: ask,
    pub c2rust_unnamed: C2RustUnnamed_16,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union C2RustUnnamed_16 {
    pub term: C2RustUnnamed_19,
    pub fenc: C2RustUnnamed_18,
    pub flsh: C2RustUnnamed_17,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_17 {
    pub fid: ::core::ffi::c_int,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_18 {
    pub fid: ::core::ffi::c_int,
    pub pos: ::core::ffi::c_int,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_19 {
    pub pid: ::core::ffi::c_int,
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
    pub saved: C2RustUnnamed_20,
    pub seen: ::core::ffi::c_int,
    pub debug_rollback_invalidation: ::core::ffi::c_int,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_20 {
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
pub type filesystem_t = filesystem_s;
pub type log_t = log_s;
pub type txp_engine_status = ::core::ffi::c_uint;
pub const DOC_TERMINATED: txp_engine_status = 1;
pub const DOC_RUNNING: txp_engine_status = 0;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct tex_engine {
    pub name: *mut ::core::ffi::c_char,
    pub engine_path: *mut ::core::ffi::c_char,
    pub inclusion_path: *mut ::core::ffi::c_char,
    pub fs: *mut filesystem_t,
    pub st: state_t,
    pub log: *mut log_t,
    pub c: *mut channel_t,
    pub processes: [process_t; 32],
    pub process_count: ::core::ffi::c_int,
    pub trace: *mut trace_entry_t,
    pub trace_cap: ::core::ffi::c_int,
    pub fences: [fence_t; 16],
    pub fence_pos: ::core::ffi::c_int,
    pub restart: mark_t,
    pub dvi: *mut xdv_index,
    pub log_entry: *mut fileentry_t,
    pub rollback: C2RustUnnamed_21,
    pub aux_dirty: bool,
    pub finishing: bool,
    pub root_pid: ::core::ffi::c_int,
    pub last_status: ::core::ffi::c_int,
    pub last_query: timespec,
    pub snapshot_barrier: ::core::ffi::c_int,
    pub barrier_pending: bool,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_21 {
    pub trace_len: ::core::ffi::c_int,
    pub offset: ::core::ffi::c_int,
    pub flush: ::core::ffi::c_int,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct fence_t {
    pub entry: *mut fileentry_t,
    pub position: ::core::ffi::c_int,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct trace_entry_t {
    pub entry: *mut fileentry_t,
    pub seen: ::core::ffi::c_int,
    pub time: ::core::ffi::c_int,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct process_t {
    pub pid: ::core::ffi::c_int,
    pub fd: ::core::ffi::c_int,
    pub trace_len: ::core::ffi::c_int,
    pub snap: mark_t,
    pub forked: bool,
}
pub const MAX_PROCESS: C2RustUnnamed_22 = 32;
pub type C2RustUnnamed_22 = ::core::ffi::c_uint;
pub const __DARWIN_NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
pub const SIGTERM: ::core::ffi::c_int = 15 as ::core::ffi::c_int;
pub const WNOHANG: ::core::ffi::c_int = 0x1 as ::core::ffi::c_int;
pub const NULL: *mut ::core::ffi::c_void = __DARWIN_NULL;
pub const INT_MAX: ::core::ffi::c_int = 2147483647 as ::core::ffi::c_int;
pub const STDOUT_FILENO: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const STDERR_FILENO: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const F_SETFD: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const FD_CLOEXEC: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const S_IFMT: ::core::ffi::c_int = 0o170000 as ::core::ffi::c_int;
pub const S_IFREG: ::core::ffi::c_int = 0o100000 as ::core::ffi::c_int;
pub const EINTR: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
pub const SOCK_STREAM: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const AF_UNIX: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const AF_LOCAL: ::core::ffi::c_int = AF_UNIX;
pub const PF_LOCAL: ::core::ffi::c_int = AF_LOCAL;
pub const PF_UNIX: ::core::ffi::c_int = PF_LOCAL;
pub const true_0: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const false_0: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const MAX_FILES: ::core::ffi::c_int = 1024 as ::core::ffi::c_int;
unsafe extern "C" fn maxi(
    mut a: ::core::ffi::c_int,
    mut b: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    return if a > b { a } else { b };
}
unsafe extern "C" fn touch_query_clock(mut self_0: *mut tex_engine) {
    clock_gettime(_CLOCK_MONOTONIC, &raw mut (*self_0).last_query);
}
unsafe extern "C" fn get_process(mut t: *mut tex_engine) -> *mut process_t {
    if (*t).process_count == 0 as ::core::ffi::c_int {
        fprintf(
            __stderrp,
            b"Aborting from driver/engine_tex.c:133\n\0" as *const u8 as *const ::core::ffi::c_char,
        );
        print_backtrace();
        abort();
    }
    return (&raw mut (*t).processes as *mut process_t)
        .offset(((*t).process_count - 1 as ::core::ffi::c_int) as isize)
        as *mut process_t;
}
unsafe extern "C" fn last_index(
    mut path: *mut ::core::ffi::c_char,
    mut needle: ::core::ffi::c_char,
) -> *mut ::core::ffi::c_char {
    let mut result: *mut ::core::ffi::c_char = path;
    while *path != 0 {
        if *path as ::core::ffi::c_int == needle as ::core::ffi::c_int {
            result = path.offset(1 as ::core::ffi::c_int as isize);
        }
        path = path.offset(1 as ::core::ffi::c_int as isize);
    }
    return result;
}
unsafe extern "C" fn exec_xelatex_generic(
    mut args: *mut *mut ::core::ffi::c_char,
    mut fd: *mut ::core::ffi::c_int,
) -> pid_t {
    let mut sockets: [::core::ffi::c_int; 2] = [0; 2];
    if socketpair(
        PF_UNIX,
        SOCK_STREAM,
        0 as ::core::ffi::c_int,
        &raw mut sockets as *mut ::core::ffi::c_int,
    ) != 0 as ::core::ffi::c_int
    {
        perror(b"exec_xelatex socketpair\0" as *const u8 as *const ::core::ffi::c_char);
        fprintf(
            __stderrp,
            b"Aborting from driver/engine_tex.c:161\n\0" as *const u8 as *const ::core::ffi::c_char,
        );
        print_backtrace();
        abort();
    }
    let mut buf: [::core::ffi::c_char; 30] = [0; 30];
    snprintf(
        &raw mut buf as *mut ::core::ffi::c_char,
        30 as size_t,
        b"%d\0" as *const u8 as *const ::core::ffi::c_char,
        sockets[1 as ::core::ffi::c_int as usize],
    );
    setenv(
        b"TEXPRESSO_FD\0" as *const u8 as *const ::core::ffi::c_char,
        &raw mut buf as *mut ::core::ffi::c_char,
        1 as ::core::ffi::c_int,
    );
    static mut env_init: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    if env_init == 0 {
        env_init = 1 as ::core::ffi::c_int;
        setenv(
            b"OBJC_DISABLE_INITIALIZE_FORK_SAFETY\0" as *const u8 as *const ::core::ffi::c_char,
            b"YES\0" as *const u8 as *const ::core::ffi::c_char,
            1 as ::core::ffi::c_int,
        );
    }
    let mut pid: pid_t = fork();
    if pid == -(1 as pid_t) {
        perror(b"exec_xelatex fork\0" as *const u8 as *const ::core::ffi::c_char);
        fprintf(
            __stderrp,
            b"Aborting from driver/engine_tex.c:182\n\0" as *const u8 as *const ::core::ffi::c_char,
        );
        print_backtrace();
        abort();
    }
    if pid == 0 as pid_t {
        close(sockets[0 as ::core::ffi::c_int as usize]);
        dup2(STDERR_FILENO, STDOUT_FILENO);
        execv(*args.offset(0 as ::core::ffi::c_int as isize), args);
        _exit(2 as ::core::ffi::c_int);
    }
    if close(sockets[1 as ::core::ffi::c_int as usize]) != 0 as ::core::ffi::c_int {
        fprintf(
            __stderrp,
            b"Aborting from driver/engine_tex.c:201\n\0" as *const u8 as *const ::core::ffi::c_char,
        );
        print_backtrace();
        abort();
    }
    fcntl(
        sockets[0 as ::core::ffi::c_int as usize],
        F_SETFD,
        FD_CLOEXEC,
    );
    *fd = sockets[0 as ::core::ffi::c_int as usize];
    return pid;
}
unsafe extern "C" fn exec_xelatex(
    mut engine_path: *mut ::core::ffi::c_char,
    mut filename: *const ::core::ffi::c_char,
    mut fd: *mut ::core::ffi::c_int,
) -> pid_t {
    let mut args: [*mut ::core::ffi::c_char; 4] = [
        engine_path,
        b"-texpresso\0" as *const u8 as *const ::core::ffi::c_char as *mut ::core::ffi::c_char,
        filename as *mut ::core::ffi::c_char,
        ::core::ptr::null_mut::<::core::ffi::c_char>(),
    ];
    let mut pid: pid_t = exec_xelatex_generic(&raw mut args as *mut *mut ::core::ffi::c_char, fd);
    fprintf(
        __stderrp,
        b"[process] launched pid %d (using %s)\n\0" as *const u8 as *const ::core::ffi::c_char,
        pid,
        engine_path,
    );
    return pid;
}
unsafe extern "C" fn reap_root(mut self_0: *mut tex_engine, mut wait: bool) {
    if (*self_0).root_pid <= 0 as ::core::ffi::c_int {
        return;
    }
    let mut status: ::core::ffi::c_int = 0;
    let mut r: pid_t = 0;
    loop {
        r = waitpid(
            (*self_0).root_pid as pid_t,
            &raw mut status,
            if wait as ::core::ffi::c_int != 0 {
                0 as ::core::ffi::c_int
            } else {
                WNOHANG
            },
        );
        if !(r == -(1 as pid_t) && *__error() == EINTR) {
            break;
        }
    }
    if r == (*self_0).root_pid as pid_t {
        (*self_0).last_status = status;
        (*self_0).root_pid = 0 as ::core::ffi::c_int;
    }
}
unsafe extern "C" fn prepare_process(mut self_0: *mut tex_engine) {
    if (*self_0).process_count == 0 as ::core::ffi::c_int {
        reap_root(self_0, false_0 != 0);
        log_rollback((*self_0).log, (*self_0).restart);
        (*self_0).process_count = 1 as ::core::ffi::c_int;
        let mut p: *mut process_t = get_process(self_0);
        (*p).pid = exec_xelatex((*self_0).engine_path, (*self_0).name, &raw mut (*p).fd)
            as ::core::ffi::c_int;
        (*p).trace_len = 0 as ::core::ffi::c_int;
        (*p).forked = false_0 != 0;
        (*self_0).root_pid = (*p).pid;
        touch_query_clock(self_0);
        if !channel_handshake((*self_0).c, (*p).fd) {
            fprintf(
                __stderrp,
                b"Aborting from driver/engine_tex.c:251\n\0" as *const u8
                    as *const ::core::ffi::c_char,
            );
            print_backtrace();
            abort();
        }
    }
}
unsafe extern "C" fn close_process(mut p: *mut process_t) {
    if (*p).fd != -(1 as ::core::ffi::c_int) {
        kill((*p).pid as pid_t, SIGTERM);
        close((*p).fd);
        (*p).fd = -(1 as ::core::ffi::c_int);
    }
}
unsafe extern "C" fn pop_process(mut self_0: *mut tex_engine) {
    let mut p: *mut process_t = get_process(self_0);
    close_process(p);
    channel_reset((*self_0).c);
    (*self_0).process_count -= 1 as ::core::ffi::c_int;
    let mut mark: mark_t = if (*self_0).process_count > 0 as ::core::ffi::c_int {
        (*get_process(self_0)).snap
    } else {
        (*self_0).restart
    };
    log_rollback((*self_0).log, mark);
    if (*self_0).process_count == 0 as ::core::ffi::c_int {
        reap_root(self_0, true_0 != 0);
    }
}
unsafe extern "C" fn clear_convergence_stash(mut self_0: *mut tex_engine) {
    let mut e: *mut fileentry_t = ::core::ptr::null_mut::<fileentry_t>();
    let mut index: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    loop {
        e = filesystem_scan((*self_0).fs, &raw mut index);
        if e.is_null() {
            break;
        }
        if (*e).edit_data_from_convergence as ::core::ffi::c_int != 0 && !(*e).edit_data.is_null() {
            tbuf_drop((*e).edit_data);
            (*e).edit_data = ::core::ptr::null_mut::<tbuf>();
            (*e).edit_data_from_convergence = false_0 != 0;
        }
    }
    (*self_0).aux_dirty = false_0 != 0;
    (*self_0).finishing = false_0 != 0;
}
unsafe extern "C" fn read_query(
    mut self_0: *mut tex_engine,
    mut t: *mut channel_t,
    mut q: *mut query_t,
) -> bool {
    let mut p: *mut process_t = get_process(self_0);
    let mut result: bool = channel_read_query(t, (*p).fd, q);
    if !result {
        fprintf(
            __stderrp,
            b"[process] terminating process\n\0" as *const u8 as *const ::core::ffi::c_char,
        );
        close_process(p);
    }
    return result;
}
unsafe extern "C" fn decimate_processes(mut self_0: *mut tex_engine) {
    let mut keep: [bool; 32] = [
        0 as ::core::ffi::c_int != 0,
        false,
        false,
        false,
        false,
        false,
        false,
        false,
        false,
        false,
        false,
        false,
        false,
        false,
        false,
        false,
        false,
        false,
        false,
        false,
        false,
        false,
        false,
        false,
        false,
        false,
        false,
        false,
        false,
        false,
        false,
        false,
    ];
    let mut target: ::core::ffi::c_int = 32 as ::core::ffi::c_int;
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < (*self_0).process_count {
        let mut p: *mut process_t =
            (&raw mut (*self_0).processes as *mut process_t).offset(i as isize) as *mut process_t;
        if (*p).trace_len >= target {
            keep[i as usize] = true_0 != 0;
            target *= 2 as ::core::ffi::c_int;
        }
        i += 1;
    }
    target =
        (*self_0).processes[((*self_0).process_count - 1 as ::core::ffi::c_int) as usize].trace_len;
    let mut delta: ::core::ffi::c_int = 32 as ::core::ffi::c_int;
    let mut i_0: ::core::ffi::c_int = (*self_0).process_count - 1 as ::core::ffi::c_int;
    while i_0 >= 0 as ::core::ffi::c_int {
        let mut p_0: *mut process_t =
            (&raw mut (*self_0).processes as *mut process_t).offset(i_0 as isize) as *mut process_t;
        if (*p_0).trace_len <= target {
            keep[i_0 as usize] = true_0 != 0;
            delta *= 2 as ::core::ffi::c_int;
            target -= delta;
        } else if keep[i_0 as usize] {
            delta *= 2 as ::core::ffi::c_int;
            target = (*p_0).trace_len - delta;
        }
        i_0 -= 1;
    }
    let mut i_1: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut j: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while j < (*self_0).process_count {
        if !keep[j as usize] {
            close_process(
                (&raw mut (*self_0).processes as *mut process_t).offset(j as isize)
                    as *mut process_t,
            );
        } else {
            if i_1 != j {
                (*self_0).processes[i_1 as usize] = (*self_0).processes[j as usize];
            }
            i_1 += 1;
        }
        j += 1;
    }
    (*self_0).process_count = i_1;
    fprintf(
        __stderrp,
        b"[process] decimated snapshots to %d\n\0" as *const u8 as *const ::core::ffi::c_char,
        (*self_0).process_count,
    );
}
#[no_mangle]
pub unsafe extern "C" fn txp_engine_free(mut self_0: *mut tex_engine) {
    while (*self_0).process_count > 0 as ::core::ffi::c_int {
        pop_process(self_0 as *mut tex_engine);
    }
    reap_root(self_0 as *mut tex_engine, true_0 != 0);
    xdv_index_free((*self_0).dvi);
    filesystem_free((*self_0).fs);
    log_free((*self_0).log);
    channel_free((*self_0).c);
    free((*self_0).trace as *mut ::core::ffi::c_void);
    free((*self_0).name as *mut ::core::ffi::c_void);
    free((*self_0).engine_path as *mut ::core::ffi::c_void);
    free((*self_0).inclusion_path as *mut ::core::ffi::c_void);
    free(self_0 as *mut ::core::ffi::c_void);
}
unsafe extern "C" fn expand_path(
    mut inclusion_path: *mut *const ::core::ffi::c_char,
    mut name: *const ::core::ffi::c_char,
    mut buffer: *mut ::core::ffi::c_char,
) -> *const ::core::ffi::c_char {
    if (*inclusion_path).is_null()
        || *(*inclusion_path).offset(0 as ::core::ffi::c_int as isize) == 0
    {
        return ::core::ptr::null::<::core::ffi::c_char>();
    }
    if *name.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int == '/' as i32 {
        return ::core::ptr::null::<::core::ffi::c_char>();
    }
    if *name.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int == '.' as i32
        && *name.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int == '/' as i32
    {
        name = name.offset(2 as ::core::ffi::c_int as isize);
        while *name as ::core::ffi::c_int == '/' as i32 {
            name = name.offset(1 as ::core::ffi::c_int as isize);
        }
    }
    let mut p: *mut ::core::ffi::c_char = buffer as *mut ::core::ffi::c_char;
    let mut i: *const ::core::ffi::c_char = *inclusion_path;
    while *i != 0 {
        if p >= buffer.offset(1023 as ::core::ffi::c_int as isize) {
            return ::core::ptr::null::<::core::ffi::c_char>();
        }
        *p = *i;
        p = p.offset(1 as ::core::ffi::c_int as isize);
        i = i.offset(1 as ::core::ffi::c_int as isize);
    }
    *inclusion_path = i.offset(1 as ::core::ffi::c_int as isize);
    if p > buffer
        && *p.offset(-(1 as ::core::ffi::c_int) as isize) as ::core::ffi::c_int != '/' as i32
    {
        if p >= buffer.offset(1023 as ::core::ffi::c_int as isize) {
            return ::core::ptr::null::<::core::ffi::c_char>();
        }
        *p.offset(0 as ::core::ffi::c_int as isize) = '/' as i32 as ::core::ffi::c_char;
        p = p.offset(1 as ::core::ffi::c_int as isize);
    }
    while *name != 0 {
        if p >= buffer.offset(1023 as ::core::ffi::c_int as isize) {
            return ::core::ptr::null::<::core::ffi::c_char>();
        }
        *p = *name;
        p = p.offset(1 as ::core::ffi::c_int as isize);
        name = name.offset(1 as ::core::ffi::c_int as isize);
    }
    *p = '\0' as i32 as ::core::ffi::c_char;
    return buffer as *const ::core::ffi::c_char;
}
unsafe extern "C" fn check_fid(mut fid: file_id) {
    if fid < 0 as ::core::ffi::c_int || fid >= MAX_FILES {
        fprintf(
            __stderrp,
            b"Aborting from driver/engine_tex.c:425\n\0" as *const u8 as *const ::core::ffi::c_char,
        );
        print_backtrace();
        abort();
    }
}
unsafe extern "C" fn record_seen(
    mut self_0: *mut tex_engine,
    mut entry: *mut fileentry_t,
    mut seen: ::core::ffi::c_int,
    mut time: ::core::ffi::c_int,
) {
    let mut p: *mut process_t = get_process(self_0);
    if (*p).trace_len > 0 as ::core::ffi::c_int
        && (*(*self_0)
            .trace
            .offset(((*p).trace_len - 1 as ::core::ffi::c_int) as isize))
        .entry
            == entry
        && ((*self_0).process_count <= 1 as ::core::ffi::c_int
            || (*self_0).processes[((*self_0).process_count - 2 as ::core::ffi::c_int) as usize]
                .trace_len
                != (*p).trace_len)
    {
        (*(*self_0)
            .trace
            .offset(((*p).trace_len - 1 as ::core::ffi::c_int) as isize))
        .time = time;
        (*entry).seen = seen;
        return;
    }
    if (*p).trace_len == (*self_0).trace_cap {
        let mut new_cap: ::core::ffi::c_int = if (*self_0).trace_cap == 0 as ::core::ffi::c_int {
            8 as ::core::ffi::c_int
        } else {
            (*self_0).trace_cap * 2 as ::core::ffi::c_int
        };
        let mut newtr: *mut trace_entry_t = calloc(
            ::core::mem::size_of::<trace_entry_t>() as size_t,
            new_cap as size_t,
        ) as *mut trace_entry_t;
        if newtr.is_null() {
            abort();
        }
        if !(*self_0).trace.is_null() {
            memcpy(
                newtr as *mut ::core::ffi::c_void,
                (*self_0).trace as *const ::core::ffi::c_void,
                ((*self_0).trace_cap as size_t)
                    .wrapping_mul(::core::mem::size_of::<trace_entry_t>() as size_t),
            );
            free((*self_0).trace as *mut ::core::ffi::c_void);
        }
        (*self_0).trace = newtr;
        (*self_0).trace_cap = new_cap;
    }
    *(*self_0).trace.offset((*p).trace_len as isize) = trace_entry_t {
        entry: entry,
        seen: (*entry).seen,
        time: time,
    };
    (*entry).seen = seen;
    (*p).trace_len += 1 as ::core::ffi::c_int;
}
unsafe extern "C" fn entry_data(mut e: *mut fileentry_t) -> *mut tbuf {
    if !(*e).saved.data.is_null() {
        return (*e).saved.data;
    }
    if !(*e).edit_data.is_null() {
        return (*e).edit_data;
    }
    return (*e).fs_data;
}
unsafe extern "C" fn lookup_path(
    mut self_0: *mut tex_engine,
    mut path: *const ::core::ffi::c_char,
    mut buf: *mut ::core::ffi::c_char,
    mut st: *mut stat,
) -> *const ::core::ffi::c_char {
    let mut st1: stat = stat {
        st_dev: 0,
        st_mode: 0,
        st_nlink: 0,
        st_ino: 0,
        st_uid: 0,
        st_gid: 0,
        st_rdev: 0,
        st_atimespec: timespec {
            tv_sec: 0,
            tv_nsec: 0,
        },
        st_mtimespec: timespec {
            tv_sec: 0,
            tv_nsec: 0,
        },
        st_ctimespec: timespec {
            tv_sec: 0,
            tv_nsec: 0,
        },
        st_birthtimespec: timespec {
            tv_sec: 0,
            tv_nsec: 0,
        },
        st_size: 0,
        st_blocks: 0,
        st_blksize: 0,
        st_flags: 0,
        st_gen: 0,
        st_lspare: 0,
        st_qspare: [0; 2],
    };
    if st.is_null() {
        st = &raw mut st1;
    }
    let mut fs_path: *const ::core::ffi::c_char = path;
    let mut inclusion_path: *const ::core::ffi::c_char = (*self_0).inclusion_path;
    while !(stat(fs_path, st) != -(1 as ::core::ffi::c_int)
        && (*st).st_mode as ::core::ffi::c_int & S_IFMT == S_IFREG)
    {
        fs_path = expand_path(&raw mut inclusion_path, path, buf);
        if fs_path.is_null() {
            break;
        }
    }
    return fs_path;
}
unsafe extern "C" fn need_snapshot(
    mut self_0: *mut tex_engine,
    mut time: ::core::ffi::c_int,
) -> bool {
    if (*self_0).fence_pos != -(1 as ::core::ffi::c_int) {
        return 0 as ::core::ffi::c_int != 0;
    }
    let mut process: ::core::ffi::c_int = (*self_0).process_count - 1 as ::core::ffi::c_int;
    if (*self_0).processes[process as usize].trace_len <= (*self_0).snapshot_barrier {
        return 0 as ::core::ffi::c_int != 0;
    }
    let mut last_time: ::core::ffi::c_int = 0;
    if process > 0 as ::core::ffi::c_int {
        if (*self_0).processes[process as usize].trace_len
            == (*self_0).processes[(process - 1 as ::core::ffi::c_int) as usize].trace_len
        {
            return 0 as ::core::ffi::c_int != 0;
        }
        last_time = (*(*self_0).trace.offset(
            ((*self_0).processes[(process - 1 as ::core::ffi::c_int) as usize].trace_len
                - 1 as ::core::ffi::c_int) as isize,
        ))
        .time;
    } else {
        if !xdv_index_output_started((*self_0).dvi) {
            return 0 as ::core::ffi::c_int != 0;
        }
        last_time = 0 as ::core::ffi::c_int;
    }
    return time > 500 as ::core::ffi::c_int + last_time;
}
unsafe extern "C" fn answer_query(mut self_0: *mut tex_engine, mut q: *mut query_t) {
    let mut p: *mut process_t = get_process(self_0);
    let mut a: answer_t = answer_t {
        tag: 0 as answer,
        c2rust_unnamed: C2RustUnnamed_10 {
            size: C2RustUnnamed_15 { size: 0 },
        },
    };
    let mut current_block_280: u64;
    match (*q).tag as ::core::ffi::c_uint {
        1146245199 | 1381453903 => {
            check_fid((*q).c2rust_unnamed.open.fid);
            let mut cell: *mut filecell_t = (&raw mut (*self_0).st.table as *mut filecell_t)
                .offset((*q).c2rust_unnamed.open.fid as isize)
                as *mut filecell_t;
            if !(*cell).entry.is_null() {
                fprintf(
                    __stderrp,
                    b"Aborting from driver/engine_tex.c:543\n\0" as *const u8
                        as *const ::core::ffi::c_char,
                );
                print_backtrace();
                abort();
            }
            let mut e: *mut fileentry_t = ::core::ptr::null_mut::<fileentry_t>();
            let mut fs_path_buffer: [::core::ffi::c_char; 1024] = [0; 1024];
            let mut fs_path: *const ::core::ffi::c_char =
                ::core::ptr::null::<::core::ffi::c_char>();
            if (*q).tag as ::core::ffi::c_uint
                == Q_OPRD as ::core::ffi::c_int as ::core::ffi::c_uint
            {
                e = filesystem_lookup((*self_0).fs, (*q).c2rust_unnamed.open.path);
                if e.is_null() || entry_data(e).is_null() {
                    fs_path = lookup_path(
                        self_0,
                        (*q).c2rust_unnamed.open.path,
                        &raw mut fs_path_buffer as *mut ::core::ffi::c_char,
                        ::core::ptr::null_mut::<stat>(),
                    );
                    if fs_path.is_null() && !(!e.is_null() && !(*e).edit_data.is_null()) {
                        e = filesystem_lookup_or_create(
                            (*self_0).fs,
                            (*q).c2rust_unnamed.open.path,
                        );
                        log_fileentry((*self_0).log, e);
                        record_seen(self_0, e, INT_MAX, (*q).time);
                        a.tag = A_PASS;
                        channel_write_answer((*self_0).c, (*p).fd, &raw mut a);
                        current_block_280 = 9675306770437543583;
                    } else {
                        current_block_280 = 8831408221741692167;
                    }
                } else {
                    current_block_280 = 8831408221741692167;
                }
            } else {
                current_block_280 = 8831408221741692167;
            }
            match current_block_280 {
                9675306770437543583 => {}
                _ => {
                    if e.is_null() {
                        e = filesystem_lookup_or_create(
                            (*self_0).fs,
                            (*q).c2rust_unnamed.open.path,
                        );
                    }
                    log_filecell((*self_0).log, cell);
                    log_fileentry((*self_0).log, e);
                    (*cell).entry = e;
                    if (*e).seen < 0 as ::core::ffi::c_int {
                        record_seen(self_0, e, 0 as ::core::ffi::c_int, (*q).time);
                    }
                    let mut level: accesslevel = (if (*q).tag as ::core::ffi::c_uint
                        == Q_OPRD as ::core::ffi::c_int as ::core::ffi::c_uint
                    {
                        FILE_READ as ::core::ffi::c_int
                    } else {
                        FILE_WRITE as ::core::ffi::c_int
                    }) as accesslevel;
                    if level as ::core::ffi::c_uint
                        == FILE_READ as ::core::ffi::c_int as ::core::ffi::c_uint
                    {
                        if ((*e).saved.level as ::core::ffi::c_uint)
                            < FILE_READ as ::core::ffi::c_int as ::core::ffi::c_uint
                        {
                            if fs_path.is_null() {
                                fs_path = lookup_path(
                                    self_0,
                                    (*q).c2rust_unnamed.open.path,
                                    &raw mut fs_path_buffer as *mut ::core::ffi::c_char,
                                    ::core::ptr::null_mut::<stat>(),
                                );
                            }
                            if fs_path.is_null() {
                                if (*e).edit_data.is_null() {
                                    fprintf(
                                        __stderrp,
                                        b"Aborting from driver/engine_tex.c:591\npath: %s\nmode:%c\n\0"
                                            as *const u8 as *const ::core::ffi::c_char,
                                        (*q).c2rust_unnamed.open.path,
                                        if (*q).tag as ::core::ffi::c_uint
                                            == Q_OPRD as ::core::ffi::c_int as ::core::ffi::c_uint
                                        {
                                            'r' as i32
                                        } else {
                                            'w' as i32
                                        },
                                    );
                                    print_backtrace();
                                    abort();
                                }
                                (*e).saved.level = FILE_READ;
                                memset(
                                    &raw mut (*e).fs_stat as *mut ::core::ffi::c_void,
                                    0 as ::core::ffi::c_int,
                                    ::core::mem::size_of::<stat>() as size_t,
                                );
                                current_block_280 = 15004371738079956865;
                            } else {
                                if fs_path
                                    == (*q).c2rust_unnamed.open.path as *const ::core::ffi::c_char
                                {
                                    fs_path = (*e).path;
                                }
                                let mut data: *mut tbuf = tbuf_read_file(fs_path);
                                if data.is_null() {
                                    log_filecell((*self_0).log, cell);
                                    (*cell).entry = ::core::ptr::null_mut::<fileentry_t>();
                                    record_seen(self_0, e, INT_MAX, (*q).time);
                                    a.tag = A_PASS;
                                    channel_write_answer((*self_0).c, (*p).fd, &raw mut a);
                                    current_block_280 = 9675306770437543583;
                                } else {
                                    tbuf_drop((*e).fs_data);
                                    (*e).fs_data = data;
                                    (*e).saved.level = FILE_READ;
                                    stat(fs_path, &raw mut (*e).fs_stat);
                                    current_block_280 = 15004371738079956865;
                                }
                            }
                        } else {
                            current_block_280 = 15004371738079956865;
                        }
                    } else {
                        (*e).saved.data = tbuf_new(1024 as size_t);
                        (*e).saved.level = level;
                        current_block_280 = 15004371738079956865;
                    }
                    match current_block_280 {
                        9675306770437543583 => {}
                        _ => {
                            if level as ::core::ffi::c_uint
                                != FILE_READ as ::core::ffi::c_int as ::core::ffi::c_uint
                            {
                                if strcmp(
                                    (*q).c2rust_unnamed.open.path,
                                    b"stdout\0" as *const u8 as *const ::core::ffi::c_char,
                                ) == 0 as ::core::ffi::c_int
                                {
                                    if !(*self_0).st.__stdoutp.entry.is_null() {
                                        fprintf(
                                            __stderrp,
                                            b"[error] two stdouts!\n\0" as *const u8
                                                as *const ::core::ffi::c_char,
                                        );
                                        fprintf(
                                            __stderrp,
                                            b"Aborting from driver/engine_tex.c:631\n\0"
                                                as *const u8
                                                as *const ::core::ffi::c_char,
                                        );
                                        print_backtrace();
                                        abort();
                                    }
                                    log_filecell((*self_0).log, &raw mut (*self_0).st.__stdoutp);
                                    (*self_0).st.__stdoutp.entry = e;
                                } else {
                                    let mut ext: *mut ::core::ffi::c_char = last_index(
                                        (*q).c2rust_unnamed.open.path,
                                        '.' as i32 as ::core::ffi::c_char,
                                    );
                                    if !ext.is_null() {
                                        if strcmp(
                                            ext,
                                            b"xdv\0" as *const u8 as *const ::core::ffi::c_char,
                                        ) == 0 as ::core::ffi::c_int
                                            || strcmp(
                                                ext,
                                                b"dvi\0" as *const u8 as *const ::core::ffi::c_char,
                                            ) == 0 as ::core::ffi::c_int
                                            || strcmp(
                                                ext,
                                                b"pdf\0" as *const u8 as *const ::core::ffi::c_char,
                                            ) == 0 as ::core::ffi::c_int
                                        {
                                            if !(*self_0).st.document.entry.is_null() {
                                                fprintf(
                                                    __stderrp,
                                                    b"[error] two outputs!\n\0" as *const u8
                                                        as *const ::core::ffi::c_char,
                                                );
                                                fprintf(
                                                    __stderrp,
                                                    b"Aborting from driver/engine_tex.c:647\n\0"
                                                        as *const u8
                                                        as *const ::core::ffi::c_char,
                                                );
                                                print_backtrace();
                                                abort();
                                            }
                                            log_filecell(
                                                (*self_0).log,
                                                &raw mut (*self_0).st.document,
                                            );
                                            (*self_0).st.document.entry = e;
                                            xdv_index_reset((*self_0).dvi);
                                        } else if strcmp(
                                            ext,
                                            b"synctex\0" as *const u8 as *const ::core::ffi::c_char,
                                        ) == 0 as ::core::ffi::c_int
                                        {
                                            if !(*self_0).st.synctex.entry.is_null() {
                                                fprintf(
                                                    __stderrp,
                                                    b"[error] two synctex!\n\0" as *const u8
                                                        as *const ::core::ffi::c_char,
                                                );
                                                fprintf(
                                                    __stderrp,
                                                    b"Aborting from driver/engine_tex.c:658\n\0"
                                                        as *const u8
                                                        as *const ::core::ffi::c_char,
                                                );
                                                print_backtrace();
                                                abort();
                                            }
                                            log_filecell(
                                                (*self_0).log,
                                                &raw mut (*self_0).st.synctex,
                                            );
                                            (*self_0).st.synctex.entry = e;
                                        } else if strcmp(
                                            ext,
                                            b"log\0" as *const u8 as *const ::core::ffi::c_char,
                                        ) == 0 as ::core::ffi::c_int
                                        {
                                            if !(*self_0).st.log.entry.is_null() {
                                                fprintf(
                                                    __stderrp,
                                                    b"[error] two log files!\n\0" as *const u8
                                                        as *const ::core::ffi::c_char,
                                                );
                                                fprintf(
                                                    __stderrp,
                                                    b"Aborting from driver/engine_tex.c:668\n\0"
                                                        as *const u8
                                                        as *const ::core::ffi::c_char,
                                                );
                                                print_backtrace();
                                                abort();
                                            }
                                            log_filecell((*self_0).log, &raw mut (*self_0).st.log);
                                            (*self_0).st.log.entry = e;
                                            (*self_0).log_entry = e;
                                        }
                                    }
                                }
                            }
                            let mut n: ::core::ffi::c_int =
                                strlen((*q).c2rust_unnamed.open.path) as ::core::ffi::c_int;
                            a.c2rust_unnamed.open.path_len = n;
                            a.tag = A_OPEN;
                            memmove(
                                channel_get_buffer((*self_0).c, n as size_t),
                                (*q).c2rust_unnamed.open.path as *const ::core::ffi::c_void,
                                n as size_t,
                            );
                            channel_write_answer((*self_0).c, (*p).fd, &raw mut a);
                        }
                    }
                }
            }
        }
        1145128274 => {
            check_fid((*q).c2rust_unnamed.read.fid);
            let mut e_0: *mut fileentry_t =
                (*self_0).st.table[(*q).c2rust_unnamed.read.fid as usize].entry;
            if e_0.is_null() {
                fprintf(
                    __stderrp,
                    b"Aborting from driver/engine_tex.c:688\n\0" as *const u8
                        as *const ::core::ffi::c_char,
                );
                print_backtrace();
                abort();
            }
            if ((*e_0).saved.level as ::core::ffi::c_uint)
                < FILE_READ as ::core::ffi::c_int as ::core::ffi::c_uint
            {
                fprintf(
                    __stderrp,
                    b"Aborting from driver/engine_tex.c:689\n\0" as *const u8
                        as *const ::core::ffi::c_char,
                );
                print_backtrace();
                abort();
            }
            let mut data_0: *mut tbuf = entry_data(e_0);
            if (*q).c2rust_unnamed.read.pos > (*data_0).len as ::core::ffi::c_int {
                fprintf(
                    __stderrp,
                    b"read:%d\ndata->len:%d\n\0" as *const u8 as *const ::core::ffi::c_char,
                    (*q).c2rust_unnamed.read.pos,
                    (*data_0).len as ::core::ffi::c_int,
                );
                fprintf(
                    __stderrp,
                    b"Aborting from driver/engine_tex.c:694\n\0" as *const u8
                        as *const ::core::ffi::c_char,
                );
                print_backtrace();
                abort();
            }
            let mut n_0: ssize_t = (*q).c2rust_unnamed.read.size as ssize_t;
            if n_0 > (*data_0).len as ssize_t - (*q).c2rust_unnamed.read.pos as ssize_t {
                n_0 = (*data_0)
                    .len
                    .wrapping_sub((*q).c2rust_unnamed.read.pos as size_t)
                    as ssize_t;
            }
            let mut fork_0: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
            if (*self_0).fence_pos >= 0 as ::core::ffi::c_int
                && (*self_0).fences[(*self_0).fence_pos as usize].entry == e_0
                && ((*self_0).fences[(*self_0).fence_pos as usize].position as ssize_t)
                    < (*q).c2rust_unnamed.read.pos as ssize_t + n_0
            {
                if n_0 < 0 as ssize_t {
                    fprintf(
                        __stderrp,
                        b"Aborting from driver/engine_tex.c:706\n\0" as *const u8
                            as *const ::core::ffi::c_char,
                    );
                    print_backtrace();
                    abort();
                }
                n_0 = ((*self_0).fences[(*self_0).fence_pos as usize].position
                    - (*q).c2rust_unnamed.read.pos) as ssize_t;
                fork_0 = (n_0 == 0 as ssize_t) as ::core::ffi::c_int;
                if n_0 < 0 as ssize_t {
                    fprintf(
                        __stderrp,
                        b"Aborting from driver/engine_tex.c:710\nn:%d fence_pos:%d read_pos:%d\n\0"
                            as *const u8 as *const ::core::ffi::c_char,
                        n_0 as ::core::ffi::c_int,
                        (*self_0).fences[(*self_0).fence_pos as usize].position,
                        (*q).c2rust_unnamed.read.pos,
                    );
                    print_backtrace();
                    abort();
                }
            }
            if fork_0 != 0 {
                a.tag = A_FORK;
                (*self_0).fence_pos -= 1 as ::core::ffi::c_int;
            } else if need_snapshot(self_0, (*q).time) {
                a.tag = A_FORK;
            } else {
                memmove(
                    channel_get_buffer((*self_0).c, n_0 as size_t),
                    (*data_0).data.offset((*q).c2rust_unnamed.read.pos as isize)
                        as *const ::core::ffi::c_void,
                    n_0 as size_t,
                );
                a.tag = A_READ;
                a.c2rust_unnamed.read.size = n_0 as ::core::ffi::c_int;
            }
            channel_write_answer((*self_0).c, (*p).fd, &raw mut a);
        }
        1145983041 => {
            let mut e_1: *mut fileentry_t = ::core::ptr::null_mut::<fileentry_t>();
            if (*q).c2rust_unnamed.apnd.fid == -(1 as ::core::ffi::c_int) {
                e_1 = (*self_0).st.__stdoutp.entry;
                if e_1.is_null() {
                    e_1 = filesystem_lookup_or_create(
                        (*self_0).fs,
                        b"stdout\0" as *const u8 as *const ::core::ffi::c_char,
                    );
                    log_fileentry((*self_0).log, e_1);
                    log_filecell((*self_0).log, &raw mut (*self_0).st.__stdoutp);
                    (*self_0).st.__stdoutp.entry = e_1;
                    if (*e_1).saved.data.is_null() {
                        (*e_1).saved.data = tbuf_new(1024 as size_t);
                        (*e_1).saved.level = FILE_WRITE;
                    }
                }
            } else {
                check_fid((*q).c2rust_unnamed.apnd.fid);
                e_1 = (*self_0).st.table[(*q).c2rust_unnamed.apnd.fid as usize].entry;
            }
            if e_1.is_null()
                || (*e_1).saved.level as ::core::ffi::c_uint
                    != FILE_WRITE as ::core::ffi::c_int as ::core::ffi::c_uint
            {
                fprintf(
                    __stderrp,
                    b"Aborting from driver/engine_tex.c:756\n\0" as *const u8
                        as *const ::core::ffi::c_char,
                );
                print_backtrace();
                abort();
            }
            log_fileentry((*self_0).log, e_1);
            tbuf_append(
                (*e_1).saved.data,
                (*q).c2rust_unnamed.apnd.buf as *const ::core::ffi::c_void,
                (*q).c2rust_unnamed.apnd.size as size_t,
            );
            if (*self_0).st.document.entry == e_1 {
                xdv_index_update(
                    (*self_0).dvi,
                    (*(*e_1).saved.data).data,
                    (*(*e_1).saved.data).len,
                );
            } else if !((*self_0).st.synctex.entry == e_1
                || (*self_0).st.log.entry == e_1
                || (*self_0).st.__stdoutp.entry == e_1)
            {
                (*self_0).aux_dirty = true_0 != 0;
            }
            a.tag = A_DONE;
            channel_write_answer((*self_0).c, (*p).fd, &raw mut a);
        }
        1397705795 => {
            check_fid((*q).c2rust_unnamed.clos.fid);
            let mut cell_0: *mut filecell_t = (&raw mut (*self_0).st.table as *mut filecell_t)
                .offset((*q).c2rust_unnamed.clos.fid as isize)
                as *mut filecell_t;
            let mut e_2: *mut fileentry_t = (*cell_0).entry;
            if e_2.is_null() {
                fprintf(
                    __stderrp,
                    b"Aborting from driver/engine_tex.c:778\n\0" as *const u8
                        as *const ::core::ffi::c_char,
                );
                print_backtrace();
                abort();
            }
            log_filecell((*self_0).log, cell_0);
            (*cell_0).entry = ::core::ptr::null_mut::<fileentry_t>();
            if (*self_0).st.__stdoutp.entry == e_2 {
                log_filecell((*self_0).log, &raw mut (*self_0).st.__stdoutp);
                (*self_0).st.__stdoutp.entry = ::core::ptr::null_mut::<fileentry_t>();
            }
            if (*self_0).st.log.entry == e_2 {
                log_filecell((*self_0).log, &raw mut (*self_0).st.log);
                (*self_0).st.log.entry = ::core::ptr::null_mut::<fileentry_t>();
            }
            a.tag = A_DONE;
            channel_write_answer((*self_0).c, (*p).fd, &raw mut a);
        }
        1163544915 => {
            check_fid((*q).c2rust_unnamed.clos.fid);
            let mut e_3: *mut fileentry_t =
                (*self_0).st.table[(*q).c2rust_unnamed.clos.fid as usize].entry;
            if e_3.is_null()
                || ((*e_3).saved.level as ::core::ffi::c_uint)
                    < FILE_READ as ::core::ffi::c_int as ::core::ffi::c_uint
            {
                fprintf(
                    __stderrp,
                    b"Aborting from driver/engine_tex.c:802\n\0" as *const u8
                        as *const ::core::ffi::c_char,
                );
                print_backtrace();
                abort();
            }
            a.tag = A_SIZE;
            a.c2rust_unnamed.size.size = (*entry_data(e_3)).len as uint32_t;
            channel_write_answer((*self_0).c, (*p).fd, &raw mut a);
        }
        1296651341 => {
            check_fid((*q).c2rust_unnamed.clos.fid);
            let mut e_4: *mut fileentry_t =
                (*self_0).st.table[(*q).c2rust_unnamed.clos.fid as usize].entry;
            if e_4.is_null()
                || ((*e_4).saved.level as ::core::ffi::c_uint)
                    < FILE_READ as ::core::ffi::c_int as ::core::ffi::c_uint
            {
                fprintf(
                    __stderrp,
                    b"Aborting from driver/engine_tex.c:812\n\0" as *const u8
                        as *const ::core::ffi::c_char,
                );
                print_backtrace();
                abort();
            }
            a.tag = A_MTIM;
            a.c2rust_unnamed.mtim.mtime = (*e_4).fs_stat.st_mtimespec.tv_sec as uint32_t;
            channel_write_answer((*self_0).c, (*p).fd, &raw mut a);
        }
        1313162579 => {
            check_fid((*q).c2rust_unnamed.seen.fid);
            let mut e_5: *mut fileentry_t =
                (*self_0).st.table[(*q).c2rust_unnamed.seen.fid as usize].entry;
            if e_5.is_null() {
                fprintf(
                    __stderrp,
                    b"Aborting from driver/engine_tex.c:822\n\0" as *const u8
                        as *const ::core::ffi::c_char,
                );
                print_backtrace();
                abort();
            }
            if ((*e_5).saved.level as ::core::ffi::c_uint)
                < FILE_READ as ::core::ffi::c_int as ::core::ffi::c_uint
            {
                fprintf(
                    __stderrp,
                    b"Aborting from driver/engine_tex.c:823\n\0" as *const u8
                        as *const ::core::ffi::c_char,
                );
                print_backtrace();
                abort();
            }
            if (*self_0).fence_pos >= 0 as ::core::ffi::c_int
                && (*self_0).fences[(*self_0).fence_pos as usize].entry == e_5
                && (*self_0).fences[(*self_0).fence_pos as usize].position
                    < (*q).c2rust_unnamed.seen.pos
            {
                fprintf(
                    __stderrp,
                    b"Seen position invalid wrt fence:\n  file %s, seen: %d -> %d\n  fence #%d position: %d\n\0"
                        as *const u8 as *const ::core::ffi::c_char,
                    (*e_5).path,
                    (*e_5).seen,
                    (*q).c2rust_unnamed.seen.pos,
                    (*self_0).fence_pos,
                    (*self_0).fences[(*self_0).fence_pos as usize].position,
                );
                fprintf(
                    __stderrp,
                    b"Aborting from driver/engine_tex.c:835\n\0" as *const u8
                        as *const ::core::ffi::c_char,
                );
                print_backtrace();
                abort();
            }
            if (*q).c2rust_unnamed.seen.pos > (*e_5).seen {
                log_fileentry((*self_0).log, e_5);
                record_seen(self_0, e_5, (*q).c2rust_unnamed.seen.pos, (*q).time);
            }
        }
        1128878151 => {
            let mut e_6: *mut fileentry_t =
                filesystem_lookup((*self_0).fs, (*q).c2rust_unnamed.gpic.path);
            if !e_6.is_null()
                && (*e_6).saved.level as ::core::ffi::c_uint
                    == FILE_READ as ::core::ffi::c_int as ::core::ffi::c_uint
                && (*e_6).pic_cache.type_0 == (*q).c2rust_unnamed.gpic.type_0
                && (*e_6).pic_cache.page == (*q).c2rust_unnamed.gpic.page
            {
                a.c2rust_unnamed.gpic.bounds[0 as ::core::ffi::c_int as usize] =
                    (*e_6).pic_cache.bounds[0 as ::core::ffi::c_int as usize];
                a.c2rust_unnamed.gpic.bounds[1 as ::core::ffi::c_int as usize] =
                    (*e_6).pic_cache.bounds[1 as ::core::ffi::c_int as usize];
                a.c2rust_unnamed.gpic.bounds[2 as ::core::ffi::c_int as usize] =
                    (*e_6).pic_cache.bounds[2 as ::core::ffi::c_int as usize];
                a.c2rust_unnamed.gpic.bounds[3 as ::core::ffi::c_int as usize] =
                    (*e_6).pic_cache.bounds[3 as ::core::ffi::c_int as usize];
                a.tag = A_GPIC;
            } else {
                a.tag = A_PASS;
            }
            channel_write_answer((*self_0).c, (*p).fd, &raw mut a);
        }
        1128878163 => {
            let mut e_7: *mut fileentry_t =
                filesystem_lookup((*self_0).fs, (*q).c2rust_unnamed.spic.path);
            if !e_7.is_null()
                && (*e_7).saved.level as ::core::ffi::c_uint
                    == FILE_READ as ::core::ffi::c_int as ::core::ffi::c_uint
            {
                (*e_7).pic_cache = (*q).c2rust_unnamed.spic.cache;
            }
            a.tag = A_DONE;
            channel_write_answer((*self_0).c, (*p).fd, &raw mut a);
        }
        1145849923 => {
            if (*self_0).process_count == MAX_PROCESS as ::core::ffi::c_int {
                decimate_processes(self_0);
                p = get_process(self_0);
            }
            channel_reset((*self_0).c);
            (*self_0).process_count += 1 as ::core::ffi::c_int;
            let mut p2: *mut process_t = get_process(self_0);
            (*p).snap = log_snapshot((*self_0).log);
            (*p2).fd = (*q).c2rust_unnamed.chld.fd;
            (*p2).pid = (*q).c2rust_unnamed.chld.pid;
            (*p2).trace_len = (*p).trace_len;
            (*p2).forked = true_0 != 0;
            fcntl((*p2).fd, F_SETFD, FD_CLOEXEC);
            a.tag = A_DONE;
            channel_write_answer((*self_0).c, (*p).fd, &raw mut a);
        }
        1112821318 => {
            (*self_0).snapshot_barrier = maxi((*self_0).snapshot_barrier, (*p).trace_len);
            (*self_0).barrier_pending = true_0 != 0;
            fprintf(
                __stderrp,
                b"[process] font barrier at trace position %d\n\0" as *const u8
                    as *const ::core::ffi::c_char,
                (*p).trace_len,
            );
        }
        _ => {}
    };
}
unsafe extern "C" fn revert_trace(mut te: *mut trace_entry_t) {
    (*(*te).entry).seen = (*te).seen;
}
unsafe extern "C" fn rollback_processes(
    mut self_0: *mut tex_engine,
    mut reverted: ::core::ffi::c_int,
    mut trace: ::core::ffi::c_int,
) {
    (*self_0).aux_dirty = false_0 != 0;
    (*self_0).finishing = false_0 != 0;
    while (*self_0).process_count > 0 as ::core::ffi::c_int
        && ((*get_process(self_0)).trace_len > trace
            || (*get_process(self_0)).fd == -(1 as ::core::ffi::c_int))
    {
        pop_process(self_0);
    }
    let mut trace_len: ::core::ffi::c_int = if (*self_0).process_count == 0 as ::core::ffi::c_int {
        0 as ::core::ffi::c_int
    } else {
        (*get_process(self_0)).trace_len
    };
    while reverted > trace_len {
        reverted -= 1;
        revert_trace((*self_0).trace.offset(reverted as isize) as *mut trace_entry_t);
    }
    if !(*self_0).st.document.entry.is_null() {
        xdv_index_update(
            (*self_0).dvi,
            (*(*(*self_0).st.document.entry).saved.data).data,
            (*(*(*self_0).st.document.entry).saved.data).len,
        );
    } else {
        xdv_index_reset((*self_0).dvi);
    };
}
unsafe extern "C" fn possible_fence(mut te: *mut trace_entry_t) -> bool {
    if (*te).seen == INT_MAX || (*te).seen == -(1 as ::core::ffi::c_int) {
        return 0 as ::core::ffi::c_int != 0;
    }
    if (*(*te).entry).saved.level as ::core::ffi::c_uint
        > FILE_READ as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        return 0 as ::core::ffi::c_int != 0;
    }
    return 1 as ::core::ffi::c_int != 0;
}
unsafe extern "C" fn compute_fences(
    mut self_0: *mut tex_engine,
    mut trace: ::core::ffi::c_int,
    mut offset: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    (*self_0).fence_pos = -(1 as ::core::ffi::c_int);
    if trace <= 0 as ::core::ffi::c_int {
        return trace;
    }
    if (*get_process(self_0)).trace_len <= trace {
        fprintf(
            __stderrp,
            b"Aborting from driver/engine_tex.c:958\n\0" as *const u8 as *const ::core::ffi::c_char,
        );
        print_backtrace();
        abort();
    }
    (*self_0).fence_pos = 0 as ::core::ffi::c_int;
    offset =
        offset - 64 as ::core::ffi::c_int & !(64 as ::core::ffi::c_int - 1 as ::core::ffi::c_int);
    if offset < (*(*self_0).trace.offset(trace as isize)).seen {
        offset = (*(*self_0).trace.offset(trace as isize)).seen;
    }
    if offset == -(1 as ::core::ffi::c_int) {
        offset = 0 as ::core::ffi::c_int;
    }
    (*self_0).fences[0 as ::core::ffi::c_int as usize].entry =
        (*(*self_0).trace.offset(trace as isize)).entry;
    (*self_0).fences[0 as ::core::ffi::c_int as usize].position = offset;
    let mut delta: ::core::ffi::c_int = 50 as ::core::ffi::c_int;
    let mut time: ::core::ffi::c_int =
        (*(*self_0).trace.offset(trace as isize)).time - 10 as ::core::ffi::c_int;
    let mut target_process: ::core::ffi::c_int = (*self_0).process_count - 1 as ::core::ffi::c_int;
    while target_process >= 0 as ::core::ffi::c_int
        && (*self_0).processes[target_process as usize].trace_len > trace
    {
        target_process -= 1 as ::core::ffi::c_int;
    }
    let mut target_trace: ::core::ffi::c_int = if target_process >= 0 as ::core::ffi::c_int {
        (*self_0).processes[target_process as usize].trace_len
    } else {
        -(1 as ::core::ffi::c_int)
    };
    while trace > target_trace && (*self_0).fence_pos < 15 as ::core::ffi::c_int {
        if (*(*self_0).trace.offset(trace as isize)).time <= time
            && possible_fence((*self_0).trace.offset(trace as isize) as *mut trace_entry_t)
                as ::core::ffi::c_int
                != 0
        {
            (*self_0).fence_pos += 1 as ::core::ffi::c_int;
            (*self_0).fences[(*self_0).fence_pos as usize].entry =
                (*(*self_0).trace.offset(trace as isize)).entry;
            (*self_0).fences[(*self_0).fence_pos as usize].position =
                (*(*self_0).trace.offset(trace as isize)).seen;
            if (*self_0).fences[(*self_0).fence_pos as usize].position == -(1 as ::core::ffi::c_int)
            {
                (*self_0).fences[(*self_0).fence_pos as usize].position = 0 as ::core::ffi::c_int;
            }
            time -= delta;
            delta *= 2 as ::core::ffi::c_int;
        }
        trace -= 1 as ::core::ffi::c_int;
    }
    return trace;
}
#[no_mangle]
pub unsafe extern "C" fn txp_engine_step(
    mut self_0: *mut tex_engine,
    mut restart_if_needed: bool,
) -> bool {
    if restart_if_needed {
        prepare_process(self_0 as *mut tex_engine);
    }
    if txp_engine_get_status(self_0) as ::core::ffi::c_uint
        == DOC_RUNNING as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        let mut q: query_t = query_t {
            time: 0,
            tag: 0 as query,
            c2rust_unnamed: C2RustUnnamed {
                open: C2RustUnnamed_9 {
                    fid: 0,
                    path: ::core::ptr::null_mut::<::core::ffi::c_char>(),
                    kind: 0 as txp_file_kind,
                },
            },
        };
        let mut fd: ::core::ffi::c_int = (*get_process(self_0 as *mut tex_engine)).fd;
        if fd == -(1 as ::core::ffi::c_int) {
            return 0 as ::core::ffi::c_int != 0;
        }
        if !channel_has_pending_query((*self_0).c, fd, 10 as ::core::ffi::c_int) {
            return 0 as ::core::ffi::c_int != 0;
        }
        if !read_query(self_0 as *mut tex_engine, (*self_0).c, &raw mut q) {
            (*get_process(self_0 as *mut tex_engine)).fd = -(1 as ::core::ffi::c_int);
            if (*self_0).process_count == 1 as ::core::ffi::c_int {
                reap_root(self_0 as *mut tex_engine, false_0 != 0);
            }
            return 0 as ::core::ffi::c_int != 0;
        }
        touch_query_clock(self_0 as *mut tex_engine);
        answer_query(self_0 as *mut tex_engine, &raw mut q);
        channel_flush((*self_0).c, fd);
        return 1 as ::core::ffi::c_int != 0;
    }
    return 0 as ::core::ffi::c_int != 0;
}
unsafe extern "C" fn scan_entry(
    mut self_0: *mut tex_engine,
    mut e: *mut fileentry_t,
) -> ::core::ffi::c_int {
    if ((*e).saved.level as ::core::ffi::c_uint)
        < FILE_READ as ::core::ffi::c_int as ::core::ffi::c_uint
        || (*e).fs_stat.st_ino == 0 as __darwin_ino64_t
        || !(*e).edit_data.is_null()
    {
        return -(1 as ::core::ffi::c_int);
    }
    let mut st: stat = stat {
        st_dev: 0,
        st_mode: 0,
        st_nlink: 0,
        st_ino: 0,
        st_uid: 0,
        st_gid: 0,
        st_rdev: 0,
        st_atimespec: timespec {
            tv_sec: 0,
            tv_nsec: 0,
        },
        st_mtimespec: timespec {
            tv_sec: 0,
            tv_nsec: 0,
        },
        st_ctimespec: timespec {
            tv_sec: 0,
            tv_nsec: 0,
        },
        st_birthtimespec: timespec {
            tv_sec: 0,
            tv_nsec: 0,
        },
        st_size: 0,
        st_blocks: 0,
        st_blksize: 0,
        st_flags: 0,
        st_gen: 0,
        st_lspare: 0,
        st_qspare: [0; 2],
    };
    let mut fs_path_buffer: [::core::ffi::c_char; 1024] = [0; 1024];
    let mut fs_path: *const ::core::ffi::c_char = lookup_path(
        self_0,
        (*e).path,
        &raw mut fs_path_buffer as *mut ::core::ffi::c_char,
        &raw mut st,
    );
    if fs_path.is_null() {
        if !(*e).fs_data.is_null() {
            tbuf_drop((*e).fs_data);
            (*e).fs_data = ::core::ptr::null_mut::<tbuf>();
            memset(
                &raw mut (*e).fs_stat as *mut ::core::ffi::c_void,
                0 as ::core::ffi::c_int,
                ::core::mem::size_of::<stat>() as size_t,
            );
            return 0 as ::core::ffi::c_int;
        }
        return -(1 as ::core::ffi::c_int);
    }
    if stat_same(&raw mut st, &raw mut (*e).fs_stat) {
        return -(1 as ::core::ffi::c_int);
    }
    (*e).fs_stat = st;
    let mut buf: *mut tbuf = tbuf_read_file(fs_path);
    if buf.is_null() {
        return -(1 as ::core::ffi::c_int);
    }
    (*e).pic_cache.type_0 = -(1 as ::core::ffi::c_int);
    let mut olen: ::core::ffi::c_int = if !(*e).fs_data.is_null() {
        (*(*e).fs_data).len as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    };
    let mut nlen: ::core::ffi::c_int = (*buf).len as ::core::ffi::c_int;
    let mut len: ::core::ffi::c_int = if olen < nlen { olen } else { nlen };
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < len
        && *(*(*e).fs_data).data.offset(i as isize) as ::core::ffi::c_int
            == *(*buf).data.offset(i as isize) as ::core::ffi::c_int
    {
        i += 1 as ::core::ffi::c_int;
    }
    if i == len && olen == nlen {
        tbuf_drop(buf);
        return -(1 as ::core::ffi::c_int);
    }
    fprintf(
        __stderrp,
        b"[scan] %s changed at byte %d\n\0" as *const u8 as *const ::core::ffi::c_char,
        (*e).path,
        i,
    );
    tbuf_drop((*e).fs_data);
    (*e).fs_data = buf;
    return i;
}
pub const NOT_IN_TRANSACTION: ::core::ffi::c_int = -(2 as ::core::ffi::c_int);
unsafe extern "C" fn rollback_begin(mut self_0: *mut tex_engine) {
    if (*self_0).rollback.trace_len != NOT_IN_TRANSACTION {
        abort();
    }
    if (*self_0).process_count == 0 as ::core::ffi::c_int {
        return;
    }
    (*self_0).rollback.trace_len = (*get_process(self_0)).trace_len;
    (*self_0).rollback.offset = -(1 as ::core::ffi::c_int);
    (*self_0).rollback.flush = 0 as ::core::ffi::c_int;
}
unsafe extern "C" fn rollback_end(
    mut self_0: *mut tex_engine,
    mut tracep: *mut ::core::ffi::c_int,
    mut offsetp: *mut ::core::ffi::c_int,
) -> bool {
    let mut trace_len: ::core::ffi::c_int = (*self_0).rollback.trace_len;
    (*self_0).rollback.trace_len = NOT_IN_TRANSACTION;
    if trace_len == NOT_IN_TRANSACTION {
        return false_0 != 0;
    }
    let mut p: *mut process_t = get_process(self_0);
    if trace_len == (*p).trace_len {
        if (*self_0).rollback.flush == 0 {
            return false_0 != 0;
        }
        if (*p).fd > -(1 as ::core::ffi::c_int) {
            let mut a: ask_t = ask_t {
                tag: 0 as ask,
                c2rust_unnamed: C2RustUnnamed_16 {
                    term: C2RustUnnamed_19 { pid: 0 },
                },
            };
            a.tag = C_FLSH;
            channel_write_ask((*self_0).c, (*p).fd, &raw mut a);
            channel_flush((*self_0).c, (*p).fd);
            return false_0 != 0;
        }
        if trace_len > 0 as ::core::ffi::c_int {
            trace_len -= 1 as ::core::ffi::c_int;
            revert_trace((*self_0).trace.offset(trace_len as isize) as *mut trace_entry_t);
        }
        if trace_len > 0 as ::core::ffi::c_int {
            (*self_0).rollback.offset = (*(*self_0).trace.offset(trace_len as isize)).seen;
        }
    }
    if !tracep.is_null() {
        *tracep = trace_len;
    }
    if !offsetp.is_null() {
        *offsetp = (*self_0).rollback.offset;
    }
    return true_0 != 0;
}
unsafe extern "C" fn process_pending_messages(mut self_0: *mut tex_engine) -> bool {
    if (*self_0).rollback.flush != 0 {
        return 1 as ::core::ffi::c_int != 0;
    }
    let mut p: *mut process_t = get_process(self_0);
    if (*p).fd == -(1 as ::core::ffi::c_int) {
        return 1 as ::core::ffi::c_int != 0;
    }
    let mut nothing_seen: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
    if !channel_has_pending_query((*self_0).c, (*p).fd, 10 as ::core::ffi::c_int) {
        fprintf(
            __stderrp,
            b"[kill] worker might be stuck, killing\n\0" as *const u8 as *const ::core::ffi::c_char,
        );
        close_process(p);
    } else {
        match channel_peek_query((*self_0).c, (*p).fd) as ::core::ffi::c_uint {
            1313162579 => {
                let mut q: query_t = query_t {
                    time: 0,
                    tag: 0 as query,
                    c2rust_unnamed: C2RustUnnamed {
                        open: C2RustUnnamed_9 {
                            fid: 0,
                            path: ::core::ptr::null_mut::<::core::ffi::c_char>(),
                            kind: 0 as txp_file_kind,
                        },
                    },
                };
                if !read_query(self_0, (*self_0).c, &raw mut q) {
                    (*p).fd = -(1 as ::core::ffi::c_int);
                } else {
                    answer_query(self_0, &raw mut q);
                    nothing_seen = 0 as ::core::ffi::c_int;
                }
            }
            _ => {}
        }
    }
    (*self_0).rollback.flush = 1 as ::core::ffi::c_int;
    return nothing_seen != 0;
}
unsafe extern "C" fn rollback_add_change(
    mut self_0: *mut tex_engine,
    mut e: *mut fileentry_t,
    mut changed: ::core::ffi::c_int,
) {
    let mut trace_len: ::core::ffi::c_int = (*self_0).rollback.trace_len;
    if trace_len == NOT_IN_TRANSACTION {
        return;
    }
    if (*e).seen < changed && trace_len == (*get_process(self_0)).trace_len {
        if process_pending_messages(self_0) {
            return;
        }
        (*self_0).rollback.trace_len = (*get_process(self_0)).trace_len;
        trace_len = (*self_0).rollback.trace_len;
    }
    if (*e).seen < changed {
        return;
    }
    while (*e).seen >= changed {
        if trace_len <= 0 as ::core::ffi::c_int {
            fprintf(
                __stderrp,
                b"Aborting from driver/engine_tex.c:1212\nrollback walked past the first trace entry for %s\n\0"
                    as *const u8 as *const ::core::ffi::c_char,
                (*e).path,
            );
            print_backtrace();
            abort();
        }
        trace_len -= 1;
        revert_trace((*self_0).trace.offset(trace_len as isize) as *mut trace_entry_t);
    }
    if (*(*self_0).trace.offset(trace_len as isize)).entry != e {
        fprintf(
            __stderrp,
            b"Rollback position: %d. Entries: %d. Seen: %d. Changed: %d.\n\0" as *const u8
                as *const ::core::ffi::c_char,
            trace_len,
            (*get_process(self_0)).trace_len,
            (*e).seen,
            changed,
        );
        fprintf(
            __stderrp,
            b"Aborting from driver/engine_tex.c:1221\n\0" as *const u8
                as *const ::core::ffi::c_char,
        );
        print_backtrace();
        abort();
    }
    (*self_0).rollback.trace_len = trace_len;
    (*self_0).rollback.offset = changed;
}
#[no_mangle]
pub unsafe extern "C" fn txp_engine_notify_file_changes(
    mut self_0: *mut tex_engine,
    mut entry: *mut fileentry_t,
    mut offset: ::core::ffi::c_int,
) {
    rollback_add_change(self_0 as *mut tex_engine, entry, offset);
}
unsafe extern "C" fn is_system_output(mut path: *const ::core::ffi::c_char) -> bool {
    if strcmp(path, b"stdout\0" as *const u8 as *const ::core::ffi::c_char)
        == 0 as ::core::ffi::c_int
    {
        return true_0 != 0;
    }
    let mut dot: *const ::core::ffi::c_char = strrchr(path, '.' as i32);
    if dot.is_null() {
        return false_0 != 0;
    }
    return strcmp(dot, b".log\0" as *const u8 as *const ::core::ffi::c_char)
        == 0 as ::core::ffi::c_int
        || strcmp(dot, b".xdv\0" as *const u8 as *const ::core::ffi::c_char)
            == 0 as ::core::ffi::c_int
        || strcmp(dot, b".dvi\0" as *const u8 as *const ::core::ffi::c_char)
            == 0 as ::core::ffi::c_int
        || strcmp(dot, b".pdf\0" as *const u8 as *const ::core::ffi::c_char)
            == 0 as ::core::ffi::c_int
        || strcmp(
            dot,
            b".synctex\0" as *const u8 as *const ::core::ffi::c_char,
        ) == 0 as ::core::ffi::c_int;
}
unsafe extern "C" fn reset_seen_for_respawn(mut self_0: *mut tex_engine) {
    let mut e: *mut fileentry_t = ::core::ptr::null_mut::<fileentry_t>();
    let mut index: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    loop {
        e = filesystem_scan((*self_0).fs, &raw mut index);
        if e.is_null() {
            break;
        }
        (*e).seen = -(1 as ::core::ffi::c_int);
    }
}
#[no_mangle]
pub unsafe extern "C" fn txp_engine_aux_dirty(mut self_0: *mut tex_engine) -> bool {
    return (*self_0).aux_dirty;
}
#[no_mangle]
pub unsafe extern "C" fn txp_engine_is_finishing(mut self_0: *mut tex_engine) -> bool {
    return (*self_0).finishing;
}
#[no_mangle]
pub unsafe extern "C" fn txp_engine_start_finishing(mut self_0: *mut tex_engine) {
    (*self_0).finishing = true_0 != 0;
}
unsafe extern "C" fn respawn_from_scratch(mut self_0: *mut tex_engine) {
    while (*self_0).process_count > 0 as ::core::ffi::c_int {
        pop_process(self_0);
    }
    reset_seen_for_respawn(self_0);
    xdv_index_reset((*self_0).dvi);
    (*self_0).fence_pos = -(1 as ::core::ffi::c_int);
    prepare_process(self_0);
}
#[no_mangle]
pub unsafe extern "C" fn txp_engine_finish_convergence(mut self_0: *mut tex_engine) -> bool {
    if !(*self_0).finishing {
        return false_0 != 0;
    }
    if txp_engine_get_status(self_0) as ::core::ffi::c_uint
        != DOC_TERMINATED as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        return false_0 != 0;
    }
    (*self_0).finishing = false_0 != 0;
    let mut converged: bool = true_0 != 0;
    let mut e: *mut fileentry_t = ::core::ptr::null_mut::<fileentry_t>();
    let mut index: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    loop {
        e = filesystem_scan((*self_0).fs, &raw mut index);
        if e.is_null() {
            break;
        }
        if (*e).saved.level as ::core::ffi::c_uint
            != FILE_WRITE as ::core::ffi::c_int as ::core::ffi::c_uint
            || (*e).saved.data.is_null()
        {
            continue;
        }
        if is_system_output((*e).path) {
            continue;
        }
        let mut stashed_match: bool = !(*e).edit_data.is_null()
            && (*e).edit_data_from_convergence as ::core::ffi::c_int != 0
            && (*(*e).saved.data).len == (*(*e).edit_data).len
            && memcmp(
                (*(*e).saved.data).data as *const ::core::ffi::c_void,
                (*(*e).edit_data).data as *const ::core::ffi::c_void,
                (*(*e).saved.data).len,
            ) == 0 as ::core::ffi::c_int;
        if !stashed_match && (*e).edit_data.is_null() && !(*e).fs_data.is_null() {
            stashed_match = (*(*e).saved.data).len == (*(*e).fs_data).len
                && memcmp(
                    (*(*e).saved.data).data as *const ::core::ffi::c_void,
                    (*(*e).fs_data).data as *const ::core::ffi::c_void,
                    (*(*e).saved.data).len,
                ) == 0 as ::core::ffi::c_int;
        }
        if stashed_match {
            continue;
        }
        converged = false_0 != 0;
        break;
    }
    if converged {
        fprintf(
            __stderrp,
            b"[rerun] aux byte-stable, convergence reached\n\0" as *const u8
                as *const ::core::ffi::c_char,
        );
        (*self_0).aux_dirty = false_0 != 0;
        return false_0 != 0;
    }
    let mut index_0: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    loop {
        e = filesystem_scan((*self_0).fs, &raw mut index_0);
        if e.is_null() {
            break;
        }
        if (*e).saved.level as ::core::ffi::c_uint
            != FILE_WRITE as ::core::ffi::c_int as ::core::ffi::c_uint
            || (*e).saved.data.is_null()
        {
            continue;
        }
        if is_system_output((*e).path) {
            continue;
        }
        if !(*e).edit_data.is_null() && !(*e).edit_data_from_convergence {
            continue;
        }
        tbuf_drop((*e).edit_data);
        (*e).edit_data = tbuf_from_copy(
            (*(*e).saved.data).data as *const ::core::ffi::c_void,
            (*(*e).saved.data).len,
        );
        (*e).edit_data_from_convergence = true_0 != 0;
    }
    respawn_from_scratch(self_0 as *mut tex_engine);
    (*self_0).aux_dirty = false_0 != 0;
    return true_0 != 0;
}
#[no_mangle]
pub unsafe extern "C" fn txp_engine_begin_changes(mut self_0: *mut tex_engine) {
    rollback_begin(self_0 as *mut tex_engine);
}
#[no_mangle]
pub unsafe extern "C" fn txp_engine_detect_changes(mut self_0: *mut tex_engine) {
    let mut e: *mut fileentry_t = ::core::ptr::null_mut::<fileentry_t>();
    let mut index: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    loop {
        e = filesystem_scan((*self_0).fs, &raw mut index);
        if e.is_null() {
            break;
        }
        let mut changed: ::core::ffi::c_int = scan_entry(self_0 as *mut tex_engine, e);
        if changed > -(1 as ::core::ffi::c_int) {
            rollback_add_change(self_0 as *mut tex_engine, e, changed);
        }
    }
}
#[no_mangle]
pub unsafe extern "C" fn txp_engine_scan_file(
    mut self_0: *mut tex_engine,
    mut e: *mut fileentry_t,
) {
    if ((*e).saved.level as ::core::ffi::c_uint)
        < FILE_READ as ::core::ffi::c_int as ::core::ffi::c_uint
        || (*e).fs_stat.st_ino == 0 as __darwin_ino64_t
    {
        return;
    }
    let mut edit: *mut tbuf = (*e).edit_data;
    (*e).edit_data = ::core::ptr::null_mut::<tbuf>();
    scan_entry(self_0 as *mut tex_engine, e);
    (*e).edit_data = edit;
}
#[no_mangle]
pub unsafe extern "C" fn txp_engine_end_changes(mut self_0: *mut tex_engine) -> bool {
    let mut reverted: ::core::ffi::c_int = 0;
    let mut trace: ::core::ffi::c_int = 0;
    let mut offset: ::core::ffi::c_int = 0;
    if !rollback_end(
        self_0 as *mut tex_engine,
        &raw mut reverted,
        &raw mut offset,
    ) {
        return false_0 != 0;
    }
    trace = if reverted >= 0 as ::core::ffi::c_int {
        compute_fences(self_0 as *mut tex_engine, reverted, offset)
    } else {
        0 as ::core::ffi::c_int
    };
    rollback_processes(self_0 as *mut tex_engine, reverted, trace);
    return true_0 != 0;
}
#[no_mangle]
pub unsafe extern "C" fn txp_engine_get_status(mut self_0: *mut tex_engine) -> txp_engine_status {
    if (*self_0).process_count == 0 as ::core::ffi::c_int {
        return DOC_TERMINATED;
    }
    return (if (*get_process(self_0 as *mut tex_engine)).fd > -(1 as ::core::ffi::c_int) {
        DOC_RUNNING as ::core::ffi::c_int
    } else {
        DOC_TERMINATED as ::core::ffi::c_int
    }) as txp_engine_status;
}
#[no_mangle]
pub unsafe extern "C" fn txp_engine_find_file(
    mut self_0: *mut tex_engine,
    mut path: *const ::core::ffi::c_char,
) -> *mut fileentry_t {
    return filesystem_lookup_or_create((*self_0).fs, path);
}
#[no_mangle]
pub unsafe extern "C" fn txp_engine_new(
    mut engine_path: *const ::core::ffi::c_char,
    mut inclusion_path: *const ::core::ffi::c_char,
    mut tex_name: *const ::core::ffi::c_char,
) -> *mut tex_engine {
    let mut self_0: *mut tex_engine =
        calloc(1 as size_t, ::core::mem::size_of::<tex_engine>() as size_t) as *mut tex_engine;
    if self_0.is_null() {
        abort();
    }
    (*self_0).name = strdup(tex_name);
    (*self_0).engine_path = strdup(engine_path);
    (*self_0).inclusion_path = strdup(if !inclusion_path.is_null() {
        inclusion_path
    } else {
        b"\0" as *const u8 as *const ::core::ffi::c_char
    });
    state_init(&raw mut (*self_0).st);
    (*self_0).fs = filesystem_new();
    (*self_0).log = log_new();
    (*self_0).trace = ::core::ptr::null_mut::<trace_entry_t>();
    (*self_0).trace_cap = 0 as ::core::ffi::c_int;
    (*self_0).fence_pos = -(1 as ::core::ffi::c_int);
    (*self_0).restart = log_snapshot((*self_0).log);
    (*self_0).c = channel_new();
    (*self_0).process_count = 0 as ::core::ffi::c_int;
    (*self_0).dvi = xdv_index_new();
    (*self_0).rollback.trace_len = NOT_IN_TRANSACTION;
    (*self_0).last_status = -(1 as ::core::ffi::c_int);
    (*self_0).snapshot_barrier = -(1 as ::core::ffi::c_int);
    return self_0 as *mut tex_engine;
}
#[no_mangle]
pub unsafe extern "C" fn txp_engine_xdv(mut self_0: *mut tex_engine) -> *mut xdv_index {
    return (*self_0).dvi;
}
#[no_mangle]
pub unsafe extern "C" fn txp_engine_document(mut self_0: *mut tex_engine) -> *mut tbuf {
    return if !(*self_0).st.document.entry.is_null() {
        (*(*self_0).st.document.entry).saved.data
    } else {
        ::core::ptr::null_mut::<tbuf>()
    };
}
#[no_mangle]
pub unsafe extern "C" fn txp_engine_synctex(mut self_0: *mut tex_engine) -> *mut tbuf {
    return if !(*self_0).st.synctex.entry.is_null() {
        (*(*self_0).st.synctex.entry).saved.data
    } else {
        ::core::ptr::null_mut::<tbuf>()
    };
}
#[no_mangle]
pub unsafe extern "C" fn txp_engine_log(mut self_0: *mut tex_engine) -> *mut tbuf {
    return if !(*self_0).log_entry.is_null() {
        (*(*self_0).log_entry).saved.data
    } else {
        ::core::ptr::null_mut::<tbuf>()
    };
}
#[no_mangle]
pub unsafe extern "C" fn txp_engine_fs(mut self_0: *mut tex_engine) -> *mut filesystem_t {
    return (*self_0).fs;
}
#[no_mangle]
pub unsafe extern "C" fn txp_engine_fd(mut self_0: *mut tex_engine) -> ::core::ffi::c_int {
    if (*self_0).process_count == 0 as ::core::ffi::c_int {
        return -(1 as ::core::ffi::c_int);
    }
    return (*get_process(self_0 as *mut tex_engine)).fd;
}
#[no_mangle]
pub unsafe extern "C" fn txp_engine_idle_ms(mut self_0: *mut tex_engine) -> ::core::ffi::c_int {
    let mut now: timespec = timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    clock_gettime(_CLOCK_MONOTONIC, &raw mut now);
    let mut ms: ::core::ffi::c_long = (now.tv_sec as ::core::ffi::c_long
        - (*self_0).last_query.tv_sec as ::core::ffi::c_long)
        * 1000 as ::core::ffi::c_long
        + (now.tv_nsec - (*self_0).last_query.tv_nsec) / 1000000 as ::core::ffi::c_long;
    return if ms > INT_MAX as ::core::ffi::c_long {
        INT_MAX
    } else {
        ms as ::core::ffi::c_int
    };
}
#[no_mangle]
pub unsafe extern "C" fn txp_engine_kill_running(mut self_0: *mut tex_engine) {
    if (*self_0).process_count == 0 as ::core::ffi::c_int {
        return;
    }
    close_process(get_process(self_0 as *mut tex_engine));
    if (*self_0).process_count == 1 as ::core::ffi::c_int {
        reap_root(self_0 as *mut tex_engine, true_0 != 0);
    }
}
#[no_mangle]
pub unsafe extern "C" fn txp_engine_restart(mut self_0: *mut tex_engine) {
    clear_convergence_stash(self_0 as *mut tex_engine);
    respawn_from_scratch(self_0 as *mut tex_engine);
}
#[no_mangle]
pub unsafe extern "C" fn txp_engine_running_forked(mut self_0: *mut tex_engine) -> bool {
    return (*self_0).process_count > 0 as ::core::ffi::c_int
        && (*get_process(self_0 as *mut tex_engine)).forked as ::core::ffi::c_int != 0;
}
#[no_mangle]
pub unsafe extern "C" fn txp_engine_last_status(mut self_0: *mut tex_engine) -> ::core::ffi::c_int {
    return (*self_0).last_status;
}
#[no_mangle]
pub unsafe extern "C" fn txp_engine_take_barrier(mut self_0: *mut tex_engine) -> bool {
    let mut b: bool = (*self_0).barrier_pending;
    (*self_0).barrier_pending = false_0 != 0;
    return b;
}
