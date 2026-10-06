// Translated from xetex/common/utils.c with C2Rust 0.22.1.
extern "C" {
    pub type _IO_wide_data;
    pub type _IO_codecvt;
    pub type _IO_marker;
    fn free(__ptr: *mut ::core::ffi::c_void);
    fn abort() -> !;
    fn exit(__status: ::core::ffi::c_int) -> !;
    fn getenv(__name: *const ::core::ffi::c_char) -> *mut ::core::ffi::c_char;
    static mut stderr: *mut FILE;
    fn fprintf(
        __stream: *mut FILE,
        __format: *const ::core::ffi::c_char,
        ...
    ) -> ::core::ffi::c_int;
    fn snprintf(
        __s: *mut ::core::ffi::c_char,
        __maxlen: size_t,
        __format: *const ::core::ffi::c_char,
        ...
    ) -> ::core::ffi::c_int;
    fn perror(__s: *const ::core::ffi::c_char);
    fn __errno_location() -> *mut ::core::ffi::c_int;
    fn backtrace(
        __array: *mut *mut ::core::ffi::c_void,
        __size: ::core::ffi::c_int,
    ) -> ::core::ffi::c_int;
    fn backtrace_symbols(
        __array: *const *mut ::core::ffi::c_void,
        __size: ::core::ffi::c_int,
    ) -> *mut *mut ::core::ffi::c_char;
    fn strlen(__s: *const ::core::ffi::c_char) -> size_t;
    fn mkdir(__path: *const ::core::ffi::c_char, __mode: __mode_t) -> ::core::ffi::c_int;
}
pub type size_t = usize;
pub type __mode_t = ::core::ffi::c_uint;
pub type __off_t = ::core::ffi::c_long;
pub type __off64_t = ::core::ffi::c_long;
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
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<
    ::core::ffi::c_void,
>();
pub const EXIT_FAILURE: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const PATH_MAX: ::core::ffi::c_int = 4096 as ::core::ffi::c_int;
pub const EEXIST: ::core::ffi::c_int = 17 as ::core::ffi::c_int;
pub const __S_IREAD: ::core::ffi::c_int = 0o400 as ::core::ffi::c_int;
pub const __S_IWRITE: ::core::ffi::c_int = 0o200 as ::core::ffi::c_int;
pub const __S_IEXEC: ::core::ffi::c_int = 0o100 as ::core::ffi::c_int;
pub const S_IRWXU: ::core::ffi::c_int = __S_IREAD | __S_IWRITE | __S_IEXEC;
#[no_mangle]
pub static mut logging: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const BT_BUF_SIZE: ::core::ffi::c_int = 100 as ::core::ffi::c_int;
#[no_mangle]
pub unsafe extern "C" fn print_backtrace() {
    let mut nptrs: ::core::ffi::c_int = 0;
    let mut buffer: [*mut ::core::ffi::c_void; 100] = [::core::ptr::null_mut::<
        ::core::ffi::c_void,
    >(); 100];
    let mut strings: *mut *mut ::core::ffi::c_char = ::core::ptr::null_mut::<
        *mut ::core::ffi::c_char,
    >();
    nptrs = backtrace(&raw mut buffer as *mut *mut ::core::ffi::c_void, BT_BUF_SIZE);
    fprintf(
        stderr,
        b"backtrace() returned %d addresses\n\0" as *const u8
            as *const ::core::ffi::c_char,
        nptrs,
    );
    strings = backtrace_symbols(&raw mut buffer as *mut *mut ::core::ffi::c_void, nptrs);
    if strings.is_null() {
        perror(b"backtrace_symbols\0" as *const u8 as *const ::core::ffi::c_char);
        exit(EXIT_FAILURE);
    }
    let mut j: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while j < nptrs {
        fprintf(
            stderr,
            b"%s\n\0" as *const u8 as *const ::core::ffi::c_char,
            *strings.offset(j as isize),
        );
        j += 1;
    }
    free(strings as *mut ::core::ffi::c_void);
}
unsafe extern "C" fn normalize_path(mut path0: *mut ::core::ffi::c_char) {
    let mut index: *mut ::core::ffi::c_char = path0;
    let mut path: *mut ::core::ffi::c_char = path0;
    while *path != 0 {
        *index = *path;
        if *path as ::core::ffi::c_int == '/' as i32 {
            while *path.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                == '/' as i32
            {
                path = path.offset(1);
            }
        }
        index = index.offset(1);
        path = path.offset(1);
    }
    while index > path0
        && *index.offset(-(1 as ::core::ffi::c_int) as isize) as ::core::ffi::c_int
            == '/' as i32
    {
        index = index.offset(-1);
    }
    *index = '\0' as i32 as ::core::ffi::c_char;
}
unsafe extern "C" fn mkdir_path(
    mut path: *mut ::core::ffi::c_char,
    mut base: *mut ::core::ffi::c_char,
) -> bool {
    let mut p: *mut ::core::ffi::c_char = base.offset(1 as ::core::ffi::c_int as isize);
    while *p != 0 {
        if !(*p as ::core::ffi::c_int != '/' as i32) {
            *p = '\0' as i32 as ::core::ffi::c_char;
            let mut ok: bool = mkdir(path, S_IRWXU as __mode_t)
                == 0 as ::core::ffi::c_int || *__errno_location() == EEXIST;
            *p = '/' as i32 as ::core::ffi::c_char;
            if !ok {
                perror(
                    b"cache initialization: mkdir failed\0" as *const u8
                        as *const ::core::ffi::c_char,
                );
                return 0 as ::core::ffi::c_int != 0;
            }
        }
        p = p.offset(1);
    }
    if mkdir(path, S_IRWXU as __mode_t) == 0 as ::core::ffi::c_int
        || *__errno_location() == EEXIST
    {
        return 1 as ::core::ffi::c_int != 0;
    }
    perror(
        b"cache initialization: mkdir failed\0" as *const u8
            as *const ::core::ffi::c_char,
    );
    return 0 as ::core::ffi::c_int != 0;
}
#[no_mangle]
pub static mut cache_path_buffer: [::core::ffi::c_char; 4097] = [0; 4097];
unsafe extern "C" fn cache_base_init() -> ::core::ffi::c_int {
    let mut var: *const ::core::ffi::c_char = ::core::ptr::null::<::core::ffi::c_char>();
    var = getenv(b"PITEX_PREVIEW_CACHE\0" as *const u8 as *const ::core::ffi::c_char);
    if !var.is_null()
        && *var.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int != 0
    {
        if snprintf(
            &raw mut cache_path_buffer as *mut ::core::ffi::c_char,
            ::core::mem::size_of::<[::core::ffi::c_char; 4097]>() as size_t,
            b"%s\0" as *const u8 as *const ::core::ffi::c_char,
            var,
        ) as usize >= ::core::mem::size_of::<[::core::ffi::c_char; 4097]>() as usize
        {
            fprintf(
                stderr,
                b"Error: overflow: $PITEX_PREVIEW_CACHE is too long.\0" as *const u8
                    as *const ::core::ffi::c_char,
            );
            return -(1 as ::core::ffi::c_int);
        }
    } else {
        var = getenv(b"XDG_CACHE_HOME\0" as *const u8 as *const ::core::ffi::c_char);
        if !var.is_null()
            && *var.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int != 0
        {
            if snprintf(
                &raw mut cache_path_buffer as *mut ::core::ffi::c_char,
                ::core::mem::size_of::<[::core::ffi::c_char; 4097]>() as size_t,
                b"%s/texpresso\0" as *const u8 as *const ::core::ffi::c_char,
                var,
            ) as usize >= ::core::mem::size_of::<[::core::ffi::c_char; 4097]>() as usize
            {
                fprintf(
                    stderr,
                    b"Error: overflow: $XDG_CACHE_HOME is too long.\0" as *const u8
                        as *const ::core::ffi::c_char,
                );
                return -(1 as ::core::ffi::c_int);
            }
        } else {
            var = getenv(b"HOME\0" as *const u8 as *const ::core::ffi::c_char);
            if !var.is_null()
                && *var.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                    != 0
            {
                if snprintf(
                    &raw mut cache_path_buffer as *mut ::core::ffi::c_char,
                    ::core::mem::size_of::<[::core::ffi::c_char; 4097]>() as size_t,
                    b"%s/.cache/texpresso\0" as *const u8 as *const ::core::ffi::c_char,
                    var,
                ) as usize
                    >= ::core::mem::size_of::<[::core::ffi::c_char; 4097]>() as usize
                {
                    fprintf(
                        stderr,
                        b"Error: overflow: $HOME/.cache/texpresso is too long.\0"
                            as *const u8 as *const ::core::ffi::c_char,
                    );
                    return -(1 as ::core::ffi::c_int);
                }
            } else {
                fprintf(
                    stderr,
                    b"Error: cannot get cache directory, neither $XDG_CACHE_HOME nor $HOME are defined.\0"
                        as *const u8 as *const ::core::ffi::c_char,
                );
                return -(1 as ::core::ffi::c_int);
            }
        }
    }
    normalize_path(&raw mut cache_path_buffer as *mut ::core::ffi::c_char);
    if !mkdir_path(
        &raw mut cache_path_buffer as *mut ::core::ffi::c_char,
        &raw mut cache_path_buffer as *mut ::core::ffi::c_char,
    ) {
        fprintf(
            stderr,
            b"Error: cannot access cache directory: %s.\0" as *const u8
                as *const ::core::ffi::c_char,
            &raw mut cache_path_buffer as *mut ::core::ffi::c_char,
        );
        return -(1 as ::core::ffi::c_int);
    }
    return strlen(&raw mut cache_path_buffer as *mut ::core::ffi::c_char)
        as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn cache_path_(
    mut folder: *const ::core::ffi::c_char,
    mut name: *mut *const ::core::ffi::c_char,
) -> *const ::core::ffi::c_char {
    static mut baselen: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    if baselen == 0 as ::core::ffi::c_int {
        baselen = cache_base_init();
    }
    if baselen < 0 as ::core::ffi::c_int {
        return ::core::ptr::null::<::core::ffi::c_char>();
    }
    let mut len: ::core::ffi::c_int = baselen;
    if !folder.is_null() && *folder as ::core::ffi::c_int != 0
        && (len as usize)
            < ::core::mem::size_of::<[::core::ffi::c_char; 4097]>() as usize
    {
        let fresh0 = len;
        len = len + 1;
        cache_path_buffer[fresh0 as usize] = '/' as i32 as ::core::ffi::c_char;
        len
            += snprintf(
                (&raw mut cache_path_buffer as *mut ::core::ffi::c_char)
                    .offset(len as isize),
                (::core::mem::size_of::<[::core::ffi::c_char; 4097]>() as size_t)
                    .wrapping_sub(len as size_t),
                b"%s\0" as *const u8 as *const ::core::ffi::c_char,
                folder,
            );
        if !mkdir_path(
            &raw mut cache_path_buffer as *mut ::core::ffi::c_char,
            (&raw mut cache_path_buffer as *mut ::core::ffi::c_char)
                .offset(baselen as isize)
                .offset(1 as ::core::ffi::c_int as isize),
        ) {
            fprintf(
                stderr,
                b"Error: cannot cache create directory %s\n\0" as *const u8
                    as *const ::core::ffi::c_char,
                &raw mut cache_path_buffer as *mut ::core::ffi::c_char,
            );
            return ::core::ptr::null::<::core::ffi::c_char>();
        }
    }
    if !name.is_null()
        && (len as usize)
            < ::core::mem::size_of::<[::core::ffi::c_char; 4097]>() as usize
    {
        let mut delim: bool = 0 as ::core::ffi::c_int != 0;
        while !(*name).is_null() {
            if !(*name).is_null() && !delim {
                let fresh1 = len;
                len = len + 1;
                cache_path_buffer[fresh1 as usize] = '/' as i32 as ::core::ffi::c_char;
                delim = 1 as ::core::ffi::c_int != 0;
            }
            len
                += snprintf(
                    (&raw mut cache_path_buffer as *mut ::core::ffi::c_char)
                        .offset(len as isize),
                    (::core::mem::size_of::<[::core::ffi::c_char; 4097]>() as size_t)
                        .wrapping_sub(len as size_t),
                    b"%s\0" as *const u8 as *const ::core::ffi::c_char,
                    *name,
                );
            name = name.offset(1);
        }
        fprintf(stderr, b"\n\0" as *const u8 as *const ::core::ffi::c_char);
    }
    if len > PATH_MAX {
        abort();
    }
    if len == PATH_MAX {
        fprintf(
            stderr,
            b"Error: cache path is too long:\n%s\n\0" as *const u8
                as *const ::core::ffi::c_char,
            &raw mut cache_path_buffer as *mut ::core::ffi::c_char,
        );
        return ::core::ptr::null::<::core::ffi::c_char>();
    }
    return &raw mut cache_path_buffer as *mut ::core::ffi::c_char;
}
