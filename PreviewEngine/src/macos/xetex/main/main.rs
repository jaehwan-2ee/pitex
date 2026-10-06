// Translated from xetex/main/main.c with C2Rust 0.22.1.
extern "C" {
    pub type __sFILEX;
    pub type ttbc_input_handle_t;
    pub type ttbc_output_handle_t;
    pub type ttbc_diagnostic_t;
    pub type txp_client;
    static mut __stdoutp: *mut FILE;
    static mut __stderrp: *mut FILE;
    fn fclose(_: *mut FILE) -> ::core::ffi::c_int;
    fn fflush(_: *mut FILE) -> ::core::ffi::c_int;
    fn fopen(
        __filename: *const ::core::ffi::c_char,
        __mode: *const ::core::ffi::c_char,
    ) -> *mut FILE;
    fn fprintf(_: *mut FILE, _: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
    fn fread(
        __ptr: *mut ::core::ffi::c_void,
        __size: size_t,
        __nitems: size_t,
        __stream: *mut FILE,
    ) -> ::core::ffi::c_ulong;
    fn fseek(_: *mut FILE, _: ::core::ffi::c_long, _: ::core::ffi::c_int) -> ::core::ffi::c_int;
    fn fwrite(
        __ptr: *const ::core::ffi::c_void,
        __size: size_t,
        __nitems: size_t,
        __stream: *mut FILE,
    ) -> ::core::ffi::c_ulong;
    fn getc(_: *mut FILE) -> ::core::ffi::c_int;
    fn putc(_: ::core::ffi::c_int, _: *mut FILE) -> ::core::ffi::c_int;
    fn rename(
        __old: *const ::core::ffi::c_char,
        __new: *const ::core::ffi::c_char,
    ) -> ::core::ffi::c_int;
    fn ungetc(_: ::core::ffi::c_int, _: *mut FILE) -> ::core::ffi::c_int;
    fn vfprintf(
        _: *mut FILE,
        _: *const ::core::ffi::c_char,
        _: ::core::ffi::VaList,
    ) -> ::core::ffi::c_int;
    fn fdopen(_: ::core::ffi::c_int, _: *const ::core::ffi::c_char) -> *mut FILE;
    fn fileno(_: *mut FILE) -> ::core::ffi::c_int;
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
    fn calloc(__count: size_t, __size: size_t) -> *mut ::core::ffi::c_void;
    fn free(_: *mut ::core::ffi::c_void);
    fn abort() -> !;
    fn getenv(_: *const ::core::ffi::c_char) -> *mut ::core::ffi::c_char;
    fn strtoll(
        __str: *const ::core::ffi::c_char,
        __endptr: *mut *mut ::core::ffi::c_char,
        __base: ::core::ffi::c_int,
    ) -> ::core::ffi::c_longlong;
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
    fn strcat(
        __s1: *mut ::core::ffi::c_char,
        __s2: *const ::core::ffi::c_char,
    ) -> *mut ::core::ffi::c_char;
    fn strcmp(
        __s1: *const ::core::ffi::c_char,
        __s2: *const ::core::ffi::c_char,
    ) -> ::core::ffi::c_int;
    fn strcpy(
        __dst: *mut ::core::ffi::c_char,
        __src: *const ::core::ffi::c_char,
    ) -> *mut ::core::ffi::c_char;
    fn strlen(__s: *const ::core::ffi::c_char) -> size_t;
    fn fstat(_: ::core::ffi::c_int, _: *mut stat) -> ::core::ffi::c_int;
    fn open(_: *const ::core::ffi::c_char, _: ::core::ffi::c_int, ...) -> ::core::ffi::c_int;
    fn flock(_: ::core::ffi::c_int, _: ::core::ffi::c_int) -> ::core::ffi::c_int;
    fn time(_: *mut time_t) -> time_t;
    fn _exit(_: ::core::ffi::c_int) -> !;
    fn access(_: *const ::core::ffi::c_char, _: ::core::ffi::c_int) -> ::core::ffi::c_int;
    fn close(_: ::core::ffi::c_int) -> ::core::ffi::c_int;
    fn unlink(_: *const ::core::ffi::c_char) -> ::core::ffi::c_int;
    fn tt_run_engine(
        dump_name: *const ::core::ffi::c_char,
        input_file_name: *const ::core::ffi::c_char,
        build_date: time_t,
    ) -> tt_history_t;
    static mut in_initex_mode: bool;
    fn format_extensions(format: ttbc_file_format) -> *mut *const ::core::ffi::c_char;
    fn ttbc_file_format_to_string(format: ttbc_file_format) -> *const ::core::ffi::c_char;
    fn texlive_available() -> bool;
    fn texlive_file_path(
        name: *const ::core::ffi::c_char,
        record_dependency: *mut FILE,
    ) -> *const ::core::ffi::c_char;
    fn texlive_check_dependencies(record: *mut FILE) -> bool;
    fn cache_path_(
        folder: *const ::core::ffi::c_char,
        name: *mut *const ::core::ffi::c_char,
    ) -> *const ::core::ffi::c_char;
    fn txp_connect(f: *mut FILE) -> *mut txp_client;
    fn txp_generation(client: *mut txp_client) -> uint32_t;
    fn txp_flush(client: *mut txp_client);
    fn txp_open(
        client: *mut txp_client,
        file: txp_file_id,
        path: *const ::core::ffi::c_char,
        kind: txp_file_kind,
        mode: txp_open_mode,
    ) -> *mut ::core::ffi::c_char;
    fn txp_read(
        client: *mut txp_client,
        file: txp_file_id,
        pos: uint32_t,
        buf: *mut ::core::ffi::c_void,
        len: size_t,
    ) -> size_t;
    fn txp_append(
        client: *mut txp_client,
        file: txp_file_id,
        buf: *const ::core::ffi::c_void,
        len: size_t,
    );
    fn txp_putc(client: *mut txp_client, file: txp_file_id, c: ::core::ffi::c_int);
    fn txp_close(client: *mut txp_client, file: txp_file_id);
    fn txp_seen(client: *mut txp_client, file: txp_file_id, pos: uint32_t);
    fn txp_gpic(
        client: *mut txp_client,
        path: *const ::core::ffi::c_char,
        typ: ::core::ffi::c_int,
        page: ::core::ffi::c_int,
        bounds: *mut ::core::ffi::c_float,
    ) -> bool;
    fn txp_spic(
        client: *mut txp_client,
        path: *const ::core::ffi::c_char,
        typ: ::core::ffi::c_int,
        page: ::core::ffi::c_int,
        bounds: *const ::core::ffi::c_float,
    );
    fn txp_font_barrier(client: *mut txp_client);
    fn txp_mtime(client: *mut txp_client, file: txp_file_id) -> uint32_t;
    fn txp_size(client: *mut txp_client, file: txp_file_id) -> uint32_t;
    static mut logging: ::core::ffi::c_int;
    fn print_backtrace();
    static mut synctex_enabled: bool;
    static mut synctex_texpresso_extension: bool;
    static mut txp_forked_child: bool;
}
pub type __builtin_va_list = *mut ::core::ffi::c_char;
pub type __uint16_t = u16;
pub type __int32_t = i32;
pub type __uint32_t = u32;
pub type __int64_t = i64;
pub type __uint64_t = u64;
pub type __darwin_size_t = usize;
pub type __darwin_va_list = __builtin_va_list;
pub type __darwin_ssize_t = isize;
pub type __darwin_time_t = ::core::ffi::c_long;
pub type __darwin_blkcnt_t = __int64_t;
pub type __darwin_blksize_t = __int32_t;
pub type __darwin_dev_t = __int32_t;
pub type __darwin_gid_t = __uint32_t;
pub type __darwin_ino64_t = __uint64_t;
pub type __darwin_mode_t = __uint16_t;
pub type __darwin_off_t = __int64_t;
pub type __darwin_uid_t = __uint32_t;
pub type int32_t = i32;
pub type intptr_t = isize;
pub type uintptr_t = usize;
pub type va_list = __darwin_va_list;
pub type size_t = __darwin_size_t;
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
pub type ssize_t = __darwin_ssize_t;
pub type uid_t = __darwin_uid_t;
pub type uint8_t = u8;
pub type uint32_t = u32;
pub type uint64_t = u64;
pub type dev_t = __darwin_dev_t;
pub type mode_t = __darwin_mode_t;
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
pub type time_t = __darwin_time_t;
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
pub type rust_input_handle_t = *mut ttbc_input_handle_t;
pub type rust_output_handle_t = *mut ttbc_output_handle_t;
pub type ttbc_file_format = ::core::ffi::c_uint;
pub const TTBC_FILE_FORMAT_VF: ttbc_file_format = 33;
pub const TTBC_FILE_FORMAT_TYPE1: ttbc_file_format = 32;
pub const TTBC_FILE_FORMAT_TRUE_TYPE: ttbc_file_format = 36;
pub const TTBC_FILE_FORMAT_TFM: ttbc_file_format = 3;
pub const TTBC_FILE_FORMAT_TEX_PS_HEADER: ttbc_file_format = 30;
pub const TTBC_FILE_FORMAT_TEX: ttbc_file_format = 26;
pub const TTBC_FILE_FORMAT_TECTONIC_PRIMARY: ttbc_file_format = 59;
pub const TTBC_FILE_FORMAT_SFD: ttbc_file_format = 46;
pub const TTBC_FILE_FORMAT_PROGRAM_DATA: ttbc_file_format = 39;
pub const TTBC_FILE_FORMAT_PK: ttbc_file_format = 1;
pub const TTBC_FILE_FORMAT_PICT: ttbc_file_format = 25;
pub const TTBC_FILE_FORMAT_OVF: ttbc_file_format = 23;
pub const TTBC_FILE_FORMAT_OPEN_TYPE: ttbc_file_format = 47;
pub const TTBC_FILE_FORMAT_OFM: ttbc_file_format = 20;
pub const TTBC_FILE_FORMAT_MISC_FONTS: ttbc_file_format = 41;
pub const TTBC_FILE_FORMAT_FONT_MAP: ttbc_file_format = 11;
pub const TTBC_FILE_FORMAT_FORMAT: ttbc_file_format = 10;
pub const TTBC_FILE_FORMAT_ENC: ttbc_file_format = 44;
pub const TTBC_FILE_FORMAT_CNF: ttbc_file_format = 8;
pub const TTBC_FILE_FORMAT_CMAP: ttbc_file_format = 45;
pub const TTBC_FILE_FORMAT_BST: ttbc_file_format = 7;
pub const TTBC_FILE_FORMAT_BIB: ttbc_file_format = 6;
pub const TTBC_FILE_FORMAT_AFM: ttbc_file_format = 4;
pub type C2RustUnnamed = ::core::ffi::c_uint;
pub const DIAG_ERROR: C2RustUnnamed = 2;
pub const DIAG_WARN: C2RustUnnamed = 1;
pub const DIAG_NONE: C2RustUnnamed = 0;
pub type txp_file_id = int32_t;
pub type txp_open_mode = ::core::ffi::c_uint;
pub const TXP_WRITE: txp_open_mode = 1;
pub const TXP_READ: txp_open_mode = 0;
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
pub struct txp_input {
    pub id: txp_file_id,
    pub c2rust_unnamed: C2RustUnnamed_0,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union C2RustUnnamed_0 {
    pub file: *mut FILE,
    pub c2rust_unnamed: C2RustUnnamed_1,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_1 {
    pub file_size: ::core::ffi::c_int,
    pub file_pos: ::core::ffi::c_int,
    pub generation: ::core::ffi::c_int,
    pub buf_pos: ::core::ffi::c_int,
    pub buf_len: ::core::ffi::c_int,
    pub buffer: [uint8_t; 1024],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct txp_input_file {
    pub id: txp_file_id,
    pub file: *mut FILE,
}
pub type tt_history_t = ::core::ffi::c_uint;
pub const HISTORY_FATAL_ERROR: tt_history_t = 3;
pub const HISTORY_ERROR_ISSUED: tt_history_t = 2;
pub const HISTORY_WARNING_ISSUED: tt_history_t = 1;
pub const HISTORY_SPOTLESS: tt_history_t = 0;
pub const NULL: *mut ::core::ffi::c_void = __DARWIN_NULL;
pub const __DARWIN_NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
pub const SEEK_CUR: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const SEEK_END: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const EOF: ::core::ffi::c_int = -(1 as ::core::ffi::c_int);
pub const UINT64_MAX: ::core::ffi::c_ulonglong = 18446744073709551615 as ::core::ffi::c_ulonglong;
pub const O_RDWR: ::core::ffi::c_int = 0x2 as ::core::ffi::c_int;
pub const O_CREAT: ::core::ffi::c_int = 0x200 as ::core::ffi::c_int;
pub const LOCK_EX: ::core::ffi::c_int = 0x2 as ::core::ffi::c_int;
pub const LOCK_UN: ::core::ffi::c_int = 0x8 as ::core::ffi::c_int;
pub const R_OK: ::core::ffi::c_int = (1 as ::core::ffi::c_int) << 2 as ::core::ffi::c_int;
pub const true_0: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const false_0: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const TECTONIC_FORMAT_VERSION: ::core::ffi::c_int = 36 as ::core::ffi::c_int;
pub const LOG: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
#[no_mangle]
pub static mut primary_document: *const ::core::ffi::c_char =
    ::core::ptr::null::<::core::ffi::c_char>();
#[no_mangle]
pub static mut last_open: [::core::ffi::c_char; 1025] = [0; 1025];
#[no_mangle]
pub static mut use_texpresso: bool = 0 as ::core::ffi::c_int != 0;
#[no_mangle]
pub static mut texpresso: *mut txp_client = ::core::ptr::null::<txp_client>() as *mut txp_client;
#[no_mangle]
pub static mut regenerate_format: bool = 0 as ::core::ffi::c_int != 0;
#[no_mangle]
pub static mut dependency_tape: *mut FILE = ::core::ptr::null::<FILE>() as *mut FILE;
#[no_mangle]
pub static mut format_name: *const ::core::ffi::c_char = ::core::ptr::null::<::core::ffi::c_char>();
static mut format_lock_fd: ::core::ffi::c_int = -(1 as ::core::ffi::c_int);
unsafe extern "C" fn lock_format() {
    let mut fmt: *const ::core::ffi::c_char =
        format_path(b".fmt\0" as *const u8 as *const ::core::ffi::c_char);
    if fmt.is_null() {
        return;
    }
    let mut lock: [::core::ffi::c_char; 1025] = [0; 1025];
    snprintf(
        &raw mut lock as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 1025]>() as size_t,
        b"%s.lock\0" as *const u8 as *const ::core::ffi::c_char,
        fmt,
    );
    format_lock_fd = open(
        &raw mut lock as *mut ::core::ffi::c_char,
        O_CREAT | O_RDWR,
        0o600 as ::core::ffi::c_int,
    );
    if format_lock_fd < 0 as ::core::ffi::c_int {
        return;
    }
    if flock(format_lock_fd, LOCK_EX) != 0 as ::core::ffi::c_int {
        close(format_lock_fd);
        format_lock_fd = -(1 as ::core::ffi::c_int);
    }
}
unsafe extern "C" fn unlock_format() {
    if format_lock_fd >= 0 as ::core::ffi::c_int {
        flock(format_lock_fd, LOCK_UN);
        close(format_lock_fd);
        format_lock_fd = -(1 as ::core::ffi::c_int);
    }
}
static mut pending_fmt: *mut FILE = ::core::ptr::null::<FILE>() as *mut FILE;
static mut pending_fmt_path: [::core::ffi::c_char; 1025] = [0; 1025];
pub const FORMAT_BUF_SIZE: ::core::ffi::c_int = 4096 as ::core::ffi::c_int;
#[no_mangle]
pub static mut format_buf: [::core::ffi::c_char; 4096] = [0; 4096];
#[no_mangle]
pub unsafe extern "C" fn _tt_abort(mut format: *const ::core::ffi::c_char, mut args: ...) -> ! {
    fprintf(
        __stderrp,
        b"Fatal error, aborting: \0" as *const u8 as *const ::core::ffi::c_char,
    );
    let mut ap: ::core::ffi::VaList<'_>;
    ap = args.clone();
    vfprintf(__stderrp, format, ap.clone());
    fprintf(
        __stderrp,
        b"\n\0" as *const u8 as *const ::core::ffi::c_char,
    );
    print_backtrace();
    abort();
}
#[no_mangle]
pub unsafe extern "C" fn ttstub_shell_escape(
    mut cmd: *const ::core::ffi::c_ushort,
    mut len: size_t,
) -> ::core::ffi::c_int {
    if logging != 0 {
        fprintf(
            __stderrp,
            b"%s(%.*s, %d)\n\0" as *const u8 as *const ::core::ffi::c_char,
            b"ttstub_shell_escape\0" as *const u8 as *const ::core::ffi::c_char,
            len as ::core::ffi::c_int,
            cmd as *mut ::core::ffi::c_char,
            len as ::core::ffi::c_int,
        );
    }
    return 0 as ::core::ffi::c_int;
}
unsafe extern "C" fn input_as_file(mut h: *mut ttbc_input_handle_t) -> *mut FILE {
    if !texpresso.is_null() {
        abort();
    }
    return h as *mut ::core::ffi::c_void as *mut FILE;
}
unsafe extern "C" fn file_as_input(mut h: *mut FILE) -> *mut ttbc_input_handle_t {
    if !texpresso.is_null() {
        abort();
    }
    return h as *mut ::core::ffi::c_void as *mut ttbc_input_handle_t;
}
#[no_mangle]
pub static mut allocated: [uint64_t; 16] = [0; 16];
unsafe extern "C" fn next_id() -> txp_file_id {
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < 16 as ::core::ffi::c_int {
        if allocated[i as usize] != UINT64_MAX as uint64_t {
            let mut j: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
            while j < 64 as ::core::ffi::c_int {
                if allocated[i as usize] & (1 as ::core::ffi::c_int as uint64_t) << j
                    == 0 as uint64_t
                {
                    return (i as txp_file_id) << 6 as ::core::ffi::c_int | j as txp_file_id;
                }
                j += 1;
            }
        }
        i += 1;
    }
    fprintf(
        __stderrp,
        b"All ids have been allocated\n\0" as *const u8 as *const ::core::ffi::c_char,
    );
    abort();
}
unsafe extern "C" fn alloc_id(mut id: txp_file_id) {
    let mut cell: ::core::ffi::c_int = id as ::core::ffi::c_int >> 6 as ::core::ffi::c_int;
    if cell < 0 as ::core::ffi::c_int || cell >= 16 as ::core::ffi::c_int {
        abort();
    }
    let mut offset: ::core::ffi::c_int = id as ::core::ffi::c_int & 63 as ::core::ffi::c_int;
    if allocated[cell as usize] & (1 as ::core::ffi::c_int as uint64_t) << offset != 0 as uint64_t {
        abort();
    }
    allocated[cell as usize] |= (1 as ::core::ffi::c_int as uint64_t) << offset;
}
unsafe extern "C" fn release_id(mut id: txp_file_id) {
    let mut cell: ::core::ffi::c_int = id as ::core::ffi::c_int >> 6 as ::core::ffi::c_int;
    if cell < 0 as ::core::ffi::c_int || cell >= 16 as ::core::ffi::c_int {
        abort();
    }
    let mut offset: ::core::ffi::c_int = id as ::core::ffi::c_int & 63 as ::core::ffi::c_int;
    if allocated[cell as usize] & ((1 as ::core::ffi::c_int) << offset) as uint64_t == 0 as uint64_t
    {
        abort();
    }
    allocated[cell as usize] &= !((1 as ::core::ffi::c_int as uint64_t) << offset);
}
unsafe extern "C" fn input_as_txp(mut h: *mut ttbc_input_handle_t) -> *mut txp_input {
    if texpresso.is_null() {
        abort();
    }
    return h as *mut ::core::ffi::c_void as *mut txp_input;
}
unsafe extern "C" fn txp_as_input(mut h: *mut txp_input) -> *mut ttbc_input_handle_t {
    if texpresso.is_null() {
        abort();
    }
    return h as *mut ::core::ffi::c_void as *mut ttbc_input_handle_t;
}
unsafe extern "C" fn kind_of_ttbc_format(mut format: ttbc_file_format) -> txp_file_kind {
    match format as ::core::ffi::c_uint {
        4 => return TXP_KIND_AFM,
        6 => return TXP_KIND_BIB,
        7 => return TXP_KIND_BST,
        45 => return TXP_KIND_CMAP,
        8 => return TXP_KIND_CNF,
        44 => return TXP_KIND_ENC,
        10 => return TXP_KIND_FORMAT,
        11 => return TXP_KIND_FONT_MAP,
        41 => return TXP_KIND_MISC_FONTS,
        20 => return TXP_KIND_OFM,
        47 => return TXP_KIND_OPEN_TYPE,
        23 => return TXP_KIND_OVF,
        25 => return TXP_KIND_PICT,
        1 => return TXP_KIND_PK,
        39 => return TXP_KIND_PROGRAM_DATA,
        46 => return TXP_KIND_SFD,
        26 => return TXP_KIND_TEX,
        30 => return TXP_KIND_TEX_PS_HEADER,
        3 => return TXP_KIND_TFM,
        36 => return TXP_KIND_TRUE_TYPE,
        32 => return TXP_KIND_TYPE1,
        33 => return TXP_KIND_VF,
        59 => return TXP_KIND_PRIMARY,
        _ => {
            abort();
        }
    };
}
#[no_mangle]
pub unsafe extern "C" fn ttstub_input_open(
    mut path: *const ::core::ffi::c_char,
    mut format: ttbc_file_format,
    mut is_gz: ::core::ffi::c_int,
) -> rust_input_handle_t {
    if logging != 0 {
        fprintf(
            __stderrp,
            b"%s(path:%s, format:%s, is_gz:%d)\n\0" as *const u8 as *const ::core::ffi::c_char,
            b"ttstub_input_open\0" as *const u8 as *const ::core::ffi::c_char,
            path,
            ttbc_file_format_to_string(format),
            is_gz,
        );
    }
    if is_gz != 0 as ::core::ffi::c_int {
        fprintf(
            __stderrp,
            b"GZ compression not supported\n\0" as *const u8 as *const ::core::ffi::c_char,
        );
        print_backtrace();
        abort();
    }
    if !texpresso.is_null()
        && format as ::core::ffi::c_uint
            == TTBC_FILE_FORMAT_FORMAT as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        let mut cached: *const ::core::ffi::c_char =
            format_path(b".fmt\0" as *const u8 as *const ::core::ffi::c_char);
        if cached.is_null() {
            return ::core::ptr::null_mut::<ttbc_input_handle_t>();
        }
        let mut f: *mut FILE =
            fopen(cached, b"rb\0" as *const u8 as *const ::core::ffi::c_char) as *mut FILE;
        if f.is_null() {
            return ::core::ptr::null_mut::<ttbc_input_handle_t>();
        }
        strcpy(&raw mut last_open as *mut ::core::ffi::c_char, cached);
        let mut input: *mut txp_input_file = calloc(
            1 as size_t,
            ::core::mem::size_of::<txp_input_file>() as size_t,
        ) as *mut txp_input_file;
        if input.is_null() {
            abort();
        }
        (*input).id = -(1 as ::core::ffi::c_int) as txp_file_id;
        (*input).file = f;
        return txp_as_input(input as *mut txp_input) as rust_input_handle_t;
    }
    if !texpresso.is_null() {
        let mut id: txp_file_id = next_id();
        let mut ipath: *mut ::core::ffi::c_char =
            txp_open(texpresso, id, path, kind_of_ttbc_format(format), TXP_READ);
        if !ipath.is_null() {
            strcpy(&raw mut last_open as *mut ::core::ffi::c_char, ipath);
            free(ipath as *mut ::core::ffi::c_void);
            let mut input_0: *mut txp_input =
                calloc(1 as size_t, ::core::mem::size_of::<txp_input>() as size_t)
                    as *mut txp_input;
            if input_0.is_null() {
                abort();
            }
            alloc_id(id);
            (*input_0).id = id;
            (*input_0).c2rust_unnamed.c2rust_unnamed.file_size = -(1 as ::core::ffi::c_int);
            (*input_0).c2rust_unnamed.c2rust_unnamed.generation =
                txp_generation(texpresso) as ::core::ffi::c_int;
            return txp_as_input(input_0) as rust_input_handle_t;
        }
    }
    let mut f_0: *mut FILE =
        fopen(path, b"rb\0" as *const u8 as *const ::core::ffi::c_char) as *mut FILE;
    let mut buffer: *const ::core::ffi::c_void = ::core::ptr::null::<::core::ffi::c_void>();
    let mut len: size_t = 0 as size_t;
    let mut tmp: [::core::ffi::c_char; 1025] = [0; 1025];
    if f_0.is_null()
        && format as ::core::ffi::c_uint
            == TTBC_FILE_FORMAT_FORMAT as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        let mut cached_0: *const ::core::ffi::c_char =
            format_path(b".fmt\0" as *const u8 as *const ::core::ffi::c_char);
        if !cached_0.is_null() {
            f_0 = fopen(cached_0, b"rb\0" as *const u8 as *const ::core::ffi::c_char) as *mut FILE;
        }
    }
    if !f_0.is_null() {
        strcpy(&raw mut last_open as *mut ::core::ffi::c_char, path);
    }
    if f_0.is_null() {
        let mut exts: *mut *const ::core::ffi::c_char = format_extensions(format);
        while f_0.is_null() && !(*exts).is_null() {
            strcpy(&raw mut tmp as *mut ::core::ffi::c_char, path);
            strcat(&raw mut tmp as *mut ::core::ffi::c_char, *exts);
            f_0 = fopen(
                &raw mut tmp as *mut ::core::ffi::c_char,
                b"rb\0" as *const u8 as *const ::core::ffi::c_char,
            ) as *mut FILE;
            if !f_0.is_null() {
                strcpy(
                    &raw mut last_open as *mut ::core::ffi::c_char,
                    &raw mut tmp as *mut ::core::ffi::c_char,
                );
            }
            exts = exts.offset(1);
        }
    }
    if f_0.is_null() {
        let mut tlpath: *const ::core::ffi::c_char = texlive_file_path(path, dependency_tape);
        let mut exts_0: *mut *const ::core::ffi::c_char = format_extensions(format);
        while tlpath.is_null() && !(*exts_0).is_null() {
            strcpy(&raw mut tmp as *mut ::core::ffi::c_char, path);
            strcat(&raw mut tmp as *mut ::core::ffi::c_char, *exts_0);
            tlpath = texlive_file_path(
                &raw mut tmp as *mut ::core::ffi::c_char,
                ::core::ptr::null_mut::<FILE>(),
            );
            exts_0 = exts_0.offset(1);
        }
        if !tlpath.is_null() {
            strcpy(&raw mut last_open as *mut ::core::ffi::c_char, tlpath);
            f_0 = fopen(tlpath, b"rb\0" as *const u8 as *const ::core::ffi::c_char) as *mut FILE;
        }
    }
    if !texpresso.is_null() {
        if f_0.is_null() {
            return ::core::ptr::null_mut::<ttbc_input_handle_t>();
        }
        let mut input_1: *mut txp_input_file = calloc(
            1 as size_t,
            ::core::mem::size_of::<txp_input_file>() as size_t,
        ) as *mut txp_input_file;
        if input_1.is_null() {
            abort();
        }
        (*input_1).id = -(1 as ::core::ffi::c_int) as txp_file_id;
        (*input_1).file = f_0;
        return txp_as_input(input_1 as *mut txp_input) as rust_input_handle_t;
    } else {
        return file_as_input(f_0) as rust_input_handle_t;
    };
}
#[no_mangle]
pub unsafe extern "C" fn ttstub_input_open_primary() -> rust_input_handle_t {
    if logging != 0 {
        fprintf(
            __stderrp,
            b"%s()\n\0" as *const u8 as *const ::core::ffi::c_char,
            b"ttstub_input_open_primary\0" as *const u8 as *const ::core::ffi::c_char,
        );
    }
    if primary_document.is_null() {
        fprintf(
            __stderrp,
            b"Document name as not been specified\n\0" as *const u8 as *const ::core::ffi::c_char,
        );
        print_backtrace();
        abort();
    }
    return ttstub_input_open(
        primary_document,
        TTBC_FILE_FORMAT_TEX,
        0 as ::core::ffi::c_int,
    );
}
#[no_mangle]
pub unsafe extern "C" fn ttstub_input_close(
    mut handle: *mut ttbc_input_handle_t,
) -> ::core::ffi::c_int {
    if !texpresso.is_null() {
        let mut input: *mut txp_input = input_as_txp(handle);
        if (*input).id == -(1 as txp_file_id) {
            let mut result: ::core::ffi::c_int = fclose((*input).c2rust_unnamed.file);
            free(input as *mut ::core::ffi::c_void);
            return result;
        }
        txp_close(texpresso, (*input).id);
        release_id((*input).id);
        free(input as *mut ::core::ffi::c_void);
        return 0 as ::core::ffi::c_int;
    }
    return fclose(input_as_file(handle));
}
#[no_mangle]
pub unsafe extern "C" fn ttstub_input_getc(
    mut handle: *mut ttbc_input_handle_t,
) -> ::core::ffi::c_int {
    if !texpresso.is_null() {
        let mut input: *mut txp_input = input_as_txp(handle);
        if (*input).id == -(1 as txp_file_id) {
            return getc((*input).c2rust_unnamed.file);
        }
        if (*input).c2rust_unnamed.c2rust_unnamed.generation as uint32_t
            != txp_generation(texpresso)
        {
            (*input).c2rust_unnamed.c2rust_unnamed.generation =
                txp_generation(texpresso) as ::core::ffi::c_int;
            (*input).c2rust_unnamed.c2rust_unnamed.file_pos +=
                (*input).c2rust_unnamed.c2rust_unnamed.buf_pos;
            (*input).c2rust_unnamed.c2rust_unnamed.buf_len = 0 as ::core::ffi::c_int;
            (*input).c2rust_unnamed.c2rust_unnamed.buf_pos =
                (*input).c2rust_unnamed.c2rust_unnamed.buf_len;
        }
        if (*input).c2rust_unnamed.c2rust_unnamed.buf_pos
            >= (*input).c2rust_unnamed.c2rust_unnamed.buf_len
        {
            (*input).c2rust_unnamed.c2rust_unnamed.file_pos +=
                (*input).c2rust_unnamed.c2rust_unnamed.buf_len;
            (*input).c2rust_unnamed.c2rust_unnamed.buf_pos = 0 as ::core::ffi::c_int;
            (*input).c2rust_unnamed.c2rust_unnamed.buf_len = txp_read(
                texpresso,
                (*input).id,
                (*input).c2rust_unnamed.c2rust_unnamed.file_pos as uint32_t,
                &raw mut (*input).c2rust_unnamed.c2rust_unnamed.buffer as *mut uint8_t
                    as *mut ::core::ffi::c_void,
                ::core::mem::size_of::<[uint8_t; 1024]>() as size_t,
            ) as ::core::ffi::c_int;
            if (*input).c2rust_unnamed.c2rust_unnamed.buf_len == 0 as ::core::ffi::c_int {
                return EOF;
            }
        }
        let fresh2 = (*input).c2rust_unnamed.c2rust_unnamed.buf_pos;
        (*input).c2rust_unnamed.c2rust_unnamed.buf_pos =
            (*input).c2rust_unnamed.c2rust_unnamed.buf_pos + 1;
        let mut result: ::core::ffi::c_int =
            (*input).c2rust_unnamed.c2rust_unnamed.buffer[fresh2 as usize] as ::core::ffi::c_int;
        txp_seen(
            texpresso,
            (*input).id,
            ((*input).c2rust_unnamed.c2rust_unnamed.file_pos
                + (*input).c2rust_unnamed.c2rust_unnamed.buf_pos) as uint32_t,
        );
        return result;
    }
    return getc(input_as_file(handle));
}
#[no_mangle]
pub unsafe extern "C" fn ttstub_input_get_mtime(mut handle: *mut ttbc_input_handle_t) -> time_t {
    let mut file_stat: stat = stat {
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
    if !texpresso.is_null() {
        let mut input: *mut txp_input = input_as_txp(handle);
        if (*input).id == -(1 as txp_file_id) {
            if fstat(fileno((*input).c2rust_unnamed.file), &raw mut file_stat)
                == -(1 as ::core::ffi::c_int)
            {
                return 0 as time_t;
            }
            return file_stat.st_mtimespec.tv_sec as time_t;
        }
        return txp_mtime(texpresso, (*input).id) as time_t;
    }
    if fstat(fileno(input_as_file(handle)), &raw mut file_stat) == -(1 as ::core::ffi::c_int) {
        return 0 as time_t;
    }
    return file_stat.st_mtimespec.tv_sec as time_t;
}
#[no_mangle]
pub unsafe extern "C" fn txp_input_size(mut input: *mut txp_input) -> uint32_t {
    if (*input).c2rust_unnamed.c2rust_unnamed.file_size == -(1 as ::core::ffi::c_int) {
        (*input).c2rust_unnamed.c2rust_unnamed.file_size =
            txp_size(texpresso, (*input).id) as ::core::ffi::c_int;
    }
    return (*input).c2rust_unnamed.c2rust_unnamed.file_size as uint32_t;
}
#[no_mangle]
pub unsafe extern "C" fn ttstub_input_get_size(mut handle: *mut ttbc_input_handle_t) -> size_t {
    let mut file_stat: stat = stat {
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
    if !texpresso.is_null() {
        let mut input: *mut txp_input = input_as_txp(handle);
        if (*input).id == -(1 as txp_file_id) {
            if fstat(fileno((*input).c2rust_unnamed.file), &raw mut file_stat)
                == -(1 as ::core::ffi::c_int)
            {
                return 0 as size_t;
            }
            return file_stat.st_size as size_t;
        }
        return txp_input_size(input) as size_t;
    }
    if fstat(fileno(input_as_file(handle)), &raw mut file_stat) == -(1 as ::core::ffi::c_int) {
        return 0 as size_t;
    }
    return file_stat.st_size as size_t;
}
#[no_mangle]
pub unsafe extern "C" fn ttstub_input_seek(
    mut handle: *mut ttbc_input_handle_t,
    mut offset: ssize_t,
    mut whence: ::core::ffi::c_int,
) -> size_t {
    if !texpresso.is_null() {
        let mut input: *mut txp_input = input_as_txp(handle);
        if (*input).id == -(1 as txp_file_id) {
            return fseek(
                (*input).c2rust_unnamed.file,
                offset as ::core::ffi::c_long,
                whence,
            ) as size_t;
        }
        let mut ofs: ::core::ffi::c_int = offset as ::core::ffi::c_int;
        if whence == SEEK_CUR {
            ofs += (*input).c2rust_unnamed.c2rust_unnamed.file_pos
                + (*input).c2rust_unnamed.c2rust_unnamed.buf_pos;
        } else if whence == SEEK_END {
            ofs = (ofs as uint32_t).wrapping_add(txp_input_size(input)) as ::core::ffi::c_int
                as ::core::ffi::c_int;
        }
        if ofs < 0 as ::core::ffi::c_int {
            return -(1 as ::core::ffi::c_int) as size_t;
        }
        let mut buf_pos: ::core::ffi::c_int = ofs - (*input).c2rust_unnamed.c2rust_unnamed.file_pos;
        if buf_pos > 0 as ::core::ffi::c_int
            && buf_pos <= (*input).c2rust_unnamed.c2rust_unnamed.buf_len
        {
            (*input).c2rust_unnamed.c2rust_unnamed.buf_pos = buf_pos;
        } else {
            (*input).c2rust_unnamed.c2rust_unnamed.file_pos = ofs;
            (*input).c2rust_unnamed.c2rust_unnamed.buf_len = 0 as ::core::ffi::c_int;
            (*input).c2rust_unnamed.c2rust_unnamed.buf_pos =
                (*input).c2rust_unnamed.c2rust_unnamed.buf_len;
        }
        return ((*input).c2rust_unnamed.c2rust_unnamed.file_pos
            + (*input).c2rust_unnamed.c2rust_unnamed.buf_pos) as size_t;
    }
    return fseek(input_as_file(handle), offset as ::core::ffi::c_long, whence) as size_t;
}
unsafe extern "C" fn internal_input_read(
    mut handle: *mut ttbc_input_handle_t,
    mut data: *mut ::core::ffi::c_char,
    mut len: size_t,
) -> ssize_t {
    if !texpresso.is_null() {
        let mut input: *mut txp_input = input_as_txp(handle);
        if (*input).id == -(1 as txp_file_id) {
            return fread(
                data as *mut ::core::ffi::c_void,
                1 as size_t,
                len,
                (*input).c2rust_unnamed.file,
            ) as ssize_t;
        }
        if (*input).c2rust_unnamed.c2rust_unnamed.generation as uint32_t
            != txp_generation(texpresso)
        {
            (*input).c2rust_unnamed.c2rust_unnamed.generation =
                txp_generation(texpresso) as ::core::ffi::c_int;
            (*input).c2rust_unnamed.c2rust_unnamed.file_pos +=
                (*input).c2rust_unnamed.c2rust_unnamed.buf_pos;
            (*input).c2rust_unnamed.c2rust_unnamed.buf_len = 0 as ::core::ffi::c_int;
            (*input).c2rust_unnamed.c2rust_unnamed.buf_pos =
                (*input).c2rust_unnamed.c2rust_unnamed.buf_len;
        }
        if (*input).c2rust_unnamed.c2rust_unnamed.buf_pos
            < (*input).c2rust_unnamed.c2rust_unnamed.buf_len
        {
            if len
                >= ((*input).c2rust_unnamed.c2rust_unnamed.buf_len
                    - (*input).c2rust_unnamed.c2rust_unnamed.buf_pos) as size_t
            {
                len = ((*input).c2rust_unnamed.c2rust_unnamed.buf_len
                    - (*input).c2rust_unnamed.c2rust_unnamed.buf_pos)
                    as size_t;
            }
            memmove(
                data as *mut ::core::ffi::c_void,
                (&raw mut (*input).c2rust_unnamed.c2rust_unnamed.buffer as *mut uint8_t)
                    .offset((*input).c2rust_unnamed.c2rust_unnamed.buf_pos as isize)
                    as *const ::core::ffi::c_void,
                len,
            );
            (*input).c2rust_unnamed.c2rust_unnamed.buf_pos =
                ((*input).c2rust_unnamed.c2rust_unnamed.buf_pos as size_t).wrapping_add(len)
                    as ::core::ffi::c_int as ::core::ffi::c_int;
            txp_seen(
                texpresso,
                (*input).id,
                ((*input).c2rust_unnamed.c2rust_unnamed.file_pos
                    + (*input).c2rust_unnamed.c2rust_unnamed.buf_pos) as uint32_t,
            );
            return len as ssize_t;
        }
        if len < ::core::mem::size_of::<[uint8_t; 1024]>() as usize {
            (*input).c2rust_unnamed.c2rust_unnamed.file_pos =
                (*input).c2rust_unnamed.c2rust_unnamed.file_pos
                    + (*input).c2rust_unnamed.c2rust_unnamed.buf_len;
            (*input).c2rust_unnamed.c2rust_unnamed.buf_len = txp_read(
                texpresso,
                (*input).id,
                (*input).c2rust_unnamed.c2rust_unnamed.file_pos as uint32_t,
                &raw mut (*input).c2rust_unnamed.c2rust_unnamed.buffer as *mut uint8_t
                    as *mut ::core::ffi::c_void,
                len,
            ) as ::core::ffi::c_int;
            if len > (*input).c2rust_unnamed.c2rust_unnamed.buf_len as size_t {
                len = (*input).c2rust_unnamed.c2rust_unnamed.buf_len as size_t;
            }
            memmove(
                data as *mut ::core::ffi::c_void,
                &raw mut (*input).c2rust_unnamed.c2rust_unnamed.buffer as *mut uint8_t
                    as *const ::core::ffi::c_void,
                len,
            );
            (*input).c2rust_unnamed.c2rust_unnamed.buf_pos = len as ::core::ffi::c_int;
            txp_seen(
                texpresso,
                (*input).id,
                ((*input).c2rust_unnamed.c2rust_unnamed.file_pos
                    + (*input).c2rust_unnamed.c2rust_unnamed.buf_pos) as uint32_t,
            );
            return len as ssize_t;
        } else {
            (*input).c2rust_unnamed.c2rust_unnamed.file_pos =
                (*input).c2rust_unnamed.c2rust_unnamed.file_pos
                    + (*input).c2rust_unnamed.c2rust_unnamed.buf_len;
            len = txp_read(
                texpresso,
                (*input).id,
                (*input).c2rust_unnamed.c2rust_unnamed.file_pos as uint32_t,
                data as *mut ::core::ffi::c_void,
                len,
            );
            (*input).c2rust_unnamed.c2rust_unnamed.file_pos =
                ((*input).c2rust_unnamed.c2rust_unnamed.file_pos as size_t).wrapping_add(len)
                    as ::core::ffi::c_int as ::core::ffi::c_int;
            (*input).c2rust_unnamed.c2rust_unnamed.buf_len = 0 as ::core::ffi::c_int;
            (*input).c2rust_unnamed.c2rust_unnamed.buf_pos =
                (*input).c2rust_unnamed.c2rust_unnamed.buf_len;
            txp_seen(
                texpresso,
                (*input).id,
                ((*input).c2rust_unnamed.c2rust_unnamed.file_pos
                    + (*input).c2rust_unnamed.c2rust_unnamed.buf_pos) as uint32_t,
            );
            return len as ssize_t;
        }
    }
    return fread(
        data as *mut ::core::ffi::c_void,
        1 as size_t,
        len,
        input_as_file(handle),
    ) as ssize_t;
}
#[no_mangle]
pub unsafe extern "C" fn ttstub_input_read(
    mut handle: *mut ttbc_input_handle_t,
    mut data: *mut ::core::ffi::c_char,
    mut len: size_t,
) -> ssize_t {
    let mut result: ssize_t = internal_input_read(handle, data, len);
    if result <= 0 as ssize_t {
        return result;
    }
    while result as size_t <= len {
        let mut delta: ssize_t = internal_input_read(
            handle,
            data.offset(result as isize),
            len.wrapping_sub(result as size_t),
        );
        if delta <= 0 as ssize_t {
            return result;
        }
        result += delta;
    }
    return result;
}
#[no_mangle]
pub unsafe extern "C" fn ttstub_input_ungetc(
    mut handle: *mut ttbc_input_handle_t,
    mut ch: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    if !texpresso.is_null() {
        let mut input: *mut txp_input = input_as_txp(handle);
        if (*input).id == -(1 as txp_file_id) {
            return ungetc(ch, (*input).c2rust_unnamed.file);
        }
        if (*input).c2rust_unnamed.c2rust_unnamed.buf_pos == 0 as ::core::ffi::c_int {
            return EOF;
        }
        (*input).c2rust_unnamed.c2rust_unnamed.buf_pos -= 1;
        (*input).c2rust_unnamed.c2rust_unnamed.buffer
            [(*input).c2rust_unnamed.c2rust_unnamed.buf_pos as usize] = ch as uint8_t;
        return ch;
    }
    return ungetc(ch, input_as_file(handle));
}
#[no_mangle]
pub unsafe extern "C" fn ttstub_get_last_input_abspath(
    mut buffer: *mut ::core::ffi::c_char,
    mut len: size_t,
) -> ssize_t {
    let mut llen: size_t = strlen(&raw mut last_open as *mut ::core::ffi::c_char);
    memmove(
        buffer as *mut ::core::ffi::c_void,
        &raw mut last_open as *mut ::core::ffi::c_char as *const ::core::ffi::c_void,
        if llen > len { len } else { llen },
    );
    if llen < len {
        *buffer.offset(llen as isize) = '\0' as i32 as ::core::ffi::c_char;
    }
    return llen as ssize_t;
}
unsafe extern "C" fn output_as_file(mut h: *mut ttbc_output_handle_t) -> *mut FILE {
    if !texpresso.is_null() {
        abort();
    }
    return h as *mut ::core::ffi::c_void as *mut FILE;
}
unsafe extern "C" fn file_as_output(mut h: *mut FILE) -> *mut ttbc_output_handle_t {
    if !texpresso.is_null() {
        abort();
    }
    return h as *mut ::core::ffi::c_void as *mut ttbc_output_handle_t;
}
unsafe extern "C" fn output_as_txp(mut p: *mut ttbc_output_handle_t) -> txp_file_id {
    let mut h: uintptr_t = p as uintptr_t;
    if texpresso.is_null()
        || h > 1024 as ::core::ffi::c_int as uintptr_t
            && h != -(1 as ::core::ffi::c_int) as uintptr_t
    {
        abort();
    }
    return h as txp_file_id;
}
unsafe extern "C" fn txp_as_output(mut h: txp_file_id) -> *mut ttbc_output_handle_t {
    let mut p: uintptr_t = h as uintptr_t;
    if texpresso.is_null() {
        abort();
    }
    return p as *mut ::core::ffi::c_void as *mut ttbc_output_handle_t;
}
#[no_mangle]
pub unsafe extern "C" fn ttstub_output_flush(
    mut handle: *mut ttbc_output_handle_t,
) -> ::core::ffi::c_int {
    if !texpresso.is_null() {
        txp_flush(texpresso);
        return 0 as ::core::ffi::c_int;
    } else {
        return fflush(output_as_file(handle));
    };
}
#[no_mangle]
pub unsafe extern "C" fn ttstub_output_close(
    mut handle: *mut ttbc_output_handle_t,
) -> ::core::ffi::c_int {
    if !texpresso.is_null() {
        txp_close(texpresso, output_as_txp(handle));
        return 0 as ::core::ffi::c_int;
    }
    let mut f: *mut FILE = output_as_file(handle);
    if !pending_fmt.is_null() && f == pending_fmt {
        pending_fmt = ::core::ptr::null_mut::<FILE>();
        if fclose(f) != 0 as ::core::ffi::c_int {
            unlink(&raw mut pending_fmt_path as *mut ::core::ffi::c_char);
            return EOF;
        }
        let mut final_0: [::core::ffi::c_char; 1025] = [0; 1025];
        let mut n: size_t = strlen(&raw mut pending_fmt_path as *mut ::core::ffi::c_char);
        if n <= 4 as size_t
            || n.wrapping_sub(4 as size_t)
                >= ::core::mem::size_of::<[::core::ffi::c_char; 1025]>() as usize
        {
            unlink(&raw mut pending_fmt_path as *mut ::core::ffi::c_char);
            return EOF;
        }
        memcpy(
            &raw mut final_0 as *mut ::core::ffi::c_char as *mut ::core::ffi::c_void,
            &raw mut pending_fmt_path as *mut ::core::ffi::c_char as *const ::core::ffi::c_void,
            n.wrapping_sub(4 as size_t),
        );
        final_0[n.wrapping_sub(4 as size_t) as usize] = 0 as ::core::ffi::c_char;
        if rename(
            &raw mut pending_fmt_path as *mut ::core::ffi::c_char,
            &raw mut final_0 as *mut ::core::ffi::c_char,
        ) != 0 as ::core::ffi::c_int
        {
            unlink(&raw mut pending_fmt_path as *mut ::core::ffi::c_char);
            return EOF;
        }
        return 0 as ::core::ffi::c_int;
    }
    return fclose(f);
}
#[no_mangle]
pub unsafe extern "C" fn ttstub_output_open(
    mut path: *const ::core::ffi::c_char,
    mut is_gz: ::core::ffi::c_int,
) -> rust_output_handle_t {
    if logging != 0 {
        fprintf(
            __stderrp,
            b"%s(path:%s, is_gz:%d)\n\0" as *const u8 as *const ::core::ffi::c_char,
            b"ttstub_output_open\0" as *const u8 as *const ::core::ffi::c_char,
            path,
            is_gz,
        );
    }
    if is_gz != 0 {
        fprintf(
            __stderrp,
            b"is_gz not supported\n\0" as *const u8 as *const ::core::ffi::c_char,
        );
        print_backtrace();
        abort();
    }
    if !texpresso.is_null() {
        let mut id: txp_file_id = next_id();
        let mut opath: *mut ::core::ffi::c_char =
            txp_open(texpresso, id, path, TXP_KIND_OTHER, TXP_WRITE);
        if opath.is_null() {
            return ::core::ptr::null_mut::<ttbc_output_handle_t>();
        }
        free(opath as *mut ::core::ffi::c_void);
        alloc_id(id);
        return txp_as_output(id) as rust_output_handle_t;
    }
    if in_initex_mode as ::core::ffi::c_int != 0 && !path.is_null() {
        let mut p: *const ::core::ffi::c_char = path;
        while *p as ::core::ffi::c_int != 0 && *p as ::core::ffi::c_int != '/' as i32 {
            p = p.offset(1);
        }
        if *p == 0 {
            path = format_path(path);
        }
    }
    return file_as_output(
        fopen(path, b"wb\0" as *const u8 as *const ::core::ffi::c_char) as *mut FILE,
    ) as rust_output_handle_t;
}
#[no_mangle]
pub unsafe extern "C" fn ttstub_output_open_format(
    mut path: *const ::core::ffi::c_char,
    mut is_gz: ::core::ffi::c_int,
) -> rust_output_handle_t {
    if logging != 0 {
        fprintf(
            __stderrp,
            b"%s(path:%s, is_gz:%d)\n\0" as *const u8 as *const ::core::ffi::c_char,
            b"ttstub_output_open_format\0" as *const u8 as *const ::core::ffi::c_char,
            path,
            is_gz,
        );
    }
    if !texpresso.is_null() || !in_initex_mode {
        abort();
    }
    path = format_path(b".fmt\0" as *const u8 as *const ::core::ffi::c_char);
    if path.is_null() {
        return ::core::ptr::null_mut::<ttbc_output_handle_t>();
    }
    snprintf(
        &raw mut pending_fmt_path as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 1025]>() as size_t,
        b"%s.tmp\0" as *const u8 as *const ::core::ffi::c_char,
        path,
    );
    pending_fmt = fopen(
        &raw mut pending_fmt_path as *mut ::core::ffi::c_char,
        b"wb\0" as *const u8 as *const ::core::ffi::c_char,
    ) as *mut FILE;
    return file_as_output(pending_fmt) as rust_output_handle_t;
}
#[no_mangle]
pub unsafe extern "C" fn ttstub_output_open_stdout() -> rust_output_handle_t {
    if !texpresso.is_null() {
        return -(1 as ::core::ffi::c_int) as uintptr_t as *mut ::core::ffi::c_void
            as rust_output_handle_t;
    }
    return file_as_output(__stdoutp) as rust_output_handle_t;
}
#[no_mangle]
pub unsafe extern "C" fn ttstub_output_putc(
    mut handle: *mut ttbc_output_handle_t,
    mut c: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    if !texpresso.is_null() {
        txp_putc(texpresso, output_as_txp(handle), c);
        return c;
    } else {
        return putc(c, output_as_file(handle));
    };
}
#[no_mangle]
pub unsafe extern "C" fn ttstub_output_write(
    mut handle: *mut ttbc_output_handle_t,
    mut data: *const ::core::ffi::c_char,
    mut len: size_t,
) -> size_t {
    if !texpresso.is_null() {
        txp_append(
            texpresso,
            output_as_txp(handle),
            data as *const ::core::ffi::c_void,
            len,
        );
        return len;
    } else {
        return fwrite(
            data as *const ::core::ffi::c_void,
            1 as size_t,
            len,
            output_as_file(handle),
        ) as size_t;
    };
}
#[no_mangle]
pub unsafe extern "C" fn ttstub_fprintf(
    mut handle: *mut ttbc_output_handle_t,
    mut format: *const ::core::ffi::c_char,
    mut args: ...
) -> ::core::ffi::c_int {
    let mut ap: ::core::ffi::VaList<'_>;
    ap = args.clone();
    let mut len: ::core::ffi::c_int = vsnprintf(
        &raw mut format_buf as *mut ::core::ffi::c_char,
        FORMAT_BUF_SIZE as size_t,
        format,
        ap.clone(),
    );
    return ttstub_output_write(
        handle,
        &raw mut format_buf as *mut ::core::ffi::c_char,
        len as size_t,
    ) as ::core::ffi::c_int;
}
#[no_mangle]
pub static mut diag_kind: C2RustUnnamed = DIAG_NONE;
#[no_mangle]
pub static mut diag_buf: [::core::ffi::c_char; 1024] = [0; 1024];
#[no_mangle]
pub static mut diag_len: size_t = 0 as size_t;
#[no_mangle]
pub static mut curr_diag: intptr_t = 0 as intptr_t;
unsafe extern "C" fn diag_write(
    mut diag: *mut ttbc_diagnostic_t,
    mut buf: *const ::core::ffi::c_char,
    mut len: ssize_t,
) {
    if diag as intptr_t != curr_diag {
        fprintf(
            __stderrp,
            b"diagnostic: use after free\n\0" as *const u8 as *const ::core::ffi::c_char,
        );
        print_backtrace();
        abort();
    }
    if diag_len.wrapping_add(len as size_t)
        > ::core::mem::size_of::<[::core::ffi::c_char; 1024]>() as usize
    {
        len = (::core::mem::size_of::<[::core::ffi::c_char; 1024]>() as usize)
            .wrapping_sub(diag_len as usize) as ssize_t;
    }
    memmove(
        (&raw mut diag_buf as *mut ::core::ffi::c_char).offset(diag_len as isize)
            as *mut ::core::ffi::c_void,
        buf as *const ::core::ffi::c_void,
        len as size_t,
    );
    diag_len = diag_len.wrapping_add(len as size_t);
}
#[no_mangle]
pub unsafe extern "C" fn ttbc_diag_append(
    mut diag: *mut ttbc_diagnostic_t,
    mut text: *const ::core::ffi::c_char,
) {
    diag_write(diag, text, strlen(text) as ssize_t);
}
#[no_mangle]
pub unsafe extern "C" fn ttbc_diag_begin_error() -> *mut ttbc_diagnostic_t {
    if diag_kind as ::core::ffi::c_uint != DIAG_NONE as ::core::ffi::c_int as ::core::ffi::c_uint {
        fprintf(
            __stderrp,
            b"diagnostic: unexpected nested diagnostics\n\0" as *const u8
                as *const ::core::ffi::c_char,
        );
        print_backtrace();
        abort();
    }
    diag_kind = DIAG_ERROR;
    diag_len = 0 as size_t;
    curr_diag += 1;
    return curr_diag as *mut ::core::ffi::c_void as *mut ttbc_diagnostic_t;
}
#[no_mangle]
pub unsafe extern "C" fn ttbc_diag_begin_warning() -> *mut ttbc_diagnostic_t {
    if diag_kind as ::core::ffi::c_uint != DIAG_NONE as ::core::ffi::c_int as ::core::ffi::c_uint {
        fprintf(
            __stderrp,
            b"Unexpected nested diagnostics\n\0" as *const u8 as *const ::core::ffi::c_char,
        );
        print_backtrace();
        abort();
    }
    diag_kind = DIAG_WARN;
    diag_len = 0 as size_t;
    curr_diag += 1;
    return curr_diag as *mut ::core::ffi::c_void as *mut ttbc_diagnostic_t;
}
#[no_mangle]
pub unsafe extern "C" fn ttstub_diag_finish(mut diag: *mut ttbc_diagnostic_t) {
    if diag as intptr_t != curr_diag {
        fprintf(
            __stderrp,
            b"diagnostic: use after free\n\0" as *const u8 as *const ::core::ffi::c_char,
        );
        print_backtrace();
        abort();
    }
    match diag_kind as ::core::ffi::c_uint {
        1 => {
            fprintf(
                __stderrp,
                b"Warning: \0" as *const u8 as *const ::core::ffi::c_char,
            );
        }
        2 => {
            fprintf(
                __stderrp,
                b"Error: \0" as *const u8 as *const ::core::ffi::c_char,
            );
        }
        _ => {
            fprintf(
                __stderrp,
                b"diagnostic: double free\n\0" as *const u8 as *const ::core::ffi::c_char,
            );
            print_backtrace();
            abort();
        }
    }
    diag_kind = DIAG_NONE;
    fprintf(
        __stderrp,
        b"%.*s\n\0" as *const u8 as *const ::core::ffi::c_char,
        diag_len as ::core::ffi::c_int,
        &raw mut diag_buf as *mut ::core::ffi::c_char,
    );
}
unsafe extern "C" fn diag_vprintf(
    mut diag: *mut ttbc_diagnostic_t,
    mut format: *const ::core::ffi::c_char,
    mut ap: ::core::ffi::VaList,
) {
    diag_write(
        diag,
        &raw mut format_buf as *mut ::core::ffi::c_char,
        vsnprintf(
            &raw mut format_buf as *mut ::core::ffi::c_char,
            FORMAT_BUF_SIZE as size_t,
            format,
            ap.clone(),
        ) as ssize_t,
    );
}
#[no_mangle]
pub unsafe extern "C" fn ttstub_diag_printf(
    mut diag: *mut ttbc_diagnostic_t,
    mut format: *const ::core::ffi::c_char,
    mut args: ...
) {
    if logging != 0 {
        fprintf(
            __stderrp,
            b"%s(\"%s\", ...)\n\0" as *const u8 as *const ::core::ffi::c_char,
            b"ttstub_diag_printf\0" as *const u8 as *const ::core::ffi::c_char,
            format,
        );
    }
    let mut ap: ::core::ffi::VaList<'_>;
    ap = args.clone();
    diag_vprintf(diag, format, ap.clone());
}
#[no_mangle]
pub unsafe extern "C" fn ttstub_issue_warning(
    mut format: *const ::core::ffi::c_char,
    mut args: ...
) {
    let mut diag: *mut ttbc_diagnostic_t = ttbc_diag_begin_warning();
    let mut ap: ::core::ffi::VaList<'_>;
    ap = args.clone();
    diag_vprintf(diag, format, ap.clone());
    ttstub_diag_finish(diag);
}
#[no_mangle]
pub unsafe extern "C" fn ttstub_issue_error(mut format: *const ::core::ffi::c_char, mut args: ...) {
    let mut diag: *mut ttbc_diagnostic_t = ttbc_diag_begin_error();
    let mut ap: ::core::ffi::VaList<'_>;
    ap = args.clone();
    diag_vprintf(diag, format, ap.clone());
    ttstub_diag_finish(diag);
}
#[no_mangle]
pub unsafe extern "C" fn ttstub_pic_get_cached_bounds(
    mut name: *const ::core::ffi::c_char,
    mut type_0: ::core::ffi::c_int,
    mut page: ::core::ffi::c_int,
    mut bounds: *mut ::core::ffi::c_float,
) -> ::core::ffi::c_int {
    if !texpresso.is_null() {
        return txp_gpic(
            texpresso,
            name,
            type_0,
            page,
            bounds as *mut ::core::ffi::c_float,
        ) as ::core::ffi::c_int;
    }
    return 0 as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn ttstub_pic_set_cached_bounds(
    mut name: *const ::core::ffi::c_char,
    mut type_0: ::core::ffi::c_int,
    mut page: ::core::ffi::c_int,
    mut bounds: *const ::core::ffi::c_float,
) {
    if !texpresso.is_null() {
        txp_spic(
            texpresso,
            name,
            type_0,
            page,
            bounds as *const ::core::ffi::c_float,
        );
    }
}
#[no_mangle]
pub unsafe extern "C" fn pitex_guard_platform_font() {
    let active: bool = true_0 != 0;
    if active as ::core::ffi::c_int != 0
        && !texpresso.is_null()
        && txp_forked_child as ::core::ffi::c_int != 0
    {
        fprintf(
            __stderrp,
            b"[pitex] platform font requested in checkpoint child\n\0" as *const u8
                as *const ::core::ffi::c_char,
        );
        txp_font_barrier(texpresso);
        _exit(0 as ::core::ffi::c_int);
    }
}
unsafe extern "C" fn usage(mut argv0: *mut ::core::ffi::c_char) {
    fprintf(
        __stderrp,
        b"Usage: %s [-texpresso] [-regenerate-format] <path.tex>\nRun XeTeX engine on <path.tex> using packages from TeX Live.\n\nOptions:\n  -texpresso   Internal (route I/O through TeXpresso)\n  -regenerate-format  Force generation of a fresh format file\n\0"
            as *const u8 as *const ::core::ffi::c_char,
        argv0,
    );
}
unsafe extern "C" fn format_path(
    mut ext: *const ::core::ffi::c_char,
) -> *const ::core::ffi::c_char {
    let mut prefix: [::core::ffi::c_char; 64] = [0; 64];
    snprintf(
        &raw mut prefix as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 64]>() as size_t,
        b"texlive-pitex-%d-%s-\0" as *const u8 as *const ::core::ffi::c_char,
        TECTONIC_FORMAT_VERSION,
        concat!(env!("PITEX_PREVIEW_FORMAT_SCHEMA"), "\0").as_ptr() as *const ::core::ffi::c_char,
    );
    if ext.is_null()
        || (*ext as ::core::ffi::c_int == '.' as i32 || *ext as ::core::ffi::c_int == '-' as i32)
    {
        let mut fresh0: [*const ::core::ffi::c_char; 4] = [
            &raw mut prefix as *mut ::core::ffi::c_char as *const ::core::ffi::c_char,
            format_name,
            ext,
            ::core::ptr::null::<::core::ffi::c_char>(),
        ];
        return cache_path_(
            b"format\0" as *const u8 as *const ::core::ffi::c_char,
            &raw mut fresh0 as *mut *const ::core::ffi::c_char,
        );
    } else {
        let mut fresh1: [*const ::core::ffi::c_char; 5] = [
            &raw mut prefix as *mut ::core::ffi::c_char as *const ::core::ffi::c_char,
            format_name,
            b"-\0" as *const u8 as *const ::core::ffi::c_char,
            ext,
            ::core::ptr::null::<::core::ffi::c_char>(),
        ];
        return cache_path_(
            b"format\0" as *const u8 as *const ::core::ffi::c_char,
            &raw mut fresh1 as *mut *const ::core::ffi::c_char,
        );
    };
}
unsafe extern "C" fn validate_format() -> bool {
    if regenerate_format {
        return 0 as ::core::ffi::c_int != 0;
    }
    let mut path: *const ::core::ffi::c_char = ::core::ptr::null::<::core::ffi::c_char>();
    path = format_path(b".fmt\0" as *const u8 as *const ::core::ffi::c_char);
    if path.is_null() || access(path, R_OK) != 0 as ::core::ffi::c_int {
        return 0 as ::core::ffi::c_int != 0;
    }
    path = format_path(b".deps\0" as *const u8 as *const ::core::ffi::c_char);
    if path.is_null() {
        return 0 as ::core::ffi::c_int != 0;
    }
    let mut tape: *mut FILE =
        fopen(path, b"rb\0" as *const u8 as *const ::core::ffi::c_char) as *mut FILE;
    if tape.is_null() {
        return 0 as ::core::ffi::c_int != 0;
    }
    let mut result: bool = texlive_check_dependencies(tape);
    fclose(tape);
    return result;
}
unsafe extern "C" fn bootstrap_format() -> bool {
    let mut path: *const ::core::ffi::c_char =
        format_path(b".deps\0" as *const u8 as *const ::core::ffi::c_char);
    if path.is_null() {
        return 0 as ::core::ffi::c_int != 0;
    }
    let mut deps_tmp: [::core::ffi::c_char; 1025] = [0; 1025];
    snprintf(
        &raw mut deps_tmp as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 1025]>() as size_t,
        b"%s.tmp\0" as *const u8 as *const ::core::ffi::c_char,
        path,
    );
    dependency_tape = fopen(
        &raw mut deps_tmp as *mut ::core::ffi::c_char,
        b"wb\0" as *const u8 as *const ::core::ffi::c_char,
    ) as *mut FILE;
    if dependency_tape.is_null() {
        return 0 as ::core::ffi::c_int != 0;
    }
    in_initex_mode = true_0 != 0;
    primary_document = format_name;
    let mut result: tt_history_t = tt_run_engine(
        b"texpresso.fmt\0" as *const u8 as *const ::core::ffi::c_char,
        format_name,
        0 as time_t,
    );
    in_initex_mode = false_0 != 0;
    primary_document = ::core::ptr::null::<::core::ffi::c_char>();
    if !dependency_tape.is_null() {
        fclose(dependency_tape);
        dependency_tape = ::core::ptr::null_mut::<FILE>();
    }
    fprintf(
        __stderrp,
        b"Format generation: \0" as *const u8 as *const ::core::ffi::c_char,
    );
    match result as ::core::ffi::c_uint {
        0 => {
            fprintf(
                __stderrp,
                b"Spotless execution.\n\0" as *const u8 as *const ::core::ffi::c_char,
            );
        }
        1 => {
            fprintf(
                __stderrp,
                b"Warnings issued.\n\0" as *const u8 as *const ::core::ffi::c_char,
            );
        }
        2 => {
            fprintf(
                __stderrp,
                b"Errors issued.\n\0" as *const u8 as *const ::core::ffi::c_char,
            );
        }
        3 => {
            fprintf(
                __stderrp,
                b"Aborted with a fatal error.\n\0" as *const u8 as *const ::core::ffi::c_char,
            );
        }
        _ => {}
    }
    if result as ::core::ffi::c_uint
        != HISTORY_SPOTLESS as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        unlink(format_path(
            b".fmt\0" as *const u8 as *const ::core::ffi::c_char,
        ));
        unlink(format_path(
            b".deps\0" as *const u8 as *const ::core::ffi::c_char,
        ));
        unlink(&raw mut deps_tmp as *mut ::core::ffi::c_char);
        let mut fmt_tmp: [::core::ffi::c_char; 1025] = [0; 1025];
        snprintf(
            &raw mut fmt_tmp as *mut ::core::ffi::c_char,
            ::core::mem::size_of::<[::core::ffi::c_char; 1025]>() as size_t,
            b"%s.tmp\0" as *const u8 as *const ::core::ffi::c_char,
            format_path(b".fmt\0" as *const u8 as *const ::core::ffi::c_char),
        );
        unlink(&raw mut fmt_tmp as *mut ::core::ffi::c_char);
    } else if rename(
        &raw mut deps_tmp as *mut ::core::ffi::c_char,
        format_path(b".deps\0" as *const u8 as *const ::core::ffi::c_char),
    ) != 0 as ::core::ffi::c_int
    {
        unlink(format_path(
            b".fmt\0" as *const u8 as *const ::core::ffi::c_char,
        ));
        unlink(format_path(
            b".deps\0" as *const u8 as *const ::core::ffi::c_char,
        ));
        result = HISTORY_FATAL_ERROR;
    }
    return result as ::core::ffi::c_uint
        == HISTORY_SPOTLESS as ::core::ffi::c_int as ::core::ffi::c_uint;
}
unsafe fn main_0(
    mut argc: ::core::ffi::c_int,
    mut argv: *mut *mut ::core::ffi::c_char,
) -> ::core::ffi::c_int {
    let mut doc_path: *const ::core::ffi::c_char = ::core::ptr::null::<::core::ffi::c_char>();
    let mut dashdash: bool = 0 as ::core::ffi::c_int != 0;
    let mut i: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
    while i < argc {
        if !dashdash
            && *(*argv.offset(i as isize)).offset(0 as ::core::ffi::c_int as isize)
                as ::core::ffi::c_int
                == '-' as i32
        {
            if strcmp(
                *argv.offset(i as isize),
                b"-texpresso\0" as *const u8 as *const ::core::ffi::c_char,
            ) == 0 as ::core::ffi::c_int
            {
                use_texpresso = 1 as ::core::ffi::c_int != 0;
            } else if strcmp(
                *argv.offset(i as isize),
                b"-regenerate-format\0" as *const u8 as *const ::core::ffi::c_char,
            ) == 0 as ::core::ffi::c_int
            {
                regenerate_format = 1 as ::core::ffi::c_int != 0;
            } else if strcmp(
                *argv.offset(i as isize),
                b"--\0" as *const u8 as *const ::core::ffi::c_char,
            ) == 0 as ::core::ffi::c_int
            {
                dashdash = 1 as ::core::ffi::c_int != 0;
            } else {
                fprintf(
                    __stderrp,
                    b"Unknown option: %s\n\0" as *const u8 as *const ::core::ffi::c_char,
                    *argv.offset(i as isize),
                );
                usage(*argv.offset(0 as ::core::ffi::c_int as isize));
                return 1 as ::core::ffi::c_int;
            }
        } else if doc_path.is_null() {
            doc_path = *argv.offset(i as isize);
        } else {
            fprintf(
                __stderrp,
                b"Extraneous argument: %s\nOnly one input file can be provided\n\0" as *const u8
                    as *const ::core::ffi::c_char,
                *argv.offset(i as isize),
            );
            usage(*argv.offset(0 as ::core::ffi::c_int as isize));
            return 1 as ::core::ffi::c_int;
        }
        i += 1;
    }
    if !texlive_available() {
        fprintf(
            __stderrp,
            b"Cannot find TeX Live (ensure the kpsewhich command is executable).\nA TeX Live or MacTeX installation is required.\n\0"
                as *const u8 as *const ::core::ffi::c_char,
        );
        usage(*argv.offset(0 as ::core::ffi::c_int as isize));
        return 1 as ::core::ffi::c_int;
    }
    fprintf(
        __stderrp,
        b"Using TeX Live.\n\0" as *const u8 as *const ::core::ffi::c_char,
    );
    format_name = b"xelatex.ini\0" as *const u8 as *const ::core::ffi::c_char;
    lock_format();
    if !validate_format() && !bootstrap_format() {
        fprintf(
            __stderrp,
            b"Failed to generate format.\n\0" as *const u8 as *const ::core::ffi::c_char,
        );
        unlock_format();
        return 1 as ::core::ffi::c_int;
    }
    unlock_format();
    if use_texpresso {
        let mut texpresso_fd: *const ::core::ffi::c_char =
            getenv(b"TEXPRESSO_FD\0" as *const u8 as *const ::core::ffi::c_char);
        let mut fd: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
        if !texpresso_fd.is_null() {
            while *texpresso_fd as ::core::ffi::c_int >= '0' as i32
                && *texpresso_fd as ::core::ffi::c_int <= '9' as i32
            {
                let fresh3 = texpresso_fd;
                texpresso_fd = texpresso_fd.offset(1);
                fd = fd * 10 as ::core::ffi::c_int + *fresh3 as ::core::ffi::c_int - '0' as i32;
            }
        }
        if fd == 0 || texpresso_fd.is_null() || *texpresso_fd as ::core::ffi::c_int != 0 {
            fprintf(
                __stderrp,
                b"Flag -texpresso is for internal use only.\n\0" as *const u8
                    as *const ::core::ffi::c_char,
            );
            return 1 as ::core::ffi::c_int;
        }
        texpresso = txp_connect(fdopen(
            fd,
            b"r+\0" as *const u8 as *const ::core::ffi::c_char,
        ));
        if texpresso.is_null() {
            fprintf(
                __stderrp,
                b"Failed to connect to TeXpresso.\n\0" as *const u8 as *const ::core::ffi::c_char,
            );
            return 1 as ::core::ffi::c_int;
        }
        synctex_enabled = 1 as ::core::ffi::c_int != 0;
        synctex_texpresso_extension = 0 as ::core::ffi::c_int != 0;
    }
    if doc_path.is_null() {
        fprintf(
            __stderrp,
            b"Expecting a .tex input file.\n\0" as *const u8 as *const ::core::ffi::c_char,
        );
        usage(*argv.offset(0 as ::core::ffi::c_int as isize));
        return 1 as ::core::ffi::c_int;
    }
    in_initex_mode = false_0 != 0;
    primary_document = doc_path;
    let mut build_date: time_t = 0;
    let mut epoch_env: *mut ::core::ffi::c_char =
        getenv(b"SOURCE_DATE_EPOCH\0" as *const u8 as *const ::core::ffi::c_char);
    if !epoch_env.is_null() {
        build_date = strtoll(
            epoch_env,
            ::core::ptr::null_mut::<*mut ::core::ffi::c_char>(),
            10 as ::core::ffi::c_int,
        ) as time_t;
    } else {
        build_date = time(::core::ptr::null_mut::<time_t>());
    }
    let mut result: tt_history_t = tt_run_engine(
        b"texpresso.fmt\0" as *const u8 as *const ::core::ffi::c_char,
        primary_document,
        build_date,
    );
    fprintf(
        __stderrp,
        b"Document generation: \0" as *const u8 as *const ::core::ffi::c_char,
    );
    match result as ::core::ffi::c_uint {
        0 => {
            fprintf(
                __stderrp,
                b"Spotless execution.\n\0" as *const u8 as *const ::core::ffi::c_char,
            );
        }
        1 => {
            fprintf(
                __stderrp,
                b"Warnings issued.\n\0" as *const u8 as *const ::core::ffi::c_char,
            );
        }
        2 => {
            fprintf(
                __stderrp,
                b"Errors issued.\n\0" as *const u8 as *const ::core::ffi::c_char,
            );
        }
        3 => {
            fprintf(
                __stderrp,
                b"Aborted with a fatal error.\n\0" as *const u8 as *const ::core::ffi::c_char,
            );
        }
        _ => {}
    }
    if result as ::core::ffi::c_uint
        == HISTORY_SPOTLESS as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        return 0 as ::core::ffi::c_int;
    }
    return 1 as ::core::ffi::c_int;
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
