/* texmfmp.c: Hand-coded routines for TeX or Metafont in C.  Originally
   written by Tim Morgan, drawing from other Unix ports of TeX.  This is
   a collection of miscellany, everything that's easier (or only
   possible) to do in C.

   This file is public domain.  */
// Translated from xetex/engine/xetex-texmfmp.c with C2Rust 0.22.1.
extern "C" {
    pub type ttbc_input_handle_t;
    fn malloc(__size: size_t) -> *mut ::core::ffi::c_void;
    fn free(__ptr: *mut ::core::ffi::c_void);
    fn abs(__x: ::core::ffi::c_int) -> ::core::ffi::c_int;
    fn strlen(__s: *const ::core::ffi::c_char) -> size_t;
    fn ttbc_get_data_md5(
        data: *const uint8_t,
        len: size_t,
        digest: *mut uint8_t,
    ) -> ::core::ffi::c_int;
    fn strftime(
        __s: *mut ::core::ffi::c_char,
        __maxsize: size_t,
        __format: *const ::core::ffi::c_char,
        __tp: *const tm,
    ) -> size_t;
    fn gmtime(__timer: *const time_t) -> *mut tm;
    fn localtime(__timer: *const time_t) -> *mut tm;
    fn _tt_abort(format: *const ::core::ffi::c_char, ...) -> !;
    fn ttstub_input_open(
        path: *const ::core::ffi::c_char,
        format: ttbc_file_format,
        is_gz: ::core::ffi::c_int,
    ) -> rust_input_handle_t;
    fn ttstub_input_get_size(handle: rust_input_handle_t) -> size_t;
    fn ttstub_input_get_mtime(handle: rust_input_handle_t) -> time_t;
    fn ttstub_input_seek(
        handle: rust_input_handle_t,
        offset: ssize_t,
        whence: ::core::ffi::c_int,
    ) -> size_t;
    fn ttstub_input_read(
        handle: rust_input_handle_t,
        data: *mut ::core::ffi::c_char,
        len: size_t,
    ) -> ssize_t;
    fn ttstub_input_close(handle: rust_input_handle_t) -> ::core::ffi::c_int;
    fn ttstub_get_file_md5(
        path: *const ::core::ffi::c_char,
        digest: *mut ::core::ffi::c_char,
    ) -> ::core::ffi::c_int;
    static mut pool_size: int32_t;
    static mut str_pool: *mut packed_UTF16_code;
    static mut str_start: *mut pool_pointer;
    static mut pool_ptr: pool_pointer;
    static offsetsFromUTF8: [uint32_t; 6];
    static bytesFromUTF8: [uint8_t; 256];
    static firstByteMark: [uint8_t; 7];
    fn make_string() -> str_number;
    fn gettimeofday(
        __tv: *mut timeval,
        __tz: *mut ::core::ffi::c_void,
    ) -> ::core::ffi::c_int;
    fn sprintf(
        __s: *mut ::core::ffi::c_char,
        __format: *const ::core::ffi::c_char,
        ...
    ) -> ::core::ffi::c_int;
    fn snprintf(
        __s: *mut ::core::ffi::c_char,
        __maxlen: size_t,
        __format: *const ::core::ffi::c_char,
        ...
    ) -> ::core::ffi::c_int;
}
pub type __uint8_t = u8;
pub type __uint16_t = u16;
pub type __int32_t = i32;
pub type __uint32_t = u32;
pub type __time_t = ::core::ffi::c_long;
pub type __suseconds_t = ::core::ffi::c_long;
pub type int32_t = __int32_t;
pub type uint8_t = __uint8_t;
pub type uint16_t = __uint16_t;
pub type uint32_t = __uint32_t;
pub type size_t = usize;
pub type ssize_t = isize;
pub type time_t = __time_t;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct timeval {
    pub tv_sec: __time_t,
    pub tv_usec: __suseconds_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct tm {
    pub tm_sec: ::core::ffi::c_int,
    pub tm_min: ::core::ffi::c_int,
    pub tm_hour: ::core::ffi::c_int,
    pub tm_mday: ::core::ffi::c_int,
    pub tm_mon: ::core::ffi::c_int,
    pub tm_year: ::core::ffi::c_int,
    pub tm_wday: ::core::ffi::c_int,
    pub tm_yday: ::core::ffi::c_int,
    pub tm_isdst: ::core::ffi::c_int,
    pub tm_gmtoff: ::core::ffi::c_long,
    pub tm_zone: *const ::core::ffi::c_char,
}
pub type rust_input_handle_t = *mut ttbc_input_handle_t;
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
pub type str_number = int32_t;
pub type packed_UTF16_code = ::core::ffi::c_ushort;
pub type UInt32 = ::core::ffi::c_uint;
pub type pool_pointer = int32_t;
pub type UInt16 = ::core::ffi::c_ushort;
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<
    ::core::ffi::c_void,
>();
static mut last_source_name: *mut ::core::ffi::c_char = ::core::ptr::null::<
    ::core::ffi::c_char,
>() as *mut ::core::ffi::c_char;
static mut last_lineno: ::core::ffi::c_int = 0;
pub const TIME_STR_SIZE: ::core::ffi::c_int = 30 as ::core::ffi::c_int;
static mut start_time_str: [::core::ffi::c_char; 30] = [0; 30];
unsafe extern "C" fn makepdftime(
    mut t: time_t,
    mut time_str: *mut ::core::ffi::c_char,
    mut utc: bool,
) {
    let mut lt: tm = tm {
        tm_sec: 0,
        tm_min: 0,
        tm_hour: 0,
        tm_mday: 0,
        tm_mon: 0,
        tm_year: 0,
        tm_wday: 0,
        tm_yday: 0,
        tm_isdst: 0,
        tm_gmtoff: 0,
        tm_zone: ::core::ptr::null::<::core::ffi::c_char>(),
    };
    let mut gmt: tm = tm {
        tm_sec: 0,
        tm_min: 0,
        tm_hour: 0,
        tm_mday: 0,
        tm_mon: 0,
        tm_year: 0,
        tm_wday: 0,
        tm_yday: 0,
        tm_isdst: 0,
        tm_gmtoff: 0,
        tm_zone: ::core::ptr::null::<::core::ffi::c_char>(),
    };
    let mut size: size_t = 0;
    let mut i: ::core::ffi::c_int = 0;
    let mut off: ::core::ffi::c_int = 0;
    let mut off_hours: ::core::ffi::c_int = 0;
    let mut off_mins: ::core::ffi::c_int = 0;
    if utc {
        lt = *gmtime(&raw mut t);
    } else {
        lt = *localtime(&raw mut t);
    }
    size = strftime(
        time_str,
        TIME_STR_SIZE as size_t,
        b"D:%Y%m%d%H%M%S\0" as *const u8 as *const ::core::ffi::c_char,
        &raw mut lt,
    );
    if size == 0 as size_t {
        *time_str.offset(0 as ::core::ffi::c_int as isize) = '\0' as i32
            as ::core::ffi::c_char;
        return;
    }
    if *time_str.offset(14 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
        == '6' as i32
    {
        *time_str.offset(14 as ::core::ffi::c_int as isize) = '5' as i32
            as ::core::ffi::c_char;
        *time_str.offset(15 as ::core::ffi::c_int as isize) = '9' as i32
            as ::core::ffi::c_char;
        *time_str.offset(16 as ::core::ffi::c_int as isize) = '\0' as i32
            as ::core::ffi::c_char;
    }
    gmt = *gmtime(&raw mut t);
    off = 60 as ::core::ffi::c_int * (lt.tm_hour - gmt.tm_hour) + lt.tm_min - gmt.tm_min;
    if lt.tm_year != gmt.tm_year {
        off
            += if lt.tm_year > gmt.tm_year {
                1440 as ::core::ffi::c_int
            } else {
                -(1440 as ::core::ffi::c_int)
            };
    } else if lt.tm_yday != gmt.tm_yday {
        off
            += if lt.tm_yday > gmt.tm_yday {
                1440 as ::core::ffi::c_int
            } else {
                -(1440 as ::core::ffi::c_int)
            };
    }
    if off == 0 as ::core::ffi::c_int {
        let fresh12 = size;
        size = size.wrapping_add(1);
        *time_str.offset(fresh12 as isize) = 'Z' as i32 as ::core::ffi::c_char;
        *time_str.offset(size as isize) = 0 as ::core::ffi::c_char;
    } else {
        off_hours = off / 60 as ::core::ffi::c_int;
        off_mins = abs(off - off_hours * 60 as ::core::ffi::c_int);
        i = snprintf(
            time_str.offset(size as isize) as *mut ::core::ffi::c_char,
            9 as size_t,
            b"%+03d'%02d'\0" as *const u8 as *const ::core::ffi::c_char,
            off_hours,
            off_mins,
        );
        if i as ::core::ffi::c_uint >= 9 as ::core::ffi::c_int as ::core::ffi::c_uint {
            _tt_abort(
                b"snprintf failed: file %s, line %d\0" as *const u8
                    as *const ::core::ffi::c_char,
                b"xetex/engine/xetex-texmfmp.c\0" as *const u8
                    as *const ::core::ffi::c_char,
                79 as ::core::ffi::c_int,
            );
        }
    };
}
#[no_mangle]
pub unsafe extern "C" fn init_start_time(mut source_date_epoch: time_t) {
    makepdftime(
        source_date_epoch,
        &raw mut start_time_str as *mut ::core::ffi::c_char,
        true_0 != 0,
    );
}
#[no_mangle]
pub unsafe extern "C" fn getcreationdate() {
    let mut len: size_t = 0;
    let mut i: ::core::ffi::c_int = 0;
    len = strlen(&raw mut start_time_str as *mut ::core::ffi::c_char);
    if (pool_ptr as size_t).wrapping_add(len) as ::core::ffi::c_uint
        >= pool_size as ::core::ffi::c_uint
    {
        pool_ptr = pool_size as pool_pointer;
        return;
    }
    i = 0 as ::core::ffi::c_int;
    while (i as size_t) < len {
        let fresh13 = pool_ptr;
        pool_ptr = pool_ptr + 1;
        *str_pool.offset(fresh13 as isize) = start_time_str[i as usize] as uint16_t
            as packed_UTF16_code;
        i += 1;
    }
}
#[no_mangle]
pub unsafe extern "C" fn get_date_and_time(
    mut source_date_epoch: time_t,
    mut minutes: *mut int32_t,
    mut day: *mut int32_t,
    mut month: *mut int32_t,
    mut year: *mut int32_t,
) {
    let mut tmptr: *mut tm = localtime(&raw mut source_date_epoch);
    *minutes = ((*tmptr).tm_hour * 60 as ::core::ffi::c_int + (*tmptr).tm_min)
        as int32_t;
    *day = (*tmptr).tm_mday as int32_t;
    *month = ((*tmptr).tm_mon + 1 as ::core::ffi::c_int) as int32_t;
    *year = ((*tmptr).tm_year + 1900 as ::core::ffi::c_int) as int32_t;
}
#[no_mangle]
pub unsafe extern "C" fn get_seconds_and_micros(
    mut seconds: *mut int32_t,
    mut micros: *mut int32_t,
) {
    let mut tv: timeval = timeval { tv_sec: 0, tv_usec: 0 };
    gettimeofday(&raw mut tv, NULL);
    *seconds = tv.tv_sec as int32_t;
    *micros = tv.tv_usec as int32_t;
}
#[no_mangle]
pub unsafe extern "C" fn getfilemoddate(mut s: str_number) {
    let mut name: *mut ::core::ffi::c_char = ::core::ptr::null_mut::<
        ::core::ffi::c_char,
    >();
    let mut mtime: time_t = 0;
    let mut text_len: size_t = 0;
    let mut handle: rust_input_handle_t = ::core::ptr::null_mut::<ttbc_input_handle_t>();
    // A local offset (+HH'MM') needs more room than UTC's Z.
    let mut buf: [::core::ffi::c_char; TIME_STR_SIZE as usize] = [0; TIME_STR_SIZE as usize];
    name = gettexstring(s);
    handle = ttstub_input_open(name, TTBC_FILE_FORMAT_TEX, 0 as ::core::ffi::c_int);
    free(name as *mut ::core::ffi::c_void);
    if handle.is_null() {
        return;
    }
    mtime = ttstub_input_get_mtime(handle);
    ttstub_input_close(handle);
    // pdfTeX reports file times in local time with their UTC offset.
    makepdftime(mtime, &raw mut buf as *mut ::core::ffi::c_char, false);
    text_len = strlen(&raw mut buf as *mut ::core::ffi::c_char);
    if (pool_ptr as size_t).wrapping_add(text_len) as ::core::ffi::c_uint
        >= pool_size as ::core::ffi::c_uint
    {
        pool_ptr = pool_size as pool_pointer;
    } else {
        let mut i: ::core::ffi::c_int = 0;
        i = 0 as ::core::ffi::c_int;
        while (i as size_t) < text_len {
            let fresh14 = pool_ptr;
            pool_ptr = pool_ptr + 1;
            *str_pool.offset(fresh14 as isize) = buf[i as usize] as uint16_t
                as packed_UTF16_code;
            i += 1;
        }
    };
}
#[no_mangle]
pub unsafe extern "C" fn getfilesize(mut s: str_number) {
    let mut name: *mut ::core::ffi::c_char = ::core::ptr::null_mut::<
        ::core::ffi::c_char,
    >();
    let mut file_len: size_t = 0;
    let mut text_len: size_t = 0;
    let mut handle: rust_input_handle_t = ::core::ptr::null_mut::<ttbc_input_handle_t>();
    let mut buf: [::core::ffi::c_char; 20] = [0; 20];
    let mut i: ::core::ffi::c_int = 0;
    name = gettexstring(s);
    handle = ttstub_input_open(name, TTBC_FILE_FORMAT_TEX, 0 as ::core::ffi::c_int);
    free(name as *mut ::core::ffi::c_void);
    if handle.is_null() {
        return;
    }
    file_len = ttstub_input_get_size(handle);
    ttstub_input_close(handle);
    i = snprintf(
        &raw mut buf as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 20]>() as size_t,
        b"%lu\0" as *const u8 as *const ::core::ffi::c_char,
        file_len as ::core::ffi::c_ulong,
    );
    if i as ::core::ffi::c_uint
        >= ::core::mem::size_of::<[::core::ffi::c_char; 20]>() as ::core::ffi::c_uint
    {
        _tt_abort(
            b"snprintf failed: file %s, line %d\0" as *const u8
                as *const ::core::ffi::c_char,
            b"xetex/engine/xetex-texmfmp.c\0" as *const u8 as *const ::core::ffi::c_char,
            203 as ::core::ffi::c_int,
        );
    }
    text_len = strlen(&raw mut buf as *mut ::core::ffi::c_char);
    if (pool_ptr as size_t).wrapping_add(text_len) as ::core::ffi::c_uint
        >= pool_size as ::core::ffi::c_uint
    {
        pool_ptr = pool_size as pool_pointer;
    } else {
        let mut i_0: ::core::ffi::c_int = 0;
        i_0 = 0 as ::core::ffi::c_int;
        while (i_0 as size_t) < text_len {
            let fresh15 = pool_ptr;
            pool_ptr = pool_ptr + 1;
            *str_pool.offset(fresh15 as isize) = buf[i_0 as usize] as uint16_t
                as packed_UTF16_code;
            i_0 += 1;
        }
    };
}
#[no_mangle]
pub unsafe extern "C" fn getfiledump(
    mut s: int32_t,
    mut offset: ::core::ffi::c_int,
    mut length: ::core::ffi::c_int,
) {
    let mut name: *mut ::core::ffi::c_char = ::core::ptr::null_mut::<
        ::core::ffi::c_char,
    >();
    let mut handle: rust_input_handle_t = ::core::ptr::null_mut::<ttbc_input_handle_t>();
    let mut buffer: *mut ::core::ffi::c_uchar = ::core::ptr::null_mut::<
        ::core::ffi::c_uchar,
    >();
    let mut i: ::core::ffi::c_int = 0;
    let mut j: ::core::ffi::c_int = 0;
    let mut k: ::core::ffi::c_int = 0;
    let mut actual: ssize_t = 0;
    let mut strbuf: [::core::ffi::c_char; 3] = [0; 3];
    if length == 0 as ::core::ffi::c_int {
        return;
    }
    if pool_ptr + 2 as pool_pointer * length as pool_pointer + 1 as pool_pointer
        >= pool_size
    {
        pool_ptr = pool_size as pool_pointer;
        return;
    }
    buffer = malloc((length + 1 as ::core::ffi::c_int) as size_t)
        as *mut ::core::ffi::c_uchar;
    if buffer.is_null() {
        pool_ptr = pool_size as pool_pointer;
        return;
    }
    name = gettexstring(s as str_number);
    handle = ttstub_input_open(name, TTBC_FILE_FORMAT_TEX, 0 as ::core::ffi::c_int);
    free(name as *mut ::core::ffi::c_void);
    if handle.is_null() {
        free(buffer as *mut ::core::ffi::c_void);
        return;
    }
    ttstub_input_seek(handle, offset as ssize_t, SEEK_SET);
    actual = ttstub_input_read(
        handle,
        buffer as *mut ::core::ffi::c_char,
        length as size_t,
    );
    ttstub_input_close(handle);
    j = 0 as ::core::ffi::c_int;
    while (j as ssize_t) < actual {
        i = snprintf(
            &raw mut strbuf as *mut ::core::ffi::c_char,
            3 as size_t,
            b"%.2X\0" as *const u8 as *const ::core::ffi::c_char,
            *buffer.offset(j as isize) as ::core::ffi::c_uint,
        );
        if i as ::core::ffi::c_uint >= 3 as ::core::ffi::c_int as ::core::ffi::c_uint {
            _tt_abort(
                b"snprintf failed: file %s, line %d\0" as *const u8
                    as *const ::core::ffi::c_char,
                b"xetex/engine/xetex-texmfmp.c\0" as *const u8
                    as *const ::core::ffi::c_char,
                256 as ::core::ffi::c_int,
            );
        }
        k = 0 as ::core::ffi::c_int;
        while k < i {
            let fresh16 = pool_ptr;
            pool_ptr = pool_ptr + 1;
            *str_pool.offset(fresh16 as isize) = strbuf[k as usize] as uint16_t
                as packed_UTF16_code;
            k += 1;
        }
        j += 1;
    }
    free(buffer as *mut ::core::ffi::c_void);
}
unsafe extern "C" fn checkpool_pointer(mut pool_ptr_0: pool_pointer, mut len: size_t) {
    if (pool_ptr_0 as size_t).wrapping_add(len) >= pool_size as size_t {
        _tt_abort(
            b"string pool overflow [%i bytes]\0" as *const u8
                as *const ::core::ffi::c_char,
            pool_size as ::core::ffi::c_int,
        );
    }
}
#[no_mangle]
pub unsafe extern "C" fn maketexstring(
    mut s: *const ::core::ffi::c_char,
) -> ::core::ffi::c_int {
    let mut len: size_t = 0;
    let mut rval: UInt32 = 0;
    let mut cp: *const ::core::ffi::c_uchar = s as *const ::core::ffi::c_uchar;
    if s.is_null() || *s as ::core::ffi::c_int == 0 as ::core::ffi::c_int {
        return EMPTY_STRING as ::core::ffi::c_int;
    }
    len = strlen(s);
    checkpool_pointer(pool_ptr, len);
    loop {
        let fresh0 = cp;
        cp = cp.offset(1);
        rval = *fresh0 as UInt32;
        if !(rval != 0 as UInt32) {
            break;
        }
        let mut extraBytes: UInt16 = bytesFromUTF8[rval as usize] as UInt16;
        let mut current_block_19: u64;
        match extraBytes as ::core::ffi::c_int {
            5 => {
                rval <<= 6 as ::core::ffi::c_int;
                if *cp != 0 {
                    let fresh1 = cp;
                    cp = cp.offset(1);
                    rval = rval.wrapping_add(*fresh1 as UInt32);
                }
                current_block_19 = 12149376058001570530;
            }
            4 => {
                current_block_19 = 12149376058001570530;
            }
            3 => {
                current_block_19 = 12022675163562480673;
            }
            2 => {
                current_block_19 = 4029790830837410459;
            }
            1 => {
                current_block_19 = 15555410512067649404;
            }
            0 | _ => {
                current_block_19 = 13797916685926291137;
            }
        }
        match current_block_19 {
            12149376058001570530 => {
                rval <<= 6 as ::core::ffi::c_int;
                if *cp != 0 {
                    let fresh2 = cp;
                    cp = cp.offset(1);
                    rval = rval.wrapping_add(*fresh2 as UInt32);
                }
                current_block_19 = 12022675163562480673;
            }
            _ => {}
        }
        match current_block_19 {
            12022675163562480673 => {
                rval <<= 6 as ::core::ffi::c_int;
                if *cp != 0 {
                    let fresh3 = cp;
                    cp = cp.offset(1);
                    rval = rval.wrapping_add(*fresh3 as UInt32);
                }
                current_block_19 = 4029790830837410459;
            }
            _ => {}
        }
        match current_block_19 {
            4029790830837410459 => {
                rval <<= 6 as ::core::ffi::c_int;
                if *cp != 0 {
                    let fresh4 = cp;
                    cp = cp.offset(1);
                    rval = rval.wrapping_add(*fresh4 as UInt32);
                }
                current_block_19 = 15555410512067649404;
            }
            _ => {}
        }
        match current_block_19 {
            15555410512067649404 => {
                rval <<= 6 as ::core::ffi::c_int;
                if *cp != 0 {
                    let fresh5 = cp;
                    cp = cp.offset(1);
                    rval = rval.wrapping_add(*fresh5 as UInt32);
                }
            }
            _ => {}
        }
        rval = (rval as ::core::ffi::c_uint)
            .wrapping_sub(offsetsFromUTF8[extraBytes as usize] as ::core::ffi::c_uint)
            as UInt32 as UInt32;
        if rval > 0xffff as UInt32 {
            rval = rval.wrapping_sub(0x10000 as ::core::ffi::c_int as UInt32);
            let fresh6 = pool_ptr;
            pool_ptr = pool_ptr + 1;
            *str_pool.offset(fresh6 as isize) = (0xd800 as UInt32)
                .wrapping_add(rval.wrapping_div(0x400 as UInt32)) as packed_UTF16_code;
            let fresh7 = pool_ptr;
            pool_ptr = pool_ptr + 1;
            *str_pool.offset(fresh7 as isize) = (0xdc00 as UInt32)
                .wrapping_add(rval.wrapping_rem(0x400 as UInt32)) as packed_UTF16_code;
        } else {
            let fresh8 = pool_ptr;
            pool_ptr = pool_ptr + 1;
            *str_pool.offset(fresh8 as isize) = rval as packed_UTF16_code;
        }
    }
    return make_string() as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn gettexstring(mut s: str_number) -> *mut ::core::ffi::c_char {
    let mut bytesToWrite: ::core::ffi::c_uint = 0 as ::core::ffi::c_uint;
    let mut len: pool_pointer = 0;
    let mut i: pool_pointer = 0;
    let mut j: pool_pointer = 0;
    let mut name: *mut ::core::ffi::c_char = ::core::ptr::null_mut::<
        ::core::ffi::c_char,
    >();
    if s as ::core::ffi::c_long >= 65536 as ::core::ffi::c_long {
        len = *str_start
            .offset(
                ((s + 1 as str_number) as ::core::ffi::c_long
                    - 65536 as ::core::ffi::c_long) as isize,
            )
            - *str_start
                .offset(
                    (s as ::core::ffi::c_long - 65536 as ::core::ffi::c_long) as isize,
                );
    } else {
        len = 0 as ::core::ffi::c_int as pool_pointer;
    }
    name = malloc((len * 3 as pool_pointer + 1 as pool_pointer) as size_t)
        as *mut ::core::ffi::c_char;
    i = 0 as ::core::ffi::c_int as pool_pointer;
    j = 0 as ::core::ffi::c_int as pool_pointer;
    while i < len {
        let mut c: uint32_t = *str_pool
            .offset(
                (i
                    + *str_start
                        .offset(
                            (s as ::core::ffi::c_long - 65536 as ::core::ffi::c_long)
                                as isize,
                        )) as isize,
            ) as uint32_t;
        if c >= 0xd800 as uint32_t && c <= 0xdbff as uint32_t {
            i += 1;
            let mut lo: uint32_t = *str_pool
                .offset(
                    (i
                        + *str_start
                            .offset(
                                (s as ::core::ffi::c_long - 65536 as ::core::ffi::c_long)
                                    as isize,
                            )) as isize,
                ) as uint32_t;
            if lo >= 0xdc00 as uint32_t && lo <= 0xdfff as uint32_t {
                c = c
                    .wrapping_sub(0xd800 as uint32_t)
                    .wrapping_mul(0x400 as uint32_t)
                    .wrapping_add(lo)
                    .wrapping_sub(0xdc00 as uint32_t)
                    .wrapping_add(0x10000 as uint32_t);
            } else {
                c = 0xfffd as uint32_t;
            }
        }
        if c < 0x80 as uint32_t {
            bytesToWrite = 1 as ::core::ffi::c_uint;
        } else if c < 0x800 as uint32_t {
            bytesToWrite = 2 as ::core::ffi::c_uint;
        } else if c < 0x10000 as uint32_t {
            bytesToWrite = 3 as ::core::ffi::c_uint;
        } else if c < 0x110000 as uint32_t {
            bytesToWrite = 4 as ::core::ffi::c_uint;
        } else {
            bytesToWrite = 3 as ::core::ffi::c_uint;
            c = 0xfffd as uint32_t;
        }
        j = (j as ::core::ffi::c_uint).wrapping_add(bytesToWrite) as pool_pointer
            as pool_pointer;
        let mut current_block_28: u64;
        match bytesToWrite {
            4 => {
                j -= 1;
                *name.offset(j as isize) = ((c | 0x80 as uint32_t) & 0xbf as uint32_t)
                    as ::core::ffi::c_char;
                c >>= 6 as ::core::ffi::c_int;
                current_block_28 = 15130007286820891071;
            }
            3 => {
                current_block_28 = 15130007286820891071;
            }
            2 => {
                current_block_28 = 12420765539082642404;
            }
            1 => {
                current_block_28 = 1895797759377347758;
            }
            _ => {
                current_block_28 = 15925075030174552612;
            }
        }
        match current_block_28 {
            15130007286820891071 => {
                j -= 1;
                *name.offset(j as isize) = ((c | 0x80 as uint32_t) & 0xbf as uint32_t)
                    as ::core::ffi::c_char;
                c >>= 6 as ::core::ffi::c_int;
                current_block_28 = 12420765539082642404;
            }
            _ => {}
        }
        match current_block_28 {
            12420765539082642404 => {
                j -= 1;
                *name.offset(j as isize) = ((c | 0x80 as uint32_t) & 0xbf as uint32_t)
                    as ::core::ffi::c_char;
                c >>= 6 as ::core::ffi::c_int;
                current_block_28 = 1895797759377347758;
            }
            _ => {}
        }
        match current_block_28 {
            1895797759377347758 => {
                j -= 1;
                *name.offset(j as isize) = (c
                    | firstByteMark[bytesToWrite as usize] as uint32_t)
                    as ::core::ffi::c_char;
            }
            _ => {}
        }
        j = (j as ::core::ffi::c_uint).wrapping_add(bytesToWrite) as pool_pointer
            as pool_pointer;
        i += 1;
    }
    *name.offset(j as isize) = 0 as ::core::ffi::c_char;
    return name;
}
unsafe extern "C" fn compare_paths(
    mut p1: *const ::core::ffi::c_char,
    mut p2: *const ::core::ffi::c_char,
) -> ::core::ffi::c_int {
    let mut ret: ::core::ffi::c_int = 0;
    loop {
        ret = *p1 as ::core::ffi::c_int - *p2 as ::core::ffi::c_int;
        if !(ret == 0 as ::core::ffi::c_int
            && *p2 as ::core::ffi::c_int != 0 as ::core::ffi::c_int
            || *p1 as ::core::ffi::c_int == '/' as i32
                && *p2 as ::core::ffi::c_int == '/' as i32)
        {
            break;
        }
        p1 = p1.offset(1);
        p2 = p2.offset(1);
    }
    ret = if ret < 0 as ::core::ffi::c_int {
        -(1 as ::core::ffi::c_int)
    } else if ret > 0 as ::core::ffi::c_int {
        1 as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    };
    return ret;
}
#[no_mangle]
pub unsafe extern "C" fn is_new_source(
    mut srcfilename: str_number,
    mut lineno: ::core::ffi::c_int,
) -> bool {
    let mut name: *mut ::core::ffi::c_char = gettexstring(srcfilename);
    return compare_paths(name, last_source_name) != 0 as ::core::ffi::c_int
        || lineno != last_lineno;
}
#[no_mangle]
pub unsafe extern "C" fn remember_source_info(
    mut srcfilename: str_number,
    mut lineno: ::core::ffi::c_int,
) {
    free(last_source_name as *mut ::core::ffi::c_void);
    last_source_name = gettexstring(srcfilename);
    last_lineno = lineno;
}
#[no_mangle]
pub unsafe extern "C" fn make_src_special(
    mut srcfilename: str_number,
    mut lineno: ::core::ffi::c_int,
) -> pool_pointer {
    let mut oldpool_ptr: pool_pointer = pool_ptr;
    let mut filename: *mut ::core::ffi::c_char = gettexstring(srcfilename);
    let mut buf: [::core::ffi::c_char; 40] = [0; 40];
    let mut s: *mut ::core::ffi::c_char = &raw mut buf as *mut ::core::ffi::c_char;
    sprintf(
        &raw mut buf as *mut ::core::ffi::c_char,
        b"src:%d \0" as *const u8 as *const ::core::ffi::c_char,
        lineno,
    );
    if (pool_ptr as size_t)
        .wrapping_add(strlen(&raw mut buf as *mut ::core::ffi::c_char))
        .wrapping_add(strlen(filename)) >= pool_size as size_t
    {
        _tt_abort(b"string pool overflow\0" as *const u8 as *const ::core::ffi::c_char);
    }
    s = &raw mut buf as *mut ::core::ffi::c_char;
    while *s != 0 {
        let fresh17 = s;
        s = s.offset(1);
        let fresh18 = pool_ptr;
        pool_ptr = pool_ptr + 1;
        *str_pool.offset(fresh18 as isize) = *fresh17 as packed_UTF16_code;
    }
    s = filename;
    while *s != 0 {
        let fresh19 = s;
        s = s.offset(1);
        let fresh20 = pool_ptr;
        pool_ptr = pool_ptr + 1;
        *str_pool.offset(fresh20 as isize) = *fresh19 as packed_UTF16_code;
    }
    return oldpool_ptr;
}
unsafe extern "C" fn convertStringToHexString(
    mut in_0: *const ::core::ffi::c_char,
    mut out: *mut ::core::ffi::c_char,
    mut lin: ::core::ffi::c_int,
) {
    static mut hexchars: [::core::ffi::c_char; 17] = unsafe {
        ::core::mem::transmute::<
            [u8; 17],
            [::core::ffi::c_char; 17],
        >(*b"0123456789ABCDEF\0")
    };
    let mut i: ::core::ffi::c_int = 0;
    let mut j: ::core::ffi::c_int = 0;
    j = 0 as ::core::ffi::c_int;
    i = 0 as ::core::ffi::c_int;
    while i < lin {
        let mut c: ::core::ffi::c_uchar = *in_0.offset(i as isize)
            as ::core::ffi::c_uchar;
        let fresh10 = j;
        j = j + 1;
        *out.offset(fresh10 as isize) = hexchars[(c as ::core::ffi::c_int
            >> 4 as ::core::ffi::c_int & 0xf as ::core::ffi::c_int) as usize];
        let fresh11 = j;
        j = j + 1;
        *out.offset(fresh11 as isize) = hexchars[(c as ::core::ffi::c_int
            & 0xf as ::core::ffi::c_int) as usize];
        i += 1;
    }
    *out.offset(j as isize) = '\0' as i32 as ::core::ffi::c_char;
}
pub const DIGEST_SIZE: ::core::ffi::c_int = 16 as ::core::ffi::c_int;
#[no_mangle]
pub unsafe extern "C" fn getmd5sum(mut s: str_number, mut file: bool) {
    let mut digest: [::core::ffi::c_char; 16] = [0; 16];
    let mut outbuf: [::core::ffi::c_char; 33] = [0; 33];
    let mut xname: *mut ::core::ffi::c_char = ::core::ptr::null_mut::<
        ::core::ffi::c_char,
    >();
    let mut ret: ::core::ffi::c_int = 0;
    let mut i: ::core::ffi::c_int = 0;
    xname = gettexstring(s);
    if file {
        ret = ttstub_get_file_md5(xname, &raw mut digest as *mut ::core::ffi::c_char);
    } else {
        ret = ttbc_get_data_md5(
            xname as *const uint8_t,
            strlen(xname),
            &raw mut digest as *mut ::core::ffi::c_char as *mut uint8_t,
        );
    }
    free(xname as *mut ::core::ffi::c_void);
    if ret != 0 {
        return;
    }
    if pool_ptr + 2 as pool_pointer * DIGEST_SIZE as pool_pointer >= pool_size {
        return;
    }
    convertStringToHexString(
        &raw mut digest as *mut ::core::ffi::c_char,
        &raw mut outbuf as *mut ::core::ffi::c_char,
        DIGEST_SIZE,
    );
    i = 0 as ::core::ffi::c_int;
    while i < 2 as ::core::ffi::c_int * DIGEST_SIZE {
        let fresh9 = pool_ptr;
        pool_ptr = pool_ptr + 1;
        *str_pool.offset(fresh9 as isize) = outbuf[i as usize] as uint16_t
            as packed_UTF16_code;
        i += 1;
    }
}
pub const EMPTY_STRING: ::core::ffi::c_long = 65536 as ::core::ffi::c_long
    + 1 as ::core::ffi::c_long;
pub const SEEK_SET: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const true_0: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
