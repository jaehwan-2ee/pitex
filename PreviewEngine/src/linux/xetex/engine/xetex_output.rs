/* tectonic/output.c -- functions related to outputting messages
 * Copyright 2016 the Tectonic Project
 * Licensed under the MIT License.
*/
// Translated from xetex/engine/xetex-output.c with C2Rust 0.22.1.
extern "C" {
    pub type ttbc_output_handle_t;
    pub type ttbc_diagnostic_t;
    fn free(__ptr: *mut ::core::ffi::c_void);
    fn abs(__x: ::core::ffi::c_int) -> ::core::ffi::c_int;
    fn strlen(__s: *const ::core::ffi::c_char) -> size_t;
    fn ttbc_diag_begin_warning() -> *mut ttbc_diagnostic_t;
    fn ttbc_diag_begin_error() -> *mut ttbc_diagnostic_t;
    fn ttbc_diag_append(diag: *mut ttbc_diagnostic_t, text: *const ::core::ffi::c_char);
    fn ttstub_diag_printf(
        diag: *mut ttbc_diagnostic_t,
        format: *const ::core::ffi::c_char,
        ...
    );
    fn ttstub_diag_finish(diag: *mut ttbc_diagnostic_t);
    fn ttstub_output_putc(
        handle: rust_output_handle_t,
        c: ::core::ffi::c_int,
    ) -> ::core::ffi::c_int;
    fn gettexstring(_: str_number) -> *mut ::core::ffi::c_char;
    static mut eqtb: *mut memory_word;
    static mut error_line: int32_t;
    static mut max_print_line: int32_t;
    static mut pool_size: int32_t;
    static mut file_line_error_style_p: ::core::ffi::c_int;
    static mut str_pool: *mut packed_UTF16_code;
    static mut str_start: *mut pool_pointer;
    static mut pool_ptr: pool_pointer;
    static mut str_ptr: str_number;
    static mut rust_stdout: rust_output_handle_t;
    static mut log_file: rust_output_handle_t;
    static mut selector: selector_t;
    static mut dig: [::core::ffi::c_uchar; 23];
    static mut tally: int32_t;
    static mut term_offset: int32_t;
    static mut file_offset: int32_t;
    static mut trick_buf: [UTF16_code; 256];
    static mut trick_count: int32_t;
    static mut doing_special: bool;
    static mut mem: *mut memory_word;
    static mut hash: *mut b32x2;
    static mut eqtb_top: int32_t;
    static mut prim: [b32x2; 2101];
    static mut in_open: int32_t;
    static mut line: int32_t;
    static mut line_stack: *mut int32_t;
    static mut full_source_filename_stack: *mut str_number;
    static mut write_file: [rust_output_handle_t; 16];
}
pub type __uint16_t = u16;
pub type __int32_t = i32;
pub type int32_t = __int32_t;
pub type uint16_t = __uint16_t;
pub type size_t = usize;
pub type rust_output_handle_t = *mut ttbc_output_handle_t;
pub type scaled_t = int32_t;
pub type selector_t = ::core::ffi::c_uint;
pub const SELECTOR_NEW_STRING: selector_t = 21;
pub const SELECTOR_PSEUDO: selector_t = 20;
pub const SELECTOR_TERM_AND_LOG: selector_t = 19;
pub const SELECTOR_LOG_ONLY: selector_t = 18;
pub const SELECTOR_TERM_ONLY: selector_t = 17;
pub const SELECTOR_NO_PRINT: selector_t = 16;
pub const SELECTOR_FILE_15: selector_t = 15;
pub const SELECTOR_FILE_0: selector_t = 0;
pub type eight_bits = ::core::ffi::c_uchar;
pub type UTF16_code = ::core::ffi::c_ushort;
pub type UnicodeScalar = int32_t;
pub type pool_pointer = int32_t;
pub type str_number = int32_t;
pub type packed_UTF16_code = ::core::ffi::c_ushort;
pub type small_number = ::core::ffi::c_short;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct b32x2_le_t {
    pub s0: int32_t,
    pub s1: int32_t,
}
pub type b32x2 = b32x2_le_t;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct b16x4_le_t {
    pub s0: uint16_t,
    pub s1: uint16_t,
    pub s2: uint16_t,
    pub s3: uint16_t,
}
pub type b16x4 = b16x4_le_t;
#[derive(Copy, Clone)]
#[repr(C)]
pub union memory_word {
    pub b32: b32x2,
    pub b16: b16x4,
    pub gr: ::core::ffi::c_double,
    pub ptr: *mut ::core::ffi::c_void,
}
pub const INT_PAR__escape_char: ::core::ffi::c_int = 45 as ::core::ffi::c_int;
pub const INT_PAR__new_line_char: ::core::ffi::c_int = 49 as ::core::ffi::c_int;
pub const INT_PARS: ::core::ffi::c_int = 89 as ::core::ffi::c_int;
pub const DIMEN_PARS: ::core::ffi::c_int = 23 as ::core::ffi::c_int;
pub const ACTIVE_BASE: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const SINGLE_BASE: ::core::ffi::c_int = 1114113 as ::core::ffi::c_int;
pub const NULL_CS: ::core::ffi::c_int = 2228225 as ::core::ffi::c_int;
pub const HASH_BASE: ::core::ffi::c_int = 2228226 as ::core::ffi::c_int;
pub const PRIM_EQTB_BASE: ::core::ffi::c_int = 2243238 as ::core::ffi::c_int;
pub const FROZEN_NULL_FONT: ::core::ffi::c_int = 2245338 as ::core::ffi::c_int;
pub const UNDEFINED_CONTROL_SEQUENCE: ::core::ffi::c_int = 2254339 as ::core::ffi::c_int;
pub const CAT_CODE_BASE: ::core::ffi::c_int = 2256169 as ::core::ffi::c_int;
pub const INT_BASE: ::core::ffi::c_int = 7826729 as ::core::ffi::c_int;
pub const COUNT_BASE: ::core::ffi::c_int = INT_BASE + INT_PARS;
pub const DEL_CODE_BASE: ::core::ffi::c_int = COUNT_BASE + 256 as ::core::ffi::c_int;
pub const DIMEN_BASE: ::core::ffi::c_int = DEL_CODE_BASE + NUMBER_USVS;
pub const SCALED_BASE: ::core::ffi::c_int = DIMEN_BASE + DIMEN_PARS;
pub const EQTB_SIZE: ::core::ffi::c_int = SCALED_BASE + 255 as ::core::ffi::c_int;
pub const LETTER: ::core::ffi::c_int = 11 as ::core::ffi::c_int;
pub const TEXT_SIZE: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const SCRIPT_SIZE: ::core::ffi::c_int = 256 as ::core::ffi::c_int;
static mut current_diagnostic: *mut ttbc_diagnostic_t = ::core::ptr::null::<
    ttbc_diagnostic_t,
>() as *mut ttbc_diagnostic_t;
#[no_mangle]
pub unsafe extern "C" fn capture_to_diagnostic(mut diagnostic: *mut ttbc_diagnostic_t) {
    if !current_diagnostic.is_null() {
        ttstub_diag_finish(current_diagnostic);
    }
    current_diagnostic = diagnostic;
}
unsafe extern "C" fn diagnostic_print_file_line(mut diagnostic: *mut ttbc_diagnostic_t) {
    let mut level: int32_t = in_open;
    while level > 0 as int32_t
        && *full_source_filename_stack.offset(level as isize) == 0 as str_number
    {
        level -= 1;
    }
    if level == 0 as int32_t {
        ttbc_diag_append(diagnostic, b"!\0" as *const u8 as *const ::core::ffi::c_char);
    } else {
        let mut source_line: int32_t = line;
        if level != in_open {
            source_line = *line_stack.offset((level + 1 as int32_t) as isize);
        }
        let mut filename: *mut ::core::ffi::c_char = gettexstring(
            *full_source_filename_stack.offset(level as isize),
        );
        ttstub_diag_printf(
            diagnostic,
            b"%s:%d: \0" as *const u8 as *const ::core::ffi::c_char,
            filename,
            source_line,
        );
        free(filename as *mut ::core::ffi::c_void);
    };
}
#[no_mangle]
pub unsafe extern "C" fn diagnostic_begin_capture_warning_here() -> *mut ttbc_diagnostic_t {
    let mut warning: *mut ttbc_diagnostic_t = ttbc_diag_begin_warning();
    diagnostic_print_file_line(warning);
    capture_to_diagnostic(warning);
    return warning;
}
#[no_mangle]
pub unsafe extern "C" fn error_here_with_diagnostic(
    mut message: *const ::core::ffi::c_char,
) -> *mut ttbc_diagnostic_t {
    let mut error: *mut ttbc_diagnostic_t = ttbc_diag_begin_error();
    diagnostic_print_file_line(error);
    ttstub_diag_printf(
        error,
        b"%s\0" as *const u8 as *const ::core::ffi::c_char,
        message,
    );
    if file_line_error_style_p != 0 {
        print_file_line();
    } else {
        print_nl_cstr(b"! \0" as *const u8 as *const ::core::ffi::c_char);
    }
    print_cstr(message);
    capture_to_diagnostic(error);
    return error;
}
unsafe extern "C" fn warn_char(mut c: ::core::ffi::c_int) {
    if !current_diagnostic.is_null() {
        let mut bytes: [::core::ffi::c_char; 2] = [
            c as ::core::ffi::c_char,
            0 as ::core::ffi::c_int as ::core::ffi::c_char,
        ];
        ttbc_diag_append(current_diagnostic, &raw mut bytes as *mut ::core::ffi::c_char);
    }
}
#[no_mangle]
pub unsafe extern "C" fn print_ln() {
    match selector as ::core::ffi::c_uint {
        19 => {
            warn_char('\n' as i32);
            ttstub_output_putc(rust_stdout, '\n' as i32);
            ttstub_output_putc(log_file, '\n' as i32);
            term_offset = 0 as ::core::ffi::c_int as int32_t;
            file_offset = 0 as ::core::ffi::c_int as int32_t;
        }
        18 => {
            warn_char('\n' as i32);
            ttstub_output_putc(log_file, '\n' as i32);
            file_offset = 0 as ::core::ffi::c_int as int32_t;
        }
        17 => {
            warn_char('\n' as i32);
            ttstub_output_putc(rust_stdout, '\n' as i32);
            term_offset = 0 as ::core::ffi::c_int as int32_t;
        }
        16 | 20 | 21 => {}
        _ => {
            ttstub_output_putc(write_file[selector as usize], '\n' as i32);
        }
    };
}
#[no_mangle]
pub unsafe extern "C" fn print_raw_char(mut s: UTF16_code, mut incr_offset: bool) {
    match selector as ::core::ffi::c_uint {
        19 => {
            warn_char(s as ::core::ffi::c_int);
            ttstub_output_putc(rust_stdout, s as ::core::ffi::c_int);
            ttstub_output_putc(log_file, s as ::core::ffi::c_int);
            if incr_offset {
                term_offset += 1;
                file_offset += 1;
            }
            if term_offset == max_print_line {
                ttstub_output_putc(rust_stdout, '\n' as i32);
                term_offset = 0 as ::core::ffi::c_int as int32_t;
            }
            if file_offset == max_print_line {
                ttstub_output_putc(log_file, '\n' as i32);
                file_offset = 0 as ::core::ffi::c_int as int32_t;
            }
        }
        18 => {
            warn_char(s as ::core::ffi::c_int);
            ttstub_output_putc(log_file, s as ::core::ffi::c_int);
            if incr_offset {
                file_offset += 1;
            }
            if file_offset == max_print_line {
                ttstub_output_putc(log_file, '\n' as i32);
                file_offset = 0 as ::core::ffi::c_int as int32_t;
            }
        }
        17 => {
            warn_char(s as ::core::ffi::c_int);
            ttstub_output_putc(rust_stdout, s as ::core::ffi::c_int);
            if incr_offset {
                term_offset += 1;
            }
            if term_offset == max_print_line {
                ttstub_output_putc(rust_stdout, '\n' as i32);
                term_offset = 0 as ::core::ffi::c_int as int32_t;
            }
        }
        16 => {}
        20 => {
            if tally < trick_count {
                trick_buf[(tally % error_line) as usize] = s;
            }
        }
        21 => {
            if pool_ptr < pool_size {
                *str_pool.offset(pool_ptr as isize) = s as packed_UTF16_code;
                pool_ptr += 1;
            }
        }
        _ => {
            ttstub_output_putc(write_file[selector as usize], s as ::core::ffi::c_int);
        }
    }
    tally += 1;
}
#[no_mangle]
pub unsafe extern "C" fn print_char(mut s: int32_t) {
    let mut l: small_number = 0;
    if selector as ::core::ffi::c_uint
        > SELECTOR_PSEUDO as ::core::ffi::c_int as ::core::ffi::c_uint && !doing_special
    {
        if s >= 0x10000 as int32_t {
            print_raw_char(
                (0xd800 as int32_t + (s - 0x10000 as int32_t) / 1024 as int32_t)
                    as UTF16_code,
                true_0 != 0,
            );
            print_raw_char(
                (0xdc00 as int32_t + (s - 0x10000 as int32_t) % 1024 as int32_t)
                    as UTF16_code,
                true_0 != 0,
            );
        } else {
            print_raw_char(s as UTF16_code, true_0 != 0);
        }
        return;
    }
    if s == (*eqtb.offset((INT_BASE + INT_PAR__new_line_char) as isize)).b32.s1 {
        if (selector as ::core::ffi::c_uint)
            < SELECTOR_PSEUDO as ::core::ffi::c_int as ::core::ffi::c_uint
        {
            print_ln();
            return;
        }
    }
    if s < 32 as int32_t && !doing_special {
        print_raw_char('^' as i32 as UTF16_code, true_0 != 0);
        print_raw_char('^' as i32 as UTF16_code, true_0 != 0);
        print_raw_char((s + 64 as int32_t) as UTF16_code, true_0 != 0);
    } else if s < 127 as int32_t {
        print_raw_char(s as UTF16_code, true_0 != 0);
    } else if s == 127 as int32_t {
        if !doing_special {
            print_raw_char('^' as i32 as UTF16_code, true_0 != 0);
            print_raw_char('^' as i32 as UTF16_code, true_0 != 0);
            print_raw_char('?' as i32 as UTF16_code, true_0 != 0);
        } else {
            print_raw_char(s as UTF16_code, true_0 != 0);
        }
    } else if s < 160 as int32_t && !doing_special {
        print_raw_char('^' as i32 as UTF16_code, true_0 != 0);
        print_raw_char('^' as i32 as UTF16_code, true_0 != 0);
        l = (s % 256 as int32_t / 16 as int32_t) as small_number;
        if (l as ::core::ffi::c_int) < 10 as ::core::ffi::c_int {
            print_raw_char(
                ('0' as i32 + l as ::core::ffi::c_int) as UTF16_code,
                true_0 != 0,
            );
        } else {
            print_raw_char(
                ('a' as i32 + l as ::core::ffi::c_int - 10 as ::core::ffi::c_int)
                    as UTF16_code,
                true_0 != 0,
            );
        }
        l = (s % 16 as int32_t) as small_number;
        if (l as ::core::ffi::c_int) < 10 as ::core::ffi::c_int {
            print_raw_char(
                ('0' as i32 + l as ::core::ffi::c_int) as UTF16_code,
                true_0 != 0,
            );
        } else {
            print_raw_char(
                ('a' as i32 + l as ::core::ffi::c_int - 10 as ::core::ffi::c_int)
                    as UTF16_code,
                true_0 != 0,
            );
        }
    } else if selector as ::core::ffi::c_uint
        == SELECTOR_PSEUDO as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        print_raw_char(s as UTF16_code, true_0 != 0);
    } else if s < 2048 as int32_t {
        print_raw_char((192 as int32_t + s / 64 as int32_t) as UTF16_code, false_0 != 0);
        print_raw_char((128 as int32_t + s % 64 as int32_t) as UTF16_code, true_0 != 0);
    } else if s < 0x10000 as int32_t {
        print_raw_char(
            (224 as int32_t + s / 4096 as int32_t) as UTF16_code,
            false_0 != 0,
        );
        print_raw_char(
            (128 as int32_t + s % 4096 as int32_t / 64 as int32_t) as UTF16_code,
            false_0 != 0,
        );
        print_raw_char((128 as int32_t + s % 64 as int32_t) as UTF16_code, true_0 != 0);
    } else {
        print_raw_char(
            (240 as int32_t + s / 0x40000 as int32_t) as UTF16_code,
            false_0 != 0,
        );
        print_raw_char(
            (128 as int32_t + s % 0x40000 as int32_t / 4096 as int32_t) as UTF16_code,
            false_0 != 0,
        );
        print_raw_char(
            (128 as int32_t + s % 4096 as int32_t / 64 as int32_t) as UTF16_code,
            false_0 != 0,
        );
        print_raw_char((128 as int32_t + s % 64 as int32_t) as UTF16_code, true_0 != 0);
    };
}
#[no_mangle]
pub unsafe extern "C" fn print(mut s: int32_t) {
    let mut nl: int32_t = 0;
    if s >= str_ptr {
        return print_cstr(b"???\0" as *const u8 as *const ::core::ffi::c_char)
    } else if s <= BIGGEST_CHAR as int32_t {
        if s < 0 as int32_t {
            return print_cstr(b"???\0" as *const u8 as *const ::core::ffi::c_char)
        } else {
            if selector as ::core::ffi::c_uint
                > SELECTOR_PSEUDO as ::core::ffi::c_int as ::core::ffi::c_uint
            {
                print_char(s);
                return;
            }
            if s == (*eqtb.offset((INT_BASE + INT_PAR__new_line_char) as isize)).b32.s1 {
                if (selector as ::core::ffi::c_uint)
                    < SELECTOR_PSEUDO as ::core::ffi::c_int as ::core::ffi::c_uint
                {
                    print_ln();
                    return;
                }
            }
            nl = (*eqtb.offset((INT_BASE + INT_PAR__new_line_char) as isize)).b32.s1;
            (*eqtb.offset((INT_BASE + INT_PAR__new_line_char) as isize)).b32.s1 = -(1
                as ::core::ffi::c_int) as int32_t;
            print_char(s);
            (*eqtb.offset((INT_BASE + INT_PAR__new_line_char) as isize)).b32.s1 = nl;
            return;
        }
    }
    let mut pool_idx: int32_t = s - 0x10000 as int32_t;
    let mut i: pool_pointer = *str_start.offset(pool_idx as isize);
    while i < *str_start.offset((pool_idx + 1 as int32_t) as isize) {
        if *str_pool.offset(i as isize) as ::core::ffi::c_int
            >= 0xd800 as ::core::ffi::c_int
            && (*str_pool.offset(i as isize) as ::core::ffi::c_int)
                < 0xdc00 as ::core::ffi::c_int
            && (i + 1 as pool_pointer)
                < *str_start.offset((pool_idx + 1 as int32_t) as isize)
            && *str_pool.offset((i + 1 as pool_pointer) as isize) as ::core::ffi::c_int
                >= 0xdc00 as ::core::ffi::c_int
            && (*str_pool.offset((i + 1 as pool_pointer) as isize) as ::core::ffi::c_int)
                < 0xe000 as ::core::ffi::c_int
        {
            print_char(
                0x10000 as int32_t
                    + (*str_pool.offset(i as isize) as int32_t - 0xd800 as int32_t)
                        * 1024 as int32_t
                    + *str_pool.offset((i + 1 as pool_pointer) as isize) as int32_t
                    - 0xdc00 as int32_t,
            );
            i += 1;
        } else {
            print_char(*str_pool.offset(i as isize) as int32_t);
        }
        i += 1;
    }
}
#[no_mangle]
pub unsafe extern "C" fn print_cstr(mut str: *const ::core::ffi::c_char) {
    let mut i: ::core::ffi::c_uint = 0 as ::core::ffi::c_uint;
    while (i as size_t) < strlen(str) {
        print_char(*str.offset(i as isize) as int32_t);
        i = i.wrapping_add(1);
    }
}
#[no_mangle]
pub unsafe extern "C" fn print_nl(mut s: str_number) {
    if term_offset > 0 as int32_t
        && selector as ::core::ffi::c_uint & 1 as ::core::ffi::c_uint != 0
        || file_offset > 0 as int32_t
            && selector as ::core::ffi::c_uint
                >= SELECTOR_LOG_ONLY as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        print_ln();
    }
    print(s as int32_t);
}
#[no_mangle]
pub unsafe extern "C" fn print_nl_cstr(mut str: *const ::core::ffi::c_char) {
    if term_offset > 0 as int32_t
        && selector as ::core::ffi::c_uint & 1 as ::core::ffi::c_uint != 0
        || file_offset > 0 as int32_t
            && selector as ::core::ffi::c_uint
                >= SELECTOR_LOG_ONLY as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        print_ln();
    }
    print_cstr(str);
}
#[no_mangle]
pub unsafe extern "C" fn print_esc(mut s: str_number) {
    let mut c: int32_t = (*eqtb.offset((INT_BASE + INT_PAR__escape_char) as isize))
        .b32
        .s1;
    if c >= 0 as int32_t && c <= BIGGEST_USV as int32_t {
        print_char(c);
    }
    print(s as int32_t);
}
#[no_mangle]
pub unsafe extern "C" fn print_esc_cstr(mut s: *const ::core::ffi::c_char) {
    let mut c: int32_t = (*eqtb.offset((INT_BASE + INT_PAR__escape_char) as isize))
        .b32
        .s1;
    if c >= 0 as int32_t && c <= BIGGEST_USV as int32_t {
        print_char(c);
    }
    print_cstr(s);
}
unsafe extern "C" fn print_the_digs(mut k: eight_bits) {
    while k as ::core::ffi::c_int > 0 as ::core::ffi::c_int {
        k = k.wrapping_sub(1);
        if (dig[k as usize] as ::core::ffi::c_int) < 10 as ::core::ffi::c_int {
            print_char('0' as i32 + dig[k as usize] as int32_t);
        } else {
            print_char(55 as int32_t + dig[k as usize] as int32_t);
        }
    }
}
#[no_mangle]
pub unsafe extern "C" fn print_int(mut n: int32_t) {
    let mut k: ::core::ffi::c_uchar = 0 as ::core::ffi::c_uchar;
    let mut m: int32_t = 0;
    if n < 0 as int32_t {
        print_char('-' as i32);
        if n as ::core::ffi::c_long > -(100000000 as ::core::ffi::c_long) {
            n = -n;
        } else {
            m = -(1 as int32_t) - n;
            n = m / 10 as int32_t;
            m = m % 10 as int32_t + 1 as int32_t;
            k = 1 as ::core::ffi::c_uchar;
            if m < 10 as int32_t {
                dig[0 as ::core::ffi::c_int as usize] = m as ::core::ffi::c_uchar;
            } else {
                dig[0 as ::core::ffi::c_int as usize] = 0 as ::core::ffi::c_uchar;
                n += 1;
            }
        }
    }
    loop {
        dig[k as usize] = (n % 10 as int32_t) as ::core::ffi::c_uchar;
        n = n / 10 as int32_t;
        k = k.wrapping_add(1);
        if n == 0 as int32_t {
            break;
        }
    }
    print_the_digs(k as eight_bits);
}
#[no_mangle]
pub unsafe extern "C" fn print_cs(mut p: int32_t) {
    if p < HASH_BASE as int32_t {
        if p >= SINGLE_BASE as int32_t {
            if p == NULL_CS as int32_t {
                print_esc_cstr(b"csname\0" as *const u8 as *const ::core::ffi::c_char);
                print_esc_cstr(
                    b"endcsname\0" as *const u8 as *const ::core::ffi::c_char,
                );
                print_char(' ' as i32);
            } else {
                print_esc(p as str_number - SINGLE_BASE as str_number);
                if (*eqtb
                    .offset(
                        (CAT_CODE_BASE as int32_t + (p - 1114113 as int32_t)) as isize,
                    ))
                    .b32
                    .s1 == LETTER as int32_t
                {
                    print_char(' ' as i32);
                }
            }
        } else if p < ACTIVE_BASE as int32_t {
            print_esc_cstr(b"IMPOSSIBLE.\0" as *const u8 as *const ::core::ffi::c_char);
        } else {
            print_char(p - 1 as int32_t);
        }
    } else if p >= UNDEFINED_CONTROL_SEQUENCE as int32_t && p <= EQTB_SIZE as int32_t
        || p > eqtb_top
    {
        print_esc_cstr(b"IMPOSSIBLE.\0" as *const u8 as *const ::core::ffi::c_char);
    } else if (*hash.offset(p as isize)).s1 >= str_ptr {
        print_esc_cstr(b"NONEXISTENT.\0" as *const u8 as *const ::core::ffi::c_char);
    } else {
        if p >= PRIM_EQTB_BASE as int32_t && p < FROZEN_NULL_FONT as int32_t {
            print_esc(
                prim[(p - PRIM_EQTB_BASE as int32_t) as usize].s1 as str_number
                    - 1 as str_number,
            );
        } else {
            print_esc((*hash.offset(p as isize)).s1 as str_number);
        }
        print_char(' ' as i32);
    };
}
#[no_mangle]
pub unsafe extern "C" fn sprint_cs(mut p: int32_t) {
    if p < HASH_BASE as int32_t {
        if p < SINGLE_BASE as int32_t {
            print_char(p - 1 as int32_t);
        } else if p < NULL_CS as int32_t {
            print_esc(p as str_number - SINGLE_BASE as str_number);
        } else {
            print_esc_cstr(b"csname\0" as *const u8 as *const ::core::ffi::c_char);
            print_esc_cstr(b"endcsname\0" as *const u8 as *const ::core::ffi::c_char);
        }
    } else if p >= PRIM_EQTB_BASE as int32_t && p < FROZEN_NULL_FONT as int32_t {
        print_esc(
            prim[(p - PRIM_EQTB_BASE as int32_t) as usize].s1 as str_number
                - 1 as str_number,
        );
    } else {
        print_esc((*hash.offset(p as isize)).s1 as str_number);
    };
}
#[no_mangle]
pub unsafe extern "C" fn print_file_name(
    mut n: int32_t,
    mut a: int32_t,
    mut e: int32_t,
) {
    let mut must_quote: bool = false_0 != 0;
    let mut quote_char: int32_t = 0 as int32_t;
    let mut j: pool_pointer = 0;
    if a != 0 as int32_t {
        j = *str_start.offset((a - 0x10000 as int32_t) as isize);
        while (!must_quote || quote_char == 0 as int32_t)
            && j < *str_start.offset((a + 1 as int32_t - 0x10000 as int32_t) as isize)
        {
            if *str_pool.offset(j as isize) as ::core::ffi::c_int == ' ' as i32 {
                must_quote = true_0 != 0;
            } else if *str_pool.offset(j as isize) as ::core::ffi::c_int == '"' as i32
                || *str_pool.offset(j as isize) as ::core::ffi::c_int == '\'' as i32
            {
                must_quote = true_0 != 0;
                quote_char = (73 as ::core::ffi::c_int
                    - *str_pool.offset(j as isize) as ::core::ffi::c_int) as int32_t;
            }
            j += 1;
        }
    }
    if n != 0 as int32_t {
        j = *str_start.offset((n - 0x10000 as int32_t) as isize);
        while (!must_quote || quote_char == 0 as int32_t)
            && j < *str_start.offset((n + 1 as int32_t - 0x10000 as int32_t) as isize)
        {
            if *str_pool.offset(j as isize) as ::core::ffi::c_int == ' ' as i32 {
                must_quote = true_0 != 0;
            } else if *str_pool.offset(j as isize) as ::core::ffi::c_int == '"' as i32
                || *str_pool.offset(j as isize) as ::core::ffi::c_int == '\'' as i32
            {
                must_quote = true_0 != 0;
                quote_char = (73 as ::core::ffi::c_int
                    - *str_pool.offset(j as isize) as ::core::ffi::c_int) as int32_t;
            }
            j += 1;
        }
    }
    if e != 0 as int32_t {
        j = *str_start.offset((e - 0x10000 as int32_t) as isize);
        while (!must_quote || quote_char == 0 as int32_t)
            && j < *str_start.offset((e + 1 as int32_t - 0x10000 as int32_t) as isize)
        {
            if *str_pool.offset(j as isize) as ::core::ffi::c_int == ' ' as i32 {
                must_quote = true_0 != 0;
            } else if *str_pool.offset(j as isize) as ::core::ffi::c_int == '"' as i32
                || *str_pool.offset(j as isize) as ::core::ffi::c_int == '\'' as i32
            {
                must_quote = true_0 != 0;
                quote_char = (73 as ::core::ffi::c_int
                    - *str_pool.offset(j as isize) as ::core::ffi::c_int) as int32_t;
            }
            j += 1;
        }
    }
    if must_quote {
        if quote_char == 0 as int32_t {
            quote_char = '"' as i32 as int32_t;
        }
        print_char(quote_char);
    }
    if a != 0 as int32_t {
        let mut for_end: int32_t = 0;
        j = *str_start.offset((a - 0x10000 as int32_t) as isize);
        for_end = (*str_start.offset((a + 1 as int32_t - 0x10000 as int32_t) as isize)
            - 1 as pool_pointer) as int32_t;
        if j <= for_end {
            loop {
                if *str_pool.offset(j as isize) as int32_t == quote_char {
                    print(quote_char);
                    quote_char = 73 as int32_t - quote_char;
                    print(quote_char);
                }
                print(*str_pool.offset(j as isize) as int32_t);
                let fresh0 = j;
                j = j + 1;
                if !(fresh0 < for_end) {
                    break;
                }
            }
        }
    }
    if n != 0 as int32_t {
        let mut for_end_0: int32_t = 0;
        j = *str_start.offset((n - 0x10000 as int32_t) as isize);
        for_end_0 = (*str_start.offset((n + 1 as int32_t - 0x10000 as int32_t) as isize)
            - 1 as pool_pointer) as int32_t;
        if j <= for_end_0 {
            loop {
                if *str_pool.offset(j as isize) as int32_t == quote_char {
                    print(quote_char);
                    quote_char = 73 as int32_t - quote_char;
                    print(quote_char);
                }
                print(*str_pool.offset(j as isize) as int32_t);
                let fresh1 = j;
                j = j + 1;
                if !(fresh1 < for_end_0) {
                    break;
                }
            }
        }
    }
    if e != 0 as int32_t {
        let mut for_end_1: int32_t = 0;
        j = *str_start.offset((e - 0x10000 as int32_t) as isize);
        for_end_1 = (*str_start.offset((e + 1 as int32_t - 0x10000 as int32_t) as isize)
            - 1 as pool_pointer) as int32_t;
        if j <= for_end_1 {
            loop {
                if *str_pool.offset(j as isize) as int32_t == quote_char {
                    print(quote_char);
                    quote_char = 73 as int32_t - quote_char;
                    print(quote_char);
                }
                print(*str_pool.offset(j as isize) as int32_t);
                let fresh2 = j;
                j = j + 1;
                if !(fresh2 < for_end_1) {
                    break;
                }
            }
        }
    }
    if quote_char != 0 as int32_t {
        print_char(quote_char);
    }
}
#[no_mangle]
pub unsafe extern "C" fn print_size(mut s: int32_t) {
    if s == TEXT_SIZE as int32_t {
        print_esc_cstr(b"textfont\0" as *const u8 as *const ::core::ffi::c_char);
    } else if s == SCRIPT_SIZE as int32_t {
        print_esc_cstr(b"scriptfont\0" as *const u8 as *const ::core::ffi::c_char);
    } else {
        print_esc_cstr(b"scriptscriptfont\0" as *const u8 as *const ::core::ffi::c_char);
    };
}
#[no_mangle]
pub unsafe extern "C" fn print_write_whatsit(
    mut s: *const ::core::ffi::c_char,
    mut p: int32_t,
) {
    print_esc_cstr(s);
    if (*mem.offset((p + 1 as int32_t) as isize)).b32.s0 < 16 as int32_t {
        print_int((*mem.offset((p + 1 as int32_t) as isize)).b32.s0);
    } else if (*mem.offset((p + 1 as int32_t) as isize)).b32.s0 == 16 as int32_t {
        print_char('*' as i32);
    } else {
        print_char('-' as i32);
    };
}
#[no_mangle]
pub unsafe extern "C" fn print_native_word(mut p: int32_t) {
    let mut i: int32_t = 0;
    let mut c: int32_t = 0;
    let mut cc: int32_t = 0;
    let mut for_end: int32_t = (*mem.offset((p + 4 as int32_t) as isize)).b16.s1
        as int32_t - 1 as int32_t;
    i = 0 as ::core::ffi::c_int as int32_t;
    while i <= for_end {
        c = *(mem.offset((p + NATIVE_NODE_SIZE as int32_t) as isize) as *mut memory_word
            as *mut ::core::ffi::c_ushort)
            .offset(i as isize) as int32_t;
        if c >= 0xd800 as int32_t && c < 0xdc00 as int32_t {
            if i
                < (*mem.offset((p + 4 as int32_t) as isize)).b16.s1 as int32_t
                    - 1 as int32_t
            {
                cc = *(mem.offset((p + NATIVE_NODE_SIZE as int32_t) as isize)
                    as *mut memory_word as *mut ::core::ffi::c_ushort)
                    .offset((i + 1 as int32_t) as isize) as int32_t;
                if cc >= 0xdc00 as int32_t && cc < 0xe000 as int32_t {
                    c = 0x10000 as int32_t + (c - 0xd800 as int32_t) * 1024 as int32_t
                        + (cc - 0xdc00 as int32_t);
                    print_char(c);
                    i += 1;
                } else {
                    print('.' as i32);
                }
            } else {
                print('.' as i32);
            }
        } else {
            print_char(c);
        }
        i += 1;
    }
}
#[no_mangle]
pub unsafe extern "C" fn print_sa_num(mut q: int32_t) {
    let mut n: int32_t = 0;
    if ((*mem.offset(q as isize)).b16.s1 as ::core::ffi::c_int) < DIMEN_VAL_LIMIT {
        n = (*mem.offset((q + 1 as int32_t) as isize)).b32.s1;
    } else {
        n = ((*mem.offset(q as isize)).b16.s1 as ::core::ffi::c_int
            % 64 as ::core::ffi::c_int) as int32_t;
        q = (*mem.offset(q as isize)).b32.s1;
        n = n + 64 as int32_t * (*mem.offset(q as isize)).b16.s1 as int32_t;
        q = (*mem.offset(q as isize)).b32.s1;
        n = n
            + 64 as int32_t * 64 as int32_t
                * ((*mem.offset(q as isize)).b16.s1 as int32_t
                    + 64 as int32_t
                        * (*mem.offset((*mem.offset(q as isize)).b32.s1 as isize)).b16.s1
                            as int32_t);
    }
    print_int(n);
}
#[no_mangle]
pub unsafe extern "C" fn print_file_line() {
    let mut level: int32_t = in_open;
    while level > 0 as int32_t
        && *full_source_filename_stack.offset(level as isize) == 0 as str_number
    {
        level -= 1;
    }
    if level == 0 as int32_t {
        print_nl_cstr(b"! \0" as *const u8 as *const ::core::ffi::c_char);
    } else {
        print_nl_cstr(b"\0" as *const u8 as *const ::core::ffi::c_char);
        print(*full_source_filename_stack.offset(level as isize) as int32_t);
        print(':' as i32);
        if level == in_open {
            print_int(line);
        } else {
            print_int(*line_stack.offset((level + 1 as int32_t) as isize));
        }
        print_cstr(b": \0" as *const u8 as *const ::core::ffi::c_char);
    };
}
#[no_mangle]
pub unsafe extern "C" fn print_two(mut n: int32_t) {
    n = (abs(n as ::core::ffi::c_int) % 100 as ::core::ffi::c_int) as int32_t;
    print_char('0' as i32 + n / 10 as int32_t);
    print_char('0' as i32 + n % 10 as int32_t);
}
#[no_mangle]
pub unsafe extern "C" fn print_hex(mut n: int32_t) {
    let mut k: ::core::ffi::c_uchar = 0 as ::core::ffi::c_uchar;
    print_char('"' as i32);
    loop {
        dig[k as usize] = (n % 16 as int32_t) as ::core::ffi::c_uchar;
        n = n / 16 as int32_t;
        k = k.wrapping_add(1);
        if !(n != 0 as int32_t) {
            break;
        }
    }
    print_the_digs(k as eight_bits);
}
#[no_mangle]
pub unsafe extern "C" fn print_roman_int(mut n: int32_t) {
    let mut u: int32_t = 0;
    let mut v: int32_t = 0;
    let mut roman_data: *const ::core::ffi::c_char = b"m2d5c2l5x2v5i\0" as *const u8
        as *const ::core::ffi::c_char;
    let mut j: ::core::ffi::c_uchar = 0 as ::core::ffi::c_uchar;
    let mut k: ::core::ffi::c_uchar = 0 as ::core::ffi::c_uchar;
    v = 1000 as ::core::ffi::c_int as int32_t;
    loop {
        while n >= v {
            print_char(*roman_data.offset(j as isize) as int32_t);
            n = n - v;
        }
        if n <= 0 as int32_t {
            return;
        }
        k = (j as ::core::ffi::c_int + 2 as ::core::ffi::c_int) as ::core::ffi::c_uchar;
        u = v
            / (*roman_data
                .offset((k as ::core::ffi::c_int - 1 as ::core::ffi::c_int) as isize)
                as int32_t - '0' as i32);
        if *roman_data
            .offset((k as ::core::ffi::c_int - 1 as ::core::ffi::c_int) as isize)
            as ::core::ffi::c_int == '2' as i32
        {
            k = (k as ::core::ffi::c_int + 2 as ::core::ffi::c_int)
                as ::core::ffi::c_uchar;
            u = u
                / (*roman_data
                    .offset((k as ::core::ffi::c_int - 1 as ::core::ffi::c_int) as isize)
                    as int32_t - '0' as i32);
        }
        if n + u >= v {
            print_char(*roman_data.offset(k as isize) as int32_t);
            n = n + u;
        } else {
            j = (j as ::core::ffi::c_int + 2 as ::core::ffi::c_int)
                as ::core::ffi::c_uchar;
            v = v
                / (*roman_data
                    .offset((j as ::core::ffi::c_int - 1 as ::core::ffi::c_int) as isize)
                    as int32_t - '0' as i32);
        }
    };
}
#[no_mangle]
pub unsafe extern "C" fn print_current_string() {
    let mut j: pool_pointer = *str_start
        .offset((str_ptr - 0x10000 as str_number) as isize);
    while j < pool_ptr {
        print_char(*str_pool.offset(j as isize) as int32_t);
        j += 1;
    }
}
#[no_mangle]
pub unsafe extern "C" fn print_scaled(mut s: scaled_t) {
    let mut delta: scaled_t = 0;
    if s < 0 as scaled_t {
        print_char('-' as i32);
        s = -s as scaled_t;
    }
    print_int(s as int32_t / 0x10000 as int32_t);
    print_char('.' as i32);
    s = 10 as scaled_t * (s % 0x10000 as scaled_t) + 5 as scaled_t;
    delta = 10 as ::core::ffi::c_int as scaled_t;
    loop {
        if delta > 0x10000 as scaled_t {
            s = s + 0x8000 as scaled_t - 50000 as scaled_t;
        }
        print_char('0' as i32 + s as int32_t / 0x10000 as int32_t);
        s = 10 as scaled_t * (s % 0x10000 as scaled_t);
        delta = delta * 10 as scaled_t;
        if !(s > delta) {
            break;
        }
    };
}
#[no_mangle]
pub unsafe extern "C" fn print_ucs_code(mut n: UnicodeScalar) {
    let mut k: ::core::ffi::c_uchar = 0 as ::core::ffi::c_uchar;
    print_cstr(b"U+\0" as *const u8 as *const ::core::ffi::c_char);
    loop {
        dig[k as usize] = (n % 16 as UnicodeScalar) as ::core::ffi::c_uchar;
        n = n / 16 as UnicodeScalar;
        k = k.wrapping_add(1);
        if !(n != 0 as UnicodeScalar) {
            break;
        }
    }
    while (k as ::core::ffi::c_int) < 4 as ::core::ffi::c_int {
        dig[k as usize] = 0 as ::core::ffi::c_uchar;
        k = k.wrapping_add(1);
    }
    print_the_digs(k as eight_bits);
}
pub const true_0: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const false_0: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const BIGGEST_CHAR: ::core::ffi::c_int = 0xffff as ::core::ffi::c_int;
pub const BIGGEST_USV: ::core::ffi::c_int = 0x10ffff as ::core::ffi::c_int;
pub const NUMBER_USVS: ::core::ffi::c_int = BIGGEST_USV + 1 as ::core::ffi::c_int;
pub const NATIVE_NODE_SIZE: ::core::ffi::c_int = 6 as ::core::ffi::c_int;
pub const DIMEN_VAL_LIMIT: ::core::ffi::c_int = 128 as ::core::ffi::c_int;
