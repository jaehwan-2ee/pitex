/* engine-interface.c: programmatic interface to control the engine behavior
   Copyright 2016-2018 The Tectonic Project
   Licensed under the MIT License.
*/
// Translated from xetex/engine/xetex-engine-interface.c with C2Rust 0.22.1.
extern "C" {
    fn setjmp(_: *mut ::core::ffi::c_int) -> ::core::ffi::c_int;
    fn sigsetjmp(_: *mut ::core::ffi::c_int, _: ::core::ffi::c_int) -> ::core::ffi::c_int;
    fn strcmp(
        __s1: *const ::core::ffi::c_char,
        __s2: *const ::core::ffi::c_char,
    ) -> ::core::ffi::c_int;
    fn tt_run_engine(
        dump_name: *const ::core::ffi::c_char,
        input_file_name: *const ::core::ffi::c_char,
        build_date: time_t,
    ) -> tt_history_t;
    static mut in_initex_mode: bool;
    static mut shell_escape_enabled: bool;
    static mut halt_on_error_p: ::core::ffi::c_int;
    static mut synctex_enabled: bool;
    static mut synctex_use_gz: bool;
    static mut synctex_texpresso_extension: bool;
    static mut semantic_pagination_enabled: bool;
}
pub type __darwin_time_t = ::core::ffi::c_long;
pub type uint64_t = u64;
pub type jmp_buf = [::core::ffi::c_int; 48];
pub type sigjmp_buf = [::core::ffi::c_int; 49];
pub type time_t = __darwin_time_t;
pub type tt_history_t = ::core::ffi::c_uint;
pub const HISTORY_FATAL_ERROR: tt_history_t = 3;
pub const HISTORY_ERROR_ISSUED: tt_history_t = 2;
pub const HISTORY_WARNING_ISSUED: tt_history_t = 1;
pub const HISTORY_SPOTLESS: tt_history_t = 0;
#[inline]
unsafe extern "C" fn streq_ptr(
    mut s1: *const ::core::ffi::c_char,
    mut s2: *const ::core::ffi::c_char,
) -> bool {
    if !s1.is_null() && !s2.is_null() {
        return strcmp(s1, s2) == 0 as ::core::ffi::c_int;
    }
    return false_0 != 0;
}
#[no_mangle]
pub unsafe extern "C" fn tt_xetex_set_int_variable(
    mut var_name: *const ::core::ffi::c_char,
    mut value: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    if streq_ptr(
        var_name,
        b"halt_on_error_p\0" as *const u8 as *const ::core::ffi::c_char,
    ) {
        halt_on_error_p = value;
    } else if streq_ptr(
        var_name,
        b"in_initex_mode\0" as *const u8 as *const ::core::ffi::c_char,
    ) {
        in_initex_mode = value != 0 as ::core::ffi::c_int;
    } else if streq_ptr(
        var_name,
        b"synctex_enabled\0" as *const u8 as *const ::core::ffi::c_char,
    ) {
        synctex_enabled = value != 0 as ::core::ffi::c_int;
    } else if streq_ptr(
        var_name,
        b"synctex_use_gz\0" as *const u8 as *const ::core::ffi::c_char,
    ) {
        synctex_use_gz = value != 0 as ::core::ffi::c_int;
    } else if streq_ptr(
        var_name,
        b"synctex_texpresso_extension\0" as *const u8 as *const ::core::ffi::c_char,
    ) {
        synctex_texpresso_extension = value != 0 as ::core::ffi::c_int;
    } else if streq_ptr(
        var_name,
        b"semantic_pagination_enabled\0" as *const u8 as *const ::core::ffi::c_char,
    ) {
        semantic_pagination_enabled = value != 0 as ::core::ffi::c_int;
    } else if streq_ptr(
        var_name,
        b"shell_escape_enabled\0" as *const u8 as *const ::core::ffi::c_char,
    ) {
        shell_escape_enabled = value != 0 as ::core::ffi::c_int;
    } else {
        return 1 as ::core::ffi::c_int;
    }
    return 0 as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn tt_xetex_set_string_variable(
    mut var_name: *const ::core::ffi::c_char,
    mut value: *const ::core::ffi::c_char,
) -> ::core::ffi::c_int {
    return 1 as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn tt_engine_xetex_main(
    mut dump_name: *const ::core::ffi::c_char,
    mut input_file_name: *const ::core::ffi::c_char,
    mut build_date: uint64_t,
) -> ::core::ffi::c_int {
    let mut rv: ::core::ffi::c_int = 0;
    rv = tt_run_engine(dump_name, input_file_name, build_date as time_t) as ::core::ffi::c_int;
    return rv;
}
#[no_mangle]
pub unsafe extern "C" fn __terrible_aarch64_musl_linker_hack_never_call_me() {
    let mut buf1: jmp_buf = [0; 48];
    let mut buf2: sigjmp_buf = [0; 49];
    setjmp(&raw mut buf1 as *mut ::core::ffi::c_int);
    sigsetjmp(
        &raw mut buf2 as *mut ::core::ffi::c_int,
        0 as ::core::ffi::c_int,
    );
}
pub const false_0: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
