/* Pitex embedded preview engine — session driver.
 *
 * Long-lived helper spawned by Pitex per workspace preview session. Reads
 * JSON-line requests on stdin (buffer updates, artifact releases), drives
 * the TeXpresso-derived checkpointing XeTeX engine, converts its XDV output
 * to PDF with the Pitex writer and publishes preview artifacts atomically
 * into per-publication directories under --out. See PreviewEngine/PROTOCOL.md.
 *
 * The update/close handling follows TeXpresso's frontend (interpret_open /
 * interpret_close in src/frontend/main.c, MIT, Frédéric Bour); everything
 * else here is Pitex-authored (AGPL-3.0-or-later). */
// Translated from driver/main.c with C2Rust 0.22.1.
use ::libc;
extern "C" {
    pub type _telldir;
    pub type __sFILEX;
    pub type filesystem_s;
    pub type xdv_index;
    pub type tex_engine;
    pub type xdv2pdf;
    fn closedir(_: *mut DIR) -> ::core::ffi::c_int;
    fn opendir(_: *const ::core::ffi::c_char) -> *mut DIR;
    fn readdir(_: *mut DIR) -> *mut dirent;
    fn __error() -> *mut ::core::ffi::c_int;
    fn open(_: *const ::core::ffi::c_char, _: ::core::ffi::c_int, ...) -> ::core::ffi::c_int;
    fn poll(_: *mut pollfd, _: nfds_t, _: ::core::ffi::c_int) -> ::core::ffi::c_int;
    fn signal(
        _: ::core::ffi::c_int,
        _: Option<unsafe extern "C" fn(::core::ffi::c_int) -> ()>,
    ) -> Option<unsafe extern "C" fn(::core::ffi::c_int) -> ()>;
    fn raise(_: ::core::ffi::c_int) -> ::core::ffi::c_int;
    fn kill(_: pid_t, _: ::core::ffi::c_int) -> ::core::ffi::c_int;
    fn sigaction(
        _: ::core::ffi::c_int,
        _: *const sigaction,
        _: *mut sigaction,
    ) -> ::core::ffi::c_int;
    fn printf(_: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
    static mut __stderrp: *mut FILE;
    fn fclose(_: *mut FILE) -> ::core::ffi::c_int;
    fn fopen(
        __filename: *const ::core::ffi::c_char,
        __mode: *const ::core::ffi::c_char,
    ) -> *mut FILE;
    fn fprintf(_: *mut FILE, _: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
    fn fputc(_: ::core::ffi::c_int, _: *mut FILE) -> ::core::ffi::c_int;
    fn fwrite(
        __ptr: *const ::core::ffi::c_void,
        __size: size_t,
        __nitems: size_t,
        __stream: *mut FILE,
    ) -> ::core::ffi::c_ulong;
    fn rename(
        __old: *const ::core::ffi::c_char,
        __new: *const ::core::ffi::c_char,
    ) -> ::core::ffi::c_int;
    fn snprintf(
        __str: *mut ::core::ffi::c_char,
        __size: size_t,
        __format: *const ::core::ffi::c_char,
        ...
    ) -> ::core::ffi::c_int;
    fn malloc(__size: size_t) -> *mut ::core::ffi::c_void;
    fn calloc(__count: size_t, __size: size_t) -> *mut ::core::ffi::c_void;
    fn free(_: *mut ::core::ffi::c_void);
    fn realloc(__ptr: *mut ::core::ffi::c_void, __size: size_t) -> *mut ::core::ffi::c_void;
    fn abort() -> !;
    fn exit(_: ::core::ffi::c_int) -> !;
    fn strtol(
        __str: *const ::core::ffi::c_char,
        __endptr: *mut *mut ::core::ffi::c_char,
        __base: ::core::ffi::c_int,
    ) -> ::core::ffi::c_long;
    fn realpath(
        _: *const ::core::ffi::c_char,
        _: *mut ::core::ffi::c_char,
    ) -> *mut ::core::ffi::c_char;
    fn setenv(
        __name: *const ::core::ffi::c_char,
        __value: *const ::core::ffi::c_char,
        __overwrite: ::core::ffi::c_int,
    ) -> ::core::ffi::c_int;
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
    fn strerror(__errnum: ::core::ffi::c_int) -> *mut ::core::ffi::c_char;
    fn strlen(__s: *const ::core::ffi::c_char) -> size_t;
    fn strncat(
        __s1: *mut ::core::ffi::c_char,
        __s2: *const ::core::ffi::c_char,
        __n: size_t,
    ) -> *mut ::core::ffi::c_char;
    fn strncmp(
        __s1: *const ::core::ffi::c_char,
        __s2: *const ::core::ffi::c_char,
        __n: size_t,
    ) -> ::core::ffi::c_int;
    fn strrchr(
        __s: *const ::core::ffi::c_char,
        __c: ::core::ffi::c_int,
    ) -> *mut ::core::ffi::c_char;
    fn strstr(
        __big: *const ::core::ffi::c_char,
        __little: *const ::core::ffi::c_char,
    ) -> *mut ::core::ffi::c_char;
    fn strdup(__s1: *const ::core::ffi::c_char) -> *mut ::core::ffi::c_char;
    fn fstat(_: ::core::ffi::c_int, _: *mut stat) -> ::core::ffi::c_int;
    fn mkdir(_: *const ::core::ffi::c_char, _: mode_t) -> ::core::ffi::c_int;
    fn stat(_: *const ::core::ffi::c_char, _: *mut stat) -> ::core::ffi::c_int;
    fn clock_gettime(__clock_id: clockid_t, __tp: *mut timespec) -> ::core::ffi::c_int;
    fn _exit(_: ::core::ffi::c_int) -> !;
    fn access(_: *const ::core::ffi::c_char, _: ::core::ffi::c_int) -> ::core::ffi::c_int;
    fn chdir(_: *const ::core::ffi::c_char) -> ::core::ffi::c_int;
    fn close(_: ::core::ffi::c_int) -> ::core::ffi::c_int;
    fn dup2(_: ::core::ffi::c_int, _: ::core::ffi::c_int) -> ::core::ffi::c_int;
    fn getpgrp() -> pid_t;
    fn getpid() -> pid_t;
    fn read(_: ::core::ffi::c_int, _: *mut ::core::ffi::c_void, __nbyte: size_t) -> ssize_t;
    fn rmdir(_: *const ::core::ffi::c_char) -> ::core::ffi::c_int;
    fn unlink(_: *const ::core::ffi::c_char) -> ::core::ffi::c_int;
    fn write(
        __fd: ::core::ffi::c_int,
        __buf: *const ::core::ffi::c_void,
        __nbyte: size_t,
    ) -> ssize_t;
    fn ftruncate(_: ::core::ffi::c_int, _: off_t) -> ::core::ffi::c_int;
    fn _NSGetExecutablePath(
        buf: *mut ::core::ffi::c_char,
        bufsize: *mut uint32_t,
    ) -> ::core::ffi::c_int;
    fn tbuf_new(cap: size_t) -> *mut tbuf;
    fn tbuf_from_copy(data: *const ::core::ffi::c_void, len: size_t) -> *mut tbuf;
    fn tbuf_read_file(path: *const ::core::ffi::c_char) -> *mut tbuf;
    fn tbuf_keep(b: *mut tbuf) -> *mut tbuf;
    fn tbuf_drop(b: *mut tbuf);
    fn filesystem_lookup(
        fs: *mut filesystem_t,
        path: *const ::core::ffi::c_char,
    ) -> *mut fileentry_t;
    fn xdv_index_new() -> *mut xdv_index;
    fn xdv_index_free(x: *mut xdv_index);
    fn xdv_index_update(x: *mut xdv_index, data: *const ::core::ffi::c_uchar, len: size_t);
    fn xdv_index_page_count(x: *const xdv_index) -> ::core::ffi::c_int;
    fn txp_engine_new(
        engine_path: *const ::core::ffi::c_char,
        inclusion_path: *const ::core::ffi::c_char,
        tex_name: *const ::core::ffi::c_char,
    ) -> *mut tex_engine;
    fn txp_engine_free(self_0: *mut tex_engine);
    fn txp_engine_step(self_0: *mut tex_engine, restart_if_needed: bool) -> bool;
    fn txp_engine_begin_changes(self_0: *mut tex_engine);
    fn txp_engine_detect_changes(self_0: *mut tex_engine);
    fn txp_engine_scan_file(self_0: *mut tex_engine, e: *mut fileentry_t);
    fn txp_engine_end_changes(self_0: *mut tex_engine) -> bool;
    fn txp_engine_notify_file_changes(
        self_0: *mut tex_engine,
        entry: *mut fileentry_t,
        offset: ::core::ffi::c_int,
    );
    fn txp_engine_find_file(
        self_0: *mut tex_engine,
        path: *const ::core::ffi::c_char,
    ) -> *mut fileentry_t;
    fn txp_engine_get_status(self_0: *mut tex_engine) -> txp_engine_status;
    fn txp_engine_aux_dirty(self_0: *mut tex_engine) -> bool;
    fn txp_engine_start_finishing(self_0: *mut tex_engine);
    fn txp_engine_finish_convergence(self_0: *mut tex_engine) -> bool;
    fn txp_engine_xdv(self_0: *mut tex_engine) -> *mut xdv_index;
    fn txp_engine_document(self_0: *mut tex_engine) -> *mut tbuf;
    fn txp_engine_synctex(self_0: *mut tex_engine) -> *mut tbuf;
    fn txp_engine_log(self_0: *mut tex_engine) -> *mut tbuf;
    fn txp_engine_fs(self_0: *mut tex_engine) -> *mut filesystem_t;
    fn txp_engine_fd(self_0: *mut tex_engine) -> ::core::ffi::c_int;
    fn txp_engine_idle_ms(self_0: *mut tex_engine) -> ::core::ffi::c_int;
    fn txp_engine_kill_running(self_0: *mut tex_engine);
    fn txp_engine_restart(self_0: *mut tex_engine);
    fn txp_engine_last_status(self_0: *mut tex_engine) -> ::core::ffi::c_int;
    fn txp_engine_take_barrier(self_0: *mut tex_engine) -> bool;
    fn pbuf_printf(b: *mut pbuf, fmt: *const ::core::ffi::c_char, ...);
    fn json_parse(text: *const ::core::ffi::c_char, len: size_t) -> *mut json;
    fn json_free(v: *mut json);
    fn json_get(obj: *const json, key: *const ::core::ffi::c_char) -> *mut json;
    fn json_string(v: *const json) -> *const ::core::ffi::c_char;
    fn json_number(v: *const json, out: *mut ::core::ffi::c_double) -> bool;
    fn json_write_string(out: *mut pbuf, s: *const ::core::ffi::c_char, len: size_t);
    fn texlive_available() -> bool;
    fn texlive_file_path(
        name: *const ::core::ffi::c_char,
        record_dependency: *mut FILE,
    ) -> *const ::core::ffi::c_char;
    fn xdv2pdf_new(resolver: xdv_resolver) -> *mut xdv2pdf;
    fn xdv2pdf_write(
        w: *mut xdv2pdf,
        ranges: *const xdv_range,
        nranges: ::core::ffi::c_int,
        pdf: *mut pbuf,
        warnings: *mut pbuf,
    ) -> ::core::ffi::c_int;
}
pub type __uint8_t = u8;
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
pub type __darwin_sigset_t = __uint32_t;
pub type __darwin_uid_t = __uint32_t;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _opaque_pthread_mutex_t {
    pub __sig: ::core::ffi::c_long,
    pub __opaque: [::core::ffi::c_char; 56],
}
pub type __darwin_pthread_mutex_t = _opaque_pthread_mutex_t;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct dirent {
    pub d_ino: __uint64_t,
    pub d_seekoff: __uint64_t,
    pub d_reclen: __uint16_t,
    pub d_namlen: __uint16_t,
    pub d_type: __uint8_t,
    pub d_name: [::core::ffi::c_char; 1024],
}
pub type size_t = __darwin_size_t;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct DIR {
    pub __dd_fd: ::core::ffi::c_int,
    pub __dd_loc: size_t,
    pub __dd_size: size_t,
    pub __dd_buf: *mut ::core::ffi::c_char,
    pub __dd_len: ::core::ffi::c_int,
    pub __dd_seek: ::core::ffi::c_long,
    pub __padding: ::core::ffi::c_long,
    pub __dd_flags: ::core::ffi::c_int,
    pub __dd_lock: __darwin_pthread_mutex_t,
    pub __dd_td: *mut _telldir,
}
pub type mode_t = __darwin_mode_t;
pub type off_t = __darwin_off_t;
pub type pid_t = __darwin_pid_t;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct timespec {
    pub tv_sec: __darwin_time_t,
    pub tv_nsec: ::core::ffi::c_long,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct pollfd {
    pub fd: ::core::ffi::c_int,
    pub events: ::core::ffi::c_short,
    pub revents: ::core::ffi::c_short,
}
pub type nfds_t = ::core::ffi::c_uint;
pub type sigset_t = __darwin_sigset_t;
pub type uid_t = __darwin_uid_t;
#[derive(Copy, Clone)]
#[repr(C)]
pub union sigval {
    pub sival_int: ::core::ffi::c_int,
    pub sival_ptr: *mut ::core::ffi::c_void,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct __siginfo {
    pub si_signo: ::core::ffi::c_int,
    pub si_errno: ::core::ffi::c_int,
    pub si_code: ::core::ffi::c_int,
    pub si_pid: pid_t,
    pub si_uid: uid_t,
    pub si_status: ::core::ffi::c_int,
    pub si_addr: *mut ::core::ffi::c_void,
    pub si_value: sigval,
    pub si_band: ::core::ffi::c_long,
    pub __pad: [::core::ffi::c_ulong; 7],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union __sigaction_u {
    pub __sa_handler: Option<unsafe extern "C" fn(::core::ffi::c_int) -> ()>,
    pub __sa_sigaction: Option<
        unsafe extern "C" fn(::core::ffi::c_int, *mut __siginfo, *mut ::core::ffi::c_void) -> (),
    >,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct sigaction {
    pub __sigaction_u: __sigaction_u,
    pub sa_mask: sigset_t,
    pub sa_flags: ::core::ffi::c_int,
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
pub type ssize_t = __darwin_ssize_t;
pub type uint32_t = u32;
pub type dev_t = __darwin_dev_t;
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
pub type clockid_t = ::core::ffi::c_uint;
pub const _CLOCK_THREAD_CPUTIME_ID: clockid_t = 16;
pub const _CLOCK_PROCESS_CPUTIME_ID: clockid_t = 12;
pub const _CLOCK_UPTIME_RAW_APPROX: clockid_t = 9;
pub const _CLOCK_UPTIME_RAW: clockid_t = 8;
pub const _CLOCK_MONOTONIC_RAW_APPROX: clockid_t = 5;
pub const _CLOCK_MONOTONIC_RAW: clockid_t = 4;
pub const _CLOCK_MONOTONIC: clockid_t = 6;
pub const _CLOCK_REALTIME: clockid_t = 0;
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
pub type filesystem_t = filesystem_s;
pub type txp_engine_status = ::core::ffi::c_uint;
pub const DOC_TERMINATED: txp_engine_status = 1;
pub const DOC_RUNNING: txp_engine_status = 0;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct pbuf {
    pub data: *mut ::core::ffi::c_uchar,
    pub len: size_t,
    pub cap: size_t,
}
pub type json_type = ::core::ffi::c_uint;
pub const JSON_OBJECT: json_type = 5;
pub const JSON_ARRAY: json_type = 4;
pub const JSON_STRING: json_type = 3;
pub const JSON_NUMBER: json_type = 2;
pub const JSON_BOOL: json_type = 1;
pub const JSON_NULL: json_type = 0;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct json {
    pub type_0: json_type,
    pub b: bool,
    pub num: ::core::ffi::c_double,
    pub str_0: *mut ::core::ffi::c_char,
    pub len: size_t,
    pub items: *mut *mut json,
    pub keys: *mut *mut ::core::ffi::c_char,
    pub n: ::core::ffi::c_int,
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
pub struct C2RustUnnamed_0 {
    pub root: [::core::ffi::c_char; 1024],
    pub main_dir: [::core::ffi::c_char; 1024],
    pub job: [::core::ffi::c_char; 256],
    pub tex_name: [::core::ffi::c_char; 256],
    pub out: [::core::ffi::c_char; 1024],
    pub engine: [::core::ffi::c_char; 1024],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct res_entry {
    pub next: *mut res_entry,
    pub name: *mut ::core::ffi::c_char,
    pub kind: ::core::ffi::c_int,
    pub path: *mut ::core::ffi::c_char,
    pub st: stat,
    pub data: *mut tbuf,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct snapshot {
    pub xdv: *mut tbuf,
    pub synctex: *mut tbuf,
    pub log: *mut tbuf,
    pub index: *mut xdv_index,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct artifact {
    pub seq: ::core::ffi::c_int,
    pub used: bool,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct merged_update {
    pub valid: bool,
    pub generation: ::core::ffi::c_longlong,
    pub n: ::core::ffi::c_int,
    pub cap: ::core::ffi::c_int,
    pub paths: *mut *mut ::core::ffi::c_char,
    pub texts: *mut *mut ::core::ffi::c_char,
    pub lens: *mut size_t,
}
pub const __DARWIN_NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
pub const EINTR: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
pub const EAGAIN: ::core::ffi::c_int = 35 as ::core::ffi::c_int;
pub const O_WRONLY: ::core::ffi::c_int = 0x1 as ::core::ffi::c_int;
pub const O_APPEND: ::core::ffi::c_int = 0x8 as ::core::ffi::c_int;
pub const O_CREAT: ::core::ffi::c_int = 0x200 as ::core::ffi::c_int;
pub const O_TRUNC: ::core::ffi::c_int = 0x400 as ::core::ffi::c_int;
pub const O_CLOEXEC: ::core::ffi::c_int = 0x1000000 as ::core::ffi::c_int;
pub const S_IFMT: ::core::ffi::c_int = 0o170000 as ::core::ffi::c_int;
pub const S_IFREG: ::core::ffi::c_int = 0o100000 as ::core::ffi::c_int;
pub const PATH_MAX: ::core::ffi::c_int = 1024 as ::core::ffi::c_int;
pub const POLLIN: ::core::ffi::c_int = 0x1 as ::core::ffi::c_int;
pub const POLLERR: ::core::ffi::c_int = 0x8 as ::core::ffi::c_int;
pub const POLLHUP: ::core::ffi::c_int = 0x10 as ::core::ffi::c_int;
pub const SIGHUP: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const SIGINT: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const SIGILL: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
pub const SIGABRT: ::core::ffi::c_int = 6 as ::core::ffi::c_int;
pub const SIGFPE: ::core::ffi::c_int = 8 as ::core::ffi::c_int;
pub const SIGKILL: ::core::ffi::c_int = 9 as ::core::ffi::c_int;
pub const SIGBUS: ::core::ffi::c_int = 10 as ::core::ffi::c_int;
pub const SIGSEGV: ::core::ffi::c_int = 11 as ::core::ffi::c_int;
pub const SIGPIPE: ::core::ffi::c_int = 13 as ::core::ffi::c_int;
pub const SIGTERM: ::core::ffi::c_int = 15 as ::core::ffi::c_int;
pub const SIG_DFL: Option<unsafe extern "C" fn(::core::ffi::c_int) -> ()> = None;
pub const SA_RESETHAND: ::core::ffi::c_int = 0x4 as ::core::ffi::c_int;
pub const true_0: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const false_0: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const NULL: *mut ::core::ffi::c_void = __DARWIN_NULL;
pub const UINTPTR_MAX: ::core::ffi::c_ulong = 18446744073709551615 as ::core::ffi::c_ulong;
pub const SIZE_MAX: ::core::ffi::c_ulong = UINTPTR_MAX;
pub const X_OK: ::core::ffi::c_int = (1 as ::core::ffi::c_int) << 0 as ::core::ffi::c_int;
pub const STDIN_FILENO: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const STDOUT_FILENO: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const STDERR_FILENO: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
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
pub const MAX_RERUNS: ::core::ffi::c_int = 5 as ::core::ffi::c_int;
pub const MAX_UNRELEASED: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
pub const INTERMEDIATE_INTERVAL_MS: ::core::ffi::c_int = 350 as ::core::ffi::c_int;
pub const WATCHDOG_MS: ::core::ffi::c_int = 20000 as ::core::ffi::c_int;
pub const LOG_CAP: ::core::ffi::c_int = (8 as ::core::ffi::c_int) << 20 as ::core::ffi::c_int;
static mut cfg: C2RustUnnamed_0 = C2RustUnnamed_0 {
    root: [0; 1024],
    main_dir: [0; 1024],
    job: [0; 256],
    tex_name: [0; 256],
    out: [0; 1024],
    engine: [0; 1024],
};
static mut log_fd: ::core::ffi::c_int = -(1 as ::core::ffi::c_int);
unsafe extern "C" fn now_ms() -> ::core::ffi::c_longlong {
    let mut ts: timespec = timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    clock_gettime(_CLOCK_MONOTONIC, &raw mut ts);
    return ts.tv_sec as ::core::ffi::c_longlong * 1000 as ::core::ffi::c_longlong
        + (ts.tv_nsec / 1000000 as ::core::ffi::c_long) as ::core::ffi::c_longlong;
}
unsafe extern "C" fn emit(mut line: *mut pbuf) {
    pbuf_putc(line, '\n' as i32);
    let mut off: size_t = 0 as size_t;
    while off < (*line).len {
        let mut n: ssize_t = write(
            STDOUT_FILENO,
            (*line).data.offset(off as isize) as *const ::core::ffi::c_void,
            (*line).len.wrapping_sub(off),
        );
        if n < 0 as ssize_t {
            if *__error() == EINTR {
                continue;
            }
            _exit(0 as ::core::ffi::c_int);
        } else {
            off = off.wrapping_add(n as size_t);
        }
    }
}
unsafe extern "C" fn emit_simple(
    mut event: *const ::core::ffi::c_char,
    mut code: *const ::core::ffi::c_char,
    mut message: *const ::core::ffi::c_char,
) {
    let mut b: pbuf = pbuf {
        data: ::core::ptr::null_mut::<::core::ffi::c_uchar>(),
        len: 0,
        cap: 0,
    };
    pbuf_printf(
        &raw mut b,
        b"{\"event\":\"%s\"\0" as *const u8 as *const ::core::ffi::c_char,
        event,
    );
    if !code.is_null() {
        pbuf_printf(
            &raw mut b,
            b",\"code\":\"%s\"\0" as *const u8 as *const ::core::ffi::c_char,
            code,
        );
    }
    if !message.is_null() {
        pbuf_puts(
            &raw mut b,
            b",\"message\":\0" as *const u8 as *const ::core::ffi::c_char,
        );
        json_write_string(&raw mut b, message, strlen(message));
    }
    pbuf_putc(&raw mut b, '}' as i32);
    emit(&raw mut b);
    pbuf_free(&raw mut b);
}
static mut engine: *mut tex_engine = ::core::ptr::null::<tex_engine>() as *mut tex_engine;
static mut res_cache: *mut res_entry = ::core::ptr::null::<res_entry>() as *mut res_entry;
unsafe extern "C" fn stat_equal(mut a: *const stat, mut b: *const stat) -> bool {
    return (*a).st_ino == (*b).st_ino
        && (*a).st_dev == (*b).st_dev
        && (*a).st_size == (*b).st_size
        && (*a).st_mtimespec.tv_sec == (*b).st_mtimespec.tv_sec;
}
static mut exts_font: [*const ::core::ffi::c_char; 8] = [
    b"\0" as *const u8 as *const ::core::ffi::c_char,
    b".otf\0" as *const u8 as *const ::core::ffi::c_char,
    b".ttf\0" as *const u8 as *const ::core::ffi::c_char,
    b".ttc\0" as *const u8 as *const ::core::ffi::c_char,
    b".OTF\0" as *const u8 as *const ::core::ffi::c_char,
    b".TTF\0" as *const u8 as *const ::core::ffi::c_char,
    b".pfb\0" as *const u8 as *const ::core::ffi::c_char,
    ::core::ptr::null::<::core::ffi::c_char>(),
];
static mut exts_tfm: [*const ::core::ffi::c_char; 3] = [
    b".tfm\0" as *const u8 as *const ::core::ffi::c_char,
    b"\0" as *const u8 as *const ::core::ffi::c_char,
    ::core::ptr::null::<::core::ffi::c_char>(),
];
static mut exts_vf: [*const ::core::ffi::c_char; 2] = [
    b".vf\0" as *const u8 as *const ::core::ffi::c_char,
    ::core::ptr::null::<::core::ffi::c_char>(),
];
static mut exts_enc: [*const ::core::ffi::c_char; 3] = [
    b"\0" as *const u8 as *const ::core::ffi::c_char,
    b".enc\0" as *const u8 as *const ::core::ffi::c_char,
    ::core::ptr::null::<::core::ffi::c_char>(),
];
static mut exts_plain: [*const ::core::ffi::c_char; 2] = [
    b"\0" as *const u8 as *const ::core::ffi::c_char,
    ::core::ptr::null::<::core::ffi::c_char>(),
];
static mut exts_type1: [*const ::core::ffi::c_char; 4] = [
    b"\0" as *const u8 as *const ::core::ffi::c_char,
    b".pfb\0" as *const u8 as *const ::core::ffi::c_char,
    b".pfa\0" as *const u8 as *const ::core::ffi::c_char,
    ::core::ptr::null::<::core::ffi::c_char>(),
];
unsafe extern "C" fn find_on_disk(
    mut name: *const ::core::ffi::c_char,
    mut exts: *const *const ::core::ffi::c_char,
    mut buf: *mut ::core::ffi::c_char,
) -> *const ::core::ffi::c_char {
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
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while !(*exts.offset(i as isize)).is_null() {
        if !(snprintf(
            buf as *mut ::core::ffi::c_char,
            PATH_MAX as size_t,
            b"%s%s\0" as *const u8 as *const ::core::ffi::c_char,
            name,
            *exts.offset(i as isize),
        ) >= PATH_MAX)
        {
            if stat(buf as *const ::core::ffi::c_char, &raw mut st) == 0 as ::core::ffi::c_int
                && st.st_mode as ::core::ffi::c_int & S_IFMT == S_IFREG
            {
                return buf as *const ::core::ffi::c_char;
            }
        }
        i += 1;
    }
    if *name.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int == '/' as i32 {
        return ::core::ptr::null::<::core::ffi::c_char>();
    }
    let mut i_0: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while !(*exts.offset(i_0 as isize)).is_null() {
        let mut tmp: [::core::ffi::c_char; 1024] = [0; 1024];
        if !(snprintf(
            &raw mut tmp as *mut ::core::ffi::c_char,
            ::core::mem::size_of::<[::core::ffi::c_char; 1024]>() as size_t,
            b"%s%s\0" as *const u8 as *const ::core::ffi::c_char,
            name,
            *exts.offset(i_0 as isize),
        ) >= ::core::mem::size_of::<[::core::ffi::c_char; 1024]>() as ::core::ffi::c_int)
        {
            let mut p: *const ::core::ffi::c_char = texlive_file_path(
                &raw mut tmp as *mut ::core::ffi::c_char,
                ::core::ptr::null_mut::<FILE>(),
            );
            if !p.is_null()
                && stat(p, &raw mut st) == 0 as ::core::ffi::c_int
                && st.st_mode as ::core::ffi::c_int & S_IFMT == S_IFREG
            {
                snprintf(
                    buf as *mut ::core::ffi::c_char,
                    PATH_MAX as size_t,
                    b"%s\0" as *const u8 as *const ::core::ffi::c_char,
                    p,
                );
                return buf as *const ::core::ffi::c_char;
            }
        }
        i_0 += 1;
    }
    return ::core::ptr::null::<::core::ffi::c_char>();
}
unsafe extern "C" fn resolve(
    mut env: *mut ::core::ffi::c_void,
    mut name: *const ::core::ffi::c_char,
    mut kind: xdv_res_kind,
) -> *mut tbuf {
    if !engine.is_null()
        && (kind as ::core::ffi::c_uint == RES_IMAGE as ::core::ffi::c_int as ::core::ffi::c_uint
            || kind as ::core::ffi::c_uint
                == RES_NATIVE_FONT as ::core::ffi::c_int as ::core::ffi::c_uint)
    {
        let mut e: *mut fileentry_t = filesystem_lookup(txp_engine_fs(engine), name);
        if !e.is_null() {
            let mut d: *mut tbuf = if !(*e).edit_data.is_null() {
                (*e).edit_data
            } else {
                (*e).fs_data
            };
            if !d.is_null() {
                return tbuf_keep(d);
            }
        }
    }
    let mut exts: *const *const ::core::ffi::c_char =
        &raw const exts_plain as *const *const ::core::ffi::c_char;
    match kind as ::core::ffi::c_uint {
        0 => {
            exts = &raw const exts_font as *const *const ::core::ffi::c_char;
        }
        1 => {
            exts = &raw const exts_tfm as *const *const ::core::ffi::c_char;
        }
        2 => {
            exts = &raw const exts_vf as *const *const ::core::ffi::c_char;
        }
        3 => {
            exts = &raw const exts_enc as *const *const ::core::ffi::c_char;
        }
        5 => {
            exts = &raw const exts_type1 as *const *const ::core::ffi::c_char;
        }
        _ => {}
    }
    let mut r: *mut res_entry = ::core::ptr::null_mut::<res_entry>();
    r = res_cache;
    while !r.is_null() {
        if (*r).kind == kind as ::core::ffi::c_int
            && strcmp((*r).name, name) == 0 as ::core::ffi::c_int
        {
            break;
        }
        r = (*r).next as *mut res_entry;
    }
    if !r.is_null() && !(*r).path.is_null() {
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
        if stat((*r).path, &raw mut st) == 0 as ::core::ffi::c_int
            && stat_equal(&raw mut st, &raw mut (*r).st) as ::core::ffi::c_int != 0
        {
            return tbuf_keep((*r).data);
        }
    }
    let mut buf: [::core::ffi::c_char; 1024] = [0; 1024];
    let mut path: *const ::core::ffi::c_char =
        find_on_disk(name, exts, &raw mut buf as *mut ::core::ffi::c_char);
    let mut data: *mut tbuf = if !path.is_null() {
        tbuf_read_file(path)
    } else {
        ::core::ptr::null_mut::<tbuf>()
    };
    if r.is_null() {
        r = calloc(1 as size_t, ::core::mem::size_of::<res_entry>() as size_t) as *mut res_entry;
        if r.is_null() {
            abort();
        }
        (*r).name = strdup(name);
        (*r).kind = kind as ::core::ffi::c_int;
        (*r).next = res_cache as *mut res_entry;
        res_cache = r;
    }
    free((*r).path as *mut ::core::ffi::c_void);
    tbuf_drop((*r).data);
    (*r).path = ::core::ptr::null_mut::<::core::ffi::c_char>();
    (*r).data = ::core::ptr::null_mut::<tbuf>();
    if !data.is_null() {
        (*r).path = strdup(path);
        stat(path, &raw mut (*r).st);
        (*r).data = data;
        return tbuf_keep(data);
    }
    return ::core::ptr::null_mut::<tbuf>();
}
unsafe extern "C" fn snapshot_clear(mut s: *mut snapshot) {
    tbuf_drop((*s).xdv);
    tbuf_drop((*s).synctex);
    tbuf_drop((*s).log);
    xdv_index_free((*s).index);
    memset(
        s as *mut ::core::ffi::c_void,
        0 as ::core::ffi::c_int,
        ::core::mem::size_of::<snapshot>() as size_t,
    );
}
unsafe extern "C" fn copy_or_empty(mut b: *mut tbuf) -> *mut tbuf {
    return if !b.is_null() {
        tbuf_from_copy((*b).data as *const ::core::ffi::c_void, (*b).len)
    } else {
        tbuf_new(16 as size_t)
    };
}
unsafe extern "C" fn snapshot_take(mut s: *mut snapshot) {
    snapshot_clear(s);
    (*s).xdv = copy_or_empty(txp_engine_document(engine));
    (*s).synctex = copy_or_empty(txp_engine_synctex(engine));
    (*s).log = copy_or_empty(txp_engine_log(engine));
    if !AUX_WARNINGS.is_empty() {
        let mut bytes = std::slice::from_raw_parts((*(*s).log).data, (*(*s).log).len).to_vec();
        for warning in &AUX_WARNINGS { bytes.extend_from_slice(format!("\nPitex embedded auxiliary warning: {warning}\n").as_bytes()); }
        tbuf_drop((*s).log);
        (*s).log = tbuf_from_copy(bytes.as_ptr().cast(), bytes.len());
    }
    (*s).index = xdv_index_new();
    xdv_index_update((*s).index, (*(*s).xdv).data, (*(*s).xdv).len);
}
static mut last_complete: snapshot = snapshot {
    xdv: ::core::ptr::null::<tbuf>() as *mut tbuf,
    synctex: ::core::ptr::null::<tbuf>() as *mut tbuf,
    log: ::core::ptr::null::<tbuf>() as *mut tbuf,
    index: ::core::ptr::null::<xdv_index>() as *mut xdv_index,
};
static mut pending: snapshot = snapshot {
    xdv: ::core::ptr::null::<tbuf>() as *mut tbuf,
    synctex: ::core::ptr::null::<tbuf>() as *mut tbuf,
    log: ::core::ptr::null::<tbuf>() as *mut tbuf,
    index: ::core::ptr::null::<xdv_index>() as *mut xdv_index,
};
static mut pending_valid: bool = false;
static mut pending_complete: bool = false;
static mut pending_generation: ::core::ffi::c_longlong = 0;
static mut writer: *mut xdv2pdf = ::core::ptr::null::<xdv2pdf>() as *mut xdv2pdf;
static mut publish_seq: ::core::ffi::c_int = 0;
static mut current_seq: ::core::ffi::c_int = 0;
static mut generation: ::core::ffi::c_longlong = 0;
static mut generation_start_ms: ::core::ffi::c_longlong = 0;
static mut last_publish_ms: ::core::ffi::c_longlong = 0;
static mut published_pages_this_pass: ::core::ffi::c_int = 0;
static mut rerun_count: ::core::ffi::c_int = 0;
static mut watchdog_killed: bool = false;
static mut crash_retried: bool = false;
static mut was_running: bool = false;
static mut artifacts: [artifact; 4] = [artifact {
    seq: 0,
    used: false,
}; 4];
unsafe extern "C" fn unreleased_count() -> ::core::ffi::c_int {
    let mut n: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < MAX_UNRELEASED {
        n += artifacts[i as usize].used as ::core::ffi::c_int;
        i += 1;
    }
    return n;
}
unsafe extern "C" fn contains(mut b: *const tbuf, mut needle: *const ::core::ffi::c_char) -> bool {
    if b.is_null() {
        return false_0 != 0;
    }
    let mut n: size_t = strlen(needle);
    let mut i: size_t = 0 as size_t;
    while i.wrapping_add(n) <= (*b).len {
        if *(*b).data.offset(i as isize) as ::core::ffi::c_int
            == *needle.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_uchar
                as ::core::ffi::c_int
            && memcmp(
                (*b).data.offset(i as isize) as *const ::core::ffi::c_void,
                needle as *const ::core::ffi::c_void,
                n,
            ) == 0 as ::core::ffi::c_int
        {
            return true_0 != 0;
        }
        i = i.wrapping_add(1);
    }
    return false_0 != 0;
}
unsafe extern "C" fn log_errors(
    mut log: *const tbuf,
    mut first: *mut ::core::ffi::c_char,
    mut cap: size_t,
) -> ::core::ffi::c_int {
    let mut count: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    *first.offset(0 as ::core::ffi::c_int as isize) = 0 as ::core::ffi::c_char;
    if log.is_null() {
        return 0 as ::core::ffi::c_int;
    }
    let mut s: *const ::core::ffi::c_char = (*log).data as *const ::core::ffi::c_char;
    let mut end: *const ::core::ffi::c_char = s.offset((*log).len as isize);
    let mut line: *const ::core::ffi::c_char = s;
    while line < end {
        let mut nl: *const ::core::ffi::c_char = memchr(
            line as *const ::core::ffi::c_void,
            '\n' as i32,
            end.offset_from(line) as ::core::ffi::c_long as size_t,
        ) as *const ::core::ffi::c_char;
        if nl.is_null() {
            nl = end;
        }
        if nl.offset_from(line) as ::core::ffi::c_long >= 2 as ::core::ffi::c_long
            && *line.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int == '!' as i32
            && *line.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int == ' ' as i32
        {
            if count == 0 as ::core::ffi::c_int {
                let mut n: ::core::ffi::c_int = (nl.offset_from(line) as ::core::ffi::c_long
                    - 2 as ::core::ffi::c_long)
                    as ::core::ffi::c_int;
                if n > cap as ::core::ffi::c_int - 32 as ::core::ffi::c_int {
                    n = cap as ::core::ffi::c_int - 32 as ::core::ffi::c_int;
                }
                snprintf(
                    first,
                    cap,
                    b"%.*s\0" as *const u8 as *const ::core::ffi::c_char,
                    n,
                    line.offset(2 as ::core::ffi::c_int as isize),
                );
                let mut q: *const ::core::ffi::c_char = nl;
                let mut k: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
                while k < 12 as ::core::ffi::c_int && q < end {
                    let mut qn: *const ::core::ffi::c_char = memchr(
                        q.offset(1 as ::core::ffi::c_int as isize) as *const ::core::ffi::c_void,
                        '\n' as i32,
                        (end.offset_from(q) as ::core::ffi::c_long - 1 as ::core::ffi::c_long)
                            as size_t,
                    )
                        as *const ::core::ffi::c_char;
                    if qn.is_null() {
                        qn = end;
                    }
                    let mut ql: *const ::core::ffi::c_char =
                        q.offset(1 as ::core::ffi::c_int as isize);
                    if qn.offset_from(ql) as ::core::ffi::c_long > 2 as ::core::ffi::c_long
                        && *ql.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                            == 'l' as i32
                        && *ql.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                            == '.' as i32
                        && *ql.offset(2 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                            >= '0' as i32
                        && *ql.offset(2 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                            <= '9' as i32
                    {
                        let mut used: size_t = strlen(first);
                        snprintf(
                            first.offset(used as isize),
                            cap.wrapping_sub(used),
                            b" (line %ld)\0" as *const u8 as *const ::core::ffi::c_char,
                            strtol(
                                ql.offset(2 as ::core::ffi::c_int as isize),
                                ::core::ptr::null_mut::<*mut ::core::ffi::c_char>(),
                                10 as ::core::ffi::c_int,
                            ),
                        );
                        break;
                    } else {
                        q = qn;
                        k += 1;
                    }
                }
            }
            count += 1;
        }
        line = nl.offset(1 as ::core::ffi::c_int as isize);
    }
    return count;
}
unsafe extern "C" fn write_synctex(
    mut f: *mut FILE,
    mut sx: *const tbuf,
    mut pages: ::core::ffi::c_int,
) {
    if sx.is_null() || (*sx).len == 0 as size_t {
        return;
    }
    let mut s: *const ::core::ffi::c_char = (*sx).data as *const ::core::ffi::c_char;
    let mut end: *const ::core::ffi::c_char = s.offset((*sx).len as isize);
    let mut cut: *const ::core::ffi::c_char = end;
    let mut sheet_at: *const ::core::ffi::c_char = ::core::ptr::null::<::core::ffi::c_char>();
    let mut post_at: *const ::core::ffi::c_char = ::core::ptr::null::<::core::ffi::c_char>();
    let mut sheets_done: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut line: *const ::core::ffi::c_char = s;
    while line < end {
        let mut nl: *const ::core::ffi::c_char = memchr(
            line as *const ::core::ffi::c_void,
            '\n' as i32,
            end.offset_from(line) as ::core::ffi::c_long as size_t,
        ) as *const ::core::ffi::c_char;
        if nl.is_null() {
            cut = if !sheet_at.is_null() && sheet_at < line {
                sheet_at
            } else {
                line
            };
            break;
        } else {
            let mut n: size_t = nl.offset_from(line) as ::core::ffi::c_long as size_t;
            if n >= 10 as size_t
                && memcmp(
                    line as *const ::core::ffi::c_void,
                    b"Postamble:\0" as *const u8 as *const ::core::ffi::c_char
                        as *const ::core::ffi::c_void,
                    10 as size_t,
                ) == 0 as ::core::ffi::c_int
            {
                post_at = line;
            }
            if n > 1 as size_t
                && *line.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                    == '{' as i32
            {
                sheet_at = line;
            } else if n > 0 as size_t
                && *line.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                    == '}' as i32
            {
                let mut num: ::core::ffi::c_uint = 0 as ::core::ffi::c_uint;
                let mut numok: bool = false_0 != 0;
                let mut i: size_t = 1 as size_t;
                while i < n
                    && *line.offset(i as isize) as ::core::ffi::c_int >= '0' as i32
                    && *line.offset(i as isize) as ::core::ffi::c_int <= '9' as i32
                {
                    num = num.wrapping_mul(10 as ::core::ffi::c_uint).wrapping_add(
                        (*line.offset(i as isize) as ::core::ffi::c_int - '0' as i32)
                            as ::core::ffi::c_uint,
                    );
                    numok = true_0 != 0;
                    i = i.wrapping_add(1);
                }
                if numok as ::core::ffi::c_int != 0 && num as ::core::ffi::c_int <= pages {
                    sheets_done += 1;
                    sheet_at = ::core::ptr::null::<::core::ffi::c_char>();
                } else {
                    cut = if !sheet_at.is_null() && sheet_at < line {
                        sheet_at
                    } else {
                        line
                    };
                    break;
                }
            }
            line = nl.offset(1 as ::core::ffi::c_int as isize);
        }
    }
    if !sheet_at.is_null() && sheet_at < cut {
        cut = sheet_at;
    }
    let post_emitted: bool = !post_at.is_null() && post_at < cut;
    let mut line_0: *const ::core::ffi::c_char = s;
    while line_0 < cut {
        let mut nl_0: *const ::core::ffi::c_char = memchr(
            line_0 as *const ::core::ffi::c_void,
            '\n' as i32,
            cut.offset_from(line_0) as ::core::ffi::c_long as size_t,
        ) as *const ::core::ffi::c_char;
        let mut n_0: size_t = (if !nl_0.is_null() { nl_0 } else { cut }).offset_from(line_0)
            as ::core::ffi::c_long as size_t;
        if n_0 > 6 as size_t
            && memcmp(
                line_0 as *const ::core::ffi::c_void,
                b"Input:\0" as *const u8 as *const ::core::ffi::c_char
                    as *const ::core::ffi::c_void,
                6 as size_t,
            ) == 0 as ::core::ffi::c_int
        {
            let mut colon: *const ::core::ffi::c_char = memchr(
                line_0.offset(6 as ::core::ffi::c_int as isize) as *const ::core::ffi::c_void,
                ':' as i32,
                n_0.wrapping_sub(6 as size_t),
            ) as *const ::core::ffi::c_char;
            if !colon.is_null()
                && colon.offset(1 as ::core::ffi::c_int as isize) < line_0.offset(n_0 as isize)
                && *colon.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                    != '/' as i32
            {
                let mut path: *const ::core::ffi::c_char =
                    colon.offset(1 as ::core::ffi::c_int as isize);
                if path.offset(2 as ::core::ffi::c_int as isize) <= line_0.offset(n_0 as isize)
                    && *path.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                        == '.' as i32
                    && *path.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                        == '/' as i32
                {
                    path = path.offset(2 as ::core::ffi::c_int as isize);
                }
                fwrite(
                    line_0 as *const ::core::ffi::c_void,
                    1 as size_t,
                    colon
                        .offset(1 as ::core::ffi::c_int as isize)
                        .offset_from(line_0) as ::core::ffi::c_long as size_t,
                    f,
                );
                fprintf(
                    f,
                    b"%s/%.*s\n\0" as *const u8 as *const ::core::ffi::c_char,
                    &raw mut cfg.main_dir as *mut ::core::ffi::c_char,
                    line_0.offset(n_0 as isize).offset_from(path) as ::core::ffi::c_long
                        as ::core::ffi::c_int,
                    path,
                );
                line_0 = line_0.offset(n_0.wrapping_add(1 as size_t) as isize);
                continue;
            }
        }
        fwrite(line_0 as *const ::core::ffi::c_void, 1 as size_t, n_0, f);
        fputc('\n' as i32, f);
        line_0 = line_0.offset(n_0.wrapping_add(1 as size_t) as isize);
    }
    if !post_emitted {
        fprintf(
            f,
            b"Postamble:\nCount:%d\nPost scriptum:\n\0" as *const u8 as *const ::core::ffi::c_char,
            sheets_done,
        );
    }
}
unsafe extern "C" fn write_file(
    mut dir: *const ::core::ffi::c_char,
    mut name: *const ::core::ffi::c_char,
    mut data: *const ::core::ffi::c_void,
    mut len: size_t,
) -> bool {
    let mut path: [::core::ffi::c_char; 1024] = [0; 1024];
    snprintf(
        &raw mut path as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 1024]>() as size_t,
        b"%s/%s\0" as *const u8 as *const ::core::ffi::c_char,
        dir,
        name,
    );
    let mut fd: ::core::ffi::c_int = open(
        &raw mut path as *mut ::core::ffi::c_char,
        O_WRONLY | O_CREAT | O_TRUNC | O_CLOEXEC,
        0o600 as ::core::ffi::c_int,
    );
    if fd < 0 as ::core::ffi::c_int {
        return false_0 != 0;
    }
    let mut p: *const ::core::ffi::c_uchar = data as *const ::core::ffi::c_uchar;
    while len > 0 as size_t {
        let mut n: ssize_t = write(fd, p as *const ::core::ffi::c_void, len);
        if n < 0 as ssize_t {
            if *__error() == EINTR {
                continue;
            }
            close(fd);
            return false_0 != 0;
        } else {
            p = p.offset(n as isize);
            len = len.wrapping_sub(n as size_t);
        }
    }
    return close(fd) == 0 as ::core::ffi::c_int;
}
unsafe extern "C" fn remove_dir(mut dir: *const ::core::ffi::c_char) {
    let mut d: *mut DIR = opendir(dir);
    if !d.is_null() {
        let mut de: *mut dirent = ::core::ptr::null_mut::<dirent>();
        loop {
            de = readdir(d);
            if de.is_null() {
                break;
            }
            if strcmp(
                &raw mut (*de).d_name as *mut ::core::ffi::c_char,
                b".\0" as *const u8 as *const ::core::ffi::c_char,
            ) == 0
                || strcmp(
                    &raw mut (*de).d_name as *mut ::core::ffi::c_char,
                    b"..\0" as *const u8 as *const ::core::ffi::c_char,
                ) == 0
            {
                continue;
            }
            let mut p: [::core::ffi::c_char; 1024] = [0; 1024];
            snprintf(
                &raw mut p as *mut ::core::ffi::c_char,
                ::core::mem::size_of::<[::core::ffi::c_char; 1024]>() as size_t,
                b"%s/%s\0" as *const u8 as *const ::core::ffi::c_char,
                dir,
                &raw mut (*de).d_name as *mut ::core::ffi::c_char,
            );
            unlink(&raw mut p as *mut ::core::ffi::c_char);
        }
        closedir(d);
    }
    rmdir(dir);
}
unsafe extern "C" fn cap_log_file() {
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
    if log_fd >= 0 as ::core::ffi::c_int
        && fstat(log_fd, &raw mut st) == 0 as ::core::ffi::c_int
        && st.st_size > LOG_CAP as off_t
    {
        ftruncate(log_fd, 0 as off_t) != 0 as ::core::ffi::c_int;
    }
}
unsafe extern "C" fn publish(
    mut cur: *const snapshot,
    mut complete: bool,
    mut gen: ::core::ffi::c_longlong,
) -> bool {
    if unreleased_count() >= MAX_UNRELEASED {
        return false_0 != 0;
    }
    let mut slot: ::core::ffi::c_int = -(1 as ::core::ffi::c_int);
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < MAX_UNRELEASED {
        if !artifacts[i as usize].used {
            slot = i;
            break;
        } else {
            i += 1;
        }
    }
    let mut t0: ::core::ffi::c_longlong = now_ms();
    let mut ncur: ::core::ffi::c_int = xdv_index_page_count((*cur).index);
    let mut ranges: [xdv_range; 2] = [xdv_range {
        data: ::core::ptr::null::<::core::ffi::c_uchar>(),
        len: 0,
        index: ::core::ptr::null::<xdv_index>(),
        first: 0,
        count: 0,
    }; 2];
    let mut nranges: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    if ncur > 0 as ::core::ffi::c_int {
        let fresh1 = nranges;
        nranges = nranges + 1;
        ranges[fresh1 as usize] = xdv_range {
            data: (*(*cur).xdv).data,
            len: (*(*cur).xdv).len,
            index: (*cur).index,
            first: 0 as ::core::ffi::c_int,
            count: ncur,
        };
    }
    let mut total: ::core::ffi::c_int = ncur;
    if !complete
        && !last_complete.index.is_null()
        && cur != &raw mut last_complete as *const snapshot
    {
        let mut nold: ::core::ffi::c_int = xdv_index_page_count(last_complete.index);
        if nold > ncur {
            let fresh2 = nranges;
            nranges = nranges + 1;
            ranges[fresh2 as usize] = xdv_range {
                data: (*last_complete.xdv).data,
                len: (*last_complete.xdv).len,
                index: last_complete.index,
                first: ncur,
                count: nold - ncur,
            };
            total = nold;
        }
    }
    if total == 0 as ::core::ffi::c_int {
        return true_0 != 0;
    }
    let with_synctex: bool =
        total == ncur && !(*cur).synctex.is_null() && (*(*cur).synctex).len != 0;
    let mut pdf: pbuf = pbuf {
        data: ::core::ptr::null_mut::<::core::ffi::c_uchar>(),
        len: 0,
        cap: 0,
    };
    let mut warnings: pbuf = pbuf {
        data: ::core::ptr::null_mut::<::core::ffi::c_uchar>(),
        len: 0,
        cap: 0,
    };
    let mut err: ::core::ffi::c_int = xdv2pdf_write(
        writer,
        &raw mut ranges as *mut xdv_range,
        nranges,
        &raw mut pdf,
        &raw mut warnings,
    );
    if warnings.len != 0 {
        fprintf(
            __stderrp,
            b"[pdf] warnings:\n%.*s\0" as *const u8 as *const ::core::ffi::c_char,
            warnings.len as ::core::ffi::c_int,
            warnings.data as *mut ::core::ffi::c_char,
        );
    }
    if err == 2 {
        let mut event = pbuf { data: std::ptr::null_mut(), len: 0, cap: 0 };
        pbuf_printf(&raw mut event, b"{\"event\":\"draft\",\"generation\":%lld}\0".as_ptr().cast(), gen);
        emit(&raw mut event);
        pbuf_free(&raw mut event);
        pbuf_free(&raw mut pdf);
        pbuf_free(&raw mut warnings);
        return true;
    }
    if err != 0 {
        let mut msg: [::core::ffi::c_char; 256] = [0; 256];
        snprintf(
            &raw mut msg as *mut ::core::ffi::c_char,
            ::core::mem::size_of::<[::core::ffi::c_char; 256]>() as size_t,
            b"PDF conversion failed (%d)\0" as *const u8 as *const ::core::ffi::c_char,
            err,
        );
        emit_simple(
            b"error\0" as *const u8 as *const ::core::ffi::c_char,
            b"pdf\0" as *const u8 as *const ::core::ffi::c_char,
            &raw mut msg as *mut ::core::ffi::c_char,
        );
        pbuf_free(&raw mut pdf);
        pbuf_free(&raw mut warnings);
        return true_0 != 0;
    }
    publish_seq += 1;
    let mut seq: ::core::ffi::c_int = publish_seq;
    let mut tmp: [::core::ffi::c_char; 1024] = [0; 1024];
    let mut dir: [::core::ffi::c_char; 1024] = [0; 1024];
    let mut name: [::core::ffi::c_char; 300] = [0; 300];
    snprintf(
        &raw mut tmp as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 1024]>() as size_t,
        b"%s/.p%d.tmp\0" as *const u8 as *const ::core::ffi::c_char,
        &raw mut cfg.out as *mut ::core::ffi::c_char,
        seq,
    );
    snprintf(
        &raw mut dir as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 1024]>() as size_t,
        b"%s/p%d\0" as *const u8 as *const ::core::ffi::c_char,
        &raw mut cfg.out as *mut ::core::ffi::c_char,
        seq,
    );
    let mut ok: bool =
        mkdir(&raw mut tmp as *mut ::core::ffi::c_char, 0o700 as mode_t) == 0 as ::core::ffi::c_int;
    snprintf(
        &raw mut name as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 300]>() as size_t,
        b"%s.pdf\0" as *const u8 as *const ::core::ffi::c_char,
        &raw mut cfg.job as *mut ::core::ffi::c_char,
    );
    ok = ok as ::core::ffi::c_int != 0
        && write_file(
            &raw mut tmp as *mut ::core::ffi::c_char,
            &raw mut name as *mut ::core::ffi::c_char,
            pdf.data as *const ::core::ffi::c_void,
            pdf.len,
        ) as ::core::ffi::c_int
            != 0;
    if ok as ::core::ffi::c_int != 0 && with_synctex as ::core::ffi::c_int != 0 {
        let mut p: [::core::ffi::c_char; 1024] = [0; 1024];
        snprintf(
            &raw mut p as *mut ::core::ffi::c_char,
            ::core::mem::size_of::<[::core::ffi::c_char; 1024]>() as size_t,
            b"%s/%s.synctex\0" as *const u8 as *const ::core::ffi::c_char,
            &raw mut tmp as *mut ::core::ffi::c_char,
            &raw mut cfg.job as *mut ::core::ffi::c_char,
        );
        let mut f: *mut FILE = fopen(
            &raw mut p as *mut ::core::ffi::c_char,
            b"wb\0" as *const u8 as *const ::core::ffi::c_char,
        ) as *mut FILE;
        ok = !f.is_null();
        if !f.is_null() {
            write_synctex(f, (*cur).synctex, ncur);
            ok = fclose(f) == 0 as ::core::ffi::c_int;
        }
    }
    snprintf(
        &raw mut name as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 300]>() as size_t,
        b"%s.log\0" as *const u8 as *const ::core::ffi::c_char,
        &raw mut cfg.job as *mut ::core::ffi::c_char,
    );
    ok = ok as ::core::ffi::c_int != 0
        && write_file(
            &raw mut tmp as *mut ::core::ffi::c_char,
            &raw mut name as *mut ::core::ffi::c_char,
            (if !(*cur).log.is_null() {
                (*(*cur).log).data as *const ::core::ffi::c_uchar
            } else {
                b"\0" as *const u8 as *const ::core::ffi::c_char as *const ::core::ffi::c_uchar
            }) as *const ::core::ffi::c_void,
            (if !(*cur).log.is_null() {
                (*(*cur).log).len
            } else {
                0 as size_t
            }),
        ) as ::core::ffi::c_int
            != 0;
    ok = ok as ::core::ffi::c_int != 0
        && rename(
            &raw mut tmp as *mut ::core::ffi::c_char,
            &raw mut dir as *mut ::core::ffi::c_char,
        ) == 0 as ::core::ffi::c_int;
    pbuf_free(&raw mut pdf);
    if !ok {
        remove_dir(&raw mut tmp as *mut ::core::ffi::c_char);
        emit_simple(
            b"error\0" as *const u8 as *const ::core::ffi::c_char,
            b"io\0" as *const u8 as *const ::core::ffi::c_char,
            strerror(*__error()),
        );
        pbuf_free(&raw mut warnings);
        return true_0 != 0;
    }
    artifacts[slot as usize].used = true_0 != 0;
    artifacts[slot as usize].seq = seq;
    let mut first: [::core::ffi::c_char; 512] = [0; 512];
    let mut errors: ::core::ffi::c_int = log_errors(
        (*cur).log,
        &raw mut first as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 512]>() as size_t,
    );
    let mut ev: pbuf = pbuf {
        data: ::core::ptr::null_mut::<::core::ffi::c_uchar>(),
        len: 0,
        cap: 0,
    };
    pbuf_printf(
        &raw mut ev,
        b"{\"event\":\"published\",\"seq\":%d,\"generation\":%lld,\"complete\":%s,\"pages\":%d,\"current_pages\":%d,\"errors\":%d,\"elapsed_ms\":%lld,\"pdf_ms\":%lld,\"dir\":\0"
            as *const u8 as *const ::core::ffi::c_char,
        seq,
        gen,
        if complete as ::core::ffi::c_int != 0 {
            b"true\0" as *const u8 as *const ::core::ffi::c_char
        } else {
            b"false\0" as *const u8 as *const ::core::ffi::c_char
        },
        total,
        ncur,
        errors,
        now_ms() - generation_start_ms,
        now_ms() - t0,
    );
    json_write_string(
        &raw mut ev,
        &raw mut dir as *mut ::core::ffi::c_char,
        strlen(&raw mut dir as *mut ::core::ffi::c_char),
    );
    let mut p_0: [::core::ffi::c_char; 1024] = [0; 1024];
    snprintf(
        &raw mut p_0 as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 1024]>() as size_t,
        b"%s/%s.pdf\0" as *const u8 as *const ::core::ffi::c_char,
        &raw mut dir as *mut ::core::ffi::c_char,
        &raw mut cfg.job as *mut ::core::ffi::c_char,
    );
    pbuf_puts(
        &raw mut ev,
        b",\"pdf\":\0" as *const u8 as *const ::core::ffi::c_char,
    );
    json_write_string(
        &raw mut ev,
        &raw mut p_0 as *mut ::core::ffi::c_char,
        strlen(&raw mut p_0 as *mut ::core::ffi::c_char),
    );
    pbuf_printf(
        &raw mut ev,
        b",\"coherent\":%s\0" as *const u8 as *const ::core::ffi::c_char,
        if total == ncur {
            b"true\0" as *const u8 as *const ::core::ffi::c_char
        } else {
            b"false\0" as *const u8 as *const ::core::ffi::c_char
        },
    );
    if with_synctex {
        snprintf(
            &raw mut p_0 as *mut ::core::ffi::c_char,
            ::core::mem::size_of::<[::core::ffi::c_char; 1024]>() as size_t,
            b"%s/%s.synctex\0" as *const u8 as *const ::core::ffi::c_char,
            &raw mut dir as *mut ::core::ffi::c_char,
            &raw mut cfg.job as *mut ::core::ffi::c_char,
        );
        pbuf_puts(
            &raw mut ev,
            b",\"synctex\":\0" as *const u8 as *const ::core::ffi::c_char,
        );
        json_write_string(
            &raw mut ev,
            &raw mut p_0 as *mut ::core::ffi::c_char,
            strlen(&raw mut p_0 as *mut ::core::ffi::c_char),
        );
    }
    snprintf(
        &raw mut p_0 as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 1024]>() as size_t,
        b"%s/%s.log\0" as *const u8 as *const ::core::ffi::c_char,
        &raw mut dir as *mut ::core::ffi::c_char,
        &raw mut cfg.job as *mut ::core::ffi::c_char,
    );
    pbuf_puts(
        &raw mut ev,
        b",\"log\":\0" as *const u8 as *const ::core::ffi::c_char,
    );
    json_write_string(
        &raw mut ev,
        &raw mut p_0 as *mut ::core::ffi::c_char,
        strlen(&raw mut p_0 as *mut ::core::ffi::c_char),
    );
    if errors != 0 {
        pbuf_puts(
            &raw mut ev,
            b",\"first_error\":\0" as *const u8 as *const ::core::ffi::c_char,
        );
        json_write_string(
            &raw mut ev,
            &raw mut first as *mut ::core::ffi::c_char,
            strlen(&raw mut first as *mut ::core::ffi::c_char),
        );
    }
    // Warnings belong to the published snapshot, including deferred older
    // generations; do not attach the current global service state to it.
    if !(*cur).log.is_null() {
        let text = String::from_utf8_lossy(std::slice::from_raw_parts((*(*cur).log).data, (*(*cur).log).len));
        for line in text.lines().filter(|line|line.starts_with("Pitex embedded auxiliary warning: ")) {
            if let Ok(warning) = std::ffi::CString::new(line.replace('\0', "\u{fffd}")) {
                pbuf_printf(&raw mut warnings, b"%s\n\0".as_ptr().cast(), warning.as_ptr());
            }
        }
    }
    if warnings.len != 0 {
        pbuf_puts(
            &raw mut ev,
            b",\"warnings\":\0" as *const u8 as *const ::core::ffi::c_char,
        );
        let text=crate::preview_diagnostics::excerpt(std::slice::from_raw_parts(warnings.data,warnings.len));
        json_write_string(&raw mut ev,text.as_ptr().cast(),text.len());
    }
    pbuf_putc(&raw mut ev, '}' as i32);
    emit(&raw mut ev);
    pbuf_free(&raw mut ev);
    pbuf_free(&raw mut warnings);
    last_publish_ms = now_ms();
    current_seq = if complete as ::core::ffi::c_int != 0 {
        seq
    } else {
        0 as ::core::ffi::c_int
    };
    cap_log_file();
    return true_0 != 0;
}
unsafe extern "C" fn release(mut seq: ::core::ffi::c_int) {
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < MAX_UNRELEASED {
        if artifacts[i as usize].used as ::core::ffi::c_int != 0 && artifacts[i as usize].seq == seq
        {
            let mut dir: [::core::ffi::c_char; 1024] = [0; 1024];
            snprintf(
                &raw mut dir as *mut ::core::ffi::c_char,
                ::core::mem::size_of::<[::core::ffi::c_char; 1024]>() as size_t,
                b"%s/p%d\0" as *const u8 as *const ::core::ffi::c_char,
                &raw mut cfg.out as *mut ::core::ffi::c_char,
                seq,
            );
            remove_dir(&raw mut dir as *mut ::core::ffi::c_char);
            artifacts[i as usize].used = false_0 != 0;
        }
        i += 1;
    }
    if pending_valid as ::core::ffi::c_int != 0
        && publish(&raw mut pending, pending_complete, pending_generation) as ::core::ffi::c_int
            != 0
    {
        pending_valid = false_0 != 0;
        snapshot_clear(&raw mut pending);
    }
}
unsafe extern "C" fn emit_failed(
    mut log: *const tbuf,
    mut code: *const ::core::ffi::c_char,
    mut fallback: *const ::core::ffi::c_char,
) {
    let mut first: [::core::ffi::c_char; 512] = [0; 512];
    let mut errors: ::core::ffi::c_int = log_errors(
        log,
        &raw mut first as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 512]>() as size_t,
    );
    current_seq = 0 as ::core::ffi::c_int;
    let mut b: pbuf = pbuf {
        data: ::core::ptr::null_mut::<::core::ffi::c_uchar>(),
        len: 0,
        cap: 0,
    };
    pbuf_printf(
        &raw mut b,
        b"{\"event\":\"failed\",\"generation\":%lld,\"code\":\"%s\",\"errors\":%d,\"message\":\0"
            as *const u8 as *const ::core::ffi::c_char,
        generation,
        code,
        errors,
    );
    let mut m: *const ::core::ffi::c_char = if errors != 0 {
        &raw mut first as *mut ::core::ffi::c_char as *const ::core::ffi::c_char
    } else {
        fallback
    };
    json_write_string(&raw mut b, m, strlen(m));
    if !log.is_null() && (*log).len != 0 {
        let mut n: size_t = if (*log).len > 8192 as size_t {
            8192 as size_t
        } else {
            (*log).len
        };
        pbuf_puts(
            &raw mut b,
            b",\"log_tail\":\0" as *const u8 as *const ::core::ffi::c_char,
        );
        json_write_string(
            &raw mut b,
            ((*log).data as *const ::core::ffi::c_char)
                .offset((*log).len as isize)
                .offset(-(n as isize)),
            n,
        );
    }
    pbuf_putc(&raw mut b, '}' as i32);
    emit(&raw mut b);
    pbuf_free(&raw mut b);
}
unsafe extern "C" fn find_diff(
    mut buf: *const tbuf,
    mut data: *const ::core::ffi::c_void,
    mut size: size_t,
) -> ::core::ffi::c_int {
    let mut ptr: *const ::core::ffi::c_uchar = data as *const ::core::ffi::c_uchar;
    let mut len: size_t = if (*buf).len < size { (*buf).len } else { size };
    let mut i: size_t = 0;
    i = 0 as size_t;
    while i < len
        && *(*buf).data.offset(i as isize) as ::core::ffi::c_int
            == *ptr.offset(i as isize) as ::core::ffi::c_int
    {
        i = i.wrapping_add(1);
    }
    return i as ::core::ffi::c_int;
}
unsafe extern "C" fn vfs_key(
    mut abs: *const ::core::ffi::c_char,
    mut out: *mut ::core::ffi::c_char,
) -> bool {
    let mut rl: size_t = strlen(&raw mut cfg.root as *mut ::core::ffi::c_char);
    if strncmp(abs, &raw mut cfg.root as *mut ::core::ffi::c_char, rl) != 0 as ::core::ffi::c_int
        || *abs.offset(rl as isize) as ::core::ffi::c_int != '/' as i32
            && *abs.offset(rl as isize) as ::core::ffi::c_int != 0 as ::core::ffi::c_int
    {
        snprintf(
            out as *mut ::core::ffi::c_char,
            PATH_MAX as size_t,
            b"%s\0" as *const u8 as *const ::core::ffi::c_char,
            abs,
        );
        return true_0 != 0;
    }
    let mut ml: size_t = strlen(&raw mut cfg.main_dir as *mut ::core::ffi::c_char);
    if strncmp(abs, &raw mut cfg.main_dir as *mut ::core::ffi::c_char, ml)
        == 0 as ::core::ffi::c_int
        && *abs.offset(ml as isize) as ::core::ffi::c_int == '/' as i32
    {
        snprintf(
            out as *mut ::core::ffi::c_char,
            PATH_MAX as size_t,
            b"%s\0" as *const u8 as *const ::core::ffi::c_char,
            abs.offset(ml as isize)
                .offset(1 as ::core::ffi::c_int as isize),
        );
        return true_0 != 0;
    }
    let mut a: *const ::core::ffi::c_char = abs;
    let mut m: *const ::core::ffi::c_char = &raw mut cfg.main_dir as *mut ::core::ffi::c_char;
    let mut common: size_t = 0 as size_t;
    let mut i: size_t = 0 as size_t;
    while *a.offset(i as isize) as ::core::ffi::c_int != 0
        && *m.offset(i as isize) as ::core::ffi::c_int != 0
        && *a.offset(i as isize) as ::core::ffi::c_int
            == *m.offset(i as isize) as ::core::ffi::c_int
    {
        if *a.offset(i as isize) as ::core::ffi::c_int == '/' as i32 {
            common = i;
        }
        i = i.wrapping_add(1);
    }
    if *m.offset(strlen(m) as isize) as ::core::ffi::c_int == 0 as ::core::ffi::c_int
        && strncmp(a, m, strlen(m)) == 0 as ::core::ffi::c_int
    {
        common = strlen(m);
    }
    *out.offset(0 as ::core::ffi::c_int as isize) = 0 as ::core::ffi::c_char;
    let mut q: *const ::core::ffi::c_char = m.offset(common as isize);
    while *q != 0 {
        if *q as ::core::ffi::c_int == '/' as i32 {
            strncat(
                out as *mut ::core::ffi::c_char,
                b"../\0" as *const u8 as *const ::core::ffi::c_char,
                (PATH_MAX as size_t)
                    .wrapping_sub(strlen(out as *const ::core::ffi::c_char))
                    .wrapping_sub(1 as size_t),
            );
        }
        q = q.offset(1);
    }
    strncat(
        out as *mut ::core::ffi::c_char,
        a.offset(common as isize)
            .offset(1 as ::core::ffi::c_int as isize),
        (PATH_MAX as size_t)
            .wrapping_sub(strlen(out as *const ::core::ffi::c_char))
            .wrapping_sub(1 as size_t),
    );
    return true_0 != 0;
}
unsafe extern "C" fn canonical_path(
    mut abs: *const ::core::ffi::c_char,
    mut out: *mut ::core::ffi::c_char,
) {
    if !realpath(abs, out as *mut ::core::ffi::c_char).is_null() {
        return;
    }
    snprintf(
        out as *mut ::core::ffi::c_char,
        PATH_MAX as size_t,
        b"%s\0" as *const u8 as *const ::core::ffi::c_char,
        abs,
    );
    let mut slash: *mut ::core::ffi::c_char =
        strrchr(out as *const ::core::ffi::c_char, '/' as i32);
    if slash.is_null() {
        return;
    }
    let mut leaf: [::core::ffi::c_char; 1024] = [0; 1024];
    snprintf(
        &raw mut leaf as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 1024]>() as size_t,
        b"%s\0" as *const u8 as *const ::core::ffi::c_char,
        slash.offset(1 as ::core::ffi::c_int as isize),
    );
    *slash = 0 as ::core::ffi::c_char;
    let mut parent: [::core::ffi::c_char; 1024] = [0; 1024];
    if realpath(
        if *out.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int != 0 {
            out as *const ::core::ffi::c_char
        } else {
            b"/\0" as *const u8 as *const ::core::ffi::c_char
        },
        &raw mut parent as *mut ::core::ffi::c_char,
    )
    .is_null()
    {
        *slash = '/' as i32 as ::core::ffi::c_char;
        return;
    }
    snprintf(
        out as *mut ::core::ffi::c_char,
        PATH_MAX as size_t,
        b"%s/%s\0" as *const u8 as *const ::core::ffi::c_char,
        &raw mut parent as *mut ::core::ffi::c_char,
        &raw mut leaf as *mut ::core::ffi::c_char,
    );
}
// The original editor buffers remain separate from generated preview overlays.
// Services never write a bibliography, cache, or modified source into the project.
static mut AUX_ORIGINAL_BUFFERS: std::collections::BTreeMap<String, String> = std::collections::BTreeMap::new();
static mut AUX_GENERATED_FILES: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
static mut AUX_WARNINGS: Vec<String> = Vec::new();
static mut AUX_SOURCE_WARNINGS: Vec<String> = Vec::new();
unsafe fn aux_load_original(path: &str) -> Option<String> {
    if let Some(text) = AUX_ORIGINAL_BUFFERS.get(path) { return Some(text.clone()); }
    let main_dir = std::ffi::CStr::from_ptr(cfg.main_dir.as_ptr()).to_string_lossy();
    if let Ok(text) = std::fs::read_to_string(std::path::Path::new(main_dir.as_ref()).join(path)) { return Some(text); }
    // Read package metadata through the same in-process TeX Live provider used
    // by the engine, so BBL version tags match the installed biblatex version.
    if path == "biblatex.sty" || path.ends_with(".bbx") || path.ends_with(".bst") {
        if let Ok(name) = std::ffi::CString::new(path) {
            let found = texlive_file_path(name.as_ptr(), std::ptr::null_mut());
            if !found.is_null() { return std::fs::read_to_string(std::ffi::CStr::from_ptr(found).to_string_lossy().as_ref()).ok(); }
        }
    }
    None
}
unsafe fn prepare_embedded_auxiliary_files() {
    let main = std::ffi::CStr::from_ptr(cfg.tex_name.as_ptr()).to_string_lossy().into_owned();
    let prepared = crate::embedded_aux::prepare(&main, |path| aux_load_original(path));
    let new_files = prepared.files.keys().cloned().collect::<std::collections::BTreeSet<_>>();
    for old in AUX_GENERATED_FILES.difference(&new_files).cloned().collect::<Vec<_>>() {
        if let Ok(key) = std::ffi::CString::new(old.as_str()) {
            if let Some(original) = aux_load_original(&old) {
                interpret_open(key.as_ptr(), original.as_ptr().cast(), original.len());
            } else {
                interpret_close(key.as_ptr());
            }
        }
    }
    for (path,text) in prepared.files {
        if let Ok(key) = std::ffi::CString::new(path) { interpret_open(key.as_ptr(), text.as_ptr().cast(), text.len()); }
    }
    AUX_GENERATED_FILES = new_files;
    AUX_SOURCE_WARNINGS = prepared.warnings[prepared.bibliography_warning_count..].to_vec();
    AUX_WARNINGS = prepared.warnings;
    for warning in &AUX_WARNINGS { eprintln!("[embedded auxiliary] {warning}"); }
}

unsafe fn aux_load_after_pass(path: &str) -> Option<String> {
    if path.ends_with(".aux") || path.ends_with(".bcf") {
        if let Ok(key)=std::ffi::CString::new(path) {
            let entry=filesystem_lookup(txp_engine_fs(engine),key.as_ptr());
            if !entry.is_null() && (*entry).seen>=0 && (*entry).saved.level==FILE_WRITE && !(*entry).saved.data.is_null() {
                let data=(*entry).saved.data;
                return Some(String::from_utf8_lossy(std::slice::from_raw_parts((*data).data,(*data).len)).into_owned());
            }
            if !entry.is_null() && (*entry).edit_data_from_convergence && !(*entry).edit_data.is_null() {
                let data=(*entry).edit_data;
                return Some(String::from_utf8_lossy(std::slice::from_raw_parts((*data).data,(*data).len)).into_owned());
            }
        }
        // Only actual outputs from this embedded session are authoritative;
        // an unrelated previous on-disk AUX/BCF must not drive new citations.
        return None;
    }
    aux_load_original(path)
}
unsafe fn refresh_embedded_bibliography() -> bool {
    let main=std::ffi::CStr::from_ptr(cfg.tex_name.as_ptr()).to_string_lossy().into_owned();
    let prepared=crate::embedded_aux::prepare_from_controls(&main,|path|aux_load_after_pass(path));
    AUX_WARNINGS=AUX_SOURCE_WARNINGS.clone();
    for warning in prepared.warnings {if !AUX_WARNINGS.contains(&warning){AUX_WARNINGS.push(warning);}}
    if prepared.files.is_empty(){return false;}
    let changed=prepared.files.iter().any(|(path,text)|{
        let Ok(key)=std::ffi::CString::new(path.as_str())else{return false;};
        let entry=filesystem_lookup(txp_engine_fs(engine),key.as_ptr());
        if entry.is_null(){return true;}
        let data=if !(*entry).edit_data.is_null(){(*entry).edit_data}else{(*entry).fs_data};
        data.is_null()||std::slice::from_raw_parts((*data).data,(*data).len)!=text.as_bytes()
    });
    if !changed{return false;}
    // Carry the actual AUX/BCF and other TeX-generated files into the next
    // pass before replacing BBL. The engine's existing convergence machinery
    // owns checkpoint state and keeps all output in its virtual filesystem.
    txp_engine_start_finishing(engine);
    let converging=txp_engine_finish_convergence(engine);
    txp_engine_begin_changes(engine);
    for (path,text) in prepared.files {
        if let Ok(key)=std::ffi::CString::new(path.as_str()){interpret_open(key.as_ptr(),text.as_ptr().cast(),text.len());AUX_GENERATED_FILES.insert(path);}
    }
    let rolled_back=txp_engine_end_changes(engine);
    if !converging && !rolled_back {txp_engine_restart(engine);}
    txp_engine_step(engine,true);
    eprintln!("[embedded auxiliary] bibliography refreshed from executed TeX control files");
    true
}

unsafe extern "C" fn interpret_open(
    mut key: *const ::core::ffi::c_char,
    mut data: *const ::core::ffi::c_void,
    mut size: size_t,
) {
    let mut e: *mut fileentry_t = txp_engine_find_file(engine, key);
    let mut changed: ::core::ffi::c_int = -(1 as ::core::ffi::c_int);
    if !(*e).edit_data.is_null() && !(*e).edit_data_from_convergence {
        changed = find_diff((*e).edit_data, data, size);
        if changed == size as ::core::ffi::c_int && (*(*e).edit_data).len == size {
            return;
        }
        tbuf_drop((*e).edit_data);
        (*e).edit_data = tbuf_from_copy(data, size);
    } else {
        tbuf_drop((*e).edit_data);
        (*e).edit_data = tbuf_from_copy(data, size);
        (*e).edit_data_from_convergence = false_0 != 0;
        if !(*e).fs_data.is_null() {
            changed = find_diff((*e).fs_data, data, size);
            if changed == size as ::core::ffi::c_int && (*(*e).fs_data).len == size {
                changed = -(1 as ::core::ffi::c_int);
            }
        } else if (*e).seen >= 0 as ::core::ffi::c_int {
            changed = 0 as ::core::ffi::c_int;
        }
    }
    if changed >= 0 as ::core::ffi::c_int {
        txp_engine_notify_file_changes(engine, e, changed);
    }
}
unsafe extern "C" fn interpret_close(mut key: *const ::core::ffi::c_char) {
    let mut e: *mut fileentry_t = filesystem_lookup(txp_engine_fs(engine), key);
    if e.is_null()
        || (*e).edit_data.is_null()
        || (*e).edit_data_from_convergence as ::core::ffi::c_int != 0
    {
        return;
    }
    txp_engine_scan_file(engine, e);
    let mut changed: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    if !(*e).fs_data.is_null() {
        changed = find_diff(
            (*e).fs_data,
            (*(*e).edit_data).data as *const ::core::ffi::c_void,
            (*(*e).edit_data).len,
        );
        if changed == (*(*e).edit_data).len as ::core::ffi::c_int
            && (*(*e).fs_data).len == (*(*e).edit_data).len
        {
            changed = -(1 as ::core::ffi::c_int);
        }
    }
    tbuf_drop((*e).edit_data);
    (*e).edit_data = ::core::ptr::null_mut::<tbuf>();
    if changed >= 0 as ::core::ffi::c_int {
        txp_engine_notify_file_changes(engine, e, changed);
    }
}
static mut upd: merged_update = merged_update {
    valid: false,
    generation: 0,
    n: 0,
    cap: 0,
    paths: ::core::ptr::null::<*mut ::core::ffi::c_char>() as *mut *mut ::core::ffi::c_char,
    texts: ::core::ptr::null::<*mut ::core::ffi::c_char>() as *mut *mut ::core::ffi::c_char,
    lens: ::core::ptr::null::<size_t>() as *mut size_t,
};
unsafe extern "C" fn upd_set(
    mut path: *const ::core::ffi::c_char,
    mut text: *const ::core::ffi::c_char,
    mut len: size_t,
) {
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < upd.n {
        if strcmp(*upd.paths.offset(i as isize), path) == 0 as ::core::ffi::c_int {
            free(*upd.texts.offset(i as isize) as *mut ::core::ffi::c_void);
            let ref mut fresh3 = *upd.texts.offset(i as isize);
            *fresh3 = (if !text.is_null() {
                malloc(len.wrapping_add(1 as size_t))
            } else {
                NULL
            }) as *mut ::core::ffi::c_char;
            if !text.is_null() {
                memcpy(
                    *upd.texts.offset(i as isize) as *mut ::core::ffi::c_void,
                    text as *const ::core::ffi::c_void,
                    len,
                );
            }
            *upd.lens.offset(i as isize) = if !text.is_null() {
                len
            } else {
                SIZE_MAX as size_t
            };
            return;
        }
        i += 1;
    }
    if upd.n == upd.cap {
        upd.cap = if upd.cap != 0 {
            upd.cap * 2 as ::core::ffi::c_int
        } else {
            8 as ::core::ffi::c_int
        };
        upd.paths = realloc(
            upd.paths as *mut ::core::ffi::c_void,
            (::core::mem::size_of::<*mut ::core::ffi::c_char>() as size_t)
                .wrapping_mul(upd.cap as size_t),
        ) as *mut *mut ::core::ffi::c_char;
        upd.texts = realloc(
            upd.texts as *mut ::core::ffi::c_void,
            (::core::mem::size_of::<*mut ::core::ffi::c_char>() as size_t)
                .wrapping_mul(upd.cap as size_t),
        ) as *mut *mut ::core::ffi::c_char;
        upd.lens = realloc(
            upd.lens as *mut ::core::ffi::c_void,
            (::core::mem::size_of::<size_t>() as size_t).wrapping_mul(upd.cap as size_t),
        ) as *mut size_t;
        if upd.paths.is_null() || upd.texts.is_null() || upd.lens.is_null() {
            abort();
        }
    }
    let ref mut fresh4 = *upd.paths.offset(upd.n as isize);
    *fresh4 = strdup(path);
    let ref mut fresh5 = *upd.texts.offset(upd.n as isize);
    *fresh5 = (if !text.is_null() {
        malloc(len.wrapping_add(1 as size_t))
    } else {
        NULL
    }) as *mut ::core::ffi::c_char;
    if !text.is_null() {
        memcpy(
            *upd.texts.offset(upd.n as isize) as *mut ::core::ffi::c_void,
            text as *const ::core::ffi::c_void,
            len,
        );
    }
    *upd.lens.offset(upd.n as isize) = if !text.is_null() {
        len
    } else {
        SIZE_MAX as size_t
    };
    upd.n += 1;
}
unsafe extern "C" fn upd_clear() {
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < upd.n {
        free(*upd.paths.offset(i as isize) as *mut ::core::ffi::c_void);
        free(*upd.texts.offset(i as isize) as *mut ::core::ffi::c_void);
        i += 1;
    }
    upd.n = 0 as ::core::ffi::c_int;
    upd.valid = false_0 != 0;
}
static mut engine_started: bool = false;
unsafe extern "C" fn apply_update() {
    if engine.is_null() {
        engine = txp_engine_new(
            &raw mut cfg.engine as *mut ::core::ffi::c_char,
            b"\0" as *const u8 as *const ::core::ffi::c_char,
            &raw mut cfg.tex_name as *mut ::core::ffi::c_char,
        );
    }
    txp_engine_begin_changes(engine);
    txp_engine_detect_changes(engine);
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < upd.n {
        let mut key: [::core::ffi::c_char; 1024] = [0; 1024];
        let mut canon: [::core::ffi::c_char; 1024] = [0; 1024];
        if !(*(*upd.paths.offset(i as isize)).offset(0 as ::core::ffi::c_int as isize)
            as ::core::ffi::c_int
            != '/' as i32)
        {
            canonical_path(
                *upd.paths.offset(i as isize),
                &raw mut canon as *mut ::core::ffi::c_char,
            );
            if vfs_key(
                &raw mut canon as *mut ::core::ffi::c_char,
                &raw mut key as *mut ::core::ffi::c_char,
            ) {
                if *upd.lens.offset(i as isize) == SIZE_MAX as size_t {
                    AUX_ORIGINAL_BUFFERS.remove(std::ffi::CStr::from_ptr(key.as_ptr()).to_string_lossy().as_ref());
                    interpret_close(&raw mut key as *mut ::core::ffi::c_char);
                } else {
                    let original = std::slice::from_raw_parts(*upd.texts.offset(i as isize) as *const u8, *upd.lens.offset(i as isize));
                    if let Ok(text) = std::str::from_utf8(original) { AUX_ORIGINAL_BUFFERS.insert(std::ffi::CStr::from_ptr(key.as_ptr()).to_string_lossy().into_owned(), text.to_string()); }
                    interpret_open(
                        &raw mut key as *mut ::core::ffi::c_char,
                        *upd.texts.offset(i as isize) as *const ::core::ffi::c_void,
                        *upd.lens.offset(i as isize),
                    );
                }
            }
        }
        i += 1;
    }
    prepare_embedded_auxiliary_files();
    let mut changed: bool = txp_engine_end_changes(engine);
    generation = upd.generation;
    upd_clear();
    if changed as ::core::ffi::c_int != 0 || !engine_started {
        engine_started = true_0 != 0;
        current_seq = 0 as ::core::ffi::c_int;
        generation_start_ms = now_ms();
        rerun_count = 0 as ::core::ffi::c_int;
        crash_retried = false_0 != 0;
        watchdog_killed = false_0 != 0;
        published_pages_this_pass = 0 as ::core::ffi::c_int;
        txp_engine_step(engine, true_0 != 0);
        was_running = txp_engine_get_status(engine) as ::core::ffi::c_uint
            == DOC_RUNNING as ::core::ffi::c_int as ::core::ffi::c_uint;
    } else if txp_engine_get_status(engine) as ::core::ffi::c_uint
        == DOC_TERMINATED as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        let mut b: pbuf = pbuf {
            data: ::core::ptr::null_mut::<::core::ffi::c_uchar>(),
            len: 0,
            cap: 0,
        };
        pbuf_printf(
            &raw mut b,
            b"{\"event\":\"idle\",\"generation\":%lld,\"seq\":%d}\0" as *const u8
                as *const ::core::ffi::c_char,
            generation,
            current_seq,
        );
        emit(&raw mut b);
        pbuf_free(&raw mut b);
    }
}
unsafe extern "C" fn handle_line(mut line: *const ::core::ffi::c_char, mut len: size_t) {
    let mut msg: *mut json = json_parse(line, len);
    if msg.is_null() {
        emit_simple(
            b"error\0" as *const u8 as *const ::core::ffi::c_char,
            b"protocol\0" as *const u8 as *const ::core::ffi::c_char,
            b"malformed request\0" as *const u8 as *const ::core::ffi::c_char,
        );
        return;
    }
    let mut op: *const ::core::ffi::c_char = json_string(json_get(
        msg,
        b"op\0" as *const u8 as *const ::core::ffi::c_char,
    ));
    if op.is_null() {
        emit_simple(
            b"error\0" as *const u8 as *const ::core::ffi::c_char,
            b"protocol\0" as *const u8 as *const ::core::ffi::c_char,
            b"missing op\0" as *const u8 as *const ::core::ffi::c_char,
        );
    } else if strcmp(op, b"update\0" as *const u8 as *const ::core::ffi::c_char)
        == 0 as ::core::ffi::c_int
    {
        let mut g: ::core::ffi::c_double = 0 as ::core::ffi::c_int as ::core::ffi::c_double;
        json_number(
            json_get(
                msg,
                b"generation\0" as *const u8 as *const ::core::ffi::c_char,
            ),
            &raw mut g,
        );
        if g as ::core::ffi::c_longlong > upd.generation || !upd.valid {
            upd.generation = g as ::core::ffi::c_longlong;
        }
        let mut files: *mut json =
            json_get(msg, b"files\0" as *const u8 as *const ::core::ffi::c_char);
        let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
        while !files.is_null()
            && (*files).type_0 as ::core::ffi::c_uint
                == JSON_ARRAY as ::core::ffi::c_int as ::core::ffi::c_uint
            && i < (*files).n
        {
            let mut path: *const ::core::ffi::c_char = json_string(json_get(
                *(*files).items.offset(i as isize),
                b"path\0" as *const u8 as *const ::core::ffi::c_char,
            ));
            let mut text: *mut json = json_get(
                *(*files).items.offset(i as isize),
                b"text\0" as *const u8 as *const ::core::ffi::c_char,
            );
            if !path.is_null()
                && !text.is_null()
                && (*text).type_0 as ::core::ffi::c_uint
                    == JSON_STRING as ::core::ffi::c_int as ::core::ffi::c_uint
            {
                upd_set(path, (*text).str_0, (*text).len);
            }
            i += 1;
        }
        let mut closed: *mut json =
            json_get(msg, b"closed\0" as *const u8 as *const ::core::ffi::c_char);
        let mut i_0: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
        while !closed.is_null()
            && (*closed).type_0 as ::core::ffi::c_uint
                == JSON_ARRAY as ::core::ffi::c_int as ::core::ffi::c_uint
            && i_0 < (*closed).n
        {
            let mut path_0: *const ::core::ffi::c_char =
                json_string(*(*closed).items.offset(i_0 as isize));
            if !path_0.is_null() {
                upd_set(
                    path_0,
                    ::core::ptr::null::<::core::ffi::c_char>(),
                    0 as size_t,
                );
            }
            i_0 += 1;
        }
        upd.valid = true_0 != 0;
    } else if strcmp(op, b"release\0" as *const u8 as *const ::core::ffi::c_char)
        == 0 as ::core::ffi::c_int
    {
        let mut s: ::core::ffi::c_double = 0.;
        if json_number(
            json_get(msg, b"seq\0" as *const u8 as *const ::core::ffi::c_char),
            &raw mut s,
        ) {
            release(s as ::core::ffi::c_int);
        }
    } else if strcmp(op, b"quit\0" as *const u8 as *const ::core::ffi::c_char)
        == 0 as ::core::ffi::c_int
    {
        json_free(msg);
        if !engine.is_null() {
            txp_engine_free(engine);
        }
        exit(0 as ::core::ffi::c_int);
    } else {
        emit_simple(
            b"error\0" as *const u8 as *const ::core::ffi::c_char,
            b"protocol\0" as *const u8 as *const ::core::ffi::c_char,
            b"unknown op\0" as *const u8 as *const ::core::ffi::c_char,
        );
    }
    json_free(msg);
}
unsafe extern "C" fn log_finished(mut log: *const tbuf) -> bool {
    return contains(
        log,
        b"Output written on\0" as *const u8 as *const ::core::ffi::c_char,
    ) as ::core::ffi::c_int
        != 0
        || contains(
            log,
            b"No pages of output\0" as *const u8 as *const ::core::ffi::c_char,
        ) as ::core::ffi::c_int
            != 0;
}
unsafe extern "C" fn publish_or_defer(mut s: *mut snapshot, mut complete: bool) {
    if publish(s, complete, generation) {
        return;
    }
    snapshot_clear(&raw mut pending);
    pending = *s;
    memset(
        s as *mut ::core::ffi::c_void,
        0 as ::core::ffi::c_int,
        ::core::mem::size_of::<snapshot>() as size_t,
    );
    pending_valid = true_0 != 0;
    pending_complete = complete;
    pending_generation = generation;
}
unsafe extern "C" fn on_pass_end() {
    if txp_engine_take_barrier(engine) {
        fprintf(
            __stderrp,
            b"[driver] restarting engine after font barrier\n\0" as *const u8
                as *const ::core::ffi::c_char,
        );
        txp_engine_restart(engine);
        published_pages_this_pass = 0 as ::core::ffi::c_int;
        was_running = true_0 != 0;
        return;
    }
    let mut log: *mut tbuf = txp_engine_log(engine);
    let mut finished: bool = log_finished(log);
    if !finished {
        if watchdog_killed {
            emit_failed(
                log,
                b"stuck\0" as *const u8 as *const ::core::ffi::c_char,
                b"TeX made no progress for 20 seconds (possible infinite loop). Edit the document to retry.\0"
                    as *const u8 as *const ::core::ffi::c_char,
            );
            return;
        }
        if !crash_retried {
            crash_retried = true_0 != 0;
            fprintf(
                __stderrp,
                b"[driver] engine ended unexpectedly (status %d); restarting from scratch\n\0"
                    as *const u8 as *const ::core::ffi::c_char,
                txp_engine_last_status(engine),
            );
            txp_engine_restart(engine);
            published_pages_this_pass = 0 as ::core::ffi::c_int;
            was_running = true_0 != 0;
            return;
        }
        emit_failed(
            log,
            b"crashed\0" as *const u8 as *const ::core::ffi::c_char,
            b"The embedded preview engine stopped unexpectedly.\0" as *const u8
                as *const ::core::ffi::c_char,
        );
        return;
    }
    let mut s: snapshot = snapshot {
        xdv: ::core::ptr::null_mut::<tbuf>(),
        synctex: ::core::ptr::null_mut::<tbuf>(),
        log: ::core::ptr::null_mut::<tbuf>(),
        index: ::core::ptr::null_mut::<xdv_index>(),
    };
    snapshot_take(&raw mut s);
    let mut rerun: bool = false_0 != 0;
    if rerun_count < MAX_RERUNS && refresh_embedded_bibliography() {
        rerun=true;rerun_count+=1;was_running=true;published_pages_this_pass=0;
    }
    if !rerun && txp_engine_aux_dirty(engine) as ::core::ffi::c_int != 0 && rerun_count < MAX_RERUNS {
        txp_engine_start_finishing(engine);
        rerun = txp_engine_finish_convergence(engine);
        if rerun {
            rerun_count += 1;
            was_running = true_0 != 0;
            published_pages_this_pass = 0 as ::core::ffi::c_int;
        }
    }
    let mut pages: ::core::ffi::c_int = xdv_index_page_count(s.index);
    if pages == 0 as ::core::ffi::c_int {
        if !rerun {
            emit_failed(
                s.log,
                b"no_pages\0" as *const u8 as *const ::core::ffi::c_char,
                b"The document produced no pages.\0" as *const u8 as *const ::core::ffi::c_char,
            );
        }
        snapshot_clear(&raw mut s);
        return;
    }
    snapshot_clear(&raw mut last_complete);
    last_complete.xdv = tbuf_keep(s.xdv);
    last_complete.synctex = tbuf_keep(s.synctex);
    last_complete.log = tbuf_keep(s.log);
    last_complete.index = xdv_index_new();
    xdv_index_update(
        last_complete.index,
        (*last_complete.xdv).data,
        (*last_complete.xdv).len,
    );
    publish_or_defer(&raw mut s, !rerun);
    snapshot_clear(&raw mut s);
}
unsafe extern "C" fn maybe_publish_intermediate() {
    if rerun_count > 0 as ::core::ffi::c_int {
        return;
    }
    let mut pages: ::core::ffi::c_int = xdv_index_page_count(txp_engine_xdv(engine));
    if pages <= published_pages_this_pass {
        return;
    }
    let mut t: ::core::ffi::c_longlong = now_ms();
    if t - last_publish_ms < INTERMEDIATE_INTERVAL_MS as ::core::ffi::c_longlong
        || t - generation_start_ms < 120 as ::core::ffi::c_longlong
    {
        return;
    }
    if unreleased_count() >= MAX_UNRELEASED {
        return;
    }
    let mut s: snapshot = snapshot {
        xdv: ::core::ptr::null_mut::<tbuf>(),
        synctex: ::core::ptr::null_mut::<tbuf>(),
        log: ::core::ptr::null_mut::<tbuf>(),
        index: ::core::ptr::null_mut::<xdv_index>(),
    };
    snapshot_take(&raw mut s);
    published_pages_this_pass = pages;
    publish(&raw mut s, false_0 != 0, generation);
    snapshot_clear(&raw mut s);
}
unsafe extern "C" fn run_engine_slice() {
    let mut t0: ::core::ffi::c_longlong = now_ms();
    while txp_engine_get_status(engine) as ::core::ffi::c_uint
        == DOC_RUNNING as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        if !txp_engine_step(engine, false_0 != 0) {
            break;
        }
        if now_ms() - t0 > 25 as ::core::ffi::c_longlong {
            break;
        }
    }
    if txp_engine_get_status(engine) as ::core::ffi::c_uint
        == DOC_RUNNING as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        if txp_engine_idle_ms(engine) > WATCHDOG_MS {
            fprintf(
                __stderrp,
                b"[driver] watchdog: no engine progress, killing\n\0" as *const u8
                    as *const ::core::ffi::c_char,
            );
            watchdog_killed = true_0 != 0;
            txp_engine_kill_running(engine);
        } else {
            maybe_publish_intermediate();
        }
    }
    if was_running as ::core::ffi::c_int != 0
        && txp_engine_get_status(engine) as ::core::ffi::c_uint
            == DOC_TERMINATED as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        was_running = false_0 != 0;
        on_pass_end();
    }
}
unsafe extern "C" fn on_fatal_signal(mut sig: ::core::ffi::c_int) {
    signal(
        SIGTERM,
        ::core::mem::transmute::<
            ::libc::intptr_t,
            Option<unsafe extern "C" fn(::core::ffi::c_int) -> ()>,
        >(1 as ::core::ffi::c_int as ::libc::intptr_t),
    );
    kill(0 as pid_t, SIGKILL);
    signal(sig, SIG_DFL);
    raise(sig);
}
unsafe extern "C" fn usage_error(mut msg: *const ::core::ffi::c_char) {
    emit_simple(
        b"error\0" as *const u8 as *const ::core::ffi::c_char,
        b"usage\0" as *const u8 as *const ::core::ffi::c_char,
        msg,
    );
    exit(2 as ::core::ffi::c_int);
}
unsafe extern "C" fn default_engine_path(mut out: *mut ::core::ffi::c_char) {
    let mut self_0: [::core::ffi::c_char; 1024] = [
        0 as ::core::ffi::c_int as ::core::ffi::c_char,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
    ];
    let mut size: uint32_t = ::core::mem::size_of::<[::core::ffi::c_char; 1024]>() as uint32_t;
    if _NSGetExecutablePath(&raw mut self_0 as *mut ::core::ffi::c_char, &raw mut size)
        != 0 as ::core::ffi::c_int
    {
        self_0[0 as ::core::ffi::c_int as usize] = 0 as ::core::ffi::c_char;
    }
    let mut real: [::core::ffi::c_char; 1024] = [0; 1024];
    if self_0[0 as ::core::ffi::c_int as usize] as ::core::ffi::c_int != 0
        && !realpath(
            &raw mut self_0 as *mut ::core::ffi::c_char,
            &raw mut real as *mut ::core::ffi::c_char,
        )
        .is_null()
    {
        let mut slash: *mut ::core::ffi::c_char =
            strrchr(&raw mut real as *mut ::core::ffi::c_char, '/' as i32);
        if !slash.is_null() {
            *slash = 0 as ::core::ffi::c_char;
        }
        snprintf(
            out as *mut ::core::ffi::c_char,
            PATH_MAX as size_t,
            b"%s/pitex-preview-xetex\0" as *const u8 as *const ::core::ffi::c_char,
            &raw mut real as *mut ::core::ffi::c_char,
        );
    } else {
        snprintf(
            out as *mut ::core::ffi::c_char,
            PATH_MAX as size_t,
            b"pitex-preview-xetex\0" as *const u8 as *const ::core::ffi::c_char,
        );
    };
}
unsafe fn main_0(
    mut argc: ::core::ffi::c_int,
    mut argv: *mut *mut ::core::ffi::c_char,
) -> ::core::ffi::c_int {
    signal(
        SIGPIPE,
        ::core::mem::transmute::<
            ::libc::intptr_t,
            Option<unsafe extern "C" fn(::core::ffi::c_int) -> ()>,
        >(1 as ::core::ffi::c_int as ::libc::intptr_t),
    );
    if getpgrp() == getpid() {
        let mut sa: sigaction = sigaction {
            __sigaction_u: __sigaction_u { __sa_handler: None },
            sa_mask: 0,
            sa_flags: 0,
        };
        memset(
            &raw mut sa as *mut ::core::ffi::c_void,
            0 as ::core::ffi::c_int,
            ::core::mem::size_of::<sigaction>() as size_t,
        );
        sa.__sigaction_u.__sa_handler =
            Some(on_fatal_signal as unsafe extern "C" fn(::core::ffi::c_int) -> ())
                as Option<unsafe extern "C" fn(::core::ffi::c_int) -> ()>;
        sa.sa_flags = SA_RESETHAND;
        let fatal: [::core::ffi::c_int; 8] = [
            SIGTERM, SIGHUP, SIGINT, SIGABRT, SIGSEGV, SIGBUS, SIGILL, SIGFPE,
        ];
        let mut i: size_t = 0 as size_t;
        while i
            < (::core::mem::size_of::<[::core::ffi::c_int; 8]>() as usize)
                .wrapping_div(::core::mem::size_of::<::core::ffi::c_int>() as usize)
        {
            sigaction(
                fatal[i as usize],
                &raw mut sa,
                ::core::ptr::null_mut::<sigaction>(),
            );
            i = i.wrapping_add(1);
        }
    }
    let mut root: *const ::core::ffi::c_char = ::core::ptr::null::<::core::ffi::c_char>();
    let mut main_rel: *const ::core::ffi::c_char = ::core::ptr::null::<::core::ffi::c_char>();
    let mut out: *const ::core::ffi::c_char = ::core::ptr::null::<::core::ffi::c_char>();
    let mut eng: *const ::core::ffi::c_char = ::core::ptr::null::<::core::ffi::c_char>();
    let mut cache: *const ::core::ffi::c_char = ::core::ptr::null::<::core::ffi::c_char>();
    let mut i_0: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
    while i_0 < argc {
        if strcmp(
            *argv.offset(i_0 as isize),
            b"--root\0" as *const u8 as *const ::core::ffi::c_char,
        ) == 0
            && (i_0 + 1 as ::core::ffi::c_int) < argc
        {
            i_0 += 1;
            root = *argv.offset(i_0 as isize);
        } else if strcmp(
            *argv.offset(i_0 as isize),
            b"--main\0" as *const u8 as *const ::core::ffi::c_char,
        ) == 0
            && (i_0 + 1 as ::core::ffi::c_int) < argc
        {
            i_0 += 1;
            main_rel = *argv.offset(i_0 as isize);
        } else if strcmp(
            *argv.offset(i_0 as isize),
            b"--out\0" as *const u8 as *const ::core::ffi::c_char,
        ) == 0
            && (i_0 + 1 as ::core::ffi::c_int) < argc
        {
            i_0 += 1;
            out = *argv.offset(i_0 as isize);
        } else if strcmp(
            *argv.offset(i_0 as isize),
            b"--engine\0" as *const u8 as *const ::core::ffi::c_char,
        ) == 0
            && (i_0 + 1 as ::core::ffi::c_int) < argc
        {
            i_0 += 1;
            eng = *argv.offset(i_0 as isize);
        } else if strcmp(
            *argv.offset(i_0 as isize),
            b"--cache\0" as *const u8 as *const ::core::ffi::c_char,
        ) == 0
            && (i_0 + 1 as ::core::ffi::c_int) < argc
        {
            i_0 += 1;
            cache = *argv.offset(i_0 as isize);
        } else if strcmp(
            *argv.offset(i_0 as isize),
            b"--version\0" as *const u8 as *const ::core::ffi::c_char,
        ) == 0
        {
            printf(
                b"pitex-preview 1 (TeXpresso e8df7709077b-derived engine)\n\0" as *const u8
                    as *const ::core::ffi::c_char,
            );
            return 0 as ::core::ffi::c_int;
        } else {
            usage_error(
                b"usage: pitex-preview --root DIR --main REL --out DIR [--engine PATH] [--cache DIR]\0"
                    as *const u8 as *const ::core::ffi::c_char,
            );
        }
        i_0 += 1;
    }
    if root.is_null() || main_rel.is_null() || out.is_null() {
        usage_error(
            b"--root, --main and --out are required\0" as *const u8 as *const ::core::ffi::c_char,
        );
    }
    if realpath(root, &raw mut cfg.root as *mut ::core::ffi::c_char).is_null() {
        usage_error(b"project root does not exist\0" as *const u8 as *const ::core::ffi::c_char);
    }
    if realpath(out, &raw mut cfg.out as *mut ::core::ffi::c_char).is_null() {
        usage_error(
            b"output directory does not exist\0" as *const u8 as *const ::core::ffi::c_char,
        );
    }
    if *main_rel.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int == '/' as i32
        || !strstr(main_rel, b"..\0" as *const u8 as *const ::core::ffi::c_char).is_null()
    {
        usage_error(
            b"--main must be relative to --root\0" as *const u8 as *const ::core::ffi::c_char,
        );
    }
    let mut main_abs: [::core::ffi::c_char; 1024] = [0; 1024];
    snprintf(
        &raw mut main_abs as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 1024]>() as size_t,
        b"%s/%s\0" as *const u8 as *const ::core::ffi::c_char,
        &raw mut cfg.root as *mut ::core::ffi::c_char,
        main_rel,
    );
    let mut main_real: [::core::ffi::c_char; 1024] = [0; 1024];
    if realpath(
        &raw mut main_abs as *mut ::core::ffi::c_char,
        &raw mut main_real as *mut ::core::ffi::c_char,
    )
    .is_null()
    {
        usage_error(b"main document does not exist\0" as *const u8 as *const ::core::ffi::c_char);
    }
    snprintf(
        &raw mut cfg.main_dir as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 1024]>() as size_t,
        b"%s\0" as *const u8 as *const ::core::ffi::c_char,
        &raw mut main_real as *mut ::core::ffi::c_char,
    );
    let mut slash: *mut ::core::ffi::c_char = strrchr(
        &raw mut cfg.main_dir as *mut ::core::ffi::c_char,
        '/' as i32,
    );
    *slash = 0 as ::core::ffi::c_char;
    snprintf(
        &raw mut cfg.tex_name as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 256]>() as size_t,
        b"%s\0" as *const u8 as *const ::core::ffi::c_char,
        slash.offset(1 as ::core::ffi::c_int as isize),
    );
    snprintf(
        &raw mut cfg.job as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 256]>() as size_t,
        b"%s\0" as *const u8 as *const ::core::ffi::c_char,
        &raw mut cfg.tex_name as *mut ::core::ffi::c_char,
    );
    let mut dot: *mut ::core::ffi::c_char =
        strrchr(&raw mut cfg.job as *mut ::core::ffi::c_char, '.' as i32);
    if !dot.is_null() {
        *dot = 0 as ::core::ffi::c_char;
    }
    if !eng.is_null() {
        snprintf(
            &raw mut cfg.engine as *mut ::core::ffi::c_char,
            ::core::mem::size_of::<[::core::ffi::c_char; 1024]>() as size_t,
            b"%s\0" as *const u8 as *const ::core::ffi::c_char,
            eng,
        );
    } else {
        default_engine_path(&raw mut cfg.engine as *mut ::core::ffi::c_char);
    }
    if access(&raw mut cfg.engine as *mut ::core::ffi::c_char, X_OK) != 0 as ::core::ffi::c_int {
        emit_simple(
            b"error\0" as *const u8 as *const ::core::ffi::c_char,
            b"no_engine\0" as *const u8 as *const ::core::ffi::c_char,
            b"pitex-preview-xetex helper not found\0" as *const u8 as *const ::core::ffi::c_char,
        );
        return 2 as ::core::ffi::c_int;
    }
    if !cache.is_null() {
        setenv(
            b"PITEX_PREVIEW_CACHE\0" as *const u8 as *const ::core::ffi::c_char,
            cache,
            1 as ::core::ffi::c_int,
        );
    }
    let mut logpath: [::core::ffi::c_char; 1024] = [0; 1024];
    snprintf(
        &raw mut logpath as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 1024]>() as size_t,
        b"%s/driver.log\0" as *const u8 as *const ::core::ffi::c_char,
        &raw mut cfg.out as *mut ::core::ffi::c_char,
    );
    log_fd = open(
        &raw mut logpath as *mut ::core::ffi::c_char,
        O_WRONLY | O_CREAT | O_APPEND | O_TRUNC,
        0o600 as ::core::ffi::c_int,
    );
    if log_fd >= 0 as ::core::ffi::c_int {
        dup2(log_fd, STDERR_FILENO);
    }
    if chdir(&raw mut cfg.main_dir as *mut ::core::ffi::c_char) != 0 as ::core::ffi::c_int {
        usage_error(
            b"cannot enter the main document directory\0" as *const u8
                as *const ::core::ffi::c_char,
        );
    }
    if !texlive_available() {
        emit_simple(
            b"error\0" as *const u8 as *const ::core::ffi::c_char,
            b"no_tex\0" as *const u8 as *const ::core::ffi::c_char,
            b"No TeX Live installation found (kpsewhich must be on PATH).\0" as *const u8
                as *const ::core::ffi::c_char,
        );
        return 2 as ::core::ffi::c_int;
    }
    writer = xdv2pdf_new(xdv_resolver {
        env: NULL,
        load: Some(
            resolve
                as unsafe extern "C" fn(
                    *mut ::core::ffi::c_void,
                    *const ::core::ffi::c_char,
                    xdv_res_kind,
                ) -> *mut tbuf,
        ),
    });
    emit_simple(
        b"ready\0" as *const u8 as *const ::core::ffi::c_char,
        ::core::ptr::null::<::core::ffi::c_char>(),
        ::core::ptr::null::<::core::ffi::c_char>(),
    );
    let mut in_0: pbuf = pbuf {
        data: ::core::ptr::null_mut::<::core::ffi::c_uchar>(),
        len: 0,
        cap: 0,
    };
    loop {
        let mut fds: [pollfd; 2] = [pollfd {
            fd: 0,
            events: 0,
            revents: 0,
        }; 2];
        let mut nfds: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
        fds[0 as ::core::ffi::c_int as usize].fd = STDIN_FILENO;
        fds[0 as ::core::ffi::c_int as usize].events = POLLIN as ::core::ffi::c_short;
        let mut efd: ::core::ffi::c_int = if !engine.is_null()
            && txp_engine_get_status(engine) as ::core::ffi::c_uint
                == DOC_RUNNING as ::core::ffi::c_int as ::core::ffi::c_uint
        {
            txp_engine_fd(engine)
        } else {
            -(1 as ::core::ffi::c_int)
        };
        if efd >= 0 as ::core::ffi::c_int {
            fds[1 as ::core::ffi::c_int as usize].fd = efd;
            fds[1 as ::core::ffi::c_int as usize].events = POLLIN as ::core::ffi::c_short;
            nfds = 2 as ::core::ffi::c_int;
        }
        let mut timeout: ::core::ffi::c_int = if efd >= 0 as ::core::ffi::c_int {
            50 as ::core::ffi::c_int
        } else {
            -(1 as ::core::ffi::c_int)
        };
        let mut r: ::core::ffi::c_int = poll(
            &raw mut fds as *mut pollfd,
            nfds as nfds_t,
            if upd.valid as ::core::ffi::c_int != 0 {
                0 as ::core::ffi::c_int
            } else {
                timeout
            },
        );
        if r < 0 as ::core::ffi::c_int && *__error() != EINTR {
            break;
        }
        if r > 0 as ::core::ffi::c_int
            && fds[0 as ::core::ffi::c_int as usize].revents as ::core::ffi::c_int
                & (POLLIN | POLLHUP | POLLERR)
                != 0
        {
            let mut buf: [::core::ffi::c_char; 65536] = [0; 65536];
            let mut n: ssize_t = read(
                STDIN_FILENO,
                &raw mut buf as *mut ::core::ffi::c_char as *mut ::core::ffi::c_void,
                ::core::mem::size_of::<[::core::ffi::c_char; 65536]>() as size_t,
            );
            if n == 0 as ssize_t || n < 0 as ssize_t && *__error() != EINTR && *__error() != EAGAIN
            {
                break;
            }
            if n > 0 as ssize_t {
                pbuf_append(
                    &raw mut in_0,
                    &raw mut buf as *mut ::core::ffi::c_char as *const ::core::ffi::c_void,
                    n as size_t,
                );
                let mut start: size_t = 0 as size_t;
                let mut i_1: size_t = 0 as size_t;
                while i_1 < in_0.len {
                    if *in_0.data.offset(i_1 as isize) as ::core::ffi::c_int == '\n' as i32 {
                        if i_1 > start {
                            handle_line(
                                (in_0.data as *mut ::core::ffi::c_char).offset(start as isize),
                                i_1.wrapping_sub(start),
                            );
                        }
                        start = i_1.wrapping_add(1 as size_t);
                    }
                    i_1 = i_1.wrapping_add(1);
                }
                memmove(
                    in_0.data as *mut ::core::ffi::c_void,
                    in_0.data.offset(start as isize) as *const ::core::ffi::c_void,
                    in_0.len.wrapping_sub(start),
                );
                in_0.len = in_0.len.wrapping_sub(start);
                let mut p: pollfd = pollfd {
                    fd: STDIN_FILENO,
                    events: POLLIN as ::core::ffi::c_short,
                    revents: 0 as ::core::ffi::c_short,
                };
                if poll(&raw mut p, 1 as nfds_t, 0 as ::core::ffi::c_int) > 0 as ::core::ffi::c_int
                    && p.revents as ::core::ffi::c_int & POLLIN != 0
                {
                    continue;
                }
            }
        }
        if upd.valid {
            apply_update();
        }
        if !engine.is_null()
            && (was_running as ::core::ffi::c_int != 0
                || txp_engine_get_status(engine) as ::core::ffi::c_uint
                    == DOC_RUNNING as ::core::ffi::c_int as ::core::ffi::c_uint)
        {
            was_running = was_running as ::core::ffi::c_int != 0
                || txp_engine_get_status(engine) as ::core::ffi::c_uint
                    == DOC_RUNNING as ::core::ffi::c_int as ::core::ffi::c_uint;
            run_engine_slice();
        }
    }
    if !engine.is_null() {
        txp_engine_free(engine);
    }
    return 0 as ::core::ffi::c_int;
}
pub fn main() {
    let mut args_strings: Vec<Vec<u8>> = ::std::env::args()
        .map(|arg| {
            ::std::ffi::CString::new(arg)
                .expect("Failed to convert argument into CString.")
                .into_bytes_with_nul()
        })
        .collect();
    let mut args_ptrs: Vec<*mut ::core::ffi::c_char> = args_strings
        .iter_mut()
        .map(|arg| arg.as_mut_ptr() as *mut ::core::ffi::c_char)
        .chain(::core::iter::once(::core::ptr::null_mut()))
        .collect();
    unsafe {
        ::std::process::exit(main_0(
            (args_ptrs.len() - 1) as ::core::ffi::c_int,
            args_ptrs.as_mut_ptr() as *mut *mut ::core::ffi::c_char,
        ) as i32)
    }
}
