/* xetex-ini.c: WEB initialization code translated to C
   Copyright 2016-2022 The Tectonic Project
   Licensed under the MIT License.
*/
// Translated from xetex/engine/xetex-ini.c with C2Rust 0.22.1.
extern "C" {
    pub type Opaque_TECkit_Converter;
    pub type ttbc_input_handle_t;
    pub type ttbc_output_handle_t;
    pub type ttbc_diagnostic_t;
    fn TECkit_DisposeConverter(converter: TECkit_Converter) -> TECkit_Status;
    fn malloc(__size: size_t) -> *mut ::core::ffi::c_void;
    fn calloc(__count: size_t, __size: size_t) -> *mut ::core::ffi::c_void;
    fn free(_: *mut ::core::ffi::c_void);
    fn abs(_: ::core::ffi::c_int) -> ::core::ffi::c_int;
    fn memset(
        __b: *mut ::core::ffi::c_void,
        __c: ::core::ffi::c_int,
        __len: size_t,
    ) -> *mut ::core::ffi::c_void;
    fn strcpy(
        __dst: *mut ::core::ffi::c_char,
        __src: *const ::core::ffi::c_char,
    ) -> *mut ::core::ffi::c_char;
    fn strlen(__s: *const ::core::ffi::c_char) -> size_t;
    fn _tt_abort(format: *const ::core::ffi::c_char, ...) -> !;
    fn ttstub_output_open_format(
        path: *const ::core::ffi::c_char,
        is_gz: ::core::ffi::c_int,
    ) -> rust_output_handle_t;
    fn ttstub_output_open_stdout() -> rust_output_handle_t;
    fn ttstub_output_write(
        handle: rust_output_handle_t,
        data: *const ::core::ffi::c_char,
        len: size_t,
    ) -> size_t;
    fn ttstub_output_flush(handle: rust_output_handle_t) -> ::core::ffi::c_int;
    fn ttstub_output_close(handle: rust_output_handle_t) -> ::core::ffi::c_int;
    fn ttstub_input_open(
        path: *const ::core::ffi::c_char,
        format: ttbc_file_format,
        is_gz: ::core::ffi::c_int,
    ) -> rust_input_handle_t;
    fn ttstub_input_read(
        handle: rust_input_handle_t,
        data: *mut ::core::ffi::c_char,
        len: size_t,
    ) -> ssize_t;
    fn ttstub_input_close(handle: rust_input_handle_t) -> ::core::ffi::c_int;
    fn set_cp_code(
        fontNum: ::core::ffi::c_int,
        code: ::core::ffi::c_uint,
        side: ::core::ffi::c_int,
        value: ::core::ffi::c_int,
    );
    fn destroy_font_manager();
    fn release_font_engine(engine: *mut ::core::ffi::c_void, type_flag: ::core::ffi::c_int);
    fn maketexstring(s: *const ::core::ffi::c_char) -> ::core::ffi::c_int;
    fn init_start_time(source_date_epoch: time_t);
    fn get_date_and_time(
        source_date_epoch: time_t,
        minutes: *mut int32_t,
        day: *mut int32_t,
        month: *mut int32_t,
        year: *mut int32_t,
    );
    fn get_seconds_and_micros(seconds: *mut int32_t, micros: *mut int32_t);
    fn u_close(f: *mut UFILE);
    fn get_avail() -> int32_t;
    fn flush_list(p: int32_t);
    fn get_node(s: int32_t) -> int32_t;
    fn free_node(p: int32_t, s: int32_t);
    fn delete_token_ref(p: int32_t);
    fn delete_glue_ref(p: int32_t);
    fn flush_node_list(p: int32_t);
    fn print_cmd_chr(cmd: uint16_t, chr_code: int32_t);
    fn id_lookup(j: int32_t, l: int32_t) -> int32_t;
    fn prim_lookup(s: str_number) -> int32_t;
    fn pseudo_close();
    fn sa_def(p: int32_t, e: int32_t);
    fn gsa_def(p: int32_t, e: int32_t);
    fn eq_define(p: int32_t, t: uint16_t, e: int32_t);
    fn eq_word_define(p: int32_t, w: int32_t);
    fn geq_define(p: int32_t, t: uint16_t, e: int32_t);
    fn geq_word_define(p: int32_t, w: int32_t);
    fn show_cur_cmd_chr();
    fn end_token_list();
    fn back_input();
    fn back_error();
    fn end_file_reading();
    fn get_token();
    fn find_sa_element(t: small_number, n: int32_t, w: bool);
    fn get_x_token();
    fn scan_left_brace();
    fn scan_optional_equals();
    fn scan_keyword(s: *const ::core::ffi::c_char) -> bool;
    fn scan_glyph_number(f: internal_font_number);
    fn scan_char_class();
    fn scan_char_class_not_ignored();
    fn scan_usv_num();
    fn scan_char_num();
    fn scan_xetex_math_char_int();
    fn scan_math_class_int();
    fn scan_math_fam_int();
    fn scan_fifteen_bit_int();
    fn scan_register_num();
    fn scan_font_ident();
    fn find_font_dimen(writing: bool);
    fn scan_int();
    fn scan_dimen(mu: bool, inf: bool, shortcut: bool);
    fn scan_glue(level: small_number);
    fn scan_toks(macro_def: bool, xpand: bool) -> int32_t;
    fn read_toks(n: int32_t, r: int32_t, j: int32_t);
    fn make_name_string() -> str_number;
    fn pack_job_name(_: *const ::core::ffi::c_char);
    fn open_log_file();
    fn start_input(primary_input_name: *const ::core::ffi::c_char);
    fn max_hyphenatable_length() -> int32_t;
    fn show_save_groups();
    fn do_marks(a: small_number, l: small_number, q: int32_t) -> bool;
    fn scan_box(box_context: int32_t);
    fn get_r_token();
    fn trap_zero_glue();
    fn do_register_command(a: small_number);
    fn alter_aux();
    fn alter_prev_graf();
    fn alter_page_so_far();
    fn alter_integer();
    fn alter_box_dimen();
    fn new_font(a: small_number);
    fn new_interaction();
    fn main_control();
    fn close_files_and_terminate();
    fn error();
    fn overflow(s: *const ::core::ffi::c_char, n: int32_t) -> !;
    fn confusion(s: *const ::core::ffi::c_char) -> !;
    fn initialize_math_variables();
    fn capture_to_diagnostic(diagnostic: *mut ttbc_diagnostic_t);
    fn error_here_with_diagnostic(message: *const ::core::ffi::c_char) -> *mut ttbc_diagnostic_t;
    fn print_ln();
    fn print_char(s: int32_t);
    fn print(s: int32_t);
    fn print_cstr(s: *const ::core::ffi::c_char);
    fn print_nl(s: str_number);
    fn print_nl_cstr(s: *const ::core::ffi::c_char);
    fn print_esc(s: str_number);
    fn print_esc_cstr(s: *const ::core::ffi::c_char);
    fn print_int(n: int32_t);
    fn print_file_name(n: int32_t, a: int32_t, e: int32_t);
    fn print_scaled(s: scaled_t);
    fn initialize_pagebuilder_variables();
    fn init_randoms(seed: int32_t);
    fn initialize_shipout_variables();
    fn deinitialize_shipout_variables();
    fn load_pool_strings(spare_size: int32_t) -> ::core::ffi::c_int;
    fn length(s: str_number) -> int32_t;
    fn make_string() -> str_number;
    fn synctex_init_command();
}
pub type TECkit_Status = ::core::ffi::c_long;
pub type TECkit_Converter = *mut Opaque_TECkit_Converter;
pub type __darwin_size_t = usize;
pub type __darwin_ssize_t = isize;
pub type __darwin_time_t = ::core::ffi::c_long;
pub type size_t = __darwin_size_t;
pub type int32_t = i32;
pub type uint16_t = u16;
pub type uintptr_t = usize;
pub type ssize_t = __darwin_ssize_t;
pub type time_t = __darwin_time_t;
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
pub type tt_history_t = ::core::ffi::c_uint;
pub const HISTORY_FATAL_ERROR: tt_history_t = 3;
pub const HISTORY_ERROR_ISSUED: tt_history_t = 2;
pub const HISTORY_WARNING_ISSUED: tt_history_t = 1;
pub const HISTORY_SPOTLESS: tt_history_t = 0;
pub type trie_pointer = int32_t;
pub type nine_bits = int32_t;
pub type font_index = int32_t;
pub type UTF16_code = ::core::ffi::c_ushort;
pub type str_number = int32_t;
pub type scaled_t = int32_t;
pub type b16x4 = b16x4_le_t;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct b16x4_le_t {
    pub s0: uint16_t,
    pub s1: uint16_t,
    pub s2: uint16_t,
    pub s3: uint16_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union memory_word {
    pub b32: b32x2,
    pub b16: b16x4,
    pub gr: ::core::ffi::c_double,
    pub ptr: *mut ::core::ffi::c_void,
}
pub type b32x2 = b32x2_le_t;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct b32x2_le_t {
    pub s0: int32_t,
    pub s1: int32_t,
}
pub type packed_UTF16_code = ::core::ffi::c_ushort;
pub type pool_pointer = int32_t;
pub type hyph_pointer = ::core::ffi::c_ushort;
pub type save_pointer = int32_t;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct UFILE {
    pub handle: rust_input_handle_t,
    pub savedChar: ::core::ffi::c_long,
    pub skipNextLF: ::core::ffi::c_short,
    pub encodingMode: ::core::ffi::c_short,
    pub conversionData: *mut ::core::ffi::c_void,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct input_state_t {
    pub state: uint16_t,
    pub index: uint16_t,
    pub start: int32_t,
    pub loc: int32_t,
    pub limit: int32_t,
    pub name: int32_t,
    pub synctex_tag: int32_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct list_state_record {
    pub mode: ::core::ffi::c_short,
    pub head: int32_t,
    pub tail: int32_t,
    pub eTeX_aux: int32_t,
    pub prev_graf: int32_t,
    pub mode_line: int32_t,
    pub aux: memory_word,
}
pub type UnicodeScalar = int32_t;
pub type trie_opcode = ::core::ffi::c_ushort;
pub type small_number = ::core::ffi::c_short;
pub type internal_font_number = int32_t;
pub type selector_t = ::core::ffi::c_uint;
pub const SELECTOR_NEW_STRING: selector_t = 21;
pub const SELECTOR_PSEUDO: selector_t = 20;
pub const SELECTOR_TERM_AND_LOG: selector_t = 19;
pub const SELECTOR_LOG_ONLY: selector_t = 18;
pub const SELECTOR_TERM_ONLY: selector_t = 17;
pub const SELECTOR_NO_PRINT: selector_t = 16;
pub const SELECTOR_FILE_15: selector_t = 15;
pub const SELECTOR_FILE_0: selector_t = 0;
pub type UTF8_code = ::core::ffi::c_uchar;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct xetex_format_primitive_def_t {
    pub name: *const ::core::ffi::c_char,
    pub cmd: eight_bits,
    pub chr: int32_t,
    pub extra_init: int32_t,
}
pub type eight_bits = ::core::ffi::c_uchar;
pub const xf_prim_init_none: xetex_format_primitive_extra_init_t = 0;
pub const xf_prim_init_write: xetex_format_primitive_extra_init_t = 2;
pub const xf_prim_init_par: xetex_format_primitive_extra_init_t = 1;
pub type glue_ord = ::core::ffi::c_uchar;
pub type group_code = ::core::ffi::c_uchar;
pub type xetex_format_primitive_extra_init_t = ::core::ffi::c_uint;
pub const __DARWIN_NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
pub const INT32_MAX: ::core::ffi::c_int = 2147483647 as ::core::ffi::c_int;
pub const UINT16_MAX: ::core::ffi::c_int = 65535 as ::core::ffi::c_int;
pub const NULL: *mut ::core::ffi::c_void = __DARWIN_NULL;
pub const MIN_HALFWORD: ::core::ffi::c_int = -(0xfffffff as ::core::ffi::c_int);
pub const MAX_HALFWORD: ::core::ffi::c_int = 0x3fffffff as ::core::ffi::c_int;
pub const TEX_NULL: ::core::ffi::c_int = MIN_HALFWORD;
pub const BIGGEST_CHAR: ::core::ffi::c_int = 0xffff as ::core::ffi::c_int;
pub const BIGGEST_USV: ::core::ffi::c_int = 0x10ffff as ::core::ffi::c_int;
pub const NUMBER_USVS: ::core::ffi::c_int = BIGGEST_USV + 1 as ::core::ffi::c_int;
pub const NUMBER_MATH_FAMILIES: ::core::ffi::c_int = 256 as ::core::ffi::c_int;
pub const NUMBER_MATH_FONTS: ::core::ffi::c_int = 3 as ::core::ffi::c_int * NUMBER_MATH_FAMILIES;
pub const NUMBER_REGS: ::core::ffi::c_int = 256 as ::core::ffi::c_int;
pub const PAGE_INS_HEAD: ::core::ffi::c_int = MEM_TOP;
pub const CONTRIB_HEAD: ::core::ffi::c_int = MEM_TOP - 1 as ::core::ffi::c_int;
pub const PAGE_HEAD: ::core::ffi::c_int = MEM_TOP - 2 as ::core::ffi::c_int;
pub const ACTIVE_LIST: ::core::ffi::c_int = MEM_TOP - 7 as ::core::ffi::c_int;
pub const END_SPAN: ::core::ffi::c_int = MEM_TOP - 9 as ::core::ffi::c_int;
pub const OMIT_TEMPLATE: ::core::ffi::c_int = MEM_TOP - 10 as ::core::ffi::c_int;
pub const NULL_LIST: ::core::ffi::c_int = MEM_TOP - 11 as ::core::ffi::c_int;
pub const GARBAGE: ::core::ffi::c_int = MEM_TOP - 12 as ::core::ffi::c_int;
pub const PRE_ADJUST_HEAD: ::core::ffi::c_int = MEM_TOP - 14 as ::core::ffi::c_int;
pub const FROZEN_PROTECTION: ::core::ffi::c_int = FROZEN_CONTROL_SEQUENCE + 0 as ::core::ffi::c_int;
pub const FROZEN_CR: ::core::ffi::c_int = FROZEN_CONTROL_SEQUENCE + 1 as ::core::ffi::c_int;
pub const FROZEN_END_GROUP: ::core::ffi::c_int = FROZEN_CONTROL_SEQUENCE + 2 as ::core::ffi::c_int;
pub const FROZEN_RIGHT: ::core::ffi::c_int = FROZEN_CONTROL_SEQUENCE + 3 as ::core::ffi::c_int;
pub const FROZEN_FI: ::core::ffi::c_int = FROZEN_CONTROL_SEQUENCE + 4 as ::core::ffi::c_int;
pub const FROZEN_END_TEMPLATE: ::core::ffi::c_int =
    FROZEN_CONTROL_SEQUENCE + 5 as ::core::ffi::c_int;
pub const FROZEN_ENDV: ::core::ffi::c_int = FROZEN_CONTROL_SEQUENCE + 6 as ::core::ffi::c_int;
pub const FROZEN_RELAX: ::core::ffi::c_int = FROZEN_CONTROL_SEQUENCE + 7 as ::core::ffi::c_int;
pub const END_WRITE: ::core::ffi::c_int = FROZEN_CONTROL_SEQUENCE + 8 as ::core::ffi::c_int;
pub const FROZEN_DONT_EXPAND: ::core::ffi::c_int =
    FROZEN_CONTROL_SEQUENCE + 9 as ::core::ffi::c_int;
pub const FROZEN_SPECIAL: ::core::ffi::c_int = FROZEN_CONTROL_SEQUENCE + 10 as ::core::ffi::c_int;
pub const FROZEN_PRIMITIVE: ::core::ffi::c_int = FROZEN_CONTROL_SEQUENCE + 11 as ::core::ffi::c_int;
pub const FONT_ID_BASE: ::core::ffi::c_int = FROZEN_NULL_FONT;
pub const LEVEL_ZERO: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const LEVEL_ONE: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const LEFT_TO_RIGHT: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const IF_NODE_SIZE: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const BOTTOM_LEVEL: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const FIL: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const FILL: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const NORMAL: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const BEGIN_L_CODE: ::core::ffi::c_int = 6 as ::core::ffi::c_int;
pub const END_L_CODE: ::core::ffi::c_int = 7 as ::core::ffi::c_int;
pub const BEGIN_R_CODE: ::core::ffi::c_int = 10 as ::core::ffi::c_int;
pub const END_R_CODE: ::core::ffi::c_int = 11 as ::core::ffi::c_int;
pub const NEW_LINE: ::core::ffi::c_int = 33 as ::core::ffi::c_int;
pub const EMPTY: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const FONT_BASE: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const NON_ADDRESS: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const TOKEN_LIST: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const HYPHENATED: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const SPLIT_UP: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const CLOSED: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const MU_VAL: ::core::ffi::c_int = 3 as ::core::ffi::c_int;
pub const TOK_VAL: ::core::ffi::c_int = 5 as ::core::ffi::c_int;
pub const INTER_CHAR_VAL: ::core::ffi::c_int = 6 as ::core::ffi::c_int;
pub const MARK_VAL: ::core::ffi::c_int = 7 as ::core::ffi::c_int;
pub const CARRIAGE_RETURN: ::core::ffi::c_int = 13 as ::core::ffi::c_int;
pub const HI_MEM_STAT_USAGE: ::core::ffi::c_int = 15 as ::core::ffi::c_int;
pub const MAX_CHAR_CODE: ::core::ffi::c_int = 15 as ::core::ffi::c_int;
pub const VRULE: ::core::ffi::c_int = 35 as ::core::ffi::c_int;
pub const BIGGEST_LANG: ::core::ffi::c_int = 255 as ::core::ffi::c_int;
pub const TOO_BIG_LANG: ::core::ffi::c_int = 256 as ::core::ffi::c_int;
pub const HYPH_PRIME: ::core::ffi::c_int = 607 as ::core::ffi::c_int;
pub const CHAR_CLASS_LIMIT: ::core::ffi::c_int = 4096 as ::core::ffi::c_int;
pub const TOO_BIG_CHAR: ::core::ffi::c_int = 65536 as ::core::ffi::c_int;
pub const ACTIVE_MATH_CHAR: ::core::ffi::c_int = 0x1fffff as ::core::ffi::c_int;
pub const CS_TOKEN_FLAG: ::core::ffi::c_int = 0x1ffffff as ::core::ffi::c_int;
pub const LEFT_BRACE_TOKEN: ::core::ffi::c_int = 0x200000 as ::core::ffi::c_int;
pub const RIGHT_BRACE_TOKEN: ::core::ffi::c_int = 0x400000 as ::core::ffi::c_int;
pub const OTHER_TOKEN: ::core::ffi::c_int = 0x1800000 as ::core::ffi::c_int;
pub const END_MATCH_TOKEN: ::core::ffi::c_int = 0x1c00000 as ::core::ffi::c_int;
pub const PROTECTED_TOKEN: ::core::ffi::c_int = END_MATCH_TOKEN + 1 as ::core::ffi::c_int;
pub const BOX_FLAG: ::core::ffi::c_int = 0x40000000 as ::core::ffi::c_int;
pub const GLOBAL_BOX_FLAG: ::core::ffi::c_int = 0x40008000 as ::core::ffi::c_int;
pub const LP_CODE_BASE: ::core::ffi::c_int = 2;
pub const RP_CODE_BASE: ::core::ffi::c_int = 3;
pub const IGNORE_DEPTH: ::core::ffi::c_int = -(65536000 as ::core::ffi::c_int);
#[inline]
unsafe extern "C" fn mfree(mut ptr: *mut ::core::ffi::c_void) -> *mut ::core::ffi::c_void {
    free(ptr);
    return NULL;
}
pub const LEFT_SIDE: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const RIGHT_SIDE: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const AAT_FONT_FLAG: ::core::ffi::c_uint = 0xffff as ::core::ffi::c_uint;
pub const OTGR_FONT_FLAG: ::core::ffi::c_uint = 0xfffe as ::core::ffi::c_uint;
pub const HASH_SIZE: ::core::ffi::c_int = 15000 as ::core::ffi::c_int;
pub const HASH_PRIME: ::core::ffi::c_int = 8501 as ::core::ffi::c_int;
pub const TOO_BIG_USV: ::core::ffi::c_int = 1114112 as ::core::ffi::c_int;
pub const PRIM_SIZE: ::core::ffi::c_int = 2100 as ::core::ffi::c_int;
pub const MAX_FONT_MAX: ::core::ffi::c_int = 9000 as ::core::ffi::c_int;
pub const MEM_TOP: ::core::ffi::c_int = 4999999 as ::core::ffi::c_int;
pub const INT_PAR__pretolerance: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const INT_PAR__tolerance: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const INT_PAR__line_penalty: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const INT_PAR__hyphen_penalty: ::core::ffi::c_int = 3 as ::core::ffi::c_int;
pub const INT_PAR__ex_hyphen_penalty: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
pub const INT_PAR__club_penalty: ::core::ffi::c_int = 5 as ::core::ffi::c_int;
pub const INT_PAR__widow_penalty: ::core::ffi::c_int = 6 as ::core::ffi::c_int;
pub const INT_PAR__display_widow_penalty: ::core::ffi::c_int = 7 as ::core::ffi::c_int;
pub const INT_PAR__broken_penalty: ::core::ffi::c_int = 8 as ::core::ffi::c_int;
pub const INT_PAR__bin_op_penalty: ::core::ffi::c_int = 9 as ::core::ffi::c_int;
pub const INT_PAR__rel_penalty: ::core::ffi::c_int = 10 as ::core::ffi::c_int;
pub const INT_PAR__pre_display_penalty: ::core::ffi::c_int = 11 as ::core::ffi::c_int;
pub const INT_PAR__post_display_penalty: ::core::ffi::c_int = 12 as ::core::ffi::c_int;
pub const INT_PAR__inter_line_penalty: ::core::ffi::c_int = 13 as ::core::ffi::c_int;
pub const INT_PAR__double_hyphen_demerits: ::core::ffi::c_int = 14 as ::core::ffi::c_int;
pub const INT_PAR__final_hyphen_demerits: ::core::ffi::c_int = 15 as ::core::ffi::c_int;
pub const INT_PAR__adj_demerits: ::core::ffi::c_int = 16 as ::core::ffi::c_int;
pub const INT_PAR__mag: ::core::ffi::c_int = 17 as ::core::ffi::c_int;
pub const INT_PAR__delimiter_factor: ::core::ffi::c_int = 18 as ::core::ffi::c_int;
pub const INT_PAR__looseness: ::core::ffi::c_int = 19 as ::core::ffi::c_int;
pub const INT_PAR__time: ::core::ffi::c_int = 20 as ::core::ffi::c_int;
pub const INT_PAR__day: ::core::ffi::c_int = 21 as ::core::ffi::c_int;
pub const INT_PAR__month: ::core::ffi::c_int = 22 as ::core::ffi::c_int;
pub const INT_PAR__year: ::core::ffi::c_int = 23 as ::core::ffi::c_int;
pub const INT_PAR__show_box_breadth: ::core::ffi::c_int = 24 as ::core::ffi::c_int;
pub const INT_PAR__show_box_depth: ::core::ffi::c_int = 25 as ::core::ffi::c_int;
pub const INT_PAR__hbadness: ::core::ffi::c_int = 26 as ::core::ffi::c_int;
pub const INT_PAR__vbadness: ::core::ffi::c_int = 27 as ::core::ffi::c_int;
pub const INT_PAR__pausing: ::core::ffi::c_int = 28 as ::core::ffi::c_int;
pub const INT_PAR__tracing_online: ::core::ffi::c_int = 29 as ::core::ffi::c_int;
pub const INT_PAR__tracing_macros: ::core::ffi::c_int = 30 as ::core::ffi::c_int;
pub const INT_PAR__tracing_stats: ::core::ffi::c_int = 31 as ::core::ffi::c_int;
pub const INT_PAR__tracing_paragraphs: ::core::ffi::c_int = 32 as ::core::ffi::c_int;
pub const INT_PAR__tracing_pages: ::core::ffi::c_int = 33 as ::core::ffi::c_int;
pub const INT_PAR__tracing_output: ::core::ffi::c_int = 34 as ::core::ffi::c_int;
pub const INT_PAR__tracing_lost_chars: ::core::ffi::c_int = 35 as ::core::ffi::c_int;
pub const INT_PAR__tracing_commands: ::core::ffi::c_int = 36 as ::core::ffi::c_int;
pub const INT_PAR__tracing_restores: ::core::ffi::c_int = 37 as ::core::ffi::c_int;
pub const INT_PAR__uc_hyph: ::core::ffi::c_int = 38 as ::core::ffi::c_int;
pub const INT_PAR__output_penalty: ::core::ffi::c_int = 39 as ::core::ffi::c_int;
pub const INT_PAR__max_dead_cycles: ::core::ffi::c_int = 40 as ::core::ffi::c_int;
pub const INT_PAR__hang_after: ::core::ffi::c_int = 41 as ::core::ffi::c_int;
pub const INT_PAR__floating_penalty: ::core::ffi::c_int = 42 as ::core::ffi::c_int;
pub const INT_PAR__global_defs: ::core::ffi::c_int = 43 as ::core::ffi::c_int;
pub const INT_PAR__cur_fam: ::core::ffi::c_int = 44 as ::core::ffi::c_int;
pub const INT_PAR__escape_char: ::core::ffi::c_int = 45 as ::core::ffi::c_int;
pub const INT_PAR__default_hyphen_char: ::core::ffi::c_int = 46 as ::core::ffi::c_int;
pub const INT_PAR__default_skew_char: ::core::ffi::c_int = 47 as ::core::ffi::c_int;
pub const INT_PAR__end_line_char: ::core::ffi::c_int = 48 as ::core::ffi::c_int;
pub const INT_PAR__new_line_char: ::core::ffi::c_int = 49 as ::core::ffi::c_int;
pub const INT_PAR__language: ::core::ffi::c_int = 50 as ::core::ffi::c_int;
pub const INT_PAR__left_hyphen_min: ::core::ffi::c_int = 51 as ::core::ffi::c_int;
pub const INT_PAR__right_hyphen_min: ::core::ffi::c_int = 52 as ::core::ffi::c_int;
pub const INT_PAR__holding_inserts: ::core::ffi::c_int = 53 as ::core::ffi::c_int;
pub const INT_PAR__error_context_lines: ::core::ffi::c_int = 54 as ::core::ffi::c_int;
pub const INT_PAR__tracing_stack_levels: ::core::ffi::c_int = 55 as ::core::ffi::c_int;
pub const INT_PAR__tracing_assigns: ::core::ffi::c_int = 56 as ::core::ffi::c_int;
pub const INT_PAR__tracing_groups: ::core::ffi::c_int = 57 as ::core::ffi::c_int;
pub const INT_PAR__tracing_ifs: ::core::ffi::c_int = 58 as ::core::ffi::c_int;
pub const INT_PAR__tracing_scan_tokens: ::core::ffi::c_int = 59 as ::core::ffi::c_int;
pub const INT_PAR__tracing_nesting: ::core::ffi::c_int = 60 as ::core::ffi::c_int;
pub const INT_PAR__pre_display_direction: ::core::ffi::c_int = 61 as ::core::ffi::c_int;
pub const INT_PAR__last_line_fit: ::core::ffi::c_int = 62 as ::core::ffi::c_int;
pub const INT_PAR__saving_vdiscards: ::core::ffi::c_int = 63 as ::core::ffi::c_int;
pub const INT_PAR__saving_hyph_codes: ::core::ffi::c_int = 64 as ::core::ffi::c_int;
pub const INT_PAR__suppress_fontnotfound_error: ::core::ffi::c_int = 65 as ::core::ffi::c_int;
pub const INT_PAR__xetex_linebreak_penalty: ::core::ffi::c_int = 67 as ::core::ffi::c_int;
pub const INT_PAR__xetex_protrude_chars: ::core::ffi::c_int = 68 as ::core::ffi::c_int;
pub const INT_PAR__texxet: ::core::ffi::c_int = 69 as ::core::ffi::c_int;
pub const INT_PAR__xetex_dash_break: ::core::ffi::c_int = 70 as ::core::ffi::c_int;
pub const INT_PAR__xetex_upwards: ::core::ffi::c_int = 71 as ::core::ffi::c_int;
pub const INT_PAR__xetex_use_glyph_metrics: ::core::ffi::c_int = 72 as ::core::ffi::c_int;
pub const INT_PAR__xetex_inter_char_tokens: ::core::ffi::c_int = 73 as ::core::ffi::c_int;
pub const INT_PAR__xetex_input_normalization: ::core::ffi::c_int = 74 as ::core::ffi::c_int;
pub const INT_PAR__xetex_tracing_fonts: ::core::ffi::c_int = 77 as ::core::ffi::c_int;
pub const INT_PAR__xetex_interword_space_shaping: ::core::ffi::c_int = 78 as ::core::ffi::c_int;
pub const INT_PAR__xetex_generate_actual_text: ::core::ffi::c_int = 79 as ::core::ffi::c_int;
pub const INT_PAR__xetex_hyphenatable_length: ::core::ffi::c_int = 80 as ::core::ffi::c_int;
pub const INT_PAR__synctex: ::core::ffi::c_int = 81 as ::core::ffi::c_int;
pub const INT_PAR__pdfoutput: ::core::ffi::c_int = 82 as ::core::ffi::c_int;
pub const INT_PAR__partoken_context: ::core::ffi::c_int = 83 as ::core::ffi::c_int;
pub const INT_PAR__ignore_primitive_error: ::core::ffi::c_int = 84;
pub const INT_PAR__pitex_font_expansion: ::core::ffi::c_int = 85;
pub const INT_PAR__pitex_font_stretch: ::core::ffi::c_int = 86;
pub const INT_PAR__pitex_font_shrink: ::core::ffi::c_int = 87;
pub const INT_PAR__pitex_font_step: ::core::ffi::c_int = 88;
pub const INT_PARS: ::core::ffi::c_int = 89 as ::core::ffi::c_int;
pub const DIMEN_PAR__par_indent: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const DIMEN_PAR__math_surround: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const DIMEN_PAR__line_skip_limit: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const DIMEN_PAR__hsize: ::core::ffi::c_int = 3 as ::core::ffi::c_int;
pub const DIMEN_PAR__vsize: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
pub const DIMEN_PAR__max_depth: ::core::ffi::c_int = 5 as ::core::ffi::c_int;
pub const DIMEN_PAR__split_max_depth: ::core::ffi::c_int = 6 as ::core::ffi::c_int;
pub const DIMEN_PAR__box_max_depth: ::core::ffi::c_int = 7 as ::core::ffi::c_int;
pub const DIMEN_PAR__hfuzz: ::core::ffi::c_int = 8 as ::core::ffi::c_int;
pub const DIMEN_PAR__vfuzz: ::core::ffi::c_int = 9 as ::core::ffi::c_int;
pub const DIMEN_PAR__delimiter_shortfall: ::core::ffi::c_int = 10 as ::core::ffi::c_int;
pub const DIMEN_PAR__null_delimiter_space: ::core::ffi::c_int = 11 as ::core::ffi::c_int;
pub const DIMEN_PAR__script_space: ::core::ffi::c_int = 12 as ::core::ffi::c_int;
pub const DIMEN_PAR__pre_display_size: ::core::ffi::c_int = 13 as ::core::ffi::c_int;
pub const DIMEN_PAR__display_width: ::core::ffi::c_int = 14 as ::core::ffi::c_int;
pub const DIMEN_PAR__display_indent: ::core::ffi::c_int = 15 as ::core::ffi::c_int;
pub const DIMEN_PAR__overfull_rule: ::core::ffi::c_int = 16 as ::core::ffi::c_int;
pub const DIMEN_PAR__hang_indent: ::core::ffi::c_int = 17 as ::core::ffi::c_int;
pub const DIMEN_PAR__h_offset: ::core::ffi::c_int = 18 as ::core::ffi::c_int;
pub const DIMEN_PAR__v_offset: ::core::ffi::c_int = 19 as ::core::ffi::c_int;
pub const DIMEN_PAR__emergency_stretch: ::core::ffi::c_int = 20 as ::core::ffi::c_int;
pub const DIMEN_PAR__pdf_page_width: ::core::ffi::c_int = 21 as ::core::ffi::c_int;
pub const DIMEN_PAR__pdf_page_height: ::core::ffi::c_int = 22 as ::core::ffi::c_int;
pub const DIMEN_PARS: ::core::ffi::c_int = 23 as ::core::ffi::c_int;
pub const GLUE_PAR__line_skip: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const GLUE_PAR__baseline_skip: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const GLUE_PAR__par_skip: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const GLUE_PAR__above_display_skip: ::core::ffi::c_int = 3 as ::core::ffi::c_int;
pub const GLUE_PAR__below_display_skip: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
pub const GLUE_PAR__above_display_short_skip: ::core::ffi::c_int = 5 as ::core::ffi::c_int;
pub const GLUE_PAR__below_display_short_skip: ::core::ffi::c_int = 6 as ::core::ffi::c_int;
pub const GLUE_PAR__left_skip: ::core::ffi::c_int = 7 as ::core::ffi::c_int;
pub const GLUE_PAR__right_skip: ::core::ffi::c_int = 8 as ::core::ffi::c_int;
pub const GLUE_PAR__top_skip: ::core::ffi::c_int = 9 as ::core::ffi::c_int;
pub const GLUE_PAR__split_top_skip: ::core::ffi::c_int = 10 as ::core::ffi::c_int;
pub const GLUE_PAR__tab_skip: ::core::ffi::c_int = 11 as ::core::ffi::c_int;
pub const GLUE_PAR__space_skip: ::core::ffi::c_int = 12 as ::core::ffi::c_int;
pub const GLUE_PAR__xspace_skip: ::core::ffi::c_int = 13 as ::core::ffi::c_int;
pub const GLUE_PAR__par_fill_skip: ::core::ffi::c_int = 14 as ::core::ffi::c_int;
pub const GLUE_PAR__xetex_linebreak_skip: ::core::ffi::c_int = 15 as ::core::ffi::c_int;
pub const GLUE_PAR__thin_mu_skip: ::core::ffi::c_int = 16 as ::core::ffi::c_int;
pub const GLUE_PAR__med_mu_skip: ::core::ffi::c_int = 17 as ::core::ffi::c_int;
pub const GLUE_PAR__thick_mu_skip: ::core::ffi::c_int = 18 as ::core::ffi::c_int;
pub const LOCAL__par_shape: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const LOCAL__output_routine: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const LOCAL__every_par: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const LOCAL__every_math: ::core::ffi::c_int = 3 as ::core::ffi::c_int;
pub const LOCAL__every_display: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
pub const LOCAL__every_hbox: ::core::ffi::c_int = 5 as ::core::ffi::c_int;
pub const LOCAL__every_vbox: ::core::ffi::c_int = 6 as ::core::ffi::c_int;
pub const LOCAL__every_job: ::core::ffi::c_int = 7 as ::core::ffi::c_int;
pub const LOCAL__every_cr: ::core::ffi::c_int = 8 as ::core::ffi::c_int;
pub const LOCAL__err_help: ::core::ffi::c_int = 9 as ::core::ffi::c_int;
pub const LOCAL__every_eof: ::core::ffi::c_int = 10 as ::core::ffi::c_int;
pub const LOCAL__xetex_inter_char_toks: ::core::ffi::c_int = 11 as ::core::ffi::c_int;
pub const LOCAL__tectonic_coda_tokens: ::core::ffi::c_int = 12 as ::core::ffi::c_int;
pub const ETEX_PENALTIES_PAR__inter_line_penalties: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const ETEX_PENALTIES_PAR__club_penalties: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const ETEX_PENALTIES_PAR__widow_penalties: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const ETEX_PENALTIES_PAR__display_widow_penalties: ::core::ffi::c_int = 3 as ::core::ffi::c_int;
pub const NUM_ETEX_PENALTIES: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
pub const ACTIVE_BASE: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const SINGLE_BASE: ::core::ffi::c_int = 1114113 as ::core::ffi::c_int;
pub const HASH_BASE: ::core::ffi::c_int = 2228226 as ::core::ffi::c_int;
pub const FROZEN_CONTROL_SEQUENCE: ::core::ffi::c_int = 2243226 as ::core::ffi::c_int;
pub const PRIM_EQTB_BASE: ::core::ffi::c_int = 2243238 as ::core::ffi::c_int;
pub const FROZEN_NULL_FONT: ::core::ffi::c_int = 2245338 as ::core::ffi::c_int;
pub const UNDEFINED_CONTROL_SEQUENCE: ::core::ffi::c_int = 2254339 as ::core::ffi::c_int;
pub const GLUE_BASE: ::core::ffi::c_int = 2254340 as ::core::ffi::c_int;
pub const SKIP_BASE: ::core::ffi::c_int = 2254359 as ::core::ffi::c_int;
pub const MU_SKIP_BASE: ::core::ffi::c_int = 2254615 as ::core::ffi::c_int;
pub const LOCAL_BASE: ::core::ffi::c_int = 2254871 as ::core::ffi::c_int;
pub const TOKS_BASE: ::core::ffi::c_int = 2254884 as ::core::ffi::c_int;
pub const ETEX_PEN_BASE: ::core::ffi::c_int = 2255140 as ::core::ffi::c_int;
pub const BOX_BASE: ::core::ffi::c_int = 2255144 as ::core::ffi::c_int;
pub const CUR_FONT_LOC: ::core::ffi::c_int = 2255400 as ::core::ffi::c_int;
pub const MATH_FONT_BASE: ::core::ffi::c_int = 2255401 as ::core::ffi::c_int;
pub const CAT_CODE_BASE: ::core::ffi::c_int = 2256169 as ::core::ffi::c_int;
pub const LC_CODE_BASE: ::core::ffi::c_int = 3370281 as ::core::ffi::c_int;
pub const UC_CODE_BASE: ::core::ffi::c_int = 4484393 as ::core::ffi::c_int;
pub const SF_CODE_BASE: ::core::ffi::c_int = 5598505 as ::core::ffi::c_int;
pub const MATH_CODE_BASE: ::core::ffi::c_int = 6712617 as ::core::ffi::c_int;
pub const INT_BASE: ::core::ffi::c_int = 7826729 as ::core::ffi::c_int;
pub const COUNT_BASE: ::core::ffi::c_int = INT_BASE + INT_PARS;
pub const DEL_CODE_BASE: ::core::ffi::c_int = COUNT_BASE + 256 as ::core::ffi::c_int;
pub const DIMEN_BASE: ::core::ffi::c_int = DEL_CODE_BASE + NUMBER_USVS;
pub const SCALED_BASE: ::core::ffi::c_int = DIMEN_BASE + DIMEN_PARS;
pub const EQTB_SIZE: ::core::ffi::c_int = SCALED_BASE + 255 as ::core::ffi::c_int;
pub const RELAX: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const ESCAPE: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const LEFT_BRACE: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const RIGHT_BRACE: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const TAB_MARK: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
pub const CAR_RET: ::core::ffi::c_int = 5 as ::core::ffi::c_int;
pub const ENDV: ::core::ffi::c_int = 9 as ::core::ffi::c_int;
pub const IGNORE: ::core::ffi::c_int = 9 as ::core::ffi::c_int;
pub const SPACER: ::core::ffi::c_int = 10 as ::core::ffi::c_int;
pub const LETTER: ::core::ffi::c_int = 11 as ::core::ffi::c_int;
pub const OTHER_CHAR: ::core::ffi::c_int = 12 as ::core::ffi::c_int;
pub const PAR_END: ::core::ffi::c_int = 13 as ::core::ffi::c_int;
pub const STOP: ::core::ffi::c_int = 14 as ::core::ffi::c_int;
pub const COMMENT: ::core::ffi::c_int = 14 as ::core::ffi::c_int;
pub const DELIM_NUM: ::core::ffi::c_int = 15 as ::core::ffi::c_int;
pub const INVALID_CHAR: ::core::ffi::c_int = 15 as ::core::ffi::c_int;
pub const CHAR_NUM: ::core::ffi::c_int = 16 as ::core::ffi::c_int;
pub const MATH_CHAR_NUM: ::core::ffi::c_int = 17 as ::core::ffi::c_int;
pub const MARK: ::core::ffi::c_int = 18 as ::core::ffi::c_int;
pub const XRAY: ::core::ffi::c_int = 19 as ::core::ffi::c_int;
pub const MAKE_BOX: ::core::ffi::c_int = 20 as ::core::ffi::c_int;
pub const HMOVE: ::core::ffi::c_int = 21 as ::core::ffi::c_int;
pub const VMOVE: ::core::ffi::c_int = 22 as ::core::ffi::c_int;
pub const UN_HBOX: ::core::ffi::c_int = 23 as ::core::ffi::c_int;
pub const UN_VBOX: ::core::ffi::c_int = 24 as ::core::ffi::c_int;
pub const REMOVE_ITEM: ::core::ffi::c_int = 25 as ::core::ffi::c_int;
pub const HSKIP: ::core::ffi::c_int = 26 as ::core::ffi::c_int;
pub const VSKIP: ::core::ffi::c_int = 27 as ::core::ffi::c_int;
pub const MSKIP: ::core::ffi::c_int = 28 as ::core::ffi::c_int;
pub const KERN: ::core::ffi::c_int = 29 as ::core::ffi::c_int;
pub const MKERN: ::core::ffi::c_int = 30 as ::core::ffi::c_int;
pub const LEADER_SHIP: ::core::ffi::c_int = 31 as ::core::ffi::c_int;
pub const HALIGN: ::core::ffi::c_int = 32 as ::core::ffi::c_int;
pub const VALIGN: ::core::ffi::c_int = 33 as ::core::ffi::c_int;
pub const NO_ALIGN: ::core::ffi::c_int = 34 as ::core::ffi::c_int;
pub const HRULE: ::core::ffi::c_int = 36 as ::core::ffi::c_int;
pub const INSERT: ::core::ffi::c_int = 37 as ::core::ffi::c_int;
pub const VADJUST: ::core::ffi::c_int = 38 as ::core::ffi::c_int;
pub const IGNORE_SPACES: ::core::ffi::c_int = 39 as ::core::ffi::c_int;
pub const AFTER_ASSIGNMENT: ::core::ffi::c_int = 40 as ::core::ffi::c_int;
pub const AFTER_GROUP: ::core::ffi::c_int = 41 as ::core::ffi::c_int;
pub const BREAK_PENALTY: ::core::ffi::c_int = 42 as ::core::ffi::c_int;
pub const START_PAR: ::core::ffi::c_int = 43 as ::core::ffi::c_int;
pub const ITAL_CORR: ::core::ffi::c_int = 44 as ::core::ffi::c_int;
pub const ACCENT: ::core::ffi::c_int = 45 as ::core::ffi::c_int;
pub const MATH_ACCENT: ::core::ffi::c_int = 46 as ::core::ffi::c_int;
pub const DISCRETIONARY: ::core::ffi::c_int = 47 as ::core::ffi::c_int;
pub const EQ_NO: ::core::ffi::c_int = 48 as ::core::ffi::c_int;
pub const LEFT_RIGHT: ::core::ffi::c_int = 49 as ::core::ffi::c_int;
pub const MATH_COMP: ::core::ffi::c_int = 50 as ::core::ffi::c_int;
pub const LIMIT_SWITCH: ::core::ffi::c_int = 51 as ::core::ffi::c_int;
pub const ABOVE: ::core::ffi::c_int = 52 as ::core::ffi::c_int;
pub const MATH_STYLE: ::core::ffi::c_int = 53 as ::core::ffi::c_int;
pub const MATH_CHOICE: ::core::ffi::c_int = 54 as ::core::ffi::c_int;
pub const NON_SCRIPT: ::core::ffi::c_int = 55 as ::core::ffi::c_int;
pub const VCENTER: ::core::ffi::c_int = 56 as ::core::ffi::c_int;
pub const CASE_SHIFT: ::core::ffi::c_int = 57 as ::core::ffi::c_int;
pub const MESSAGE: ::core::ffi::c_int = 58 as ::core::ffi::c_int;
pub const EXTENSION: ::core::ffi::c_int = 59 as ::core::ffi::c_int;
pub const IN_STREAM: ::core::ffi::c_int = 60 as ::core::ffi::c_int;
pub const BEGIN_GROUP: ::core::ffi::c_int = 61 as ::core::ffi::c_int;
pub const END_GROUP: ::core::ffi::c_int = 62 as ::core::ffi::c_int;
pub const OMIT: ::core::ffi::c_int = 63 as ::core::ffi::c_int;
pub const EX_SPACE: ::core::ffi::c_int = 64 as ::core::ffi::c_int;
pub const NO_BOUNDARY: ::core::ffi::c_int = 65 as ::core::ffi::c_int;
pub const RADICAL: ::core::ffi::c_int = 66 as ::core::ffi::c_int;
pub const END_CS_NAME: ::core::ffi::c_int = 67 as ::core::ffi::c_int;
pub const CHAR_GIVEN: ::core::ffi::c_int = 68 as ::core::ffi::c_int;
pub const MATH_GIVEN: ::core::ffi::c_int = 69 as ::core::ffi::c_int;
pub const XETEX_MATH_GIVEN: ::core::ffi::c_int = 70 as ::core::ffi::c_int;
pub const LAST_ITEM: ::core::ffi::c_int = 71 as ::core::ffi::c_int;
pub const TOKS_REGISTER: ::core::ffi::c_int = 72 as ::core::ffi::c_int;
pub const MAX_NON_PREFIXED_COMMAND: ::core::ffi::c_int = 71 as ::core::ffi::c_int;
pub const ASSIGN_TOKS: ::core::ffi::c_int = 73 as ::core::ffi::c_int;
pub const ASSIGN_INT: ::core::ffi::c_int = 74 as ::core::ffi::c_int;
pub const ASSIGN_DIMEN: ::core::ffi::c_int = 75 as ::core::ffi::c_int;
pub const ASSIGN_GLUE: ::core::ffi::c_int = 76 as ::core::ffi::c_int;
pub const ASSIGN_MU_GLUE: ::core::ffi::c_int = 77 as ::core::ffi::c_int;
pub const ASSIGN_FONT_DIMEN: ::core::ffi::c_int = 78 as ::core::ffi::c_int;
pub const ASSIGN_FONT_INT: ::core::ffi::c_int = 79 as ::core::ffi::c_int;
pub const SET_AUX: ::core::ffi::c_int = 80 as ::core::ffi::c_int;
pub const SET_PREV_GRAF: ::core::ffi::c_int = 81 as ::core::ffi::c_int;
pub const SET_PAGE_DIMEN: ::core::ffi::c_int = 82 as ::core::ffi::c_int;
pub const SET_PAGE_INT: ::core::ffi::c_int = 83 as ::core::ffi::c_int;
pub const SET_BOX_DIMEN: ::core::ffi::c_int = 84 as ::core::ffi::c_int;
pub const SET_SHAPE: ::core::ffi::c_int = 85 as ::core::ffi::c_int;
pub const DEF_CODE: ::core::ffi::c_int = 86 as ::core::ffi::c_int;
pub const XETEX_DEF_CODE: ::core::ffi::c_int = 87 as ::core::ffi::c_int;
pub const DEF_FAMILY: ::core::ffi::c_int = 88 as ::core::ffi::c_int;
pub const SET_FONT: ::core::ffi::c_int = 89 as ::core::ffi::c_int;
pub const DEF_FONT: ::core::ffi::c_int = 90 as ::core::ffi::c_int;
pub const REGISTER: ::core::ffi::c_int = 91 as ::core::ffi::c_int;
pub const ADVANCE: ::core::ffi::c_int = 92 as ::core::ffi::c_int;
pub const MULTIPLY: ::core::ffi::c_int = 93 as ::core::ffi::c_int;
pub const DIVIDE: ::core::ffi::c_int = 94 as ::core::ffi::c_int;
pub const PREFIX: ::core::ffi::c_int = 95 as ::core::ffi::c_int;
pub const LET: ::core::ffi::c_int = 96 as ::core::ffi::c_int;
pub const SHORTHAND_DEF: ::core::ffi::c_int = 97 as ::core::ffi::c_int;
pub const READ_TO_CS: ::core::ffi::c_int = 98 as ::core::ffi::c_int;
pub const DEF: ::core::ffi::c_int = 99 as ::core::ffi::c_int;
pub const SET_BOX: ::core::ffi::c_int = 100 as ::core::ffi::c_int;
pub const HYPH_DATA: ::core::ffi::c_int = 101 as ::core::ffi::c_int;
pub const SET_INTERACTION: ::core::ffi::c_int = 102 as ::core::ffi::c_int;
pub const UNDEFINED_CS: ::core::ffi::c_int = 103 as ::core::ffi::c_int;
pub const EXPAND_AFTER: ::core::ffi::c_int = 104 as ::core::ffi::c_int;
pub const NO_EXPAND: ::core::ffi::c_int = 105 as ::core::ffi::c_int;
pub const INPUT: ::core::ffi::c_int = 106 as ::core::ffi::c_int;
pub const IF_TEST: ::core::ffi::c_int = 107 as ::core::ffi::c_int;
pub const FI_OR_ELSE: ::core::ffi::c_int = 108 as ::core::ffi::c_int;
pub const CS_NAME: ::core::ffi::c_int = 109 as ::core::ffi::c_int;
pub const CONVERT: ::core::ffi::c_int = 110 as ::core::ffi::c_int;
pub const THE: ::core::ffi::c_int = 111 as ::core::ffi::c_int;
pub const TOP_BOT_MARK: ::core::ffi::c_int = 112 as ::core::ffi::c_int;
pub const CALL: ::core::ffi::c_int = 113 as ::core::ffi::c_int;
pub const OUTER_CALL: ::core::ffi::c_int = 115 as ::core::ffi::c_int;
pub const END_TEMPLATE: ::core::ffi::c_int = 117 as ::core::ffi::c_int;
pub const DONT_EXPAND: ::core::ffi::c_int = 118 as ::core::ffi::c_int;
pub const GLUE_REF: ::core::ffi::c_int = 119 as ::core::ffi::c_int;
pub const SHAPE_REF: ::core::ffi::c_int = 120 as ::core::ffi::c_int;
pub const BOX_REF: ::core::ffi::c_int = 121 as ::core::ffi::c_int;
pub const DATA: ::core::ffi::c_int = 122 as ::core::ffi::c_int;
pub const VMODE: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const HMODE: ::core::ffi::c_int = 104 as ::core::ffi::c_int;
pub const GLUE_NODE: ::core::ffi::c_int = 10 as ::core::ffi::c_int;
pub const KERN_NODE: ::core::ffi::c_int = 11 as ::core::ffi::c_int;
pub const PENALTY_NODE: ::core::ffi::c_int = 12 as ::core::ffi::c_int;
pub const TT_LEFT_RIGHT_MIDDLE_MODE: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const ORD_NOAD: ::core::ffi::c_int = 16 as ::core::ffi::c_int;
pub const OP_NOAD: ::core::ffi::c_int = 17 as ::core::ffi::c_int;
pub const BIN_NOAD: ::core::ffi::c_int = 18 as ::core::ffi::c_int;
pub const REL_NOAD: ::core::ffi::c_int = 19 as ::core::ffi::c_int;
pub const OPEN_NOAD: ::core::ffi::c_int = 20 as ::core::ffi::c_int;
pub const CLOSE_NOAD: ::core::ffi::c_int = 21 as ::core::ffi::c_int;
pub const PUNCT_NOAD: ::core::ffi::c_int = 22 as ::core::ffi::c_int;
pub const INNER_NOAD: ::core::ffi::c_int = 23 as ::core::ffi::c_int;
pub const UNDER_NOAD: ::core::ffi::c_int = 26 as ::core::ffi::c_int;
pub const OVER_NOAD: ::core::ffi::c_int = 27 as ::core::ffi::c_int;
pub const LEFT_NOAD: ::core::ffi::c_int = 30 as ::core::ffi::c_int;
pub const RIGHT_NOAD: ::core::ffi::c_int = 31 as ::core::ffi::c_int;
pub const MU_GLUE: ::core::ffi::c_int = 99 as ::core::ffi::c_int;
pub const A_LEADERS: ::core::ffi::c_int = 100 as ::core::ffi::c_int;
pub const C_LEADERS: ::core::ffi::c_int = 101 as ::core::ffi::c_int;
pub const X_LEADERS: ::core::ffi::c_int = 102 as ::core::ffi::c_int;
pub const EXPLICIT: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const BEFORE: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const DISPLAY_STYLE: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const TEXT_STYLE: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const SCRIPT_STYLE: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
pub const SCRIPT_SCRIPT_STYLE: ::core::ffi::c_int = 6 as ::core::ffi::c_int;
pub const LIMITS: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const NO_LIMITS: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const OPEN_NODE: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const WRITE_NODE: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const CLOSE_NODE: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const SPECIAL_NODE: ::core::ffi::c_int = 3 as ::core::ffi::c_int;
pub const PDF_SAVE_POS_NODE: ::core::ffi::c_int = 21 as ::core::ffi::c_int;
pub const ABOVE_CODE: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const OVER_CODE: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const ATOP_CODE: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const TT_ABOVE_WITH_DELIMS: ::core::ffi::c_int = 3 as ::core::ffi::c_int;
pub const TT_OVER_WITH_DELIMS: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
pub const TT_ATOP_WITH_DELIMS: ::core::ffi::c_int = 5 as ::core::ffi::c_int;
pub const BOX_CODE: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const COPY_CODE: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const LAST_BOX_CODE: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const VSPLIT_CODE: ::core::ffi::c_int = 3 as ::core::ffi::c_int;
pub const VTOP_CODE: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
pub const TT_VBOX_CODE: ::core::ffi::c_int = 5 as ::core::ffi::c_int;
pub const TT_HBOX_CODE: ::core::ffi::c_int = 108 as ::core::ffi::c_int;
pub const NUMBER_CODE: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const ROMAN_NUMERAL_CODE: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const STRING_CODE: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const MEANING_CODE: ::core::ffi::c_int = 3 as ::core::ffi::c_int;
pub const FONT_NAME_CODE: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
pub const ETEX_REVISION_CODE: ::core::ffi::c_int = 5 as ::core::ffi::c_int;
pub const EXPANDED_CODE: ::core::ffi::c_int = 6 as ::core::ffi::c_int;
pub const LEFT_MARGIN_KERN_CODE: ::core::ffi::c_int = 16 as ::core::ffi::c_int;
pub const RIGHT_MARGIN_KERN_CODE: ::core::ffi::c_int = 17 as ::core::ffi::c_int;
pub const PDF_STRCMP_CODE: ::core::ffi::c_int = 18 as ::core::ffi::c_int;
pub const PDF_CREATION_DATE_CODE: ::core::ffi::c_int = 22 as ::core::ffi::c_int;
pub const PDF_FILE_MOD_DATE_CODE: ::core::ffi::c_int = 23 as ::core::ffi::c_int;
pub const PDF_FILE_SIZE_CODE: ::core::ffi::c_int = 24 as ::core::ffi::c_int;
pub const PDF_MDFIVE_SUM_CODE: ::core::ffi::c_int = 25 as ::core::ffi::c_int;
pub const PDF_FILE_DUMP_CODE: ::core::ffi::c_int = 26 as ::core::ffi::c_int;
pub const UNIFORM_DEVIATE_CODE: ::core::ffi::c_int = 29 as ::core::ffi::c_int;
pub const NORMAL_DEVIATE_CODE: ::core::ffi::c_int = 30 as ::core::ffi::c_int;
pub const XETEX_REVISION_CODE: ::core::ffi::c_int = 33 as ::core::ffi::c_int;
pub const XETEX_VARIATION_NAME_CODE: ::core::ffi::c_int = 34 as ::core::ffi::c_int;
pub const XETEX_FEATURE_NAME_CODE: ::core::ffi::c_int = 35 as ::core::ffi::c_int;
pub const XETEX_SELECTOR_NAME_CODE: ::core::ffi::c_int = 36 as ::core::ffi::c_int;
pub const XETEX_GLYPH_NAME_CODE: ::core::ffi::c_int = 37 as ::core::ffi::c_int;
pub const XETEX_UCHAR_CODE: ::core::ffi::c_int = 38 as ::core::ffi::c_int;
pub const XETEX_UCHARCAT_CODE: ::core::ffi::c_int = 39 as ::core::ffi::c_int;
pub const JOB_NAME_CODE: ::core::ffi::c_int = 40 as ::core::ffi::c_int;
pub const IMMEDIATE_CODE: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
pub const SET_LANGUAGE_CODE: ::core::ffi::c_int = 5 as ::core::ffi::c_int;
pub const RESET_TIMER_CODE: ::core::ffi::c_int = 31 as ::core::ffi::c_int;
pub const SET_RANDOM_SEED_CODE: ::core::ffi::c_int = 33 as ::core::ffi::c_int;
pub const PIC_FILE_CODE: ::core::ffi::c_int = 41 as ::core::ffi::c_int;
pub const PDF_FILE_CODE: ::core::ffi::c_int = 42 as ::core::ffi::c_int;
pub const GLYPH_CODE: ::core::ffi::c_int = 43 as ::core::ffi::c_int;
pub const XETEX_INPUT_ENCODING_EXTENSION_CODE: ::core::ffi::c_int = 44 as ::core::ffi::c_int;
pub const XETEX_DEFAULT_ENCODING_EXTENSION_CODE: ::core::ffi::c_int = 45 as ::core::ffi::c_int;
pub const XETEX_LINEBREAK_LOCALE_EXTENSION_CODE: ::core::ffi::c_int = 46 as ::core::ffi::c_int;
pub const FI_CODE: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const ELSE_CODE: ::core::ffi::c_int = 3 as ::core::ffi::c_int;
pub const OR_CODE: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
pub const IF_CHAR_CODE: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const IF_CAT_CODE: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const IF_INT_CODE: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const IF_DIM_CODE: ::core::ffi::c_int = 3 as ::core::ffi::c_int;
pub const IF_ODD_CODE: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
pub const IF_VMODE_CODE: ::core::ffi::c_int = 5 as ::core::ffi::c_int;
pub const IF_HMODE_CODE: ::core::ffi::c_int = 6 as ::core::ffi::c_int;
pub const IF_MMODE_CODE: ::core::ffi::c_int = 7 as ::core::ffi::c_int;
pub const IF_INNER_CODE: ::core::ffi::c_int = 8 as ::core::ffi::c_int;
pub const IF_VOID_CODE: ::core::ffi::c_int = 9 as ::core::ffi::c_int;
pub const IF_HBOX_CODE: ::core::ffi::c_int = 10 as ::core::ffi::c_int;
pub const IF_VBOX_CODE: ::core::ffi::c_int = 11 as ::core::ffi::c_int;
pub const IFX_CODE: ::core::ffi::c_int = 12 as ::core::ffi::c_int;
pub const IF_EOF_CODE: ::core::ffi::c_int = 13 as ::core::ffi::c_int;
pub const IF_TRUE_CODE: ::core::ffi::c_int = 14 as ::core::ffi::c_int;
pub const IF_FALSE_CODE: ::core::ffi::c_int = 15 as ::core::ffi::c_int;
pub const IF_CASE_CODE: ::core::ffi::c_int = 16 as ::core::ffi::c_int;
pub const IF_DEF_CODE: ::core::ffi::c_int = 17 as ::core::ffi::c_int;
pub const IF_CS_CODE: ::core::ffi::c_int = 18 as ::core::ffi::c_int;
pub const IF_FONT_CHAR_CODE: ::core::ffi::c_int = 19 as ::core::ffi::c_int;
pub const IF_IN_CSNAME_CODE: ::core::ffi::c_int = 20 as ::core::ffi::c_int;
pub const IF_PRIMITIVE_CODE: ::core::ffi::c_int = 21 as ::core::ffi::c_int;
pub const INT_VAL: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const DIMEN_VAL: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const GLUE_VAL: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const LAST_NODE_TYPE_CODE: ::core::ffi::c_int = 3 as ::core::ffi::c_int;
pub const INPUT_LINE_NO_CODE: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
pub const BADNESS_CODE: ::core::ffi::c_int = 5 as ::core::ffi::c_int;
pub const PDF_LAST_X_POS_CODE: ::core::ffi::c_int = 12 as ::core::ffi::c_int;
pub const PDF_LAST_Y_POS_CODE: ::core::ffi::c_int = 13 as ::core::ffi::c_int;
pub const ELAPSED_TIME_CODE: ::core::ffi::c_int = 16 as ::core::ffi::c_int;
pub const PDF_SHELL_ESCAPE_CODE: ::core::ffi::c_int = 17 as ::core::ffi::c_int;
pub const RANDOM_SEED_CODE: ::core::ffi::c_int = 18 as ::core::ffi::c_int;
pub const ETEX_VERSION_CODE: ::core::ffi::c_int = 19 as ::core::ffi::c_int;
pub const CURRENT_GROUP_LEVEL_CODE: ::core::ffi::c_int = 20 as ::core::ffi::c_int;
pub const CURRENT_GROUP_TYPE_CODE: ::core::ffi::c_int = 21 as ::core::ffi::c_int;
pub const CURRENT_IF_LEVEL_CODE: ::core::ffi::c_int = 22 as ::core::ffi::c_int;
pub const CURRENT_IF_TYPE_CODE: ::core::ffi::c_int = 23 as ::core::ffi::c_int;
pub const CURRENT_IF_BRANCH_CODE: ::core::ffi::c_int = 24 as ::core::ffi::c_int;
pub const GLUE_STRETCH_ORDER_CODE: ::core::ffi::c_int = 25 as ::core::ffi::c_int;
pub const GLUE_SHRINK_ORDER_CODE: ::core::ffi::c_int = 26 as ::core::ffi::c_int;
pub const XETEX_VERSION_CODE: ::core::ffi::c_int = 27 as ::core::ffi::c_int;
pub const XETEX_COUNT_GLYPHS_CODE: ::core::ffi::c_int = 28 as ::core::ffi::c_int;
pub const XETEX_COUNT_VARIATIONS_CODE: ::core::ffi::c_int = 29 as ::core::ffi::c_int;
pub const XETEX_VARIATION_CODE: ::core::ffi::c_int = 30 as ::core::ffi::c_int;
pub const XETEX_FIND_VARIATION_BY_NAME_CODE: ::core::ffi::c_int = 31 as ::core::ffi::c_int;
pub const XETEX_VARIATION_MIN_CODE: ::core::ffi::c_int = 32 as ::core::ffi::c_int;
pub const XETEX_VARIATION_MAX_CODE: ::core::ffi::c_int = 33 as ::core::ffi::c_int;
pub const XETEX_VARIATION_DEFAULT_CODE: ::core::ffi::c_int = 34 as ::core::ffi::c_int;
pub const XETEX_COUNT_FEATURES_CODE: ::core::ffi::c_int = 35 as ::core::ffi::c_int;
pub const XETEX_FEATURE_CODE_CODE: ::core::ffi::c_int = 36 as ::core::ffi::c_int;
pub const XETEX_FIND_FEATURE_BY_NAME_CODE: ::core::ffi::c_int = 37 as ::core::ffi::c_int;
pub const XETEX_IS_EXCLUSIVE_FEATURE_CODE: ::core::ffi::c_int = 38 as ::core::ffi::c_int;
pub const XETEX_COUNT_SELECTORS_CODE: ::core::ffi::c_int = 39 as ::core::ffi::c_int;
pub const XETEX_SELECTOR_CODE_CODE: ::core::ffi::c_int = 40 as ::core::ffi::c_int;
pub const XETEX_FIND_SELECTOR_BY_NAME_CODE: ::core::ffi::c_int = 41 as ::core::ffi::c_int;
pub const XETEX_IS_DEFAULT_SELECTOR_CODE: ::core::ffi::c_int = 42 as ::core::ffi::c_int;
pub const XETEX_OT_COUNT_SCRIPTS_CODE: ::core::ffi::c_int = 43 as ::core::ffi::c_int;
pub const XETEX_OT_COUNT_LANGUAGES_CODE: ::core::ffi::c_int = 44 as ::core::ffi::c_int;
pub const XETEX_OT_COUNT_FEATURES_CODE: ::core::ffi::c_int = 45 as ::core::ffi::c_int;
pub const XETEX_OT_SCRIPT_CODE: ::core::ffi::c_int = 46 as ::core::ffi::c_int;
pub const XETEX_OT_LANGUAGE_CODE: ::core::ffi::c_int = 47 as ::core::ffi::c_int;
pub const XETEX_OT_FEATURE_CODE: ::core::ffi::c_int = 48 as ::core::ffi::c_int;
pub const XETEX_MAP_CHAR_TO_GLYPH_CODE: ::core::ffi::c_int = 49 as ::core::ffi::c_int;
pub const XETEX_GLYPH_INDEX_CODE: ::core::ffi::c_int = 50 as ::core::ffi::c_int;
pub const XETEX_FONT_TYPE_CODE: ::core::ffi::c_int = 51 as ::core::ffi::c_int;
pub const XETEX_FIRST_CHAR_CODE: ::core::ffi::c_int = 52 as ::core::ffi::c_int;
pub const XETEX_LAST_CHAR_CODE: ::core::ffi::c_int = 53 as ::core::ffi::c_int;
pub const XETEX_PDF_PAGE_COUNT_CODE: ::core::ffi::c_int = 54 as ::core::ffi::c_int;
pub const XETEX_GLYPH_BOUNDS_CODE: ::core::ffi::c_int = 55 as ::core::ffi::c_int;
pub const FONT_CHAR_WD_CODE: ::core::ffi::c_int = 56 as ::core::ffi::c_int;
pub const FONT_CHAR_HT_CODE: ::core::ffi::c_int = 57 as ::core::ffi::c_int;
pub const FONT_CHAR_DP_CODE: ::core::ffi::c_int = 58 as ::core::ffi::c_int;
pub const FONT_CHAR_IC_CODE: ::core::ffi::c_int = 59 as ::core::ffi::c_int;
pub const PAR_SHAPE_LENGTH_CODE: ::core::ffi::c_int = 60 as ::core::ffi::c_int;
pub const PAR_SHAPE_INDENT_CODE: ::core::ffi::c_int = 61 as ::core::ffi::c_int;
pub const PAR_SHAPE_DIMEN_CODE: ::core::ffi::c_int = 62 as ::core::ffi::c_int;
pub const GLUE_STRETCH_CODE: ::core::ffi::c_int = 63 as ::core::ffi::c_int;
pub const GLUE_SHRINK_CODE: ::core::ffi::c_int = 64 as ::core::ffi::c_int;
pub const MU_TO_GLUE_CODE: ::core::ffi::c_int = 65 as ::core::ffi::c_int;
pub const GLUE_TO_MU_CODE: ::core::ffi::c_int = 66 as ::core::ffi::c_int;
pub const TT_ETEX_NUM_EXPR_CODE: ::core::ffi::c_int = 67 as ::core::ffi::c_int;
pub const TT_ETEX_DIM_EXPR_CODE: ::core::ffi::c_int = 68 as ::core::ffi::c_int;
pub const TT_ETEX_GLUE_EXPR_CODE: ::core::ffi::c_int = 69 as ::core::ffi::c_int;
pub const TT_ETEX_MU_EXPR_CODE: ::core::ffi::c_int = 70 as ::core::ffi::c_int;
pub const BATCH_MODE: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const NONSTOP_MODE: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const SCROLL_MODE: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const ERROR_STOP_MODE: ::core::ffi::c_int = 3 as ::core::ffi::c_int;
pub const WIDTH_OFFSET: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const DEPTH_OFFSET: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const HEIGHT_OFFSET: ::core::ffi::c_int = 3 as ::core::ffi::c_int;
pub const CHAR_DEF_CODE: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const MATH_CHAR_DEF_CODE: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const COUNT_DEF_CODE: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const DIMEN_DEF_CODE: ::core::ffi::c_int = 3 as ::core::ffi::c_int;
pub const SKIP_DEF_CODE: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
pub const MU_SKIP_DEF_CODE: ::core::ffi::c_int = 5 as ::core::ffi::c_int;
pub const TOKS_DEF_CODE: ::core::ffi::c_int = 6 as ::core::ffi::c_int;
pub const CHAR_SUB_DEF_CODE: ::core::ffi::c_int = 7 as ::core::ffi::c_int;
pub const XETEX_MATH_CHAR_NUM_DEF_CODE: ::core::ffi::c_int = 8 as ::core::ffi::c_int;
pub const XETEX_MATH_CHAR_DEF_CODE: ::core::ffi::c_int = 9 as ::core::ffi::c_int;
pub const FIL_CODE: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const FILL_CODE: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const SS_CODE: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const FIL_NEG_CODE: ::core::ffi::c_int = 3 as ::core::ffi::c_int;
pub const SKIP_CODE: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
pub const MSKIP_CODE: ::core::ffi::c_int = 5 as ::core::ffi::c_int;
pub const SPAN_CODE: ::core::ffi::c_int = 1114113 as ::core::ffi::c_int;
pub const CR_CODE: ::core::ffi::c_int = 1114114 as ::core::ffi::c_int;
pub const CR_CR_CODE: ::core::ffi::c_int = 1114115 as ::core::ffi::c_int;
pub const TOP_MARK_CODE: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const FIRST_MARK_CODE: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const BOT_MARK_CODE: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const SPLIT_FIRST_MARK_CODE: ::core::ffi::c_int = 3 as ::core::ffi::c_int;
pub const SPLIT_BOT_MARK_CODE: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
pub const TT_TOP_MARKS_CODE: ::core::ffi::c_int = 5 as ::core::ffi::c_int;
pub const TT_FIRST_MARKS_CODE: ::core::ffi::c_int = 6 as ::core::ffi::c_int;
pub const TT_BOT_MARKS_CODE: ::core::ffi::c_int = 7 as ::core::ffi::c_int;
pub const TT_SPLIT_FIRST_MARKS_CODE: ::core::ffi::c_int = 8 as ::core::ffi::c_int;
pub const TT_SPLIT_BOT_MARKS_CODE: ::core::ffi::c_int = 9 as ::core::ffi::c_int;
pub const SHOW_CODE: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const SHOW_BOX_CODE: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const SHOW_THE_CODE: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const SHOW_LISTS: ::core::ffi::c_int = 3 as ::core::ffi::c_int;
pub const SHOW_GROUPS: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
pub const SHOW_TOKENS: ::core::ffi::c_int = 5 as ::core::ffi::c_int;
pub const SHOW_IFS: ::core::ffi::c_int = 6 as ::core::ffi::c_int;
pub const MIN_TRIE_OP: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const TRIE_OP_SIZE: ::core::ffi::c_long = 35111 as ::core::ffi::c_long;
pub const UTF8: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const EMPTY_STRING: ::core::ffi::c_long =
    65536 as ::core::ffi::c_long + 1 as ::core::ffi::c_long;
pub const FORMAT_SERIAL: ::core::ffi::c_int = 36 as ::core::ffi::c_int;
#[no_mangle]
pub static mut shell_escape_enabled: bool = false_0 != 0;
#[no_mangle]
pub static mut eqtb: *mut memory_word = ::core::ptr::null::<memory_word>() as *mut memory_word;
#[no_mangle]
pub static mut bad: int32_t = 0;
#[no_mangle]
pub static mut name_of_file: *mut ::core::ffi::c_char =
    ::core::ptr::null::<::core::ffi::c_char>() as *mut ::core::ffi::c_char;
#[no_mangle]
pub static mut name_of_file16: *mut UTF16_code =
    ::core::ptr::null::<UTF16_code>() as *mut UTF16_code;
#[no_mangle]
pub static mut name_length: int32_t = 0;
#[no_mangle]
pub static mut name_length16: int32_t = 0;
#[no_mangle]
pub static mut buffer: *mut UnicodeScalar =
    ::core::ptr::null::<UnicodeScalar>() as *mut UnicodeScalar;
#[no_mangle]
pub static mut first: int32_t = 0;
#[no_mangle]
pub static mut last: int32_t = 0;
#[no_mangle]
pub static mut max_buf_stack: int32_t = 0;
#[no_mangle]
pub static mut in_initex_mode: bool = false;
#[no_mangle]
pub static mut error_line: int32_t = 0;
#[no_mangle]
pub static mut half_error_line: int32_t = 0;
#[no_mangle]
pub static mut max_print_line: int32_t = 0;
#[no_mangle]
pub static mut max_strings: int32_t = 0;
#[no_mangle]
pub static mut strings_free: int32_t = 0;
#[no_mangle]
pub static mut string_vacancies: int32_t = 0;
#[no_mangle]
pub static mut pool_size: int32_t = 0;
#[no_mangle]
pub static mut pool_free: int32_t = 0;
#[no_mangle]
pub static mut font_mem_size: int32_t = 0;
#[no_mangle]
pub static mut font_max: int32_t = 0;
#[no_mangle]
pub static mut hyph_size: int32_t = 0;
#[no_mangle]
pub static mut trie_size: int32_t = 0;
#[no_mangle]
pub static mut buf_size: int32_t = 0;
#[no_mangle]
pub static mut stack_size: int32_t = 0;
#[no_mangle]
pub static mut max_in_open: int32_t = 0;
#[no_mangle]
pub static mut param_size: int32_t = 0;
#[no_mangle]
pub static mut nest_size: int32_t = 0;
#[no_mangle]
pub static mut save_size: int32_t = 0;
#[no_mangle]
pub static mut expand_depth: int32_t = 0;
#[no_mangle]
pub static mut file_line_error_style_p: ::core::ffi::c_int = 0;
#[no_mangle]
pub static mut halt_on_error_p: ::core::ffi::c_int = 0;
#[no_mangle]
pub static mut quoted_filename: bool = false;
#[no_mangle]
pub static mut insert_src_special_auto: bool = false;
#[no_mangle]
pub static mut insert_src_special_every_par: bool = false;
#[no_mangle]
pub static mut insert_src_special_every_math: bool = false;
#[no_mangle]
pub static mut insert_src_special_every_vbox: bool = false;
#[no_mangle]
pub static mut str_pool: *mut packed_UTF16_code =
    ::core::ptr::null::<packed_UTF16_code>() as *mut packed_UTF16_code;
#[no_mangle]
pub static mut str_start: *mut pool_pointer =
    ::core::ptr::null::<pool_pointer>() as *mut pool_pointer;
#[no_mangle]
pub static mut pool_ptr: pool_pointer = 0;
#[no_mangle]
pub static mut str_ptr: str_number = 0;
#[no_mangle]
pub static mut init_pool_ptr: pool_pointer = 0;
#[no_mangle]
pub static mut init_str_ptr: str_number = 0;
#[no_mangle]
pub static mut rust_stdout: rust_output_handle_t =
    ::core::ptr::null::<ttbc_output_handle_t>() as *mut ttbc_output_handle_t;
#[no_mangle]
pub static mut log_file: rust_output_handle_t =
    ::core::ptr::null::<ttbc_output_handle_t>() as *mut ttbc_output_handle_t;
#[no_mangle]
pub static mut selector: selector_t = SELECTOR_FILE_0;
#[no_mangle]
pub static mut dig: [::core::ffi::c_uchar; 23] = [0; 23];
#[no_mangle]
pub static mut tally: int32_t = 0;
#[no_mangle]
pub static mut term_offset: int32_t = 0;
#[no_mangle]
pub static mut file_offset: int32_t = 0;
#[no_mangle]
pub static mut trick_buf: [UTF16_code; 256] = [0; 256];
#[no_mangle]
pub static mut trick_count: int32_t = 0;
#[no_mangle]
pub static mut first_count: int32_t = 0;
#[no_mangle]
pub static mut doing_special: bool = false;
#[no_mangle]
pub static mut native_text: *mut UTF16_code = ::core::ptr::null::<UTF16_code>() as *mut UTF16_code;
#[no_mangle]
pub static mut native_text_size: int32_t = 0;
#[no_mangle]
pub static mut native_len: int32_t = 0;
#[no_mangle]
pub static mut save_native_len: int32_t = 0;
#[no_mangle]
pub static mut interaction: ::core::ffi::c_uchar = 0;
#[no_mangle]
pub static mut deletions_allowed: bool = false;
#[no_mangle]
pub static mut set_box_allowed: bool = false;
#[no_mangle]
pub static mut history: tt_history_t = HISTORY_SPOTLESS;
#[no_mangle]
pub static mut error_count: ::core::ffi::c_schar = 0;
#[no_mangle]
pub static mut help_line: [*const ::core::ffi::c_char; 6] =
    [::core::ptr::null::<::core::ffi::c_char>(); 6];
#[no_mangle]
pub static mut help_ptr: ::core::ffi::c_uchar = 0;
#[no_mangle]
pub static mut use_err_help: bool = false;
#[no_mangle]
pub static mut arith_error: bool = false;
#[no_mangle]
pub static mut tex_remainder: scaled_t = 0;
#[no_mangle]
pub static mut randoms: [int32_t; 55] = [0; 55];
#[no_mangle]
pub static mut j_random: ::core::ffi::c_uchar = 0;
#[no_mangle]
pub static mut random_seed: scaled_t = 0;
#[no_mangle]
pub static mut two_to_the: [int32_t; 31] = [0; 31];
#[no_mangle]
pub static mut spec_log: [int32_t; 29] = [0; 29];
#[no_mangle]
pub static mut temp_ptr: int32_t = 0;
#[no_mangle]
pub static mut mem: *mut memory_word = ::core::ptr::null::<memory_word>() as *mut memory_word;
#[no_mangle]
pub static mut lo_mem_max: int32_t = 0;
#[no_mangle]
pub static mut hi_mem_min: int32_t = 0;
#[no_mangle]
pub static mut dyn_used: int32_t = 0;
#[no_mangle]
pub static mut var_used: int32_t = 0;
#[no_mangle]
pub static mut avail: int32_t = 0;
#[no_mangle]
pub static mut mem_end: int32_t = 0;
#[no_mangle]
pub static mut rover: int32_t = 0;
#[no_mangle]
pub static mut last_leftmost_char: int32_t = 0;
#[no_mangle]
pub static mut last_rightmost_char: int32_t = 0;
#[no_mangle]
pub static mut hlist_stack: [int32_t; 513] = [0; 513];
#[no_mangle]
pub static mut hlist_stack_level: ::core::ffi::c_short = 0;
#[no_mangle]
pub static mut first_p: int32_t = 0;
#[no_mangle]
pub static mut global_prev_p: int32_t = 0;
#[no_mangle]
pub static mut font_in_short_display: int32_t = 0;
#[no_mangle]
pub static mut depth_threshold: int32_t = 0;
#[no_mangle]
pub static mut breadth_max: int32_t = 0;
#[no_mangle]
pub static mut nest: *mut list_state_record =
    ::core::ptr::null::<list_state_record>() as *mut list_state_record;
#[no_mangle]
pub static mut nest_ptr: int32_t = 0;
#[no_mangle]
pub static mut max_nest_stack: int32_t = 0;
#[no_mangle]
pub static mut cur_list: list_state_record = list_state_record {
    mode: 0,
    head: 0,
    tail: 0,
    eTeX_aux: 0,
    prev_graf: 0,
    mode_line: 0,
    aux: memory_word {
        b32: b32x2_le_t { s0: 0, s1: 0 },
    },
};
#[no_mangle]
pub static mut shown_mode: ::core::ffi::c_short = 0;
#[no_mangle]
pub static mut old_setting: ::core::ffi::c_uchar = 0;
#[no_mangle]
pub static mut hash: *mut b32x2 = ::core::ptr::null::<b32x2>() as *mut b32x2;
#[no_mangle]
pub static mut hash_used: int32_t = 0;
#[no_mangle]
pub static mut hash_extra: int32_t = 0;
#[no_mangle]
pub static mut hash_top: int32_t = 0;
#[no_mangle]
pub static mut eqtb_top: int32_t = 0;
#[no_mangle]
pub static mut hash_high: int32_t = 0;
#[no_mangle]
pub static mut no_new_control_sequence: bool = false;
#[no_mangle]
pub static mut cs_count: int32_t = 0;
#[no_mangle]
pub static mut prim: [b32x2; 2101] = [b32x2_le_t { s0: 0, s1: 0 }; 2101];
#[no_mangle]
pub static mut prim_used: int32_t = 0;
#[no_mangle]
pub static mut save_stack: *mut memory_word =
    ::core::ptr::null::<memory_word>() as *mut memory_word;
#[no_mangle]
pub static mut save_ptr: int32_t = 0;
#[no_mangle]
pub static mut max_save_stack: int32_t = 0;
#[no_mangle]
pub static mut cur_level: uint16_t = 0;
#[no_mangle]
pub static mut cur_group: group_code = 0;
#[no_mangle]
pub static mut cur_boundary: int32_t = 0;
#[no_mangle]
pub static mut mag_set: int32_t = 0;
#[no_mangle]
pub static mut cur_cmd: eight_bits = 0;
#[no_mangle]
pub static mut cur_chr: int32_t = 0;
#[no_mangle]
pub static mut cur_cs: int32_t = 0;
#[no_mangle]
pub static mut cur_tok: int32_t = 0;
#[no_mangle]
pub static mut input_stack: *mut input_state_t =
    ::core::ptr::null::<input_state_t>() as *mut input_state_t;
#[no_mangle]
pub static mut input_ptr: int32_t = 0;
#[no_mangle]
pub static mut max_in_stack: int32_t = 0;
#[no_mangle]
pub static mut cur_input: input_state_t = input_state_t {
    state: 0,
    index: 0,
    start: 0,
    loc: 0,
    limit: 0,
    name: 0,
    synctex_tag: 0,
};
#[no_mangle]
pub static mut in_open: int32_t = 0;
#[no_mangle]
pub static mut open_parens: int32_t = 0;
#[no_mangle]
pub static mut input_file: *mut *mut UFILE = ::core::ptr::null::<*mut UFILE>() as *mut *mut UFILE;
#[no_mangle]
pub static mut line: int32_t = 0;
#[no_mangle]
pub static mut line_stack: *mut int32_t = ::core::ptr::null::<int32_t>() as *mut int32_t;
#[no_mangle]
pub static mut source_filename_stack: *mut str_number =
    ::core::ptr::null::<str_number>() as *mut str_number;
#[no_mangle]
pub static mut full_source_filename_stack: *mut str_number =
    ::core::ptr::null::<str_number>() as *mut str_number;
#[no_mangle]
pub static mut scanner_status: ::core::ffi::c_uchar = 0;
#[no_mangle]
pub static mut warning_index: int32_t = 0;
#[no_mangle]
pub static mut def_ref: int32_t = 0;
#[no_mangle]
pub static mut param_stack: *mut int32_t = ::core::ptr::null::<int32_t>() as *mut int32_t;
#[no_mangle]
pub static mut param_ptr: int32_t = 0;
#[no_mangle]
pub static mut max_param_stack: int32_t = 0;
#[no_mangle]
pub static mut align_state: int32_t = 0;
#[no_mangle]
pub static mut base_ptr: int32_t = 0;
#[no_mangle]
pub static mut par_loc: int32_t = 0;
#[no_mangle]
pub static mut par_token: int32_t = 0;
#[no_mangle]
pub static mut force_eof: bool = false;
#[no_mangle]
pub static mut expand_depth_count: int32_t = 0;
#[no_mangle]
pub static mut is_in_csname: bool = false;
#[no_mangle]
pub static mut cur_mark: [int32_t; 5] = [0; 5];
#[no_mangle]
pub static mut long_state: ::core::ffi::c_uchar = 0;
#[no_mangle]
pub static mut pstack: [int32_t; 9] = [0; 9];
#[no_mangle]
pub static mut cur_val: int32_t = 0;
#[no_mangle]
pub static mut cur_val1: int32_t = 0;
#[no_mangle]
pub static mut cur_val_level: ::core::ffi::c_uchar = 0;
#[no_mangle]
pub static mut radix: small_number = 0;
#[no_mangle]
pub static mut cur_order: glue_ord = 0;
#[no_mangle]
pub static mut read_file: [*mut UFILE; 16] = [::core::ptr::null::<UFILE>() as *mut UFILE; 16];
#[no_mangle]
pub static mut read_open: [::core::ffi::c_uchar; 17] = [0; 17];
#[no_mangle]
pub static mut cond_ptr: int32_t = 0;
#[no_mangle]
pub static mut if_limit: ::core::ffi::c_uchar = 0;
#[no_mangle]
pub static mut cur_if: small_number = 0;
#[no_mangle]
pub static mut if_line: int32_t = 0;
#[no_mangle]
pub static mut skip_line: int32_t = 0;
#[no_mangle]
pub static mut cur_name: str_number = 0;
#[no_mangle]
pub static mut cur_area: str_number = 0;
#[no_mangle]
pub static mut cur_ext: str_number = 0;
#[no_mangle]
pub static mut area_delimiter: pool_pointer = 0;
#[no_mangle]
pub static mut ext_delimiter: pool_pointer = 0;
#[no_mangle]
pub static mut file_name_quote_char: UTF16_code = 0;
#[no_mangle]
pub static mut format_default_length: int32_t = 0;
#[no_mangle]
pub static mut TEX_format_default: *mut ::core::ffi::c_char =
    ::core::ptr::null::<::core::ffi::c_char>() as *mut ::core::ffi::c_char;
#[no_mangle]
pub static mut name_in_progress: bool = false;
#[no_mangle]
pub static mut job_name: str_number = 0;
#[no_mangle]
pub static mut log_opened: bool = false;
#[no_mangle]
pub static mut output_file_extension: *const ::core::ffi::c_char =
    ::core::ptr::null::<::core::ffi::c_char>();
#[no_mangle]
pub static mut texmf_log_name: str_number = 0;
#[no_mangle]
pub static mut font_info: *mut memory_word = ::core::ptr::null::<memory_word>() as *mut memory_word;
#[no_mangle]
pub static mut fmem_ptr: font_index = 0;
#[no_mangle]
pub static mut font_ptr: internal_font_number = 0;
#[no_mangle]
pub static mut font_check: *mut b16x4 = ::core::ptr::null::<b16x4>() as *mut b16x4;
#[no_mangle]
pub static mut font_size: *mut scaled_t = ::core::ptr::null::<scaled_t>() as *mut scaled_t;
#[no_mangle]
pub static mut font_dsize: *mut scaled_t = ::core::ptr::null::<scaled_t>() as *mut scaled_t;
#[no_mangle]
pub static mut font_params: *mut font_index = ::core::ptr::null::<font_index>() as *mut font_index;
#[no_mangle]
pub static mut font_name: *mut str_number = ::core::ptr::null::<str_number>() as *mut str_number;
#[no_mangle]
pub static mut font_area: *mut str_number = ::core::ptr::null::<str_number>() as *mut str_number;
#[no_mangle]
pub static mut font_bc: *mut UTF16_code = ::core::ptr::null::<UTF16_code>() as *mut UTF16_code;
#[no_mangle]
pub static mut font_ec: *mut UTF16_code = ::core::ptr::null::<UTF16_code>() as *mut UTF16_code;
#[no_mangle]
pub static mut font_glue: *mut int32_t = ::core::ptr::null::<int32_t>() as *mut int32_t;
#[no_mangle]
pub static mut font_used: *mut bool = ::core::ptr::null::<bool>() as *mut bool;
#[no_mangle]
pub static mut hyphen_char: *mut int32_t = ::core::ptr::null::<int32_t>() as *mut int32_t;
#[no_mangle]
pub static mut skew_char: *mut int32_t = ::core::ptr::null::<int32_t>() as *mut int32_t;
#[no_mangle]
pub static mut bchar_label: *mut font_index = ::core::ptr::null::<font_index>() as *mut font_index;
#[no_mangle]
pub static mut font_bchar: *mut nine_bits = ::core::ptr::null::<nine_bits>() as *mut nine_bits;
#[no_mangle]
pub static mut font_false_bchar: *mut nine_bits =
    ::core::ptr::null::<nine_bits>() as *mut nine_bits;
#[no_mangle]
pub static mut font_layout_engine: *mut *mut ::core::ffi::c_void =
    ::core::ptr::null::<*mut ::core::ffi::c_void>() as *mut *mut ::core::ffi::c_void;
#[no_mangle]
pub static mut font_mapping: *mut *mut ::core::ffi::c_void =
    ::core::ptr::null::<*mut ::core::ffi::c_void>() as *mut *mut ::core::ffi::c_void;
#[no_mangle]
pub static mut font_flags: *mut ::core::ffi::c_char =
    ::core::ptr::null::<::core::ffi::c_char>() as *mut ::core::ffi::c_char;
#[no_mangle]
pub static mut font_letter_space: *mut scaled_t = ::core::ptr::null::<scaled_t>() as *mut scaled_t;
#[no_mangle]
pub static mut loaded_font_mapping: *mut ::core::ffi::c_void =
    ::core::ptr::null::<::core::ffi::c_void>() as *mut ::core::ffi::c_void;
#[no_mangle]
pub static mut loaded_font_flags: ::core::ffi::c_char = 0;
#[no_mangle]
pub static mut loaded_font_letter_space: scaled_t = 0;
#[no_mangle]
pub static mut mapped_text: *mut UTF16_code = ::core::ptr::null::<UTF16_code>() as *mut UTF16_code;
#[no_mangle]
pub static mut xdv_buffer: *mut ::core::ffi::c_char =
    ::core::ptr::null::<::core::ffi::c_char>() as *mut ::core::ffi::c_char;
#[no_mangle]
pub static mut char_base: *mut int32_t = ::core::ptr::null::<int32_t>() as *mut int32_t;
#[no_mangle]
pub static mut width_base: *mut int32_t = ::core::ptr::null::<int32_t>() as *mut int32_t;
#[no_mangle]
pub static mut height_base: *mut int32_t = ::core::ptr::null::<int32_t>() as *mut int32_t;
#[no_mangle]
pub static mut depth_base: *mut int32_t = ::core::ptr::null::<int32_t>() as *mut int32_t;
#[no_mangle]
pub static mut italic_base: *mut int32_t = ::core::ptr::null::<int32_t>() as *mut int32_t;
#[no_mangle]
pub static mut lig_kern_base: *mut int32_t = ::core::ptr::null::<int32_t>() as *mut int32_t;
#[no_mangle]
pub static mut kern_base: *mut int32_t = ::core::ptr::null::<int32_t>() as *mut int32_t;
#[no_mangle]
pub static mut exten_base: *mut int32_t = ::core::ptr::null::<int32_t>() as *mut int32_t;
#[no_mangle]
pub static mut param_base: *mut int32_t = ::core::ptr::null::<int32_t>() as *mut int32_t;
#[no_mangle]
pub static mut null_character: b16x4 = b16x4_le_t {
    s0: 0,
    s1: 0,
    s2: 0,
    s3: 0,
};
#[no_mangle]
pub static mut total_pages: int32_t = 0;
#[no_mangle]
pub static mut max_v: scaled_t = 0;
#[no_mangle]
pub static mut max_h: scaled_t = 0;
#[no_mangle]
pub static mut max_push: int32_t = 0;
#[no_mangle]
pub static mut last_bop: int32_t = 0;
#[no_mangle]
pub static mut dead_cycles: int32_t = 0;
#[no_mangle]
pub static mut doing_leaders: bool = false;
#[no_mangle]
pub static mut rule_ht: scaled_t = 0;
#[no_mangle]
pub static mut rule_dp: scaled_t = 0;
#[no_mangle]
pub static mut rule_wd: scaled_t = 0;
#[no_mangle]
pub static mut cur_h: scaled_t = 0;
#[no_mangle]
pub static mut cur_v: scaled_t = 0;
#[no_mangle]
pub static mut epochseconds: int32_t = 0;
#[no_mangle]
pub static mut microseconds: int32_t = 0;
#[no_mangle]
pub static mut total_stretch: [scaled_t; 4] = [0; 4];
#[no_mangle]
pub static mut total_shrink: [scaled_t; 4] = [0; 4];
#[no_mangle]
pub static mut last_badness: int32_t = 0;
#[no_mangle]
pub static mut adjust_tail: int32_t = 0;
#[no_mangle]
pub static mut pre_adjust_tail: int32_t = 0;
#[no_mangle]
pub static mut pack_begin_line: int32_t = 0;
#[no_mangle]
pub static mut empty: b32x2 = b32x2_le_t { s0: 0, s1: 0 };
#[no_mangle]
pub static mut cur_f: internal_font_number = 0;
#[no_mangle]
pub static mut cur_c: int32_t = 0;
#[no_mangle]
pub static mut cur_i: b16x4 = b16x4_le_t {
    s0: 0,
    s1: 0,
    s2: 0,
    s3: 0,
};
#[no_mangle]
pub static mut cur_align: int32_t = 0;
#[no_mangle]
pub static mut cur_span: int32_t = 0;
#[no_mangle]
pub static mut cur_loop: int32_t = 0;
#[no_mangle]
pub static mut align_ptr: int32_t = 0;
#[no_mangle]
pub static mut cur_head: int32_t = 0;
#[no_mangle]
pub static mut cur_tail: int32_t = 0;
#[no_mangle]
pub static mut cur_pre_head: int32_t = 0;
#[no_mangle]
pub static mut cur_pre_tail: int32_t = 0;
#[no_mangle]
pub static mut just_box: int32_t = 0;
#[no_mangle]
pub static mut active_width: [scaled_t; 7] = [0; 7];
#[no_mangle]
pub static mut hc: [int32_t; 4099] = [0; 4099];
#[no_mangle]
pub static mut hf: internal_font_number = 0;
#[no_mangle]
pub static mut hu: [int32_t; 4097] = [0; 4097];
#[no_mangle]
pub static mut cur_lang: ::core::ffi::c_uchar = 0;
#[no_mangle]
pub static mut max_hyph_char: int32_t = 0;
#[no_mangle]
pub static mut hyf: [::core::ffi::c_uchar; 4097] = [0; 4097];
#[no_mangle]
pub static mut init_list: int32_t = 0;
#[no_mangle]
pub static mut init_lig: bool = false;
#[no_mangle]
pub static mut init_lft: bool = false;
#[no_mangle]
pub static mut hyphen_passed: small_number = 0;
#[no_mangle]
pub static mut cur_r: int32_t = 0;
#[no_mangle]
pub static mut cur_l: int32_t = 0;
#[no_mangle]
pub static mut cur_q: int32_t = 0;
#[no_mangle]
pub static mut lig_stack: int32_t = 0;
#[no_mangle]
pub static mut ligature_present: bool = false;
#[no_mangle]
pub static mut rt_hit: bool = false;
#[no_mangle]
pub static mut lft_hit: bool = false;
#[no_mangle]
pub static mut trie_trl: *mut trie_pointer =
    ::core::ptr::null::<trie_pointer>() as *mut trie_pointer;
#[no_mangle]
pub static mut trie_tro: *mut trie_pointer =
    ::core::ptr::null::<trie_pointer>() as *mut trie_pointer;
#[no_mangle]
pub static mut trie_trc: *mut uint16_t = ::core::ptr::null::<uint16_t>() as *mut uint16_t;
#[no_mangle]
pub static mut hyf_distance: [small_number; 35112] = [0; 35112];
#[no_mangle]
pub static mut hyf_num: [small_number; 35112] = [0; 35112];
#[no_mangle]
pub static mut hyf_next: [trie_opcode; 35112] = [0; 35112];
#[no_mangle]
pub static mut op_start: [int32_t; 256] = [0; 256];
#[no_mangle]
pub static mut hyph_word: *mut str_number = ::core::ptr::null::<str_number>() as *mut str_number;
#[no_mangle]
pub static mut hyph_list: *mut int32_t = ::core::ptr::null::<int32_t>() as *mut int32_t;
#[no_mangle]
pub static mut hyph_link: *mut hyph_pointer =
    ::core::ptr::null::<hyph_pointer>() as *mut hyph_pointer;
#[no_mangle]
pub static mut hyph_count: int32_t = 0;
#[no_mangle]
pub static mut hyph_next: int32_t = 0;
#[no_mangle]
pub static mut trie_used: [trie_opcode; 256] = [0; 256];
#[no_mangle]
pub static mut trie_op_lang: [::core::ffi::c_uchar; 35112] = [0; 35112];
#[no_mangle]
pub static mut trie_op_val: [trie_opcode; 35112] = [0; 35112];
#[no_mangle]
pub static mut trie_op_ptr: int32_t = 0;
#[no_mangle]
pub static mut max_op_used: trie_opcode = 0;
#[no_mangle]
pub static mut trie_c: *mut packed_UTF16_code =
    ::core::ptr::null::<packed_UTF16_code>() as *mut packed_UTF16_code;
#[no_mangle]
pub static mut trie_o: *mut trie_opcode = ::core::ptr::null::<trie_opcode>() as *mut trie_opcode;
#[no_mangle]
pub static mut trie_l: *mut trie_pointer = ::core::ptr::null::<trie_pointer>() as *mut trie_pointer;
#[no_mangle]
pub static mut trie_r: *mut trie_pointer = ::core::ptr::null::<trie_pointer>() as *mut trie_pointer;
#[no_mangle]
pub static mut trie_ptr: trie_pointer = 0;
#[no_mangle]
pub static mut trie_hash: *mut trie_pointer =
    ::core::ptr::null::<trie_pointer>() as *mut trie_pointer;
#[no_mangle]
pub static mut trie_taken: *mut bool = ::core::ptr::null::<bool>() as *mut bool;
#[no_mangle]
pub static mut trie_min: [trie_pointer; 65536] = [0; 65536];
#[no_mangle]
pub static mut trie_max: trie_pointer = 0;
#[no_mangle]
pub static mut trie_not_ready: bool = false;
#[no_mangle]
pub static mut best_height_plus_depth: scaled_t = 0;
#[no_mangle]
pub static mut main_f: internal_font_number = 0;
#[no_mangle]
pub static mut main_i: b16x4 = b16x4_le_t {
    s0: 0,
    s1: 0,
    s2: 0,
    s3: 0,
};
#[no_mangle]
pub static mut main_j: b16x4 = b16x4_le_t {
    s0: 0,
    s1: 0,
    s2: 0,
    s3: 0,
};
#[no_mangle]
pub static mut main_k: font_index = 0;
#[no_mangle]
pub static mut main_p: int32_t = 0;
#[no_mangle]
pub static mut main_ppp: int32_t = 0;
#[no_mangle]
pub static mut main_pp: int32_t = 0;
#[no_mangle]
pub static mut main_h: int32_t = 0;
#[no_mangle]
pub static mut is_hyph: bool = false;
#[no_mangle]
pub static mut space_class: int32_t = 0;
#[no_mangle]
pub static mut prev_class: int32_t = 0;
#[no_mangle]
pub static mut main_s: int32_t = 0;
#[no_mangle]
pub static mut bchar: int32_t = 0;
#[no_mangle]
pub static mut false_bchar: int32_t = 0;
#[no_mangle]
pub static mut cancel_boundary: bool = false;
#[no_mangle]
pub static mut ins_disc: bool = false;
#[no_mangle]
pub static mut cur_box: int32_t = 0;
#[no_mangle]
pub static mut after_token: int32_t = 0;
#[no_mangle]
pub static mut long_help_seen: bool = false;
#[no_mangle]
pub static mut format_ident: str_number = 0;
#[no_mangle]
pub static mut write_file: [rust_output_handle_t; 16] =
    [::core::ptr::null::<ttbc_output_handle_t>() as *mut ttbc_output_handle_t; 16];
#[no_mangle]
pub static mut write_open: [bool; 18] = [false; 18];
#[no_mangle]
pub static mut write_loc: int32_t = 0;
#[no_mangle]
pub static mut cur_page_width: scaled_t = 0;
#[no_mangle]
pub static mut cur_page_height: scaled_t = 0;
#[no_mangle]
pub static mut cur_h_offset: scaled_t = 0;
#[no_mangle]
pub static mut cur_v_offset: scaled_t = 0;
#[no_mangle]
pub static mut pdf_last_x_pos: int32_t = 0;
#[no_mangle]
pub static mut pdf_last_y_pos: int32_t = 0;
#[no_mangle]
pub static mut eof_seen: *mut bool = ::core::ptr::null::<bool>() as *mut bool;
#[no_mangle]
pub static mut LR_ptr: int32_t = 0;
#[no_mangle]
pub static mut LR_problems: int32_t = 0;
#[no_mangle]
pub static mut cur_dir: small_number = 0;
#[no_mangle]
pub static mut pseudo_files: int32_t = 0;
#[no_mangle]
pub static mut grp_stack: *mut save_pointer =
    ::core::ptr::null::<save_pointer>() as *mut save_pointer;
#[no_mangle]
pub static mut if_stack: *mut int32_t = ::core::ptr::null::<int32_t>() as *mut int32_t;
#[no_mangle]
pub static mut max_reg_num: int32_t = 0;
#[no_mangle]
pub static mut max_reg_help_line: *const ::core::ffi::c_char =
    ::core::ptr::null::<::core::ffi::c_char>();
#[no_mangle]
pub static mut sa_root: [int32_t; 8] = [0; 8];
#[no_mangle]
pub static mut cur_ptr: int32_t = 0;
#[no_mangle]
pub static mut sa_null: memory_word = memory_word {
    b32: b32x2_le_t { s0: 0, s1: 0 },
};
#[no_mangle]
pub static mut sa_chain: int32_t = 0;
#[no_mangle]
pub static mut sa_level: uint16_t = 0;
#[no_mangle]
pub static mut hyph_start: trie_pointer = 0;
#[no_mangle]
pub static mut hyph_index: trie_pointer = 0;
#[no_mangle]
pub static mut disc_ptr: [int32_t; 4] = [0; 4];
#[no_mangle]
pub static mut edit_name_start: pool_pointer = 0;
#[no_mangle]
pub static mut stop_at_space: bool = false;
#[no_mangle]
pub static mut native_font_type_flag: int32_t = 0;
#[no_mangle]
pub static mut xtx_ligature_present: bool = false;
#[no_mangle]
pub static mut delta: scaled_t = 0;
#[no_mangle]
pub static mut synctex_enabled: bool = false;
#[no_mangle]
pub static mut synctex_use_gz: bool = false;
#[no_mangle]
pub static mut synctex_texpresso_extension: bool = false;
#[no_mangle]
pub static mut used_tectonic_coda_tokens: bool = false;
#[no_mangle]
pub static mut semantic_pagination_enabled: bool = false;
#[no_mangle]
pub static mut gave_char_warning_help: bool = false;
#[no_mangle]
pub static mut page_tail: int32_t = 0;
#[no_mangle]
pub static mut page_contents: ::core::ffi::c_uchar = 0;
#[no_mangle]
pub static mut page_so_far: [scaled_t; 8] = [0; 8];
#[no_mangle]
pub static mut last_glue: int32_t = 0;
#[no_mangle]
pub static mut last_penalty: int32_t = 0;
#[no_mangle]
pub static mut last_kern: scaled_t = 0;
#[no_mangle]
pub static mut last_node_type: int32_t = 0;
#[no_mangle]
pub static mut insert_penalties: int32_t = 0;
#[no_mangle]
pub static mut output_active: bool = false;
#[no_mangle]
pub static mut _xeq_level_array: [uint16_t; 1114736] = [0; 1114736];
pub const NEG_TRIE_OP_SIZE: ::core::ffi::c_long = -(35111 as ::core::ffi::c_long);
pub const MAX_TRIE_OP: ::core::ffi::c_long = 65535 as ::core::ffi::c_long;
static mut _trie_op_hash_array: [int32_t; 70223] = [0; 70223];
static mut yhash: *mut b32x2 = ::core::ptr::null::<b32x2>() as *mut b32x2;
pub const FORMAT_HEADER_MAGIC: ::core::ffi::c_int = 0x54544e43 as ::core::ffi::c_int;
pub const FORMAT_FOOTER_MAGIC: ::core::ffi::c_int = 0x29a as ::core::ffi::c_int;
unsafe extern "C" fn swap_items(
    mut p: *mut ::core::ffi::c_char,
    mut nitems: size_t,
    mut size: size_t,
) {
    let mut temp: ::core::ffi::c_char = 0;
    match size {
        16 => loop {
            let fresh5 = nitems;
            nitems = nitems.wrapping_sub(1);
            if !(fresh5 != 0) {
                break;
            }
            temp = *p.offset(0 as ::core::ffi::c_int as isize);
            *p.offset(0 as ::core::ffi::c_int as isize) =
                *p.offset(15 as ::core::ffi::c_int as isize);
            *p.offset(15 as ::core::ffi::c_int as isize) = temp;
            temp = *p.offset(1 as ::core::ffi::c_int as isize);
            *p.offset(1 as ::core::ffi::c_int as isize) =
                *p.offset(14 as ::core::ffi::c_int as isize);
            *p.offset(14 as ::core::ffi::c_int as isize) = temp;
            temp = *p.offset(2 as ::core::ffi::c_int as isize);
            *p.offset(2 as ::core::ffi::c_int as isize) =
                *p.offset(13 as ::core::ffi::c_int as isize);
            *p.offset(13 as ::core::ffi::c_int as isize) = temp;
            temp = *p.offset(3 as ::core::ffi::c_int as isize);
            *p.offset(3 as ::core::ffi::c_int as isize) =
                *p.offset(12 as ::core::ffi::c_int as isize);
            *p.offset(12 as ::core::ffi::c_int as isize) = temp;
            temp = *p.offset(4 as ::core::ffi::c_int as isize);
            *p.offset(4 as ::core::ffi::c_int as isize) =
                *p.offset(11 as ::core::ffi::c_int as isize);
            *p.offset(11 as ::core::ffi::c_int as isize) = temp;
            temp = *p.offset(5 as ::core::ffi::c_int as isize);
            *p.offset(5 as ::core::ffi::c_int as isize) =
                *p.offset(10 as ::core::ffi::c_int as isize);
            *p.offset(10 as ::core::ffi::c_int as isize) = temp;
            temp = *p.offset(6 as ::core::ffi::c_int as isize);
            *p.offset(6 as ::core::ffi::c_int as isize) =
                *p.offset(9 as ::core::ffi::c_int as isize);
            *p.offset(9 as ::core::ffi::c_int as isize) = temp;
            temp = *p.offset(7 as ::core::ffi::c_int as isize);
            *p.offset(7 as ::core::ffi::c_int as isize) =
                *p.offset(8 as ::core::ffi::c_int as isize);
            *p.offset(8 as ::core::ffi::c_int as isize) = temp;
            p = p.offset(size as isize);
        },
        8 => loop {
            let fresh6 = nitems;
            nitems = nitems.wrapping_sub(1);
            if !(fresh6 != 0) {
                break;
            }
            temp = *p.offset(0 as ::core::ffi::c_int as isize);
            *p.offset(0 as ::core::ffi::c_int as isize) =
                *p.offset(7 as ::core::ffi::c_int as isize);
            *p.offset(7 as ::core::ffi::c_int as isize) = temp;
            temp = *p.offset(1 as ::core::ffi::c_int as isize);
            *p.offset(1 as ::core::ffi::c_int as isize) =
                *p.offset(6 as ::core::ffi::c_int as isize);
            *p.offset(6 as ::core::ffi::c_int as isize) = temp;
            temp = *p.offset(2 as ::core::ffi::c_int as isize);
            *p.offset(2 as ::core::ffi::c_int as isize) =
                *p.offset(5 as ::core::ffi::c_int as isize);
            *p.offset(5 as ::core::ffi::c_int as isize) = temp;
            temp = *p.offset(3 as ::core::ffi::c_int as isize);
            *p.offset(3 as ::core::ffi::c_int as isize) =
                *p.offset(4 as ::core::ffi::c_int as isize);
            *p.offset(4 as ::core::ffi::c_int as isize) = temp;
            p = p.offset(size as isize);
        },
        4 => loop {
            let fresh7 = nitems;
            nitems = nitems.wrapping_sub(1);
            if !(fresh7 != 0) {
                break;
            }
            temp = *p.offset(0 as ::core::ffi::c_int as isize);
            *p.offset(0 as ::core::ffi::c_int as isize) =
                *p.offset(3 as ::core::ffi::c_int as isize);
            *p.offset(3 as ::core::ffi::c_int as isize) = temp;
            temp = *p.offset(1 as ::core::ffi::c_int as isize);
            *p.offset(1 as ::core::ffi::c_int as isize) =
                *p.offset(2 as ::core::ffi::c_int as isize);
            *p.offset(2 as ::core::ffi::c_int as isize) = temp;
            p = p.offset(size as isize);
        },
        2 => loop {
            let fresh8 = nitems;
            nitems = nitems.wrapping_sub(1);
            if !(fresh8 != 0) {
                break;
            }
            temp = *p.offset(0 as ::core::ffi::c_int as isize);
            *p.offset(0 as ::core::ffi::c_int as isize) =
                *p.offset(1 as ::core::ffi::c_int as isize);
            *p.offset(1 as ::core::ffi::c_int as isize) = temp;
            p = p.offset(size as isize);
        },
        1 => {}
        _ => {
            _tt_abort(
                b"can't swap a %zu-byte item for (un)dumping\0" as *const u8
                    as *const ::core::ffi::c_char,
                size,
            );
        }
    };
}
unsafe extern "C" fn do_dump(
    mut p: *mut ::core::ffi::c_char,
    mut item_size: size_t,
    mut nitems: size_t,
    mut out_file: rust_output_handle_t,
) {
    swap_items(p, nitems, item_size);
    let mut r: ssize_t =
        ttstub_output_write(out_file, p, item_size.wrapping_mul(nitems)) as ssize_t;
    if r < 0 as ssize_t || r as size_t != item_size.wrapping_mul(nitems) {
        _tt_abort(
            b"could not write %zu %zu-byte item(s) to %s\0" as *const u8
                as *const ::core::ffi::c_char,
            nitems,
            item_size,
            name_of_file,
        );
    }
    swap_items(p, nitems, item_size);
}
unsafe extern "C" fn do_undump(
    mut p: *mut ::core::ffi::c_char,
    mut item_size: size_t,
    mut nitems: size_t,
    mut in_file: rust_input_handle_t,
) {
    let mut r: ssize_t = ttstub_input_read(in_file, p, item_size.wrapping_mul(nitems));
    if r < 0 as ssize_t || r as size_t != item_size.wrapping_mul(nitems) {
        _tt_abort(
            b"could not undump %zu %zu-byte item(s) from %s\0" as *const u8
                as *const ::core::ffi::c_char,
            nitems,
            item_size,
            name_of_file,
        );
    }
    swap_items(p, nitems, item_size);
}
pub const hash_offset: ::core::ffi::c_int = 514 as ::core::ffi::c_int;
pub const sup_max_strings: ::core::ffi::c_long = 2097151 as ::core::ffi::c_long;
pub const sup_font_mem_size: ::core::ffi::c_long = 147483647 as ::core::ffi::c_long;
pub const sup_pool_size: ::core::ffi::c_long = 40000000 as ::core::ffi::c_long;
pub const sup_hash_extra: ::core::ffi::c_long = sup_max_strings;
unsafe extern "C" fn sort_avail() {
    let mut p: int32_t = 0;
    let mut q: int32_t = 0;
    let mut r: int32_t = 0;
    let mut old_rover: int32_t = 0;
    p = get_node(0x40000000 as int32_t);
    p = (*mem.offset((rover + 1 as int32_t) as isize)).b32.s1;
    (*mem.offset((rover + 1 as int32_t) as isize)).b32.s1 = MAX_HALFWORD as int32_t;
    old_rover = rover;
    while p != old_rover {
        if p < rover {
            q = p;
            p = (*mem.offset((q + 1 as int32_t) as isize)).b32.s1;
            (*mem.offset((q + 1 as int32_t) as isize)).b32.s1 = rover;
            rover = q;
        } else {
            q = rover;
            while (*mem.offset((q + 1 as int32_t) as isize)).b32.s1 < p {
                q = (*mem.offset((q + 1 as int32_t) as isize)).b32.s1;
            }
            r = (*mem.offset((p + 1 as int32_t) as isize)).b32.s1;
            (*mem.offset((p + 1 as int32_t) as isize)).b32.s1 =
                (*mem.offset((q + 1 as int32_t) as isize)).b32.s1;
            (*mem.offset((q + 1 as int32_t) as isize)).b32.s1 = p;
            p = r;
        }
    }
    p = rover;
    while (*mem.offset((p + 1 as int32_t) as isize)).b32.s1 != MAX_HALFWORD as int32_t {
        (*mem
            .offset(((*mem.offset((p + 1 as int32_t) as isize)).b32.s1 + 1 as int32_t) as isize))
        .b32
        .s0 = p;
        p = (*mem.offset((p + 1 as int32_t) as isize)).b32.s1;
    }
    (*mem.offset((p + 1 as int32_t) as isize)).b32.s1 = rover;
    (*mem.offset((rover + 1 as int32_t) as isize)).b32.s0 = p;
}
unsafe extern "C" fn primitive(
    mut ident: *const ::core::ffi::c_char,
    mut c: uint16_t,
    mut o: int32_t,
) {
    let mut prim_val: int32_t = 0;
    let mut len: ::core::ffi::c_int = strlen(ident) as ::core::ffi::c_int;
    if len > 1 as ::core::ffi::c_int {
        let mut s: str_number = maketexstring(ident) as str_number;
        if first + len as int32_t > buf_size + 1 as int32_t {
            overflow(
                b"buffer size\0" as *const u8 as *const ::core::ffi::c_char,
                buf_size,
            );
        }
        let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
        while i < len {
            *buffer.offset((first + i as int32_t) as isize) =
                *ident.offset(i as isize) as UnicodeScalar;
            i += 1;
        }
        cur_val = id_lookup(first, len as int32_t);
        str_ptr -= 1;
        pool_ptr = *str_start.offset((str_ptr - TOO_BIG_CHAR as str_number) as isize);
        (*hash.offset(cur_val as isize)).s1 = s as int32_t;
        prim_val = prim_lookup(s);
    } else {
        cur_val = (*ident.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
            + SINGLE_BASE) as int32_t;
        prim_val = prim_lookup(*ident.offset(0 as ::core::ffi::c_int as isize) as str_number);
    }
    (*eqtb.offset(cur_val as isize)).b16.s0 = LEVEL_ONE as uint16_t;
    (*eqtb.offset(cur_val as isize)).b16.s1 = c;
    (*eqtb.offset(cur_val as isize)).b32.s1 = o;
    (*eqtb.offset((PRIM_EQTB_BASE as int32_t + prim_val) as isize))
        .b16
        .s0 = LEVEL_ONE as uint16_t;
    (*eqtb.offset((PRIM_EQTB_BASE as int32_t + prim_val) as isize))
        .b16
        .s1 = c;
    (*eqtb.offset((PRIM_EQTB_BASE as int32_t + prim_val) as isize))
        .b32
        .s1 = o;
}
#[no_mangle]
pub unsafe extern "C" fn new_trie_op(
    mut d: small_number,
    mut n: small_number,
    mut v: trie_opcode,
) -> trie_opcode {
    let mut h: int32_t = 0;
    let mut u: trie_opcode = 0;
    let mut l: int32_t = 0;
    h = (abs(n as ::core::ffi::c_int
        + 313 as ::core::ffi::c_int * d as ::core::ffi::c_int
        + 361 as ::core::ffi::c_int * v as ::core::ffi::c_int
        + 1009 as ::core::ffi::c_int * cur_lang as ::core::ffi::c_int)
        as ::core::ffi::c_long
        % (TRIE_OP_SIZE - NEG_TRIE_OP_SIZE)
        + NEG_TRIE_OP_SIZE) as int32_t;
    loop {
        l = _trie_op_hash_array[(h as ::core::ffi::c_long - NEG_TRIE_OP_SIZE) as usize];
        if l == 0 as int32_t {
            if trie_op_ptr as ::core::ffi::c_long == TRIE_OP_SIZE {
                overflow(
                    b"pattern memory ops\0" as *const u8 as *const ::core::ffi::c_char,
                    TRIE_OP_SIZE as int32_t,
                );
            }
            u = trie_used[cur_lang as usize];
            if u as ::core::ffi::c_long == MAX_TRIE_OP {
                overflow(
                    b"pattern memory ops per language\0" as *const u8 as *const ::core::ffi::c_char,
                    (MAX_TRIE_OP - MIN_TRIE_OP as ::core::ffi::c_long) as int32_t,
                );
            }
            trie_op_ptr += 1;
            u = u.wrapping_add(1);
            trie_used[cur_lang as usize] = u;
            if u as ::core::ffi::c_int > max_op_used as ::core::ffi::c_int {
                max_op_used = u;
            }
            hyf_distance[trie_op_ptr as usize] = d;
            hyf_num[trie_op_ptr as usize] = n;
            hyf_next[trie_op_ptr as usize] = v;
            trie_op_lang[trie_op_ptr as usize] = cur_lang;
            _trie_op_hash_array[(h as ::core::ffi::c_long - NEG_TRIE_OP_SIZE) as usize] =
                trie_op_ptr;
            trie_op_val[trie_op_ptr as usize] = u;
            return u;
        }
        if hyf_distance[l as usize] as ::core::ffi::c_int == d as ::core::ffi::c_int
            && hyf_num[l as usize] as ::core::ffi::c_int == n as ::core::ffi::c_int
            && hyf_next[l as usize] as ::core::ffi::c_int == v as ::core::ffi::c_int
            && trie_op_lang[l as usize] as ::core::ffi::c_int == cur_lang as ::core::ffi::c_int
        {
            return trie_op_val[l as usize];
        }
        if h > -(TRIE_OP_SIZE as int32_t) {
            h -= 1;
        } else {
            h = TRIE_OP_SIZE as int32_t;
        }
    }
}
#[no_mangle]
pub unsafe extern "C" fn trie_node(mut p: trie_pointer) -> trie_pointer {
    let mut h: trie_pointer = 0;
    let mut q: trie_pointer = 0;
    h = (abs(*trie_c.offset(p as isize) as ::core::ffi::c_int
        + 1009 as ::core::ffi::c_int * *trie_o.offset(p as isize) as ::core::ffi::c_int
        + 2718 as ::core::ffi::c_int * *trie_l.offset(p as isize) as ::core::ffi::c_int
        + 3142 as ::core::ffi::c_int * *trie_r.offset(p as isize) as ::core::ffi::c_int)
        as int32_t
        % trie_size) as trie_pointer;
    loop {
        q = *trie_hash.offset(h as isize);
        if q == 0 as trie_pointer {
            *trie_hash.offset(h as isize) = p;
            return p;
        }
        if *trie_c.offset(q as isize) as ::core::ffi::c_int
            == *trie_c.offset(p as isize) as ::core::ffi::c_int
            && *trie_o.offset(q as isize) as ::core::ffi::c_int
                == *trie_o.offset(p as isize) as ::core::ffi::c_int
            && *trie_l.offset(q as isize) == *trie_l.offset(p as isize)
            && *trie_r.offset(q as isize) == *trie_r.offset(p as isize)
        {
            return q;
        }
        if h > 0 as trie_pointer {
            h -= 1;
        } else {
            h = trie_size as trie_pointer;
        }
    }
}
#[no_mangle]
pub unsafe extern "C" fn compress_trie(mut p: trie_pointer) -> trie_pointer {
    if p == 0 as trie_pointer {
        return 0 as trie_pointer;
    } else {
        *trie_l.offset(p as isize) = compress_trie(*trie_l.offset(p as isize));
        *trie_r.offset(p as isize) = compress_trie(*trie_r.offset(p as isize));
        return trie_node(p);
    };
}
#[no_mangle]
pub unsafe extern "C" fn first_fit(mut p: trie_pointer) {
    let mut h: trie_pointer = 0;
    let mut z: trie_pointer = 0;
    let mut q: trie_pointer = 0;
    let mut c: UTF16_code = 0;
    let mut l: trie_pointer = 0;
    let mut r: trie_pointer = 0;
    let mut ll: int32_t = 0;
    c = *trie_c.offset(p as isize) as UTF16_code;
    z = trie_min[c as usize];
    's_22: loop {
        h = z - c as trie_pointer;
        if trie_max < h + max_hyph_char as trie_pointer {
            if trie_size <= h + max_hyph_char as trie_pointer {
                overflow(
                    b"pattern memory\0" as *const u8 as *const ::core::ffi::c_char,
                    trie_size,
                );
            }
            loop {
                trie_max += 1;
                *trie_taken.offset(trie_max as isize) = false_0 != 0;
                *trie_trl.offset(trie_max as isize) = trie_max + 1 as trie_pointer;
                *trie_tro.offset(trie_max as isize) = trie_max - 1 as trie_pointer;
                if trie_max == h + max_hyph_char as trie_pointer {
                    break;
                }
            }
        }
        if !*trie_taken.offset(h as isize) {
            q = *trie_r.offset(p as isize);
            loop {
                if !(q > 0 as trie_pointer) {
                    break 's_22;
                }
                if *trie_trl.offset((h + *trie_c.offset(q as isize) as trie_pointer) as isize)
                    == 0 as trie_pointer
                {
                    break;
                }
                q = *trie_r.offset(q as isize);
            }
        }
        z = *trie_trl.offset(z as isize);
    }
    *trie_taken.offset(h as isize) = true_0 != 0;
    *trie_hash.offset(p as isize) = h;
    q = p;
    loop {
        z = h + *trie_c.offset(q as isize) as trie_pointer;
        l = *trie_tro.offset(z as isize);
        r = *trie_trl.offset(z as isize);
        *trie_tro.offset(r as isize) = l;
        *trie_trl.offset(l as isize) = r;
        *trie_trl.offset(z as isize) = 0 as ::core::ffi::c_int as trie_pointer;
        if l < max_hyph_char {
            if z < max_hyph_char {
                ll = z as int32_t;
            } else {
                ll = max_hyph_char;
            }
            loop {
                trie_min[l as usize] = r;
                l += 1;
                if l == ll {
                    break;
                }
            }
        }
        q = *trie_r.offset(q as isize);
        if q == 0 as trie_pointer {
            break;
        }
    }
}
#[no_mangle]
pub unsafe extern "C" fn trie_pack(mut p: trie_pointer) {
    let mut q: trie_pointer = 0;
    loop {
        q = *trie_l.offset(p as isize);
        if q > 0 as trie_pointer && *trie_hash.offset(q as isize) == 0 as trie_pointer {
            first_fit(q);
            trie_pack(q);
        }
        p = *trie_r.offset(p as isize);
        if p == 0 as trie_pointer {
            break;
        }
    }
}
#[no_mangle]
pub unsafe extern "C" fn trie_fix(mut p: trie_pointer) {
    let mut q: trie_pointer = 0;
    let mut c: UTF16_code = 0;
    let mut z: trie_pointer = 0;
    z = *trie_hash.offset(p as isize);
    loop {
        q = *trie_l.offset(p as isize);
        c = *trie_c.offset(p as isize) as UTF16_code;
        *trie_trl.offset((z + c as trie_pointer) as isize) = *trie_hash.offset(q as isize);
        *trie_trc.offset((z + c as trie_pointer) as isize) = c as uint16_t;
        *trie_tro.offset((z + c as trie_pointer) as isize) =
            *trie_o.offset(p as isize) as trie_pointer;
        if q > 0 as trie_pointer {
            trie_fix(q);
        }
        p = *trie_r.offset(p as isize);
        if p == 0 as trie_pointer {
            break;
        }
    }
}
unsafe extern "C" fn new_patterns() {
    let mut k: ::core::ffi::c_short = 0;
    let mut l: ::core::ffi::c_short = 0;
    let mut digit_sensed: bool = false;
    let mut v: trie_opcode = 0;
    let mut p: trie_pointer = 0;
    let mut q: trie_pointer = 0;
    let mut first_child: bool = false;
    let mut c: UTF16_code = 0;
    if trie_not_ready {
        if (*eqtb.offset((INT_BASE + INT_PAR__language) as isize))
            .b32
            .s1
            <= 0 as int32_t
        {
            cur_lang = 0 as ::core::ffi::c_uchar;
        } else if (*eqtb.offset((INT_BASE + INT_PAR__language) as isize))
            .b32
            .s1
            > BIGGEST_LANG as int32_t
        {
            cur_lang = 0 as ::core::ffi::c_uchar;
        } else {
            cur_lang = (*eqtb.offset((INT_BASE + INT_PAR__language) as isize))
                .b32
                .s1 as ::core::ffi::c_uchar;
        }
        scan_left_brace();
        k = 0 as ::core::ffi::c_short;
        hyf[0 as ::core::ffi::c_int as usize] = 0 as ::core::ffi::c_uchar;
        digit_sensed = false_0 != 0;
        loop {
            get_x_token();
            match cur_cmd as ::core::ffi::c_int {
                LETTER | OTHER_CHAR => {
                    if digit_sensed as ::core::ffi::c_int != 0
                        || cur_chr < '0' as i32
                        || cur_chr > '9' as i32
                    {
                        if cur_chr == '.' as i32 {
                            cur_chr = 0 as ::core::ffi::c_int as int32_t;
                        } else {
                            cur_chr = (*eqtb.offset((LC_CODE_BASE as int32_t + cur_chr) as isize))
                                .b32
                                .s1;
                            if cur_chr == 0 as int32_t {
                                error_here_with_diagnostic(
                                    b"Nonletter\0" as *const u8 as *const ::core::ffi::c_char,
                                );
                                capture_to_diagnostic(::core::ptr::null_mut::<ttbc_diagnostic_t>());
                                help_ptr = 1 as ::core::ffi::c_uchar;
                                help_line[0 as ::core::ffi::c_int as usize] = b"(See Appendix H.)\0"
                                    as *const u8
                                    as *const ::core::ffi::c_char;
                                error();
                            }
                        }
                        if cur_chr > max_hyph_char {
                            max_hyph_char = cur_chr;
                        }
                        if (k as int32_t) < max_hyphenatable_length() {
                            k += 1;
                            hc[k as usize] = cur_chr;
                            hyf[k as usize] = 0 as ::core::ffi::c_uchar;
                            digit_sensed = false_0 != 0;
                        }
                    } else if (k as int32_t) < max_hyphenatable_length() {
                        hyf[k as usize] = (cur_chr - 48 as int32_t) as ::core::ffi::c_uchar;
                        digit_sensed = true_0 != 0;
                    }
                }
                SPACER | RIGHT_BRACE => {
                    if k as ::core::ffi::c_int > 0 as ::core::ffi::c_int {
                        if hc[1 as ::core::ffi::c_int as usize] == 0 as int32_t {
                            hyf[0 as ::core::ffi::c_int as usize] = 0 as ::core::ffi::c_uchar;
                        }
                        if hc[k as usize] == 0 as int32_t {
                            hyf[k as usize] = 0 as ::core::ffi::c_uchar;
                        }
                        l = k;
                        v = MIN_TRIE_OP as trie_opcode;
                        loop {
                            if hyf[l as usize] as ::core::ffi::c_int != 0 as ::core::ffi::c_int {
                                v = new_trie_op(
                                    (k as ::core::ffi::c_int - l as ::core::ffi::c_int)
                                        as small_number,
                                    hyf[l as usize] as small_number,
                                    v,
                                );
                            }
                            if !(l as ::core::ffi::c_int > 0 as ::core::ffi::c_int) {
                                break;
                            }
                            l -= 1;
                        }
                        q = 0 as ::core::ffi::c_int as trie_pointer;
                        hc[0 as ::core::ffi::c_int as usize] = cur_lang as int32_t;
                        while l as ::core::ffi::c_int <= k as ::core::ffi::c_int {
                            c = hc[l as usize] as UTF16_code;
                            l += 1;
                            p = *trie_l.offset(q as isize);
                            first_child = true_0 != 0;
                            while p > 0 as trie_pointer
                                && c as ::core::ffi::c_int
                                    > *trie_c.offset(p as isize) as ::core::ffi::c_int
                            {
                                q = p;
                                p = *trie_r.offset(q as isize);
                                first_child = false_0 != 0;
                            }
                            if p == 0 as trie_pointer
                                || (c as ::core::ffi::c_int)
                                    < *trie_c.offset(p as isize) as ::core::ffi::c_int
                            {
                                if trie_ptr == trie_size {
                                    overflow(
                                        b"pattern memory\0" as *const u8
                                            as *const ::core::ffi::c_char,
                                        trie_size,
                                    );
                                }
                                trie_ptr += 1;
                                *trie_r.offset(trie_ptr as isize) = p;
                                p = trie_ptr;
                                *trie_l.offset(p as isize) =
                                    0 as ::core::ffi::c_int as trie_pointer;
                                if first_child {
                                    *trie_l.offset(q as isize) = p;
                                } else {
                                    *trie_r.offset(q as isize) = p;
                                }
                                *trie_c.offset(p as isize) = c as packed_UTF16_code;
                                *trie_o.offset(p as isize) = MIN_TRIE_OP as trie_opcode;
                            }
                            q = p;
                        }
                        if *trie_o.offset(q as isize) as ::core::ffi::c_int != MIN_TRIE_OP {
                            error_here_with_diagnostic(
                                b"Duplicate pattern\0" as *const u8 as *const ::core::ffi::c_char,
                            );
                            capture_to_diagnostic(::core::ptr::null_mut::<ttbc_diagnostic_t>());
                            help_ptr = 1 as ::core::ffi::c_uchar;
                            help_line[0 as ::core::ffi::c_int as usize] =
                                b"(See Appendix H.)\0" as *const u8 as *const ::core::ffi::c_char;
                            error();
                        }
                        *trie_o.offset(q as isize) = v;
                    }
                    if cur_cmd as ::core::ffi::c_int == RIGHT_BRACE {
                        break;
                    }
                    k = 0 as ::core::ffi::c_short;
                    hyf[0 as ::core::ffi::c_int as usize] = 0 as ::core::ffi::c_uchar;
                    digit_sensed = false_0 != 0;
                }
                _ => {
                    error_here_with_diagnostic(
                        b"Bad \0" as *const u8 as *const ::core::ffi::c_char,
                    );
                    print_esc_cstr(b"patterns\0" as *const u8 as *const ::core::ffi::c_char);
                    capture_to_diagnostic(::core::ptr::null_mut::<ttbc_diagnostic_t>());
                    help_ptr = 1 as ::core::ffi::c_uchar;
                    help_line[0 as ::core::ffi::c_int as usize] =
                        b"(See Appendix H.)\0" as *const u8 as *const ::core::ffi::c_char;
                    error();
                }
            }
        }
        if (*eqtb.offset((INT_BASE + INT_PAR__saving_hyph_codes) as isize))
            .b32
            .s1
            > 0 as int32_t
        {
            c = cur_lang as UTF16_code;
            first_child = false_0 != 0;
            p = 0 as ::core::ffi::c_int as trie_pointer;
            loop {
                q = p;
                p = *trie_r.offset(q as isize);
                if p == 0 as trie_pointer
                    || c as ::core::ffi::c_int <= *trie_c.offset(p as isize) as ::core::ffi::c_int
                {
                    break;
                }
            }
            if p == 0 as trie_pointer
                || (c as ::core::ffi::c_int) < *trie_c.offset(p as isize) as ::core::ffi::c_int
            {
                if trie_ptr == trie_size {
                    overflow(
                        b"pattern memory\0" as *const u8 as *const ::core::ffi::c_char,
                        trie_size,
                    );
                }
                trie_ptr += 1;
                *trie_r.offset(trie_ptr as isize) = p;
                p = trie_ptr;
                *trie_l.offset(p as isize) = 0 as ::core::ffi::c_int as trie_pointer;
                if first_child {
                    *trie_l.offset(q as isize) = p;
                } else {
                    *trie_r.offset(q as isize) = p;
                }
                *trie_c.offset(p as isize) = c as packed_UTF16_code;
                *trie_o.offset(p as isize) = MIN_TRIE_OP as trie_opcode;
            }
            q = p;
            p = *trie_l.offset(q as isize);
            first_child = true_0 != 0;
            c = 0 as UTF16_code;
            while c as ::core::ffi::c_int <= 255 as ::core::ffi::c_int {
                if (*eqtb.offset((LC_CODE_BASE + c as ::core::ffi::c_int) as isize))
                    .b32
                    .s1
                    > 0 as int32_t
                    || c as ::core::ffi::c_int == 255 as ::core::ffi::c_int
                        && first_child as ::core::ffi::c_int != 0
                {
                    if p == 0 as trie_pointer {
                        if trie_ptr == trie_size {
                            overflow(
                                b"pattern memory\0" as *const u8 as *const ::core::ffi::c_char,
                                trie_size,
                            );
                        }
                        trie_ptr += 1;
                        *trie_r.offset(trie_ptr as isize) = p;
                        p = trie_ptr;
                        *trie_l.offset(p as isize) = 0 as ::core::ffi::c_int as trie_pointer;
                        if first_child {
                            *trie_l.offset(q as isize) = p;
                        } else {
                            *trie_r.offset(q as isize) = p;
                        }
                        *trie_c.offset(p as isize) = c as packed_UTF16_code;
                        *trie_o.offset(p as isize) = MIN_TRIE_OP as trie_opcode;
                    } else {
                        *trie_c.offset(p as isize) = c as packed_UTF16_code;
                    }
                    *trie_o.offset(p as isize) = (*eqtb
                        .offset((LC_CODE_BASE + c as ::core::ffi::c_int) as isize))
                    .b32
                    .s1 as trie_opcode;
                    q = p;
                    p = *trie_r.offset(q as isize);
                    first_child = false_0 != 0;
                }
                c = c.wrapping_add(1);
            }
            if first_child {
                *trie_l.offset(q as isize) = 0 as ::core::ffi::c_int as trie_pointer;
            } else {
                *trie_r.offset(q as isize) = 0 as ::core::ffi::c_int as trie_pointer;
            }
        }
    } else {
        error_here_with_diagnostic(b"Too late for \0" as *const u8 as *const ::core::ffi::c_char);
        print_esc_cstr(b"patterns\0" as *const u8 as *const ::core::ffi::c_char);
        capture_to_diagnostic(::core::ptr::null_mut::<ttbc_diagnostic_t>());
        help_ptr = 1 as ::core::ffi::c_uchar;
        help_line[0 as ::core::ffi::c_int as usize] =
            b"All patterns must be given before typesetting begins.\0" as *const u8
                as *const ::core::ffi::c_char;
        error();
        (*mem.offset(GARBAGE as isize)).b32.s1 = scan_toks(false_0 != 0, false_0 != 0);
        flush_list(def_ref);
    };
}
#[no_mangle]
pub unsafe extern "C" fn init_trie() {
    let mut p: trie_pointer = 0;
    let mut j: int32_t = 0;
    let mut k: int32_t = 0;
    let mut t: int32_t = 0;
    let mut r: trie_pointer = 0;
    let mut s: trie_pointer = 0;
    max_hyph_char += 1;
    op_start[0 as ::core::ffi::c_int as usize] = -(MIN_TRIE_OP as int32_t);
    let mut for_end: int32_t = 0;
    j = 1 as ::core::ffi::c_int as int32_t;
    for_end = BIGGEST_LANG as int32_t;
    if j <= for_end {
        loop {
            op_start[j as usize] = op_start[(j - 1 as int32_t) as usize]
                + trie_used[(j - 1 as int32_t) as usize] as int32_t;
            let fresh9 = j;
            j = j + 1;
            if !(fresh9 < for_end) {
                break;
            }
        }
    }
    let mut for_end_0: int32_t = 0;
    j = 1 as ::core::ffi::c_int as int32_t;
    for_end_0 = trie_op_ptr;
    if j <= for_end_0 {
        loop {
            _trie_op_hash_array[(j as ::core::ffi::c_long - NEG_TRIE_OP_SIZE) as usize] =
                op_start[trie_op_lang[j as usize] as usize] + trie_op_val[j as usize] as int32_t;
            let fresh10 = j;
            j = j + 1;
            if !(fresh10 < for_end_0) {
                break;
            }
        }
    }
    let mut for_end_1: int32_t = 0;
    j = 1 as ::core::ffi::c_int as int32_t;
    for_end_1 = trie_op_ptr;
    if j <= for_end_1 {
        loop {
            while _trie_op_hash_array[(j as ::core::ffi::c_long - NEG_TRIE_OP_SIZE) as usize] > j {
                k = _trie_op_hash_array[(j as ::core::ffi::c_long - NEG_TRIE_OP_SIZE) as usize];
                t = hyf_distance[k as usize] as int32_t;
                hyf_distance[k as usize] = hyf_distance[j as usize];
                hyf_distance[j as usize] = t as small_number;
                t = hyf_num[k as usize] as int32_t;
                hyf_num[k as usize] = hyf_num[j as usize];
                hyf_num[j as usize] = t as small_number;
                t = hyf_next[k as usize] as int32_t;
                hyf_next[k as usize] = hyf_next[j as usize];
                hyf_next[j as usize] = t as trie_opcode;
                _trie_op_hash_array[(j as ::core::ffi::c_long - NEG_TRIE_OP_SIZE) as usize] =
                    _trie_op_hash_array[(k as ::core::ffi::c_long - NEG_TRIE_OP_SIZE) as usize];
                _trie_op_hash_array[(k as ::core::ffi::c_long - NEG_TRIE_OP_SIZE) as usize] = k;
            }
            let fresh11 = j;
            j = j + 1;
            if !(fresh11 < for_end_1) {
                break;
            }
        }
    }
    let mut for_end_2: int32_t = 0;
    p = 0 as ::core::ffi::c_int as trie_pointer;
    for_end_2 = trie_size;
    if p <= for_end_2 {
        loop {
            *trie_hash.offset(p as isize) = 0 as ::core::ffi::c_int as trie_pointer;
            let fresh12 = p;
            p = p + 1;
            if !(fresh12 < for_end_2) {
                break;
            }
        }
    }
    *trie_r.offset(0 as ::core::ffi::c_int as isize) =
        compress_trie(*trie_r.offset(0 as ::core::ffi::c_int as isize));
    *trie_l.offset(0 as ::core::ffi::c_int as isize) =
        compress_trie(*trie_l.offset(0 as ::core::ffi::c_int as isize));
    let mut for_end_3: int32_t = 0;
    p = 0 as ::core::ffi::c_int as trie_pointer;
    for_end_3 = trie_ptr as int32_t;
    if p <= for_end_3 {
        loop {
            *trie_hash.offset(p as isize) = 0 as ::core::ffi::c_int as trie_pointer;
            let fresh13 = p;
            p = p + 1;
            if !(fresh13 < for_end_3) {
                break;
            }
        }
    }
    let mut for_end_4: int32_t = 0;
    p = 0 as ::core::ffi::c_int as trie_pointer;
    for_end_4 = BIGGEST_CHAR as int32_t;
    if p <= for_end_4 {
        loop {
            trie_min[p as usize] = p + 1 as trie_pointer;
            let fresh14 = p;
            p = p + 1;
            if !(fresh14 < for_end_4) {
                break;
            }
        }
    }
    *trie_trl.offset(0 as ::core::ffi::c_int as isize) = 1 as ::core::ffi::c_int as trie_pointer;
    trie_max = 0 as ::core::ffi::c_int as trie_pointer;
    if *trie_l.offset(0 as ::core::ffi::c_int as isize) != 0 as trie_pointer {
        first_fit(*trie_l.offset(0 as ::core::ffi::c_int as isize));
        trie_pack(*trie_l.offset(0 as ::core::ffi::c_int as isize));
    }
    if *trie_r.offset(0 as ::core::ffi::c_int as isize) != 0 as trie_pointer {
        if *trie_l.offset(0 as ::core::ffi::c_int as isize) == 0 as trie_pointer {
            let mut for_end_5: int32_t = 0;
            p = 0 as ::core::ffi::c_int as trie_pointer;
            for_end_5 = 255 as ::core::ffi::c_int as int32_t;
            if p <= for_end_5 {
                loop {
                    trie_min[p as usize] = p + 2 as trie_pointer;
                    let fresh15 = p;
                    p = p + 1;
                    if !(fresh15 < for_end_5) {
                        break;
                    }
                }
            }
        }
        first_fit(*trie_r.offset(0 as ::core::ffi::c_int as isize));
        trie_pack(*trie_r.offset(0 as ::core::ffi::c_int as isize));
        hyph_start = *trie_hash.offset(*trie_r.offset(0 as ::core::ffi::c_int as isize) as isize);
    }
    if trie_max == 0 as trie_pointer {
        let mut for_end_6: int32_t = 0;
        r = 0 as ::core::ffi::c_int as trie_pointer;
        for_end_6 = max_hyph_char;
        if r <= for_end_6 {
            loop {
                *trie_trl.offset(r as isize) = 0 as ::core::ffi::c_int as trie_pointer;
                *trie_tro.offset(r as isize) = MIN_TRIE_OP as trie_pointer;
                *trie_trc.offset(r as isize) = 0 as uint16_t;
                let fresh16 = r;
                r = r + 1;
                if !(fresh16 < for_end_6) {
                    break;
                }
            }
        }
        trie_max = max_hyph_char as trie_pointer;
    } else {
        if *trie_r.offset(0 as ::core::ffi::c_int as isize) > 0 as trie_pointer {
            trie_fix(*trie_r.offset(0 as ::core::ffi::c_int as isize));
        }
        if *trie_l.offset(0 as ::core::ffi::c_int as isize) > 0 as trie_pointer {
            trie_fix(*trie_l.offset(0 as ::core::ffi::c_int as isize));
        }
        r = 0 as ::core::ffi::c_int as trie_pointer;
        loop {
            s = *trie_trl.offset(r as isize);
            *trie_trl.offset(r as isize) = 0 as ::core::ffi::c_int as trie_pointer;
            *trie_tro.offset(r as isize) = MIN_TRIE_OP as trie_pointer;
            *trie_trc.offset(r as isize) = 0 as uint16_t;
            r = s;
            if r > trie_max {
                break;
            }
        }
    }
    *trie_trc.offset(0 as ::core::ffi::c_int as isize) = '?' as i32 as uint16_t;
    trie_not_ready = false_0 != 0;
}
unsafe extern "C" fn new_hyph_exceptions() {
    let mut current_block: u64;
    let mut n: ::core::ffi::c_short = 0;
    let mut j: ::core::ffi::c_short = 0;
    let mut h: hyph_pointer = 0;
    let mut k: str_number = 0;
    let mut p: int32_t = 0;
    let mut q: int32_t = 0;
    let mut s: str_number = 0;
    let mut u: pool_pointer = 0;
    let mut v: pool_pointer = 0;
    scan_left_brace();
    if (*eqtb.offset((INT_BASE + INT_PAR__language) as isize))
        .b32
        .s1
        <= 0 as int32_t
    {
        cur_lang = 0 as ::core::ffi::c_uchar;
    } else if (*eqtb.offset((INT_BASE + INT_PAR__language) as isize))
        .b32
        .s1
        > BIGGEST_LANG as int32_t
    {
        cur_lang = 0 as ::core::ffi::c_uchar;
    } else {
        cur_lang = (*eqtb.offset((INT_BASE + INT_PAR__language) as isize))
            .b32
            .s1 as ::core::ffi::c_uchar;
    }
    if trie_not_ready {
        hyph_index = 0 as ::core::ffi::c_int as trie_pointer;
    } else if *trie_trc.offset((hyph_start + cur_lang as trie_pointer) as isize)
        as ::core::ffi::c_int
        != cur_lang as ::core::ffi::c_int
    {
        hyph_index = 0 as ::core::ffi::c_int as trie_pointer;
    } else {
        hyph_index = *trie_trl.offset((hyph_start + cur_lang as trie_pointer) as isize);
    }
    n = 0 as ::core::ffi::c_short;
    p = TEX_NULL as int32_t;
    's_74: loop {
        get_x_token();
        loop {
            match cur_cmd as ::core::ffi::c_int {
                LETTER | OTHER_CHAR | CHAR_GIVEN => {
                    if cur_chr == '-' as i32 {
                        if (n as int32_t) < max_hyphenatable_length() {
                            q = get_avail();
                            (*mem.offset(q as isize)).b32.s1 = p;
                            (*mem.offset(q as isize)).b32.s0 = n as int32_t;
                            p = q;
                        }
                    } else {
                        if hyph_index == 0 as trie_pointer || cur_chr > 255 as int32_t {
                            hc[0 as ::core::ffi::c_int as usize] = (*eqtb
                                .offset((LC_CODE_BASE as int32_t + cur_chr) as isize))
                            .b32
                            .s1;
                        } else if *trie_trc.offset((hyph_index + cur_chr as trie_pointer) as isize)
                            as int32_t
                            != cur_chr
                        {
                            hc[0 as ::core::ffi::c_int as usize] =
                                0 as ::core::ffi::c_int as int32_t;
                        } else {
                            hc[0 as ::core::ffi::c_int as usize] = *trie_tro
                                .offset((hyph_index + cur_chr as trie_pointer) as isize)
                                as int32_t;
                        }
                        if hc[0 as ::core::ffi::c_int as usize] == 0 as int32_t {
                            error_here_with_diagnostic(
                                b"Not a letter\0" as *const u8 as *const ::core::ffi::c_char,
                            );
                            capture_to_diagnostic(::core::ptr::null_mut::<ttbc_diagnostic_t>());
                            help_ptr = 2 as ::core::ffi::c_uchar;
                            help_line[1 as ::core::ffi::c_int as usize] =
                                b"Letters in \\hyphenation words must have \\lccode>0.\0"
                                    as *const u8
                                    as *const ::core::ffi::c_char;
                            help_line[0 as ::core::ffi::c_int as usize] =
                                b"Proceed; I'll ignore the character I just read.\0" as *const u8
                                    as *const ::core::ffi::c_char;
                            error();
                        } else if (n as int32_t) < max_hyphenatable_length() {
                            n += 1;
                            if (hc[0 as ::core::ffi::c_int as usize] as ::core::ffi::c_long)
                                < 65536 as ::core::ffi::c_long
                            {
                                hc[n as usize] = hc[0 as ::core::ffi::c_int as usize];
                            } else {
                                hc[n as usize] = ((hc[0 as ::core::ffi::c_int as usize]
                                    as ::core::ffi::c_long
                                    - 65536 as ::core::ffi::c_long)
                                    / 1024 as ::core::ffi::c_long
                                    + 55296 as ::core::ffi::c_long)
                                    as int32_t;
                                n += 1;
                                hc[n as usize] = ((hc[0 as ::core::ffi::c_int as usize]
                                    % 1024 as int32_t)
                                    as ::core::ffi::c_long
                                    + 56320 as ::core::ffi::c_long)
                                    as int32_t;
                            }
                        }
                    }
                    continue 's_74;
                }
                CHAR_NUM => {
                    scan_char_num();
                    cur_chr = cur_val;
                    cur_cmd = CHAR_GIVEN as eight_bits;
                }
                SPACER | RIGHT_BRACE => {
                    if n as ::core::ffi::c_int > 1 as ::core::ffi::c_int {
                        current_block = 16799951812150840583;
                        break;
                    } else {
                        current_block = 17787701279558130514;
                        break;
                    }
                }
                _ => {
                    error_here_with_diagnostic(
                        b"Improper \0" as *const u8 as *const ::core::ffi::c_char,
                    );
                    print_esc_cstr(b"hyphenation\0" as *const u8 as *const ::core::ffi::c_char);
                    print_cstr(b" will be flushed\0" as *const u8 as *const ::core::ffi::c_char);
                    capture_to_diagnostic(::core::ptr::null_mut::<ttbc_diagnostic_t>());
                    help_ptr = 2 as ::core::ffi::c_uchar;
                    help_line[1 as ::core::ffi::c_int as usize] =
                        b"Hyphenation exceptions must contain only letters\0" as *const u8
                            as *const ::core::ffi::c_char;
                    help_line[0 as ::core::ffi::c_int as usize] =
                        b"and hyphens. But continue; I'll forgive and forget.\0" as *const u8
                            as *const ::core::ffi::c_char;
                    error();
                    continue 's_74;
                }
            }
        }
        match current_block {
            16799951812150840583 => {
                n += 1;
                hc[n as usize] = cur_lang as int32_t;
                if pool_ptr + n as pool_pointer > pool_size {
                    overflow(
                        b"pool size\0" as *const u8 as *const ::core::ffi::c_char,
                        pool_size - init_pool_ptr as int32_t,
                    );
                }
                h = 0 as hyph_pointer;
                j = 1 as ::core::ffi::c_short;
                while j as ::core::ffi::c_int <= n as ::core::ffi::c_int {
                    h = ((h as int32_t + h as int32_t + hc[j as usize]) % HYPH_PRIME as int32_t)
                        as hyph_pointer;
                    *str_pool.offset(pool_ptr as isize) = hc[j as usize] as packed_UTF16_code;
                    pool_ptr += 1;
                    j += 1;
                }
                s = make_string();
                if hyph_next <= HYPH_PRIME as int32_t {
                    while hyph_next > 0 as int32_t
                        && *hyph_word.offset((hyph_next - 1 as int32_t) as isize) > 0 as str_number
                    {
                        hyph_next -= 1;
                    }
                }
                if hyph_count == hyph_size || hyph_next == 0 as int32_t {
                    overflow(
                        b"exception dictionary\0" as *const u8 as *const ::core::ffi::c_char,
                        hyph_size,
                    );
                }
                hyph_count += 1;
                while *hyph_word.offset(h as isize) != 0 as str_number {
                    k = *hyph_word.offset(h as isize);
                    if !(length(k) != length(s)) {
                        u = *str_start.offset(
                            (k as ::core::ffi::c_long - 65536 as ::core::ffi::c_long) as isize,
                        );
                        v = *str_start.offset(
                            (s as ::core::ffi::c_long - 65536 as ::core::ffi::c_long) as isize,
                        );
                        loop {
                            if *str_pool.offset(u as isize) as ::core::ffi::c_int
                                != *str_pool.offset(v as isize) as ::core::ffi::c_int
                            {
                                current_block = 3950261577287087443;
                                break;
                            }
                            u += 1;
                            v += 1;
                            if !(u
                                != *str_start.offset(
                                    ((k + 1 as str_number) as ::core::ffi::c_long
                                        - 65536 as ::core::ffi::c_long)
                                        as isize,
                                ))
                            {
                                current_block = 12758904613967585247;
                                break;
                            }
                        }
                        match current_block {
                            3950261577287087443 => {}
                            _ => {
                                str_ptr -= 1;
                                pool_ptr = *str_start
                                    .offset((str_ptr - TOO_BIG_CHAR as str_number) as isize);
                                s = *hyph_word.offset(h as isize);
                                hyph_count -= 1;
                                break;
                            }
                        }
                    }
                    if *hyph_link.offset(h as isize) as ::core::ffi::c_int
                        == 0 as ::core::ffi::c_int
                    {
                        *hyph_link.offset(h as isize) = hyph_next as hyph_pointer;
                        if hyph_next >= hyph_size {
                            hyph_next = HYPH_PRIME as int32_t;
                        }
                        if hyph_next > HYPH_PRIME as int32_t {
                            hyph_next += 1;
                        }
                    }
                    h = (*hyph_link.offset(h as isize) as ::core::ffi::c_int
                        - 1 as ::core::ffi::c_int) as hyph_pointer;
                }
                *hyph_word.offset(h as isize) = s;
                *hyph_list.offset(h as isize) = p;
            }
            _ => {}
        }
        if cur_cmd as ::core::ffi::c_int == RIGHT_BRACE {
            return;
        }
        n = 0 as ::core::ffi::c_short;
        p = TEX_NULL as int32_t;
    }
}
#[no_mangle]
pub unsafe extern "C" fn prefixed_command() {
    let mut current_block: u64;
    let mut a: small_number = 0;
    let mut f: internal_font_number = 0;
    let mut j: int32_t = 0;
    let mut k: font_index = 0;
    let mut p: int32_t = 0;
    let mut q: int32_t = 0;
    let mut n: int32_t = 0;
    let mut e: bool = false;
    a = 0 as small_number;
    while cur_cmd as ::core::ffi::c_int == PREFIX {
        if a as int32_t / cur_chr & 1 as int32_t == 0 {
            a = (a as int32_t + cur_chr) as small_number;
        }
        loop {
            get_x_token();
            if !(cur_cmd as ::core::ffi::c_int == SPACER || cur_cmd as ::core::ffi::c_int == RELAX)
            {
                break;
            }
        }
        if cur_cmd as ::core::ffi::c_int <= MAX_NON_PREFIXED_COMMAND {
            error_here_with_diagnostic(
                b"You can't use a prefix with `\0" as *const u8 as *const ::core::ffi::c_char,
            );
            print_cmd_chr(cur_cmd as uint16_t, cur_chr);
            print_char('\'' as i32);
            capture_to_diagnostic(::core::ptr::null_mut::<ttbc_diagnostic_t>());
            help_ptr = 1 as ::core::ffi::c_uchar;
            help_line[0 as ::core::ffi::c_int as usize] =
                b"I'll pretend you didn't say \\long or \\outer or \\global or \\protected.\0"
                    as *const u8 as *const ::core::ffi::c_char;
            back_error();
            return;
        }
        if (*eqtb.offset((INT_BASE + INT_PAR__tracing_commands) as isize))
            .b32
            .s1
            > 2 as int32_t
        {
            show_cur_cmd_chr();
        }
    }
    if a as ::core::ffi::c_int >= 8 as ::core::ffi::c_int {
        j = PROTECTED_TOKEN as int32_t;
        a = (a as ::core::ffi::c_int - 8 as ::core::ffi::c_int) as small_number;
    } else {
        j = 0 as ::core::ffi::c_int as int32_t;
    }
    if cur_cmd as ::core::ffi::c_int != DEF
        && (a as ::core::ffi::c_int % 4 as ::core::ffi::c_int != 0 as ::core::ffi::c_int
            || j != 0 as int32_t)
    {
        error_here_with_diagnostic(b"You can't use `\0" as *const u8 as *const ::core::ffi::c_char);
        print_esc_cstr(b"long\0" as *const u8 as *const ::core::ffi::c_char);
        print_cstr(b"' or `\0" as *const u8 as *const ::core::ffi::c_char);
        print_esc_cstr(b"outer\0" as *const u8 as *const ::core::ffi::c_char);
        print_cstr(b"' or `\0" as *const u8 as *const ::core::ffi::c_char);
        print_esc_cstr(b"protected\0" as *const u8 as *const ::core::ffi::c_char);
        print_cstr(b"' with `\0" as *const u8 as *const ::core::ffi::c_char);
        print_cmd_chr(cur_cmd as uint16_t, cur_chr);
        print_char('\'' as i32);
        capture_to_diagnostic(::core::ptr::null_mut::<ttbc_diagnostic_t>());
        help_ptr = 1 as ::core::ffi::c_uchar;
        help_line[0 as ::core::ffi::c_int as usize] =
            b"I'll pretend you didn't say \\long or \\outer or \\protected here.\0" as *const u8
                as *const ::core::ffi::c_char;
        error();
    }
    if (*eqtb.offset((INT_BASE + INT_PAR__global_defs) as isize))
        .b32
        .s1
        != 0 as int32_t
    {
        if (*eqtb.offset((INT_BASE + INT_PAR__global_defs) as isize))
            .b32
            .s1
            < 0 as int32_t
        {
            if a as ::core::ffi::c_int >= 4 as ::core::ffi::c_int {
                a = (a as ::core::ffi::c_int - 4 as ::core::ffi::c_int) as small_number;
            }
        } else if (a as ::core::ffi::c_int) < 4 as ::core::ffi::c_int {
            a = (a as ::core::ffi::c_int + 4 as ::core::ffi::c_int) as small_number;
        }
    }
    match cur_cmd as ::core::ffi::c_int {
        SET_FONT => {
            if a as ::core::ffi::c_int >= 4 as ::core::ffi::c_int {
                geq_define(CUR_FONT_LOC as int32_t, DATA as uint16_t, cur_chr);
            } else {
                eq_define(CUR_FONT_LOC as int32_t, DATA as uint16_t, cur_chr);
            }
        }
        DEF => {
            if cur_chr & 1 as int32_t != 0
                && (a as ::core::ffi::c_int) < 4 as ::core::ffi::c_int
                && (*eqtb.offset((INT_BASE + INT_PAR__global_defs) as isize))
                    .b32
                    .s1
                    >= 0 as int32_t
            {
                a = (a as ::core::ffi::c_int + 4 as ::core::ffi::c_int) as small_number;
            }
            e = cur_chr >= 2 as int32_t;
            get_r_token();
            p = cur_cs;
            q = scan_toks(true_0 != 0, e);
            if j != 0 as int32_t {
                q = get_avail();
                (*mem.offset(q as isize)).b32.s0 = j;
                (*mem.offset(q as isize)).b32.s1 = (*mem.offset(def_ref as isize)).b32.s1;
                (*mem.offset(def_ref as isize)).b32.s1 = q;
            }
            if a as ::core::ffi::c_int >= 4 as ::core::ffi::c_int {
                geq_define(
                    p,
                    (CALL + a as ::core::ffi::c_int % 4 as ::core::ffi::c_int) as uint16_t,
                    def_ref,
                );
            } else {
                eq_define(
                    p,
                    (CALL + a as ::core::ffi::c_int % 4 as ::core::ffi::c_int) as uint16_t,
                    def_ref,
                );
            }
        }
        LET => {
            n = cur_chr;
            get_r_token();
            p = cur_cs;
            if n == NORMAL as int32_t {
                loop {
                    get_token();
                    if !(cur_cmd as ::core::ffi::c_int == SPACER) {
                        break;
                    }
                }
                if cur_tok == OTHER_TOKEN as int32_t + '=' as i32 {
                    get_token();
                    if cur_cmd as ::core::ffi::c_int == SPACER {
                        get_token();
                    }
                }
            } else {
                get_token();
                q = cur_tok;
                get_token();
                back_input();
                cur_tok = q;
                back_input();
            }
            if cur_cmd as ::core::ffi::c_int >= CALL {
                let ref mut fresh20 = (*mem.offset(cur_chr as isize)).b32.s0;
                *fresh20 += 1;
            } else if cur_cmd as ::core::ffi::c_int == REGISTER
                || cur_cmd as ::core::ffi::c_int == TOKS_REGISTER
            {
                if cur_chr < 0 as int32_t || cur_chr > 19 as int32_t {
                    let ref mut fresh21 = (*mem.offset((cur_chr + 1 as int32_t) as isize)).b32.s0;
                    *fresh21 += 1;
                }
            }
            if a as ::core::ffi::c_int >= 4 as ::core::ffi::c_int {
                geq_define(p, cur_cmd as uint16_t, cur_chr);
            } else {
                eq_define(p, cur_cmd as uint16_t, cur_chr);
            }
        }
        SHORTHAND_DEF => {
            if cur_chr == CHAR_SUB_DEF_CODE as int32_t {
                error_here_with_diagnostic(
                    b"vestigial MLTeX shorthand encountered??\0" as *const u8
                        as *const ::core::ffi::c_char,
                );
                capture_to_diagnostic(::core::ptr::null_mut::<ttbc_diagnostic_t>());
                help_ptr = 1 as ::core::ffi::c_uchar;
                help_line[0 as ::core::ffi::c_int as usize] =
                    b"This should never happen in Tectonic, where MLTeX has been excised.\0"
                        as *const u8 as *const ::core::ffi::c_char;
                error();
            } else {
                n = cur_chr;
                get_r_token();
                p = cur_cs;
                if a as ::core::ffi::c_int >= 4 as ::core::ffi::c_int {
                    geq_define(p, RELAX as uint16_t, TOO_BIG_USV as int32_t);
                } else {
                    eq_define(p, RELAX as uint16_t, TOO_BIG_USV as int32_t);
                }
                scan_optional_equals();
                match n {
                    CHAR_DEF_CODE => {
                        scan_usv_num();
                        if a as ::core::ffi::c_int >= 4 as ::core::ffi::c_int {
                            geq_define(p, CHAR_GIVEN as uint16_t, cur_val);
                        } else {
                            eq_define(p, CHAR_GIVEN as uint16_t, cur_val);
                        }
                    }
                    MATH_CHAR_DEF_CODE => {
                        scan_fifteen_bit_int();
                        if a as ::core::ffi::c_int >= 4 as ::core::ffi::c_int {
                            geq_define(p, MATH_GIVEN as uint16_t, cur_val);
                        } else {
                            eq_define(p, MATH_GIVEN as uint16_t, cur_val);
                        }
                    }
                    XETEX_MATH_CHAR_NUM_DEF_CODE => {
                        scan_xetex_math_char_int();
                        if a as ::core::ffi::c_int >= 4 as ::core::ffi::c_int {
                            geq_define(p, XETEX_MATH_GIVEN as uint16_t, cur_val);
                        } else {
                            eq_define(p, XETEX_MATH_GIVEN as uint16_t, cur_val);
                        }
                    }
                    XETEX_MATH_CHAR_DEF_CODE => {
                        scan_math_class_int();
                        n = ((cur_val as ::core::ffi::c_uint & 0x7 as ::core::ffi::c_uint)
                            << 21 as ::core::ffi::c_int) as int32_t;
                        scan_math_fam_int();
                        n = (n as ::core::ffi::c_uint).wrapping_add(
                            (cur_val as ::core::ffi::c_uint & 0xff as ::core::ffi::c_uint)
                                << 24 as ::core::ffi::c_int,
                        ) as int32_t;
                        scan_usv_num();
                        n = n + cur_val;
                        if a as ::core::ffi::c_int >= 4 as ::core::ffi::c_int {
                            geq_define(p, XETEX_MATH_GIVEN as uint16_t, n);
                        } else {
                            eq_define(p, XETEX_MATH_GIVEN as uint16_t, n);
                        }
                    }
                    _ => {
                        scan_register_num();
                        if cur_val > 255 as int32_t {
                            j = n - 2 as int32_t;
                            if j > MU_VAL as int32_t {
                                j = TOK_VAL as int32_t;
                            }
                            find_sa_element(j as small_number, cur_val, true_0 != 0);
                            let ref mut fresh22 =
                                (*mem.offset((cur_ptr + 1 as int32_t) as isize)).b32.s0;
                            *fresh22 += 1;
                            if j == TOK_VAL as int32_t {
                                j = TOKS_REGISTER as int32_t;
                            } else {
                                j = REGISTER as int32_t;
                            }
                            if a as ::core::ffi::c_int >= 4 as ::core::ffi::c_int {
                                geq_define(p, j as uint16_t, cur_ptr);
                            } else {
                                eq_define(p, j as uint16_t, cur_ptr);
                            }
                        } else {
                            match n {
                                COUNT_DEF_CODE => {
                                    if a as ::core::ffi::c_int >= 4 as ::core::ffi::c_int {
                                        geq_define(
                                            p,
                                            ASSIGN_INT as uint16_t,
                                            COUNT_BASE as int32_t + cur_val,
                                        );
                                    } else {
                                        eq_define(
                                            p,
                                            ASSIGN_INT as uint16_t,
                                            COUNT_BASE as int32_t + cur_val,
                                        );
                                    }
                                }
                                DIMEN_DEF_CODE => {
                                    if a as ::core::ffi::c_int >= 4 as ::core::ffi::c_int {
                                        geq_define(
                                            p,
                                            ASSIGN_DIMEN as uint16_t,
                                            SCALED_BASE as int32_t + cur_val,
                                        );
                                    } else {
                                        eq_define(
                                            p,
                                            ASSIGN_DIMEN as uint16_t,
                                            SCALED_BASE as int32_t + cur_val,
                                        );
                                    }
                                }
                                SKIP_DEF_CODE => {
                                    if a as ::core::ffi::c_int >= 4 as ::core::ffi::c_int {
                                        geq_define(
                                            p,
                                            ASSIGN_GLUE as uint16_t,
                                            SKIP_BASE as int32_t + cur_val,
                                        );
                                    } else {
                                        eq_define(
                                            p,
                                            ASSIGN_GLUE as uint16_t,
                                            SKIP_BASE as int32_t + cur_val,
                                        );
                                    }
                                }
                                MU_SKIP_DEF_CODE => {
                                    if a as ::core::ffi::c_int >= 4 as ::core::ffi::c_int {
                                        geq_define(
                                            p,
                                            ASSIGN_MU_GLUE as uint16_t,
                                            MU_SKIP_BASE as int32_t + cur_val,
                                        );
                                    } else {
                                        eq_define(
                                            p,
                                            ASSIGN_MU_GLUE as uint16_t,
                                            MU_SKIP_BASE as int32_t + cur_val,
                                        );
                                    }
                                }
                                TOKS_DEF_CODE => {
                                    if a as ::core::ffi::c_int >= 4 as ::core::ffi::c_int {
                                        geq_define(
                                            p,
                                            ASSIGN_TOKS as uint16_t,
                                            TOKS_BASE as int32_t + cur_val,
                                        );
                                    } else {
                                        eq_define(
                                            p,
                                            ASSIGN_TOKS as uint16_t,
                                            TOKS_BASE as int32_t + cur_val,
                                        );
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
        }
        READ_TO_CS => {
            j = cur_chr;
            scan_int();
            n = cur_val;
            if !scan_keyword(b"to\0" as *const u8 as *const ::core::ffi::c_char) {
                error_here_with_diagnostic(
                    b"Missing `to' inserted\0" as *const u8 as *const ::core::ffi::c_char,
                );
                capture_to_diagnostic(::core::ptr::null_mut::<ttbc_diagnostic_t>());
                help_ptr = 2 as ::core::ffi::c_uchar;
                help_line[1 as ::core::ffi::c_int as usize] =
                    b"You should have said `\\read<number> to \\cs'.\0" as *const u8
                        as *const ::core::ffi::c_char;
                help_line[0 as ::core::ffi::c_int as usize] =
                    b"I'm going to look for the \\cs now.\0" as *const u8
                        as *const ::core::ffi::c_char;
                error();
            }
            get_r_token();
            p = cur_cs;
            read_toks(n, p, j);
            if a as ::core::ffi::c_int >= 4 as ::core::ffi::c_int {
                geq_define(p, CALL as uint16_t, cur_val);
            } else {
                eq_define(p, CALL as uint16_t, cur_val);
            }
        }
        TOKS_REGISTER | ASSIGN_TOKS => {
            q = cur_cs;
            e = false_0 != 0;
            if cur_cmd as ::core::ffi::c_int == TOKS_REGISTER {
                if cur_chr == 0 as int32_t {
                    scan_register_num();
                    if cur_val > 255 as int32_t {
                        find_sa_element(TOK_VAL as small_number, cur_val, true_0 != 0);
                        cur_chr = cur_ptr;
                        e = true_0 != 0;
                    } else {
                        cur_chr = TOKS_BASE as int32_t + cur_val;
                    }
                } else {
                    e = true_0 != 0;
                }
            } else if cur_chr == LOCAL_BASE as int32_t + LOCAL__xetex_inter_char_toks as int32_t {
                scan_char_class_not_ignored();
                cur_ptr = cur_val;
                scan_char_class_not_ignored();
                find_sa_element(
                    INTER_CHAR_VAL as small_number,
                    cur_ptr * CHAR_CLASS_LIMIT as int32_t + cur_val,
                    true_0 != 0,
                );
                cur_chr = cur_ptr;
                e = true_0 != 0;
            }
            p = cur_chr;
            scan_optional_equals();
            loop {
                get_x_token();
                if !(cur_cmd as ::core::ffi::c_int == SPACER
                    || cur_cmd as ::core::ffi::c_int == RELAX)
                {
                    break;
                }
            }
            if cur_cmd as ::core::ffi::c_int != LEFT_BRACE {
                if cur_cmd as ::core::ffi::c_int == TOKS_REGISTER
                    || cur_cmd as ::core::ffi::c_int == ASSIGN_TOKS
                {
                    if cur_cmd as ::core::ffi::c_int == TOKS_REGISTER {
                        if cur_chr == 0 as int32_t {
                            scan_register_num();
                            if cur_val < 256 as int32_t {
                                q = (*eqtb.offset((TOKS_BASE as int32_t + cur_val) as isize))
                                    .b32
                                    .s1;
                            } else {
                                find_sa_element(TOK_VAL as small_number, cur_val, false_0 != 0);
                                if cur_ptr == TEX_NULL as int32_t {
                                    q = TEX_NULL as int32_t;
                                } else {
                                    q = (*mem.offset((cur_ptr + 1 as int32_t) as isize)).b32.s1;
                                }
                            }
                        } else {
                            q = (*mem.offset((cur_chr + 1 as int32_t) as isize)).b32.s1;
                        }
                    } else if cur_chr
                        == LOCAL_BASE as int32_t + LOCAL__xetex_inter_char_toks as int32_t
                    {
                        scan_char_class_not_ignored();
                        cur_ptr = cur_val;
                        scan_char_class_not_ignored();
                        find_sa_element(
                            INTER_CHAR_VAL as small_number,
                            cur_ptr * CHAR_CLASS_LIMIT as int32_t + cur_val,
                            false_0 != 0,
                        );
                        if cur_ptr == TEX_NULL as int32_t {
                            q = TEX_NULL as int32_t;
                        } else {
                            q = (*mem.offset((cur_ptr + 1 as int32_t) as isize)).b32.s1;
                        }
                    } else {
                        q = (*eqtb.offset(cur_chr as isize)).b32.s1;
                    }
                    if q == TEX_NULL as int32_t {
                        if e {
                            if a as ::core::ffi::c_int >= 4 as ::core::ffi::c_int {
                                gsa_def(p, TEX_NULL as int32_t);
                            } else {
                                sa_def(p, TEX_NULL as int32_t);
                            }
                        } else if a as ::core::ffi::c_int >= 4 as ::core::ffi::c_int {
                            geq_define(p, UNDEFINED_CS as uint16_t, TEX_NULL as int32_t);
                        } else {
                            eq_define(p, UNDEFINED_CS as uint16_t, TEX_NULL as int32_t);
                        }
                    } else {
                        let ref mut fresh23 = (*mem.offset(q as isize)).b32.s0;
                        *fresh23 += 1;
                        if e {
                            if a as ::core::ffi::c_int >= 4 as ::core::ffi::c_int {
                                gsa_def(p, q);
                            } else {
                                sa_def(p, q);
                            }
                        } else if a as ::core::ffi::c_int >= 4 as ::core::ffi::c_int {
                            geq_define(p, CALL as uint16_t, q);
                        } else {
                            eq_define(p, CALL as uint16_t, q);
                        }
                    }
                    current_block = 15461963850607122465;
                } else {
                    current_block = 3906822848181906220;
                }
            } else {
                current_block = 3906822848181906220;
            }
            match current_block {
                15461963850607122465 => {}
                _ => {
                    back_input();
                    cur_cs = q;
                    q = scan_toks(false_0 != 0, false_0 != 0);
                    if (*mem.offset(def_ref as isize)).b32.s1 == TEX_NULL as int32_t {
                        if e {
                            if a as ::core::ffi::c_int >= 4 as ::core::ffi::c_int {
                                gsa_def(p, TEX_NULL as int32_t);
                            } else {
                                sa_def(p, TEX_NULL as int32_t);
                            }
                        } else if a as ::core::ffi::c_int >= 4 as ::core::ffi::c_int {
                            geq_define(p, UNDEFINED_CS as uint16_t, TEX_NULL as int32_t);
                        } else {
                            eq_define(p, UNDEFINED_CS as uint16_t, TEX_NULL as int32_t);
                        }
                        (*mem.offset(def_ref as isize)).b32.s1 = avail;
                        avail = def_ref;
                    } else {
                        if p == LOCAL_BASE as int32_t + LOCAL__output_routine as int32_t && !e {
                            (*mem.offset(q as isize)).b32.s1 = get_avail();
                            q = (*mem.offset(q as isize)).b32.s1;
                            (*mem.offset(q as isize)).b32.s0 =
                                (RIGHT_BRACE_TOKEN + 125 as ::core::ffi::c_int) as int32_t;
                            q = get_avail();
                            (*mem.offset(q as isize)).b32.s0 =
                                (LEFT_BRACE_TOKEN + 123 as ::core::ffi::c_int) as int32_t;
                            (*mem.offset(q as isize)).b32.s1 =
                                (*mem.offset(def_ref as isize)).b32.s1;
                            (*mem.offset(def_ref as isize)).b32.s1 = q;
                        }
                        if e {
                            if a as ::core::ffi::c_int >= 4 as ::core::ffi::c_int {
                                gsa_def(p, def_ref);
                            } else {
                                sa_def(p, def_ref);
                            }
                        } else if a as ::core::ffi::c_int >= 4 as ::core::ffi::c_int {
                            geq_define(p, CALL as uint16_t, def_ref);
                        } else {
                            eq_define(p, CALL as uint16_t, def_ref);
                        }
                    }
                }
            }
        }
        ASSIGN_INT => {
            p = cur_chr;
            scan_optional_equals();
            scan_int();
            if a as ::core::ffi::c_int >= 4 as ::core::ffi::c_int {
                geq_word_define(p, cur_val);
            } else {
                eq_word_define(p, cur_val);
            }
        }
        ASSIGN_DIMEN => {
            p = cur_chr;
            scan_optional_equals();
            scan_dimen(false_0 != 0, false_0 != 0, false_0 != 0);
            if a as ::core::ffi::c_int >= 4 as ::core::ffi::c_int {
                geq_word_define(p, cur_val);
            } else {
                eq_word_define(p, cur_val);
            }
        }
        ASSIGN_GLUE | ASSIGN_MU_GLUE => {
            p = cur_chr;
            n = cur_cmd as int32_t;
            scan_optional_equals();
            if n == ASSIGN_MU_GLUE as int32_t {
                scan_glue(MU_VAL as small_number);
            } else {
                scan_glue(GLUE_VAL as small_number);
            }
            trap_zero_glue();
            if a as ::core::ffi::c_int >= 4 as ::core::ffi::c_int {
                geq_define(p, GLUE_REF as uint16_t, cur_val);
            } else {
                eq_define(p, GLUE_REF as uint16_t, cur_val);
            }
        }
        XETEX_DEF_CODE => {
            if cur_chr == SF_CODE_BASE as int32_t {
                p = cur_chr;
                scan_usv_num();
                p = p + cur_val;
                n = ((*eqtb.offset((SF_CODE_BASE as int32_t + cur_val) as isize))
                    .b32
                    .s1 as ::core::ffi::c_long
                    % 65536 as ::core::ffi::c_long) as int32_t;
                scan_optional_equals();
                scan_char_class();
                if a as ::core::ffi::c_int >= 4 as ::core::ffi::c_int {
                    geq_define(
                        p,
                        DATA as uint16_t,
                        (cur_val as ::core::ffi::c_long * 65536 as ::core::ffi::c_long
                            + n as ::core::ffi::c_long) as int32_t,
                    );
                } else {
                    eq_define(
                        p,
                        DATA as uint16_t,
                        (cur_val as ::core::ffi::c_long * 65536 as ::core::ffi::c_long
                            + n as ::core::ffi::c_long) as int32_t,
                    );
                }
            } else if cur_chr == MATH_CODE_BASE as int32_t {
                p = cur_chr;
                scan_usv_num();
                p = p + cur_val;
                scan_optional_equals();
                scan_xetex_math_char_int();
                if a as ::core::ffi::c_int >= 4 as ::core::ffi::c_int {
                    geq_define(p, DATA as uint16_t, cur_val);
                } else {
                    eq_define(p, DATA as uint16_t, cur_val);
                }
            } else if cur_chr == MATH_CODE_BASE as int32_t + 1 as int32_t {
                p = cur_chr - 1 as int32_t;
                scan_usv_num();
                p = p + cur_val;
                scan_optional_equals();
                scan_math_class_int();
                n = ((cur_val as ::core::ffi::c_uint & 0x7 as ::core::ffi::c_uint)
                    << 21 as ::core::ffi::c_int) as int32_t;
                scan_math_fam_int();
                n = (n as ::core::ffi::c_uint).wrapping_add(
                    (cur_val as ::core::ffi::c_uint & 0xff as ::core::ffi::c_uint)
                        << 24 as ::core::ffi::c_int,
                ) as int32_t;
                scan_usv_num();
                n = n + cur_val;
                if a as ::core::ffi::c_int >= 4 as ::core::ffi::c_int {
                    geq_define(p, DATA as uint16_t, n);
                } else {
                    eq_define(p, DATA as uint16_t, n);
                }
            } else if cur_chr == DEL_CODE_BASE as int32_t {
                p = cur_chr;
                scan_usv_num();
                p = p + cur_val;
                scan_optional_equals();
                scan_int();
                if a as ::core::ffi::c_int >= 4 as ::core::ffi::c_int {
                    geq_word_define(p, cur_val);
                } else {
                    eq_word_define(p, cur_val);
                }
            } else {
                p = cur_chr - 1 as int32_t;
                scan_usv_num();
                p = p + cur_val;
                scan_optional_equals();
                n = 0x40000000 as ::core::ffi::c_int as int32_t;
                scan_math_fam_int();
                n = n + cur_val * 0x200000 as int32_t;
                scan_usv_num();
                n = n + cur_val;
                if a as ::core::ffi::c_int >= 4 as ::core::ffi::c_int {
                    geq_word_define(p, n);
                } else {
                    eq_word_define(p, n);
                }
            }
        }
        DEF_CODE => {
            if cur_chr == CAT_CODE_BASE as int32_t {
                n = MAX_CHAR_CODE as int32_t;
            } else if cur_chr == MATH_CODE_BASE as int32_t {
                n = 0x8000 as ::core::ffi::c_int as int32_t;
            } else if cur_chr == SF_CODE_BASE as int32_t {
                n = 0x7fff as ::core::ffi::c_int as int32_t;
            } else if cur_chr == DEL_CODE_BASE as int32_t {
                n = 0xffffff as ::core::ffi::c_int as int32_t;
            } else {
                n = BIGGEST_USV as int32_t;
            }
            p = cur_chr;
            scan_usv_num();
            p = p + cur_val;
            scan_optional_equals();
            scan_int();
            if cur_val < 0 as int32_t && p < DEL_CODE_BASE as int32_t || cur_val > n {
                error_here_with_diagnostic(
                    b"Invalid code (\0" as *const u8 as *const ::core::ffi::c_char,
                );
                print_int(cur_val);
                if p < DEL_CODE_BASE as int32_t {
                    print_cstr(
                        b"), should be in the range 0..\0" as *const u8
                            as *const ::core::ffi::c_char,
                    );
                } else {
                    print_cstr(
                        b"), should be at most \0" as *const u8 as *const ::core::ffi::c_char,
                    );
                }
                print_int(n);
                capture_to_diagnostic(::core::ptr::null_mut::<ttbc_diagnostic_t>());
                help_ptr = 1 as ::core::ffi::c_uchar;
                help_line[0 as ::core::ffi::c_int as usize] =
                    b"I'm going to use 0 instead of that illegal code value.\0" as *const u8
                        as *const ::core::ffi::c_char;
                error();
                cur_val = 0 as ::core::ffi::c_int as int32_t;
            }
            if p < MATH_CODE_BASE as int32_t {
                if p >= SF_CODE_BASE as int32_t {
                    n = ((*eqtb.offset(p as isize)).b32.s1 as ::core::ffi::c_long
                        / 65536 as ::core::ffi::c_long) as int32_t;
                    if a as ::core::ffi::c_int >= 4 as ::core::ffi::c_int {
                        geq_define(
                            p,
                            DATA as uint16_t,
                            (n as ::core::ffi::c_long * 65536 as ::core::ffi::c_long
                                + cur_val as ::core::ffi::c_long)
                                as int32_t,
                        );
                    } else {
                        eq_define(
                            p,
                            DATA as uint16_t,
                            (n as ::core::ffi::c_long * 65536 as ::core::ffi::c_long
                                + cur_val as ::core::ffi::c_long)
                                as int32_t,
                        );
                    }
                } else if a as ::core::ffi::c_int >= 4 as ::core::ffi::c_int {
                    geq_define(p, DATA as uint16_t, cur_val);
                } else {
                    eq_define(p, DATA as uint16_t, cur_val);
                }
            } else if p < DEL_CODE_BASE as int32_t {
                if cur_val as ::core::ffi::c_long == 32768 as ::core::ffi::c_long {
                    cur_val = ACTIVE_MATH_CHAR as int32_t;
                } else {
                    cur_val = (((cur_val / 4096 as int32_t) as ::core::ffi::c_uint
                        & 0x7 as ::core::ffi::c_uint)
                        << 21 as ::core::ffi::c_int)
                        .wrapping_add(
                            ((cur_val % 4096 as int32_t / 256 as int32_t) as ::core::ffi::c_uint
                                & 0xff as ::core::ffi::c_uint)
                                << 24 as ::core::ffi::c_int,
                        )
                        .wrapping_add((cur_val % 256 as int32_t) as ::core::ffi::c_uint)
                        as int32_t;
                }
                if a as ::core::ffi::c_int >= 4 as ::core::ffi::c_int {
                    geq_define(p, DATA as uint16_t, cur_val);
                } else {
                    eq_define(p, DATA as uint16_t, cur_val);
                }
            } else if a as ::core::ffi::c_int >= 4 as ::core::ffi::c_int {
                geq_word_define(p, cur_val);
            } else {
                eq_word_define(p, cur_val);
            }
        }
        DEF_FAMILY => {
            p = cur_chr;
            scan_math_fam_int();
            p = p + cur_val;
            scan_optional_equals();
            scan_font_ident();
            if a as ::core::ffi::c_int >= 4 as ::core::ffi::c_int {
                geq_define(p, DATA as uint16_t, cur_val);
            } else {
                eq_define(p, DATA as uint16_t, cur_val);
            }
        }
        REGISTER | ADVANCE | MULTIPLY | DIVIDE => {
            do_register_command(a);
        }
        SET_BOX => {
            scan_register_num();
            if a as ::core::ffi::c_int >= 4 as ::core::ffi::c_int {
                n = GLOBAL_BOX_FLAG as int32_t + cur_val;
            } else {
                n = BOX_FLAG as int32_t + cur_val;
            }
            scan_optional_equals();
            if set_box_allowed {
                scan_box(n);
            } else {
                error_here_with_diagnostic(
                    b"Improper \0" as *const u8 as *const ::core::ffi::c_char,
                );
                print_esc_cstr(b"setbox\0" as *const u8 as *const ::core::ffi::c_char);
                capture_to_diagnostic(::core::ptr::null_mut::<ttbc_diagnostic_t>());
                help_ptr = 2 as ::core::ffi::c_uchar;
                help_line[1 as ::core::ffi::c_int as usize] =
                    b"Sorry, \\setbox is not allowed after \\halign in a display,\0" as *const u8
                        as *const ::core::ffi::c_char;
                help_line[0 as ::core::ffi::c_int as usize] =
                    b"or between \\accent and an accented character.\0" as *const u8
                        as *const ::core::ffi::c_char;
                error();
            }
        }
        SET_AUX => {
            alter_aux();
        }
        SET_PREV_GRAF => {
            alter_prev_graf();
        }
        SET_PAGE_DIMEN => {
            alter_page_so_far();
        }
        SET_PAGE_INT => {
            alter_integer();
        }
        SET_BOX_DIMEN => {
            alter_box_dimen();
        }
        SET_SHAPE => {
            q = cur_chr;
            scan_optional_equals();
            scan_int();
            n = cur_val;
            if n <= 0 as int32_t {
                p = TEX_NULL as int32_t;
            } else if q > LOCAL_BASE as int32_t + LOCAL__par_shape as int32_t {
                n = cur_val / 2 as int32_t + 1 as int32_t;
                p = get_node(2 as int32_t * n + 1 as int32_t);
                (*mem.offset(p as isize)).b32.s0 = n;
                n = cur_val;
                (*mem.offset((p + 1 as int32_t) as isize)).b32.s1 = n;
                j = p + 2 as int32_t;
                while j <= p + n + 1 as int32_t {
                    scan_int();
                    (*mem.offset(j as isize)).b32.s1 = cur_val;
                    j += 1;
                }
                if n & 1 as int32_t == 0 {
                    (*mem.offset((p + n + 2 as int32_t) as isize)).b32.s1 =
                        0 as ::core::ffi::c_int as int32_t;
                }
            } else {
                p = get_node(2 as int32_t * n + 1 as int32_t);
                (*mem.offset(p as isize)).b32.s0 = n;
                j = 1 as ::core::ffi::c_int as int32_t;
                while j <= n {
                    scan_dimen(false_0 != 0, false_0 != 0, false_0 != 0);
                    (*mem.offset((p + 2 as int32_t * j - 1 as int32_t) as isize))
                        .b32
                        .s1 = cur_val;
                    scan_dimen(false_0 != 0, false_0 != 0, false_0 != 0);
                    (*mem.offset((p + 2 as int32_t * j) as isize)).b32.s1 = cur_val;
                    j += 1;
                }
            }
            if a as ::core::ffi::c_int >= 4 as ::core::ffi::c_int {
                geq_define(q, SHAPE_REF as uint16_t, p);
            } else {
                eq_define(q, SHAPE_REF as uint16_t, p);
            }
        }
        HYPH_DATA => {
            if cur_chr == 1 as int32_t {
                if in_initex_mode {
                    new_patterns();
                } else {
                    error_here_with_diagnostic(
                        b"Patterns can be loaded only by INITEX\0" as *const u8
                            as *const ::core::ffi::c_char,
                    );
                    capture_to_diagnostic(::core::ptr::null_mut::<ttbc_diagnostic_t>());
                    help_ptr = 0 as ::core::ffi::c_uchar;
                    error();
                    loop {
                        get_token();
                        if !(cur_cmd as ::core::ffi::c_int != RIGHT_BRACE) {
                            break;
                        }
                    }
                    return;
                }
            } else {
                new_hyph_exceptions();
            }
        }
        ASSIGN_FONT_DIMEN => {
            find_font_dimen(true_0 != 0);
            k = cur_val as font_index;
            scan_optional_equals();
            scan_dimen(false_0 != 0, false_0 != 0, false_0 != 0);
            (*font_info.offset(k as isize)).b32.s1 = cur_val;
        }
        ASSIGN_FONT_INT => {
            n = cur_chr;
            scan_font_ident();
            f = cur_val as internal_font_number;
            if n < 2 as int32_t {
                scan_optional_equals();
                scan_int();
                if n == 0 as int32_t {
                    *hyphen_char.offset(f as isize) = cur_val;
                } else {
                    *skew_char.offset(f as isize) = cur_val;
                }
            } else {
                if *font_area.offset(f as isize) as ::core::ffi::c_uint == AAT_FONT_FLAG
                    || *font_area.offset(f as isize) as ::core::ffi::c_uint == OTGR_FONT_FLAG
                {
                    scan_glyph_number(f);
                } else {
                    scan_char_num();
                }
                p = cur_val;
                scan_optional_equals();
                scan_int();
                match n {
                    100..=106 => { crate::engine_fonts::set_code(n, f as i32, p, cur_val); }
                    LP_CODE_BASE => {
                        set_cp_code(
                            f as ::core::ffi::c_int,
                            p as ::core::ffi::c_uint,
                            LEFT_SIDE,
                            cur_val as ::core::ffi::c_int,
                        );
                    }
                    RP_CODE_BASE => {
                        set_cp_code(
                            f as ::core::ffi::c_int,
                            p as ::core::ffi::c_uint,
                            RIGHT_SIDE,
                            cur_val as ::core::ffi::c_int,
                        );
                    }
                    _ => {}
                }
            }
        }
        DEF_FONT => {
            new_font(a);
        }
        SET_INTERACTION => {
            new_interaction();
        }
        _ => {
            confusion(b"prefix\0" as *const u8 as *const ::core::ffi::c_char);
        }
    }
    if after_token != 0 as int32_t {
        cur_tok = after_token;
        back_input();
        after_token = 0 as ::core::ffi::c_int as int32_t;
    }
}
unsafe extern "C" fn store_fmt_file() {
    let mut current_block: u64;
    let mut j: int32_t = 0;
    let mut k: int32_t = 0;
    let mut l: int32_t = 0;
    let mut p: int32_t = 0;
    let mut q: int32_t = 0;
    let mut x: int32_t = 0;
    let mut fmt_out: rust_output_handle_t = ::core::ptr::null_mut::<ttbc_output_handle_t>();
    if save_ptr != 0 as int32_t {
        error_here_with_diagnostic(
            b"You can't dump inside a group\0" as *const u8 as *const ::core::ffi::c_char,
        );
        capture_to_diagnostic(::core::ptr::null_mut::<ttbc_diagnostic_t>());
        help_ptr = 1 as ::core::ffi::c_uchar;
        help_line[0 as ::core::ffi::c_int as usize] =
            b"`{...\\dump}' is a no-no.\0" as *const u8 as *const ::core::ffi::c_char;
        if interaction as ::core::ffi::c_int == ERROR_STOP_MODE {
            interaction = SCROLL_MODE as ::core::ffi::c_uchar;
        }
        if log_opened {
            error();
        }
        history = HISTORY_FATAL_ERROR;
        close_files_and_terminate();
        ttstub_output_flush(rust_stdout);
        _tt_abort(b"\\dump inside a group\0" as *const u8 as *const ::core::ffi::c_char);
    }
    selector = SELECTOR_NEW_STRING;
    print_cstr(b" (preloaded format=\0" as *const u8 as *const ::core::ffi::c_char);
    print(job_name as int32_t);
    print_char(' ' as i32);
    print_int((*eqtb.offset((INT_BASE + INT_PAR__year) as isize)).b32.s1);
    print_char('.' as i32);
    print_int((*eqtb.offset((INT_BASE + INT_PAR__month) as isize)).b32.s1);
    print_char('.' as i32);
    print_int((*eqtb.offset((INT_BASE + INT_PAR__day) as isize)).b32.s1);
    print_char(')' as i32);
    if interaction as ::core::ffi::c_int == BATCH_MODE {
        selector = SELECTOR_LOG_ONLY;
    } else {
        selector = SELECTOR_TERM_AND_LOG;
    }
    if pool_ptr + 1 as pool_pointer > pool_size {
        overflow(
            b"pool size\0" as *const u8 as *const ::core::ffi::c_char,
            pool_size - init_pool_ptr as int32_t,
        );
    }
    format_ident = make_string();
    pack_job_name(b".fmt\0" as *const u8 as *const ::core::ffi::c_char);
    fmt_out = ttstub_output_open_format(name_of_file, 0 as ::core::ffi::c_int);
    if fmt_out.is_null() {
        _tt_abort(
            b"cannot open format output file \"%s\"\0" as *const u8 as *const ::core::ffi::c_char,
            name_of_file,
        );
    }
    print_nl_cstr(b"Beginning to dump on file \0" as *const u8 as *const ::core::ffi::c_char);
    print(make_name_string() as int32_t);
    str_ptr -= 1;
    pool_ptr = *str_start.offset((str_ptr - TOO_BIG_CHAR as str_number) as isize);
    print_nl_cstr(b"\0" as *const u8 as *const ::core::ffi::c_char);
    print(format_ident as int32_t);
    let mut x_val: int32_t = 0x54544e43 as int32_t;
    do_dump(
        &raw mut x_val as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<int32_t>() as size_t,
        1 as ::core::ffi::c_int as size_t,
        fmt_out,
    );
    let mut x_val_0: int32_t = FORMAT_SERIAL as int32_t;
    do_dump(
        &raw mut x_val_0 as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<int32_t>() as size_t,
        1 as ::core::ffi::c_int as size_t,
        fmt_out,
    );
    let mut x_val_1: int32_t = hash_high;
    do_dump(
        &raw mut x_val_1 as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<int32_t>() as size_t,
        1 as ::core::ffi::c_int as size_t,
        fmt_out,
    );
    while pseudo_files != TEX_NULL as int32_t {
        pseudo_close();
    }
    let mut x_val_2: int32_t = 4999999 as int32_t;
    do_dump(
        &raw mut x_val_2 as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<int32_t>() as size_t,
        1 as ::core::ffi::c_int as size_t,
        fmt_out,
    );
    let mut x_val_3: int32_t = EQTB_SIZE as int32_t;
    do_dump(
        &raw mut x_val_3 as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<int32_t>() as size_t,
        1 as ::core::ffi::c_int as size_t,
        fmt_out,
    );
    let mut x_val_4: int32_t = 8501 as int32_t;
    do_dump(
        &raw mut x_val_4 as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<int32_t>() as size_t,
        1 as ::core::ffi::c_int as size_t,
        fmt_out,
    );
    let mut x_val_5: int32_t = 607 as int32_t;
    do_dump(
        &raw mut x_val_5 as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<int32_t>() as size_t,
        1 as ::core::ffi::c_int as size_t,
        fmt_out,
    );
    let mut x_val_6: int32_t = pool_ptr as int32_t;
    do_dump(
        &raw mut x_val_6 as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<int32_t>() as size_t,
        1 as ::core::ffi::c_int as size_t,
        fmt_out,
    );
    let mut x_val_7: int32_t = str_ptr as int32_t;
    do_dump(
        &raw mut x_val_7 as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<int32_t>() as size_t,
        1 as ::core::ffi::c_int as size_t,
        fmt_out,
    );
    do_dump(
        str_start.offset(0 as ::core::ffi::c_int as isize) as *mut pool_pointer
            as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<pool_pointer>() as size_t,
        (str_ptr - 65536 as str_number + 1 as str_number) as size_t,
        fmt_out,
    );
    do_dump(
        str_pool.offset(0 as ::core::ffi::c_int as isize) as *mut packed_UTF16_code
            as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<packed_UTF16_code>() as size_t,
        pool_ptr as size_t,
        fmt_out,
    );
    print_ln();
    print_int(str_ptr as int32_t);
    print_cstr(b" strings of total length \0" as *const u8 as *const ::core::ffi::c_char);
    print_int(pool_ptr as int32_t);
    sort_avail();
    var_used = 0 as ::core::ffi::c_int as int32_t;
    let mut x_val_8: int32_t = lo_mem_max;
    do_dump(
        &raw mut x_val_8 as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<int32_t>() as size_t,
        1 as ::core::ffi::c_int as size_t,
        fmt_out,
    );
    let mut x_val_9: int32_t = rover;
    do_dump(
        &raw mut x_val_9 as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<int32_t>() as size_t,
        1 as ::core::ffi::c_int as size_t,
        fmt_out,
    );
    k = INT_VAL as int32_t;
    while k <= INTER_CHAR_VAL as int32_t {
        let mut x_val_10: int32_t = sa_root[k as usize];
        do_dump(
            &raw mut x_val_10 as *mut ::core::ffi::c_char,
            ::core::mem::size_of::<int32_t>() as size_t,
            1 as ::core::ffi::c_int as size_t,
            fmt_out,
        );
        k += 1;
    }
    p = 0 as ::core::ffi::c_int as int32_t;
    q = rover;
    x = 0 as ::core::ffi::c_int as int32_t;
    loop {
        do_dump(
            mem.offset(p as isize) as *mut memory_word as *mut ::core::ffi::c_char,
            ::core::mem::size_of::<memory_word>() as size_t,
            (q + 2 as int32_t - p) as size_t,
            fmt_out,
        );
        x = x + q + 2 as int32_t - p;
        var_used = var_used + q - p;
        p = q + (*mem.offset(q as isize)).b32.s0;
        q = (*mem.offset((q + 1 as int32_t) as isize)).b32.s1;
        if !(q != rover) {
            break;
        }
    }
    var_used = var_used + lo_mem_max - p;
    dyn_used = mem_end + 1 as int32_t - hi_mem_min;
    do_dump(
        mem.offset(p as isize) as *mut memory_word as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<memory_word>() as size_t,
        (lo_mem_max + 1 as int32_t - p) as size_t,
        fmt_out,
    );
    x = x + lo_mem_max + 1 as int32_t - p;
    let mut x_val_11: int32_t = hi_mem_min;
    do_dump(
        &raw mut x_val_11 as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<int32_t>() as size_t,
        1 as ::core::ffi::c_int as size_t,
        fmt_out,
    );
    let mut x_val_12: int32_t = avail;
    do_dump(
        &raw mut x_val_12 as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<int32_t>() as size_t,
        1 as ::core::ffi::c_int as size_t,
        fmt_out,
    );
    do_dump(
        mem.offset(hi_mem_min as isize) as *mut memory_word as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<memory_word>() as size_t,
        (mem_end + 1 as int32_t - hi_mem_min) as size_t,
        fmt_out,
    );
    x = x + mem_end + 1 as int32_t - hi_mem_min;
    p = avail;
    while p != TEX_NULL as int32_t {
        dyn_used -= 1;
        p = (*mem.offset(p as isize)).b32.s1;
    }
    let mut x_val_13: int32_t = var_used;
    do_dump(
        &raw mut x_val_13 as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<int32_t>() as size_t,
        1 as ::core::ffi::c_int as size_t,
        fmt_out,
    );
    let mut x_val_14: int32_t = dyn_used;
    do_dump(
        &raw mut x_val_14 as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<int32_t>() as size_t,
        1 as ::core::ffi::c_int as size_t,
        fmt_out,
    );
    print_ln();
    print_int(x);
    print_cstr(
        b" memory locations dumped; current usage is \0" as *const u8 as *const ::core::ffi::c_char,
    );
    print_int(var_used);
    print_char('&' as i32);
    print_int(dyn_used);
    k = ACTIVE_BASE as int32_t;
    loop {
        j = k;
        loop {
            if !(j < INT_BASE as int32_t - 1 as int32_t) {
                current_block = 5846959088466685742;
                break;
            }
            if (*eqtb.offset(j as isize)).b32.s1
                == (*eqtb.offset((j + 1 as int32_t) as isize)).b32.s1
                && (*eqtb.offset(j as isize)).b16.s1 as ::core::ffi::c_int
                    == (*eqtb.offset((j + 1 as int32_t) as isize)).b16.s1 as ::core::ffi::c_int
                && (*eqtb.offset(j as isize)).b16.s0 as ::core::ffi::c_int
                    == (*eqtb.offset((j + 1 as int32_t) as isize)).b16.s0 as ::core::ffi::c_int
            {
                current_block = 2019795833981416981;
                break;
            }
            j += 1;
        }
        match current_block {
            5846959088466685742 => {
                l = INT_BASE as int32_t;
            }
            _ => {
                j += 1;
                l = j;
                while j < INT_BASE as int32_t - 1 as int32_t {
                    if (*eqtb.offset(j as isize)).b32.s1
                        != (*eqtb.offset((j + 1 as int32_t) as isize)).b32.s1
                        || (*eqtb.offset(j as isize)).b16.s1 as ::core::ffi::c_int
                            != (*eqtb.offset((j + 1 as int32_t) as isize)).b16.s1
                                as ::core::ffi::c_int
                        || (*eqtb.offset(j as isize)).b16.s0 as ::core::ffi::c_int
                            != (*eqtb.offset((j + 1 as int32_t) as isize)).b16.s0
                                as ::core::ffi::c_int
                    {
                        break;
                    }
                    j += 1;
                }
            }
        }
        let mut x_val_15: int32_t = l - k;
        do_dump(
            &raw mut x_val_15 as *mut ::core::ffi::c_char,
            ::core::mem::size_of::<int32_t>() as size_t,
            1 as ::core::ffi::c_int as size_t,
            fmt_out,
        );
        do_dump(
            eqtb.offset(k as isize) as *mut memory_word as *mut ::core::ffi::c_char,
            ::core::mem::size_of::<memory_word>() as size_t,
            (l - k) as size_t,
            fmt_out,
        );
        k = j + 1 as int32_t;
        let mut x_val_16: int32_t = k - l;
        do_dump(
            &raw mut x_val_16 as *mut ::core::ffi::c_char,
            ::core::mem::size_of::<int32_t>() as size_t,
            1 as ::core::ffi::c_int as size_t,
            fmt_out,
        );
        if !(k != INT_BASE as int32_t) {
            break;
        }
    }
    loop {
        j = k;
        loop {
            if !(j < EQTB_SIZE as int32_t) {
                current_block = 18039443766442739006;
                break;
            }
            if (*eqtb.offset(j as isize)).b32.s1
                == (*eqtb.offset((j + 1 as int32_t) as isize)).b32.s1
            {
                current_block = 12567515199740373004;
                break;
            }
            j += 1;
        }
        match current_block {
            18039443766442739006 => {
                l = (EQTB_SIZE + 1 as ::core::ffi::c_int) as int32_t;
            }
            _ => {
                j += 1;
                l = j;
                while j < EQTB_SIZE as int32_t {
                    if (*eqtb.offset(j as isize)).b32.s1
                        != (*eqtb.offset((j + 1 as int32_t) as isize)).b32.s1
                    {
                        break;
                    }
                    j += 1;
                }
            }
        }
        let mut x_val_17: int32_t = l - k;
        do_dump(
            &raw mut x_val_17 as *mut ::core::ffi::c_char,
            ::core::mem::size_of::<int32_t>() as size_t,
            1 as ::core::ffi::c_int as size_t,
            fmt_out,
        );
        do_dump(
            eqtb.offset(k as isize) as *mut memory_word as *mut ::core::ffi::c_char,
            ::core::mem::size_of::<memory_word>() as size_t,
            (l - k) as size_t,
            fmt_out,
        );
        k = j + 1 as int32_t;
        let mut x_val_18: int32_t = k - l;
        do_dump(
            &raw mut x_val_18 as *mut ::core::ffi::c_char,
            ::core::mem::size_of::<int32_t>() as size_t,
            1 as ::core::ffi::c_int as size_t,
            fmt_out,
        );
        if !(k <= EQTB_SIZE as int32_t) {
            break;
        }
    }
    if hash_high > 0 as int32_t {
        do_dump(
            eqtb.offset(
                (7826729 as ::core::ffi::c_int
                    + INT_PARS as ::core::ffi::c_int
                    + 256 as ::core::ffi::c_int
                    + (0x10ffff as ::core::ffi::c_int + 1 as ::core::ffi::c_int)
                    + 23 as ::core::ffi::c_int
                    + 255 as ::core::ffi::c_int
                    + 1 as ::core::ffi::c_int) as isize,
            ) as *mut memory_word as *mut ::core::ffi::c_char,
            ::core::mem::size_of::<memory_word>() as size_t,
            hash_high as size_t,
            fmt_out,
        );
    }
    let mut x_val_19: int32_t = par_loc;
    do_dump(
        &raw mut x_val_19 as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<int32_t>() as size_t,
        1 as ::core::ffi::c_int as size_t,
        fmt_out,
    );
    let mut x_val_20: int32_t = write_loc;
    do_dump(
        &raw mut x_val_20 as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<int32_t>() as size_t,
        1 as ::core::ffi::c_int as size_t,
        fmt_out,
    );
    p = 0 as ::core::ffi::c_int as int32_t;
    while p <= PRIM_SIZE as int32_t {
        do_dump(
            (&raw mut prim as *mut b32x2).offset(p as isize) as *mut b32x2
                as *mut ::core::ffi::c_char,
            ::core::mem::size_of::<b32x2>() as size_t,
            1 as ::core::ffi::c_int as size_t,
            fmt_out,
        );
        p += 1;
    }
    let mut x_val_21: int32_t = hash_used;
    do_dump(
        &raw mut x_val_21 as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<int32_t>() as size_t,
        1 as ::core::ffi::c_int as size_t,
        fmt_out,
    );
    cs_count = FROZEN_CONTROL_SEQUENCE as int32_t - 1 as int32_t - hash_used + hash_high;
    p = HASH_BASE as int32_t;
    while p <= hash_used {
        if (*hash.offset(p as isize)).s1 != 0 as int32_t {
            let mut x_val_22: int32_t = p;
            do_dump(
                &raw mut x_val_22 as *mut ::core::ffi::c_char,
                ::core::mem::size_of::<int32_t>() as size_t,
                1 as ::core::ffi::c_int as size_t,
                fmt_out,
            );
            do_dump(
                hash.offset(p as isize) as *mut b32x2 as *mut ::core::ffi::c_char,
                ::core::mem::size_of::<b32x2>() as size_t,
                1 as ::core::ffi::c_int as size_t,
                fmt_out,
            );
            cs_count += 1;
        }
        p += 1;
    }
    do_dump(
        hash.offset((hash_used + 1 as int32_t) as isize) as *mut b32x2 as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<b32x2>() as size_t,
        (2254339 as int32_t - 1 as int32_t - hash_used) as size_t,
        fmt_out,
    );
    if hash_high > 0 as int32_t {
        do_dump(
            hash.offset(
                (7826729 as ::core::ffi::c_int
                    + INT_PARS as ::core::ffi::c_int
                    + 256 as ::core::ffi::c_int
                    + (0x10ffff as ::core::ffi::c_int + 1 as ::core::ffi::c_int)
                    + 23 as ::core::ffi::c_int
                    + 255 as ::core::ffi::c_int
                    + 1 as ::core::ffi::c_int) as isize,
            ) as *mut b32x2 as *mut ::core::ffi::c_char,
            ::core::mem::size_of::<b32x2>() as size_t,
            hash_high as size_t,
            fmt_out,
        );
    }
    let mut x_val_23: int32_t = cs_count;
    do_dump(
        &raw mut x_val_23 as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<int32_t>() as size_t,
        1 as ::core::ffi::c_int as size_t,
        fmt_out,
    );
    print_ln();
    print_int(cs_count);
    print_cstr(b" multiletter control sequences\0" as *const u8 as *const ::core::ffi::c_char);
    let mut x_val_24: int32_t = fmem_ptr as int32_t;
    do_dump(
        &raw mut x_val_24 as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<int32_t>() as size_t,
        1 as ::core::ffi::c_int as size_t,
        fmt_out,
    );
    do_dump(
        font_info.offset(0 as ::core::ffi::c_int as isize) as *mut memory_word
            as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<memory_word>() as size_t,
        fmem_ptr as size_t,
        fmt_out,
    );
    let mut x_val_25: int32_t = font_ptr as int32_t;
    do_dump(
        &raw mut x_val_25 as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<int32_t>() as size_t,
        1 as ::core::ffi::c_int as size_t,
        fmt_out,
    );
    do_dump(
        font_check.offset(0 as ::core::ffi::c_int as isize) as *mut b16x4
            as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<b16x4>() as size_t,
        (font_ptr + 1 as internal_font_number) as size_t,
        fmt_out,
    );
    do_dump(
        font_size.offset(0 as ::core::ffi::c_int as isize) as *mut scaled_t
            as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<scaled_t>() as size_t,
        (font_ptr + 1 as internal_font_number) as size_t,
        fmt_out,
    );
    do_dump(
        font_dsize.offset(0 as ::core::ffi::c_int as isize) as *mut scaled_t
            as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<scaled_t>() as size_t,
        (font_ptr + 1 as internal_font_number) as size_t,
        fmt_out,
    );
    do_dump(
        font_params.offset(0 as ::core::ffi::c_int as isize) as *mut font_index
            as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<font_index>() as size_t,
        (font_ptr + 1 as internal_font_number) as size_t,
        fmt_out,
    );
    do_dump(
        hyphen_char.offset(0 as ::core::ffi::c_int as isize) as *mut int32_t
            as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<int32_t>() as size_t,
        (font_ptr + 1 as internal_font_number) as size_t,
        fmt_out,
    );
    do_dump(
        skew_char.offset(0 as ::core::ffi::c_int as isize) as *mut int32_t
            as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<int32_t>() as size_t,
        (font_ptr + 1 as internal_font_number) as size_t,
        fmt_out,
    );
    do_dump(
        font_name.offset(0 as ::core::ffi::c_int as isize) as *mut str_number
            as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<str_number>() as size_t,
        (font_ptr + 1 as internal_font_number) as size_t,
        fmt_out,
    );
    do_dump(
        font_area.offset(0 as ::core::ffi::c_int as isize) as *mut str_number
            as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<str_number>() as size_t,
        (font_ptr + 1 as internal_font_number) as size_t,
        fmt_out,
    );
    do_dump(
        font_bc.offset(0 as ::core::ffi::c_int as isize) as *mut UTF16_code
            as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<UTF16_code>() as size_t,
        (font_ptr + 1 as internal_font_number) as size_t,
        fmt_out,
    );
    do_dump(
        font_ec.offset(0 as ::core::ffi::c_int as isize) as *mut UTF16_code
            as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<UTF16_code>() as size_t,
        (font_ptr + 1 as internal_font_number) as size_t,
        fmt_out,
    );
    do_dump(
        char_base.offset(0 as ::core::ffi::c_int as isize) as *mut int32_t
            as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<int32_t>() as size_t,
        (font_ptr + 1 as internal_font_number) as size_t,
        fmt_out,
    );
    do_dump(
        width_base.offset(0 as ::core::ffi::c_int as isize) as *mut int32_t
            as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<int32_t>() as size_t,
        (font_ptr + 1 as internal_font_number) as size_t,
        fmt_out,
    );
    do_dump(
        height_base.offset(0 as ::core::ffi::c_int as isize) as *mut int32_t
            as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<int32_t>() as size_t,
        (font_ptr + 1 as internal_font_number) as size_t,
        fmt_out,
    );
    do_dump(
        depth_base.offset(0 as ::core::ffi::c_int as isize) as *mut int32_t
            as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<int32_t>() as size_t,
        (font_ptr + 1 as internal_font_number) as size_t,
        fmt_out,
    );
    do_dump(
        italic_base.offset(0 as ::core::ffi::c_int as isize) as *mut int32_t
            as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<int32_t>() as size_t,
        (font_ptr + 1 as internal_font_number) as size_t,
        fmt_out,
    );
    do_dump(
        lig_kern_base.offset(0 as ::core::ffi::c_int as isize) as *mut int32_t
            as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<int32_t>() as size_t,
        (font_ptr + 1 as internal_font_number) as size_t,
        fmt_out,
    );
    do_dump(
        kern_base.offset(0 as ::core::ffi::c_int as isize) as *mut int32_t
            as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<int32_t>() as size_t,
        (font_ptr + 1 as internal_font_number) as size_t,
        fmt_out,
    );
    do_dump(
        exten_base.offset(0 as ::core::ffi::c_int as isize) as *mut int32_t
            as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<int32_t>() as size_t,
        (font_ptr + 1 as internal_font_number) as size_t,
        fmt_out,
    );
    do_dump(
        param_base.offset(0 as ::core::ffi::c_int as isize) as *mut int32_t
            as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<int32_t>() as size_t,
        (font_ptr + 1 as internal_font_number) as size_t,
        fmt_out,
    );
    do_dump(
        font_glue.offset(0 as ::core::ffi::c_int as isize) as *mut int32_t
            as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<int32_t>() as size_t,
        (font_ptr + 1 as internal_font_number) as size_t,
        fmt_out,
    );
    do_dump(
        bchar_label.offset(0 as ::core::ffi::c_int as isize) as *mut font_index
            as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<font_index>() as size_t,
        (font_ptr + 1 as internal_font_number) as size_t,
        fmt_out,
    );
    do_dump(
        font_bchar.offset(0 as ::core::ffi::c_int as isize) as *mut nine_bits
            as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<nine_bits>() as size_t,
        (font_ptr + 1 as internal_font_number) as size_t,
        fmt_out,
    );
    do_dump(
        font_false_bchar.offset(0 as ::core::ffi::c_int as isize) as *mut nine_bits
            as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<nine_bits>() as size_t,
        (font_ptr + 1 as internal_font_number) as size_t,
        fmt_out,
    );
    k = FONT_BASE as int32_t;
    while k <= font_ptr {
        print_nl_cstr(b"\\font\0" as *const u8 as *const ::core::ffi::c_char);
        print_esc((*hash.offset((FONT_ID_BASE as int32_t + k) as isize)).s1 as str_number);
        print_char('=' as i32);
        if *font_area.offset(k as isize) as ::core::ffi::c_uint == AAT_FONT_FLAG
            || *font_area.offset(k as isize) as ::core::ffi::c_uint == OTGR_FONT_FLAG
            || !(*font_mapping.offset(k as isize)).is_null()
        {
            print_file_name(
                *font_name.offset(k as isize) as int32_t,
                EMPTY_STRING as int32_t,
                EMPTY_STRING as int32_t,
            );
            error_here_with_diagnostic(
                b"Can't \\dump a format with native fonts or font-mappings\0" as *const u8
                    as *const ::core::ffi::c_char,
            );
            capture_to_diagnostic(::core::ptr::null_mut::<ttbc_diagnostic_t>());
            help_ptr = 3 as ::core::ffi::c_uchar;
            help_line[2 as ::core::ffi::c_int as usize] =
                b"You really, really don't want to do this.\0" as *const u8
                    as *const ::core::ffi::c_char;
            help_line[1 as ::core::ffi::c_int as usize] = b"It won't work, and only confuses me.\0"
                as *const u8
                as *const ::core::ffi::c_char;
            help_line[0 as ::core::ffi::c_int as usize] =
                b"(Load them at runtime, not as part of the format file.)\0" as *const u8
                    as *const ::core::ffi::c_char;
            error();
        } else {
            print_file_name(
                *font_name.offset(k as isize) as int32_t,
                *font_area.offset(k as isize) as int32_t,
                EMPTY_STRING as int32_t,
            );
        }
        if *font_size.offset(k as isize) != *font_dsize.offset(k as isize) {
            print_cstr(b" at \0" as *const u8 as *const ::core::ffi::c_char);
            print_scaled(*font_size.offset(k as isize));
            print_cstr(b"pt\0" as *const u8 as *const ::core::ffi::c_char);
        }
        k += 1;
    }
    print_ln();
    print_int(fmem_ptr as int32_t - 7 as int32_t);
    print_cstr(b" words of font info for \0" as *const u8 as *const ::core::ffi::c_char);
    print_int(font_ptr as int32_t - 0 as int32_t);
    if font_ptr != FONT_BASE as internal_font_number + 1 as internal_font_number {
        print_cstr(b" preloaded fonts\0" as *const u8 as *const ::core::ffi::c_char);
    } else {
        print_cstr(b" preloaded font\0" as *const u8 as *const ::core::ffi::c_char);
    }
    let mut x_val_26: int32_t = hyph_count;
    do_dump(
        &raw mut x_val_26 as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<int32_t>() as size_t,
        1 as ::core::ffi::c_int as size_t,
        fmt_out,
    );
    if hyph_next <= HYPH_PRIME as int32_t {
        hyph_next = hyph_size;
    }
    let mut x_val_27: int32_t = hyph_next;
    do_dump(
        &raw mut x_val_27 as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<int32_t>() as size_t,
        1 as ::core::ffi::c_int as size_t,
        fmt_out,
    );
    k = 0 as ::core::ffi::c_int as int32_t;
    while k <= hyph_size {
        if *hyph_word.offset(k as isize) != 0 as str_number {
            let mut x_val_28: int32_t = (k as ::core::ffi::c_long
                + 65536 as ::core::ffi::c_long
                    * *hyph_link.offset(k as isize) as ::core::ffi::c_long)
                as int32_t;
            do_dump(
                &raw mut x_val_28 as *mut ::core::ffi::c_char,
                ::core::mem::size_of::<int32_t>() as size_t,
                1 as ::core::ffi::c_int as size_t,
                fmt_out,
            );
            let mut x_val_29: int32_t = *hyph_word.offset(k as isize) as int32_t;
            do_dump(
                &raw mut x_val_29 as *mut ::core::ffi::c_char,
                ::core::mem::size_of::<int32_t>() as size_t,
                1 as ::core::ffi::c_int as size_t,
                fmt_out,
            );
            let mut x_val_30: int32_t = *hyph_list.offset(k as isize);
            do_dump(
                &raw mut x_val_30 as *mut ::core::ffi::c_char,
                ::core::mem::size_of::<int32_t>() as size_t,
                1 as ::core::ffi::c_int as size_t,
                fmt_out,
            );
        }
        k += 1;
    }
    print_ln();
    print_int(hyph_count);
    if hyph_count != 1 as int32_t {
        print_cstr(b" hyphenation exceptions\0" as *const u8 as *const ::core::ffi::c_char);
    } else {
        print_cstr(b" hyphenation exception\0" as *const u8 as *const ::core::ffi::c_char);
    }
    if trie_not_ready {
        init_trie();
    }
    let mut x_val_31: int32_t = trie_max as int32_t;
    do_dump(
        &raw mut x_val_31 as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<int32_t>() as size_t,
        1 as ::core::ffi::c_int as size_t,
        fmt_out,
    );
    let mut x_val_32: int32_t = hyph_start as int32_t;
    do_dump(
        &raw mut x_val_32 as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<int32_t>() as size_t,
        1 as ::core::ffi::c_int as size_t,
        fmt_out,
    );
    do_dump(
        trie_trl.offset(0 as ::core::ffi::c_int as isize) as *mut trie_pointer
            as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<trie_pointer>() as size_t,
        (trie_max + 1 as trie_pointer) as size_t,
        fmt_out,
    );
    do_dump(
        trie_tro.offset(0 as ::core::ffi::c_int as isize) as *mut trie_pointer
            as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<trie_pointer>() as size_t,
        (trie_max + 1 as trie_pointer) as size_t,
        fmt_out,
    );
    do_dump(
        trie_trc.offset(0 as ::core::ffi::c_int as isize) as *mut uint16_t
            as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<uint16_t>() as size_t,
        (trie_max + 1 as trie_pointer) as size_t,
        fmt_out,
    );
    let mut x_val_33: int32_t = max_hyph_char;
    do_dump(
        &raw mut x_val_33 as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<int32_t>() as size_t,
        1 as ::core::ffi::c_int as size_t,
        fmt_out,
    );
    let mut x_val_34: int32_t = trie_op_ptr;
    do_dump(
        &raw mut x_val_34 as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<int32_t>() as size_t,
        1 as ::core::ffi::c_int as size_t,
        fmt_out,
    );
    do_dump(
        (&raw mut hyf_distance as *mut small_number).offset(1 as ::core::ffi::c_int as isize)
            as *mut small_number as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<small_number>() as size_t,
        trie_op_ptr as size_t,
        fmt_out,
    );
    do_dump(
        (&raw mut hyf_num as *mut small_number).offset(1 as ::core::ffi::c_int as isize)
            as *mut small_number as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<small_number>() as size_t,
        trie_op_ptr as size_t,
        fmt_out,
    );
    do_dump(
        (&raw mut hyf_next as *mut trie_opcode).offset(1 as ::core::ffi::c_int as isize)
            as *mut trie_opcode as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<trie_opcode>() as size_t,
        trie_op_ptr as size_t,
        fmt_out,
    );
    print_nl_cstr(b"Hyphenation trie of length \0" as *const u8 as *const ::core::ffi::c_char);
    print_int(trie_max as int32_t);
    print_cstr(b" has \0" as *const u8 as *const ::core::ffi::c_char);
    print_int(trie_op_ptr);
    if trie_op_ptr != 1 as int32_t {
        print_cstr(b" ops\0" as *const u8 as *const ::core::ffi::c_char);
    } else {
        print_cstr(b" op\0" as *const u8 as *const ::core::ffi::c_char);
    }
    print_cstr(b" out of \0" as *const u8 as *const ::core::ffi::c_char);
    print_int(TRIE_OP_SIZE as int32_t);
    k = BIGGEST_LANG as int32_t;
    while k >= 0 as int32_t {
        if trie_used[k as usize] as ::core::ffi::c_int > 0 as ::core::ffi::c_int {
            print_nl_cstr(b"  \0" as *const u8 as *const ::core::ffi::c_char);
            print_int(trie_used[k as usize] as int32_t);
            print_cstr(b" for language \0" as *const u8 as *const ::core::ffi::c_char);
            print_int(k);
            let mut x_val_35: int32_t = k;
            do_dump(
                &raw mut x_val_35 as *mut ::core::ffi::c_char,
                ::core::mem::size_of::<int32_t>() as size_t,
                1 as ::core::ffi::c_int as size_t,
                fmt_out,
            );
            let mut x_val_36: int32_t = trie_used[k as usize] as int32_t;
            do_dump(
                &raw mut x_val_36 as *mut ::core::ffi::c_char,
                ::core::mem::size_of::<int32_t>() as size_t,
                1 as ::core::ffi::c_int as size_t,
                fmt_out,
            );
        }
        k -= 1;
    }
    let mut x_val_37: int32_t = 0x29a as int32_t;
    do_dump(
        &raw mut x_val_37 as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<int32_t>() as size_t,
        1 as ::core::ffi::c_int as size_t,
        fmt_out,
    );
    (*eqtb.offset((INT_BASE + INT_PAR__tracing_stats) as isize))
        .b32
        .s1 = 0 as ::core::ffi::c_int as int32_t;
    ttstub_output_close(fmt_out);
}
unsafe extern "C" fn pack_buffered_name(mut n: small_number, mut a: int32_t, mut b: int32_t) {
    free(name_of_file as *mut ::core::ffi::c_void);
    name_of_file = malloc(
        ((format_default_length + 1 as int32_t + 1 as int32_t) as size_t)
            .wrapping_mul(::core::mem::size_of::<UTF8_code>() as size_t),
    ) as *mut ::core::ffi::c_char;
    strcpy(name_of_file, TEX_format_default);
    name_length = strlen(name_of_file) as int32_t;
}
unsafe extern "C" fn load_fmt_file() -> bool {
    let mut current_block: u64;
    let mut j: int32_t = 0;
    let mut k: int32_t = 0;
    let mut p: int32_t = 0;
    let mut q: int32_t = 0;
    let mut x: int32_t = 0;
    let mut fmt_in: rust_input_handle_t = ::core::ptr::null_mut::<ttbc_input_handle_t>();
    j = cur_input.loc;
    pack_buffered_name(
        (format_default_length - 4 as int32_t) as small_number,
        1 as int32_t,
        0 as int32_t,
    );
    fmt_in = ttstub_input_open(
        name_of_file,
        TTBC_FILE_FORMAT_FORMAT,
        0 as ::core::ffi::c_int,
    );
    if fmt_in.is_null() {
        _tt_abort(
            b"cannot open the format file \"%s\"\0" as *const u8 as *const ::core::ffi::c_char,
            name_of_file,
        );
    }
    cur_input.loc = j;
    if in_initex_mode {
        free(font_info as *mut ::core::ffi::c_void);
        free(str_pool as *mut ::core::ffi::c_void);
        free(str_start as *mut ::core::ffi::c_void);
        free(yhash as *mut ::core::ffi::c_void);
        free(eqtb as *mut ::core::ffi::c_void);
        free(mem as *mut ::core::ffi::c_void);
        mem = ::core::ptr::null_mut::<memory_word>();
    }
    do_undump(
        &raw mut x as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<int32_t>() as size_t,
        1 as ::core::ffi::c_int as size_t,
        fmt_in,
    );
    if !(x != FORMAT_HEADER_MAGIC as int32_t) {
        do_undump(
            &raw mut x as *mut ::core::ffi::c_char,
            ::core::mem::size_of::<int32_t>() as size_t,
            1 as ::core::ffi::c_int as size_t,
            fmt_in,
        );
        if x != FORMAT_SERIAL as int32_t {
            _tt_abort(
                b"format file \"%s\" is of the wrong version: expected %d, found %d\0" as *const u8
                    as *const ::core::ffi::c_char,
                name_of_file,
                FORMAT_SERIAL,
                x,
            );
        }
        do_undump(
            &raw mut hash_high as *mut ::core::ffi::c_char,
            ::core::mem::size_of::<int32_t>() as size_t,
            1 as ::core::ffi::c_int as size_t,
            fmt_in,
        );
        if !(hash_high < 0 as int32_t || hash_high as ::core::ffi::c_long > sup_hash_extra) {
            if hash_extra < hash_high {
                hash_extra = hash_high;
            }
            eqtb_top = EQTB_SIZE as int32_t + hash_extra;
            if hash_extra == 0 as int32_t {
                hash_top = UNDEFINED_CONTROL_SEQUENCE as int32_t;
            } else {
                hash_top = eqtb_top;
            }
            yhash = malloc(
                ((1 as int32_t + hash_top - 514 as int32_t + 1 as int32_t) as size_t)
                    .wrapping_mul(::core::mem::size_of::<b32x2>() as size_t),
            ) as *mut b32x2;
            hash = yhash.offset(-(hash_offset as isize));
            (*hash.offset(HASH_BASE as isize)).s0 = 0 as ::core::ffi::c_int as int32_t;
            (*hash.offset(HASH_BASE as isize)).s1 = 0 as ::core::ffi::c_int as int32_t;
            x = (HASH_BASE + 1 as ::core::ffi::c_int) as int32_t;
            while x <= hash_top {
                *hash.offset(x as isize) = *hash.offset(HASH_BASE as isize);
                x += 1;
            }
            eqtb = malloc(
                ((eqtb_top + 1 as int32_t + 1 as int32_t) as size_t)
                    .wrapping_mul(::core::mem::size_of::<memory_word>() as size_t),
            ) as *mut memory_word;
            (*eqtb.offset(UNDEFINED_CONTROL_SEQUENCE as isize)).b16.s1 = UNDEFINED_CS as uint16_t;
            (*eqtb.offset(UNDEFINED_CONTROL_SEQUENCE as isize)).b32.s1 = TEX_NULL as int32_t;
            (*eqtb.offset(UNDEFINED_CONTROL_SEQUENCE as isize)).b16.s0 = LEVEL_ZERO as uint16_t;
            x = (EQTB_SIZE + 1 as ::core::ffi::c_int) as int32_t;
            while x <= eqtb_top {
                *eqtb.offset(x as isize) = *eqtb.offset(UNDEFINED_CONTROL_SEQUENCE as isize);
                x += 1;
            }
            max_reg_num = 32767 as ::core::ffi::c_int as int32_t;
            max_reg_help_line = b"A register number must be between 0 and 32767.\0" as *const u8
                as *const ::core::ffi::c_char;
            do_undump(
                &raw mut x as *mut ::core::ffi::c_char,
                ::core::mem::size_of::<int32_t>() as size_t,
                1 as ::core::ffi::c_int as size_t,
                fmt_in,
            );
            if !(x != MEM_TOP as int32_t) {
                cur_list.head = CONTRIB_HEAD as int32_t;
                cur_list.tail = CONTRIB_HEAD as int32_t;
                page_tail = PAGE_HEAD as int32_t;
                mem = malloc(
                    ((4999999 as ::core::ffi::c_int
                        + 1 as ::core::ffi::c_int
                        + 1 as ::core::ffi::c_int) as size_t)
                        .wrapping_mul(::core::mem::size_of::<memory_word>() as size_t),
                ) as *mut memory_word;
                do_undump(
                    &raw mut x as *mut ::core::ffi::c_char,
                    ::core::mem::size_of::<int32_t>() as size_t,
                    1 as ::core::ffi::c_int as size_t,
                    fmt_in,
                );
                if !(x != EQTB_SIZE as int32_t) {
                    do_undump(
                        &raw mut x as *mut ::core::ffi::c_char,
                        ::core::mem::size_of::<int32_t>() as size_t,
                        1 as ::core::ffi::c_int as size_t,
                        fmt_in,
                    );
                    if !(x != HASH_PRIME as int32_t) {
                        do_undump(
                            &raw mut x as *mut ::core::ffi::c_char,
                            ::core::mem::size_of::<int32_t>() as size_t,
                            1 as ::core::ffi::c_int as size_t,
                            fmt_in,
                        );
                        if !(x != HYPH_PRIME as int32_t) {
                            do_undump(
                                &raw mut x as *mut ::core::ffi::c_char,
                                ::core::mem::size_of::<int32_t>() as size_t,
                                1 as ::core::ffi::c_int as size_t,
                                fmt_in,
                            );
                            if !(x < 0 as int32_t) {
                                if x as ::core::ffi::c_long
                                    > sup_pool_size - pool_free as ::core::ffi::c_long
                                {
                                    _tt_abort(
                                        b"must increase string_pool_size\0" as *const u8
                                            as *const ::core::ffi::c_char,
                                    );
                                }
                                pool_ptr = x as pool_pointer;
                                if pool_size < pool_ptr + pool_free as pool_pointer {
                                    pool_size = (pool_ptr + pool_free as pool_pointer) as int32_t;
                                }
                                do_undump(
                                    &raw mut x as *mut ::core::ffi::c_char,
                                    ::core::mem::size_of::<int32_t>() as size_t,
                                    1 as ::core::ffi::c_int as size_t,
                                    fmt_in,
                                );
                                if !(x < 0 as int32_t) {
                                    if x as ::core::ffi::c_long
                                        > sup_max_strings - strings_free as ::core::ffi::c_long
                                    {
                                        _tt_abort(
                                            b"must increase sup_strings\0" as *const u8
                                                as *const ::core::ffi::c_char,
                                        );
                                    }
                                    str_ptr = x as str_number;
                                    if max_strings < str_ptr + strings_free as str_number {
                                        max_strings =
                                            (str_ptr + strings_free as str_number) as int32_t;
                                    }
                                    str_start = malloc(
                                        ((max_strings + 1 as int32_t) as size_t).wrapping_mul(
                                            ::core::mem::size_of::<pool_pointer>() as size_t,
                                        ),
                                    )
                                        as *mut pool_pointer;
                                    let mut i: ::core::ffi::c_int = 0;
                                    do_undump(
                                        str_start.offset(0 as ::core::ffi::c_int as isize)
                                            as *mut pool_pointer
                                            as *mut ::core::ffi::c_char,
                                        ::core::mem::size_of::<pool_pointer>() as size_t,
                                        (str_ptr - 65536 as str_number + 1 as str_number) as size_t,
                                        fmt_in,
                                    );
                                    i = 0 as ::core::ffi::c_int;
                                    while (i as str_number)
                                        < str_ptr - 65536 as str_number + 1 as str_number
                                    {
                                        if *(str_start.offset(0 as ::core::ffi::c_int as isize)
                                            as *mut pool_pointer)
                                            .offset(i as isize)
                                            < 0 as pool_pointer
                                            || *(str_start.offset(0 as ::core::ffi::c_int as isize)
                                                as *mut pool_pointer)
                                                .offset(i as isize)
                                                > pool_ptr
                                        {
                                            _tt_abort(
                                                b"item %u (=%ld) of .fmt array at %lx <%ld or >%ld\0"
                                                    as *const u8 as *const ::core::ffi::c_char,
                                                i,
                                                *(str_start.offset(0 as ::core::ffi::c_int as isize)
                                                    as *mut pool_pointer)
                                                    .offset(i as isize) as uintptr_t,
                                                str_start.offset(0 as ::core::ffi::c_int as isize)
                                                    as *mut pool_pointer as uintptr_t,
                                                0 as ::core::ffi::c_int as uintptr_t,
                                                pool_ptr as uintptr_t,
                                            );
                                        }
                                        i += 1;
                                    }
                                    str_pool =
                                        malloc(((pool_size + 1 as int32_t) as size_t).wrapping_mul(
                                            ::core::mem::size_of::<packed_UTF16_code>() as size_t,
                                        ))
                                            as *mut packed_UTF16_code;
                                    do_undump(
                                        str_pool.offset(0 as ::core::ffi::c_int as isize)
                                            as *mut packed_UTF16_code
                                            as *mut ::core::ffi::c_char,
                                        ::core::mem::size_of::<packed_UTF16_code>() as size_t,
                                        pool_ptr as size_t,
                                        fmt_in,
                                    );
                                    init_str_ptr = str_ptr;
                                    init_pool_ptr = pool_ptr;
                                    do_undump(
                                        &raw mut x as *mut ::core::ffi::c_char,
                                        ::core::mem::size_of::<int32_t>() as size_t,
                                        1 as ::core::ffi::c_int as size_t,
                                        fmt_in,
                                    );
                                    if !(x < 1019 as int32_t
                                        || x > MEM_TOP as int32_t - HI_MEM_STAT_USAGE as int32_t)
                                    {
                                        lo_mem_max = x;
                                        do_undump(
                                            &raw mut x as *mut ::core::ffi::c_char,
                                            ::core::mem::size_of::<int32_t>() as size_t,
                                            1 as ::core::ffi::c_int as size_t,
                                            fmt_in,
                                        );
                                        if !(x < 20 as int32_t || x > lo_mem_max) {
                                            rover = x;
                                            k = INT_VAL as int32_t;
                                            loop {
                                                if !(k <= INTER_CHAR_VAL as int32_t) {
                                                    current_block = 13910774313357589740;
                                                    break;
                                                }
                                                do_undump(
                                                    &raw mut x as *mut ::core::ffi::c_char,
                                                    ::core::mem::size_of::<int32_t>() as size_t,
                                                    1 as ::core::ffi::c_int as size_t,
                                                    fmt_in,
                                                );
                                                if x < MIN_HALFWORD as int32_t || x > lo_mem_max {
                                                    current_block = 10982004377386733518;
                                                    break;
                                                }
                                                sa_root[k as usize] = x;
                                                k += 1;
                                            }
                                            match current_block {
                                                10982004377386733518 => {}
                                                _ => {
                                                    p = 0 as ::core::ffi::c_int as int32_t;
                                                    q = rover;
                                                    loop {
                                                        do_undump(
                                                            mem.offset(p as isize)
                                                                as *mut memory_word
                                                                as *mut ::core::ffi::c_char,
                                                            ::core::mem::size_of::<memory_word>()
                                                                as size_t,
                                                            (q + 2 as int32_t - p) as size_t,
                                                            fmt_in,
                                                        );
                                                        p = q + (*mem.offset(q as isize)).b32.s0;
                                                        if p > lo_mem_max
                                                            || q >= (*mem.offset(
                                                                (q + 1 as int32_t) as isize,
                                                            ))
                                                            .b32
                                                            .s1 && (*mem.offset(
                                                                (q + 1 as int32_t) as isize,
                                                            ))
                                                            .b32
                                                            .s1 != rover
                                                        {
                                                            current_block = 10982004377386733518;
                                                            break;
                                                        }
                                                        q = (*mem
                                                            .offset((q + 1 as int32_t) as isize))
                                                        .b32
                                                        .s1;
                                                        if !(q != rover) {
                                                            current_block = 479107131381816815;
                                                            break;
                                                        }
                                                    }
                                                    match current_block {
                                                        10982004377386733518 => {}
                                                        _ => {
                                                            do_undump(
                                                                mem.offset(p as isize)
                                                                    as *mut memory_word
                                                                    as *mut ::core::ffi::c_char,
                                                                ::core::mem::size_of::<memory_word>(
                                                                )
                                                                    as size_t,
                                                                (lo_mem_max + 1 as int32_t - p)
                                                                    as size_t,
                                                                fmt_in,
                                                            );
                                                            do_undump(
                                                                &raw mut x
                                                                    as *mut ::core::ffi::c_char,
                                                                ::core::mem::size_of::<int32_t>()
                                                                    as size_t,
                                                                1 as ::core::ffi::c_int as size_t,
                                                                fmt_in,
                                                            );
                                                            if !(x < lo_mem_max + 1 as int32_t
                                                                || x > PRE_ADJUST_HEAD as int32_t)
                                                            {
                                                                hi_mem_min = x;
                                                                do_undump(
                                                                    &raw mut x
                                                                        as *mut ::core::ffi::c_char,
                                                                    ::core::mem::size_of::<int32_t>(
                                                                    )
                                                                        as size_t,
                                                                    1 as ::core::ffi::c_int
                                                                        as size_t,
                                                                    fmt_in,
                                                                );
                                                                if !(x < MIN_HALFWORD as int32_t
                                                                    || x > MEM_TOP as int32_t)
                                                                {
                                                                    avail = x;
                                                                    mem_end = MEM_TOP as int32_t;
                                                                    do_undump(
                                                                        mem.offset(hi_mem_min as isize) as *mut memory_word
                                                                            as *mut ::core::ffi::c_char,
                                                                        ::core::mem::size_of::<memory_word>() as size_t,
                                                                        (mem_end + 1 as int32_t - hi_mem_min) as size_t,
                                                                        fmt_in,
                                                                    );
                                                                    do_undump(
                                                                        &raw mut var_used as *mut ::core::ffi::c_char,
                                                                        ::core::mem::size_of::<int32_t>() as size_t,
                                                                        1 as ::core::ffi::c_int as size_t,
                                                                        fmt_in,
                                                                    );
                                                                    do_undump(
                                                                        &raw mut dyn_used as *mut ::core::ffi::c_char,
                                                                        ::core::mem::size_of::<int32_t>() as size_t,
                                                                        1 as ::core::ffi::c_int as size_t,
                                                                        fmt_in,
                                                                    );
                                                                    k = ACTIVE_BASE as int32_t;
                                                                    loop {
                                                                        do_undump(
                                                                            &raw mut x as *mut ::core::ffi::c_char,
                                                                            ::core::mem::size_of::<int32_t>() as size_t,
                                                                            1 as ::core::ffi::c_int as size_t,
                                                                            fmt_in,
                                                                        );
                                                                        if x < 1 as int32_t
                                                                            || k + x
                                                                                > EQTB_SIZE
                                                                                    as int32_t
                                                                                    + 1 as int32_t
                                                                        {
                                                                            current_block = 10982004377386733518;
                                                                            break;
                                                                        }
                                                                        do_undump(
                                                                            eqtb.offset(k as isize) as *mut memory_word
                                                                                as *mut ::core::ffi::c_char,
                                                                            ::core::mem::size_of::<memory_word>() as size_t,
                                                                            x as size_t,
                                                                            fmt_in,
                                                                        );
                                                                        k = k + x;
                                                                        do_undump(
                                                                            &raw mut x as *mut ::core::ffi::c_char,
                                                                            ::core::mem::size_of::<int32_t>() as size_t,
                                                                            1 as ::core::ffi::c_int as size_t,
                                                                            fmt_in,
                                                                        );
                                                                        if x < 0 as int32_t
                                                                            || k + x
                                                                                > EQTB_SIZE
                                                                                    as int32_t
                                                                                    + 1 as int32_t
                                                                        {
                                                                            current_block = 10982004377386733518;
                                                                            break;
                                                                        }
                                                                        j = k;
                                                                        while j
                                                                            <= k + x - 1 as int32_t
                                                                        {
                                                                            *eqtb.offset(
                                                                                j as isize,
                                                                            ) = *eqtb.offset(
                                                                                (k - 1 as int32_t)
                                                                                    as isize,
                                                                            );
                                                                            j += 1;
                                                                        }
                                                                        k = k + x;
                                                                        if !(k
                                                                            <= EQTB_SIZE as int32_t)
                                                                        {
                                                                            current_block = 18425699056680496821;
                                                                            break;
                                                                        }
                                                                    }
                                                                    match current_block {
                                                                        10982004377386733518 => {}
                                                                        _ => {
                                                                            if hash_high
                                                                                > 0 as int32_t
                                                                            {
                                                                                do_undump(
                                                                                    eqtb
                                                                                        .offset(
                                                                                            (7826729 as ::core::ffi::c_int + INT_PARS as ::core::ffi::c_int
                                                                                                + 256 as ::core::ffi::c_int
                                                                                                + (0x10ffff as ::core::ffi::c_int + 1 as ::core::ffi::c_int)
                                                                                                + 23 as ::core::ffi::c_int + 255 as ::core::ffi::c_int
                                                                                                + 1 as ::core::ffi::c_int) as isize,
                                                                                        ) as *mut memory_word as *mut ::core::ffi::c_char,
                                                                                    ::core::mem::size_of::<memory_word>() as size_t,
                                                                                    hash_high as size_t,
                                                                                    fmt_in,
                                                                                );
                                                                            }
                                                                            do_undump(
                                                                                &raw mut x as *mut ::core::ffi::c_char,
                                                                                ::core::mem::size_of::<int32_t>() as size_t,
                                                                                1 as ::core::ffi::c_int as size_t,
                                                                                fmt_in,
                                                                            );
                                                                            if !(x <= 0 as int32_t
                                                                                || x > hash_top)
                                                                            {
                                                                                par_loc = x;
                                                                                par_token =
                                                                                    CS_TOKEN_FLAG
                                                                                        as int32_t
                                                                                        + par_loc;
                                                                                do_undump(
                                                                                    &raw mut x as *mut ::core::ffi::c_char,
                                                                                    ::core::mem::size_of::<int32_t>() as size_t,
                                                                                    1 as ::core::ffi::c_int as size_t,
                                                                                    fmt_in,
                                                                                );
                                                                                if !(x < HASH_BASE
                                                                                    as int32_t
                                                                                    || x > hash_top)
                                                                                {
                                                                                    write_loc = x;
                                                                                    p = 0 as ::core::ffi::c_int as int32_t;
                                                                                    while p <= PRIM_SIZE as int32_t {
                                                                                        do_undump(
                                                                                            (&raw mut prim as *mut b32x2).offset(p as isize)
                                                                                                as *mut b32x2 as *mut ::core::ffi::c_char,
                                                                                            ::core::mem::size_of::<b32x2>() as size_t,
                                                                                            1 as ::core::ffi::c_int as size_t,
                                                                                            fmt_in,
                                                                                        );
                                                                                        p += 1;
                                                                                    }
                                                                                    do_undump(
                                                                                        &raw mut x as *mut ::core::ffi::c_char,
                                                                                        ::core::mem::size_of::<int32_t>() as size_t,
                                                                                        1 as ::core::ffi::c_int as size_t,
                                                                                        fmt_in,
                                                                                    );
                                                                                    if !(x < HASH_BASE as int32_t
                                                                                        || x > FROZEN_CONTROL_SEQUENCE as int32_t)
                                                                                    {
                                                                                        hash_used = x;
                                                                                        p = (HASH_BASE - 1 as ::core::ffi::c_int) as int32_t;
                                                                                        loop {
                                                                                            do_undump(
                                                                                                &raw mut x as *mut ::core::ffi::c_char,
                                                                                                ::core::mem::size_of::<int32_t>() as size_t,
                                                                                                1 as ::core::ffi::c_int as size_t,
                                                                                                fmt_in,
                                                                                            );
                                                                                            if x < p + 1 as int32_t || x > hash_used {
                                                                                                current_block = 10982004377386733518;
                                                                                                break;
                                                                                            }
                                                                                            p = x;
                                                                                            do_undump(
                                                                                                hash.offset(p as isize) as *mut b32x2
                                                                                                    as *mut ::core::ffi::c_char,
                                                                                                ::core::mem::size_of::<b32x2>() as size_t,
                                                                                                1 as ::core::ffi::c_int as size_t,
                                                                                                fmt_in,
                                                                                            );
                                                                                            if !(p != hash_used) {
                                                                                                current_block = 16972322153429435017;
                                                                                                break;
                                                                                            }
                                                                                        }
                                                                                        match current_block {
                                                                                            10982004377386733518 => {}
                                                                                            _ => {
                                                                                                do_undump(
                                                                                                    hash.offset((hash_used + 1 as int32_t) as isize)
                                                                                                        as *mut b32x2 as *mut ::core::ffi::c_char,
                                                                                                    ::core::mem::size_of::<b32x2>() as size_t,
                                                                                                    (2254339 as int32_t - 1 as int32_t - hash_used) as size_t,
                                                                                                    fmt_in,
                                                                                                );
                                                                                                if hash_high > 0 as int32_t {
                                                                                                    do_undump(
                                                                                                        hash
                                                                                                            .offset(
                                                                                                                (7826729 as ::core::ffi::c_int + INT_PARS as ::core::ffi::c_int
                                                                                                                    + 256 as ::core::ffi::c_int
                                                                                                                    + (0x10ffff as ::core::ffi::c_int + 1 as ::core::ffi::c_int)
                                                                                                                    + 23 as ::core::ffi::c_int + 255 as ::core::ffi::c_int
                                                                                                                    + 1 as ::core::ffi::c_int) as isize,
                                                                                                            ) as *mut b32x2 as *mut ::core::ffi::c_char,
                                                                                                        ::core::mem::size_of::<b32x2>() as size_t,
                                                                                                        hash_high as size_t,
                                                                                                        fmt_in,
                                                                                                    );
                                                                                                }
                                                                                                do_undump(
                                                                                                    &raw mut cs_count as *mut ::core::ffi::c_char,
                                                                                                    ::core::mem::size_of::<int32_t>() as size_t,
                                                                                                    1 as ::core::ffi::c_int as size_t,
                                                                                                    fmt_in,
                                                                                                );
                                                                                                do_undump(
                                                                                                    &raw mut x as *mut ::core::ffi::c_char,
                                                                                                    ::core::mem::size_of::<int32_t>() as size_t,
                                                                                                    1 as ::core::ffi::c_int as size_t,
                                                                                                    fmt_in,
                                                                                                );
                                                                                                if !(x < 7 as int32_t) {
                                                                                                    if x as ::core::ffi::c_long > sup_font_mem_size {
                                                                                                        _tt_abort(
                                                                                                            b"must increase font_mem_size\0" as *const u8
                                                                                                                as *const ::core::ffi::c_char,
                                                                                                        );
                                                                                                    }
                                                                                                    fmem_ptr = x as font_index;
                                                                                                    if fmem_ptr > font_mem_size {
                                                                                                        font_mem_size = fmem_ptr as int32_t;
                                                                                                    }
                                                                                                    font_info = malloc(
                                                                                                        ((font_mem_size + 1 as int32_t) as size_t)
                                                                                                            .wrapping_mul(
                                                                                                                ::core::mem::size_of::<memory_word>() as size_t,
                                                                                                            ),
                                                                                                    ) as *mut memory_word;
                                                                                                    do_undump(
                                                                                                        font_info.offset(0 as ::core::ffi::c_int as isize)
                                                                                                            as *mut memory_word as *mut ::core::ffi::c_char,
                                                                                                        ::core::mem::size_of::<memory_word>() as size_t,
                                                                                                        fmem_ptr as size_t,
                                                                                                        fmt_in,
                                                                                                    );
                                                                                                    do_undump(
                                                                                                        &raw mut x as *mut ::core::ffi::c_char,
                                                                                                        ::core::mem::size_of::<int32_t>() as size_t,
                                                                                                        1 as ::core::ffi::c_int as size_t,
                                                                                                        fmt_in,
                                                                                                    );
                                                                                                    if !(x < FONT_BASE as int32_t) {
                                                                                                        if x > FONT_BASE as int32_t + MAX_FONT_MAX as int32_t {
                                                                                                            _tt_abort(
                                                                                                                b"must increase font_max\0" as *const u8
                                                                                                                    as *const ::core::ffi::c_char,
                                                                                                            );
                                                                                                        }
                                                                                                        font_ptr = x as internal_font_number;
                                                                                                        font_mapping = calloc(
                                                                                                            (font_max + 1 as int32_t) as size_t,
                                                                                                            ::core::mem::size_of::<*mut ::core::ffi::c_void>() as size_t,
                                                                                                        ) as *mut *mut ::core::ffi::c_void;
                                                                                                        font_layout_engine = calloc(
                                                                                                            (font_max + 1 as int32_t) as size_t,
                                                                                                            ::core::mem::size_of::<*mut ::core::ffi::c_void>() as size_t,
                                                                                                        ) as *mut *mut ::core::ffi::c_void;
                                                                                                        font_flags = malloc(
                                                                                                            ((font_max + 1 as int32_t) as size_t)
                                                                                                                .wrapping_mul(
                                                                                                                    ::core::mem::size_of::<::core::ffi::c_char>() as size_t,
                                                                                                                ),
                                                                                                        ) as *mut ::core::ffi::c_char;
                                                                                                        font_letter_space = malloc(
                                                                                                            ((font_max + 1 as int32_t) as size_t)
                                                                                                                .wrapping_mul(::core::mem::size_of::<scaled_t>() as size_t),
                                                                                                        ) as *mut scaled_t;
                                                                                                        font_check = malloc(
                                                                                                            ((font_max + 1 as int32_t) as size_t)
                                                                                                                .wrapping_mul(::core::mem::size_of::<b16x4>() as size_t),
                                                                                                        ) as *mut b16x4;
                                                                                                        font_size = malloc(
                                                                                                            ((font_max + 1 as int32_t) as size_t)
                                                                                                                .wrapping_mul(::core::mem::size_of::<scaled_t>() as size_t),
                                                                                                        ) as *mut scaled_t;
                                                                                                        font_dsize = malloc(
                                                                                                            ((font_max + 1 as int32_t) as size_t)
                                                                                                                .wrapping_mul(::core::mem::size_of::<scaled_t>() as size_t),
                                                                                                        ) as *mut scaled_t;
                                                                                                        font_params = malloc(
                                                                                                            ((font_max + 1 as int32_t) as size_t)
                                                                                                                .wrapping_mul(
                                                                                                                    ::core::mem::size_of::<font_index>() as size_t,
                                                                                                                ),
                                                                                                        ) as *mut font_index;
                                                                                                        font_name = malloc(
                                                                                                            ((font_max + 1 as int32_t) as size_t)
                                                                                                                .wrapping_mul(
                                                                                                                    ::core::mem::size_of::<str_number>() as size_t,
                                                                                                                ),
                                                                                                        ) as *mut str_number;
                                                                                                        font_area = malloc(
                                                                                                            ((font_max + 1 as int32_t) as size_t)
                                                                                                                .wrapping_mul(
                                                                                                                    ::core::mem::size_of::<str_number>() as size_t,
                                                                                                                ),
                                                                                                        ) as *mut str_number;
                                                                                                        font_bc = malloc(
                                                                                                            ((font_max + 1 as int32_t) as size_t)
                                                                                                                .wrapping_mul(
                                                                                                                    ::core::mem::size_of::<UTF16_code>() as size_t,
                                                                                                                ),
                                                                                                        ) as *mut UTF16_code;
                                                                                                        font_ec = malloc(
                                                                                                            ((font_max + 1 as int32_t) as size_t)
                                                                                                                .wrapping_mul(
                                                                                                                    ::core::mem::size_of::<UTF16_code>() as size_t,
                                                                                                                ),
                                                                                                        ) as *mut UTF16_code;
                                                                                                        font_glue = malloc(
                                                                                                            ((font_max + 1 as int32_t) as size_t)
                                                                                                                .wrapping_mul(::core::mem::size_of::<int32_t>() as size_t),
                                                                                                        ) as *mut int32_t;
                                                                                                        hyphen_char = malloc(
                                                                                                            ((font_max + 1 as int32_t) as size_t)
                                                                                                                .wrapping_mul(::core::mem::size_of::<int32_t>() as size_t),
                                                                                                        ) as *mut int32_t;
                                                                                                        skew_char = malloc(
                                                                                                            ((font_max + 1 as int32_t) as size_t)
                                                                                                                .wrapping_mul(::core::mem::size_of::<int32_t>() as size_t),
                                                                                                        ) as *mut int32_t;
                                                                                                        bchar_label = malloc(
                                                                                                            ((font_max + 1 as int32_t) as size_t)
                                                                                                                .wrapping_mul(
                                                                                                                    ::core::mem::size_of::<font_index>() as size_t,
                                                                                                                ),
                                                                                                        ) as *mut font_index;
                                                                                                        font_bchar = malloc(
                                                                                                            ((font_max + 1 as int32_t) as size_t)
                                                                                                                .wrapping_mul(::core::mem::size_of::<nine_bits>() as size_t),
                                                                                                        ) as *mut nine_bits;
                                                                                                        font_false_bchar = malloc(
                                                                                                            ((font_max + 1 as int32_t) as size_t)
                                                                                                                .wrapping_mul(::core::mem::size_of::<nine_bits>() as size_t),
                                                                                                        ) as *mut nine_bits;
                                                                                                        char_base = malloc(
                                                                                                            ((font_max + 1 as int32_t) as size_t)
                                                                                                                .wrapping_mul(::core::mem::size_of::<int32_t>() as size_t),
                                                                                                        ) as *mut int32_t;
                                                                                                        width_base = malloc(
                                                                                                            ((font_max + 1 as int32_t) as size_t)
                                                                                                                .wrapping_mul(::core::mem::size_of::<int32_t>() as size_t),
                                                                                                        ) as *mut int32_t;
                                                                                                        height_base = malloc(
                                                                                                            ((font_max + 1 as int32_t) as size_t)
                                                                                                                .wrapping_mul(::core::mem::size_of::<int32_t>() as size_t),
                                                                                                        ) as *mut int32_t;
                                                                                                        depth_base = malloc(
                                                                                                            ((font_max + 1 as int32_t) as size_t)
                                                                                                                .wrapping_mul(::core::mem::size_of::<int32_t>() as size_t),
                                                                                                        ) as *mut int32_t;
                                                                                                        italic_base = malloc(
                                                                                                            ((font_max + 1 as int32_t) as size_t)
                                                                                                                .wrapping_mul(::core::mem::size_of::<int32_t>() as size_t),
                                                                                                        ) as *mut int32_t;
                                                                                                        lig_kern_base = malloc(
                                                                                                            ((font_max + 1 as int32_t) as size_t)
                                                                                                                .wrapping_mul(::core::mem::size_of::<int32_t>() as size_t),
                                                                                                        ) as *mut int32_t;
                                                                                                        kern_base = malloc(
                                                                                                            ((font_max + 1 as int32_t) as size_t)
                                                                                                                .wrapping_mul(::core::mem::size_of::<int32_t>() as size_t),
                                                                                                        ) as *mut int32_t;
                                                                                                        exten_base = malloc(
                                                                                                            ((font_max + 1 as int32_t) as size_t)
                                                                                                                .wrapping_mul(::core::mem::size_of::<int32_t>() as size_t),
                                                                                                        ) as *mut int32_t;
                                                                                                        param_base = malloc(
                                                                                                            ((font_max + 1 as int32_t) as size_t)
                                                                                                                .wrapping_mul(::core::mem::size_of::<int32_t>() as size_t),
                                                                                                        ) as *mut int32_t;
                                                                                                        k = FONT_BASE as int32_t;
                                                                                                        while k <= font_ptr {
                                                                                                            let ref mut fresh17 = *font_mapping.offset(k as isize);
                                                                                                            *fresh17 = ::core::ptr::null_mut::<::core::ffi::c_void>();
                                                                                                            k += 1;
                                                                                                        }
                                                                                                        do_undump(
                                                                                                            font_check.offset(0 as ::core::ffi::c_int as isize)
                                                                                                                as *mut b16x4 as *mut ::core::ffi::c_char,
                                                                                                            ::core::mem::size_of::<b16x4>() as size_t,
                                                                                                            (font_ptr + 1 as internal_font_number) as size_t,
                                                                                                            fmt_in,
                                                                                                        );
                                                                                                        do_undump(
                                                                                                            font_size.offset(0 as ::core::ffi::c_int as isize)
                                                                                                                as *mut scaled_t as *mut ::core::ffi::c_char,
                                                                                                            ::core::mem::size_of::<scaled_t>() as size_t,
                                                                                                            (font_ptr + 1 as internal_font_number) as size_t,
                                                                                                            fmt_in,
                                                                                                        );
                                                                                                        do_undump(
                                                                                                            font_dsize.offset(0 as ::core::ffi::c_int as isize)
                                                                                                                as *mut scaled_t as *mut ::core::ffi::c_char,
                                                                                                            ::core::mem::size_of::<scaled_t>() as size_t,
                                                                                                            (font_ptr + 1 as internal_font_number) as size_t,
                                                                                                            fmt_in,
                                                                                                        );
                                                                                                        let mut i_0: ::core::ffi::c_int = 0;
                                                                                                        do_undump(
                                                                                                            font_params.offset(0 as ::core::ffi::c_int as isize)
                                                                                                                as *mut font_index as *mut ::core::ffi::c_char,
                                                                                                            ::core::mem::size_of::<font_index>() as size_t,
                                                                                                            (font_ptr + 1 as internal_font_number) as size_t,
                                                                                                            fmt_in,
                                                                                                        );
                                                                                                        i_0 = 0 as ::core::ffi::c_int;
                                                                                                        while (i_0 as internal_font_number)
                                                                                                            < font_ptr + 1 as internal_font_number
                                                                                                        {
                                                                                                            if *(font_params.offset(0 as ::core::ffi::c_int as isize)
                                                                                                                as *mut font_index)
                                                                                                                .offset(i_0 as isize) < -(0xfffffff as font_index)
                                                                                                                || *(font_params.offset(0 as ::core::ffi::c_int as isize)
                                                                                                                    as *mut font_index)
                                                                                                                    .offset(i_0 as isize) > 0x3fffffff as font_index
                                                                                                            {
                                                                                                                _tt_abort(
                                                                                                                    b"item %u (=%ld) of .fmt array at %lx <%ld or >%ld\0"
                                                                                                                        as *const u8 as *const ::core::ffi::c_char,
                                                                                                                    i_0,
                                                                                                                    *(font_params.offset(0 as ::core::ffi::c_int as isize)
                                                                                                                        as *mut font_index)
                                                                                                                        .offset(i_0 as isize) as uintptr_t,
                                                                                                                    font_params.offset(0 as ::core::ffi::c_int as isize)
                                                                                                                        as *mut font_index as uintptr_t,
                                                                                                                    -(0xfffffff as ::core::ffi::c_int) as uintptr_t,
                                                                                                                    0x3fffffff as ::core::ffi::c_int as uintptr_t,
                                                                                                                );
                                                                                                            }
                                                                                                            i_0 += 1;
                                                                                                        }
                                                                                                        do_undump(
                                                                                                            hyphen_char.offset(0 as ::core::ffi::c_int as isize)
                                                                                                                as *mut int32_t as *mut ::core::ffi::c_char,
                                                                                                            ::core::mem::size_of::<int32_t>() as size_t,
                                                                                                            (font_ptr + 1 as internal_font_number) as size_t,
                                                                                                            fmt_in,
                                                                                                        );
                                                                                                        do_undump(
                                                                                                            skew_char.offset(0 as ::core::ffi::c_int as isize)
                                                                                                                as *mut int32_t as *mut ::core::ffi::c_char,
                                                                                                            ::core::mem::size_of::<int32_t>() as size_t,
                                                                                                            (font_ptr + 1 as internal_font_number) as size_t,
                                                                                                            fmt_in,
                                                                                                        );
                                                                                                        let mut i_1: ::core::ffi::c_int = 0;
                                                                                                        do_undump(
                                                                                                            font_name.offset(0 as ::core::ffi::c_int as isize)
                                                                                                                as *mut str_number as *mut ::core::ffi::c_char,
                                                                                                            ::core::mem::size_of::<str_number>() as size_t,
                                                                                                            (font_ptr + 1 as internal_font_number) as size_t,
                                                                                                            fmt_in,
                                                                                                        );
                                                                                                        i_1 = 0 as ::core::ffi::c_int;
                                                                                                        while (i_1 as internal_font_number)
                                                                                                            < font_ptr + 1 as internal_font_number
                                                                                                        {
                                                                                                            if *(font_name.offset(0 as ::core::ffi::c_int as isize)
                                                                                                                as *mut str_number)
                                                                                                                .offset(i_1 as isize) > str_ptr
                                                                                                            {
                                                                                                                _tt_abort(
                                                                                                                    b"Item %u (=%ld) of .fmt array at %lx >%ld\0" as *const u8
                                                                                                                        as *const ::core::ffi::c_char,
                                                                                                                    i_1,
                                                                                                                    *(font_name.offset(0 as ::core::ffi::c_int as isize)
                                                                                                                        as *mut str_number)
                                                                                                                        .offset(i_1 as isize) as uintptr_t,
                                                                                                                    font_name.offset(0 as ::core::ffi::c_int as isize)
                                                                                                                        as *mut str_number as uintptr_t,
                                                                                                                    str_ptr as uintptr_t,
                                                                                                                );
                                                                                                            }
                                                                                                            i_1 += 1;
                                                                                                        }
                                                                                                        let mut i_2: ::core::ffi::c_int = 0;
                                                                                                        do_undump(
                                                                                                            font_area.offset(0 as ::core::ffi::c_int as isize)
                                                                                                                as *mut str_number as *mut ::core::ffi::c_char,
                                                                                                            ::core::mem::size_of::<str_number>() as size_t,
                                                                                                            (font_ptr + 1 as internal_font_number) as size_t,
                                                                                                            fmt_in,
                                                                                                        );
                                                                                                        i_2 = 0 as ::core::ffi::c_int;
                                                                                                        while (i_2 as internal_font_number)
                                                                                                            < font_ptr + 1 as internal_font_number
                                                                                                        {
                                                                                                            if *(font_area.offset(0 as ::core::ffi::c_int as isize)
                                                                                                                as *mut str_number)
                                                                                                                .offset(i_2 as isize) > str_ptr
                                                                                                            {
                                                                                                                _tt_abort(
                                                                                                                    b"Item %u (=%ld) of .fmt array at %lx >%ld\0" as *const u8
                                                                                                                        as *const ::core::ffi::c_char,
                                                                                                                    i_2,
                                                                                                                    *(font_area.offset(0 as ::core::ffi::c_int as isize)
                                                                                                                        as *mut str_number)
                                                                                                                        .offset(i_2 as isize) as uintptr_t,
                                                                                                                    font_area.offset(0 as ::core::ffi::c_int as isize)
                                                                                                                        as *mut str_number as uintptr_t,
                                                                                                                    str_ptr as uintptr_t,
                                                                                                                );
                                                                                                            }
                                                                                                            i_2 += 1;
                                                                                                        }
                                                                                                        do_undump(
                                                                                                            font_bc.offset(0 as ::core::ffi::c_int as isize)
                                                                                                                as *mut UTF16_code as *mut ::core::ffi::c_char,
                                                                                                            ::core::mem::size_of::<UTF16_code>() as size_t,
                                                                                                            (font_ptr + 1 as internal_font_number) as size_t,
                                                                                                            fmt_in,
                                                                                                        );
                                                                                                        do_undump(
                                                                                                            font_ec.offset(0 as ::core::ffi::c_int as isize)
                                                                                                                as *mut UTF16_code as *mut ::core::ffi::c_char,
                                                                                                            ::core::mem::size_of::<UTF16_code>() as size_t,
                                                                                                            (font_ptr + 1 as internal_font_number) as size_t,
                                                                                                            fmt_in,
                                                                                                        );
                                                                                                        do_undump(
                                                                                                            char_base.offset(0 as ::core::ffi::c_int as isize)
                                                                                                                as *mut int32_t as *mut ::core::ffi::c_char,
                                                                                                            ::core::mem::size_of::<int32_t>() as size_t,
                                                                                                            (font_ptr + 1 as internal_font_number) as size_t,
                                                                                                            fmt_in,
                                                                                                        );
                                                                                                        do_undump(
                                                                                                            width_base.offset(0 as ::core::ffi::c_int as isize)
                                                                                                                as *mut int32_t as *mut ::core::ffi::c_char,
                                                                                                            ::core::mem::size_of::<int32_t>() as size_t,
                                                                                                            (font_ptr + 1 as internal_font_number) as size_t,
                                                                                                            fmt_in,
                                                                                                        );
                                                                                                        do_undump(
                                                                                                            height_base.offset(0 as ::core::ffi::c_int as isize)
                                                                                                                as *mut int32_t as *mut ::core::ffi::c_char,
                                                                                                            ::core::mem::size_of::<int32_t>() as size_t,
                                                                                                            (font_ptr + 1 as internal_font_number) as size_t,
                                                                                                            fmt_in,
                                                                                                        );
                                                                                                        do_undump(
                                                                                                            depth_base.offset(0 as ::core::ffi::c_int as isize)
                                                                                                                as *mut int32_t as *mut ::core::ffi::c_char,
                                                                                                            ::core::mem::size_of::<int32_t>() as size_t,
                                                                                                            (font_ptr + 1 as internal_font_number) as size_t,
                                                                                                            fmt_in,
                                                                                                        );
                                                                                                        do_undump(
                                                                                                            italic_base.offset(0 as ::core::ffi::c_int as isize)
                                                                                                                as *mut int32_t as *mut ::core::ffi::c_char,
                                                                                                            ::core::mem::size_of::<int32_t>() as size_t,
                                                                                                            (font_ptr + 1 as internal_font_number) as size_t,
                                                                                                            fmt_in,
                                                                                                        );
                                                                                                        do_undump(
                                                                                                            lig_kern_base.offset(0 as ::core::ffi::c_int as isize)
                                                                                                                as *mut int32_t as *mut ::core::ffi::c_char,
                                                                                                            ::core::mem::size_of::<int32_t>() as size_t,
                                                                                                            (font_ptr + 1 as internal_font_number) as size_t,
                                                                                                            fmt_in,
                                                                                                        );
                                                                                                        do_undump(
                                                                                                            kern_base.offset(0 as ::core::ffi::c_int as isize)
                                                                                                                as *mut int32_t as *mut ::core::ffi::c_char,
                                                                                                            ::core::mem::size_of::<int32_t>() as size_t,
                                                                                                            (font_ptr + 1 as internal_font_number) as size_t,
                                                                                                            fmt_in,
                                                                                                        );
                                                                                                        do_undump(
                                                                                                            exten_base.offset(0 as ::core::ffi::c_int as isize)
                                                                                                                as *mut int32_t as *mut ::core::ffi::c_char,
                                                                                                            ::core::mem::size_of::<int32_t>() as size_t,
                                                                                                            (font_ptr + 1 as internal_font_number) as size_t,
                                                                                                            fmt_in,
                                                                                                        );
                                                                                                        do_undump(
                                                                                                            param_base.offset(0 as ::core::ffi::c_int as isize)
                                                                                                                as *mut int32_t as *mut ::core::ffi::c_char,
                                                                                                            ::core::mem::size_of::<int32_t>() as size_t,
                                                                                                            (font_ptr + 1 as internal_font_number) as size_t,
                                                                                                            fmt_in,
                                                                                                        );
                                                                                                        let mut i_3: ::core::ffi::c_int = 0;
                                                                                                        do_undump(
                                                                                                            font_glue.offset(0 as ::core::ffi::c_int as isize)
                                                                                                                as *mut int32_t as *mut ::core::ffi::c_char,
                                                                                                            ::core::mem::size_of::<int32_t>() as size_t,
                                                                                                            (font_ptr + 1 as internal_font_number) as size_t,
                                                                                                            fmt_in,
                                                                                                        );
                                                                                                        i_3 = 0 as ::core::ffi::c_int;
                                                                                                        while (i_3 as internal_font_number)
                                                                                                            < font_ptr + 1 as internal_font_number
                                                                                                        {
                                                                                                            if *(font_glue.offset(0 as ::core::ffi::c_int as isize)
                                                                                                                as *mut int32_t)
                                                                                                                .offset(i_3 as isize) < -(0xfffffff as int32_t)
                                                                                                                || *(font_glue.offset(0 as ::core::ffi::c_int as isize)
                                                                                                                    as *mut int32_t)
                                                                                                                    .offset(i_3 as isize) > lo_mem_max
                                                                                                            {
                                                                                                                _tt_abort(
                                                                                                                    b"item %u (=%ld) of .fmt array at %lx <%ld or >%ld\0"
                                                                                                                        as *const u8 as *const ::core::ffi::c_char,
                                                                                                                    i_3,
                                                                                                                    *(font_glue.offset(0 as ::core::ffi::c_int as isize)
                                                                                                                        as *mut int32_t)
                                                                                                                        .offset(i_3 as isize) as uintptr_t,
                                                                                                                    font_glue.offset(0 as ::core::ffi::c_int as isize)
                                                                                                                        as *mut int32_t as uintptr_t,
                                                                                                                    -(0xfffffff as ::core::ffi::c_int) as uintptr_t,
                                                                                                                    lo_mem_max as uintptr_t,
                                                                                                                );
                                                                                                            }
                                                                                                            i_3 += 1;
                                                                                                        }
                                                                                                        let mut i_4: ::core::ffi::c_int = 0;
                                                                                                        do_undump(
                                                                                                            bchar_label.offset(0 as ::core::ffi::c_int as isize)
                                                                                                                as *mut font_index as *mut ::core::ffi::c_char,
                                                                                                            ::core::mem::size_of::<font_index>() as size_t,
                                                                                                            (font_ptr + 1 as internal_font_number) as size_t,
                                                                                                            fmt_in,
                                                                                                        );
                                                                                                        i_4 = 0 as ::core::ffi::c_int;
                                                                                                        while (i_4 as internal_font_number)
                                                                                                            < font_ptr + 1 as internal_font_number
                                                                                                        {
                                                                                                            if *(bchar_label.offset(0 as ::core::ffi::c_int as isize)
                                                                                                                as *mut font_index)
                                                                                                                .offset(i_4 as isize) < 0 as font_index
                                                                                                                || *(bchar_label.offset(0 as ::core::ffi::c_int as isize)
                                                                                                                    as *mut font_index)
                                                                                                                    .offset(i_4 as isize) > fmem_ptr - 1 as font_index
                                                                                                            {
                                                                                                                _tt_abort(
                                                                                                                    b"item %u (=%ld) of .fmt array at %lx <%ld or >%ld\0"
                                                                                                                        as *const u8 as *const ::core::ffi::c_char,
                                                                                                                    i_4,
                                                                                                                    *(bchar_label.offset(0 as ::core::ffi::c_int as isize)
                                                                                                                        as *mut font_index)
                                                                                                                        .offset(i_4 as isize) as uintptr_t,
                                                                                                                    bchar_label.offset(0 as ::core::ffi::c_int as isize)
                                                                                                                        as *mut font_index as uintptr_t,
                                                                                                                    0 as ::core::ffi::c_int as uintptr_t,
                                                                                                                    (fmem_ptr as uintptr_t)
                                                                                                                        .wrapping_sub(1 as ::core::ffi::c_int as uintptr_t),
                                                                                                                );
                                                                                                            }
                                                                                                            i_4 += 1;
                                                                                                        }
                                                                                                        let mut i_5: ::core::ffi::c_int = 0;
                                                                                                        do_undump(
                                                                                                            font_bchar.offset(0 as ::core::ffi::c_int as isize)
                                                                                                                as *mut nine_bits as *mut ::core::ffi::c_char,
                                                                                                            ::core::mem::size_of::<nine_bits>() as size_t,
                                                                                                            (font_ptr + 1 as internal_font_number) as size_t,
                                                                                                            fmt_in,
                                                                                                        );
                                                                                                        i_5 = 0 as ::core::ffi::c_int;
                                                                                                        while (i_5 as internal_font_number)
                                                                                                            < font_ptr + 1 as internal_font_number
                                                                                                        {
                                                                                                            if *(font_bchar.offset(0 as ::core::ffi::c_int as isize)
                                                                                                                as *mut nine_bits)
                                                                                                                .offset(i_5 as isize) < 0 as nine_bits
                                                                                                                || *(font_bchar.offset(0 as ::core::ffi::c_int as isize)
                                                                                                                    as *mut nine_bits)
                                                                                                                    .offset(i_5 as isize) > 65536 as nine_bits
                                                                                                            {
                                                                                                                _tt_abort(
                                                                                                                    b"item %u (=%ld) of .fmt array at %lx <%ld or >%ld\0"
                                                                                                                        as *const u8 as *const ::core::ffi::c_char,
                                                                                                                    i_5,
                                                                                                                    *(font_bchar.offset(0 as ::core::ffi::c_int as isize)
                                                                                                                        as *mut nine_bits)
                                                                                                                        .offset(i_5 as isize) as uintptr_t,
                                                                                                                    font_bchar.offset(0 as ::core::ffi::c_int as isize)
                                                                                                                        as *mut nine_bits as uintptr_t,
                                                                                                                    0 as ::core::ffi::c_int as uintptr_t,
                                                                                                                    65536 as ::core::ffi::c_int as uintptr_t,
                                                                                                                );
                                                                                                            }
                                                                                                            i_5 += 1;
                                                                                                        }
                                                                                                        let mut i_6: ::core::ffi::c_int = 0;
                                                                                                        do_undump(
                                                                                                            font_false_bchar.offset(0 as ::core::ffi::c_int as isize)
                                                                                                                as *mut nine_bits as *mut ::core::ffi::c_char,
                                                                                                            ::core::mem::size_of::<nine_bits>() as size_t,
                                                                                                            (font_ptr + 1 as internal_font_number) as size_t,
                                                                                                            fmt_in,
                                                                                                        );
                                                                                                        i_6 = 0 as ::core::ffi::c_int;
                                                                                                        while (i_6 as internal_font_number)
                                                                                                            < font_ptr + 1 as internal_font_number
                                                                                                        {
                                                                                                            if *(font_false_bchar
                                                                                                                .offset(0 as ::core::ffi::c_int as isize) as *mut nine_bits)
                                                                                                                .offset(i_6 as isize) < 0 as nine_bits
                                                                                                                || *(font_false_bchar
                                                                                                                    .offset(0 as ::core::ffi::c_int as isize) as *mut nine_bits)
                                                                                                                    .offset(i_6 as isize) > 65536 as nine_bits
                                                                                                            {
                                                                                                                _tt_abort(
                                                                                                                    b"item %u (=%ld) of .fmt array at %lx <%ld or >%ld\0"
                                                                                                                        as *const u8 as *const ::core::ffi::c_char,
                                                                                                                    i_6,
                                                                                                                    *(font_false_bchar.offset(0 as ::core::ffi::c_int as isize)
                                                                                                                        as *mut nine_bits)
                                                                                                                        .offset(i_6 as isize) as uintptr_t,
                                                                                                                    font_false_bchar.offset(0 as ::core::ffi::c_int as isize)
                                                                                                                        as *mut nine_bits as uintptr_t,
                                                                                                                    0 as ::core::ffi::c_int as uintptr_t,
                                                                                                                    65536 as ::core::ffi::c_int as uintptr_t,
                                                                                                                );
                                                                                                            }
                                                                                                            i_6 += 1;
                                                                                                        }
                                                                                                        do_undump(
                                                                                                            &raw mut x as *mut ::core::ffi::c_char,
                                                                                                            ::core::mem::size_of::<int32_t>() as size_t,
                                                                                                            1 as ::core::ffi::c_int as size_t,
                                                                                                            fmt_in,
                                                                                                        );
                                                                                                        if !(x < 0 as int32_t) {
                                                                                                            if x > hyph_size {
                                                                                                                _tt_abort(
                                                                                                                    b"must increase hyph_size\0" as *const u8
                                                                                                                        as *const ::core::ffi::c_char,
                                                                                                                );
                                                                                                            }
                                                                                                            hyph_count = x;
                                                                                                            do_undump(
                                                                                                                &raw mut x as *mut ::core::ffi::c_char,
                                                                                                                ::core::mem::size_of::<int32_t>() as size_t,
                                                                                                                1 as ::core::ffi::c_int as size_t,
                                                                                                                fmt_in,
                                                                                                            );
                                                                                                            if !(x < HYPH_PRIME as int32_t) {
                                                                                                                if x > hyph_size {
                                                                                                                    _tt_abort(
                                                                                                                        b"must increase hyph_size\0" as *const u8
                                                                                                                            as *const ::core::ffi::c_char,
                                                                                                                    );
                                                                                                                }
                                                                                                                hyph_next = x;
                                                                                                                j = 0 as ::core::ffi::c_int as int32_t;
                                                                                                                k = 1 as ::core::ffi::c_int as int32_t;
                                                                                                                loop {
                                                                                                                    if !(k <= hyph_count) {
                                                                                                                        current_block = 3240127892612706009;
                                                                                                                        break;
                                                                                                                    }
                                                                                                                    do_undump(
                                                                                                                        &raw mut j as *mut ::core::ffi::c_char,
                                                                                                                        ::core::mem::size_of::<int32_t>() as size_t,
                                                                                                                        1 as ::core::ffi::c_int as size_t,
                                                                                                                        fmt_in,
                                                                                                                    );
                                                                                                                    if j < 0 as int32_t {
                                                                                                                        current_block = 10982004377386733518;
                                                                                                                        break;
                                                                                                                    }
                                                                                                                    if j as ::core::ffi::c_long > 65535 as ::core::ffi::c_long {
                                                                                                                        hyph_next = (j as ::core::ffi::c_long
                                                                                                                            / 65536 as ::core::ffi::c_long) as int32_t;
                                                                                                                        j = (j as ::core::ffi::c_long
                                                                                                                            - hyph_next as ::core::ffi::c_long
                                                                                                                                * 65536 as ::core::ffi::c_long) as int32_t;
                                                                                                                    } else {
                                                                                                                        hyph_next = 0 as ::core::ffi::c_int as int32_t;
                                                                                                                    }
                                                                                                                    if j >= hyph_size || hyph_next > hyph_size {
                                                                                                                        current_block = 10982004377386733518;
                                                                                                                        break;
                                                                                                                    }
                                                                                                                    *hyph_link.offset(j as isize) = hyph_next as hyph_pointer;
                                                                                                                    do_undump(
                                                                                                                        &raw mut x as *mut ::core::ffi::c_char,
                                                                                                                        ::core::mem::size_of::<int32_t>() as size_t,
                                                                                                                        1 as ::core::ffi::c_int as size_t,
                                                                                                                        fmt_in,
                                                                                                                    );
                                                                                                                    if x < 0 as int32_t || x > str_ptr {
                                                                                                                        current_block = 10982004377386733518;
                                                                                                                        break;
                                                                                                                    }
                                                                                                                    *hyph_word.offset(j as isize) = x as str_number;
                                                                                                                    do_undump(
                                                                                                                        &raw mut x as *mut ::core::ffi::c_char,
                                                                                                                        ::core::mem::size_of::<int32_t>() as size_t,
                                                                                                                        1 as ::core::ffi::c_int as size_t,
                                                                                                                        fmt_in,
                                                                                                                    );
                                                                                                                    if x < MIN_HALFWORD as int32_t
                                                                                                                        || x > MAX_HALFWORD as int32_t
                                                                                                                    {
                                                                                                                        current_block = 10982004377386733518;
                                                                                                                        break;
                                                                                                                    }
                                                                                                                    *hyph_list.offset(j as isize) = x;
                                                                                                                    k += 1;
                                                                                                                }
                                                                                                                match current_block {
                                                                                                                    10982004377386733518 => {}
                                                                                                                    _ => {
                                                                                                                        j += 1;
                                                                                                                        if j < HYPH_PRIME as int32_t {
                                                                                                                            j = HYPH_PRIME as int32_t;
                                                                                                                        }
                                                                                                                        hyph_next = j;
                                                                                                                        if hyph_next >= hyph_size {
                                                                                                                            hyph_next = HYPH_PRIME as int32_t;
                                                                                                                        } else if hyph_next >= HYPH_PRIME as int32_t {
                                                                                                                            hyph_next += 1;
                                                                                                                        }
                                                                                                                        do_undump(
                                                                                                                            &raw mut x as *mut ::core::ffi::c_char,
                                                                                                                            ::core::mem::size_of::<int32_t>() as size_t,
                                                                                                                            1 as ::core::ffi::c_int as size_t,
                                                                                                                            fmt_in,
                                                                                                                        );
                                                                                                                        if !(x < 0 as int32_t) {
                                                                                                                            if x > trie_size {
                                                                                                                                _tt_abort(
                                                                                                                                    b"must increase trie_size\0" as *const u8
                                                                                                                                        as *const ::core::ffi::c_char,
                                                                                                                                );
                                                                                                                            }
                                                                                                                            j = x;
                                                                                                                            trie_max = j as trie_pointer;
                                                                                                                            do_undump(
                                                                                                                                &raw mut x as *mut ::core::ffi::c_char,
                                                                                                                                ::core::mem::size_of::<int32_t>() as size_t,
                                                                                                                                1 as ::core::ffi::c_int as size_t,
                                                                                                                                fmt_in,
                                                                                                                            );
                                                                                                                            if !(x < 0 as int32_t || x > j) {
                                                                                                                                hyph_start = x as trie_pointer;
                                                                                                                                if trie_trl.is_null() {
                                                                                                                                    trie_trl = malloc(
                                                                                                                                        ((j + 1 as int32_t + 1 as int32_t) as size_t)
                                                                                                                                            .wrapping_mul(
                                                                                                                                                ::core::mem::size_of::<trie_pointer>() as size_t,
                                                                                                                                            ),
                                                                                                                                    ) as *mut trie_pointer;
                                                                                                                                }
                                                                                                                                do_undump(
                                                                                                                                    trie_trl.offset(0 as ::core::ffi::c_int as isize)
                                                                                                                                        as *mut trie_pointer as *mut ::core::ffi::c_char,
                                                                                                                                    ::core::mem::size_of::<trie_pointer>() as size_t,
                                                                                                                                    (j + 1 as int32_t) as size_t,
                                                                                                                                    fmt_in,
                                                                                                                                );
                                                                                                                                if trie_tro.is_null() {
                                                                                                                                    trie_tro = malloc(
                                                                                                                                        ((j + 1 as int32_t + 1 as int32_t) as size_t)
                                                                                                                                            .wrapping_mul(
                                                                                                                                                ::core::mem::size_of::<trie_pointer>() as size_t,
                                                                                                                                            ),
                                                                                                                                    ) as *mut trie_pointer;
                                                                                                                                }
                                                                                                                                do_undump(
                                                                                                                                    trie_tro.offset(0 as ::core::ffi::c_int as isize)
                                                                                                                                        as *mut trie_pointer as *mut ::core::ffi::c_char,
                                                                                                                                    ::core::mem::size_of::<trie_pointer>() as size_t,
                                                                                                                                    (j + 1 as int32_t) as size_t,
                                                                                                                                    fmt_in,
                                                                                                                                );
                                                                                                                                if trie_trc.is_null() {
                                                                                                                                    trie_trc = malloc(
                                                                                                                                        ((j + 1 as int32_t + 1 as int32_t) as size_t)
                                                                                                                                            .wrapping_mul(::core::mem::size_of::<uint16_t>() as size_t),
                                                                                                                                    ) as *mut uint16_t;
                                                                                                                                }
                                                                                                                                do_undump(
                                                                                                                                    trie_trc.offset(0 as ::core::ffi::c_int as isize)
                                                                                                                                        as *mut uint16_t as *mut ::core::ffi::c_char,
                                                                                                                                    ::core::mem::size_of::<uint16_t>() as size_t,
                                                                                                                                    (j + 1 as int32_t) as size_t,
                                                                                                                                    fmt_in,
                                                                                                                                );
                                                                                                                                do_undump(
                                                                                                                                    &raw mut max_hyph_char as *mut ::core::ffi::c_char,
                                                                                                                                    ::core::mem::size_of::<int32_t>() as size_t,
                                                                                                                                    1 as ::core::ffi::c_int as size_t,
                                                                                                                                    fmt_in,
                                                                                                                                );
                                                                                                                                do_undump(
                                                                                                                                    &raw mut x as *mut ::core::ffi::c_char,
                                                                                                                                    ::core::mem::size_of::<int32_t>() as size_t,
                                                                                                                                    1 as ::core::ffi::c_int as size_t,
                                                                                                                                    fmt_in,
                                                                                                                                );
                                                                                                                                if !(x < 0 as int32_t) {
                                                                                                                                    if x as ::core::ffi::c_long > TRIE_OP_SIZE {
                                                                                                                                        _tt_abort(
                                                                                                                                            b"must increase TRIE_OP_SIZE\0" as *const u8
                                                                                                                                                as *const ::core::ffi::c_char,
                                                                                                                                        );
                                                                                                                                    }
                                                                                                                                    j = x;
                                                                                                                                    trie_op_ptr = j;
                                                                                                                                    do_undump(
                                                                                                                                        (&raw mut hyf_distance as *mut small_number)
                                                                                                                                            .offset(1 as ::core::ffi::c_int as isize)
                                                                                                                                            as *mut small_number as *mut ::core::ffi::c_char,
                                                                                                                                        ::core::mem::size_of::<small_number>() as size_t,
                                                                                                                                        j as size_t,
                                                                                                                                        fmt_in,
                                                                                                                                    );
                                                                                                                                    do_undump(
                                                                                                                                        (&raw mut hyf_num as *mut small_number)
                                                                                                                                            .offset(1 as ::core::ffi::c_int as isize)
                                                                                                                                            as *mut small_number as *mut ::core::ffi::c_char,
                                                                                                                                        ::core::mem::size_of::<small_number>() as size_t,
                                                                                                                                        j as size_t,
                                                                                                                                        fmt_in,
                                                                                                                                    );
                                                                                                                                    let mut i_7: ::core::ffi::c_int = 0;
                                                                                                                                    do_undump(
                                                                                                                                        (&raw mut hyf_next as *mut trie_opcode)
                                                                                                                                            .offset(1 as ::core::ffi::c_int as isize)
                                                                                                                                            as *mut trie_opcode as *mut ::core::ffi::c_char,
                                                                                                                                        ::core::mem::size_of::<trie_opcode>() as size_t,
                                                                                                                                        j as size_t,
                                                                                                                                        fmt_in,
                                                                                                                                    );
                                                                                                                                    i_7 = 0 as ::core::ffi::c_int;
                                                                                                                                    while (i_7 as int32_t) < j {
                                                                                                                                        if *((&raw mut hyf_next as *mut trie_opcode)
                                                                                                                                            .offset(1 as ::core::ffi::c_int as isize)
                                                                                                                                            as *mut trie_opcode)
                                                                                                                                            .offset(i_7 as isize) as ::core::ffi::c_long
                                                                                                                                            > 65535 as ::core::ffi::c_long
                                                                                                                                        {
                                                                                                                                            _tt_abort(
                                                                                                                                                b"Item %u (=%ld) of .fmt array at %lx >%ld\0" as *const u8
                                                                                                                                                    as *const ::core::ffi::c_char,
                                                                                                                                                i_7,
                                                                                                                                                *((&raw mut hyf_next as *mut trie_opcode)
                                                                                                                                                    .offset(1 as ::core::ffi::c_int as isize)
                                                                                                                                                    as *mut trie_opcode)
                                                                                                                                                    .offset(i_7 as isize) as uintptr_t,
                                                                                                                                                (&raw mut hyf_next as *mut trie_opcode)
                                                                                                                                                    .offset(1 as ::core::ffi::c_int as isize)
                                                                                                                                                    as *mut trie_opcode as uintptr_t,
                                                                                                                                                65535 as ::core::ffi::c_long as uintptr_t,
                                                                                                                                            );
                                                                                                                                        }
                                                                                                                                        i_7 += 1;
                                                                                                                                    }
                                                                                                                                    k = 0 as ::core::ffi::c_int as int32_t;
                                                                                                                                    while k <= BIGGEST_LANG as int32_t {
                                                                                                                                        trie_used[k as usize] = 0 as trie_opcode;
                                                                                                                                        k += 1;
                                                                                                                                    }
                                                                                                                                    k = (BIGGEST_LANG + 1 as ::core::ffi::c_int) as int32_t;
                                                                                                                                    loop {
                                                                                                                                        if !(j > 0 as int32_t) {
                                                                                                                                            current_block = 7344615536999694015;
                                                                                                                                            break;
                                                                                                                                        }
                                                                                                                                        do_undump(
                                                                                                                                            &raw mut x as *mut ::core::ffi::c_char,
                                                                                                                                            ::core::mem::size_of::<int32_t>() as size_t,
                                                                                                                                            1 as ::core::ffi::c_int as size_t,
                                                                                                                                            fmt_in,
                                                                                                                                        );
                                                                                                                                        if x < 0 as int32_t || x > k - 1 as int32_t {
                                                                                                                                            current_block = 10982004377386733518;
                                                                                                                                            break;
                                                                                                                                        }
                                                                                                                                        k = x;
                                                                                                                                        do_undump(
                                                                                                                                            &raw mut x as *mut ::core::ffi::c_char,
                                                                                                                                            ::core::mem::size_of::<int32_t>() as size_t,
                                                                                                                                            1 as ::core::ffi::c_int as size_t,
                                                                                                                                            fmt_in,
                                                                                                                                        );
                                                                                                                                        if x < 1 as int32_t || x > j {
                                                                                                                                            current_block = 10982004377386733518;
                                                                                                                                            break;
                                                                                                                                        }
                                                                                                                                        trie_used[k as usize] = x as trie_opcode;
                                                                                                                                        j = j - x;
                                                                                                                                        op_start[k as usize] = j;
                                                                                                                                    }
                                                                                                                                    match current_block {
                                                                                                                                        10982004377386733518 => {}
                                                                                                                                        _ => {
                                                                                                                                            trie_not_ready = false_0 != 0;
                                                                                                                                            do_undump(
                                                                                                                                                &raw mut x as *mut ::core::ffi::c_char,
                                                                                                                                                ::core::mem::size_of::<int32_t>() as size_t,
                                                                                                                                                1 as ::core::ffi::c_int as size_t,
                                                                                                                                                fmt_in,
                                                                                                                                            );
                                                                                                                                            if !(x != FORMAT_FOOTER_MAGIC as int32_t) {
                                                                                                                                                ttstub_input_close(fmt_in);
                                                                                                                                                return true_0 != 0;
                                                                                                                                            }
                                                                                                                                        }
                                                                                                                                    }
                                                                                                                                }
                                                                                                                            }
                                                                                                                        }
                                                                                                                    }
                                                                                                                }
                                                                                                            }
                                                                                                        }
                                                                                                    }
                                                                                                }
                                                                                            }
                                                                                        }
                                                                                    }
                                                                                }
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    _tt_abort(b"fatal format file error\0" as *const u8 as *const ::core::ffi::c_char);
}
unsafe extern "C" fn final_cleanup() {
    let mut c: small_number = 0;
    c = cur_chr as small_number;
    if c as ::core::ffi::c_int != 1 as ::core::ffi::c_int {
        (*eqtb.offset((INT_BASE + INT_PAR__new_line_char) as isize))
            .b32
            .s1 = -(1 as ::core::ffi::c_int) as int32_t;
    }
    if job_name == 0 as str_number {
        open_log_file();
    }
    while input_ptr > 0 as int32_t {
        if cur_input.state as ::core::ffi::c_int == TOKEN_LIST {
            end_token_list();
        } else {
            end_file_reading();
        }
    }
    while open_parens > 0 as int32_t {
        print_cstr(b" )\0" as *const u8 as *const ::core::ffi::c_char);
        open_parens -= 1;
    }
    if cur_level as ::core::ffi::c_int > LEVEL_ONE {
        print_nl('(' as i32);
        print_esc_cstr(b"end occurred \0" as *const u8 as *const ::core::ffi::c_char);
        print_cstr(b"inside a group at level \0" as *const u8 as *const ::core::ffi::c_char);
        print_int(cur_level as int32_t - 1 as int32_t);
        print_char(')' as i32);
        show_save_groups();
    }
    while cond_ptr != TEX_NULL as int32_t {
        print_nl('(' as i32);
        print_esc_cstr(b"end occurred \0" as *const u8 as *const ::core::ffi::c_char);
        print_cstr(b"when \0" as *const u8 as *const ::core::ffi::c_char);
        print_cmd_chr(IF_TEST as uint16_t, cur_if as int32_t);
        if if_line != 0 as int32_t {
            print_cstr(b" on line \0" as *const u8 as *const ::core::ffi::c_char);
            print_int(if_line);
        }
        print_cstr(b" was incomplete)\0" as *const u8 as *const ::core::ffi::c_char);
        if_line = (*mem.offset((cond_ptr + 1 as int32_t) as isize)).b32.s1;
        cur_if = (*mem.offset(cond_ptr as isize)).b16.s0 as small_number;
        temp_ptr = cond_ptr;
        cond_ptr = (*mem.offset(cond_ptr as isize)).b32.s1;
        free_node(temp_ptr, IF_NODE_SIZE as int32_t);
    }
    if history as ::core::ffi::c_uint
        != HISTORY_SPOTLESS as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        if history as ::core::ffi::c_uint
            == HISTORY_WARNING_ISSUED as ::core::ffi::c_int as ::core::ffi::c_uint
            || (interaction as ::core::ffi::c_int) < ERROR_STOP_MODE
        {
            if selector as ::core::ffi::c_uint
                == SELECTOR_TERM_AND_LOG as ::core::ffi::c_int as ::core::ffi::c_uint
            {
                selector = SELECTOR_TERM_ONLY;
                print_nl_cstr(
                    b"(see the transcript file for additional information)\0" as *const u8
                        as *const ::core::ffi::c_char,
                );
                selector = SELECTOR_TERM_AND_LOG;
            }
        }
    }
    if c as ::core::ffi::c_int == 1 as ::core::ffi::c_int {
        if in_initex_mode {
            let mut for_end: int32_t = 0;
            c = TOP_MARK_CODE as small_number;
            for_end = SPLIT_BOT_MARK_CODE as int32_t;
            if c as int32_t <= for_end {
                loop {
                    if cur_mark[c as usize] != TEX_NULL as int32_t {
                        delete_token_ref(cur_mark[c as usize]);
                    }
                    let fresh3 = c;
                    c = c + 1;
                    if !((fresh3 as int32_t) < for_end) {
                        break;
                    }
                }
            }
            if sa_root[MARK_VAL as usize] != TEX_NULL as int32_t {
                if do_marks(
                    3 as small_number,
                    0 as small_number,
                    sa_root[MARK_VAL as usize],
                ) {
                    sa_root[MARK_VAL as usize] = TEX_NULL as int32_t;
                }
            }
            let mut for_end_0: int32_t = 0;
            c = LAST_BOX_CODE as small_number;
            for_end_0 = VSPLIT_CODE as int32_t;
            if c as int32_t <= for_end_0 {
                loop {
                    flush_node_list(disc_ptr[c as usize]);
                    let fresh4 = c;
                    c = c + 1;
                    if !((fresh4 as int32_t) < for_end_0) {
                        break;
                    }
                }
            }
            if last_glue != MAX_HALFWORD as int32_t {
                delete_glue_ref(last_glue);
            }
            store_fmt_file();
            return;
        }
        print_nl_cstr(
            b"(\\dump is performed only by INITEX)\0" as *const u8 as *const ::core::ffi::c_char,
        );
        return;
    }
}
static mut stdin_ufile: UFILE = UFILE {
    handle: ::core::ptr::null::<ttbc_input_handle_t>() as *mut ttbc_input_handle_t,
    savedChar: 0,
    skipNextLF: 0,
    encodingMode: 0,
    conversionData: ::core::ptr::null::<::core::ffi::c_void>() as *mut ::core::ffi::c_void,
};
unsafe extern "C" fn init_io() {
    stdin_ufile.handle = ::core::ptr::null_mut::<ttbc_input_handle_t>();
    stdin_ufile.savedChar = -(1 as ::core::ffi::c_int) as ::core::ffi::c_long;
    stdin_ufile.skipNextLF = 0 as ::core::ffi::c_short;
    stdin_ufile.encodingMode = UTF8 as ::core::ffi::c_short;
    stdin_ufile.conversionData = ::core::ptr::null_mut::<::core::ffi::c_void>();
    let ref mut fresh18 = *input_file.offset(0 as ::core::ffi::c_int as isize);
    *fresh18 = &raw mut stdin_ufile;
    *buffer.offset(first as isize) = 0 as ::core::ffi::c_int as UnicodeScalar;
    last = first;
    cur_input.loc = first;
    cur_input.limit = last;
    first = last + 1 as int32_t;
}
unsafe extern "C" fn initialize_more_variables() {
    let mut k: int32_t = 0;
    let mut z: hyph_pointer = 0;
    doing_special = false_0 != 0;
    native_text_size = 128 as ::core::ffi::c_int as int32_t;
    native_text = malloc(
        (native_text_size as size_t).wrapping_mul(::core::mem::size_of::<UTF16_code>() as size_t),
    ) as *mut UTF16_code;
    interaction = ERROR_STOP_MODE as ::core::ffi::c_uchar;
    deletions_allowed = true_0 != 0;
    set_box_allowed = true_0 != 0;
    error_count = 0 as ::core::ffi::c_schar;
    help_ptr = 0 as ::core::ffi::c_uchar;
    use_err_help = false_0 != 0;
    two_to_the[0 as ::core::ffi::c_int as usize] = 1 as ::core::ffi::c_int as int32_t;
    k = 1 as ::core::ffi::c_int as int32_t;
    while k <= 30 as int32_t {
        two_to_the[k as usize] = 2 as int32_t * two_to_the[(k - 1 as int32_t) as usize];
        k += 1;
    }
    spec_log[1 as ::core::ffi::c_int as usize] = 93032640 as int32_t;
    spec_log[2 as ::core::ffi::c_int as usize] = 38612034 as int32_t;
    spec_log[3 as ::core::ffi::c_int as usize] = 17922280 as int32_t;
    spec_log[4 as ::core::ffi::c_int as usize] = 8662214 as int32_t;
    spec_log[5 as ::core::ffi::c_int as usize] = 4261238 as int32_t;
    spec_log[6 as ::core::ffi::c_int as usize] = 2113709 as int32_t;
    spec_log[7 as ::core::ffi::c_int as usize] = 1052693 as int32_t;
    spec_log[8 as ::core::ffi::c_int as usize] = 525315 as int32_t;
    spec_log[9 as ::core::ffi::c_int as usize] = 262400 as int32_t;
    spec_log[10 as ::core::ffi::c_int as usize] = 131136 as int32_t;
    spec_log[11 as ::core::ffi::c_int as usize] = 65552 as int32_t;
    spec_log[12 as ::core::ffi::c_int as usize] = 32772 as int32_t;
    spec_log[13 as ::core::ffi::c_int as usize] = 16385 as ::core::ffi::c_int as int32_t;
    k = 14 as ::core::ffi::c_int as int32_t;
    while k <= 27 as int32_t {
        spec_log[k as usize] = two_to_the[(27 as int32_t - k) as usize];
        k += 1;
    }
    spec_log[28 as ::core::ffi::c_int as usize] = 1 as ::core::ffi::c_int as int32_t;
    nest_ptr = 0 as ::core::ffi::c_int as int32_t;
    max_nest_stack = 0 as ::core::ffi::c_int as int32_t;
    cur_list.mode = VMODE as ::core::ffi::c_short;
    cur_list.head = CONTRIB_HEAD as int32_t;
    cur_list.tail = CONTRIB_HEAD as int32_t;
    cur_list.eTeX_aux = TEX_NULL as int32_t;
    cur_list.aux.b32.s1 = IGNORE_DEPTH as int32_t;
    cur_list.mode_line = 0 as ::core::ffi::c_int as int32_t;
    cur_list.prev_graf = 0 as ::core::ffi::c_int as int32_t;
    shown_mode = 0 as ::core::ffi::c_short;
    page_contents = EMPTY as ::core::ffi::c_uchar;
    page_tail = PAGE_HEAD as int32_t;
    last_glue = MAX_HALFWORD as int32_t;
    last_penalty = 0 as ::core::ffi::c_int as int32_t;
    last_kern = 0 as ::core::ffi::c_int as scaled_t;
    last_node_type = -(1 as ::core::ffi::c_int) as int32_t;
    page_so_far[7 as ::core::ffi::c_int as usize] = 0 as ::core::ffi::c_int as scaled_t;
    k = INT_BASE as int32_t;
    while k <= EQTB_SIZE as int32_t {
        _xeq_level_array[(k - INT_BASE as int32_t) as usize] = LEVEL_ONE as uint16_t;
        k += 1;
    }
    no_new_control_sequence = true_0 != 0;
    prim[0 as ::core::ffi::c_int as usize].s0 = 0 as ::core::ffi::c_int as int32_t;
    prim[0 as ::core::ffi::c_int as usize].s1 = 0 as ::core::ffi::c_int as int32_t;
    k = 1 as ::core::ffi::c_int as int32_t;
    while k <= PRIM_SIZE as int32_t {
        prim[k as usize] = prim[0 as ::core::ffi::c_int as usize];
        k += 1;
    }
    save_ptr = 0 as ::core::ffi::c_int as int32_t;
    cur_level = LEVEL_ONE as uint16_t;
    cur_group = BOTTOM_LEVEL as group_code;
    cur_boundary = 0 as ::core::ffi::c_int as int32_t;
    max_save_stack = 0 as ::core::ffi::c_int as int32_t;
    mag_set = 0 as ::core::ffi::c_int as int32_t;
    expand_depth_count = 0 as ::core::ffi::c_int as int32_t;
    is_in_csname = false_0 != 0;
    cur_mark[TOP_MARK_CODE as usize] = TEX_NULL as int32_t;
    cur_mark[FIRST_MARK_CODE as usize] = TEX_NULL as int32_t;
    cur_mark[BOT_MARK_CODE as usize] = TEX_NULL as int32_t;
    cur_mark[SPLIT_FIRST_MARK_CODE as usize] = TEX_NULL as int32_t;
    cur_mark[SPLIT_BOT_MARK_CODE as usize] = TEX_NULL as int32_t;
    cur_val = 0 as ::core::ffi::c_int as int32_t;
    cur_val_level = INT_VAL as ::core::ffi::c_uchar;
    radix = 0 as small_number;
    cur_order = NORMAL as glue_ord;
    k = 0 as ::core::ffi::c_int as int32_t;
    while k <= 16 as int32_t {
        read_open[k as usize] = CLOSED as ::core::ffi::c_uchar;
        k += 1;
    }
    cond_ptr = TEX_NULL as int32_t;
    if_limit = NORMAL as ::core::ffi::c_uchar;
    cur_if = 0 as small_number;
    if_line = 0 as ::core::ffi::c_int as int32_t;
    null_character.s3 = 0 as uint16_t;
    null_character.s2 = 0 as uint16_t;
    null_character.s1 = 0 as uint16_t;
    null_character.s0 = 0 as uint16_t;
    total_pages = 0 as ::core::ffi::c_int as int32_t;
    max_v = 0 as ::core::ffi::c_int as scaled_t;
    max_h = 0 as ::core::ffi::c_int as scaled_t;
    max_push = 0 as ::core::ffi::c_int as int32_t;
    last_bop = -(1 as ::core::ffi::c_int) as int32_t;
    doing_leaders = false_0 != 0;
    dead_cycles = 0 as ::core::ffi::c_int as int32_t;
    adjust_tail = TEX_NULL as int32_t;
    last_badness = 0 as ::core::ffi::c_int as int32_t;
    pre_adjust_tail = TEX_NULL as int32_t;
    pack_begin_line = 0 as ::core::ffi::c_int as int32_t;
    empty.s1 = EMPTY as int32_t;
    empty.s0 = TEX_NULL as int32_t;
    align_ptr = TEX_NULL as int32_t;
    cur_align = TEX_NULL as int32_t;
    cur_span = TEX_NULL as int32_t;
    cur_loop = TEX_NULL as int32_t;
    cur_head = TEX_NULL as int32_t;
    cur_tail = TEX_NULL as int32_t;
    cur_pre_head = TEX_NULL as int32_t;
    cur_pre_tail = TEX_NULL as int32_t;
    cur_f = 0 as ::core::ffi::c_int as internal_font_number;
    max_hyph_char = TOO_BIG_LANG as int32_t;
    z = 0 as hyph_pointer;
    while z as int32_t <= hyph_size {
        *hyph_word.offset(z as isize) = 0 as ::core::ffi::c_int as str_number;
        *hyph_list.offset(z as isize) = TEX_NULL as int32_t;
        *hyph_link.offset(z as isize) = 0 as hyph_pointer;
        z = z.wrapping_add(1);
    }
    hyph_count = 0 as ::core::ffi::c_int as int32_t;
    hyph_next = (HYPH_PRIME + 1 as ::core::ffi::c_int) as int32_t;
    if hyph_next > hyph_size {
        hyph_next = HYPH_PRIME as int32_t;
    }
    output_active = false_0 != 0;
    insert_penalties = 0 as ::core::ffi::c_int as int32_t;
    ligature_present = false_0 != 0;
    cancel_boundary = false_0 != 0;
    lft_hit = false_0 != 0;
    rt_hit = false_0 != 0;
    ins_disc = false_0 != 0;
    after_token = 0 as ::core::ffi::c_int as int32_t;
    long_help_seen = false_0 != 0;
    format_ident = 0 as ::core::ffi::c_int as str_number;
    k = 0 as ::core::ffi::c_int as int32_t;
    while k <= 17 as int32_t {
        write_open[k as usize] = false_0 != 0;
        k += 1;
    }
    LR_ptr = TEX_NULL as int32_t;
    LR_problems = 0 as ::core::ffi::c_int as int32_t;
    cur_dir = LEFT_TO_RIGHT as small_number;
    pseudo_files = TEX_NULL as int32_t;
    sa_root[MARK_VAL as usize] = TEX_NULL as int32_t;
    sa_null.b32.s0 = TEX_NULL as int32_t;
    sa_null.b32.s1 = TEX_NULL as int32_t;
    sa_chain = TEX_NULL as int32_t;
    sa_level = LEVEL_ZERO as uint16_t;
    disc_ptr[LAST_BOX_CODE as usize] = TEX_NULL as int32_t;
    disc_ptr[VSPLIT_CODE as usize] = TEX_NULL as int32_t;
    edit_name_start = 0 as ::core::ffi::c_int as pool_pointer;
    stop_at_space = true_0 != 0;
}
unsafe extern "C" fn initialize_more_initex_variables() {
    let mut i: int32_t = 0;
    let mut k: int32_t = 0;
    k = 1 as ::core::ffi::c_int as int32_t;
    while k <= 19 as int32_t {
        (*mem.offset(k as isize)).b32.s1 = 0 as ::core::ffi::c_int as int32_t;
        k += 1;
    }
    k = 0 as ::core::ffi::c_int as int32_t;
    while k <= 19 as int32_t {
        (*mem.offset(k as isize)).b32.s1 = (TEX_NULL + 1 as ::core::ffi::c_int) as int32_t;
        (*mem.offset(k as isize)).b16.s1 = NORMAL as uint16_t;
        (*mem.offset(k as isize)).b16.s0 = NORMAL as uint16_t;
        k = (k as ::core::ffi::c_int + 4 as ::core::ffi::c_int) as int32_t;
    }
    (*mem.offset(6 as ::core::ffi::c_int as isize)).b32.s1 = 65536 as int32_t;
    (*mem.offset(4 as ::core::ffi::c_int as isize)).b16.s1 = FIL as uint16_t;
    (*mem.offset(10 as ::core::ffi::c_int as isize)).b32.s1 = 65536 as int32_t;
    (*mem.offset(8 as ::core::ffi::c_int as isize)).b16.s1 = FILL as uint16_t;
    (*mem.offset(14 as ::core::ffi::c_int as isize)).b32.s1 = 65536 as int32_t;
    (*mem.offset(12 as ::core::ffi::c_int as isize)).b16.s1 = FIL as uint16_t;
    (*mem.offset(15 as ::core::ffi::c_int as isize)).b32.s1 = 65536 as int32_t;
    (*mem.offset(12 as ::core::ffi::c_int as isize)).b16.s0 = FIL as uint16_t;
    (*mem.offset(18 as ::core::ffi::c_int as isize)).b32.s1 =
        -(65536 as ::core::ffi::c_long) as int32_t;
    (*mem.offset(16 as ::core::ffi::c_int as isize)).b16.s1 = FIL as uint16_t;
    rover = 20 as ::core::ffi::c_int as int32_t;
    (*mem.offset(rover as isize)).b32.s1 = MAX_HALFWORD as int32_t;
    (*mem.offset(rover as isize)).b32.s0 = 1000 as ::core::ffi::c_int as int32_t;
    (*mem.offset((rover + 1 as int32_t) as isize)).b32.s0 = rover;
    (*mem.offset((rover + 1 as int32_t) as isize)).b32.s1 = rover;
    lo_mem_max = rover + 1000 as int32_t;
    (*mem.offset(lo_mem_max as isize)).b32.s1 = TEX_NULL as int32_t;
    (*mem.offset(lo_mem_max as isize)).b32.s0 = TEX_NULL as int32_t;
    k = PRE_ADJUST_HEAD as int32_t;
    while k <= MEM_TOP as int32_t {
        *mem.offset(k as isize) = *mem.offset(lo_mem_max as isize);
        k += 1;
    }
    (*mem.offset(OMIT_TEMPLATE as isize)).b32.s0 = (CS_TOKEN_FLAG + FROZEN_END_TEMPLATE) as int32_t;
    (*mem.offset(END_SPAN as isize)).b32.s1 = (UINT16_MAX + 1 as ::core::ffi::c_int) as int32_t;
    (*mem.offset(END_SPAN as isize)).b32.s0 = TEX_NULL as int32_t;
    (*mem.offset(ACTIVE_LIST as isize)).b16.s1 = HYPHENATED as uint16_t;
    (*mem.offset((ACTIVE_LIST + 1 as ::core::ffi::c_int) as isize))
        .b32
        .s0 = MAX_HALFWORD as int32_t;
    (*mem.offset(ACTIVE_LIST as isize)).b16.s0 = 0 as uint16_t;
    (*mem.offset(PAGE_INS_HEAD as isize)).b16.s0 = 255 as uint16_t;
    (*mem.offset(PAGE_INS_HEAD as isize)).b16.s1 = SPLIT_UP as uint16_t;
    (*mem.offset(PAGE_INS_HEAD as isize)).b32.s1 = PAGE_INS_HEAD as int32_t;
    (*mem.offset((4999999 as ::core::ffi::c_int - 2 as ::core::ffi::c_int) as isize))
        .b16
        .s1 = GLUE_NODE as uint16_t;
    (*mem.offset(PAGE_HEAD as isize)).b16.s0 = NORMAL as uint16_t;
    avail = TEX_NULL as int32_t;
    mem_end = MEM_TOP as int32_t;
    hi_mem_min = PRE_ADJUST_HEAD as int32_t;
    var_used = 20 as ::core::ffi::c_int as int32_t;
    dyn_used = HI_MEM_STAT_USAGE as int32_t;
    (*eqtb.offset(UNDEFINED_CONTROL_SEQUENCE as isize)).b16.s1 = UNDEFINED_CS as uint16_t;
    (*eqtb.offset(UNDEFINED_CONTROL_SEQUENCE as isize)).b32.s1 = TEX_NULL as int32_t;
    (*eqtb.offset(UNDEFINED_CONTROL_SEQUENCE as isize)).b16.s0 = LEVEL_ZERO as uint16_t;
    k = ACTIVE_BASE as int32_t;
    while k <= eqtb_top {
        *eqtb.offset(k as isize) = *eqtb.offset(UNDEFINED_CONTROL_SEQUENCE as isize);
        k += 1;
    }
    (*eqtb.offset(GLUE_BASE as isize)).b32.s1 = 0 as ::core::ffi::c_int as int32_t;
    (*eqtb.offset(GLUE_BASE as isize)).b16.s0 = LEVEL_ONE as uint16_t;
    (*eqtb.offset(GLUE_BASE as isize)).b16.s1 = GLUE_REF as uint16_t;
    k = (GLUE_BASE + 1 as ::core::ffi::c_int) as int32_t;
    while k <= LOCAL_BASE as int32_t - 1 as int32_t {
        *eqtb.offset(k as isize) = *eqtb.offset(GLUE_BASE as isize);
        k += 1;
    }
    let ref mut fresh19 = (*mem.offset(0 as ::core::ffi::c_int as isize)).b32.s1;
    *fresh19 = (*fresh19 as ::core::ffi::c_int + 531 as ::core::ffi::c_int) as int32_t;
    (*eqtb.offset((LOCAL_BASE + LOCAL__par_shape) as isize))
        .b32
        .s1 = TEX_NULL as int32_t;
    (*eqtb.offset((LOCAL_BASE + LOCAL__par_shape) as isize))
        .b16
        .s1 = SHAPE_REF as uint16_t;
    (*eqtb.offset((LOCAL_BASE + LOCAL__par_shape) as isize))
        .b16
        .s0 = LEVEL_ONE as uint16_t;
    k = ETEX_PEN_BASE as int32_t;
    while k <= NUM_ETEX_PENALTIES as int32_t - 1 as int32_t {
        *eqtb.offset(k as isize) = *eqtb.offset((LOCAL_BASE + LOCAL__par_shape) as isize);
        k += 1;
    }
    k = (LOCAL_BASE + LOCAL__output_routine) as int32_t;
    while k <= TOKS_BASE as int32_t + NUMBER_REGS as int32_t - 1 as int32_t {
        *eqtb.offset(k as isize) = *eqtb.offset(UNDEFINED_CONTROL_SEQUENCE as isize);
        k += 1;
    }
    (*eqtb.offset(BOX_BASE as isize)).b32.s1 = TEX_NULL as int32_t;
    (*eqtb.offset(BOX_BASE as isize)).b16.s1 = BOX_REF as uint16_t;
    (*eqtb.offset(BOX_BASE as isize)).b16.s0 = LEVEL_ONE as uint16_t;
    k = (BOX_BASE + 1 as ::core::ffi::c_int) as int32_t;
    while k <= BOX_BASE as int32_t + NUMBER_REGS as int32_t - 1 as int32_t {
        *eqtb.offset(k as isize) = *eqtb.offset(BOX_BASE as isize);
        k += 1;
    }
    (*eqtb.offset(CUR_FONT_LOC as isize)).b32.s1 = FONT_BASE as int32_t;
    (*eqtb.offset(CUR_FONT_LOC as isize)).b16.s1 = DATA as uint16_t;
    (*eqtb.offset(CUR_FONT_LOC as isize)).b16.s0 = LEVEL_ONE as uint16_t;
    k = MATH_FONT_BASE as int32_t;
    while k <= MATH_FONT_BASE as int32_t + NUMBER_MATH_FONTS as int32_t - 1 as int32_t {
        *eqtb.offset(k as isize) = *eqtb.offset(CUR_FONT_LOC as isize);
        k += 1;
    }
    (*eqtb.offset(CAT_CODE_BASE as isize)).b32.s1 = 0 as ::core::ffi::c_int as int32_t;
    (*eqtb.offset(CAT_CODE_BASE as isize)).b16.s1 = DATA as uint16_t;
    (*eqtb.offset(CAT_CODE_BASE as isize)).b16.s0 = LEVEL_ONE as uint16_t;
    k = (CAT_CODE_BASE + 1 as ::core::ffi::c_int) as int32_t;
    while k <= INT_BASE as int32_t - 1 as int32_t {
        *eqtb.offset(k as isize) = *eqtb.offset(CAT_CODE_BASE as isize);
        k += 1;
    }
    k = 0 as ::core::ffi::c_int as int32_t;
    while k <= NUMBER_USVS as int32_t - 1 as int32_t {
        (*eqtb.offset((CAT_CODE_BASE as int32_t + k) as isize))
            .b32
            .s1 = OTHER_CHAR as int32_t;
        (*eqtb.offset((MATH_CODE_BASE as int32_t + k) as isize))
            .b32
            .s1 = k;
        (*eqtb.offset((SF_CODE_BASE as int32_t + k) as isize))
            .b32
            .s1 = 1000 as ::core::ffi::c_int as int32_t;
        k += 1;
    }
    (*eqtb.offset((CAT_CODE_BASE + 13 as ::core::ffi::c_int) as isize))
        .b32
        .s1 = CAR_RET as int32_t;
    (*eqtb.offset((CAT_CODE_BASE + 32 as ::core::ffi::c_int) as isize))
        .b32
        .s1 = SPACER as int32_t;
    (*eqtb.offset((CAT_CODE_BASE + 92 as ::core::ffi::c_int) as isize))
        .b32
        .s1 = ESCAPE as int32_t;
    (*eqtb.offset((CAT_CODE_BASE + 37 as ::core::ffi::c_int) as isize))
        .b32
        .s1 = COMMENT as int32_t;
    (*eqtb.offset((CAT_CODE_BASE + 127 as ::core::ffi::c_int) as isize))
        .b32
        .s1 = INVALID_CHAR as int32_t;
    (*eqtb.offset(CAT_CODE_BASE as isize)).b32.s1 = IGNORE as int32_t;
    k = '0' as i32 as int32_t;
    while k <= '9' as i32 {
        (*eqtb.offset((MATH_CODE_BASE as int32_t + k) as isize))
            .b32
            .s1 = (k as ::core::ffi::c_uint).wrapping_add(
            (7 as ::core::ffi::c_int as ::core::ffi::c_uint & 0x7 as ::core::ffi::c_uint)
                << 21 as ::core::ffi::c_int,
        ) as int32_t;
        k += 1;
    }
    k = 'A' as i32 as int32_t;
    while k <= 'Z' as i32 {
        (*eqtb.offset((CAT_CODE_BASE as int32_t + k) as isize))
            .b32
            .s1 = LETTER as int32_t;
        (*eqtb.offset((CAT_CODE_BASE as int32_t + (k + 32 as int32_t)) as isize))
            .b32
            .s1 = LETTER as int32_t;
        (*eqtb.offset((MATH_CODE_BASE as int32_t + k) as isize))
            .b32
            .s1 = (k as ::core::ffi::c_uint)
            .wrapping_add(
                (1 as ::core::ffi::c_int as ::core::ffi::c_uint & 0xff as ::core::ffi::c_uint)
                    << 24 as ::core::ffi::c_int,
            )
            .wrapping_add(
                (7 as ::core::ffi::c_int as ::core::ffi::c_uint & 0x7 as ::core::ffi::c_uint)
                    << 21 as ::core::ffi::c_int,
            ) as int32_t;
        (*eqtb.offset((MATH_CODE_BASE as int32_t + (k + 32 as int32_t)) as isize))
            .b32
            .s1 = ((k + 32 as int32_t) as ::core::ffi::c_uint)
            .wrapping_add(
                (1 as ::core::ffi::c_int as ::core::ffi::c_uint & 0xff as ::core::ffi::c_uint)
                    << 24 as ::core::ffi::c_int,
            )
            .wrapping_add(
                (7 as ::core::ffi::c_int as ::core::ffi::c_uint & 0x7 as ::core::ffi::c_uint)
                    << 21 as ::core::ffi::c_int,
            ) as int32_t;
        (*eqtb.offset((LC_CODE_BASE as int32_t + k) as isize))
            .b32
            .s1 = k + 32 as int32_t;
        (*eqtb.offset((LC_CODE_BASE as int32_t + (k + 32 as int32_t)) as isize))
            .b32
            .s1 = k + 32 as int32_t;
        (*eqtb.offset((UC_CODE_BASE as int32_t + k) as isize))
            .b32
            .s1 = k;
        (*eqtb.offset((UC_CODE_BASE as int32_t + (k + 32 as int32_t)) as isize))
            .b32
            .s1 = k;
        (*eqtb.offset((SF_CODE_BASE as int32_t + k) as isize))
            .b32
            .s1 = 999 as ::core::ffi::c_int as int32_t;
        k += 1;
    }
    k = INT_BASE as int32_t;
    while k <= DEL_CODE_BASE as int32_t - 1 as int32_t {
        (*eqtb.offset(k as isize)).b32.s1 = 0 as ::core::ffi::c_int as int32_t;
        k += 1;
    }
    (*eqtb.offset((INT_BASE + INT_PAR__mag) as isize)).b32.s1 =
        1000 as ::core::ffi::c_int as int32_t;
    (*eqtb.offset((INT_BASE + INT_PAR__tolerance) as isize))
        .b32
        .s1 = 10000 as ::core::ffi::c_int as int32_t;
    (*eqtb.offset((INT_BASE + INT_PAR__hang_after) as isize))
        .b32
        .s1 = 1 as ::core::ffi::c_int as int32_t;
    (*eqtb.offset((INT_BASE + INT_PAR__max_dead_cycles) as isize))
        .b32
        .s1 = 25 as ::core::ffi::c_int as int32_t;
    (*eqtb.offset((INT_BASE + INT_PAR__escape_char) as isize))
        .b32
        .s1 = '\\' as i32 as int32_t;
    (*eqtb.offset((INT_BASE + INT_PAR__end_line_char) as isize))
        .b32
        .s1 = CARRIAGE_RETURN as int32_t;
    k = 0 as ::core::ffi::c_int as int32_t;
    while k <= NUMBER_USVS as int32_t - 1 as int32_t {
        (*eqtb.offset((DEL_CODE_BASE as int32_t + k) as isize))
            .b32
            .s1 = -(1 as ::core::ffi::c_int) as int32_t;
        k += 1;
    }
    (*eqtb.offset((DEL_CODE_BASE + 46 as ::core::ffi::c_int) as isize))
        .b32
        .s1 = 0 as ::core::ffi::c_int as int32_t;
    k = DIMEN_BASE as int32_t;
    while k <= EQTB_SIZE as int32_t {
        (*eqtb.offset(k as isize)).b32.s1 = 0 as ::core::ffi::c_int as int32_t;
        k += 1;
    }
    prim_used = PRIM_SIZE as int32_t;
    hash_used = FROZEN_CONTROL_SEQUENCE as int32_t;
    hash_high = 0 as ::core::ffi::c_int as int32_t;
    cs_count = 0 as ::core::ffi::c_int as int32_t;
    k = -(TRIE_OP_SIZE as int32_t);
    while k as ::core::ffi::c_long <= TRIE_OP_SIZE {
        _trie_op_hash_array[(k as ::core::ffi::c_long - NEG_TRIE_OP_SIZE) as usize] =
            0 as ::core::ffi::c_int as int32_t;
        k += 1;
    }
    k = 0 as ::core::ffi::c_int as int32_t;
    while k <= BIGGEST_LANG as int32_t {
        trie_used[k as usize] = MIN_TRIE_OP as trie_opcode;
        k += 1;
    }
    max_op_used = MIN_TRIE_OP as trie_opcode;
    trie_op_ptr = 0 as ::core::ffi::c_int as int32_t;
    trie_not_ready = true_0 != 0;
    format_ident =
        maketexstring(b" (INITEX)\0" as *const u8 as *const ::core::ffi::c_char) as str_number;
    max_reg_num = 32767 as ::core::ffi::c_int as int32_t;
    max_reg_help_line = b"A register number must be between 0 and 32767.\0" as *const u8
        as *const ::core::ffi::c_char;
    i = INT_VAL as int32_t;
    while i <= INTER_CHAR_VAL as int32_t {
        sa_root[i as usize] = TEX_NULL as int32_t;
        i += 1;
    }
    (*eqtb.offset((INT_BASE + INT_PAR__xetex_hyphenatable_length) as isize))
        .b32
        .s1 = 63 as ::core::ffi::c_int as int32_t;
    (*eqtb.offset((INT_BASE + INT_PAR__xetex_generate_actual_text) as isize))
        .b32
        .s1 = 1 as ::core::ffi::c_int as int32_t;
    // Pitex font expansion is opt-in; its capacities have useful defaults.
    (*eqtb.offset((INT_BASE + INT_PAR__pitex_font_stretch) as isize)).b32.s1 = 20;
    (*eqtb.offset((INT_BASE + INT_PAR__pitex_font_shrink) as isize)).b32.s1 = 20;
    (*eqtb.offset((INT_BASE + INT_PAR__pitex_font_step) as isize)).b32.s1 = 1;
}
unsafe fn pitex_register_parameter(parameter: &crate::backend_definitions::Parameter) {
    use crate::backend_definitions::Storage;
    let name=std::ffi::CString::new(parameter.name).unwrap();
    let hidden=std::ffi::CString::new(format!("PitexStorage{}",parameter.name)).unwrap();
    let command=match parameter.storage {Storage::Integer=>ASSIGN_INT,Storage::Dimension=>ASSIGN_DIMEN,Storage::Tokens=>ASSIGN_TOKS};
    let value=if parameter.storage==Storage::Tokens {TEX_NULL as i32}else{parameter.initial};
    primitive(hidden.as_ptr(),UNDEFINED_CS as uint16_t,value);
    let location=cur_val;
    primitive(name.as_ptr(),command as uint16_t,location);
}
unsafe fn pitex_register_alias(name:&str,target:&str) {
    let target=std::ffi::CString::new(target).unwrap();
    let string=maketexstring(target.as_ptr());let index=prim_lookup(string);str_ptr-=1;
    pool_ptr=*str_start.offset((str_ptr-TOO_BIG_CHAR as str_number) as isize);
    let definition=*eqtb.offset((PRIM_EQTB_BASE + index) as isize);
    let name=std::ffi::CString::new(name).unwrap();primitive(name.as_ptr(),definition.b16.s1,definition.b32.s1);
}
unsafe fn pitex_register_shared() {
    pitex_register_parameter(&crate::backend_definitions::Parameter {name:"PitexGlyphUnicodeMappings",storage:crate::backend_definitions::Storage::Tokens,initial:0});
    pitex_register_parameter(&crate::backend_definitions::Parameter {name:"PitexGlyphUnicodeTail",storage:crate::backend_definitions::Storage::Integer,initial:TEX_NULL as i32});
    pitex_register_parameter(&crate::backend_definitions::Parameter {name:"PitexGlyphUnicodeVersion",storage:crate::backend_definitions::Storage::Integer,initial:0});
    primitive(b"pdfglyphtounicode\0".as_ptr().cast(),EXTENSION as uint16_t,210);
    for parameter in crate::backend_definitions::PARAMETERS {pitex_register_parameter(parameter);}
    for parameter in crate::pdf_definitions::PARAMETERS {pitex_register_parameter(parameter);}
    for parameter in crate::font_definitions::PARAMETERS {pitex_register_parameter(parameter);}
    for &(name,target) in crate::backend_definitions::ALIASES {pitex_register_alias(name,target);}
    for &(name,kind,code) in crate::pdf_definitions::PRIMITIVES {
        let command=match kind {"extension"=>EXTENSION,"convert"=>CONVERT,"lastitem"=>LAST_ITEM,_=>panic!("unknown compatibility command")};
        let name=std::ffi::CString::new(name).unwrap();primitive(name.as_ptr(),command as uint16_t,code);
    }
    for &(name,kind,code) in crate::font_definitions::PRIMITIVES {
        let command=match kind {"extension"=>EXTENSION,"convert"=>CONVERT,"lastitem"=>LAST_ITEM,"fontinteger"=>ASSIGN_FONT_INT,_=>panic!("unknown font command")};
        let name=std::ffi::CString::new(name).unwrap();primitive(name.as_ptr(),command as uint16_t,code);
    }
    for &(name,kind,code) in crate::string_definitions::PRIMITIVES {
        let command=match kind {"extension"=>EXTENSION,"convert"=>CONVERT,"lastitem"=>LAST_ITEM,_=>panic!("unknown compatibility command")};
        let name=std::ffi::CString::new(name).unwrap();primitive(name.as_ptr(),command as uint16_t,code);
    }
}
unsafe extern "C" fn initialize_primitives() {
    static mut primitives: [xetex_format_primitive_def_t; 566] = [
        xetex_format_primitive_def_t {
            name: b"relax\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: RELAX as eight_bits,
            chr: TOO_BIG_USV as int32_t,
            extra_init: FROZEN_RELAX as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"span\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: TAB_MARK as eight_bits,
            chr: SPAN_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"cr\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: CAR_RET as eight_bits,
            chr: CR_CODE as int32_t,
            extra_init: FROZEN_CR as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"crcr\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: CAR_RET as eight_bits,
            chr: CR_CR_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"par\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: PAR_END as eight_bits,
            chr: TOO_BIG_USV as int32_t,
            extra_init: xf_prim_init_par as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"end\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: STOP as eight_bits,
            chr: 0 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"dump\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: STOP as eight_bits,
            chr: 1 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"delimiter\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: DELIM_NUM as eight_bits,
            chr: 0 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"Udelimiter\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: DELIM_NUM as eight_bits,
            chr: 1 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"XeTeXdelimiter\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: DELIM_NUM as eight_bits,
            chr: 1 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"char\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: CHAR_NUM as eight_bits,
            chr: 0 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"mathchar\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: MATH_CHAR_NUM as eight_bits,
            chr: 0 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"Umathcharnum\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: MATH_CHAR_NUM as eight_bits,
            chr: 1 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"XeTeXmathcharnum\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: MATH_CHAR_NUM as eight_bits,
            chr: 1 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"Umathchar\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: MATH_CHAR_NUM as eight_bits,
            chr: 2 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"XeTeXmathchar\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: MATH_CHAR_NUM as eight_bits,
            chr: 2 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"mark\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: MARK as eight_bits,
            chr: 0 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"marks\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: MARK as eight_bits,
            chr: 5 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"show\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: XRAY as eight_bits,
            chr: SHOW_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"showbox\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: XRAY as eight_bits,
            chr: SHOW_BOX_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"showthe\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: XRAY as eight_bits,
            chr: SHOW_THE_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"showlists\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: XRAY as eight_bits,
            chr: SHOW_LISTS as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"showgroups\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: XRAY as eight_bits,
            chr: SHOW_GROUPS as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"showtokens\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: XRAY as eight_bits,
            chr: SHOW_TOKENS as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"showifs\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: XRAY as eight_bits,
            chr: SHOW_IFS as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"box\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: MAKE_BOX as eight_bits,
            chr: BOX_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"copy\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: MAKE_BOX as eight_bits,
            chr: COPY_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"lastbox\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: MAKE_BOX as eight_bits,
            chr: LAST_BOX_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"vsplit\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: MAKE_BOX as eight_bits,
            chr: VSPLIT_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"vtop\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: MAKE_BOX as eight_bits,
            chr: VTOP_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"vbox\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: MAKE_BOX as eight_bits,
            chr: TT_VBOX_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"hbox\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: MAKE_BOX as eight_bits,
            chr: TT_HBOX_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"moveright\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: HMOVE as eight_bits,
            chr: 0 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"moveleft\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: HMOVE as eight_bits,
            chr: 1 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"lower\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: VMOVE as eight_bits,
            chr: 0 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"raise\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: VMOVE as eight_bits,
            chr: 1 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"unhbox\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: UN_HBOX as eight_bits,
            chr: BOX_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"unhcopy\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: UN_HBOX as eight_bits,
            chr: COPY_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"unvbox\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: UN_VBOX as eight_bits,
            chr: BOX_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"unvcopy\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: UN_VBOX as eight_bits,
            chr: COPY_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"pagediscards\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: UN_VBOX as eight_bits,
            chr: LAST_BOX_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"splitdiscards\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: UN_VBOX as eight_bits,
            chr: VSPLIT_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"unskip\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: REMOVE_ITEM as eight_bits,
            chr: GLUE_NODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"unkern\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: REMOVE_ITEM as eight_bits,
            chr: KERN_NODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"unpenalty\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: REMOVE_ITEM as eight_bits,
            chr: PENALTY_NODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"hfil\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: HSKIP as eight_bits,
            chr: FIL_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"hfill\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: HSKIP as eight_bits,
            chr: FILL_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"hss\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: HSKIP as eight_bits,
            chr: SS_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"hfilneg\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: HSKIP as eight_bits,
            chr: FIL_NEG_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"hskip\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: HSKIP as eight_bits,
            chr: SKIP_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"vfil\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: VSKIP as eight_bits,
            chr: FIL_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"vfill\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: VSKIP as eight_bits,
            chr: FILL_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"vss\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: VSKIP as eight_bits,
            chr: SS_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"vfilneg\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: VSKIP as eight_bits,
            chr: FIL_NEG_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"vskip\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: VSKIP as eight_bits,
            chr: SKIP_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"mskip\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: MSKIP as eight_bits,
            chr: MSKIP_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"kern\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: KERN as eight_bits,
            chr: EXPLICIT as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"mkern\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: MKERN as eight_bits,
            chr: MU_GLUE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"shipout\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: LEADER_SHIP as eight_bits,
            chr: MU_GLUE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"leaders\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: LEADER_SHIP as eight_bits,
            chr: A_LEADERS as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"cleaders\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: LEADER_SHIP as eight_bits,
            chr: C_LEADERS as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"xleaders\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: LEADER_SHIP as eight_bits,
            chr: X_LEADERS as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"halign\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: HALIGN as eight_bits,
            chr: 0 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"valign\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: VALIGN as eight_bits,
            chr: BEFORE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"beginL\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: VALIGN as eight_bits,
            chr: BEGIN_L_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"endL\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: VALIGN as eight_bits,
            chr: END_L_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"beginR\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: VALIGN as eight_bits,
            chr: BEGIN_R_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"endR\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: VALIGN as eight_bits,
            chr: END_R_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"noalign\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: NO_ALIGN as eight_bits,
            chr: 0 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"vrule\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: VRULE as eight_bits,
            chr: 0 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"hrule\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: HRULE as eight_bits,
            chr: 0 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"insert\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: INSERT as eight_bits,
            chr: 0 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"vadjust\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: VADJUST as eight_bits,
            chr: 0 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"ignorespaces\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: IGNORE_SPACES as eight_bits,
            chr: 0 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"afterassignment\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: AFTER_ASSIGNMENT as eight_bits,
            chr: 0 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"aftergroup\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: AFTER_GROUP as eight_bits,
            chr: 0 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"penalty\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: BREAK_PENALTY as eight_bits,
            chr: 0 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"noindent\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: START_PAR as eight_bits,
            chr: 0 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"indent\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: START_PAR as eight_bits,
            chr: 1 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"/\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ITAL_CORR as eight_bits,
            chr: 0 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"accent\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ACCENT as eight_bits,
            chr: 0 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"mathaccent\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: MATH_ACCENT as eight_bits,
            chr: 0 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"Umathaccent\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: MATH_ACCENT as eight_bits,
            chr: 1 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"XeTeXmathaccent\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: MATH_ACCENT as eight_bits,
            chr: 1 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"discretionary\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: DISCRETIONARY as eight_bits,
            chr: 0 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"-\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: DISCRETIONARY as eight_bits,
            chr: 1 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"eqno\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: EQ_NO as eight_bits,
            chr: 0 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"leqno\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: EQ_NO as eight_bits,
            chr: 1 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"middle\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: LEFT_RIGHT as eight_bits,
            chr: TT_LEFT_RIGHT_MIDDLE_MODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"left\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: LEFT_RIGHT as eight_bits,
            chr: LEFT_NOAD as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"right\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: LEFT_RIGHT as eight_bits,
            chr: RIGHT_NOAD as int32_t,
            extra_init: FROZEN_RIGHT as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"mathord\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: MATH_COMP as eight_bits,
            chr: ORD_NOAD as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"mathop\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: MATH_COMP as eight_bits,
            chr: OP_NOAD as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"mathbin\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: MATH_COMP as eight_bits,
            chr: BIN_NOAD as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"mathrel\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: MATH_COMP as eight_bits,
            chr: REL_NOAD as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"mathopen\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: MATH_COMP as eight_bits,
            chr: OPEN_NOAD as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"mathclose\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: MATH_COMP as eight_bits,
            chr: CLOSE_NOAD as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"mathpunct\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: MATH_COMP as eight_bits,
            chr: PUNCT_NOAD as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"mathinner\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: MATH_COMP as eight_bits,
            chr: INNER_NOAD as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"underline\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: MATH_COMP as eight_bits,
            chr: UNDER_NOAD as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"overline\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: MATH_COMP as eight_bits,
            chr: OVER_NOAD as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"displaylimits\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: LIMIT_SWITCH as eight_bits,
            chr: NORMAL as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"limits\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: LIMIT_SWITCH as eight_bits,
            chr: LIMITS as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"nolimits\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: LIMIT_SWITCH as eight_bits,
            chr: NO_LIMITS as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"above\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ABOVE as eight_bits,
            chr: ABOVE_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"over\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ABOVE as eight_bits,
            chr: OVER_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"atop\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ABOVE as eight_bits,
            chr: ATOP_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"abovewithdelims\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ABOVE as eight_bits,
            chr: TT_ABOVE_WITH_DELIMS as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"overwithdelims\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ABOVE as eight_bits,
            chr: TT_OVER_WITH_DELIMS as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"atopwithdelims\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ABOVE as eight_bits,
            chr: TT_ATOP_WITH_DELIMS as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"displaystyle\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: MATH_STYLE as eight_bits,
            chr: DISPLAY_STYLE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"textstyle\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: MATH_STYLE as eight_bits,
            chr: TEXT_STYLE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"scriptstyle\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: MATH_STYLE as eight_bits,
            chr: SCRIPT_STYLE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"scriptscriptstyle\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: MATH_STYLE as eight_bits,
            chr: SCRIPT_SCRIPT_STYLE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"mathchoice\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: MATH_CHOICE as eight_bits,
            chr: 0 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"nonscript\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: NON_SCRIPT as eight_bits,
            chr: 0 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"vcenter\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: VCENTER as eight_bits,
            chr: 0 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"lowercase\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: CASE_SHIFT as eight_bits,
            chr: LC_CODE_BASE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"uppercase\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: CASE_SHIFT as eight_bits,
            chr: UC_CODE_BASE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"message\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: MESSAGE as eight_bits,
            chr: 0 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"errmessage\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: MESSAGE as eight_bits,
            chr: 1 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"openout\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: EXTENSION as eight_bits,
            chr: OPEN_NODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"write\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: EXTENSION as eight_bits,
            chr: WRITE_NODE as int32_t,
            extra_init: xf_prim_init_write as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"closeout\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: EXTENSION as eight_bits,
            chr: CLOSE_NODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"special\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: EXTENSION as eight_bits,
            chr: SPECIAL_NODE as int32_t,
            extra_init: FROZEN_SPECIAL as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"immediate\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: EXTENSION as eight_bits,
            chr: IMMEDIATE_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"setlanguage\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: EXTENSION as eight_bits,
            chr: SET_LANGUAGE_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"pdfsavepos\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: EXTENSION as eight_bits,
            chr: PDF_SAVE_POS_NODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"resettimer\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: EXTENSION as eight_bits,
            chr: RESET_TIMER_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"setrandomseed\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: EXTENSION as eight_bits,
            chr: SET_RANDOM_SEED_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"XeTeXpicfile\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: EXTENSION as eight_bits,
            chr: PIC_FILE_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"XeTeXpdffile\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: EXTENSION as eight_bits,
            chr: PDF_FILE_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"XeTeXglyph\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: EXTENSION as eight_bits,
            chr: GLYPH_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"XeTeXinputencoding\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: EXTENSION as eight_bits,
            chr: XETEX_INPUT_ENCODING_EXTENSION_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"XeTeXdefaultencoding\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: EXTENSION as eight_bits,
            chr: XETEX_DEFAULT_ENCODING_EXTENSION_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"XeTeXlinebreaklocale\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: EXTENSION as eight_bits,
            chr: XETEX_LINEBREAK_LOCALE_EXTENSION_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"closein\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: IN_STREAM as eight_bits,
            chr: 0 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"openin\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: IN_STREAM as eight_bits,
            chr: 1 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"begingroup\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: BEGIN_GROUP as eight_bits,
            chr: 0 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"endgroup\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: END_GROUP as eight_bits,
            chr: 0 as int32_t,
            extra_init: FROZEN_END_GROUP as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"omit\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: OMIT as eight_bits,
            chr: 0 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b" \0" as *const u8 as *const ::core::ffi::c_char,
            cmd: EX_SPACE as eight_bits,
            chr: 0 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"noboundary\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: NO_BOUNDARY as eight_bits,
            chr: 0 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"radical\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: RADICAL as eight_bits,
            chr: 0 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"Uradical\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: RADICAL as eight_bits,
            chr: 1 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"XeTeXradical\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: RADICAL as eight_bits,
            chr: 1 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"endcsname\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: END_CS_NAME as eight_bits,
            chr: 0 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"lastpenalty\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: LAST_ITEM as eight_bits,
            chr: INT_VAL as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"lastkern\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: LAST_ITEM as eight_bits,
            chr: DIMEN_VAL as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"lastskip\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: LAST_ITEM as eight_bits,
            chr: GLUE_VAL as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"lastnodetype\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: LAST_ITEM as eight_bits,
            chr: LAST_NODE_TYPE_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"inputlineno\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: LAST_ITEM as eight_bits,
            chr: INPUT_LINE_NO_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"badness\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: LAST_ITEM as eight_bits,
            chr: BADNESS_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"pdflastxpos\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: LAST_ITEM as eight_bits,
            chr: PDF_LAST_X_POS_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"pdflastypos\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: LAST_ITEM as eight_bits,
            chr: PDF_LAST_Y_POS_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"elapsedtime\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: LAST_ITEM as eight_bits,
            chr: ELAPSED_TIME_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"shellescape\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: LAST_ITEM as eight_bits,
            chr: PDF_SHELL_ESCAPE_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"randomseed\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: LAST_ITEM as eight_bits,
            chr: RANDOM_SEED_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"eTeXversion\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: LAST_ITEM as eight_bits,
            chr: ETEX_VERSION_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"currentgrouplevel\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: LAST_ITEM as eight_bits,
            chr: CURRENT_GROUP_LEVEL_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"currentgrouptype\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: LAST_ITEM as eight_bits,
            chr: CURRENT_GROUP_TYPE_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"currentiflevel\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: LAST_ITEM as eight_bits,
            chr: CURRENT_IF_LEVEL_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"currentiftype\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: LAST_ITEM as eight_bits,
            chr: CURRENT_IF_TYPE_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"currentifbranch\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: LAST_ITEM as eight_bits,
            chr: CURRENT_IF_BRANCH_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"gluestretchorder\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: LAST_ITEM as eight_bits,
            chr: GLUE_STRETCH_ORDER_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"glueshrinkorder\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: LAST_ITEM as eight_bits,
            chr: GLUE_SHRINK_ORDER_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"XeTeXversion\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: LAST_ITEM as eight_bits,
            chr: XETEX_VERSION_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"XeTeXcountglyphs\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: LAST_ITEM as eight_bits,
            chr: XETEX_COUNT_GLYPHS_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"XeTeXcountvariations\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: LAST_ITEM as eight_bits,
            chr: XETEX_COUNT_VARIATIONS_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"XeTeXvariation\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: LAST_ITEM as eight_bits,
            chr: XETEX_VARIATION_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"XeTeXfindvariationbyname\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: LAST_ITEM as eight_bits,
            chr: XETEX_FIND_VARIATION_BY_NAME_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"XeTeXvariationmin\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: LAST_ITEM as eight_bits,
            chr: XETEX_VARIATION_MIN_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"XeTeXvariationmax\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: LAST_ITEM as eight_bits,
            chr: XETEX_VARIATION_MAX_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"XeTeXvariationdefault\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: LAST_ITEM as eight_bits,
            chr: XETEX_VARIATION_DEFAULT_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"XeTeXcountfeatures\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: LAST_ITEM as eight_bits,
            chr: XETEX_COUNT_FEATURES_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"XeTeXfeaturecode\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: LAST_ITEM as eight_bits,
            chr: XETEX_FEATURE_CODE_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"XeTeXfindfeaturebyname\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: LAST_ITEM as eight_bits,
            chr: XETEX_FIND_FEATURE_BY_NAME_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"XeTeXisexclusivefeature\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: LAST_ITEM as eight_bits,
            chr: XETEX_IS_EXCLUSIVE_FEATURE_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"XeTeXcountselectors\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: LAST_ITEM as eight_bits,
            chr: XETEX_COUNT_SELECTORS_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"XeTeXselectorcode\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: LAST_ITEM as eight_bits,
            chr: XETEX_SELECTOR_CODE_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"XeTeXfindselectorbyname\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: LAST_ITEM as eight_bits,
            chr: XETEX_FIND_SELECTOR_BY_NAME_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"XeTeXisdefaultselector\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: LAST_ITEM as eight_bits,
            chr: XETEX_IS_DEFAULT_SELECTOR_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"XeTeXOTcountscripts\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: LAST_ITEM as eight_bits,
            chr: XETEX_OT_COUNT_SCRIPTS_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"XeTeXOTcountlanguages\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: LAST_ITEM as eight_bits,
            chr: XETEX_OT_COUNT_LANGUAGES_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"XeTeXOTcountfeatures\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: LAST_ITEM as eight_bits,
            chr: XETEX_OT_COUNT_FEATURES_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"XeTeXOTscripttag\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: LAST_ITEM as eight_bits,
            chr: XETEX_OT_SCRIPT_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"XeTeXOTlanguagetag\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: LAST_ITEM as eight_bits,
            chr: XETEX_OT_LANGUAGE_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"XeTeXOTfeaturetag\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: LAST_ITEM as eight_bits,
            chr: XETEX_OT_FEATURE_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"XeTeXcharglyph\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: LAST_ITEM as eight_bits,
            chr: XETEX_MAP_CHAR_TO_GLYPH_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"XeTeXglyphindex\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: LAST_ITEM as eight_bits,
            chr: XETEX_GLYPH_INDEX_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"XeTeXfonttype\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: LAST_ITEM as eight_bits,
            chr: XETEX_FONT_TYPE_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"XeTeXfirstfontchar\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: LAST_ITEM as eight_bits,
            chr: XETEX_FIRST_CHAR_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"XeTeXlastfontchar\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: LAST_ITEM as eight_bits,
            chr: XETEX_LAST_CHAR_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"XeTeXpdfpagecount\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: LAST_ITEM as eight_bits,
            chr: XETEX_PDF_PAGE_COUNT_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"XeTeXglyphbounds\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: LAST_ITEM as eight_bits,
            chr: XETEX_GLYPH_BOUNDS_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"fontcharwd\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: LAST_ITEM as eight_bits,
            chr: FONT_CHAR_WD_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"fontcharht\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: LAST_ITEM as eight_bits,
            chr: FONT_CHAR_HT_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"fontchardp\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: LAST_ITEM as eight_bits,
            chr: FONT_CHAR_DP_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"fontcharic\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: LAST_ITEM as eight_bits,
            chr: FONT_CHAR_IC_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"parshapelength\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: LAST_ITEM as eight_bits,
            chr: PAR_SHAPE_LENGTH_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"parshapeindent\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: LAST_ITEM as eight_bits,
            chr: PAR_SHAPE_INDENT_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"parshapedimen\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: LAST_ITEM as eight_bits,
            chr: PAR_SHAPE_DIMEN_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"gluestretch\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: LAST_ITEM as eight_bits,
            chr: GLUE_STRETCH_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"glueshrink\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: LAST_ITEM as eight_bits,
            chr: GLUE_SHRINK_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"mutoglue\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: LAST_ITEM as eight_bits,
            chr: MU_TO_GLUE_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"gluetomu\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: LAST_ITEM as eight_bits,
            chr: GLUE_TO_MU_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"numexpr\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: LAST_ITEM as eight_bits,
            chr: TT_ETEX_NUM_EXPR_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"dimexpr\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: LAST_ITEM as eight_bits,
            chr: TT_ETEX_DIM_EXPR_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"glueexpr\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: LAST_ITEM as eight_bits,
            chr: TT_ETEX_GLUE_EXPR_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"muexpr\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: LAST_ITEM as eight_bits,
            chr: TT_ETEX_MU_EXPR_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"toks\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: TOKS_REGISTER as eight_bits,
            chr: 0 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"fontdimen\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_FONT_DIMEN as eight_bits,
            chr: 0 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"hyphenchar\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_FONT_INT as eight_bits,
            chr: 0 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"skewchar\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_FONT_INT as eight_bits,
            chr: 1 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"lpcode\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_FONT_INT as eight_bits,
            chr: 2 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"rpcode\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_FONT_INT as eight_bits,
            chr: 3 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"prevdepth\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: SET_AUX as eight_bits,
            chr: VMODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"spacefactor\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: SET_AUX as eight_bits,
            chr: HMODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"prevgraf\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: SET_PREV_GRAF as eight_bits,
            chr: 0 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"pagegoal\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: SET_PAGE_DIMEN as eight_bits,
            chr: 0 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"pagetotal\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: SET_PAGE_DIMEN as eight_bits,
            chr: 1 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"pagestretch\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: SET_PAGE_DIMEN as eight_bits,
            chr: 2 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"pagefilstretch\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: SET_PAGE_DIMEN as eight_bits,
            chr: 3 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"pagefillstretch\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: SET_PAGE_DIMEN as eight_bits,
            chr: 4 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"pagefilllstretch\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: SET_PAGE_DIMEN as eight_bits,
            chr: 5 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"pageshrink\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: SET_PAGE_DIMEN as eight_bits,
            chr: 6 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"pagedepth\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: SET_PAGE_DIMEN as eight_bits,
            chr: 7 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"deadcycles\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: SET_PAGE_INT as eight_bits,
            chr: 0 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"insertpenalties\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: SET_PAGE_INT as eight_bits,
            chr: 1 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"interactionmode\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: SET_PAGE_INT as eight_bits,
            chr: 2 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"wd\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: SET_BOX_DIMEN as eight_bits,
            chr: WIDTH_OFFSET as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"dp\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: SET_BOX_DIMEN as eight_bits,
            chr: DEPTH_OFFSET as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"ht\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: SET_BOX_DIMEN as eight_bits,
            chr: HEIGHT_OFFSET as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"catcode\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: DEF_CODE as eight_bits,
            chr: CAT_CODE_BASE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"lccode\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: DEF_CODE as eight_bits,
            chr: LC_CODE_BASE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"uccode\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: DEF_CODE as eight_bits,
            chr: UC_CODE_BASE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"sfcode\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: DEF_CODE as eight_bits,
            chr: SF_CODE_BASE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"mathcode\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: DEF_CODE as eight_bits,
            chr: MATH_CODE_BASE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"delcode\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: DEF_CODE as eight_bits,
            chr: DEL_CODE_BASE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"XeTeXcharclass\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: XETEX_DEF_CODE as eight_bits,
            chr: SF_CODE_BASE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"Umathcodenum\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: XETEX_DEF_CODE as eight_bits,
            chr: MATH_CODE_BASE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"XeTeXmathcodenum\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: XETEX_DEF_CODE as eight_bits,
            chr: MATH_CODE_BASE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"Umathcode\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: XETEX_DEF_CODE as eight_bits,
            chr: MATH_CODE_BASE as int32_t + 1 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"XeTeXmathcode\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: XETEX_DEF_CODE as eight_bits,
            chr: MATH_CODE_BASE as int32_t + 1 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"Udelcodenum\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: XETEX_DEF_CODE as eight_bits,
            chr: DEL_CODE_BASE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"XeTeXdelcodenum\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: XETEX_DEF_CODE as eight_bits,
            chr: DEL_CODE_BASE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"Udelcode\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: XETEX_DEF_CODE as eight_bits,
            chr: DEL_CODE_BASE as int32_t + 1 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"XeTeXdelcode\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: XETEX_DEF_CODE as eight_bits,
            chr: DEL_CODE_BASE as int32_t + 1 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"textfont\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: DEF_FAMILY as eight_bits,
            chr: 2255401 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"scriptfont\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: DEF_FAMILY as eight_bits,
            chr: 2255657 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"scriptscriptfont\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: DEF_FAMILY as eight_bits,
            chr: 2255913 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"nullfont\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: SET_FONT as eight_bits,
            chr: 0 as int32_t,
            extra_init: FROZEN_NULL_FONT as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"font\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: DEF_FONT as eight_bits,
            chr: 0 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"count\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: REGISTER as eight_bits,
            chr: 0 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"dimen\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: REGISTER as eight_bits,
            chr: 1 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"skip\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: REGISTER as eight_bits,
            chr: 2 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"muskip\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: REGISTER as eight_bits,
            chr: 3 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"advance\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ADVANCE as eight_bits,
            chr: 0 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"multiply\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: MULTIPLY as eight_bits,
            chr: 0 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"divide\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: DIVIDE as eight_bits,
            chr: 0 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"long\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: PREFIX as eight_bits,
            chr: 1 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"outer\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: PREFIX as eight_bits,
            chr: 2 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"global\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: PREFIX as eight_bits,
            chr: 4 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"protected\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: PREFIX as eight_bits,
            chr: 8 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"let\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: LET as eight_bits,
            chr: 0 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"futurelet\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: LET as eight_bits,
            chr: 1 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"chardef\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: SHORTHAND_DEF as eight_bits,
            chr: CHAR_DEF_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"mathchardef\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: SHORTHAND_DEF as eight_bits,
            chr: MATH_CHAR_DEF_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"countdef\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: SHORTHAND_DEF as eight_bits,
            chr: COUNT_DEF_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"dimendef\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: SHORTHAND_DEF as eight_bits,
            chr: DIMEN_DEF_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"skipdef\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: SHORTHAND_DEF as eight_bits,
            chr: SKIP_DEF_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"muskipdef\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: SHORTHAND_DEF as eight_bits,
            chr: MU_SKIP_DEF_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"toksdef\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: SHORTHAND_DEF as eight_bits,
            chr: TOKS_DEF_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"Umathcharnumdef\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: SHORTHAND_DEF as eight_bits,
            chr: XETEX_MATH_CHAR_NUM_DEF_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"XeTeXmathcharnumdef\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: SHORTHAND_DEF as eight_bits,
            chr: XETEX_MATH_CHAR_NUM_DEF_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"Umathchardef\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: SHORTHAND_DEF as eight_bits,
            chr: XETEX_MATH_CHAR_DEF_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"XeTeXmathchardef\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: SHORTHAND_DEF as eight_bits,
            chr: XETEX_MATH_CHAR_DEF_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"read\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: READ_TO_CS as eight_bits,
            chr: 0 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"readline\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: READ_TO_CS as eight_bits,
            chr: 1 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"def\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: DEF as eight_bits,
            chr: 0 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"gdef\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: DEF as eight_bits,
            chr: 1 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"edef\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: DEF as eight_bits,
            chr: 2 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"xdef\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: DEF as eight_bits,
            chr: 3 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"setbox\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: SET_BOX as eight_bits,
            chr: 0 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"hyphenation\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: HYPH_DATA as eight_bits,
            chr: 0 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"patterns\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: HYPH_DATA as eight_bits,
            chr: 1 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"batchmode\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: SET_INTERACTION as eight_bits,
            chr: BATCH_MODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"nonstopmode\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: SET_INTERACTION as eight_bits,
            chr: NONSTOP_MODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"scrollmode\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: SET_INTERACTION as eight_bits,
            chr: SCROLL_MODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"errorstopmode\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: SET_INTERACTION as eight_bits,
            chr: ERROR_STOP_MODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"expandafter\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: EXPAND_AFTER as eight_bits,
            chr: 0 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"unless\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: EXPAND_AFTER as eight_bits,
            chr: 1 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"noexpand\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: NO_EXPAND as eight_bits,
            chr: 0 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"primitive\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: NO_EXPAND as eight_bits,
            chr: 1 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"input\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: INPUT as eight_bits,
            chr: 0 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"endinput\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: INPUT as eight_bits,
            chr: 1 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"scantokens\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: INPUT as eight_bits,
            chr: 2 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"if\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: IF_TEST as eight_bits,
            chr: IF_CHAR_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"ifcat\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: IF_TEST as eight_bits,
            chr: IF_CAT_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"ifnum\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: IF_TEST as eight_bits,
            chr: IF_INT_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"ifdim\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: IF_TEST as eight_bits,
            chr: IF_DIM_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"ifodd\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: IF_TEST as eight_bits,
            chr: IF_ODD_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"ifvmode\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: IF_TEST as eight_bits,
            chr: IF_VMODE_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"ifhmode\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: IF_TEST as eight_bits,
            chr: IF_HMODE_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"ifmmode\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: IF_TEST as eight_bits,
            chr: IF_MMODE_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"ifinner\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: IF_TEST as eight_bits,
            chr: IF_INNER_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"ifvoid\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: IF_TEST as eight_bits,
            chr: IF_VOID_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"ifhbox\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: IF_TEST as eight_bits,
            chr: IF_HBOX_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"ifvbox\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: IF_TEST as eight_bits,
            chr: IF_VBOX_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"ifx\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: IF_TEST as eight_bits,
            chr: IFX_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"ifeof\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: IF_TEST as eight_bits,
            chr: IF_EOF_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"iftrue\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: IF_TEST as eight_bits,
            chr: IF_TRUE_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"iffalse\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: IF_TEST as eight_bits,
            chr: IF_FALSE_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"ifcase\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: IF_TEST as eight_bits,
            chr: IF_CASE_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"ifdefined\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: IF_TEST as eight_bits,
            chr: IF_DEF_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"ifcsname\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: IF_TEST as eight_bits,
            chr: IF_CS_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"iffontchar\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: IF_TEST as eight_bits,
            chr: IF_FONT_CHAR_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"ifincsname\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: IF_TEST as eight_bits,
            chr: IF_IN_CSNAME_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"ifprimitive\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: IF_TEST as eight_bits,
            chr: IF_PRIMITIVE_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"fi\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: FI_OR_ELSE as eight_bits,
            chr: FI_CODE as int32_t,
            extra_init: FROZEN_FI as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"else\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: FI_OR_ELSE as eight_bits,
            chr: ELSE_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"or\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: FI_OR_ELSE as eight_bits,
            chr: OR_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"csname\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: CS_NAME as eight_bits,
            chr: 0 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"number\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: CONVERT as eight_bits,
            chr: NUMBER_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"romannumeral\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: CONVERT as eight_bits,
            chr: ROMAN_NUMERAL_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"string\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: CONVERT as eight_bits,
            chr: STRING_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"meaning\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: CONVERT as eight_bits,
            chr: MEANING_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"fontname\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: CONVERT as eight_bits,
            chr: FONT_NAME_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"eTeXrevision\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: CONVERT as eight_bits,
            chr: ETEX_REVISION_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"expanded\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: CONVERT as eight_bits,
            chr: EXPANDED_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"leftmarginkern\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: CONVERT as eight_bits,
            chr: LEFT_MARGIN_KERN_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"rightmarginkern\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: CONVERT as eight_bits,
            chr: RIGHT_MARGIN_KERN_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"strcmp\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: CONVERT as eight_bits,
            chr: PDF_STRCMP_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"creationdate\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: CONVERT as eight_bits,
            chr: PDF_CREATION_DATE_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"filemoddate\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: CONVERT as eight_bits,
            chr: PDF_FILE_MOD_DATE_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"filesize\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: CONVERT as eight_bits,
            chr: PDF_FILE_SIZE_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"mdfivesum\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: CONVERT as eight_bits,
            chr: PDF_MDFIVE_SUM_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"filedump\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: CONVERT as eight_bits,
            chr: PDF_FILE_DUMP_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"uniformdeviate\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: CONVERT as eight_bits,
            chr: UNIFORM_DEVIATE_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"normaldeviate\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: CONVERT as eight_bits,
            chr: NORMAL_DEVIATE_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"XeTeXrevision\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: CONVERT as eight_bits,
            chr: XETEX_REVISION_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"XeTeXvariationname\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: CONVERT as eight_bits,
            chr: XETEX_VARIATION_NAME_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"XeTeXfeaturename\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: CONVERT as eight_bits,
            chr: XETEX_FEATURE_NAME_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"XeTeXselectorname\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: CONVERT as eight_bits,
            chr: XETEX_SELECTOR_NAME_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"XeTeXglyphname\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: CONVERT as eight_bits,
            chr: XETEX_GLYPH_NAME_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"Uchar\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: CONVERT as eight_bits,
            chr: XETEX_UCHAR_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"Ucharcat\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: CONVERT as eight_bits,
            chr: XETEX_UCHARCAT_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"jobname\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: CONVERT as eight_bits,
            chr: JOB_NAME_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"the\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: THE as eight_bits,
            chr: SHOW_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"unexpanded\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: THE as eight_bits,
            chr: SHOW_BOX_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"detokenize\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: THE as eight_bits,
            chr: SHOW_TOKENS as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"topmark\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: TOP_BOT_MARK as eight_bits,
            chr: TOP_MARK_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"firstmark\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: TOP_BOT_MARK as eight_bits,
            chr: FIRST_MARK_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"botmark\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: TOP_BOT_MARK as eight_bits,
            chr: BOT_MARK_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"splitfirstmark\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: TOP_BOT_MARK as eight_bits,
            chr: SPLIT_FIRST_MARK_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"splitbotmark\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: TOP_BOT_MARK as eight_bits,
            chr: SPLIT_BOT_MARK_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"topmarks\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: TOP_BOT_MARK as eight_bits,
            chr: TT_TOP_MARKS_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"firstmarks\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: TOP_BOT_MARK as eight_bits,
            chr: TT_FIRST_MARKS_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"botmarks\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: TOP_BOT_MARK as eight_bits,
            chr: TT_BOT_MARKS_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"splitfirstmarks\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: TOP_BOT_MARK as eight_bits,
            chr: TT_SPLIT_FIRST_MARKS_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"splitbotmarks\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: TOP_BOT_MARK as eight_bits,
            chr: TT_SPLIT_BOT_MARKS_CODE as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"pretolerance\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_INT as eight_bits,
            chr: INT_BASE as int32_t + INT_PAR__pretolerance as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"tolerance\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_INT as eight_bits,
            chr: INT_BASE as int32_t + INT_PAR__tolerance as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"linepenalty\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_INT as eight_bits,
            chr: INT_BASE as int32_t + INT_PAR__line_penalty as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"hyphenpenalty\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_INT as eight_bits,
            chr: INT_BASE as int32_t + INT_PAR__hyphen_penalty as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"exhyphenpenalty\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_INT as eight_bits,
            chr: INT_BASE as int32_t + INT_PAR__ex_hyphen_penalty as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"clubpenalty\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_INT as eight_bits,
            chr: INT_BASE as int32_t + INT_PAR__club_penalty as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"widowpenalty\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_INT as eight_bits,
            chr: INT_BASE as int32_t + INT_PAR__widow_penalty as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"displaywidowpenalty\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_INT as eight_bits,
            chr: INT_BASE as int32_t + INT_PAR__display_widow_penalty as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"brokenpenalty\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_INT as eight_bits,
            chr: INT_BASE as int32_t + INT_PAR__broken_penalty as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"binoppenalty\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_INT as eight_bits,
            chr: INT_BASE as int32_t + INT_PAR__bin_op_penalty as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"relpenalty\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_INT as eight_bits,
            chr: INT_BASE as int32_t + INT_PAR__rel_penalty as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"predisplaypenalty\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_INT as eight_bits,
            chr: INT_BASE as int32_t + INT_PAR__pre_display_penalty as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"postdisplaypenalty\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_INT as eight_bits,
            chr: INT_BASE as int32_t + INT_PAR__post_display_penalty as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"interlinepenalty\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_INT as eight_bits,
            chr: INT_BASE as int32_t + INT_PAR__inter_line_penalty as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"doublehyphendemerits\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_INT as eight_bits,
            chr: INT_BASE as int32_t + INT_PAR__double_hyphen_demerits as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"finalhyphendemerits\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_INT as eight_bits,
            chr: INT_BASE as int32_t + INT_PAR__final_hyphen_demerits as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"adjdemerits\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_INT as eight_bits,
            chr: INT_BASE as int32_t + INT_PAR__adj_demerits as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"mag\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_INT as eight_bits,
            chr: INT_BASE as int32_t + INT_PAR__mag as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"delimiterfactor\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_INT as eight_bits,
            chr: INT_BASE as int32_t + INT_PAR__delimiter_factor as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"looseness\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_INT as eight_bits,
            chr: INT_BASE as int32_t + INT_PAR__looseness as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"time\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_INT as eight_bits,
            chr: INT_BASE as int32_t + INT_PAR__time as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"day\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_INT as eight_bits,
            chr: INT_BASE as int32_t + INT_PAR__day as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"month\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_INT as eight_bits,
            chr: INT_BASE as int32_t + INT_PAR__month as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"year\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_INT as eight_bits,
            chr: INT_BASE as int32_t + INT_PAR__year as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"showboxbreadth\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_INT as eight_bits,
            chr: INT_BASE as int32_t + INT_PAR__show_box_breadth as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"showboxdepth\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_INT as eight_bits,
            chr: INT_BASE as int32_t + INT_PAR__show_box_depth as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"hbadness\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_INT as eight_bits,
            chr: INT_BASE as int32_t + INT_PAR__hbadness as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"vbadness\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_INT as eight_bits,
            chr: INT_BASE as int32_t + INT_PAR__vbadness as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"pausing\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_INT as eight_bits,
            chr: INT_BASE as int32_t + INT_PAR__pausing as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"tracingonline\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_INT as eight_bits,
            chr: INT_BASE as int32_t + INT_PAR__tracing_online as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"tracingmacros\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_INT as eight_bits,
            chr: INT_BASE as int32_t + INT_PAR__tracing_macros as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"tracingstats\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_INT as eight_bits,
            chr: INT_BASE as int32_t + INT_PAR__tracing_stats as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"tracingparagraphs\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_INT as eight_bits,
            chr: INT_BASE as int32_t + INT_PAR__tracing_paragraphs as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"tracingpages\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_INT as eight_bits,
            chr: INT_BASE as int32_t + INT_PAR__tracing_pages as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"tracingoutput\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_INT as eight_bits,
            chr: INT_BASE as int32_t + INT_PAR__tracing_output as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"tracinglostchars\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_INT as eight_bits,
            chr: INT_BASE as int32_t + INT_PAR__tracing_lost_chars as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"tracingcommands\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_INT as eight_bits,
            chr: INT_BASE as int32_t + INT_PAR__tracing_commands as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"tracingrestores\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_INT as eight_bits,
            chr: INT_BASE as int32_t + INT_PAR__tracing_restores as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"uchyph\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_INT as eight_bits,
            chr: INT_BASE as int32_t + INT_PAR__uc_hyph as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"outputpenalty\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_INT as eight_bits,
            chr: INT_BASE as int32_t + INT_PAR__output_penalty as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"maxdeadcycles\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_INT as eight_bits,
            chr: INT_BASE as int32_t + INT_PAR__max_dead_cycles as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"hangafter\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_INT as eight_bits,
            chr: INT_BASE as int32_t + INT_PAR__hang_after as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"floatingpenalty\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_INT as eight_bits,
            chr: INT_BASE as int32_t + INT_PAR__floating_penalty as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"globaldefs\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_INT as eight_bits,
            chr: INT_BASE as int32_t + INT_PAR__global_defs as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"fam\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_INT as eight_bits,
            chr: INT_BASE as int32_t + INT_PAR__cur_fam as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"escapechar\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_INT as eight_bits,
            chr: INT_BASE as int32_t + INT_PAR__escape_char as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"defaulthyphenchar\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_INT as eight_bits,
            chr: INT_BASE as int32_t + INT_PAR__default_hyphen_char as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"defaultskewchar\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_INT as eight_bits,
            chr: INT_BASE as int32_t + INT_PAR__default_skew_char as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"endlinechar\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_INT as eight_bits,
            chr: INT_BASE as int32_t + INT_PAR__end_line_char as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"newlinechar\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_INT as eight_bits,
            chr: INT_BASE as int32_t + INT_PAR__new_line_char as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"language\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_INT as eight_bits,
            chr: INT_BASE as int32_t + INT_PAR__language as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"lefthyphenmin\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_INT as eight_bits,
            chr: INT_BASE as int32_t + INT_PAR__left_hyphen_min as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"righthyphenmin\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_INT as eight_bits,
            chr: INT_BASE as int32_t + INT_PAR__right_hyphen_min as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"holdinginserts\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_INT as eight_bits,
            chr: INT_BASE as int32_t + INT_PAR__holding_inserts as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"errorcontextlines\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_INT as eight_bits,
            chr: INT_BASE as int32_t + INT_PAR__error_context_lines as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"tracingstacklevels\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_INT as eight_bits,
            chr: INT_BASE as int32_t + INT_PAR__tracing_stack_levels as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"tracingassigns\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_INT as eight_bits,
            chr: INT_BASE as int32_t + INT_PAR__tracing_assigns as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"tracinggroups\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_INT as eight_bits,
            chr: INT_BASE as int32_t + INT_PAR__tracing_groups as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"tracingifs\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_INT as eight_bits,
            chr: INT_BASE as int32_t + INT_PAR__tracing_ifs as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"tracingscantokens\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_INT as eight_bits,
            chr: INT_BASE as int32_t + INT_PAR__tracing_scan_tokens as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"tracingnesting\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_INT as eight_bits,
            chr: INT_BASE as int32_t + INT_PAR__tracing_nesting as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"predisplaydirection\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_INT as eight_bits,
            chr: INT_BASE as int32_t + INT_PAR__pre_display_direction as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"lastlinefit\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_INT as eight_bits,
            chr: INT_BASE as int32_t + INT_PAR__last_line_fit as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"savingvdiscards\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_INT as eight_bits,
            chr: INT_BASE as int32_t + INT_PAR__saving_vdiscards as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"savinghyphcodes\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_INT as eight_bits,
            chr: INT_BASE as int32_t + INT_PAR__saving_hyph_codes as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"suppressfontnotfounderror\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_INT as eight_bits,
            chr: INT_BASE as int32_t + INT_PAR__suppress_fontnotfound_error as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"XeTeXlinebreakpenalty\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_INT as eight_bits,
            chr: INT_BASE as int32_t + INT_PAR__xetex_linebreak_penalty as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"XeTeXprotrudechars\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_INT as eight_bits,
            chr: INT_BASE as int32_t + INT_PAR__xetex_protrude_chars as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"TeXXeTstate\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_INT as eight_bits,
            chr: INT_BASE as int32_t + INT_PAR__texxet as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"XeTeXdashbreakstate\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_INT as eight_bits,
            chr: INT_BASE as int32_t + INT_PAR__xetex_dash_break as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"XeTeXupwardsmode\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_INT as eight_bits,
            chr: INT_BASE as int32_t + INT_PAR__xetex_upwards as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"XeTeXuseglyphmetrics\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_INT as eight_bits,
            chr: INT_BASE as int32_t + INT_PAR__xetex_use_glyph_metrics as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"XeTeXinterchartokenstate\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_INT as eight_bits,
            chr: INT_BASE as int32_t + INT_PAR__xetex_inter_char_tokens as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"XeTeXinputnormalization\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_INT as eight_bits,
            chr: INT_BASE as int32_t + INT_PAR__xetex_input_normalization as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"XeTeXtracingfonts\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_INT as eight_bits,
            chr: INT_BASE as int32_t + INT_PAR__xetex_tracing_fonts as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"XeTeXinterwordspaceshaping\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_INT as eight_bits,
            chr: INT_BASE as int32_t + INT_PAR__xetex_interword_space_shaping as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"XeTeXgenerateactualtext\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_INT as eight_bits,
            chr: INT_BASE as int32_t + INT_PAR__xetex_generate_actual_text as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"XeTeXhyphenatablelength\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_INT as eight_bits,
            chr: INT_BASE as int32_t + INT_PAR__xetex_hyphenatable_length as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"synctex\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_INT as eight_bits,
            chr: INT_BASE as int32_t + INT_PAR__synctex as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"pdfoutput\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_INT as eight_bits,
            chr: INT_BASE as int32_t + INT_PAR__pdfoutput as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"partokencontext\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_INT as eight_bits,
            chr: INT_BASE as int32_t + INT_PAR__partoken_context as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"partokenname\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: AFTER_GROUP as eight_bits,
            chr: 1 as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"parindent\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_DIMEN as eight_bits,
            chr: DIMEN_BASE as int32_t + DIMEN_PAR__par_indent as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"mathsurround\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_DIMEN as eight_bits,
            chr: DIMEN_BASE as int32_t + DIMEN_PAR__math_surround as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"lineskiplimit\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_DIMEN as eight_bits,
            chr: DIMEN_BASE as int32_t + DIMEN_PAR__line_skip_limit as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"hsize\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_DIMEN as eight_bits,
            chr: DIMEN_BASE as int32_t + DIMEN_PAR__hsize as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"vsize\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_DIMEN as eight_bits,
            chr: DIMEN_BASE as int32_t + DIMEN_PAR__vsize as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"maxdepth\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_DIMEN as eight_bits,
            chr: DIMEN_BASE as int32_t + DIMEN_PAR__max_depth as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"splitmaxdepth\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_DIMEN as eight_bits,
            chr: DIMEN_BASE as int32_t + DIMEN_PAR__split_max_depth as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"boxmaxdepth\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_DIMEN as eight_bits,
            chr: DIMEN_BASE as int32_t + DIMEN_PAR__box_max_depth as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"hfuzz\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_DIMEN as eight_bits,
            chr: DIMEN_BASE as int32_t + DIMEN_PAR__hfuzz as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"vfuzz\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_DIMEN as eight_bits,
            chr: DIMEN_BASE as int32_t + DIMEN_PAR__vfuzz as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"delimitershortfall\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_DIMEN as eight_bits,
            chr: DIMEN_BASE as int32_t + DIMEN_PAR__delimiter_shortfall as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"nulldelimiterspace\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_DIMEN as eight_bits,
            chr: DIMEN_BASE as int32_t + DIMEN_PAR__null_delimiter_space as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"scriptspace\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_DIMEN as eight_bits,
            chr: DIMEN_BASE as int32_t + DIMEN_PAR__script_space as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"predisplaysize\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_DIMEN as eight_bits,
            chr: DIMEN_BASE as int32_t + DIMEN_PAR__pre_display_size as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"displaywidth\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_DIMEN as eight_bits,
            chr: DIMEN_BASE as int32_t + DIMEN_PAR__display_width as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"displayindent\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_DIMEN as eight_bits,
            chr: DIMEN_BASE as int32_t + DIMEN_PAR__display_indent as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"overfullrule\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_DIMEN as eight_bits,
            chr: DIMEN_BASE as int32_t + DIMEN_PAR__overfull_rule as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"hangindent\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_DIMEN as eight_bits,
            chr: DIMEN_BASE as int32_t + DIMEN_PAR__hang_indent as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"hoffset\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_DIMEN as eight_bits,
            chr: DIMEN_BASE as int32_t + DIMEN_PAR__h_offset as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"voffset\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_DIMEN as eight_bits,
            chr: DIMEN_BASE as int32_t + DIMEN_PAR__v_offset as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"emergencystretch\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_DIMEN as eight_bits,
            chr: DIMEN_BASE as int32_t + DIMEN_PAR__emergency_stretch as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"pdfpagewidth\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_DIMEN as eight_bits,
            chr: DIMEN_BASE as int32_t + DIMEN_PAR__pdf_page_width as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"pdfpageheight\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_DIMEN as eight_bits,
            chr: DIMEN_BASE as int32_t + DIMEN_PAR__pdf_page_height as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"lineskip\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_GLUE as eight_bits,
            chr: GLUE_BASE as int32_t + GLUE_PAR__line_skip as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"baselineskip\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_GLUE as eight_bits,
            chr: GLUE_BASE as int32_t + GLUE_PAR__baseline_skip as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"parskip\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_GLUE as eight_bits,
            chr: GLUE_BASE as int32_t + GLUE_PAR__par_skip as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"abovedisplayskip\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_GLUE as eight_bits,
            chr: GLUE_BASE as int32_t + GLUE_PAR__above_display_skip as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"belowdisplayskip\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_GLUE as eight_bits,
            chr: GLUE_BASE as int32_t + GLUE_PAR__below_display_skip as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"abovedisplayshortskip\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_GLUE as eight_bits,
            chr: GLUE_BASE as int32_t + GLUE_PAR__above_display_short_skip as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"belowdisplayshortskip\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_GLUE as eight_bits,
            chr: GLUE_BASE as int32_t + GLUE_PAR__below_display_short_skip as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"leftskip\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_GLUE as eight_bits,
            chr: GLUE_BASE as int32_t + GLUE_PAR__left_skip as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"rightskip\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_GLUE as eight_bits,
            chr: GLUE_BASE as int32_t + GLUE_PAR__right_skip as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"topskip\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_GLUE as eight_bits,
            chr: GLUE_BASE as int32_t + GLUE_PAR__top_skip as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"splittopskip\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_GLUE as eight_bits,
            chr: GLUE_BASE as int32_t + GLUE_PAR__split_top_skip as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"tabskip\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_GLUE as eight_bits,
            chr: GLUE_BASE as int32_t + GLUE_PAR__tab_skip as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"spaceskip\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_GLUE as eight_bits,
            chr: GLUE_BASE as int32_t + GLUE_PAR__space_skip as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"xspaceskip\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_GLUE as eight_bits,
            chr: GLUE_BASE as int32_t + GLUE_PAR__xspace_skip as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"parfillskip\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_GLUE as eight_bits,
            chr: GLUE_BASE as int32_t + GLUE_PAR__par_fill_skip as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"XeTeXlinebreakskip\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_GLUE as eight_bits,
            chr: GLUE_BASE as int32_t + GLUE_PAR__xetex_linebreak_skip as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"thinmuskip\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_MU_GLUE as eight_bits,
            chr: GLUE_BASE as int32_t + GLUE_PAR__thin_mu_skip as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"medmuskip\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_MU_GLUE as eight_bits,
            chr: GLUE_BASE as int32_t + GLUE_PAR__med_mu_skip as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"thickmuskip\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_MU_GLUE as eight_bits,
            chr: GLUE_BASE as int32_t + GLUE_PAR__thick_mu_skip as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"parshape\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: SET_SHAPE as eight_bits,
            chr: LOCAL_BASE as int32_t + LOCAL__par_shape as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"output\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_TOKS as eight_bits,
            chr: LOCAL_BASE as int32_t + LOCAL__output_routine as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"everypar\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_TOKS as eight_bits,
            chr: LOCAL_BASE as int32_t + LOCAL__every_par as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"everymath\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_TOKS as eight_bits,
            chr: LOCAL_BASE as int32_t + LOCAL__every_math as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"everydisplay\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_TOKS as eight_bits,
            chr: LOCAL_BASE as int32_t + LOCAL__every_display as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"everyhbox\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_TOKS as eight_bits,
            chr: LOCAL_BASE as int32_t + LOCAL__every_hbox as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"everyvbox\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_TOKS as eight_bits,
            chr: LOCAL_BASE as int32_t + LOCAL__every_vbox as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"everyjob\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_TOKS as eight_bits,
            chr: LOCAL_BASE as int32_t + LOCAL__every_job as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"everycr\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_TOKS as eight_bits,
            chr: LOCAL_BASE as int32_t + LOCAL__every_cr as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"errhelp\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_TOKS as eight_bits,
            chr: LOCAL_BASE as int32_t + LOCAL__err_help as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"everyeof\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_TOKS as eight_bits,
            chr: LOCAL_BASE as int32_t + LOCAL__every_eof as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"XeTeXinterchartoks\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_TOKS as eight_bits,
            chr: LOCAL_BASE as int32_t + LOCAL__xetex_inter_char_toks as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"TectonicCodaTokens\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_TOKS as eight_bits,
            chr: LOCAL_BASE as int32_t + LOCAL__tectonic_coda_tokens as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"interlinepenalties\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: SET_SHAPE as eight_bits,
            chr: ETEX_PEN_BASE as int32_t + ETEX_PENALTIES_PAR__inter_line_penalties as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"clubpenalties\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: SET_SHAPE as eight_bits,
            chr: ETEX_PEN_BASE as int32_t + ETEX_PENALTIES_PAR__club_penalties as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"widowpenalties\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: SET_SHAPE as eight_bits,
            chr: ETEX_PEN_BASE as int32_t + ETEX_PENALTIES_PAR__widow_penalties as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"displaywidowpenalties\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: SET_SHAPE as eight_bits,
            chr: ETEX_PEN_BASE as int32_t + ETEX_PENALTIES_PAR__display_widow_penalties as int32_t,
            extra_init: xf_prim_init_none as ::core::ffi::c_int as int32_t,
        },
        xetex_format_primitive_def_t {
            name: b"ignoreprimitiveerror\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_INT as eight_bits,
            chr: INT_BASE as int32_t + INT_PAR__ignore_primitive_error as int32_t,
            extra_init: 0,
        },
        xetex_format_primitive_def_t {
            name: b"PitexFontExpansion\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_INT as eight_bits,
            chr: INT_BASE as int32_t + INT_PAR__pitex_font_expansion as int32_t,
            extra_init: 0,
        },
        xetex_format_primitive_def_t {
            name: b"PitexFontStretch\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_INT as eight_bits,
            chr: INT_BASE as int32_t + INT_PAR__pitex_font_stretch as int32_t,
            extra_init: 0,
        },
        xetex_format_primitive_def_t {
            name: b"PitexFontShrink\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_INT as eight_bits,
            chr: INT_BASE as int32_t + INT_PAR__pitex_font_shrink as int32_t,
            extra_init: 0,
        },
        xetex_format_primitive_def_t {
            name: b"PitexFontStep\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_INT as eight_bits,
            chr: INT_BASE as int32_t + INT_PAR__pitex_font_step as int32_t,
            extra_init: 0,
        },
        xetex_format_primitive_def_t {
            name: b"pdfliteral\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: EXTENSION as eight_bits,
            chr: 100,
            extra_init: 0,
        },
        xetex_format_primitive_def_t {
            name: b"pdfobj\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: EXTENSION as eight_bits,
            chr: 101,
            extra_init: 0,
        },
        xetex_format_primitive_def_t {
            name: b"pdfrefobj\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: EXTENSION as eight_bits,
            chr: 102,
            extra_init: 0,
        },
        xetex_format_primitive_def_t {
            name: b"pdfinfo\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: EXTENSION as eight_bits,
            chr: 103,
            extra_init: 0,
        },
        xetex_format_primitive_def_t {
            name: b"pdfcatalog\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: EXTENSION as eight_bits,
            chr: 104,
            extra_init: 0,
        },
        xetex_format_primitive_def_t {
            name: b"pdfsave\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: EXTENSION as eight_bits,
            chr: 105,
            extra_init: 0,
        },
        xetex_format_primitive_def_t {
            name: b"pdfrestore\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: EXTENSION as eight_bits,
            chr: 106,
            extra_init: 0,
        },
        xetex_format_primitive_def_t {
            name: b"pdfsetmatrix\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: EXTENSION as eight_bits,
            chr: 107,
            extra_init: 0,
        },
        xetex_format_primitive_def_t {
            name: b"pdflastobj\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: LAST_ITEM as eight_bits,
            chr: 6,
            extra_init: 0,
        },
        xetex_format_primitive_def_t {
            name: b"pdfannot\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: EXTENSION as eight_bits,
            chr: 108,
            extra_init: 0,
        },
        xetex_format_primitive_def_t {
            name: b"pdfstartlink\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: EXTENSION as eight_bits,
            chr: 109,
            extra_init: 0,
        },
        xetex_format_primitive_def_t {
            name: b"pdfendlink\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: EXTENSION as eight_bits,
            chr: 110,
            extra_init: 0,
        },
        xetex_format_primitive_def_t {
            name: b"pdfdest\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: EXTENSION as eight_bits,
            chr: 111,
            extra_init: 0,
        },
        xetex_format_primitive_def_t {
            name: b"pdfxform\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: EXTENSION as eight_bits,
            chr: 112,
            extra_init: 0,
        },
        xetex_format_primitive_def_t {
            name: b"pdfrefxform\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: EXTENSION as eight_bits,
            chr: 113,
            extra_init: 0,
        },
        xetex_format_primitive_def_t {
            name: b"pdfximage\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: EXTENSION as eight_bits,
            chr: 114,
            extra_init: 0,
        },
        xetex_format_primitive_def_t {
            name: b"pdfrefximage\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: EXTENSION as eight_bits,
            chr: 115,
            extra_init: 0,
        },
        xetex_format_primitive_def_t {
            name: b"pdfoutline\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: EXTENSION as eight_bits,
            chr: 116,
            extra_init: 0,
        },
        xetex_format_primitive_def_t {
            name: b"pdflastxform\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: LAST_ITEM as eight_bits,
            chr: 7,
            extra_init: 0,
        },
        xetex_format_primitive_def_t {
            name: b"pdflastximage\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: LAST_ITEM as eight_bits,
            chr: 8,
            extra_init: 0,
        },
        xetex_format_primitive_def_t {
            name: b"pdflastximagepages\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: LAST_ITEM as eight_bits,
            chr: 9,
            extra_init: 0,
        },
        xetex_format_primitive_def_t {
            name: b"pdfescapestring\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: CONVERT as eight_bits,
            chr: 100,
            extra_init: 0,
        },
        xetex_format_primitive_def_t {
            name: b"pdfescapename\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: CONVERT as eight_bits,
            chr: 101,
            extra_init: 0,
        },
        xetex_format_primitive_def_t {
            name: b"pdfescapehex\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: CONVERT as eight_bits,
            chr: 102,
            extra_init: 0,
        },
        xetex_format_primitive_def_t {
            name: b"pdfunescapehex\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: CONVERT as eight_bits,
            chr: 103,
            extra_init: 0,
        },
        xetex_format_primitive_def_t {
            name: b"pdfstrcmp\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: CONVERT as eight_bits,
            chr: PDF_STRCMP_CODE as int32_t,
            extra_init: 0,
        },
        xetex_format_primitive_def_t {
            name: b"pdfcreationdate\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: CONVERT as eight_bits,
            chr: PDF_CREATION_DATE_CODE as int32_t,
            extra_init: 0,
        },
        xetex_format_primitive_def_t {
            name: b"pdffilemoddate\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: CONVERT as eight_bits,
            chr: PDF_FILE_MOD_DATE_CODE as int32_t,
            extra_init: 0,
        },
        xetex_format_primitive_def_t {
            name: b"pdffilesize\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: CONVERT as eight_bits,
            chr: PDF_FILE_SIZE_CODE as int32_t,
            extra_init: 0,
        },
        xetex_format_primitive_def_t {
            name: b"pdfmdfivesum\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: CONVERT as eight_bits,
            chr: PDF_MDFIVE_SUM_CODE as int32_t,
            extra_init: 0,
        },
        xetex_format_primitive_def_t {
            name: b"pdffiledump\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: CONVERT as eight_bits,
            chr: PDF_FILE_DUMP_CODE as int32_t,
            extra_init: 0,
        },
        xetex_format_primitive_def_t {
            name: b"pdfuniformdeviate\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: CONVERT as eight_bits,
            chr: UNIFORM_DEVIATE_CODE as int32_t,
            extra_init: 0,
        },
        xetex_format_primitive_def_t {
            name: b"pdfnormaldeviate\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: CONVERT as eight_bits,
            chr: NORMAL_DEVIATE_CODE as int32_t,
            extra_init: 0,
        },
        xetex_format_primitive_def_t {
            name: b"pdfresettimer\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: EXTENSION as eight_bits,
            chr: RESET_TIMER_CODE as int32_t,
            extra_init: 0,
        },
        xetex_format_primitive_def_t {
            name: b"pdfsetrandomseed\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: EXTENSION as eight_bits,
            chr: SET_RANDOM_SEED_CODE as int32_t,
            extra_init: 0,
        },
        xetex_format_primitive_def_t {
            name: b"pdfrandomseed\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: LAST_ITEM as eight_bits,
            chr: RANDOM_SEED_CODE as int32_t,
            extra_init: 0,
        },
        xetex_format_primitive_def_t {
            name: b"pdfelapsedtime\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: LAST_ITEM as eight_bits,
            chr: ELAPSED_TIME_CODE as int32_t,
            extra_init: 0,
        },
        xetex_format_primitive_def_t {
            name: b"pdfshellescape\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: LAST_ITEM as eight_bits,
            chr: PDF_SHELL_ESCAPE_CODE as int32_t,
            extra_init: 0,
        },
        xetex_format_primitive_def_t {
            name: b"pdfprotrudechars\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_INT as eight_bits,
            chr: INT_BASE + INT_PAR__xetex_protrude_chars as int32_t,
            extra_init: 0,
        },
        xetex_format_primitive_def_t {
            name: b"pdfadjustspacing\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_INT as eight_bits,
            chr: INT_BASE + INT_PAR__pitex_font_expansion as int32_t,
            extra_init: 0,
        },
        xetex_format_primitive_def_t {
            name: b"pdfprimitive\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: CONVERT as eight_bits,
            chr: 112 as int32_t,
            extra_init: 0,
        },
        xetex_format_primitive_def_t {
            name: b"ifpdfprimitive\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: IF_TEST as eight_bits,
            chr: IF_PRIMITIVE_CODE as int32_t,
            extra_init: 0,
        },
        xetex_format_primitive_def_t {
            name: b"ifpdfabsnum\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: IF_TEST as eight_bits,
            chr: 22 as int32_t,
            extra_init: 0,
        },
        xetex_format_primitive_def_t {
            name: b"ifpdfabsdim\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: IF_TEST as eight_bits,
            chr: 23 as int32_t,
            extra_init: 0,
        },
        xetex_format_primitive_def_t {
            name: b"pdfcolorstackinit\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: CONVERT as eight_bits,
            chr: 104 as int32_t,
            extra_init: 0,
        },
        xetex_format_primitive_def_t {
            name: b"pdfcolorstack\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: EXTENSION as eight_bits,
            chr: 117 as int32_t,
            extra_init: 0,
        },
        xetex_format_primitive_def_t {
            name: b"pdfmapline\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: EXTENSION as eight_bits,
            chr: 118 as int32_t,
            extra_init: 0,
        },
        xetex_format_primitive_def_t {
            name: b"pdfmapfile\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: EXTENSION as eight_bits,
            chr: 119 as int32_t,
            extra_init: 0,
        },
        xetex_format_primitive_def_t {
            name: b"pdfnames\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: EXTENSION as eight_bits,
            chr: 120 as int32_t,
            extra_init: 0,
        },
        xetex_format_primitive_def_t {
            name: b"pdftrailer\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: EXTENSION as eight_bits,
            chr: 121 as int32_t,
            extra_init: 0,
        },
        xetex_format_primitive_def_t {
            name: b"pdftrailerid\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: EXTENSION as eight_bits,
            chr: 122 as int32_t,
            extra_init: 0,
        },
        xetex_format_primitive_def_t {
            name: b"pdfpageattr\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_TOKS as eight_bits,
            chr: 0,
            extra_init: 3,
        },
        xetex_format_primitive_def_t {
            name: b"pdfpageresources\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_TOKS as eight_bits,
            chr: 0,
            extra_init: 4,
        },
        xetex_format_primitive_def_t {
            name: b"pdfmajorversion\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_INT as eight_bits,
            chr: 1,
            extra_init: 5,
        },
        xetex_format_primitive_def_t {
            name: b"pdfminorversion\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_INT as eight_bits,
            chr: 7,
            extra_init: 6,
        },
        xetex_format_primitive_def_t {
            name: b"pdfcompresslevel\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_INT as eight_bits,
            chr: 6,
            extra_init: 7,
        },
        xetex_format_primitive_def_t {
            name: b"pdfhorigin\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_DIMEN as eight_bits,
            chr: 4736287,
            extra_init: 8,
        },
        xetex_format_primitive_def_t {
            name: b"pdfvorigin\0" as *const u8 as *const ::core::ffi::c_char,
            cmd: ASSIGN_DIMEN as eight_bits,
            chr: 4736287,
            extra_init: 9,
        },
        xetex_format_primitive_def_t {
            name: ::core::ptr::null::<::core::ffi::c_char>(),
            cmd: 0 as eight_bits,
            chr: 0 as int32_t,
            extra_init: 0 as int32_t,
        },
    ];
    let mut i: ::core::ffi::c_int = 0;
    no_new_control_sequence = false_0 != 0;
    first = 0 as ::core::ffi::c_int as int32_t;
    i = 0 as ::core::ffi::c_int;
    while (i as usize)
        < (::core::mem::size_of::<[xetex_format_primitive_def_t; 566]>() as usize)
            .wrapping_div(::core::mem::size_of::<xetex_format_primitive_def_t>() as usize)
    {
        let mut prim_0: xetex_format_primitive_def_t = primitives[i as usize];
        if !prim_0.name.is_null() {
            if !(3..=9).contains(&prim_0.extra_init) {
                primitive(prim_0.name, prim_0.cmd as uint16_t, prim_0.chr);
            }
            match prim_0.extra_init {
                0 => {}
                1 => {
                    par_loc = cur_val;
                    par_token = CS_TOKEN_FLAG as int32_t + par_loc;
                }
                2 => {
                    write_loc = cur_val;
                }
                3..=9 => {
                    let public_name = std::ffi::CStr::from_ptr(prim_0.name).to_string_lossy();
                    let storage_name = std::ffi::CString::new(format!("PitexStorage{}", public_name)).unwrap();
                    let initial = if prim_0.cmd as i32 == ASSIGN_TOKS { TEX_NULL as int32_t } else { prim_0.chr };
                    primitive(storage_name.as_ptr(), UNDEFINED_CS as uint16_t, initial);
                    let storage = cur_val;
                    primitive(prim_0.name, prim_0.cmd as uint16_t, storage);
                }
                _ => {
                    (*hash.offset(prim_0.extra_init as isize)).s1 =
                        maketexstring(prim_0.name) as int32_t;
                    *eqtb.offset(prim_0.extra_init as isize) = *eqtb.offset(cur_val as isize);
                }
            }
        }
        i += 1;
    }
    pitex_register_shared();
    (*hash.offset(FROZEN_END_TEMPLATE as isize)).s1 =
        maketexstring(b"endtemplate\0" as *const u8 as *const ::core::ffi::c_char) as int32_t;
    (*eqtb.offset(FROZEN_END_TEMPLATE as isize)).b16.s1 = END_TEMPLATE as uint16_t;
    (*eqtb.offset(FROZEN_END_TEMPLATE as isize)).b32.s1 = NULL_LIST as int32_t;
    (*eqtb.offset(FROZEN_END_TEMPLATE as isize)).b16.s0 = LEVEL_ONE as uint16_t;
    (*hash.offset(FROZEN_ENDV as isize)).s1 =
        maketexstring(b"endtemplate\0" as *const u8 as *const ::core::ffi::c_char) as int32_t;
    (*eqtb.offset(FROZEN_ENDV as isize)).b16.s1 = ENDV as uint16_t;
    (*eqtb.offset(FROZEN_ENDV as isize)).b32.s1 = NULL_LIST as int32_t;
    (*eqtb.offset(FROZEN_ENDV as isize)).b16.s0 = LEVEL_ONE as uint16_t;
    (*hash.offset(FROZEN_DONT_EXPAND as isize)).s1 =
        maketexstring(b"notexpanded:\0" as *const u8 as *const ::core::ffi::c_char) as int32_t;
    (*eqtb.offset(FROZEN_DONT_EXPAND as isize)).b16.s1 = DONT_EXPAND as uint16_t;
    (*hash.offset(FROZEN_PRIMITIVE as isize)).s1 =
        maketexstring(b"primitive\0" as *const u8 as *const ::core::ffi::c_char) as int32_t;
    (*eqtb.offset(FROZEN_PRIMITIVE as isize)).b16.s1 = IGNORE_SPACES as uint16_t;
    (*eqtb.offset(FROZEN_PRIMITIVE as isize)).b32.s1 = 1 as ::core::ffi::c_int as int32_t;
    (*eqtb.offset(FROZEN_PRIMITIVE as isize)).b16.s0 = LEVEL_ONE as uint16_t;
    (*hash.offset(FROZEN_PROTECTION as isize)).s1 =
        maketexstring(b"inaccessible\0" as *const u8 as *const ::core::ffi::c_char) as int32_t;
    (*hash.offset(END_WRITE as isize)).s1 =
        maketexstring(b"endwrite\0" as *const u8 as *const ::core::ffi::c_char) as int32_t;
    (*eqtb.offset(END_WRITE as isize)).b16.s0 = LEVEL_ONE as uint16_t;
    (*eqtb.offset(END_WRITE as isize)).b16.s1 = OUTER_CALL as uint16_t;
    (*eqtb.offset(END_WRITE as isize)).b32.s1 = TEX_NULL as int32_t;
    no_new_control_sequence = true_0 != 0;
}
unsafe extern "C" fn get_strings_started() {
    pool_ptr = 0 as ::core::ffi::c_int as pool_pointer;
    str_ptr = 0 as ::core::ffi::c_int as str_number;
    *str_start.offset(0 as ::core::ffi::c_int as isize) = 0 as ::core::ffi::c_int as pool_pointer;
    str_ptr = TOO_BIG_CHAR as str_number;
    if load_pool_strings(pool_size - string_vacancies) == 0 as ::core::ffi::c_int {
        _tt_abort(b"must increase pool_size\0" as *const u8 as *const ::core::ffi::c_char);
    }
}
#[no_mangle]
pub unsafe extern "C" fn tt_cleanup() {
    free(TEX_format_default as *mut ::core::ffi::c_void);
    free(font_used as *mut ::core::ffi::c_void);
    deinitialize_shipout_variables();
    destroy_font_manager();
    let mut font_k: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while (font_k as int32_t) < font_max {
        if !(*font_layout_engine.offset(font_k as isize)).is_null() {
            release_font_engine(
                *font_layout_engine.offset(font_k as isize),
                *font_area.offset(font_k as isize) as ::core::ffi::c_int,
            );
            let ref mut fresh1 = *font_layout_engine.offset(font_k as isize);
            *fresh1 = NULL;
        }
        if !(*font_mapping.offset(font_k as isize)).is_null() {
            TECkit_DisposeConverter(*font_mapping.offset(font_k as isize) as TECkit_Converter);
            let ref mut fresh2 = *font_mapping.offset(font_k as isize);
            *fresh2 = NULL;
        }
        font_k += 1;
    }
    let mut i: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
    while i as int32_t <= in_open {
        if !(*input_file.offset(i as isize)).is_null() {
            u_close(*input_file.offset(i as isize));
        }
        i += 1;
    }
    free(buffer as *mut ::core::ffi::c_void);
    free(nest as *mut ::core::ffi::c_void);
    free(save_stack as *mut ::core::ffi::c_void);
    free(input_stack as *mut ::core::ffi::c_void);
    free(input_file as *mut ::core::ffi::c_void);
    free(line_stack as *mut ::core::ffi::c_void);
    free(eof_seen as *mut ::core::ffi::c_void);
    free(grp_stack as *mut ::core::ffi::c_void);
    free(if_stack as *mut ::core::ffi::c_void);
    free(source_filename_stack as *mut ::core::ffi::c_void);
    free(full_source_filename_stack as *mut ::core::ffi::c_void);
    free(param_stack as *mut ::core::ffi::c_void);
    free(hyph_word as *mut ::core::ffi::c_void);
    free(hyph_list as *mut ::core::ffi::c_void);
    free(hyph_link as *mut ::core::ffi::c_void);
    free(native_text as *mut ::core::ffi::c_void);
    free(yhash as *mut ::core::ffi::c_void);
    free(eqtb as *mut ::core::ffi::c_void);
    free(mem as *mut ::core::ffi::c_void);
    free(str_start as *mut ::core::ffi::c_void);
    free(str_pool as *mut ::core::ffi::c_void);
    free(font_info as *mut ::core::ffi::c_void);
    free(font_mapping as *mut ::core::ffi::c_void);
    free(font_layout_engine as *mut ::core::ffi::c_void);
    free(font_flags as *mut ::core::ffi::c_void);
    free(font_letter_space as *mut ::core::ffi::c_void);
    free(font_check as *mut ::core::ffi::c_void);
    free(font_size as *mut ::core::ffi::c_void);
    free(font_dsize as *mut ::core::ffi::c_void);
    free(font_params as *mut ::core::ffi::c_void);
    free(font_name as *mut ::core::ffi::c_void);
    free(font_area as *mut ::core::ffi::c_void);
    free(font_bc as *mut ::core::ffi::c_void);
    free(font_ec as *mut ::core::ffi::c_void);
    free(font_glue as *mut ::core::ffi::c_void);
    free(hyphen_char as *mut ::core::ffi::c_void);
    free(skew_char as *mut ::core::ffi::c_void);
    free(bchar_label as *mut ::core::ffi::c_void);
    free(font_bchar as *mut ::core::ffi::c_void);
    free(font_false_bchar as *mut ::core::ffi::c_void);
    free(char_base as *mut ::core::ffi::c_void);
    free(width_base as *mut ::core::ffi::c_void);
    free(height_base as *mut ::core::ffi::c_void);
    free(depth_base as *mut ::core::ffi::c_void);
    free(italic_base as *mut ::core::ffi::c_void);
    free(lig_kern_base as *mut ::core::ffi::c_void);
    free(kern_base as *mut ::core::ffi::c_void);
    free(exten_base as *mut ::core::ffi::c_void);
    free(param_base as *mut ::core::ffi::c_void);
    trie_trl = mfree(trie_trl as *mut ::core::ffi::c_void) as *mut trie_pointer;
    trie_tro = mfree(trie_tro as *mut ::core::ffi::c_void) as *mut trie_pointer;
    trie_trc = mfree(trie_trc as *mut ::core::ffi::c_void) as *mut uint16_t;
}
#[no_mangle]
pub unsafe extern "C" fn tt_run_engine(
    mut dump_name: *const ::core::ffi::c_char,
    mut input_file_name: *const ::core::ffi::c_char,
    mut build_date: time_t,
) -> tt_history_t {
    let mut font_k: int32_t = 0;
    rust_stdout = ttstub_output_open_stdout();
    let mut len: size_t = strlen(dump_name);
    TEX_format_default = malloc(len.wrapping_add(1 as size_t)) as *mut ::core::ffi::c_char;
    strcpy(TEX_format_default, dump_name);
    format_default_length = len as int32_t;
    if file_line_error_style_p < 0 as ::core::ffi::c_int {
        file_line_error_style_p = 0 as ::core::ffi::c_int;
    }
    pool_size = 6250000 as int32_t;
    string_vacancies = 90000 as int32_t;
    pool_free = 47500 as int32_t;
    max_strings = 565536 as int32_t;
    strings_free = 100 as ::core::ffi::c_int as int32_t;
    font_mem_size = 8000000 as int32_t;
    font_max = 9000 as ::core::ffi::c_int as int32_t;
    trie_size = 1000000 as int32_t;
    hyph_size = 8191 as ::core::ffi::c_int as int32_t;
    buf_size = 200000 as int32_t;
    nest_size = 500 as ::core::ffi::c_int as int32_t;
    max_in_open = 15 as ::core::ffi::c_int as int32_t;
    param_size = 10000 as ::core::ffi::c_int as int32_t;
    save_size = 80000 as int32_t;
    stack_size = 5000 as ::core::ffi::c_int as int32_t;
    error_line = 79 as ::core::ffi::c_int as int32_t;
    half_error_line = 50 as ::core::ffi::c_int as int32_t;
    max_print_line = 79 as ::core::ffi::c_int as int32_t;
    hash_extra = 600000 as int32_t;
    expand_depth = 10000 as ::core::ffi::c_int as int32_t;
    buffer = malloc(
        ((buf_size + 1 as int32_t) as size_t)
            .wrapping_mul(::core::mem::size_of::<UnicodeScalar>() as size_t),
    ) as *mut UnicodeScalar;
    nest = malloc(
        ((nest_size + 1 as int32_t) as size_t)
            .wrapping_mul(::core::mem::size_of::<list_state_record>() as size_t),
    ) as *mut list_state_record;
    save_stack = malloc(
        ((save_size + 1 as int32_t) as size_t)
            .wrapping_mul(::core::mem::size_of::<memory_word>() as size_t),
    ) as *mut memory_word;
    input_stack = malloc(
        ((stack_size + 1 as int32_t) as size_t)
            .wrapping_mul(::core::mem::size_of::<input_state_t>() as size_t),
    ) as *mut input_state_t;
    input_file = malloc(
        ((max_in_open + 1 as int32_t) as size_t)
            .wrapping_mul(::core::mem::size_of::<*mut UFILE>() as size_t),
    ) as *mut *mut UFILE;
    line_stack = malloc(
        ((max_in_open + 1 as int32_t) as size_t)
            .wrapping_mul(::core::mem::size_of::<int32_t>() as size_t),
    ) as *mut int32_t;
    eof_seen = malloc(
        ((max_in_open + 1 as int32_t) as size_t)
            .wrapping_mul(::core::mem::size_of::<bool>() as size_t),
    ) as *mut bool;
    grp_stack = malloc(
        ((max_in_open + 1 as int32_t) as size_t)
            .wrapping_mul(::core::mem::size_of::<save_pointer>() as size_t),
    ) as *mut save_pointer;
    if_stack = malloc(
        ((max_in_open + 1 as int32_t) as size_t)
            .wrapping_mul(::core::mem::size_of::<int32_t>() as size_t),
    ) as *mut int32_t;
    source_filename_stack = malloc(
        ((max_in_open + 1 as int32_t) as size_t)
            .wrapping_mul(::core::mem::size_of::<str_number>() as size_t),
    ) as *mut str_number;
    full_source_filename_stack = malloc(
        ((max_in_open + 1 as int32_t) as size_t)
            .wrapping_mul(::core::mem::size_of::<str_number>() as size_t),
    ) as *mut str_number;
    param_stack = malloc(
        ((param_size + 1 as int32_t) as size_t)
            .wrapping_mul(::core::mem::size_of::<int32_t>() as size_t),
    ) as *mut int32_t;
    hyph_word = malloc(
        ((hyph_size + 1 as int32_t) as size_t)
            .wrapping_mul(::core::mem::size_of::<str_number>() as size_t),
    ) as *mut str_number;
    hyph_list = malloc(
        ((hyph_size + 1 as int32_t) as size_t)
            .wrapping_mul(::core::mem::size_of::<int32_t>() as size_t),
    ) as *mut int32_t;
    hyph_link = malloc(
        ((hyph_size + 1 as int32_t) as size_t)
            .wrapping_mul(::core::mem::size_of::<hyph_pointer>() as size_t),
    ) as *mut hyph_pointer;
    if in_initex_mode {
        mem = malloc(
            ((4999999 as ::core::ffi::c_int + 1 as ::core::ffi::c_int + 1 as ::core::ffi::c_int)
                as size_t)
                .wrapping_mul(::core::mem::size_of::<memory_word>() as size_t),
        ) as *mut memory_word;
        eqtb_top = EQTB_SIZE as int32_t + hash_extra;
        if hash_extra == 0 as int32_t {
            hash_top = UNDEFINED_CONTROL_SEQUENCE as int32_t;
        } else {
            hash_top = eqtb_top;
        }
        yhash = malloc(
            ((1 as int32_t + hash_top - 514 as int32_t + 1 as int32_t) as size_t)
                .wrapping_mul(::core::mem::size_of::<b32x2>() as size_t),
        ) as *mut b32x2;
        hash = yhash.offset(-(hash_offset as isize));
        (*hash.offset(HASH_BASE as isize)).s0 = 0 as ::core::ffi::c_int as int32_t;
        (*hash.offset(HASH_BASE as isize)).s1 = 0 as ::core::ffi::c_int as int32_t;
        hash_used = (HASH_BASE + 1 as ::core::ffi::c_int) as int32_t;
        while hash_used <= hash_top {
            *hash.offset(hash_used as isize) = *hash.offset(HASH_BASE as isize);
            hash_used += 1;
        }
        eqtb = calloc(
            (eqtb_top + 1 as int32_t) as size_t,
            ::core::mem::size_of::<memory_word>() as size_t,
        ) as *mut memory_word;
        str_start = malloc(
            ((max_strings + 1 as int32_t) as size_t)
                .wrapping_mul(::core::mem::size_of::<pool_pointer>() as size_t),
        ) as *mut pool_pointer;
        str_pool = malloc(
            ((pool_size + 1 as int32_t) as size_t)
                .wrapping_mul(::core::mem::size_of::<packed_UTF16_code>() as size_t),
        ) as *mut packed_UTF16_code;
        font_info = malloc(
            ((font_mem_size + 1 as int32_t) as size_t)
                .wrapping_mul(::core::mem::size_of::<memory_word>() as size_t),
        ) as *mut memory_word;
    }
    history = HISTORY_FATAL_ERROR;
    bad = 0 as ::core::ffi::c_int as int32_t;
    if half_error_line < 30 as int32_t || half_error_line > error_line - 15 as int32_t {
        bad = 1 as ::core::ffi::c_int as int32_t;
    }
    if max_print_line < 60 as int32_t {
        bad = 2 as ::core::ffi::c_int as int32_t;
    }
    if 1100 as ::core::ffi::c_int > MEM_TOP {
        bad = 4 as ::core::ffi::c_int as int32_t;
    }
    if HASH_PRIME > HASH_SIZE {
        bad = 5 as ::core::ffi::c_int as int32_t;
    }
    if max_in_open >= 128 as int32_t {
        bad = 6 as ::core::ffi::c_int as int32_t;
    }
    if MEM_TOP < 267 as ::core::ffi::c_int {
        bad = 7 as ::core::ffi::c_int as int32_t;
    }
    if MIN_HALFWORD > 0 as ::core::ffi::c_int {
        bad = 12 as ::core::ffi::c_int as int32_t;
    }
    if MAX_FONT_MAX < MIN_HALFWORD || MAX_FONT_MAX > MAX_HALFWORD {
        bad = 15 as ::core::ffi::c_int as int32_t;
    }
    if font_max > FONT_BASE as int32_t + 9000 as int32_t {
        bad = 16 as ::core::ffi::c_int as int32_t;
    }
    if save_size > MAX_HALFWORD as int32_t || max_strings > MAX_HALFWORD as int32_t {
        bad = 17 as ::core::ffi::c_int as int32_t;
    }
    if buf_size > MAX_HALFWORD as int32_t {
        bad = 18 as ::core::ffi::c_int as int32_t;
    }
    if CS_TOKEN_FLAG as int32_t + EQTB_SIZE as int32_t + hash_extra > MAX_HALFWORD as int32_t {
        bad = 21 as ::core::ffi::c_int as int32_t;
    }
    if hash_offset < 0 as ::core::ffi::c_int || hash_offset > HASH_BASE {
        bad = 42 as ::core::ffi::c_int as int32_t;
    }
    if format_default_length > INT32_MAX as int32_t {
        bad = 31 as ::core::ffi::c_int as int32_t;
    }
    if 2 as ::core::ffi::c_int * MAX_HALFWORD < MEM_TOP {
        bad = 41 as ::core::ffi::c_int as int32_t;
    }
    if bad > 0 as int32_t {
        _tt_abort(
            b"failed internal consistency check #%d\0" as *const u8 as *const ::core::ffi::c_char,
            bad,
        );
    }
    initialize_more_variables();
    if in_initex_mode {
        get_strings_started();
        initialize_more_initex_variables();
        initialize_primitives();
        init_str_ptr = str_ptr;
        init_pool_ptr = pool_ptr;
    }
    initialize_math_variables();
    initialize_pagebuilder_variables();
    initialize_shipout_variables();
    get_seconds_and_micros(&raw mut epochseconds, &raw mut microseconds);
    init_start_time(build_date);
    selector = SELECTOR_TERM_ONLY;
    tally = 0 as ::core::ffi::c_int as int32_t;
    term_offset = 0 as ::core::ffi::c_int as int32_t;
    file_offset = 0 as ::core::ffi::c_int as int32_t;
    job_name = 0 as ::core::ffi::c_int as str_number;
    name_in_progress = false_0 != 0;
    log_opened = false_0 != 0;
    if semantic_pagination_enabled {
        output_file_extension = b".spx\0" as *const u8 as *const ::core::ffi::c_char;
    } else {
        output_file_extension = b".xdv\0" as *const u8 as *const ::core::ffi::c_char;
    }
    input_ptr = 0 as ::core::ffi::c_int as int32_t;
    max_in_stack = 0 as ::core::ffi::c_int as int32_t;
    *source_filename_stack.offset(0 as ::core::ffi::c_int as isize) =
        0 as ::core::ffi::c_int as str_number;
    *full_source_filename_stack.offset(0 as ::core::ffi::c_int as isize) =
        0 as ::core::ffi::c_int as str_number;
    in_open = 0 as ::core::ffi::c_int as int32_t;
    open_parens = 0 as ::core::ffi::c_int as int32_t;
    max_buf_stack = 0 as ::core::ffi::c_int as int32_t;
    *grp_stack.offset(0 as ::core::ffi::c_int as isize) = 0 as ::core::ffi::c_int as save_pointer;
    *if_stack.offset(0 as ::core::ffi::c_int as isize) = TEX_NULL as int32_t;
    param_ptr = 0 as ::core::ffi::c_int as int32_t;
    max_param_stack = 0 as ::core::ffi::c_int as int32_t;
    used_tectonic_coda_tokens = false_0 != 0;
    gave_char_warning_help = false_0 != 0;
    memset(
        buffer as *mut ::core::ffi::c_void,
        0 as ::core::ffi::c_int,
        (buf_size as size_t).wrapping_mul(::core::mem::size_of::<UnicodeScalar>() as size_t),
    );
    first = 0 as ::core::ffi::c_int as int32_t;
    scanner_status = NORMAL as ::core::ffi::c_uchar;
    warning_index = TEX_NULL as int32_t;
    first = 1 as ::core::ffi::c_int as int32_t;
    cur_input.state = NEW_LINE as uint16_t;
    cur_input.start = 1 as ::core::ffi::c_int as int32_t;
    cur_input.index = 0 as uint16_t;
    line = 0 as ::core::ffi::c_int as int32_t;
    cur_input.name = 0 as ::core::ffi::c_int as int32_t;
    force_eof = false_0 != 0;
    align_state = 1000000 as int32_t;
    init_io();
    if in_initex_mode {
        no_new_control_sequence = false_0 != 0;
        max_reg_num = 32767 as ::core::ffi::c_int as int32_t;
        max_reg_help_line = b"A register number must be between 0 and 32767.\0" as *const u8
            as *const ::core::ffi::c_char;
    }
    no_new_control_sequence = true_0 != 0;
    if !in_initex_mode {
        if !load_fmt_file() {
            return history;
        }
    }
    if (*eqtb.offset((INT_BASE + INT_PAR__end_line_char) as isize))
        .b32
        .s1
        < 0 as int32_t
        || (*eqtb.offset((INT_BASE + INT_PAR__end_line_char) as isize))
            .b32
            .s1
            > BIGGEST_CHAR as int32_t
    {
        cur_input.limit -= 1;
    } else {
        *buffer.offset(cur_input.limit as isize) = (*eqtb
            .offset((INT_BASE + INT_PAR__end_line_char) as isize))
        .b32
        .s1 as UnicodeScalar;
    }
    if in_initex_mode {
        (*eqtb.offset((INT_BASE + INT_PAR__time) as isize)).b32.s1 =
            0 as ::core::ffi::c_int as int32_t;
        (*eqtb.offset((INT_BASE + INT_PAR__day) as isize)).b32.s1 =
            0 as ::core::ffi::c_int as int32_t;
        (*eqtb.offset((INT_BASE + INT_PAR__month) as isize)).b32.s1 =
            0 as ::core::ffi::c_int as int32_t;
        (*eqtb.offset((INT_BASE + INT_PAR__year) as isize)).b32.s1 =
            0 as ::core::ffi::c_int as int32_t;
    } else {
        get_date_and_time(
            build_date,
            &raw mut (*eqtb.offset((INT_BASE + INT_PAR__time) as isize)).b32.s1,
            &raw mut (*eqtb.offset((INT_BASE + INT_PAR__day) as isize)).b32.s1,
            &raw mut (*eqtb.offset((INT_BASE + INT_PAR__month) as isize)).b32.s1,
            &raw mut (*eqtb.offset((INT_BASE + INT_PAR__year) as isize)).b32.s1,
        );
    }
    if trie_not_ready {
        trie_trl = malloc(
            ((trie_size + 1 as int32_t) as size_t)
                .wrapping_mul(::core::mem::size_of::<trie_pointer>() as size_t),
        ) as *mut trie_pointer;
        trie_tro = malloc(
            ((trie_size + 1 as int32_t) as size_t)
                .wrapping_mul(::core::mem::size_of::<trie_pointer>() as size_t),
        ) as *mut trie_pointer;
        trie_trc = malloc(
            ((trie_size + 1 as int32_t) as size_t)
                .wrapping_mul(::core::mem::size_of::<uint16_t>() as size_t),
        ) as *mut uint16_t;
        trie_c = malloc(
            ((trie_size + 1 as int32_t) as size_t)
                .wrapping_mul(::core::mem::size_of::<packed_UTF16_code>() as size_t),
        ) as *mut packed_UTF16_code;
        trie_o = malloc(
            ((trie_size + 1 as int32_t) as size_t)
                .wrapping_mul(::core::mem::size_of::<trie_opcode>() as size_t),
        ) as *mut trie_opcode;
        trie_l = malloc(
            ((trie_size + 1 as int32_t) as size_t)
                .wrapping_mul(::core::mem::size_of::<trie_pointer>() as size_t),
        ) as *mut trie_pointer;
        trie_r = malloc(
            ((trie_size + 1 as int32_t) as size_t)
                .wrapping_mul(::core::mem::size_of::<trie_pointer>() as size_t),
        ) as *mut trie_pointer;
        trie_hash = malloc(
            ((trie_size + 1 as int32_t) as size_t)
                .wrapping_mul(::core::mem::size_of::<trie_pointer>() as size_t),
        ) as *mut trie_pointer;
        trie_taken = malloc(
            ((trie_size + 1 as int32_t) as size_t)
                .wrapping_mul(::core::mem::size_of::<bool>() as size_t),
        ) as *mut bool;
        *trie_l.offset(0 as ::core::ffi::c_int as isize) = 0 as ::core::ffi::c_int as trie_pointer;
        *trie_c.offset(0 as ::core::ffi::c_int as isize) = 0 as packed_UTF16_code;
        trie_ptr = 0 as ::core::ffi::c_int as trie_pointer;
        *trie_r.offset(0 as ::core::ffi::c_int as isize) = 0 as ::core::ffi::c_int as trie_pointer;
        hyph_start = 0 as ::core::ffi::c_int as trie_pointer;
        font_mapping = calloc(
            (font_max + 1 as int32_t) as size_t,
            ::core::mem::size_of::<*mut ::core::ffi::c_void>() as size_t,
        ) as *mut *mut ::core::ffi::c_void;
        font_layout_engine = calloc(
            (font_max + 1 as int32_t) as size_t,
            ::core::mem::size_of::<*mut ::core::ffi::c_void>() as size_t,
        ) as *mut *mut ::core::ffi::c_void;
        font_flags = calloc(
            (font_max + 1 as int32_t) as size_t,
            ::core::mem::size_of::<::core::ffi::c_char>() as size_t,
        ) as *mut ::core::ffi::c_char;
        font_letter_space = calloc(
            (font_max + 1 as int32_t) as size_t,
            ::core::mem::size_of::<scaled_t>() as size_t,
        ) as *mut scaled_t;
        font_check = calloc(
            (font_max + 1 as int32_t) as size_t,
            ::core::mem::size_of::<b16x4>() as size_t,
        ) as *mut b16x4;
        font_size = calloc(
            (font_max + 1 as int32_t) as size_t,
            ::core::mem::size_of::<scaled_t>() as size_t,
        ) as *mut scaled_t;
        font_dsize = calloc(
            (font_max + 1 as int32_t) as size_t,
            ::core::mem::size_of::<scaled_t>() as size_t,
        ) as *mut scaled_t;
        font_params = calloc(
            (font_max + 1 as int32_t) as size_t,
            ::core::mem::size_of::<font_index>() as size_t,
        ) as *mut font_index;
        font_name = calloc(
            (font_max + 1 as int32_t) as size_t,
            ::core::mem::size_of::<str_number>() as size_t,
        ) as *mut str_number;
        font_area = calloc(
            (font_max + 1 as int32_t) as size_t,
            ::core::mem::size_of::<str_number>() as size_t,
        ) as *mut str_number;
        font_bc = calloc(
            (font_max + 1 as int32_t) as size_t,
            ::core::mem::size_of::<UTF16_code>() as size_t,
        ) as *mut UTF16_code;
        font_ec = calloc(
            (font_max + 1 as int32_t) as size_t,
            ::core::mem::size_of::<UTF16_code>() as size_t,
        ) as *mut UTF16_code;
        font_glue = calloc(
            (font_max + 1 as int32_t) as size_t,
            ::core::mem::size_of::<int32_t>() as size_t,
        ) as *mut int32_t;
        hyphen_char = calloc(
            (font_max + 1 as int32_t) as size_t,
            ::core::mem::size_of::<int32_t>() as size_t,
        ) as *mut int32_t;
        skew_char = calloc(
            (font_max + 1 as int32_t) as size_t,
            ::core::mem::size_of::<int32_t>() as size_t,
        ) as *mut int32_t;
        bchar_label = calloc(
            (font_max + 1 as int32_t) as size_t,
            ::core::mem::size_of::<font_index>() as size_t,
        ) as *mut font_index;
        font_bchar = calloc(
            (font_max + 1 as int32_t) as size_t,
            ::core::mem::size_of::<nine_bits>() as size_t,
        ) as *mut nine_bits;
        font_false_bchar = calloc(
            (font_max + 1 as int32_t) as size_t,
            ::core::mem::size_of::<nine_bits>() as size_t,
        ) as *mut nine_bits;
        char_base = calloc(
            (font_max + 1 as int32_t) as size_t,
            ::core::mem::size_of::<int32_t>() as size_t,
        ) as *mut int32_t;
        width_base = calloc(
            (font_max + 1 as int32_t) as size_t,
            ::core::mem::size_of::<int32_t>() as size_t,
        ) as *mut int32_t;
        height_base = calloc(
            (font_max + 1 as int32_t) as size_t,
            ::core::mem::size_of::<int32_t>() as size_t,
        ) as *mut int32_t;
        depth_base = calloc(
            (font_max + 1 as int32_t) as size_t,
            ::core::mem::size_of::<int32_t>() as size_t,
        ) as *mut int32_t;
        italic_base = calloc(
            (font_max + 1 as int32_t) as size_t,
            ::core::mem::size_of::<int32_t>() as size_t,
        ) as *mut int32_t;
        lig_kern_base = calloc(
            (font_max + 1 as int32_t) as size_t,
            ::core::mem::size_of::<int32_t>() as size_t,
        ) as *mut int32_t;
        kern_base = calloc(
            (font_max + 1 as int32_t) as size_t,
            ::core::mem::size_of::<int32_t>() as size_t,
        ) as *mut int32_t;
        exten_base = calloc(
            (font_max + 1 as int32_t) as size_t,
            ::core::mem::size_of::<int32_t>() as size_t,
        ) as *mut int32_t;
        param_base = calloc(
            (font_max + 1 as int32_t) as size_t,
            ::core::mem::size_of::<int32_t>() as size_t,
        ) as *mut int32_t;
        font_ptr = FONT_BASE as internal_font_number;
        fmem_ptr = 7 as ::core::ffi::c_int as font_index;
        *font_name.offset(FONT_BASE as isize) =
            maketexstring(b"nullfont\0" as *const u8 as *const ::core::ffi::c_char) as str_number;
        *font_area.offset(FONT_BASE as isize) = EMPTY_STRING as str_number;
        *hyphen_char.offset(FONT_BASE as isize) = '-' as i32 as int32_t;
        *skew_char.offset(FONT_BASE as isize) = -(1 as ::core::ffi::c_int) as int32_t;
        *bchar_label.offset(FONT_BASE as isize) = NON_ADDRESS as font_index;
        *font_bchar.offset(FONT_BASE as isize) = TOO_BIG_CHAR as nine_bits;
        *font_false_bchar.offset(FONT_BASE as isize) = TOO_BIG_CHAR as nine_bits;
        *font_bc.offset(FONT_BASE as isize) = 1 as UTF16_code;
        *font_ec.offset(FONT_BASE as isize) = 0 as UTF16_code;
        *font_size.offset(FONT_BASE as isize) = 0 as ::core::ffi::c_int as scaled_t;
        *font_dsize.offset(FONT_BASE as isize) = 0 as ::core::ffi::c_int as scaled_t;
        *char_base.offset(FONT_BASE as isize) = 0 as ::core::ffi::c_int as int32_t;
        *width_base.offset(FONT_BASE as isize) = 0 as ::core::ffi::c_int as int32_t;
        *height_base.offset(FONT_BASE as isize) = 0 as ::core::ffi::c_int as int32_t;
        *depth_base.offset(FONT_BASE as isize) = 0 as ::core::ffi::c_int as int32_t;
        *italic_base.offset(FONT_BASE as isize) = 0 as ::core::ffi::c_int as int32_t;
        *lig_kern_base.offset(FONT_BASE as isize) = 0 as ::core::ffi::c_int as int32_t;
        *kern_base.offset(FONT_BASE as isize) = 0 as ::core::ffi::c_int as int32_t;
        *exten_base.offset(FONT_BASE as isize) = 0 as ::core::ffi::c_int as int32_t;
        *font_glue.offset(FONT_BASE as isize) = TEX_NULL as int32_t;
        *font_params.offset(FONT_BASE as isize) = 7 as ::core::ffi::c_int as font_index;
        let ref mut fresh0 = *font_mapping.offset(FONT_BASE as isize);
        *fresh0 = ::core::ptr::null_mut::<::core::ffi::c_void>();
        *param_base.offset(FONT_BASE as isize) = -(1 as ::core::ffi::c_int) as int32_t;
        font_k = 0 as ::core::ffi::c_int as int32_t;
        while font_k <= 6 as int32_t {
            (*font_info.offset(font_k as isize)).b32.s1 = 0 as ::core::ffi::c_int as int32_t;
            font_k += 1;
        }
    }
    font_used = malloc(
        ((font_max + 1 as int32_t) as size_t)
            .wrapping_mul(::core::mem::size_of::<bool>() as size_t),
    ) as *mut bool;
    font_k = 0 as ::core::ffi::c_int as int32_t;
    while font_k <= font_max {
        *font_used.offset(font_k as isize) = false_0 != 0;
        font_k += 1;
    }
    random_seed = ((microseconds * 1000 as int32_t) as ::core::ffi::c_long
        + epochseconds as ::core::ffi::c_long % 1000000 as ::core::ffi::c_long)
        as scaled_t;
    init_randoms(random_seed as int32_t);
    if interaction as ::core::ffi::c_int == BATCH_MODE {
        selector = SELECTOR_NO_PRINT;
    } else {
        selector = SELECTOR_TERM_ONLY;
    }
    if semantic_pagination_enabled {
        (*eqtb.offset((INT_BASE + INT_PAR__xetex_generate_actual_text) as isize))
            .b32
            .s1 = 1 as ::core::ffi::c_int as int32_t;
    }
    synctex_init_command();
    start_input(input_file_name);
    history = HISTORY_SPOTLESS;
    main_control();
    final_cleanup();
    close_files_and_terminate();
    tt_cleanup();
    return history;
}
pub const true_0: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const false_0: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
