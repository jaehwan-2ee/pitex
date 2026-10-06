/* tectonic/errors.c -- error handling
 * Copyright 2016 the Tectonic Project
 * Licensed under the MIT License.
*/
// Translated from xetex/engine/xetex-errors.c with C2Rust 0.22.1.
extern "C" {
    pub type ttbc_output_handle_t;
    pub type ttbc_diagnostic_t;
    fn _tt_abort(format: *const ::core::ffi::c_char, ...) -> !;
    fn ttstub_output_flush(handle: rust_output_handle_t) -> ::core::ffi::c_int;
    static mut halt_on_error_p: ::core::ffi::c_int;
    static mut rust_stdout: rust_output_handle_t;
    static mut selector: selector_t;
    static mut interaction: ::core::ffi::c_uchar;
    static mut history: tt_history_t;
    static mut error_count: ::core::ffi::c_schar;
    static mut help_line: [*const ::core::ffi::c_char; 6];
    static mut help_ptr: ::core::ffi::c_uchar;
    static mut use_err_help: bool;
    static mut job_name: str_number;
    static mut log_opened: bool;
    fn show_context();
    fn open_log_file();
    fn give_err_help();
    fn close_files_and_terminate();
    fn capture_to_diagnostic(diagnostic: *mut ttbc_diagnostic_t);
    fn error_here_with_diagnostic(message: *const ::core::ffi::c_char) -> *mut ttbc_diagnostic_t;
    fn print_ln();
    fn print_char(s: int32_t);
    fn print(s: int32_t);
    fn print_cstr(s: *const ::core::ffi::c_char);
    fn print_nl_cstr(s: *const ::core::ffi::c_char);
    fn print_int(n: int32_t);
    fn tt_cleanup();
}
pub type int32_t = i32;
pub type rust_output_handle_t = *mut ttbc_output_handle_t;
pub type tt_history_t = ::core::ffi::c_uint;
pub const HISTORY_FATAL_ERROR: tt_history_t = 3;
pub const HISTORY_ERROR_ISSUED: tt_history_t = 2;
pub const HISTORY_WARNING_ISSUED: tt_history_t = 1;
pub const HISTORY_SPOTLESS: tt_history_t = 0;
pub type selector_t = ::core::ffi::c_uint;
pub const SELECTOR_NEW_STRING: selector_t = 21;
pub const SELECTOR_PSEUDO: selector_t = 20;
pub const SELECTOR_TERM_AND_LOG: selector_t = 19;
pub const SELECTOR_LOG_ONLY: selector_t = 18;
pub const SELECTOR_TERM_ONLY: selector_t = 17;
pub const SELECTOR_NO_PRINT: selector_t = 16;
pub const SELECTOR_FILE_15: selector_t = 15;
pub const SELECTOR_FILE_0: selector_t = 0;
pub type str_number = int32_t;
pub const __DARWIN_NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
pub const NULL: *mut ::core::ffi::c_void = __DARWIN_NULL;
pub const BATCH_MODE: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const SCROLL_MODE: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const ERROR_STOP_MODE: ::core::ffi::c_int = 3 as ::core::ffi::c_int;
unsafe extern "C" fn pre_error_message() {
    if log_opened {
        selector = SELECTOR_TERM_AND_LOG;
    } else {
        selector = SELECTOR_TERM_ONLY;
    }
    if job_name == 0 as str_number {
        open_log_file();
    }
    if interaction as ::core::ffi::c_int == BATCH_MODE {
        selector -= 1;
    }
    error_here_with_diagnostic(b"\0" as *const u8 as *const ::core::ffi::c_char);
}
unsafe extern "C" fn post_error_message(mut need_to_print_it: ::core::ffi::c_int) {
    capture_to_diagnostic(::core::ptr::null_mut::<ttbc_diagnostic_t>());
    if interaction as ::core::ffi::c_int == ERROR_STOP_MODE {
        interaction = SCROLL_MODE as ::core::ffi::c_uchar;
    }
    if need_to_print_it != 0 && log_opened as ::core::ffi::c_int != 0 {
        error();
    }
    history = HISTORY_FATAL_ERROR;
    close_files_and_terminate();
    tt_cleanup();
    ttstub_output_flush(rust_stdout);
}
#[no_mangle]
pub unsafe extern "C" fn error() {
    if (history as ::core::ffi::c_uint)
        < HISTORY_ERROR_ISSUED as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        history = HISTORY_ERROR_ISSUED;
    }
    print_char('.' as i32);
    show_context();
    if halt_on_error_p != 0 {
        history = HISTORY_FATAL_ERROR;
        post_error_message(0 as ::core::ffi::c_int);
        _tt_abort(
            b"halted on potentially-recoverable error as specified\0" as *const u8
                as *const ::core::ffi::c_char,
        );
    }
    error_count += 1;
    if error_count as ::core::ffi::c_int == 100 as ::core::ffi::c_int {
        print_nl_cstr(
            b"(That makes 100 errors; please try again.)\0" as *const u8
                as *const ::core::ffi::c_char,
        );
        history = HISTORY_FATAL_ERROR;
        post_error_message(0 as ::core::ffi::c_int);
        _tt_abort(
            b"halted after 100 potentially-recoverable errors\0" as *const u8
                as *const ::core::ffi::c_char,
        );
    }
    if interaction as ::core::ffi::c_int > BATCH_MODE {
        selector -= 1;
    }
    if use_err_help {
        print_ln();
        give_err_help();
    } else {
        while help_ptr as ::core::ffi::c_int > 0 as ::core::ffi::c_int {
            help_ptr = help_ptr.wrapping_sub(1);
            print_nl_cstr(help_line[help_ptr as usize]);
        }
    }
    print_ln();
    if interaction as ::core::ffi::c_int > BATCH_MODE {
        selector += 1;
    }
    print_ln();
}
#[no_mangle]
pub unsafe extern "C" fn fatal_error(mut s: *const ::core::ffi::c_char) -> ! {
    pre_error_message();
    print_cstr(b"Emergency stop\0" as *const u8 as *const ::core::ffi::c_char);
    print_nl_cstr(s);
    capture_to_diagnostic(::core::ptr::null_mut::<ttbc_diagnostic_t>());
    close_files_and_terminate();
    tt_cleanup();
    ttstub_output_flush(rust_stdout);
    _tt_abort(b"%s\0" as *const u8 as *const ::core::ffi::c_char, s);
}
#[no_mangle]
pub unsafe extern "C" fn overflow(mut s: *const ::core::ffi::c_char, mut n: int32_t) -> ! {
    pre_error_message();
    print_cstr(b"TeX capacity exceeded, sorry [\0" as *const u8 as *const ::core::ffi::c_char);
    print_cstr(s);
    print_char('=' as i32);
    print_int(n);
    print_char(']' as i32);
    help_ptr = 2 as ::core::ffi::c_uchar;
    help_line[1 as ::core::ffi::c_int as usize] = b"If you really absolutely need more capacity,\0"
        as *const u8
        as *const ::core::ffi::c_char;
    help_line[0 as ::core::ffi::c_int as usize] =
        b"you can ask a wizard to enlarge me.\0" as *const u8 as *const ::core::ffi::c_char;
    post_error_message(1 as ::core::ffi::c_int);
    _tt_abort(b"halted on overflow()\0" as *const u8 as *const ::core::ffi::c_char);
}
#[no_mangle]
pub unsafe extern "C" fn confusion(mut s: *const ::core::ffi::c_char) -> ! {
    pre_error_message();
    if (history as ::core::ffi::c_uint)
        < HISTORY_ERROR_ISSUED as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        print_cstr(b"This can't happen (\0" as *const u8 as *const ::core::ffi::c_char);
        print_cstr(s);
        print_char(')' as i32);
        help_ptr = 1 as ::core::ffi::c_uchar;
        help_line[0 as ::core::ffi::c_int as usize] =
            b"I'm broken. Please show this to someone who can fix can fix\0" as *const u8
                as *const ::core::ffi::c_char;
    } else {
        print_cstr(
            b"I can't go on meeting you like this\0" as *const u8 as *const ::core::ffi::c_char,
        );
        help_ptr = 2 as ::core::ffi::c_uchar;
        help_line[1 as ::core::ffi::c_int as usize] =
            b"One of your faux pas seems to have wounded me deeply...\0" as *const u8
                as *const ::core::ffi::c_char;
        help_line[0 as ::core::ffi::c_int as usize] =
            b"in fact, I'm barely conscious. Please fix it and try again.\0" as *const u8
                as *const ::core::ffi::c_char;
    }
    post_error_message(1 as ::core::ffi::c_int);
    _tt_abort(b"halted on confusion()\0" as *const u8 as *const ::core::ffi::c_char);
}
#[no_mangle]
pub unsafe extern "C" fn pdf_error(
    mut t: *const ::core::ffi::c_char,
    mut p: *const ::core::ffi::c_char,
) -> ! {
    pre_error_message();
    print_cstr(b"Error\0" as *const u8 as *const ::core::ffi::c_char);
    if !t.is_null() {
        print_cstr(b" (\0" as *const u8 as *const ::core::ffi::c_char);
        print_cstr(t);
        print(')' as i32);
    }
    print_cstr(b": \0" as *const u8 as *const ::core::ffi::c_char);
    print_cstr(p);
    post_error_message(1 as ::core::ffi::c_int);
    _tt_abort(b"halted on pdf_error()\0" as *const u8 as *const ::core::ffi::c_char);
}
