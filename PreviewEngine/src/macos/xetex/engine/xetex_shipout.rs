/* Copyright 2016-2018 The Tectonic Project
 * Licensed under the MIT License.
 */
// Translated from xetex/engine/xetex-shipout.c with C2Rust 0.22.1.
static mut PITEX_LAST_EMITTED_GLYPH_VERSION:i32=-1;
extern "C" {
    fn pitex_shipout_page_reference(page:i32) -> i32;
    fn pitex_global_special_count() -> usize;
    fn pitex_global_special_at(index: usize) -> *const ::core::ffi::c_char;
    fn pitex_global_specials_clear();
    fn pitex_pending_form_count() -> usize;
    fn pitex_pending_form_at(index: usize) -> int32_t;
    fn pitex_pending_forms_clear();
    fn pitex_output_parameter(name: *const ::core::ffi::c_char) -> int32_t;
    fn pitex_page_tokens(name: *const ::core::ffi::c_char) -> *mut ::core::ffi::c_char;
    fn pitex_page_tokens_free(value: *mut ::core::ffi::c_char);
    pub type ttbc_output_handle_t;
    pub type ttbc_diagnostic_t;
    fn malloc(__size: size_t) -> *mut ::core::ffi::c_void;
    fn free(_: *mut ::core::ffi::c_void);
    fn abs(_: ::core::ffi::c_int) -> ::core::ffi::c_int;
    fn strerror(__errnum: ::core::ffi::c_int) -> *mut ::core::ffi::c_char;
    fn strlen(__s: *const ::core::ffi::c_char) -> size_t;
    fn _tt_abort(format: *const ::core::ffi::c_char, ...) -> !;
    fn ttstub_output_open(
        path: *const ::core::ffi::c_char,
        is_gz: ::core::ffi::c_int,
    ) -> rust_output_handle_t;
    fn ttstub_output_write(
        handle: rust_output_handle_t,
        data: *const ::core::ffi::c_char,
        len: size_t,
    ) -> size_t;
    fn ttstub_output_flush(handle: rust_output_handle_t) -> ::core::ffi::c_int;
    fn ttstub_output_close(handle: rust_output_handle_t) -> ::core::ffi::c_int;
    fn ttstub_shell_escape(cmd: *const ::core::ffi::c_ushort, len: size_t) -> ::core::ffi::c_int;
    fn makeXDVGlyphArrayData(p: *mut ::core::ffi::c_void) -> ::core::ffi::c_int;
    fn make_font_def(f: int32_t) -> ::core::ffi::c_int;
    fn store_justified_native_glyphs(node: *mut ::core::ffi::c_void);
    fn maketexstring(s: *const ::core::ffi::c_char) -> ::core::ffi::c_int;
    fn apply_tfm_font_mapping(
        mapping: *mut ::core::ffi::c_void,
        c: ::core::ffi::c_int,
    ) -> ::core::ffi::c_int;
    static mut shell_escape_enabled: bool;
    static mut eqtb: *mut memory_word;
    static mut name_of_file: *mut ::core::ffi::c_char;
    static mut max_print_line: int32_t;
    static mut pool_size: int32_t;
    static mut str_pool: *mut packed_UTF16_code;
    static mut str_start: *mut pool_pointer;
    static mut pool_ptr: pool_pointer;
    static mut str_ptr: str_number;
    static mut init_pool_ptr: pool_pointer;
    static mut rust_stdout: rust_output_handle_t;
    static mut selector: selector_t;
    static mut term_offset: int32_t;
    static mut file_offset: int32_t;
    static mut doing_special: bool;
    static mut help_line: [*const ::core::ffi::c_char; 6];
    static mut help_ptr: ::core::ffi::c_uchar;
    static mut temp_ptr: int32_t;
    static mut mem: *mut memory_word;
    static mut hi_mem_min: int32_t;
    static mut avail: int32_t;
    static mut cur_list: list_state_record;
    static mut cur_cs: int32_t;
    static mut cur_tok: int32_t;
    static mut def_ref: int32_t;
    static mut cur_name: str_number;
    static mut cur_area: str_number;
    static mut cur_ext: str_number;
    static mut job_name: str_number;
    static mut log_opened: bool;
    static mut output_file_extension: *const ::core::ffi::c_char;
    static mut font_info: *mut memory_word;
    static mut font_ptr: internal_font_number;
    static mut font_check: *mut b16x4;
    static mut font_size: *mut scaled_t;
    static mut font_dsize: *mut scaled_t;
    static mut font_name: *mut str_number;
    static mut font_area: *mut str_number;
    static mut font_bc: *mut UTF16_code;
    static mut font_ec: *mut UTF16_code;
    static mut font_glue: *mut int32_t;
    static mut font_used: *mut bool;
    static mut font_mapping: *mut *mut ::core::ffi::c_void;
    static mut font_letter_space: *mut scaled_t;
    static mut xdv_buffer: *mut ::core::ffi::c_char;
    static mut char_base: *mut int32_t;
    static mut width_base: *mut int32_t;
    static mut total_pages: int32_t;
    static mut max_v: scaled_t;
    static mut max_h: scaled_t;
    static mut max_push: int32_t;
    static mut last_bop: int32_t;
    static mut dead_cycles: int32_t;
    static mut doing_leaders: bool;
    static mut rule_ht: scaled_t;
    static mut rule_dp: scaled_t;
    static mut rule_wd: scaled_t;
    static mut cur_h: scaled_t;
    static mut cur_v: scaled_t;
    static mut write_file: [rust_output_handle_t; 16];
    static mut write_open: [bool; 18];
    static mut write_loc: int32_t;
    static mut cur_page_width: scaled_t;
    static mut cur_page_height: scaled_t;
    static mut cur_h_offset: scaled_t;
    static mut cur_v_offset: scaled_t;
    static mut pdf_last_x_pos: int32_t;
    static mut pdf_last_y_pos: int32_t;
    static mut LR_ptr: int32_t;
    static mut LR_problems: int32_t;
    static mut cur_dir: small_number;
    static mut xtx_ligature_present: bool;
    static mut semantic_pagination_enabled: bool;
    fn show_token_list(p: int32_t, q: int32_t, l: int32_t);
    fn get_avail() -> int32_t;
    fn flush_list(p: int32_t);
    fn get_node(s: int32_t) -> int32_t;
    fn free_node(p: int32_t, s: int32_t);
    fn new_math(w: scaled_t, s: small_number) -> int32_t;
    fn new_kern(w: scaled_t) -> int32_t;
    fn show_box(p: int32_t);
    fn flush_node_list(p: int32_t);
    fn begin_diagnostic();
    fn end_diagnostic(blank_line: bool);
    fn prepare_mag();
    fn token_show(p: int32_t);
    fn begin_token_list(p: int32_t, t: uint16_t);
    fn end_token_list();
    fn get_token();
    fn effective_char(err_p: bool, f: internal_font_number, c: uint16_t) -> int32_t;
    fn scan_toks(macro_def: bool, xpand: bool) -> int32_t;
    fn pack_file_name(n: str_number, a: str_number, e: str_number);
    fn make_name_string() -> str_number;
    fn pack_job_name(_: *const ::core::ffi::c_char);
    fn open_log_file();
    fn new_native_word_node(f: internal_font_number, n: int32_t) -> int32_t;
    fn error();
    fn fatal_error(s: *const ::core::ffi::c_char) -> !;
    fn overflow(s: *const ::core::ffi::c_char, n: int32_t) -> !;
    fn confusion(s: *const ::core::ffi::c_char) -> !;
    fn diagnostic_begin_capture_warning_here() -> *mut ttbc_diagnostic_t;
    fn capture_to_diagnostic(diagnostic: *mut ttbc_diagnostic_t);
    fn error_here_with_diagnostic(message: *const ::core::ffi::c_char) -> *mut ttbc_diagnostic_t;
    fn print_ln();
    fn print_raw_char(s: UTF16_code, incr_offset: bool);
    fn print_char(s: int32_t);
    fn print(s: int32_t);
    fn print_cstr(s: *const ::core::ffi::c_char);
    fn print_nl_cstr(s: *const ::core::ffi::c_char);
    fn print_int(n: int32_t);
    fn print_file_name(n: int32_t, a: int32_t, e: int32_t);
    fn print_scaled(s: scaled_t);
    fn tex_round(_: ::core::ffi::c_double) -> int32_t;
    fn length(s: str_number) -> int32_t;
    fn synctex_sheet(mag: int32_t);
    fn synctex_teehs();
    fn synctex_vlist(this_box: int32_t);
    fn synctex_tsilv(this_box: int32_t);
    fn synctex_void_vlist(p: int32_t, this_box: int32_t);
    fn synctex_hlist(this_box: int32_t);
    fn synctex_tsilh(this_box: int32_t);
    fn synctex_void_hlist(p: int32_t, this_box: int32_t);
    fn synctex_math(p: int32_t, this_box: int32_t);
    fn synctex_horizontal_rule_or_glue(p: int32_t, this_box: int32_t);
    fn synctex_kern(p: int32_t, this_box: int32_t);
    fn synctex_current();
}
pub type __darwin_size_t = usize;
pub type size_t = __darwin_size_t;
pub type int32_t = i32;
pub type uint16_t = u16;
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
pub type glue_ord = ::core::ffi::c_uchar;
pub type internal_font_number = int32_t;
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
pub const __DARWIN_NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
pub const NULL: *mut ::core::ffi::c_void = __DARWIN_NULL;
pub const MIN_HALFWORD: ::core::ffi::c_int = -(0xfffffff as ::core::ffi::c_int);
pub const MAX_HALFWORD: ::core::ffi::c_int = 0x3fffffff as ::core::ffi::c_int;
pub const TEX_NULL: ::core::ffi::c_int = MIN_HALFWORD;
pub const TEX_INFINITY: ::core::ffi::c_int = 0x7fffffff as ::core::ffi::c_int;
pub const NULL_FLAG: ::core::ffi::c_int = -(0x40000000 as ::core::ffi::c_int);
pub const BIGGEST_USV: ::core::ffi::c_int = 0x10ffff as ::core::ffi::c_int;
pub const NUMBER_USVS: ::core::ffi::c_int = BIGGEST_USV + 1 as ::core::ffi::c_int;
pub const LIG_TRICK: ::core::ffi::c_int = MEM_TOP - 12 as ::core::ffi::c_int;
pub const END_WRITE: ::core::ffi::c_int = FROZEN_CONTROL_SEQUENCE + 8 as ::core::ffi::c_int;
pub const LEFT_TO_RIGHT: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const RIGHT_TO_LEFT: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const SYNCTEX_FIELD_SIZE: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const EDGE_NODE: ::core::ffi::c_int = STYLE_NODE;
pub const SMALL_NODE_SIZE: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const EDGE_NODE_SIZE: ::core::ffi::c_int = 3 as ::core::ffi::c_int;
pub const MEDIUM_NODE_SIZE: ::core::ffi::c_int = 3 as ::core::ffi::c_int;
pub const MOVEMENT_NODE_SIZE: ::core::ffi::c_int = 3 as ::core::ffi::c_int;
pub const GLUE_SPEC_SIZE: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
pub const NATIVE_NODE_SIZE: ::core::ffi::c_int = 6 as ::core::ffi::c_int;
pub const PIC_NODE_SIZE: ::core::ffi::c_int = 9 as ::core::ffi::c_int;
pub const L_CODE: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
pub const R_CODE: ::core::ffi::c_int = 8 as ::core::ffi::c_int;
pub const FILLL: ::core::ffi::c_int = 3 as ::core::ffi::c_int;
pub const NORMAL: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const INSERTED: ::core::ffi::c_int = 5 as ::core::ffi::c_int;
pub const WRITE_TEXT: ::core::ffi::c_int = 18 as ::core::ffi::c_int;
pub const XDV_ID_BYTE: ::core::ffi::c_int = 7 as ::core::ffi::c_int;
pub const SPX_ID_BYTE: ::core::ffi::c_int = 100 as ::core::ffi::c_int;
pub const SET1: ::core::ffi::c_int = 128 as ::core::ffi::c_int;
pub const SET_RULE: ::core::ffi::c_int = 132 as ::core::ffi::c_int;
pub const PUT_RULE: ::core::ffi::c_int = 137 as ::core::ffi::c_int;
pub const BOP: ::core::ffi::c_int = 139 as ::core::ffi::c_int;
pub const EOP: ::core::ffi::c_int = 140 as ::core::ffi::c_int;
pub const PUSH: ::core::ffi::c_int = 141 as ::core::ffi::c_int;
pub const POP: ::core::ffi::c_int = 142 as ::core::ffi::c_int;
pub const RIGHT1: ::core::ffi::c_int = 143 as ::core::ffi::c_int;
pub const DOWN1: ::core::ffi::c_int = 157 as ::core::ffi::c_int;
pub const FNT1: ::core::ffi::c_int = 235 as ::core::ffi::c_int;
pub const XXX1: ::core::ffi::c_int = 239 as ::core::ffi::c_int;
pub const XXX4: ::core::ffi::c_int = 242 as ::core::ffi::c_int;
pub const FNT_DEF1: ::core::ffi::c_int = 243 as ::core::ffi::c_int;
pub const PRE: ::core::ffi::c_int = 247 as ::core::ffi::c_int;
pub const POST: ::core::ffi::c_int = 248 as ::core::ffi::c_int;
pub const POST_POST: ::core::ffi::c_int = 249 as ::core::ffi::c_int;
pub const DEFINE_NATIVE_FONT: ::core::ffi::c_int = 252 as ::core::ffi::c_int;
pub const SET_GLYPHS: ::core::ffi::c_int = 253 as ::core::ffi::c_int;
pub const SET_TEXT_AND_GLYPHS: ::core::ffi::c_int = 254 as ::core::ffi::c_int;
pub const FONT_BASE: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const REVERSED: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const STRETCHING: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const DLIST: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const SHRINKING: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const TOO_BIG_CHAR: ::core::ffi::c_int = 65536 as ::core::ffi::c_int;
pub const CS_TOKEN_FLAG: ::core::ffi::c_int = 0x1ffffff as ::core::ffi::c_int;
pub const LEFT_BRACE_TOKEN: ::core::ffi::c_int = 0x200000 as ::core::ffi::c_int;
pub const RIGHT_BRACE_TOKEN: ::core::ffi::c_int = 0x400000 as ::core::ffi::c_int;
pub const MOV_NONE_SEEN: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const MOV_Y_HERE: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const MOV_Z_HERE: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const MOV_YZ_OK: ::core::ffi::c_int = 3;
pub const MOV_Y_OK: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
pub const MOV_Z_OK: ::core::ffi::c_int = 5;
pub const MOV_Y_SEEN: ::core::ffi::c_int = 6 as ::core::ffi::c_int;
pub const MOV_D_FIXED: ::core::ffi::c_int = 6 as ::core::ffi::c_int;
pub const MOV_Z_SEEN: ::core::ffi::c_int = 12 as ::core::ffi::c_int;
pub const AAT_FONT_FLAG: ::core::ffi::c_uint = 0xffff as ::core::ffi::c_uint;
pub const OTGR_FONT_FLAG: ::core::ffi::c_uint = 0xfffe as ::core::ffi::c_uint;
pub const MEM_TOP: ::core::ffi::c_int = 4999999 as ::core::ffi::c_int;
pub const INT_PAR__mag: ::core::ffi::c_int = 17 as ::core::ffi::c_int;
pub const INT_PAR__tracing_online: ::core::ffi::c_int = 29 as ::core::ffi::c_int;
pub const INT_PAR__tracing_output: ::core::ffi::c_int = 34 as ::core::ffi::c_int;
pub const INT_PAR__xetex_interword_space_shaping: ::core::ffi::c_int = 78 as ::core::ffi::c_int;
pub const INT_PARS: ::core::ffi::c_int = 89 as ::core::ffi::c_int;
pub const DIMEN_PAR__h_offset: ::core::ffi::c_int = 18 as ::core::ffi::c_int;
pub const DIMEN_PAR__v_offset: ::core::ffi::c_int = 19 as ::core::ffi::c_int;
pub const DIMEN_PAR__pdf_page_width: ::core::ffi::c_int = 21 as ::core::ffi::c_int;
pub const DIMEN_PAR__pdf_page_height: ::core::ffi::c_int = 22 as ::core::ffi::c_int;
pub const FROZEN_CONTROL_SEQUENCE: ::core::ffi::c_int = 2243226 as ::core::ffi::c_int;
pub const INT_BASE: ::core::ffi::c_int = 7826729 as ::core::ffi::c_int;
pub const COUNT_BASE: ::core::ffi::c_int = INT_BASE + INT_PARS;
pub const DEL_CODE_BASE: ::core::ffi::c_int = COUNT_BASE + 256 as ::core::ffi::c_int;
pub const DIMEN_BASE: ::core::ffi::c_int = DEL_CODE_BASE + NUMBER_USVS;
pub const HLIST_NODE: ::core::ffi::c_int = 0;
pub const VLIST_NODE: ::core::ffi::c_int = 1;
pub const RULE_NODE: ::core::ffi::c_int = 2;
pub const INS_NODE: ::core::ffi::c_int = 3 as ::core::ffi::c_int;
pub const MARK_NODE: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
pub const ADJUST_NODE: ::core::ffi::c_int = 5 as ::core::ffi::c_int;
pub const LIGATURE_NODE: ::core::ffi::c_int = 6;
pub const WHATSIT_NODE: ::core::ffi::c_int = 8;
pub const MATH_NODE: ::core::ffi::c_int = 9;
pub const GLUE_NODE: ::core::ffi::c_int = 10;
pub const KERN_NODE: ::core::ffi::c_int = 11 as ::core::ffi::c_int;
pub const PENALTY_NODE: ::core::ffi::c_int = 12 as ::core::ffi::c_int;
pub const STYLE_NODE: ::core::ffi::c_int = 14 as ::core::ffi::c_int;
pub const MARGIN_KERN_NODE: ::core::ffi::c_int = 40;
pub const A_LEADERS: ::core::ffi::c_int = 100 as ::core::ffi::c_int;
pub const C_LEADERS: ::core::ffi::c_int = 101 as ::core::ffi::c_int;
pub const SPACE_ADJUSTMENT: ::core::ffi::c_int = 3 as ::core::ffi::c_int;
pub const BEFORE: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const END_M_CODE: ::core::ffi::c_int = 3 as ::core::ffi::c_int;
pub const OPEN_NODE: ::core::ffi::c_int = 0;
pub const WRITE_NODE: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const CLOSE_NODE: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const SPECIAL_NODE: ::core::ffi::c_int = 3;
pub const LANGUAGE_NODE: ::core::ffi::c_int = 4;
pub const PDF_SAVE_POS_NODE: ::core::ffi::c_int = 21;
pub const NATIVE_WORD_NODE: ::core::ffi::c_int = 40 as ::core::ffi::c_int;
pub const NATIVE_WORD_NODE_AT: ::core::ffi::c_int = 41 as ::core::ffi::c_int;
pub const GLYPH_NODE: ::core::ffi::c_int = 42 as ::core::ffi::c_int;
pub const PIC_NODE: ::core::ffi::c_int = 43 as ::core::ffi::c_int;
pub const PDF_NODE: ::core::ffi::c_int = 44 as ::core::ffi::c_int;
#[inline]
unsafe extern "C" fn is_char_node(p: int32_t) -> bool {
    return p >= hi_mem_min;
}
#[inline]
unsafe extern "C" fn print_c_string(mut str: *const ::core::ffi::c_char) {
    while *str != 0 {
        let fresh8 = str;
        str = str.offset(1);
        print_char(*fresh8 as int32_t);
    }
}
#[inline]
unsafe extern "C" fn cur_length() -> pool_pointer {
    return pool_ptr - *str_start.offset((str_ptr - TOO_BIG_CHAR as str_number) as isize);
}
pub const DVI_BUF_SIZE: ::core::ffi::c_int = 16384 as ::core::ffi::c_int;
pub const HALF_BUF: ::core::ffi::c_int = 8192 as ::core::ffi::c_int;
pub const FNT_NUM_0: ::core::ffi::c_int = 171 as ::core::ffi::c_int;
static mut dvi_file: rust_output_handle_t =
    ::core::ptr::null::<ttbc_output_handle_t>() as *mut ttbc_output_handle_t;
static mut output_file_name: str_number = 0;
static mut dvi_buf: *mut eight_bits = ::core::ptr::null::<eight_bits>() as *mut eight_bits;
static mut dvi_limit: int32_t = 0;
static mut g: int32_t = 0;
static mut lq: int32_t = 0;
static mut lr: int32_t = 0;
static mut dvi_ptr: int32_t = 0;
static mut dvi_offset: int32_t = 0;
static mut dvi_gone: int32_t = 0;
static mut down_ptr: int32_t = 0;
static mut right_ptr: int32_t = 0;
static mut dvi_v: scaled_t = 0;
static mut dvi_h: scaled_t = 0;
static mut dvi_f: internal_font_number = 0;
static mut cur_s: int32_t = 0;
#[no_mangle]
pub unsafe extern "C" fn initialize_shipout_variables() {
    output_file_name = 0 as ::core::ffi::c_int as str_number;
    dvi_buf = malloc(
        ((16384 as ::core::ffi::c_int + 1 as ::core::ffi::c_int) as size_t)
            .wrapping_mul(::core::mem::size_of::<eight_bits>() as size_t),
    ) as *mut eight_bits;
    dvi_limit = DVI_BUF_SIZE as int32_t;
    dvi_ptr = 0 as ::core::ffi::c_int as int32_t;
    dvi_offset = 0 as ::core::ffi::c_int as int32_t;
    dvi_gone = 0 as ::core::ffi::c_int as int32_t;
    down_ptr = TEX_NULL as int32_t;
    right_ptr = TEX_NULL as int32_t;
    cur_s = -(1 as ::core::ffi::c_int) as int32_t;
}
#[no_mangle]
pub unsafe extern "C" fn deinitialize_shipout_variables() {
    free(dvi_buf as *mut ::core::ffi::c_void);
    dvi_buf = ::core::ptr::null_mut::<eight_bits>();
}
#[inline]
unsafe extern "C" fn dvi_out(mut c: eight_bits) {
    let fresh1 = dvi_ptr;
    dvi_ptr = dvi_ptr + 1;
    *dvi_buf.offset(fresh1 as isize) = c;
    if dvi_ptr == dvi_limit {
        dvi_swap();
    }
}
#[no_mangle]
pub unsafe extern "C" fn ship_out(mut p: int32_t) {
    let mut page_loc: int32_t = 0;
    let mut j: ::core::ffi::c_uchar = 0;
    let mut k: ::core::ffi::c_uchar = 0;
    let mut s: pool_pointer = 0;
    let mut old_setting: ::core::ffi::c_uchar = 0;
    let mut l: ::core::ffi::c_uchar = 0;
    let mut output_comment: *const ::core::ffi::c_char =
        b"tectonic\0" as *const u8 as *const ::core::ffi::c_char;
    synctex_sheet((*eqtb.offset((INT_BASE + INT_PAR__mag) as isize)).b32.s1);
    if job_name == 0 as str_number {
        open_log_file();
    }
    if (*eqtb.offset((INT_BASE + INT_PAR__tracing_output) as isize))
        .b32
        .s1
        > 0 as int32_t
    {
        print_nl_cstr(b"\0" as *const u8 as *const ::core::ffi::c_char);
        print_ln();
        print_cstr(b"Completed box being shipped out\0" as *const u8 as *const ::core::ffi::c_char);
    }
    if term_offset > max_print_line - 9 as int32_t {
        print_ln();
    } else if term_offset > 0 as int32_t || file_offset > 0 as int32_t {
        print_char(' ' as i32);
    }
    print_char('[' as i32);
    j = 9 as ::core::ffi::c_uchar;
    while j as ::core::ffi::c_int > 0 as ::core::ffi::c_int
        && (*eqtb.offset((COUNT_BASE + j as ::core::ffi::c_int) as isize))
            .b32
            .s1
            == 0 as int32_t
    {
        j = j.wrapping_sub(1);
    }
    k = 0 as ::core::ffi::c_uchar;
    while k as ::core::ffi::c_int <= j as ::core::ffi::c_int {
        print_int(
            (*eqtb.offset((COUNT_BASE + k as ::core::ffi::c_int) as isize))
                .b32
                .s1,
        );
        if (k as ::core::ffi::c_int) < j as ::core::ffi::c_int {
            print_char('.' as i32);
        }
        k = k.wrapping_add(1);
    }
    ttstub_output_flush(rust_stdout);
    if (*eqtb.offset((INT_BASE + INT_PAR__tracing_output) as isize))
        .b32
        .s1
        > 0 as int32_t
    {
        print_char(']' as i32);
        begin_diagnostic();
        show_box(p);
        end_diagnostic(true_0 != 0);
    }
    if (*mem.offset((p + 3 as int32_t) as isize)).b32.s1 > MAX_HALFWORD as int32_t
        || (*mem.offset((p + 2 as int32_t) as isize)).b32.s1 > MAX_HALFWORD as int32_t
        || (*mem.offset((p + 3 as int32_t) as isize)).b32.s1
            + (*mem.offset((p + 2 as int32_t) as isize)).b32.s1
            + (*eqtb.offset((DIMEN_BASE + DIMEN_PAR__v_offset) as isize))
                .b32
                .s1
            > MAX_HALFWORD as int32_t
        || (*mem.offset((p + 1 as int32_t) as isize)).b32.s1
            + (*eqtb.offset((DIMEN_BASE + DIMEN_PAR__h_offset) as isize))
                .b32
                .s1
            > MAX_HALFWORD as int32_t
    {
        error_here_with_diagnostic(
            b"Huge page cannot be shipped out\0" as *const u8 as *const ::core::ffi::c_char,
        );
        capture_to_diagnostic(::core::ptr::null_mut::<ttbc_diagnostic_t>());
        help_ptr = 2 as ::core::ffi::c_uchar;
        help_line[1 as ::core::ffi::c_int as usize] =
            b"The page just created is more than 18 feet tall or\0" as *const u8
                as *const ::core::ffi::c_char;
        help_line[0 as ::core::ffi::c_int as usize] =
            b"more than 18 feet wide, so I suspect something went wrong.\0" as *const u8
                as *const ::core::ffi::c_char;
        error();
        if (*eqtb.offset((INT_BASE + INT_PAR__tracing_output) as isize))
            .b32
            .s1
            <= 0 as int32_t
        {
            begin_diagnostic();
            print_nl_cstr(
                b"The following box has been deleted:\0" as *const u8 as *const ::core::ffi::c_char,
            );
            show_box(p);
            end_diagnostic(true_0 != 0);
        }
    } else {
        if (*mem.offset((p + 3 as int32_t) as isize)).b32.s1
            + (*mem.offset((p + 2 as int32_t) as isize)).b32.s1
            + (*eqtb.offset((DIMEN_BASE + DIMEN_PAR__v_offset) as isize))
                .b32
                .s1
            > max_v
        {
            max_v = ((*mem.offset((p + 3 as int32_t) as isize)).b32.s1
                + (*mem.offset((p + 2 as int32_t) as isize)).b32.s1
                + (*eqtb.offset((DIMEN_BASE + DIMEN_PAR__v_offset) as isize))
                    .b32
                    .s1) as scaled_t;
        }
        if (*mem.offset((p + 1 as int32_t) as isize)).b32.s1
            + (*eqtb.offset((DIMEN_BASE + DIMEN_PAR__h_offset) as isize))
                .b32
                .s1
            > max_h
        {
            max_h = ((*mem.offset((p + 1 as int32_t) as isize)).b32.s1
                + (*eqtb.offset((DIMEN_BASE + DIMEN_PAR__h_offset) as isize))
                    .b32
                    .s1) as scaled_t;
        }
        dvi_h = 0 as ::core::ffi::c_int as scaled_t;
        dvi_v = 0 as ::core::ffi::c_int as scaled_t;
        cur_h = (*eqtb.offset((DIMEN_BASE + DIMEN_PAR__h_offset) as isize))
            .b32
            .s1 as scaled_t;
        dvi_f = FONT_BASE as internal_font_number;
        cur_h_offset = ((*eqtb.offset((DIMEN_BASE + DIMEN_PAR__h_offset) as isize))
            .b32
            .s1
            + pitex_output_parameter(b"pdfhorigin\0".as_ptr().cast())) as scaled_t;
        cur_v_offset = ((*eqtb.offset((DIMEN_BASE + DIMEN_PAR__v_offset) as isize))
            .b32
            .s1
            + pitex_output_parameter(b"pdfvorigin\0".as_ptr().cast())) as scaled_t;
        if (*eqtb.offset((DIMEN_BASE + DIMEN_PAR__pdf_page_width) as isize))
            .b32
            .s1
            != 0 as int32_t
        {
            cur_page_width = (*eqtb.offset((DIMEN_BASE + DIMEN_PAR__pdf_page_width) as isize))
                .b32
                .s1 as scaled_t;
        } else {
            cur_page_width = ((*mem.offset((p + 1 as int32_t) as isize)).b32.s1
                + 2 as int32_t * cur_h_offset as int32_t) as scaled_t;
        }
        if (*eqtb.offset((DIMEN_BASE + DIMEN_PAR__pdf_page_height) as isize))
            .b32
            .s1
            != 0 as int32_t
        {
            cur_page_height = (*eqtb.offset((DIMEN_BASE + DIMEN_PAR__pdf_page_height) as isize))
                .b32
                .s1 as scaled_t;
        } else {
            cur_page_height = ((*mem.offset((p + 3 as int32_t) as isize)).b32.s1
                + (*mem.offset((p + 2 as int32_t) as isize)).b32.s1
                + 2 as int32_t * cur_v_offset as int32_t) as scaled_t;
        }
        if output_file_name == 0 as str_number {
            if job_name == 0 as str_number {
                open_log_file();
            }
            pack_job_name(output_file_extension);
            dvi_file = ttstub_output_open(name_of_file, 0 as ::core::ffi::c_int);
            if dvi_file.is_null() {
                _tt_abort(
                    b"cannot open output file \"%s\"\0" as *const u8 as *const ::core::ffi::c_char,
                    name_of_file,
                );
            }
            output_file_name = make_name_string();
        }
        if total_pages == 0 as int32_t {
            dvi_out(PRE as eight_bits);
            if semantic_pagination_enabled {
                dvi_out(SPX_ID_BYTE as eight_bits);
            } else {
                dvi_out(XDV_ID_BYTE as eight_bits);
            }
            dvi_four(25400000 as int32_t);
            dvi_four(473628672 as int32_t);
            prepare_mag();
            dvi_four((*eqtb.offset((INT_BASE + INT_PAR__mag) as isize)).b32.s1);
            l = strlen(output_comment) as ::core::ffi::c_uchar;
            dvi_out(l as eight_bits);
            s = 0 as ::core::ffi::c_int as pool_pointer;
            while s < l as pool_pointer {
                dvi_out(*output_comment.offset(s as isize) as eight_bits);
                s += 1;
            }
        }
        page_loc = dvi_offset + dvi_ptr;
        dvi_out(BOP as eight_bits);
        k = 0 as ::core::ffi::c_uchar;
        while (k as ::core::ffi::c_int) < 10 as ::core::ffi::c_int {
            dvi_four(
                (*eqtb.offset((COUNT_BASE + k as ::core::ffi::c_int) as isize))
                    .b32
                    .s1,
            );
            k = k.wrapping_add(1);
        }
        dvi_four(last_bop);
        last_bop = page_loc;
        old_setting = selector as ::core::ffi::c_uchar;
        selector = SELECTOR_NEW_STRING;
        print_cstr(b"pdf:pagesize \0" as *const u8 as *const ::core::ffi::c_char);
        if (*eqtb.offset((DIMEN_BASE + DIMEN_PAR__pdf_page_width) as isize))
            .b32
            .s1
            <= 0 as int32_t
            || (*eqtb.offset((DIMEN_BASE + DIMEN_PAR__pdf_page_height) as isize))
                .b32
                .s1
                <= 0 as int32_t
        {
            print_cstr(b"default\0" as *const u8 as *const ::core::ffi::c_char);
        } else {
            print_cstr(b"width\0" as *const u8 as *const ::core::ffi::c_char);
            print(' ' as i32);
            print_scaled(
                (*eqtb.offset((DIMEN_BASE + DIMEN_PAR__pdf_page_width) as isize))
                    .b32
                    .s1 as scaled_t,
            );
            print_cstr(b"pt\0" as *const u8 as *const ::core::ffi::c_char);
            print(' ' as i32);
            print_cstr(b"height\0" as *const u8 as *const ::core::ffi::c_char);
            print(' ' as i32);
            print_scaled(
                (*eqtb.offset((DIMEN_BASE + DIMEN_PAR__pdf_page_height) as isize))
                    .b32
                    .s1 as scaled_t,
            );
            print_cstr(b"pt\0" as *const u8 as *const ::core::ffi::c_char);
        }
        selector = old_setting as selector_t;
        dvi_out(XXX1 as eight_bits);
        dvi_out(cur_length() as eight_bits);
        s = *str_start.offset((str_ptr - TOO_BIG_CHAR as str_number) as isize);
        while s < pool_ptr {
            dvi_out(*str_pool.offset(s as isize) as eight_bits);
            s += 1;
        }
        pool_ptr = *str_start.offset((str_ptr - TOO_BIG_CHAR as str_number) as isize);
        for (name,command) in [(b"pdfpageattr\0" as &[u8], "pdf:put @thispage"), (b"pdfpageresources\0", "pdf:pitexresources")] {
            let value = pitex_page_tokens(name.as_ptr().cast());
            let body = std::ffi::CStr::from_ptr(value).to_string_lossy();
            if !body.is_empty() {
                let text = format!("{} << {} >>",command,body);
                dvi_out(XXX4 as eight_bits); dvi_four(text.len() as i32);
                for byte in text.bytes() { dvi_out(byte as eight_bits); }
            }
            pitex_page_tokens_free(value);
        }
        let settings = format!("pdf:pitexoutput {} {} {}", pitex_output_parameter(b"pdfmajorversion\0".as_ptr().cast()),pitex_output_parameter(b"pdfminorversion\0".as_ptr().cast()),pitex_output_parameter(b"pdfcompresslevel\0".as_ptr().cast()));
        let origin = format!("pdf:pitexorigin {} {}",pitex_output_parameter(b"pdfhorigin\0".as_ptr().cast()),pitex_output_parameter(b"pdfvorigin\0".as_ptr().cast()));
        dvi_out(XXX4 as eight_bits);dvi_four(origin.len() as i32);for byte in origin.bytes(){dvi_out(byte as eight_bits);}
        dvi_out(XXX4 as eight_bits); dvi_four(settings.len() as i32);
        for byte in settings.bytes() { dvi_out(byte as eight_bits); }
        let mut configuration=String::from("pdf:pitexconfig << ");
        for parameter in crate::backend_definitions::PARAMETERS {
            let name=std::ffi::CString::new(parameter.name).unwrap();
            if parameter.storage==crate::backend_definitions::Storage::Tokens {
                let value=pitex_page_tokens(name.as_ptr());let body=std::ffi::CStr::from_ptr(value).to_string_lossy();
                if !body.is_empty() || parameter.name=="pdfpkmode" {
                    let command=if parameter.name=="pdfpagesattr" {"pdf:put @pages"}else{"pdf:pitextokenconfig"};
                    let text=if parameter.name=="pdfpagesattr" {format!("{} << {} >>",command,body)}else{format!("{} {} ({})",command,parameter.name,body)};
                    dvi_out(XXX4 as eight_bits);dvi_four(text.len() as i32);for byte in text.bytes(){dvi_out(byte as eight_bits);}
                }
                pitex_page_tokens_free(value);
            } else {configuration.push_str(&format!("/{} {} ",parameter.name,pitex_output_parameter(name.as_ptr())));}
        }
        configuration.push_str(">>");dvi_out(XXX4 as eight_bits);dvi_four(configuration.len() as i32);for byte in configuration.bytes(){dvi_out(byte as eight_bits);}
        for parameter in crate::pdf_definitions::PARAMETERS {
            let name=std::ffi::CString::new(parameter.name).unwrap();let value=pitex_output_parameter(name.as_ptr());
            let value=if parameter.storage==crate::backend_definitions::Storage::Dimension {format!("{:.6}pt",value as f64/65536.0)}else{value.to_string()};
            let text=format!("pdf:{} {}",parameter.name.strip_prefix("pdf").unwrap_or(parameter.name),value);
            dvi_out(XXX4 as eight_bits);dvi_four(text.len() as i32);for byte in text.bytes(){dvi_out(byte as eight_bits);}
        }
        let glyphversion=pitex_output_parameter(b"PitexGlyphUnicodeVersion\0".as_ptr().cast());
        if glyphversion!=PITEX_LAST_EMITTED_GLYPH_VERSION {
            PITEX_LAST_EMITTED_GLYPH_VERSION=glyphversion;
        let glyphmap=pitex_page_tokens(b"PitexGlyphUnicodeMappings\0".as_ptr().cast());
        let glyphs=std::ffi::CStr::from_ptr(glyphmap).to_string_lossy();
        if !glyphs.is_empty() {
            let text=format!("pdf:pitexglyphmap {}",glyphs);
            dvi_out(XXX4 as eight_bits);dvi_four(text.len() as i32);for byte in text.bytes(){dvi_out(byte as eight_bits);}
        }
        pitex_page_tokens_free(glyphmap);
        }
        for font in crate::engine_fonts::pending_definitions() {
            if !*font_used.offset(font as isize) {dvi_font_def(font);*font_used.offset(font as isize)=true;}
        }
        pitex_shipout_page_reference(total_pages+1);
        // Page-global objects must survive an unused/discarded TeX box.
        for index in 0..pitex_global_special_count() {
            let text = std::ffi::CStr::from_ptr(pitex_global_special_at(index)).to_bytes();
            dvi_out(XXX4 as eight_bits);
            dvi_four(text.len() as int32_t);
            for byte in text { dvi_out(*byte as eight_bits); }
        }
        pitex_global_specials_clear();
        for index in 0..pitex_pending_form_count() {
            let form = pitex_pending_form_at(index);
            let saved_h = cur_h;
            let saved_v = cur_v;
            temp_ptr = form;
            hlist_out();
            cur_h = saved_h;
            cur_v = saved_v;
            flush_node_list(form);
        }
        pitex_pending_forms_clear();
        cur_v = ((*mem.offset((p + 3 as int32_t) as isize)).b32.s1
            + (*eqtb.offset((DIMEN_BASE + DIMEN_PAR__v_offset) as isize))
                .b32
                .s1) as scaled_t;
        temp_ptr = p;
        if (*mem.offset(p as isize)).b16.s1 as ::core::ffi::c_int == VLIST_NODE {
            vlist_out();
        } else {
            hlist_out();
        }
        dvi_eop();
        cur_s = -(1 as ::core::ffi::c_int) as int32_t;
    }
    if LR_problems > 0 as int32_t {
        print_ln();
        print_nl_cstr(b"\\endL or \\endR problem (\0" as *const u8 as *const ::core::ffi::c_char);
        print_int(LR_problems / 10000 as int32_t);
        print_cstr(b" missing, \0" as *const u8 as *const ::core::ffi::c_char);
        print_int(LR_problems % 10000 as int32_t);
        print_cstr(b" extra\0" as *const u8 as *const ::core::ffi::c_char);
        LR_problems = 0 as ::core::ffi::c_int as int32_t;
        print_char(')' as i32);
        print_ln();
    }
    if LR_ptr != TEX_NULL as int32_t || cur_dir as ::core::ffi::c_int != LEFT_TO_RIGHT {
        confusion(b"LR3\0" as *const u8 as *const ::core::ffi::c_char);
    }
    if (*eqtb.offset((INT_BASE + INT_PAR__tracing_output) as isize))
        .b32
        .s1
        <= 0 as int32_t
    {
        print_char(']' as i32);
    }
    dead_cycles = 0 as ::core::ffi::c_int as int32_t;
    ttstub_output_flush(rust_stdout);
    flush_node_list(p);
    synctex_teehs();
}
unsafe fn pitex_box_special(level:i32,kind:&str,node:i32,begin:bool) {
    let text=if begin {format!("pdf:boxbegin {} {} bbox 0pt {:.6}pt {:.6}pt {:.6}pt",level,kind,-((*mem.offset((node+2) as isize)).b32.s1 as f64/65536.0),(*mem.offset((node+1) as isize)).b32.s1 as f64/65536.0,(*mem.offset((node+3) as isize)).b32.s1 as f64/65536.0)}else{format!("pdf:boxend {}",level)};
    if begin {
        if cur_h!=dvi_h {movement(cur_h-dvi_h,RIGHT1 as eight_bits);dvi_h=cur_h;}
        if cur_v!=dvi_v {movement(cur_v-dvi_v,DOWN1 as eight_bits);dvi_v=cur_v;}
    }
    dvi_out(XXX4 as eight_bits);dvi_four(text.len() as i32);for byte in text.bytes(){dvi_out(byte as eight_bits);}
}
unsafe extern "C" fn hlist_out() {
    let mut current_block: u64;
    let mut base_line: scaled_t = 0;
    let mut left_edge: scaled_t = 0;
    let mut save_h: scaled_t = 0;
    let mut save_v: scaled_t = 0;
    let mut this_box: int32_t = 0;
    let mut g_order: glue_ord = 0;
    let mut g_sign: ::core::ffi::c_uchar = 0;
    let mut p: int32_t = 0;
    let mut save_loc: int32_t = 0;
    let mut leader_box: int32_t = 0;
    let mut leader_wd: scaled_t = 0;
    let mut lx: scaled_t = 0;
    let mut outer_doing_leaders: bool = false;
    let mut edge: scaled_t = 0;
    let mut prev_p: int32_t = 0;
    let mut len: int32_t = 0;
    let mut q: int32_t = 0;
    let mut r: int32_t = 0;
    let mut k: int32_t = 0;
    let mut j: int32_t = 0;
    let mut glue_temp: ::core::ffi::c_double = 0.;
    let mut cur_glue: ::core::ffi::c_double = 0.;
    let mut cur_g: scaled_t = 0;
    let mut c: uint16_t = 0;
    let mut f: internal_font_number = 0;
    cur_g = 0 as ::core::ffi::c_int as scaled_t;
    cur_glue = 0.0f64;
    this_box = temp_ptr;
    g_order = (*mem.offset((this_box + 5 as int32_t) as isize)).b16.s0 as glue_ord;
    g_sign = (*mem.offset((this_box + 5 as int32_t) as isize)).b16.s1 as ::core::ffi::c_uchar;
    if (*eqtb.offset((INT_BASE + INT_PAR__xetex_interword_space_shaping) as isize))
        .b32
        .s1
        > 1 as int32_t
    {
        p = (*mem.offset((this_box + 5 as int32_t) as isize)).b32.s1;
        prev_p = this_box + 5 as int32_t;
        while p != TEX_NULL as int32_t {
            if (*mem.offset(p as isize)).b32.s1 != TEX_NULL as int32_t {
                if p != TEX_NULL as int32_t
                    && !is_char_node(p)
                    && (*mem.offset(p as isize)).b16.s1 as ::core::ffi::c_int == WHATSIT_NODE
                    && ((*mem.offset(p as isize)).b16.s0 as ::core::ffi::c_int == NATIVE_WORD_NODE
                        || (*mem.offset(p as isize)).b16.s0 as ::core::ffi::c_int
                            == NATIVE_WORD_NODE_AT)
                    && *font_letter_space
                        .offset((*mem.offset((p + 4 as int32_t) as isize)).b16.s2 as isize)
                        == 0 as scaled_t
                {
                    r = p;
                    k = (*mem.offset((r + 4 as int32_t) as isize)).b16.s1 as int32_t;
                    q = (*mem.offset(p as isize)).b32.s1;
                    loop {
                        while q != TEX_NULL as int32_t
                            && !is_char_node(q)
                            && ((*mem.offset(q as isize)).b16.s1 as ::core::ffi::c_int
                                == PENALTY_NODE
                                || (*mem.offset(q as isize)).b16.s1 as ::core::ffi::c_int
                                    == INS_NODE
                                || (*mem.offset(q as isize)).b16.s1 as ::core::ffi::c_int
                                    == MARK_NODE
                                || (*mem.offset(q as isize)).b16.s1 as ::core::ffi::c_int
                                    == ADJUST_NODE
                                || (*mem.offset(q as isize)).b16.s1 as ::core::ffi::c_int
                                    == WHATSIT_NODE
                                    && (*mem.offset(q as isize)).b16.s0 as ::core::ffi::c_int
                                        <= 4 as ::core::ffi::c_int)
                        {
                            q = (*mem.offset(q as isize)).b32.s1;
                        }
                        if !(q != TEX_NULL as int32_t && !is_char_node(q)) {
                            break;
                        }
                        if (*mem.offset(q as isize)).b16.s1 as ::core::ffi::c_int == GLUE_NODE
                            && (*mem.offset(q as isize)).b16.s0 as ::core::ffi::c_int == NORMAL
                        {
                            if (*mem.offset((q + 1 as int32_t) as isize)).b32.s0
                                == *font_glue.offset(
                                    (*mem.offset((r + 4 as int32_t) as isize)).b16.s2 as isize,
                                )
                            {
                                q = (*mem.offset(q as isize)).b32.s1;
                                while q != TEX_NULL as int32_t
                                    && !is_char_node(q)
                                    && ((*mem.offset(q as isize)).b16.s1 as ::core::ffi::c_int
                                        == PENALTY_NODE
                                        || (*mem.offset(q as isize)).b16.s1 as ::core::ffi::c_int
                                            == INS_NODE
                                        || (*mem.offset(q as isize)).b16.s1 as ::core::ffi::c_int
                                            == MARK_NODE
                                        || (*mem.offset(q as isize)).b16.s1 as ::core::ffi::c_int
                                            == ADJUST_NODE
                                        || (*mem.offset(q as isize)).b16.s1 as ::core::ffi::c_int
                                            == WHATSIT_NODE
                                            && (*mem.offset(q as isize)).b16.s0
                                                as ::core::ffi::c_int
                                                <= 4 as ::core::ffi::c_int)
                                {
                                    q = (*mem.offset(q as isize)).b32.s1;
                                }
                                if q != TEX_NULL as int32_t
                                    && !is_char_node(q)
                                    && (*mem.offset(q as isize)).b16.s1 as ::core::ffi::c_int
                                        == WHATSIT_NODE
                                    && ((*mem.offset(q as isize)).b16.s0 as ::core::ffi::c_int
                                        == NATIVE_WORD_NODE
                                        || (*mem.offset(q as isize)).b16.s0 as ::core::ffi::c_int
                                            == NATIVE_WORD_NODE_AT)
                                    && (*mem.offset((q + 4 as int32_t) as isize)).b16.s2
                                        as ::core::ffi::c_int
                                        == (*mem.offset((r + 4 as int32_t) as isize)).b16.s2
                                            as ::core::ffi::c_int
                                {
                                    p = q;
                                    k = (k as ::core::ffi::c_int
                                        + (1 as ::core::ffi::c_int
                                            + (*mem.offset((q + 4 as int32_t) as isize)).b16.s1
                                                as ::core::ffi::c_int))
                                        as int32_t;
                                    q = (*mem.offset(q as isize)).b32.s1;
                                    continue;
                                }
                            } else {
                                q = (*mem.offset(q as isize)).b32.s1;
                            }
                            if !(q != TEX_NULL as int32_t
                                && !is_char_node(q)
                                && (*mem.offset(q as isize)).b16.s1 as ::core::ffi::c_int
                                    == KERN_NODE
                                && (*mem.offset(q as isize)).b16.s0 as ::core::ffi::c_int
                                    == SPACE_ADJUSTMENT)
                            {
                                break;
                            }
                            q = (*mem.offset(q as isize)).b32.s1;
                            while q != TEX_NULL as int32_t
                                && !is_char_node(q)
                                && ((*mem.offset(q as isize)).b16.s1 as ::core::ffi::c_int
                                    == PENALTY_NODE
                                    || (*mem.offset(q as isize)).b16.s1 as ::core::ffi::c_int
                                        == INS_NODE
                                    || (*mem.offset(q as isize)).b16.s1 as ::core::ffi::c_int
                                        == MARK_NODE
                                    || (*mem.offset(q as isize)).b16.s1 as ::core::ffi::c_int
                                        == ADJUST_NODE
                                    || (*mem.offset(q as isize)).b16.s1 as ::core::ffi::c_int
                                        == WHATSIT_NODE
                                        && (*mem.offset(q as isize)).b16.s0 as ::core::ffi::c_int
                                            <= 4 as ::core::ffi::c_int)
                            {
                                q = (*mem.offset(q as isize)).b32.s1;
                            }
                            if !(q != TEX_NULL as int32_t
                                && !is_char_node(q)
                                && (*mem.offset(q as isize)).b16.s1 as ::core::ffi::c_int
                                    == WHATSIT_NODE
                                && ((*mem.offset(q as isize)).b16.s0 as ::core::ffi::c_int
                                    == NATIVE_WORD_NODE
                                    || (*mem.offset(q as isize)).b16.s0 as ::core::ffi::c_int
                                        == NATIVE_WORD_NODE_AT)
                                && (*mem.offset((q + 4 as int32_t) as isize)).b16.s2
                                    as ::core::ffi::c_int
                                    == (*mem.offset((r + 4 as int32_t) as isize)).b16.s2
                                        as ::core::ffi::c_int)
                            {
                                break;
                            }
                            p = q;
                            k = (k as ::core::ffi::c_int
                                + (1 as ::core::ffi::c_int
                                    + (*mem.offset((q + 4 as int32_t) as isize)).b16.s1
                                        as ::core::ffi::c_int))
                                as int32_t;
                            q = (*mem.offset(q as isize)).b32.s1;
                        } else {
                            if !(q != TEX_NULL as int32_t
                                && !is_char_node(q)
                                && (*mem.offset(q as isize)).b16.s1 as ::core::ffi::c_int
                                    == WHATSIT_NODE
                                && ((*mem.offset(q as isize)).b16.s0 as ::core::ffi::c_int
                                    == NATIVE_WORD_NODE
                                    || (*mem.offset(q as isize)).b16.s0 as ::core::ffi::c_int
                                        == NATIVE_WORD_NODE_AT)
                                && (*mem.offset((q + 4 as int32_t) as isize)).b16.s2
                                    as ::core::ffi::c_int
                                    == (*mem.offset((r + 4 as int32_t) as isize)).b16.s2
                                        as ::core::ffi::c_int)
                            {
                                break;
                            }
                            p = q;
                            q = (*mem.offset(q as isize)).b32.s1;
                        }
                    }
                    if p != r {
                        if pool_ptr + k as pool_pointer > pool_size {
                            overflow(
                                b"pool size\0" as *const u8 as *const ::core::ffi::c_char,
                                pool_size - init_pool_ptr as int32_t,
                            );
                        }
                        k = 0 as ::core::ffi::c_int as int32_t;
                        q = r;
                        loop {
                            if (*mem.offset(q as isize)).b16.s1 as ::core::ffi::c_int
                                == WHATSIT_NODE
                            {
                                if (*mem.offset(q as isize)).b16.s0 as ::core::ffi::c_int
                                    == NATIVE_WORD_NODE
                                    || (*mem.offset(q as isize)).b16.s0 as ::core::ffi::c_int
                                        == NATIVE_WORD_NODE_AT
                                {
                                    j = 0 as ::core::ffi::c_int as int32_t;
                                    while j
                                        < (*mem.offset((q + 4 as int32_t) as isize)).b16.s1
                                            as int32_t
                                    {
                                        *str_pool.offset(pool_ptr as isize) = *(mem
                                            .offset((q + NATIVE_NODE_SIZE as int32_t) as isize)
                                            as *mut memory_word
                                            as *mut ::core::ffi::c_ushort)
                                            .offset(j as isize)
                                            as packed_UTF16_code;
                                        pool_ptr += 1;
                                        j += 1;
                                    }
                                    k += (*mem.offset((q + 1 as int32_t) as isize)).b32.s1;
                                }
                            } else if (*mem.offset(q as isize)).b16.s1 as ::core::ffi::c_int
                                == GLUE_NODE
                            {
                                *str_pool.offset(pool_ptr as isize) =
                                    ' ' as i32 as packed_UTF16_code;
                                pool_ptr += 1;
                                g = (*mem.offset((q + 1 as int32_t) as isize)).b32.s0;
                                k += (*mem.offset((g + 1 as int32_t) as isize)).b32.s1;
                                if g_sign as ::core::ffi::c_int != NORMAL {
                                    if g_sign as ::core::ffi::c_int == STRETCHING {
                                        if (*mem.offset(g as isize)).b16.s1 as ::core::ffi::c_int
                                            == g_order as ::core::ffi::c_int
                                        {
                                            k += tex_round(
                                                (*mem.offset((this_box + 6 as int32_t) as isize))
                                                    .gr
                                                    * (*mem.offset((g + 2 as int32_t) as isize))
                                                        .b32
                                                        .s1
                                                        as ::core::ffi::c_double,
                                            );
                                        }
                                    } else if (*mem.offset(g as isize)).b16.s0 as ::core::ffi::c_int
                                        == g_order as ::core::ffi::c_int
                                    {
                                        k -= tex_round(
                                            (*mem.offset((this_box + 6 as int32_t) as isize)).gr
                                                * (*mem.offset((g + 3 as int32_t) as isize)).b32.s1
                                                    as ::core::ffi::c_double,
                                        );
                                    }
                                }
                            } else if (*mem.offset(q as isize)).b16.s1 as ::core::ffi::c_int
                                == KERN_NODE
                            {
                                k += (*mem.offset((q + 1 as int32_t) as isize)).b32.s1;
                            }
                            if q == p {
                                break;
                            }
                            q = (*mem.offset(q as isize)).b32.s1;
                        }
                        q = new_native_word_node(
                            (*mem.offset((r + 4 as int32_t) as isize)).b16.s2
                                as internal_font_number,
                            cur_length() as int32_t,
                        );
                        (*mem.offset(q as isize)).b16.s0 = (*mem.offset(r as isize)).b16.s0;
                        j = 0 as ::core::ffi::c_int as int32_t;
                        while j < cur_length() {
                            *(mem.offset((q + NATIVE_NODE_SIZE as int32_t) as isize)
                                as *mut memory_word
                                as *mut ::core::ffi::c_ushort)
                                .offset(j as isize) = *str_pool.offset(
                                (*str_start.offset((str_ptr - TOO_BIG_CHAR as str_number) as isize)
                                    + j as pool_pointer) as isize,
                            )
                                as ::core::ffi::c_ushort;
                            j += 1;
                        }
                        (*mem.offset((q + 1 as int32_t) as isize)).b32.s1 = k;
                        store_justified_native_glyphs(
                            mem.offset(q as isize) as *mut memory_word as *mut ::core::ffi::c_void
                        );
                        (*mem.offset(prev_p as isize)).b32.s1 = q;
                        (*mem.offset(q as isize)).b32.s1 = (*mem.offset(p as isize)).b32.s1;
                        (*mem.offset(p as isize)).b32.s1 = TEX_NULL as int32_t;
                        prev_p = r;
                        p = (*mem.offset(r as isize)).b32.s1;
                        while p != TEX_NULL as int32_t {
                            if !is_char_node(p)
                                && ((*mem.offset(p as isize)).b16.s1 as ::core::ffi::c_int
                                    == PENALTY_NODE
                                    || (*mem.offset(p as isize)).b16.s1 as ::core::ffi::c_int
                                        == INS_NODE
                                    || (*mem.offset(p as isize)).b16.s1 as ::core::ffi::c_int
                                        == MARK_NODE
                                    || (*mem.offset(p as isize)).b16.s1 as ::core::ffi::c_int
                                        == ADJUST_NODE
                                    || (*mem.offset(p as isize)).b16.s1 as ::core::ffi::c_int
                                        == WHATSIT_NODE
                                        && (*mem.offset(p as isize)).b16.s0 as ::core::ffi::c_int
                                            <= 4 as ::core::ffi::c_int)
                            {
                                (*mem.offset(prev_p as isize)).b32.s1 =
                                    (*mem.offset(p as isize)).b32.s1;
                                (*mem.offset(p as isize)).b32.s1 = (*mem.offset(q as isize)).b32.s1;
                                (*mem.offset(q as isize)).b32.s1 = p;
                                q = p;
                            }
                            prev_p = p;
                            p = (*mem.offset(p as isize)).b32.s1;
                        }
                        flush_node_list(r);
                        pool_ptr =
                            *str_start.offset((str_ptr - TOO_BIG_CHAR as str_number) as isize);
                        p = q;
                    }
                }
                prev_p = p;
            }
            p = (*mem.offset(p as isize)).b32.s1;
        }
    }
    p = (*mem.offset((this_box + 5 as int32_t) as isize)).b32.s1;
    cur_s += 1;
    if cur_s > 0 as int32_t {
        dvi_out(PUSH as eight_bits);
    }
    save_loc = dvi_offset + dvi_ptr;
    pitex_box_special(cur_s,"h",this_box,true);
    if cur_s > max_push {
        max_push = cur_s;
    }
    base_line = cur_v;
    prev_p = this_box + 5 as int32_t;
    temp_ptr = get_avail();
    (*mem.offset(temp_ptr as isize)).b32.s0 = BEFORE as int32_t;
    (*mem.offset(temp_ptr as isize)).b32.s1 = LR_ptr;
    LR_ptr = temp_ptr;
    if (*mem.offset(this_box as isize)).b16.s0 as ::core::ffi::c_int == DLIST {
        if cur_dir as ::core::ffi::c_int == RIGHT_TO_LEFT {
            cur_dir = LEFT_TO_RIGHT as small_number;
            cur_h -= (*mem.offset((this_box + 1 as int32_t) as isize)).b32.s1;
        } else {
            (*mem.offset(this_box as isize)).b16.s0 = 0 as uint16_t;
        }
    }
    if cur_dir as ::core::ffi::c_int == RIGHT_TO_LEFT
        && (*mem.offset(this_box as isize)).b16.s0 as ::core::ffi::c_int != REVERSED
    {
        save_h = cur_h;
        temp_ptr = p;
        p = new_kern(0 as scaled_t);
        (*mem.offset((p + 3 as int32_t - SYNCTEX_FIELD_SIZE as int32_t) as isize))
            .b32
            .s0 = 0 as ::core::ffi::c_int as int32_t;
        (*mem.offset(prev_p as isize)).b32.s1 = p;
        cur_h = 0 as ::core::ffi::c_int as scaled_t;
        (*mem.offset(p as isize)).b32.s1 = reverse(
            this_box,
            TEX_NULL as int32_t,
            &raw mut cur_g,
            &raw mut cur_glue,
        );
        (*mem.offset((p + 1 as int32_t) as isize)).b32.s1 = -cur_h as int32_t;
        cur_h = save_h;
        (*mem.offset(this_box as isize)).b16.s0 = REVERSED as uint16_t;
    }
    left_edge = cur_h;
    synctex_hlist(this_box);
    's_591: while p != TEX_NULL as int32_t {
        loop {
            if is_char_node(p) {
                if cur_h != dvi_h {
                    movement(cur_h - dvi_h, RIGHT1 as eight_bits);
                    dvi_h = cur_h;
                }
                if cur_v != dvi_v {
                    movement(cur_v - dvi_v, DOWN1 as eight_bits);
                    dvi_v = cur_v;
                }
                loop {
                    f = (*mem.offset(p as isize)).b16.s1 as internal_font_number;
                    c = (*mem.offset(p as isize)).b16.s0;
                    if p != LIG_TRICK as int32_t && !(*font_mapping.offset(f as isize)).is_null() {
                        c = apply_tfm_font_mapping(
                            *font_mapping.offset(f as isize),
                            c as ::core::ffi::c_int,
                        ) as uint16_t;
                    }
                    if f != dvi_f {
                        if !*font_used.offset(f as isize) {
                            dvi_font_def(f);
                            *font_used.offset(f as isize) = true_0 != 0;
                        }
                        if f <= 64 as internal_font_number {
                            dvi_out(
                                (f + FNT_NUM_0 as internal_font_number - 1 as internal_font_number)
                                    as eight_bits,
                            );
                        } else if f <= 256 as internal_font_number {
                            dvi_out(FNT1 as eight_bits);
                            dvi_out((f - 1 as internal_font_number) as eight_bits);
                        } else {
                            dvi_out((FNT1 + 1 as ::core::ffi::c_int) as eight_bits);
                            dvi_out(
                                ((f - 1 as internal_font_number) / 256 as internal_font_number)
                                    as eight_bits,
                            );
                            dvi_out(
                                ((f - 1 as internal_font_number) % 256 as internal_font_number)
                                    as eight_bits,
                            );
                        }
                        dvi_f = f;
                    }
                    if *font_ec.offset(f as isize) as ::core::ffi::c_int >= c as ::core::ffi::c_int
                    {
                        if *font_bc.offset(f as isize) as ::core::ffi::c_int
                            <= c as ::core::ffi::c_int
                        {
                            if (*font_info
                                .offset((*char_base.offset(f as isize) + c as int32_t) as isize))
                            .b16
                            .s3 as ::core::ffi::c_int
                                > 0 as ::core::ffi::c_int
                            {
                                if c as ::core::ffi::c_int >= 128 as ::core::ffi::c_int {
                                    dvi_out(SET1 as eight_bits);
                                }
                                dvi_out(c as eight_bits);
                                cur_h += (*font_info.offset(
                                    (*width_base.offset(f as isize)
                                        + (*font_info.offset(
                                            (*char_base.offset(f as isize) + c as int32_t) as isize,
                                        ))
                                        .b16
                                        .s3 as int32_t)
                                        as isize,
                                ))
                                .b32
                                .s1;
                            }
                        }
                    }
                    prev_p = (*mem.offset(prev_p as isize)).b32.s1;
                    p = (*mem.offset(p as isize)).b32.s1;
                    if !is_char_node(p) {
                        break;
                    }
                }
                synctex_current();
                dvi_h = cur_h;
                continue 's_591;
            } else {
                match (*mem.offset(p as isize)).b16.s1 as ::core::ffi::c_int {
                    HLIST_NODE | VLIST_NODE => {
                        if (*mem.offset((p + 5 as int32_t) as isize)).b32.s1 == TEX_NULL as int32_t
                        {
                            if (*mem.offset(p as isize)).b16.s1 as ::core::ffi::c_int == VLIST_NODE
                            {
                                synctex_void_vlist(p, this_box);
                            } else {
                                synctex_void_hlist(p, this_box);
                            }
                            cur_h += (*mem.offset((p + 1 as int32_t) as isize)).b32.s1;
                        } else {
                            save_h = dvi_h;
                            save_v = dvi_v;
                            cur_v = base_line
                                + (*mem.offset((p + 4 as int32_t) as isize)).b32.s1 as scaled_t;
                            temp_ptr = p;
                            edge = cur_h
                                + (*mem.offset((p + 1 as int32_t) as isize)).b32.s1 as scaled_t;
                            if cur_dir as ::core::ffi::c_int == RIGHT_TO_LEFT {
                                cur_h = edge;
                            }
                            if (*mem.offset(p as isize)).b16.s1 as ::core::ffi::c_int == VLIST_NODE
                            {
                                vlist_out();
                            } else {
                                hlist_out();
                            }
                            dvi_h = save_h;
                            dvi_v = save_v;
                            cur_h = edge;
                            cur_v = base_line;
                        }
                        current_block = 10791422562291087306;
                        break;
                    }
                    RULE_NODE => {
                        rule_ht = (*mem.offset((p + 3 as int32_t) as isize)).b32.s1 as scaled_t;
                        rule_dp = (*mem.offset((p + 2 as int32_t) as isize)).b32.s1 as scaled_t;
                        rule_wd = (*mem.offset((p + 1 as int32_t) as isize)).b32.s1 as scaled_t;
                        current_block = 1347935348126480785;
                        break;
                    }
                    WHATSIT_NODE => {
                        match (*mem.offset(p as isize)).b16.s0 as ::core::ffi::c_int {
                            NATIVE_WORD_NODE | NATIVE_WORD_NODE_AT | GLYPH_NODE => {
                                if cur_h != dvi_h {
                                    movement(cur_h - dvi_h, RIGHT1 as eight_bits);
                                    dvi_h = cur_h;
                                }
                                if cur_v != dvi_v {
                                    movement(cur_v - dvi_v, DOWN1 as eight_bits);
                                    dvi_v = cur_v;
                                }
                                f = (*mem.offset((p + 4 as int32_t) as isize)).b16.s2
                                    as internal_font_number;
                                if f != dvi_f {
                                    if !*font_used.offset(f as isize) {
                                        dvi_font_def(f);
                                        *font_used.offset(f as isize) = true_0 != 0;
                                    }
                                    if f <= 64 as internal_font_number {
                                        dvi_out((f + 170 as internal_font_number) as eight_bits);
                                    } else if f <= 256 as internal_font_number {
                                        dvi_out(FNT1 as eight_bits);
                                        dvi_out((f - 1 as internal_font_number) as eight_bits);
                                    } else {
                                        dvi_out((FNT1 + 1 as ::core::ffi::c_int) as eight_bits);
                                        dvi_out(
                                            ((f - 1 as internal_font_number)
                                                / 256 as internal_font_number)
                                                as eight_bits,
                                        );
                                        dvi_out(
                                            ((f - 1 as internal_font_number)
                                                % 256 as internal_font_number)
                                                as eight_bits,
                                        );
                                    }
                                    dvi_f = f;
                                }
                                if (*mem.offset(p as isize)).b16.s0 as ::core::ffi::c_int
                                    == GLYPH_NODE
                                {
                                    dvi_out(SET_GLYPHS as eight_bits);
                                    dvi_four((*mem.offset((p + 1 as int32_t) as isize)).b32.s1);
                                    dvi_two(1 as UTF16_code);
                                    dvi_four(0 as int32_t);
                                    dvi_four(0 as int32_t);
                                    dvi_two(
                                        (*mem.offset((p + 4 as int32_t) as isize)).b16.s1
                                            as UTF16_code,
                                    );
                                    cur_h += (*mem.offset((p + 1 as int32_t) as isize)).b32.s1;
                                } else {
                                    if (*mem.offset(p as isize)).b16.s0 as ::core::ffi::c_int
                                        == NATIVE_WORD_NODE_AT
                                    {
                                        if (*mem.offset((p + 4 as int32_t) as isize)).b16.s1
                                            as ::core::ffi::c_int
                                            > 0 as ::core::ffi::c_int
                                            || !(*mem.offset((p + 5 as int32_t) as isize))
                                                .ptr
                                                .is_null()
                                        {
                                            dvi_out(SET_TEXT_AND_GLYPHS as eight_bits);
                                            len = (*mem.offset((p + 4 as int32_t) as isize)).b16.s1
                                                as int32_t;
                                            dvi_two(len as UTF16_code);
                                            k = 0 as ::core::ffi::c_int as int32_t;
                                            while k < len {
                                                dvi_two(
                                                    *(mem.offset(
                                                        (p + NATIVE_NODE_SIZE as int32_t) as isize,
                                                    )
                                                        as *mut memory_word
                                                        as *mut ::core::ffi::c_ushort)
                                                        .offset(k as isize)
                                                        as UTF16_code,
                                                );
                                                k += 1;
                                            }
                                            len = makeXDVGlyphArrayData(mem.offset(p as isize)
                                                as *mut memory_word
                                                as *mut ::core::ffi::c_void)
                                                as int32_t;
                                            k = 0 as ::core::ffi::c_int as int32_t;
                                            while k < len {
                                                dvi_out(
                                                    *xdv_buffer.offset(k as isize) as eight_bits
                                                );
                                                k += 1;
                                            }
                                        }
                                    } else if !(*mem.offset((p + 5 as int32_t) as isize))
                                        .ptr
                                        .is_null()
                                    {
                                        dvi_out(SET_GLYPHS as eight_bits);
                                        len = makeXDVGlyphArrayData(mem.offset(p as isize)
                                            as *mut memory_word
                                            as *mut ::core::ffi::c_void)
                                            as int32_t;
                                        k = 0 as ::core::ffi::c_int as int32_t;
                                        while k < len {
                                            dvi_out(*xdv_buffer.offset(k as isize) as eight_bits);
                                            k += 1;
                                        }
                                    }
                                    cur_h += (*mem.offset((p + 1 as int32_t) as isize)).b32.s1;
                                }
                                dvi_h = cur_h;
                            }
                            PIC_NODE | PDF_NODE => {
                                save_h = dvi_h;
                                save_v = dvi_v;
                                cur_v = base_line;
                                edge = cur_h
                                    + (*mem.offset((p + 1 as int32_t) as isize)).b32.s1 as scaled_t;
                                pic_out(p);
                                dvi_h = save_h;
                                dvi_v = save_v;
                                cur_h = edge;
                                cur_v = base_line;
                            }
                            PDF_SAVE_POS_NODE => {
                                pdf_last_x_pos = (cur_h as ::core::ffi::c_long
                                    + 4736286 as ::core::ffi::c_long)
                                    as int32_t;
                                pdf_last_y_pos = ((cur_page_height - cur_v) as ::core::ffi::c_long
                                    - 4736286 as ::core::ffi::c_long)
                                    as int32_t;
                            }
                            _ => {
                                out_what(p);
                            }
                        }
                        current_block = 10791422562291087306;
                        break;
                    }
                    GLUE_NODE => {
                        g = (*mem.offset((p + 1 as int32_t) as isize)).b32.s0;
                        rule_wd = ((*mem.offset((g + 1 as int32_t) as isize)).b32.s1
                            - cur_g as int32_t) as scaled_t;
                        if g_sign as ::core::ffi::c_int != NORMAL {
                            if g_sign as ::core::ffi::c_int == STRETCHING {
                                if (*mem.offset(g as isize)).b16.s1 as ::core::ffi::c_int
                                    == g_order as ::core::ffi::c_int
                                {
                                    cur_glue += (*mem.offset((g + 2 as int32_t) as isize)).b32.s1
                                        as ::core::ffi::c_double;
                                    glue_temp = (*mem.offset((this_box + 6 as int32_t) as isize))
                                        .gr
                                        * cur_glue;
                                    if glue_temp > 1000000000.0f64 {
                                        glue_temp = 1000000000.0f64;
                                    } else if glue_temp < -1000000000.0f64 {
                                        glue_temp = -1000000000.0f64;
                                    }
                                    cur_g = tex_round(glue_temp) as scaled_t;
                                }
                            } else if (*mem.offset(g as isize)).b16.s0 as ::core::ffi::c_int
                                == g_order as ::core::ffi::c_int
                            {
                                cur_glue -= (*mem.offset((g + 3 as int32_t) as isize)).b32.s1
                                    as ::core::ffi::c_double;
                                glue_temp =
                                    (*mem.offset((this_box + 6 as int32_t) as isize)).gr * cur_glue;
                                if glue_temp > 1000000000.0f64 {
                                    glue_temp = 1000000000.0f64;
                                } else if glue_temp < -1000000000.0f64 {
                                    glue_temp = -1000000000.0f64;
                                }
                                cur_g = tex_round(glue_temp) as scaled_t;
                            }
                        }
                        rule_wd += cur_g;
                        if g_sign as ::core::ffi::c_int == STRETCHING
                            && (*mem.offset(g as isize)).b16.s1 as ::core::ffi::c_int
                                == g_order as ::core::ffi::c_int
                            || g_sign as ::core::ffi::c_int == SHRINKING
                                && (*mem.offset(g as isize)).b16.s0 as ::core::ffi::c_int
                                    == g_order as ::core::ffi::c_int
                        {
                            if (*mem.offset(g as isize)).b32.s1 == TEX_NULL as int32_t {
                                free_node(g, GLUE_SPEC_SIZE as int32_t);
                            } else {
                                let ref mut fresh2 = (*mem.offset(g as isize)).b32.s1;
                                *fresh2 -= 1;
                            }
                            if ((*mem.offset(p as isize)).b16.s0 as ::core::ffi::c_int) < A_LEADERS
                            {
                                (*mem.offset(p as isize)).b16.s1 = KERN_NODE as uint16_t;
                                (*mem.offset((p + 1 as int32_t) as isize)).b32.s1 =
                                    rule_wd as int32_t;
                            } else {
                                g = get_node(GLUE_SPEC_SIZE as int32_t);
                                (*mem.offset(g as isize)).b16.s1 =
                                    (FILLL + 1 as ::core::ffi::c_int) as uint16_t;
                                (*mem.offset(g as isize)).b16.s0 =
                                    (FILLL + 1 as ::core::ffi::c_int) as uint16_t;
                                (*mem.offset((g + 1 as int32_t) as isize)).b32.s1 =
                                    rule_wd as int32_t;
                                (*mem.offset((g + 2 as int32_t) as isize)).b32.s1 =
                                    0 as ::core::ffi::c_int as int32_t;
                                (*mem.offset((g + 3 as int32_t) as isize)).b32.s1 =
                                    0 as ::core::ffi::c_int as int32_t;
                                (*mem.offset((p + 1 as int32_t) as isize)).b32.s0 = g;
                            }
                        }
                        if (*mem.offset(p as isize)).b16.s0 as ::core::ffi::c_int >= A_LEADERS {
                            current_block = 17249482508640382006;
                            break;
                        } else {
                            current_block = 17717689054697001253;
                            break;
                        }
                    }
                    MARGIN_KERN_NODE => {
                        cur_h += (*mem.offset((p + 1 as int32_t) as isize)).b32.s1;
                        current_block = 10791422562291087306;
                        break;
                    }
                    KERN_NODE => {
                        synctex_kern(p, this_box);
                        cur_h += (*mem.offset((p + 1 as int32_t) as isize)).b32.s1;
                        current_block = 10791422562291087306;
                        break;
                    }
                    MATH_NODE => {
                        synctex_math(p, this_box);
                        if (*mem.offset(p as isize)).b16.s0 as ::core::ffi::c_int
                            & 1 as ::core::ffi::c_int
                            != 0
                        {
                            if (*mem.offset(LR_ptr as isize)).b32.s0
                                == L_CODE as int32_t
                                    * ((*mem.offset(p as isize)).b16.s0 as int32_t
                                        / L_CODE as int32_t)
                                    + END_M_CODE as int32_t
                            {
                                temp_ptr = LR_ptr;
                                LR_ptr = (*mem.offset(temp_ptr as isize)).b32.s1;
                                (*mem.offset(temp_ptr as isize)).b32.s1 = avail;
                                avail = temp_ptr;
                            } else if (*mem.offset(p as isize)).b16.s0 as ::core::ffi::c_int
                                > L_CODE
                            {
                                LR_problems += 1;
                            }
                            current_block = 14785121481331406365;
                            break;
                        } else {
                            temp_ptr = get_avail();
                            (*mem.offset(temp_ptr as isize)).b32.s0 = (L_CODE
                                * ((*mem.offset(p as isize)).b16.s0 as ::core::ffi::c_int / L_CODE)
                                + END_M_CODE)
                                as int32_t;
                            (*mem.offset(temp_ptr as isize)).b32.s1 = LR_ptr;
                            LR_ptr = temp_ptr;
                            if !((*mem.offset(p as isize)).b16.s0 as ::core::ffi::c_int / R_CODE
                                != cur_dir as ::core::ffi::c_int)
                            {
                                current_block = 14785121481331406365;
                                break;
                            }
                            save_h = cur_h;
                            temp_ptr = (*mem.offset(p as isize)).b32.s1;
                            rule_wd = (*mem.offset((p + 1 as int32_t) as isize)).b32.s1 as scaled_t;
                            free_node(p, MEDIUM_NODE_SIZE as int32_t);
                            cur_dir = (1 as ::core::ffi::c_int - cur_dir as ::core::ffi::c_int)
                                as small_number;
                            p = new_edge(cur_dir, rule_wd);
                            (*mem.offset(prev_p as isize)).b32.s1 = p;
                            cur_h = cur_h - left_edge + rule_wd;
                            (*mem.offset(p as isize)).b32.s1 = reverse(
                                this_box,
                                new_edge(
                                    (1 as ::core::ffi::c_int - cur_dir as ::core::ffi::c_int)
                                        as small_number,
                                    0 as scaled_t,
                                ),
                                &raw mut cur_g,
                                &raw mut cur_glue,
                            );
                            (*mem.offset((p + 2 as int32_t) as isize)).b32.s1 = cur_h as int32_t;
                            cur_dir = (1 as ::core::ffi::c_int - cur_dir as ::core::ffi::c_int)
                                as small_number;
                            cur_h = save_h;
                        }
                    }
                    LIGATURE_NODE => {
                        *mem.offset(LIG_TRICK as isize) = *mem.offset((p + 1 as int32_t) as isize);
                        (*mem.offset(
                            (4999999 as ::core::ffi::c_int - 12 as ::core::ffi::c_int) as isize,
                        ))
                        .b32
                        .s1 = (*mem.offset(p as isize)).b32.s1;
                        p = LIG_TRICK as int32_t;
                        xtx_ligature_present = true_0 != 0;
                    }
                    EDGE_NODE => {
                        cur_h += (*mem.offset((p + 1 as int32_t) as isize)).b32.s1;
                        left_edge =
                            cur_h + (*mem.offset((p + 2 as int32_t) as isize)).b32.s1 as scaled_t;
                        cur_dir = (*mem.offset(p as isize)).b16.s0 as small_number;
                        current_block = 10791422562291087306;
                        break;
                    }
                    _ => {
                        current_block = 10791422562291087306;
                        break;
                    }
                }
            }
        }
        match current_block {
            17249482508640382006 => {
                leader_box = (*mem.offset((p + 1 as int32_t) as isize)).b32.s1;
                if (*mem.offset(leader_box as isize)).b16.s1 as ::core::ffi::c_int == RULE_NODE {
                    rule_ht =
                        (*mem.offset((leader_box + 3 as int32_t) as isize)).b32.s1 as scaled_t;
                    rule_dp =
                        (*mem.offset((leader_box + 2 as int32_t) as isize)).b32.s1 as scaled_t;
                    current_block = 1347935348126480785;
                } else {
                    leader_wd =
                        (*mem.offset((leader_box + 1 as int32_t) as isize)).b32.s1 as scaled_t;
                    if leader_wd > 0 as scaled_t && rule_wd > 0 as scaled_t {
                        rule_wd =
                            (rule_wd as ::core::ffi::c_int + 10 as ::core::ffi::c_int) as scaled_t;
                        if cur_dir as ::core::ffi::c_int == RIGHT_TO_LEFT {
                            cur_h = (cur_h as ::core::ffi::c_int - 10 as ::core::ffi::c_int)
                                as scaled_t;
                        }
                        edge = cur_h + rule_wd;
                        lx = 0 as ::core::ffi::c_int as scaled_t;
                        if (*mem.offset(p as isize)).b16.s0 as ::core::ffi::c_int == A_LEADERS {
                            save_h = cur_h;
                            cur_h = left_edge + leader_wd * ((cur_h - left_edge) / leader_wd);
                            if cur_h < save_h {
                                cur_h = cur_h + leader_wd;
                            }
                        } else {
                            lq = (rule_wd / leader_wd) as int32_t;
                            lr = (rule_wd % leader_wd) as int32_t;
                            if (*mem.offset(p as isize)).b16.s0 as ::core::ffi::c_int == C_LEADERS {
                                cur_h = cur_h + lr as scaled_t / 2 as scaled_t;
                            } else {
                                lx = (lr / (lq + 1 as int32_t)) as scaled_t;
                                cur_h = cur_h
                                    + (lr as scaled_t - (lq as scaled_t - 1 as scaled_t) * lx)
                                        / 2 as scaled_t;
                            }
                        }
                        while cur_h + leader_wd <= edge {
                            cur_v = base_line
                                + (*mem.offset((leader_box + 4 as int32_t) as isize)).b32.s1
                                    as scaled_t;
                            if cur_v != dvi_v {
                                movement(cur_v - dvi_v, DOWN1 as eight_bits);
                                dvi_v = cur_v;
                            }
                            save_v = dvi_v;
                            if cur_h != dvi_h {
                                movement(cur_h - dvi_h, RIGHT1 as eight_bits);
                                dvi_h = cur_h;
                            }
                            save_h = dvi_h;
                            temp_ptr = leader_box;
                            if cur_dir as ::core::ffi::c_int == RIGHT_TO_LEFT {
                                cur_h += leader_wd;
                            }
                            outer_doing_leaders = doing_leaders;
                            doing_leaders = true_0 != 0;
                            if (*mem.offset(leader_box as isize)).b16.s1 as ::core::ffi::c_int
                                == VLIST_NODE
                            {
                                vlist_out();
                            } else {
                                hlist_out();
                            }
                            doing_leaders = outer_doing_leaders;
                            dvi_v = save_v;
                            dvi_h = save_h;
                            cur_v = base_line;
                            cur_h = save_h + leader_wd + lx;
                        }
                        if cur_dir as ::core::ffi::c_int == RIGHT_TO_LEFT {
                            cur_h = edge;
                        } else {
                            cur_h = edge - 10 as scaled_t;
                        }
                        current_block = 10791422562291087306;
                    } else {
                        current_block = 17717689054697001253;
                    }
                }
            }
            14785121481331406365 => {
                (*mem.offset(p as isize)).b16.s1 = KERN_NODE as uint16_t;
                cur_h += (*mem.offset((p + 1 as int32_t) as isize)).b32.s1;
                current_block = 10791422562291087306;
            }
            _ => {}
        }
        match current_block {
            1347935348126480785 => {
                if rule_ht == NULL_FLAG as scaled_t {
                    rule_ht = (*mem.offset((this_box + 3 as int32_t) as isize)).b32.s1 as scaled_t;
                }
                if rule_dp == NULL_FLAG as scaled_t {
                    rule_dp = (*mem.offset((this_box + 2 as int32_t) as isize)).b32.s1 as scaled_t;
                }
                rule_ht += rule_dp;
                if rule_ht > 0 as scaled_t && rule_wd > 0 as scaled_t {
                    if cur_h != dvi_h {
                        movement(cur_h - dvi_h, RIGHT1 as eight_bits);
                        dvi_h = cur_h;
                    }
                    cur_v = base_line + rule_dp;
                    if cur_v != dvi_v {
                        movement(cur_v - dvi_v, DOWN1 as eight_bits);
                        dvi_v = cur_v;
                    }
                    dvi_out(SET_RULE as eight_bits);
                    dvi_four(rule_ht as int32_t);
                    dvi_four(rule_wd as int32_t);
                    cur_v = base_line;
                    dvi_h += rule_wd;
                }
                current_block = 17717689054697001253;
            }
            _ => {}
        }
        match current_block {
            17717689054697001253 => {
                cur_h += rule_wd;
                synctex_horizontal_rule_or_glue(p, this_box);
            }
            _ => {}
        }
        prev_p = p;
        p = (*mem.offset(p as isize)).b32.s1;
    }
    synctex_tsilh(this_box);
    while (*mem.offset(LR_ptr as isize)).b32.s0 != BEFORE as int32_t {
        if (*mem.offset(LR_ptr as isize)).b32.s0 > L_CODE as int32_t {
            LR_problems =
                (LR_problems as ::core::ffi::c_int + 10000 as ::core::ffi::c_int) as int32_t;
        }
        temp_ptr = LR_ptr;
        LR_ptr = (*mem.offset(temp_ptr as isize)).b32.s1;
        (*mem.offset(temp_ptr as isize)).b32.s1 = avail;
        avail = temp_ptr;
    }
    temp_ptr = LR_ptr;
    LR_ptr = (*mem.offset(temp_ptr as isize)).b32.s1;
    (*mem.offset(temp_ptr as isize)).b32.s1 = avail;
    avail = temp_ptr;
    if (*mem.offset(this_box as isize)).b16.s0 as ::core::ffi::c_int == DLIST {
        cur_dir = RIGHT_TO_LEFT as small_number;
    }
    prune_movements(save_loc);
    pitex_box_special(cur_s,"h",this_box,false);
    if cur_s > 0 as int32_t {
        dvi_pop(save_loc);
    }
    cur_s -= 1;
}
unsafe extern "C" fn vlist_out() {
    let mut current_block: u64;
    let mut left_edge: scaled_t = 0;
    let mut top_edge: scaled_t = 0;
    let mut save_h: scaled_t = 0;
    let mut save_v: scaled_t = 0;
    let mut this_box: int32_t = 0;
    let mut g_order: glue_ord = 0;
    let mut g_sign: ::core::ffi::c_uchar = 0;
    let mut p: int32_t = 0;
    let mut save_loc: int32_t = 0;
    let mut leader_box: int32_t = 0;
    let mut leader_ht: scaled_t = 0;
    let mut lx: scaled_t = 0;
    let mut outer_doing_leaders: bool = false;
    let mut edge: scaled_t = 0;
    let mut glue_temp: ::core::ffi::c_double = 0.;
    let mut cur_glue: ::core::ffi::c_double = 0.;
    let mut cur_g: scaled_t = 0;
    let mut upwards: bool = false;
    let mut f: internal_font_number = 0;
    cur_g = 0 as ::core::ffi::c_int as scaled_t;
    cur_glue = 0.0f64;
    this_box = temp_ptr;
    g_order = (*mem.offset((this_box + 5 as int32_t) as isize)).b16.s0 as glue_ord;
    g_sign = (*mem.offset((this_box + 5 as int32_t) as isize)).b16.s1 as ::core::ffi::c_uchar;
    p = (*mem.offset((this_box + 5 as int32_t) as isize)).b32.s1;
    upwards =
        (*mem.offset(this_box as isize)).b16.s0 as ::core::ffi::c_int == 1 as ::core::ffi::c_int;
    cur_s += 1;
    if cur_s > 0 as int32_t {
        dvi_out(PUSH as eight_bits);
    }
    save_loc = dvi_offset + dvi_ptr;
    pitex_box_special(cur_s,"v",this_box,true);
    if cur_s > max_push {
        max_push = cur_s;
    }
    left_edge = cur_h;
    synctex_vlist(this_box);
    if upwards {
        cur_v += (*mem.offset((this_box + 2 as int32_t) as isize)).b32.s1;
    } else {
        cur_v -= (*mem.offset((this_box + 3 as int32_t) as isize)).b32.s1;
    }
    top_edge = cur_v;
    while p != TEX_NULL as int32_t {
        if is_char_node(p) {
            confusion(b"vlistout\0" as *const u8 as *const ::core::ffi::c_char);
        } else {
            match (*mem.offset(p as isize)).b16.s1 as ::core::ffi::c_int {
                HLIST_NODE | VLIST_NODE => {
                    if (*mem.offset((p + 5 as int32_t) as isize)).b32.s1 == TEX_NULL as int32_t {
                        if upwards {
                            cur_v -= (*mem.offset((p + 2 as int32_t) as isize)).b32.s1;
                        } else {
                            cur_v += (*mem.offset((p + 3 as int32_t) as isize)).b32.s1;
                        }
                        if (*mem.offset(p as isize)).b16.s1 as ::core::ffi::c_int == VLIST_NODE {
                            synctex_void_vlist(p, this_box);
                        } else {
                            synctex_void_hlist(p, this_box);
                        }
                        if upwards {
                            cur_v -= (*mem.offset((p + 3 as int32_t) as isize)).b32.s1;
                        } else {
                            cur_v += (*mem.offset((p + 2 as int32_t) as isize)).b32.s1;
                        }
                    } else {
                        if upwards {
                            cur_v -= (*mem.offset((p + 2 as int32_t) as isize)).b32.s1;
                        } else {
                            cur_v += (*mem.offset((p + 3 as int32_t) as isize)).b32.s1;
                        }
                        if cur_v != dvi_v {
                            movement(cur_v - dvi_v, DOWN1 as eight_bits);
                            dvi_v = cur_v;
                        }
                        save_h = dvi_h;
                        save_v = dvi_v;
                        if cur_dir as ::core::ffi::c_int == RIGHT_TO_LEFT {
                            cur_h = left_edge
                                - (*mem.offset((p + 4 as int32_t) as isize)).b32.s1 as scaled_t;
                        } else {
                            cur_h = left_edge
                                + (*mem.offset((p + 4 as int32_t) as isize)).b32.s1 as scaled_t;
                        }
                        temp_ptr = p;
                        if (*mem.offset(p as isize)).b16.s1 as ::core::ffi::c_int == VLIST_NODE {
                            vlist_out();
                        } else {
                            hlist_out();
                        }
                        dvi_h = save_h;
                        dvi_v = save_v;
                        if upwards {
                            cur_v = save_v
                                - (*mem.offset((p + 3 as int32_t) as isize)).b32.s1 as scaled_t;
                        } else {
                            cur_v = save_v
                                + (*mem.offset((p + 2 as int32_t) as isize)).b32.s1 as scaled_t;
                        }
                        cur_h = left_edge;
                    }
                    current_block = 10242758336123158219;
                }
                RULE_NODE => {
                    rule_ht = (*mem.offset((p + 3 as int32_t) as isize)).b32.s1 as scaled_t;
                    rule_dp = (*mem.offset((p + 2 as int32_t) as isize)).b32.s1 as scaled_t;
                    rule_wd = (*mem.offset((p + 1 as int32_t) as isize)).b32.s1 as scaled_t;
                    current_block = 12116826043691017702;
                }
                WHATSIT_NODE => {
                    match (*mem.offset(p as isize)).b16.s0 as ::core::ffi::c_int {
                        GLYPH_NODE => {
                            cur_v = cur_v
                                + (*mem.offset((p + 3 as int32_t) as isize)).b32.s1 as scaled_t;
                            cur_h = left_edge;
                            if cur_h != dvi_h {
                                movement(cur_h - dvi_h, RIGHT1 as eight_bits);
                                dvi_h = cur_h;
                            }
                            if cur_v != dvi_v {
                                movement(cur_v - dvi_v, DOWN1 as eight_bits);
                                dvi_v = cur_v;
                            }
                            f = (*mem.offset((p + 4 as int32_t) as isize)).b16.s2
                                as internal_font_number;
                            if f != dvi_f {
                                if !*font_used.offset(f as isize) {
                                    dvi_font_def(f);
                                    *font_used.offset(f as isize) = true_0 != 0;
                                }
                                if f <= 64 as internal_font_number {
                                    dvi_out((f + 170 as internal_font_number) as eight_bits);
                                } else if f <= 256 as internal_font_number {
                                    dvi_out(FNT1 as eight_bits);
                                    dvi_out((f - 1 as internal_font_number) as eight_bits);
                                } else {
                                    dvi_out((FNT1 + 1 as ::core::ffi::c_int) as eight_bits);
                                    dvi_out(
                                        ((f - 1 as internal_font_number)
                                            / 256 as internal_font_number)
                                            as eight_bits,
                                    );
                                    dvi_out(
                                        ((f - 1 as internal_font_number)
                                            % 256 as internal_font_number)
                                            as eight_bits,
                                    );
                                }
                                dvi_f = f;
                            }
                            dvi_out(SET_GLYPHS as eight_bits);
                            dvi_four(0 as int32_t);
                            dvi_two(1 as UTF16_code);
                            dvi_four(0 as int32_t);
                            dvi_four(0 as int32_t);
                            dvi_two(
                                (*mem.offset((p + 4 as int32_t) as isize)).b16.s1 as UTF16_code,
                            );
                            cur_v += (*mem.offset((p + 2 as int32_t) as isize)).b32.s1;
                            cur_h = left_edge;
                        }
                        PIC_NODE | PDF_NODE => {
                            save_h = dvi_h;
                            save_v = dvi_v;
                            cur_v = cur_v
                                + (*mem.offset((p + 3 as int32_t) as isize)).b32.s1 as scaled_t;
                            pic_out(p);
                            dvi_h = save_h;
                            dvi_v = save_v;
                            cur_v = save_v
                                + (*mem.offset((p + 2 as int32_t) as isize)).b32.s1 as scaled_t;
                            cur_h = left_edge;
                        }
                        PDF_SAVE_POS_NODE => {
                            pdf_last_x_pos = (cur_h as ::core::ffi::c_long
                                + 4736286 as ::core::ffi::c_long)
                                as int32_t;
                            pdf_last_y_pos = ((cur_page_height - cur_v) as ::core::ffi::c_long
                                - 4736286 as ::core::ffi::c_long)
                                as int32_t;
                        }
                        _ => {
                            out_what(p);
                        }
                    }
                    current_block = 10242758336123158219;
                }
                GLUE_NODE => {
                    g = (*mem.offset((p + 1 as int32_t) as isize)).b32.s0;
                    rule_ht = ((*mem.offset((g + 1 as int32_t) as isize)).b32.s1 - cur_g as int32_t)
                        as scaled_t;
                    if g_sign as ::core::ffi::c_int != NORMAL {
                        if g_sign as ::core::ffi::c_int == STRETCHING {
                            if (*mem.offset(g as isize)).b16.s1 as ::core::ffi::c_int
                                == g_order as ::core::ffi::c_int
                            {
                                cur_glue += (*mem.offset((g + 2 as int32_t) as isize)).b32.s1
                                    as ::core::ffi::c_double;
                                glue_temp =
                                    (*mem.offset((this_box + 6 as int32_t) as isize)).gr * cur_glue;
                                if glue_temp > 1000000000.0f64 {
                                    glue_temp = 1000000000.0f64;
                                } else if glue_temp < -1000000000.0f64 {
                                    glue_temp = -1000000000.0f64;
                                }
                                cur_g = tex_round(glue_temp) as scaled_t;
                            }
                        } else if (*mem.offset(g as isize)).b16.s0 as ::core::ffi::c_int
                            == g_order as ::core::ffi::c_int
                        {
                            cur_glue -= (*mem.offset((g + 3 as int32_t) as isize)).b32.s1
                                as ::core::ffi::c_double;
                            glue_temp =
                                (*mem.offset((this_box + 6 as int32_t) as isize)).gr * cur_glue;
                            if glue_temp > 1000000000.0f64 {
                                glue_temp = 1000000000.0f64;
                            } else if glue_temp < -1000000000.0f64 {
                                glue_temp = -1000000000.0f64;
                            }
                            cur_g = tex_round(glue_temp) as scaled_t;
                        }
                    }
                    rule_ht += cur_g;
                    if (*mem.offset(p as isize)).b16.s0 as ::core::ffi::c_int >= A_LEADERS {
                        leader_box = (*mem.offset((p + 1 as int32_t) as isize)).b32.s1;
                        if (*mem.offset(leader_box as isize)).b16.s1 as ::core::ffi::c_int
                            == RULE_NODE
                        {
                            rule_wd = (*mem.offset((leader_box + 1 as int32_t) as isize)).b32.s1
                                as scaled_t;
                            rule_dp = 0 as ::core::ffi::c_int as scaled_t;
                            current_block = 12116826043691017702;
                        } else {
                            leader_ht = ((*mem.offset((leader_box + 3 as int32_t) as isize)).b32.s1
                                + (*mem.offset((leader_box + 2 as int32_t) as isize)).b32.s1)
                                as scaled_t;
                            if leader_ht > 0 as scaled_t && rule_ht > 0 as scaled_t {
                                rule_ht = (rule_ht as ::core::ffi::c_int + 10 as ::core::ffi::c_int)
                                    as scaled_t;
                                edge = cur_v + rule_ht;
                                lx = 0 as ::core::ffi::c_int as scaled_t;
                                if (*mem.offset(p as isize)).b16.s0 as ::core::ffi::c_int
                                    == A_LEADERS
                                {
                                    save_v = cur_v;
                                    cur_v = top_edge + leader_ht * ((cur_v - top_edge) / leader_ht);
                                    if cur_v < save_v {
                                        cur_v = cur_v + leader_ht;
                                    }
                                } else {
                                    lq = (rule_ht / leader_ht) as int32_t;
                                    lr = (rule_ht % leader_ht) as int32_t;
                                    if (*mem.offset(p as isize)).b16.s0 as ::core::ffi::c_int
                                        == C_LEADERS
                                    {
                                        cur_v = cur_v + lr as scaled_t / 2 as scaled_t;
                                    } else {
                                        lx = (lr / (lq + 1 as int32_t)) as scaled_t;
                                        cur_v = cur_v
                                            + (lr as scaled_t
                                                - (lq as scaled_t - 1 as scaled_t) * lx)
                                                / 2 as scaled_t;
                                    }
                                }
                                while cur_v + leader_ht <= edge {
                                    if cur_dir as ::core::ffi::c_int == RIGHT_TO_LEFT {
                                        cur_h = left_edge
                                            - (*mem.offset((leader_box + 4 as int32_t) as isize))
                                                .b32
                                                .s1
                                                as scaled_t;
                                    } else {
                                        cur_h = left_edge
                                            + (*mem.offset((leader_box + 4 as int32_t) as isize))
                                                .b32
                                                .s1
                                                as scaled_t;
                                    }
                                    if cur_h != dvi_h {
                                        movement(cur_h - dvi_h, RIGHT1 as eight_bits);
                                        dvi_h = cur_h;
                                    }
                                    save_h = dvi_h;
                                    cur_v +=
                                        (*mem.offset((leader_box + 3 as int32_t) as isize)).b32.s1;
                                    if cur_v != dvi_v {
                                        movement(cur_v - dvi_v, DOWN1 as eight_bits);
                                        dvi_v = cur_v;
                                    }
                                    save_v = dvi_v;
                                    temp_ptr = leader_box;
                                    outer_doing_leaders = doing_leaders;
                                    doing_leaders = true_0 != 0;
                                    if (*mem.offset(leader_box as isize)).b16.s1
                                        as ::core::ffi::c_int
                                        == VLIST_NODE
                                    {
                                        vlist_out();
                                    } else {
                                        hlist_out();
                                    }
                                    doing_leaders = outer_doing_leaders;
                                    dvi_v = save_v;
                                    dvi_h = save_h;
                                    cur_h = left_edge;
                                    cur_v = save_v
                                        - (*mem.offset((leader_box + 3 as int32_t) as isize)).b32.s1
                                            as scaled_t
                                        + leader_ht
                                        + lx;
                                }
                                cur_v = edge - 10 as scaled_t;
                                current_block = 10242758336123158219;
                            } else {
                                current_block = 16426516343905374363;
                            }
                        }
                    } else {
                        current_block = 16426516343905374363;
                    }
                    match current_block {
                        10242758336123158219 => {}
                        12116826043691017702 => {}
                        _ => {
                            if upwards {
                                cur_v -= rule_ht;
                            } else {
                                cur_v += rule_ht;
                            }
                            current_block = 10242758336123158219;
                        }
                    }
                }
                KERN_NODE => {
                    if upwards {
                        cur_v -= (*mem.offset((p + 1 as int32_t) as isize)).b32.s1;
                    } else {
                        cur_v += (*mem.offset((p + 1 as int32_t) as isize)).b32.s1;
                    }
                    current_block = 10242758336123158219;
                }
                _ => {
                    current_block = 10242758336123158219;
                }
            }
            match current_block {
                12116826043691017702 => {
                    if rule_wd == NULL_FLAG as scaled_t {
                        rule_wd =
                            (*mem.offset((this_box + 1 as int32_t) as isize)).b32.s1 as scaled_t;
                    }
                    rule_ht += rule_dp;
                    if upwards {
                        cur_v -= rule_ht;
                    } else {
                        cur_v += rule_ht;
                    }
                    if rule_ht > 0 as scaled_t && rule_wd > 0 as scaled_t {
                        if cur_dir as ::core::ffi::c_int == RIGHT_TO_LEFT {
                            cur_h -= rule_wd;
                        }
                        if cur_h != dvi_h {
                            movement(cur_h - dvi_h, RIGHT1 as eight_bits);
                            dvi_h = cur_h;
                        }
                        if cur_v != dvi_v {
                            movement(cur_v - dvi_v, DOWN1 as eight_bits);
                            dvi_v = cur_v;
                        }
                        dvi_out(PUT_RULE as eight_bits);
                        dvi_four(rule_ht as int32_t);
                        dvi_four(rule_wd as int32_t);
                        cur_h = left_edge;
                    }
                }
                _ => {}
            }
            p = (*mem.offset(p as isize)).b32.s1;
        }
    }
    synctex_tsilv(this_box);
    prune_movements(save_loc);
    pitex_box_special(cur_s,"v",this_box,false);
    if cur_s > 0 as int32_t {
        dvi_pop(save_loc);
    }
    cur_s -= 1;
}
unsafe extern "C" fn reverse(
    mut this_box: int32_t,
    mut t: int32_t,
    mut cur_g: *mut scaled_t,
    mut cur_glue: *mut ::core::ffi::c_double,
) -> int32_t {
    let mut current_block: u64;
    let mut l: int32_t = 0;
    let mut p: int32_t = 0;
    let mut q: int32_t = 0;
    let mut g_order: glue_ord = 0;
    let mut g_sign: ::core::ffi::c_uchar = 0;
    let mut glue_temp: ::core::ffi::c_double = 0.;
    let mut m: int32_t = 0;
    let mut n: int32_t = 0;
    let mut c: uint16_t = 0;
    let mut f: internal_font_number = 0;
    g_order = (*mem.offset((this_box + 5 as int32_t) as isize)).b16.s0 as glue_ord;
    g_sign = (*mem.offset((this_box + 5 as int32_t) as isize)).b16.s1 as ::core::ffi::c_uchar;
    l = t;
    p = temp_ptr;
    m = MIN_HALFWORD as int32_t;
    n = MIN_HALFWORD as int32_t;
    's_41: loop {
        if p != TEX_NULL as int32_t {
            loop {
                if is_char_node(p) {
                    loop {
                        f = (*mem.offset(p as isize)).b16.s1 as internal_font_number;
                        c = (*mem.offset(p as isize)).b16.s0;
                        cur_h += (*font_info.offset(
                            (*width_base.offset(f as isize)
                                + (*font_info.offset(
                                    (*char_base.offset(f as isize)
                                        + effective_char(1 as ::core::ffi::c_int != 0, f, c))
                                        as isize,
                                ))
                                .b16
                                .s3 as int32_t) as isize,
                        ))
                        .b32
                        .s1;
                        q = (*mem.offset(p as isize)).b32.s1;
                        (*mem.offset(p as isize)).b32.s1 = l;
                        l = p;
                        p = q;
                        if !is_char_node(p) {
                            break;
                        }
                    }
                    continue 's_41;
                } else {
                    q = (*mem.offset(p as isize)).b32.s1;
                    match (*mem.offset(p as isize)).b16.s1 as ::core::ffi::c_int {
                        HLIST_NODE | VLIST_NODE | RULE_NODE | KERN_NODE => {
                            rule_wd = (*mem.offset((p + 1 as int32_t) as isize)).b32.s1 as scaled_t;
                            current_block = 2750570471926810434;
                            break;
                        }
                        WHATSIT_NODE => {
                            if (*mem.offset(p as isize)).b16.s0 as ::core::ffi::c_int
                                == NATIVE_WORD_NODE
                                || (*mem.offset(p as isize)).b16.s0 as ::core::ffi::c_int
                                    == NATIVE_WORD_NODE_AT
                                || (*mem.offset(p as isize)).b16.s0 as ::core::ffi::c_int
                                    == GLYPH_NODE
                                || (*mem.offset(p as isize)).b16.s0 as ::core::ffi::c_int
                                    == PIC_NODE
                                || (*mem.offset(p as isize)).b16.s0 as ::core::ffi::c_int
                                    == PDF_NODE
                            {
                                current_block = 11584701595673473500;
                                break;
                            } else {
                                current_block = 14525168427465536419;
                                break;
                            }
                        }
                        GLUE_NODE => {
                            g = (*mem.offset((p + 1 as int32_t) as isize)).b32.s0;
                            rule_wd = ((*mem.offset((g + 1 as int32_t) as isize)).b32.s1 - *cur_g)
                                as scaled_t;
                            if g_sign as ::core::ffi::c_int != NORMAL {
                                if g_sign as ::core::ffi::c_int == STRETCHING {
                                    if (*mem.offset(g as isize)).b16.s1 as ::core::ffi::c_int
                                        == g_order as ::core::ffi::c_int
                                    {
                                        *cur_glue = *cur_glue
                                            + (*mem.offset((g + 2 as int32_t) as isize)).b32.s1
                                                as ::core::ffi::c_double;
                                        glue_temp = (*mem
                                            .offset((this_box + 6 as int32_t) as isize))
                                        .gr * *cur_glue;
                                        if glue_temp > 1000000000.0f64 {
                                            glue_temp = 1000000000.0f64;
                                        } else if glue_temp < -1000000000.0f64 {
                                            glue_temp = -1000000000.0f64;
                                        }
                                        *cur_g = tex_round(glue_temp) as scaled_t;
                                    }
                                } else if (*mem.offset(g as isize)).b16.s0 as ::core::ffi::c_int
                                    == g_order as ::core::ffi::c_int
                                {
                                    *cur_glue = *cur_glue
                                        - (*mem.offset((g + 3 as int32_t) as isize)).b32.s1
                                            as ::core::ffi::c_double;
                                    glue_temp = (*mem.offset((this_box + 6 as int32_t) as isize))
                                        .gr
                                        * *cur_glue;
                                    if glue_temp > 1000000000.0f64 {
                                        glue_temp = 1000000000.0f64;
                                    } else if glue_temp < -1000000000.0f64 {
                                        glue_temp = -1000000000.0f64;
                                    }
                                    *cur_g = tex_round(glue_temp) as scaled_t;
                                }
                            }
                            rule_wd += *cur_g;
                            if g_sign as ::core::ffi::c_int == STRETCHING
                                && (*mem.offset(g as isize)).b16.s1 as ::core::ffi::c_int
                                    == g_order as ::core::ffi::c_int
                                || g_sign as ::core::ffi::c_int == SHRINKING
                                    && (*mem.offset(g as isize)).b16.s0 as ::core::ffi::c_int
                                        == g_order as ::core::ffi::c_int
                            {
                                if (*mem.offset(g as isize)).b32.s1 == TEX_NULL as int32_t {
                                    free_node(g, GLUE_SPEC_SIZE as int32_t);
                                } else {
                                    let ref mut fresh3 = (*mem.offset(g as isize)).b32.s1;
                                    *fresh3 -= 1;
                                }
                                if ((*mem.offset(p as isize)).b16.s0 as ::core::ffi::c_int)
                                    < A_LEADERS
                                {
                                    (*mem.offset(p as isize)).b16.s1 = KERN_NODE as uint16_t;
                                    (*mem.offset((p + 1 as int32_t) as isize)).b32.s1 =
                                        rule_wd as int32_t;
                                } else {
                                    g = get_node(GLUE_SPEC_SIZE as int32_t);
                                    (*mem.offset(g as isize)).b16.s1 =
                                        (FILLL + 1 as ::core::ffi::c_int) as uint16_t;
                                    (*mem.offset(g as isize)).b16.s0 =
                                        (FILLL + 1 as ::core::ffi::c_int) as uint16_t;
                                    (*mem.offset((g + 1 as int32_t) as isize)).b32.s1 =
                                        rule_wd as int32_t;
                                    (*mem.offset((g + 2 as int32_t) as isize)).b32.s1 =
                                        0 as ::core::ffi::c_int as int32_t;
                                    (*mem.offset((g + 3 as int32_t) as isize)).b32.s1 =
                                        0 as ::core::ffi::c_int as int32_t;
                                    (*mem.offset((p + 1 as int32_t) as isize)).b32.s0 = g;
                                }
                            }
                            current_block = 2750570471926810434;
                            break;
                        }
                        LIGATURE_NODE => {
                            flush_node_list((*mem.offset((p + 1 as int32_t) as isize)).b32.s1);
                            temp_ptr = p;
                            p = get_avail();
                            *mem.offset(p as isize) =
                                *mem.offset((temp_ptr + 1 as int32_t) as isize);
                            (*mem.offset(p as isize)).b32.s1 = q;
                            free_node(temp_ptr, SMALL_NODE_SIZE as int32_t);
                        }
                        MATH_NODE => {
                            rule_wd = (*mem.offset((p + 1 as int32_t) as isize)).b32.s1 as scaled_t;
                            if (*mem.offset(p as isize)).b16.s0 as ::core::ffi::c_int
                                & 1 as ::core::ffi::c_int
                                != 0
                            {
                                current_block = 9925100494328262799;
                                break;
                            } else {
                                current_block = 2706659501864706830;
                                break;
                            }
                        }
                        EDGE_NODE => {
                            confusion(b"LR2\0" as *const u8 as *const ::core::ffi::c_char);
                        }
                        _ => {
                            current_block = 14525168427465536419;
                            break;
                        }
                    }
                }
            }
            match current_block {
                9925100494328262799 => {
                    if (*mem.offset(LR_ptr as isize)).b32.s0
                        != L_CODE as int32_t
                            * ((*mem.offset(p as isize)).b16.s0 as int32_t / L_CODE as int32_t)
                            + END_M_CODE as int32_t
                    {
                        (*mem.offset(p as isize)).b16.s1 = KERN_NODE as uint16_t;
                        LR_problems += 1;
                    } else {
                        temp_ptr = LR_ptr;
                        LR_ptr = (*mem.offset(temp_ptr as isize)).b32.s1;
                        (*mem.offset(temp_ptr as isize)).b32.s1 = avail;
                        avail = temp_ptr;
                        if n > MIN_HALFWORD as int32_t {
                            n -= 1;
                            let ref mut fresh4 = (*mem.offset(p as isize)).b16.s0;
                            *fresh4 = (*fresh4).wrapping_sub(1);
                        } else {
                            (*mem.offset(p as isize)).b16.s1 = KERN_NODE as uint16_t;
                            if m > MIN_HALFWORD as int32_t {
                                m -= 1;
                            } else {
                                free_node(p, MEDIUM_NODE_SIZE as int32_t);
                                (*mem.offset(t as isize)).b32.s1 = q;
                                (*mem.offset((t + 1 as int32_t) as isize)).b32.s1 =
                                    rule_wd as int32_t;
                                (*mem.offset((t + 2 as int32_t) as isize)).b32.s1 =
                                    (-cur_h - rule_wd) as int32_t;
                                break;
                            }
                        }
                    }
                    current_block = 2750570471926810434;
                }
                2706659501864706830 => {
                    temp_ptr = get_avail();
                    (*mem.offset(temp_ptr as isize)).b32.s0 =
                        (L_CODE * ((*mem.offset(p as isize)).b16.s0 as ::core::ffi::c_int / L_CODE)
                            + END_M_CODE) as int32_t;
                    (*mem.offset(temp_ptr as isize)).b32.s1 = LR_ptr;
                    LR_ptr = temp_ptr;
                    if n > MIN_HALFWORD as int32_t
                        || (*mem.offset(p as isize)).b16.s0 as ::core::ffi::c_int / R_CODE
                            != cur_dir as ::core::ffi::c_int
                    {
                        n += 1;
                        let ref mut fresh5 = (*mem.offset(p as isize)).b16.s0;
                        *fresh5 = (*fresh5).wrapping_add(1);
                    } else {
                        (*mem.offset(p as isize)).b16.s1 = KERN_NODE as uint16_t;
                        m += 1;
                    }
                    current_block = 2750570471926810434;
                }
                11584701595673473500 => {
                    rule_wd = (*mem.offset((p + 1 as int32_t) as isize)).b32.s1 as scaled_t;
                    current_block = 2750570471926810434;
                }
                _ => {}
            }
            match current_block {
                2750570471926810434 => {
                    cur_h += rule_wd;
                }
                _ => {}
            }
            (*mem.offset(p as isize)).b32.s1 = l;
            if (*mem.offset(p as isize)).b16.s1 as ::core::ffi::c_int == KERN_NODE {
                if rule_wd == 0 as scaled_t || l == TEX_NULL as int32_t {
                    free_node(p, MEDIUM_NODE_SIZE as int32_t);
                    p = l;
                }
            }
            l = p;
            p = q;
        } else {
            if t == TEX_NULL as int32_t
                && m == MIN_HALFWORD as int32_t
                && n == MIN_HALFWORD as int32_t
            {
                break;
            }
            p = new_math(
                0 as scaled_t,
                (*mem.offset(LR_ptr as isize)).b32.s0 as small_number,
            );
            LR_problems =
                (LR_problems as ::core::ffi::c_int + 10000 as ::core::ffi::c_int) as int32_t;
        }
    }
    return l;
}
#[no_mangle]
pub unsafe extern "C" fn new_edge(mut s: small_number, mut w: scaled_t) -> int32_t {
    let mut p: int32_t = 0;
    p = get_node(EDGE_NODE_SIZE as int32_t);
    (*mem.offset(p as isize)).b16.s1 = EDGE_NODE as uint16_t;
    (*mem.offset(p as isize)).b16.s0 = s as uint16_t;
    (*mem.offset((p + 1 as int32_t) as isize)).b32.s1 = w as int32_t;
    (*mem.offset((p + 2 as int32_t) as isize)).b32.s1 = 0 as ::core::ffi::c_int as int32_t;
    return p;
}
#[no_mangle]
pub unsafe extern "C" fn out_what(mut p: int32_t) {
    let mut j: small_number = 0;
    let mut old_setting: ::core::ffi::c_uchar = 0;
    match (*mem.offset(p as isize)).b16.s0 as ::core::ffi::c_int {
        OPEN_NODE | WRITE_NODE | CLOSE_NODE => {
            if !doing_leaders {
                j = (*mem.offset((p + 1 as int32_t) as isize)).b32.s0 as small_number;
                if (*mem.offset(p as isize)).b16.s0 as ::core::ffi::c_int == WRITE_NODE {
                    write_out(p);
                } else {
                    if write_open[j as usize] {
                        ttstub_output_close(write_file[j as usize]);
                    }
                    if (*mem.offset(p as isize)).b16.s0 as ::core::ffi::c_int == CLOSE_NODE {
                        write_open[j as usize] = false_0 != 0;
                    } else if !(j as ::core::ffi::c_int >= 16 as ::core::ffi::c_int) {
                        cur_name = (*mem.offset((p + 1 as int32_t) as isize)).b32.s1 as str_number;
                        cur_area = (*mem.offset((p + 2 as int32_t) as isize)).b32.s0 as str_number;
                        cur_ext = (*mem.offset((p + 2 as int32_t) as isize)).b32.s1 as str_number;
                        if length(cur_ext) == 0 as int32_t {
                            cur_ext =
                                maketexstring(b".tex\0" as *const u8 as *const ::core::ffi::c_char)
                                    as str_number;
                        }
                        pack_file_name(cur_name, cur_area, cur_ext);
                        write_file[j as usize] =
                            ttstub_output_open(name_of_file, 0 as ::core::ffi::c_int);
                        if write_file[j as usize].is_null() {
                            _tt_abort(
                                b"cannot open output file \"%s\"\0" as *const u8
                                    as *const ::core::ffi::c_char,
                                name_of_file,
                            );
                        }
                        write_open[j as usize] = true_0 != 0;
                        if log_opened {
                            old_setting = selector as ::core::ffi::c_uchar;
                            if (*eqtb.offset((INT_BASE + INT_PAR__tracing_online) as isize))
                                .b32
                                .s1
                                <= 0 as int32_t
                            {
                                selector = SELECTOR_LOG_ONLY;
                            } else {
                                selector = SELECTOR_TERM_AND_LOG;
                            }
                            print_nl_cstr(
                                b"\\openout\0" as *const u8 as *const ::core::ffi::c_char,
                            );
                            print_int(j as int32_t);
                            print_cstr(b" = `\0" as *const u8 as *const ::core::ffi::c_char);
                            print_file_name(
                                cur_name as int32_t,
                                cur_area as int32_t,
                                cur_ext as int32_t,
                            );
                            print_cstr(b"'.\0" as *const u8 as *const ::core::ffi::c_char);
                            print_nl_cstr(b"\0" as *const u8 as *const ::core::ffi::c_char);
                            print_ln();
                            selector = old_setting as selector_t;
                        }
                    }
                }
            }
        }
        SPECIAL_NODE => {
            special_out(p);
        }
        LANGUAGE_NODE => {}
        _ => {
            confusion(b"ext4\0" as *const u8 as *const ::core::ffi::c_char);
        }
    };
}
unsafe extern "C" fn dvi_native_font_def(mut f: internal_font_number) {
    let mut font_def_length: int32_t = 0;
    let mut i: int32_t = 0;
    dvi_out(DEFINE_NATIVE_FONT as eight_bits);
    dvi_four(f as int32_t - 1 as int32_t);
    font_def_length = make_font_def(f as int32_t) as int32_t;
    i = 0 as ::core::ffi::c_int as int32_t;
    while i < font_def_length {
        dvi_out(*xdv_buffer.offset(i as isize) as eight_bits);
        i += 1;
    }
}
unsafe extern "C" fn dvi_font_def(mut f: internal_font_number) {
    let mut k: pool_pointer = 0;
    let mut l: int32_t = 0;
    if *font_area.offset(f as isize) as ::core::ffi::c_uint == AAT_FONT_FLAG
        || *font_area.offset(f as isize) as ::core::ffi::c_uint == OTGR_FONT_FLAG
    {
        dvi_native_font_def(f);
    } else {
        if f <= 256 as internal_font_number {
            dvi_out(FNT_DEF1 as eight_bits);
            dvi_out((f - 1 as internal_font_number) as eight_bits);
        } else {
            dvi_out((FNT_DEF1 + 1 as ::core::ffi::c_int) as eight_bits);
            dvi_out(((f - 1 as internal_font_number) / 256 as internal_font_number) as eight_bits);
            dvi_out(((f - 1 as internal_font_number) % 256 as internal_font_number) as eight_bits);
        }
        dvi_out((*font_check.offset(f as isize)).s3 as eight_bits);
        dvi_out((*font_check.offset(f as isize)).s2 as eight_bits);
        dvi_out((*font_check.offset(f as isize)).s1 as eight_bits);
        dvi_out((*font_check.offset(f as isize)).s0 as eight_bits);
        dvi_four(*font_size.offset(f as isize) as int32_t);
        dvi_four(*font_dsize.offset(f as isize) as int32_t);
        dvi_out(length(*font_area.offset(f as isize)) as eight_bits);
        l = 0 as ::core::ffi::c_int as int32_t;
        k = *str_start.offset(
            (*font_name.offset(f as isize) as ::core::ffi::c_long - 65536 as ::core::ffi::c_long)
                as isize,
        );
        while l == 0 as int32_t
            && k < *str_start.offset(
                ((*font_name.offset(f as isize) + 1 as str_number) as ::core::ffi::c_long
                    - 65536 as ::core::ffi::c_long) as isize,
            )
        {
            if *str_pool.offset(k as isize) as ::core::ffi::c_int == ':' as i32 {
                l = (k - *str_start.offset(
                    (*font_name.offset(f as isize) as ::core::ffi::c_long
                        - 65536 as ::core::ffi::c_long) as isize,
                )) as int32_t;
            }
            k += 1;
        }
        if l == 0 as int32_t {
            l = length(*font_name.offset(f as isize));
        }
        dvi_out(l as eight_bits);
        let mut for_end: int32_t = 0;
        k = *str_start.offset(
            (*font_area.offset(f as isize) as ::core::ffi::c_long - 65536 as ::core::ffi::c_long)
                as isize,
        );
        for_end = (*str_start.offset(
            ((*font_area.offset(f as isize) + 1 as str_number) as ::core::ffi::c_long
                - 65536 as ::core::ffi::c_long) as isize,
        ) - 1 as pool_pointer) as int32_t;
        if k <= for_end {
            loop {
                dvi_out(*str_pool.offset(k as isize) as eight_bits);
                let fresh6 = k;
                k = k + 1;
                if !(fresh6 < for_end) {
                    break;
                }
            }
        }
        let mut for_end_0: int32_t = 0;
        k = *str_start.offset(
            (*font_name.offset(f as isize) as ::core::ffi::c_long - 65536 as ::core::ffi::c_long)
                as isize,
        );
        for_end_0 = (*str_start.offset(
            (*font_name.offset(f as isize) as ::core::ffi::c_long - 65536 as ::core::ffi::c_long)
                as isize,
        ) + l as pool_pointer
            - 1 as pool_pointer) as int32_t;
        if k <= for_end_0 {
            loop {
                dvi_out(*str_pool.offset(k as isize) as eight_bits);
                let fresh7 = k;
                k = k + 1;
                if !(fresh7 < for_end_0) {
                    break;
                }
            }
        }
    };
}
unsafe extern "C" fn movement(mut w: scaled_t, mut o: eight_bits) {
    let mut current_block: u64;
    let mut mstate: small_number = 0;
    let mut p: int32_t = 0;
    let mut q: int32_t = 0;
    let mut k: int32_t = 0;
    q = get_node(MOVEMENT_NODE_SIZE as int32_t);
    (*mem.offset((q + 1 as int32_t) as isize)).b32.s1 = w as int32_t;
    (*mem.offset((q + 2 as int32_t) as isize)).b32.s1 = dvi_offset + dvi_ptr;
    if o as ::core::ffi::c_int == DOWN1 {
        (*mem.offset(q as isize)).b32.s1 = down_ptr;
        down_ptr = q;
    } else {
        (*mem.offset(q as isize)).b32.s1 = right_ptr;
        right_ptr = q;
    }
    p = (*mem.offset(q as isize)).b32.s1;
    mstate = MOV_NONE_SEEN as small_number;
    loop {
        if !(p != TEX_NULL as int32_t) {
            current_block = 11213436766421334984;
            break;
        }
        if (*mem.offset((p + 1 as int32_t) as isize)).b32.s1 == w {
            match mstate as int32_t + (*mem.offset(p as isize)).b32.s0 {
                3 | 4 | 15 | 16 => {
                    current_block = 659964596893704337;
                    match current_block {
                        7815301962955275263 => {
                            if (*mem.offset((p + 2 as int32_t) as isize)).b32.s1 < dvi_gone {
                                current_block = 11213436766421334984;
                                break;
                            }
                            k = (*mem.offset((p + 2 as int32_t) as isize)).b32.s1 - dvi_offset;
                            if k < 0 as int32_t {
                                k = k + DVI_BUF_SIZE as int32_t;
                            }
                            *dvi_buf.offset(k as isize) = (*dvi_buf.offset(k as isize)
                                as ::core::ffi::c_int
                                + 10 as ::core::ffi::c_int)
                                as eight_bits;
                            (*mem.offset(p as isize)).b32.s0 = MOV_Z_HERE as int32_t;
                            current_block = 13972003802672699133;
                            break;
                        }
                        _ => {
                            if (*mem.offset((p + 2 as int32_t) as isize)).b32.s1 < dvi_gone {
                                current_block = 11213436766421334984;
                                break;
                            }
                            k = (*mem.offset((p + 2 as int32_t) as isize)).b32.s1 - dvi_offset;
                            if k < 0 as int32_t {
                                k = k + DVI_BUF_SIZE as int32_t;
                            }
                            *dvi_buf.offset(k as isize) = (*dvi_buf.offset(k as isize)
                                as ::core::ffi::c_int
                                + 5 as ::core::ffi::c_int)
                                as eight_bits;
                            (*mem.offset(p as isize)).b32.s0 = MOV_Y_HERE as int32_t;
                            current_block = 13972003802672699133;
                            break;
                        }
                    }
                }
                5 | 9 | 11 => {
                    current_block = 7815301962955275263;
                    match current_block {
                        7815301962955275263 => {
                            if (*mem.offset((p + 2 as int32_t) as isize)).b32.s1 < dvi_gone {
                                current_block = 11213436766421334984;
                                break;
                            }
                            k = (*mem.offset((p + 2 as int32_t) as isize)).b32.s1 - dvi_offset;
                            if k < 0 as int32_t {
                                k = k + DVI_BUF_SIZE as int32_t;
                            }
                            *dvi_buf.offset(k as isize) = (*dvi_buf.offset(k as isize)
                                as ::core::ffi::c_int
                                + 10 as ::core::ffi::c_int)
                                as eight_bits;
                            (*mem.offset(p as isize)).b32.s0 = MOV_Z_HERE as int32_t;
                            current_block = 13972003802672699133;
                            break;
                        }
                        _ => {
                            if (*mem.offset((p + 2 as int32_t) as isize)).b32.s1 < dvi_gone {
                                current_block = 11213436766421334984;
                                break;
                            }
                            k = (*mem.offset((p + 2 as int32_t) as isize)).b32.s1 - dvi_offset;
                            if k < 0 as int32_t {
                                k = k + DVI_BUF_SIZE as int32_t;
                            }
                            *dvi_buf.offset(k as isize) = (*dvi_buf.offset(k as isize)
                                as ::core::ffi::c_int
                                + 5 as ::core::ffi::c_int)
                                as eight_bits;
                            (*mem.offset(p as isize)).b32.s0 = MOV_Y_HERE as int32_t;
                            current_block = 13972003802672699133;
                            break;
                        }
                    }
                }
                1 | 2 | 8 | 13 => {
                    current_block = 13972003802672699133;
                    break;
                }
                _ => {}
            }
        } else {
            match mstate as int32_t + (*mem.offset(p as isize)).b32.s0 {
                1 => {
                    current_block = 2099638578971418440;
                    match current_block {
                        12769498904039717978 => {
                            mstate = MOV_Z_SEEN as small_number;
                        }
                        _ => {
                            mstate = MOV_Y_SEEN as small_number;
                        }
                    }
                }
                2 => {
                    current_block = 12769498904039717978;
                    match current_block {
                        12769498904039717978 => {
                            mstate = MOV_Z_SEEN as small_number;
                        }
                        _ => {
                            mstate = MOV_Y_SEEN as small_number;
                        }
                    }
                }
                8 | 13 => {
                    current_block = 11213436766421334984;
                    break;
                }
                _ => {}
            }
        }
        p = (*mem.offset(p as isize)).b32.s1;
    }
    match current_block {
        13972003802672699133 => {
            (*mem.offset(q as isize)).b32.s0 = (*mem.offset(p as isize)).b32.s0;
            if (*mem.offset(q as isize)).b32.s0 == MOV_Y_HERE as int32_t {
                dvi_out((o as ::core::ffi::c_int + 4 as ::core::ffi::c_int) as eight_bits);
                while (*mem.offset(q as isize)).b32.s1 != p {
                    q = (*mem.offset(q as isize)).b32.s1;
                    match (*mem.offset(q as isize)).b32.s0 {
                        MOV_YZ_OK => {
                            (*mem.offset(q as isize)).b32.s0 = MOV_Z_OK as int32_t;
                        }
                        MOV_Y_OK => {
                            (*mem.offset(q as isize)).b32.s0 = MOV_D_FIXED as int32_t;
                        }
                        _ => {}
                    }
                }
            } else {
                dvi_out((o as ::core::ffi::c_int + 9 as ::core::ffi::c_int) as eight_bits);
                while (*mem.offset(q as isize)).b32.s1 != p {
                    q = (*mem.offset(q as isize)).b32.s1;
                    match (*mem.offset(q as isize)).b32.s0 {
                        MOV_YZ_OK => {
                            (*mem.offset(q as isize)).b32.s0 = MOV_Y_OK as int32_t;
                        }
                        MOV_Z_OK => {
                            (*mem.offset(q as isize)).b32.s0 = MOV_D_FIXED as int32_t;
                        }
                        _ => {}
                    }
                }
            }
            return;
        }
        _ => {
            (*mem.offset(q as isize)).b32.s0 = MOV_YZ_OK as int32_t;
            if abs(w as ::core::ffi::c_int) >= 0x800000 as ::core::ffi::c_int {
                dvi_out((o as ::core::ffi::c_int + 3 as ::core::ffi::c_int) as eight_bits);
                dvi_four(w as int32_t);
                return;
            }
            if abs(w as ::core::ffi::c_int) >= 0x8000 as ::core::ffi::c_int {
                dvi_out((o as ::core::ffi::c_int + 2 as ::core::ffi::c_int) as eight_bits);
                if w < 0 as scaled_t {
                    w = w + 0x1000000 as scaled_t;
                }
                dvi_out((w / 0x10000 as scaled_t) as eight_bits);
                w = w % 0x10000 as scaled_t;
                current_block = 5243009508006772947;
            } else if abs(w as ::core::ffi::c_int) >= 128 as ::core::ffi::c_int {
                dvi_out((o as ::core::ffi::c_int + 1 as ::core::ffi::c_int) as eight_bits);
                if w < 0 as scaled_t {
                    w = w + 0x10000 as scaled_t;
                }
                current_block = 5243009508006772947;
            } else {
                dvi_out(o);
                if w < 0 as scaled_t {
                    w = w + 256 as scaled_t;
                }
                current_block = 1155322688975793421;
            }
            match current_block {
                5243009508006772947 => {
                    dvi_out((w / 256 as scaled_t) as eight_bits);
                }
                _ => {}
            }
            dvi_out((w % 256 as scaled_t) as eight_bits);
            return;
        }
    };
}
unsafe extern "C" fn prune_movements(mut l: int32_t) {
    let mut p: int32_t = 0;
    while down_ptr != TEX_NULL as int32_t {
        if (*mem.offset((down_ptr + 2 as int32_t) as isize)).b32.s1 < l {
            break;
        }
        p = down_ptr;
        down_ptr = (*mem.offset(p as isize)).b32.s1;
        free_node(p, MOVEMENT_NODE_SIZE as int32_t);
    }
    while right_ptr != TEX_NULL as int32_t {
        if (*mem.offset((right_ptr + 2 as int32_t) as isize)).b32.s1 < l {
            return;
        }
        p = right_ptr;
        right_ptr = (*mem.offset(p as isize)).b32.s1;
        free_node(p, MOVEMENT_NODE_SIZE as int32_t);
    }
}
unsafe extern "C" fn special_out(mut p: int32_t) {
    let mut old_setting: ::core::ffi::c_uchar = 0;
    let mut k: pool_pointer = 0;
    if cur_h != dvi_h {
        movement(cur_h - dvi_h, RIGHT1 as eight_bits);
        dvi_h = cur_h;
    }
    if cur_v != dvi_v {
        movement(cur_v - dvi_v, DOWN1 as eight_bits);
        dvi_v = cur_v;
    }
    doing_special = true_0 != 0;
    old_setting = selector as ::core::ffi::c_uchar;
    selector = SELECTOR_NEW_STRING;
    show_token_list(
        (*mem.offset((*mem.offset((p + 1 as int32_t) as isize)).b32.s1 as isize))
            .b32
            .s1,
        TEX_NULL as int32_t,
        pool_size - pool_ptr as int32_t,
    );
    selector = old_setting as selector_t;
    if pool_ptr + 1 as pool_pointer > pool_size {
        overflow(
            b"pool size\0" as *const u8 as *const ::core::ffi::c_char,
            pool_size - init_pool_ptr as int32_t,
        );
    }
    if cur_length() < 256 as pool_pointer {
        dvi_out(XXX1 as eight_bits);
        dvi_out(cur_length() as eight_bits);
    } else {
        dvi_out(XXX4 as eight_bits);
        dvi_four(cur_length() as int32_t);
    }
    let mut for_end: int32_t = 0;
    k = *str_start.offset((str_ptr - TOO_BIG_CHAR as str_number) as isize);
    for_end = (pool_ptr - 1 as pool_pointer) as int32_t;
    if k <= for_end {
        loop {
            dvi_out(*str_pool.offset(k as isize) as eight_bits);
            let fresh0 = k;
            k = k + 1;
            if !(fresh0 < for_end) {
                break;
            }
        }
    }
    pool_ptr = *str_start.offset((str_ptr - TOO_BIG_CHAR as str_number) as isize);
    doing_special = false_0 != 0;
}
unsafe extern "C" fn write_out(mut p: int32_t) {
    let mut old_setting: ::core::ffi::c_uchar = 0;
    let mut old_mode: int32_t = 0;
    let mut j: small_number = 0;
    let mut q: int32_t = 0;
    let mut r: int32_t = 0;
    let mut d: int32_t = 0;
    q = get_avail();
    (*mem.offset(q as isize)).b32.s0 = (RIGHT_BRACE_TOKEN + '}' as i32) as int32_t;
    r = get_avail();
    (*mem.offset(q as isize)).b32.s1 = r;
    (*mem.offset(r as isize)).b32.s0 = (CS_TOKEN_FLAG + END_WRITE) as int32_t;
    begin_token_list(q, INSERTED as uint16_t);
    begin_token_list(
        (*mem.offset((p + 1 as int32_t) as isize)).b32.s1,
        WRITE_TEXT as uint16_t,
    );
    q = get_avail();
    (*mem.offset(q as isize)).b32.s0 = (LEFT_BRACE_TOKEN + '{' as i32) as int32_t;
    begin_token_list(q, INSERTED as uint16_t);
    old_mode = cur_list.mode as int32_t;
    cur_list.mode = 0 as ::core::ffi::c_short;
    cur_cs = write_loc;
    q = scan_toks(false_0 != 0, true_0 != 0);
    get_token();
    if cur_tok != CS_TOKEN_FLAG as int32_t + END_WRITE as int32_t {
        error_here_with_diagnostic(
            b"Unbalanced write command\0" as *const u8 as *const ::core::ffi::c_char,
        );
        capture_to_diagnostic(::core::ptr::null_mut::<ttbc_diagnostic_t>());
        help_ptr = 2 as ::core::ffi::c_uchar;
        help_line[1 as ::core::ffi::c_int as usize] =
            b"On this page there's a \\write with fewer real {'s than }'s.\0" as *const u8
                as *const ::core::ffi::c_char;
        help_line[0 as ::core::ffi::c_int as usize] = b"I can't handle that very well; good luck.\0"
            as *const u8
            as *const ::core::ffi::c_char;
        error();
        loop {
            get_token();
            if !(cur_tok != CS_TOKEN_FLAG as int32_t + END_WRITE as int32_t) {
                break;
            }
        }
    }
    cur_list.mode = old_mode as ::core::ffi::c_short;
    end_token_list();
    old_setting = selector as ::core::ffi::c_uchar;
    j = (*mem.offset((p + 1 as int32_t) as isize)).b32.s0 as small_number;
    if j as ::core::ffi::c_int == 18 as ::core::ffi::c_int {
        selector = SELECTOR_NEW_STRING;
    } else if write_open[j as usize] {
        selector = j as selector_t;
    } else {
        if j as ::core::ffi::c_int == 17 as ::core::ffi::c_int
            && selector as ::core::ffi::c_uint
                == SELECTOR_TERM_AND_LOG as ::core::ffi::c_int as ::core::ffi::c_uint
        {
            selector = SELECTOR_LOG_ONLY;
        }
        print_nl_cstr(b"\0" as *const u8 as *const ::core::ffi::c_char);
    }
    token_show(def_ref);
    print_ln();
    flush_list(def_ref);
    if j as ::core::ffi::c_int == 18 as ::core::ffi::c_int {
        if (*eqtb.offset((INT_BASE + INT_PAR__tracing_online) as isize))
            .b32
            .s1
            <= 0 as int32_t
        {
            selector = SELECTOR_LOG_ONLY;
        } else {
            selector = SELECTOR_TERM_AND_LOG;
        }
        if !log_opened {
            selector = SELECTOR_TERM_ONLY;
        }
        if !shell_escape_enabled {
            diagnostic_begin_capture_warning_here();
            print_nl_cstr(b"runsystem(\0" as *const u8 as *const ::core::ffi::c_char);
            d = 0 as ::core::ffi::c_int as int32_t;
            while d <= cur_length() - 1 as pool_pointer {
                print(*str_pool.offset(
                    (*str_start.offset((str_ptr - TOO_BIG_CHAR as str_number) as isize)
                        + d as pool_pointer) as isize,
                ) as int32_t);
                d += 1;
            }
            print_cstr(b")...\0" as *const u8 as *const ::core::ffi::c_char);
            print_cstr(b"disabled\0" as *const u8 as *const ::core::ffi::c_char);
            print_char('.' as i32);
            capture_to_diagnostic(::core::ptr::null_mut::<ttbc_diagnostic_t>());
            print_nl_cstr(b"\0" as *const u8 as *const ::core::ffi::c_char);
            print_ln();
        } else {
            ttstub_shell_escape(
                str_pool.offset(
                    *str_start.offset((str_ptr - TOO_BIG_CHAR as str_number) as isize) as isize,
                ) as *mut packed_UTF16_code,
                cur_length() as size_t,
            );
        }
        pool_ptr = *str_start.offset((str_ptr - TOO_BIG_CHAR as str_number) as isize);
    }
    selector = old_setting as selector_t;
}
unsafe extern "C" fn pic_out(mut p: int32_t) {
    let mut old_setting: ::core::ffi::c_uchar = 0;
    let mut i: int32_t = 0;
    let mut k: pool_pointer = 0;
    if cur_h != dvi_h {
        movement(cur_h - dvi_h, RIGHT1 as eight_bits);
        dvi_h = cur_h;
    }
    if cur_v != dvi_v {
        movement(cur_v - dvi_v, DOWN1 as eight_bits);
        dvi_v = cur_v;
    }
    old_setting = selector as ::core::ffi::c_uchar;
    selector = SELECTOR_NEW_STRING;
    print_cstr(b"pdf:image \0" as *const u8 as *const ::core::ffi::c_char);
    print_cstr(b"matrix \0" as *const u8 as *const ::core::ffi::c_char);
    print_scaled((*mem.offset((p + 5 as int32_t) as isize)).b32.s0 as scaled_t);
    print(' ' as i32);
    print_scaled((*mem.offset((p + 5 as int32_t) as isize)).b32.s1 as scaled_t);
    print(' ' as i32);
    print_scaled((*mem.offset((p + 6 as int32_t) as isize)).b32.s0 as scaled_t);
    print(' ' as i32);
    print_scaled((*mem.offset((p + 6 as int32_t) as isize)).b32.s1 as scaled_t);
    print(' ' as i32);
    print_scaled((*mem.offset((p + 7 as int32_t) as isize)).b32.s0 as scaled_t);
    print(' ' as i32);
    print_scaled((*mem.offset((p + 7 as int32_t) as isize)).b32.s1 as scaled_t);
    print(' ' as i32);
    print_cstr(b"page \0" as *const u8 as *const ::core::ffi::c_char);
    print_int((*mem.offset((p + 4 as int32_t) as isize)).b16.s0 as int32_t);
    print(' ' as i32);
    match (*mem.offset((p + 8 as int32_t) as isize)).b16.s1 as ::core::ffi::c_int {
        1 => {
            print_cstr(b"pagebox cropbox \0" as *const u8 as *const ::core::ffi::c_char);
        }
        2 => {
            print_cstr(b"pagebox mediabox \0" as *const u8 as *const ::core::ffi::c_char);
        }
        3 => {
            print_cstr(b"pagebox bleedbox \0" as *const u8 as *const ::core::ffi::c_char);
        }
        5 => {
            print_cstr(b"pagebox artbox \0" as *const u8 as *const ::core::ffi::c_char);
        }
        4 => {
            print_cstr(b"pagebox trimbox \0" as *const u8 as *const ::core::ffi::c_char);
        }
        _ => {}
    }
    print('(' as i32);
    i = 0 as ::core::ffi::c_int as int32_t;
    while i < (*mem.offset((p + 4 as int32_t) as isize)).b16.s1 as int32_t {
        print_raw_char(
            *(mem.offset((p + PIC_NODE_SIZE as int32_t) as isize) as *mut memory_word
                as *mut ::core::ffi::c_uchar)
                .offset(i as isize) as UTF16_code,
            true_0 != 0,
        );
        i += 1;
    }
    print(')' as i32);
    selector = old_setting as selector_t;
    if cur_length() < 256 as pool_pointer {
        dvi_out(XXX1 as eight_bits);
        dvi_out(cur_length() as eight_bits);
    } else {
        dvi_out(XXX4 as eight_bits);
        dvi_four(cur_length() as int32_t);
    }
    k = *str_start.offset((str_ptr - TOO_BIG_CHAR as str_number) as isize);
    while k < pool_ptr {
        dvi_out(*str_pool.offset(k as isize) as eight_bits);
        k += 1;
    }
    pool_ptr = *str_start.offset((str_ptr - TOO_BIG_CHAR as str_number) as isize);
}
#[no_mangle]
pub unsafe extern "C" fn finalize_dvi_file() {
    let mut k: ::core::ffi::c_uchar = 0;
    while cur_s > -(1 as int32_t) {
        if cur_s > 0 as int32_t {
            dvi_out(POP as eight_bits);
        } else {
            dvi_eop();
        }
        cur_s -= 1;
    }
    if total_pages == 0 as int32_t {
        print_nl_cstr(b"No pages of output.\0" as *const u8 as *const ::core::ffi::c_char);
        return;
    }
    if cur_s == -(2 as int32_t) {
        return;
    }
    dvi_out(POST as eight_bits);
    dvi_four(last_bop);
    last_bop = dvi_offset + dvi_ptr - 5 as int32_t;
    dvi_four(25400000 as int32_t);
    dvi_four(473628672 as int32_t);
    prepare_mag();
    dvi_four((*eqtb.offset((INT_BASE + INT_PAR__mag) as isize)).b32.s1);
    dvi_four(max_v as int32_t);
    dvi_four(max_h as int32_t);
    dvi_out((max_push / 256 as int32_t) as eight_bits);
    dvi_out((max_push % 256 as int32_t) as eight_bits);
    dvi_out((total_pages / 256 as int32_t % 256 as int32_t) as eight_bits);
    dvi_out((total_pages % 256 as int32_t) as eight_bits);
    while font_ptr > FONT_BASE as internal_font_number {
        if *font_used.offset(font_ptr as isize) {
            dvi_font_def(font_ptr);
        }
        font_ptr -= 1;
    }
    dvi_out(POST_POST as eight_bits);
    dvi_four(last_bop);
    if semantic_pagination_enabled {
        dvi_out(SPX_ID_BYTE as eight_bits);
    } else {
        dvi_out(XDV_ID_BYTE as eight_bits);
    }
    k = (4 as int32_t + (DVI_BUF_SIZE as int32_t - dvi_ptr) % 4 as int32_t) as ::core::ffi::c_uchar;
    while k as ::core::ffi::c_int > 0 as ::core::ffi::c_int {
        dvi_out(223 as eight_bits);
        k = k.wrapping_sub(1);
    }
    if dvi_limit == HALF_BUF as int32_t {
        write_to_dvi(HALF_BUF as int32_t, DVI_BUF_SIZE as int32_t - 1 as int32_t);
    }
    if dvi_ptr > TEX_INFINITY as int32_t - dvi_offset {
        cur_s = -(2 as ::core::ffi::c_int) as int32_t;
        fatal_error(b"dvi length exceeds 0x7FFFFFFF\0" as *const u8 as *const ::core::ffi::c_char);
    }
    if dvi_ptr > 0 as int32_t {
        write_to_dvi(0 as int32_t, dvi_ptr - 1 as int32_t);
    }
    k = ttstub_output_close(dvi_file) as ::core::ffi::c_uchar;
    if k as ::core::ffi::c_int == 0 as ::core::ffi::c_int {
        print_nl_cstr(b"Output written on \0" as *const u8 as *const ::core::ffi::c_char);
        print(output_file_name as int32_t);
        print_cstr(b" (\0" as *const u8 as *const ::core::ffi::c_char);
        print_int(total_pages);
        if total_pages != 1 as int32_t {
            print_cstr(b" pages\0" as *const u8 as *const ::core::ffi::c_char);
        } else {
            print_cstr(b" page\0" as *const u8 as *const ::core::ffi::c_char);
        }
        print_cstr(b", \0" as *const u8 as *const ::core::ffi::c_char);
        print_int(dvi_offset + dvi_ptr);
        print_cstr(b" bytes).\0" as *const u8 as *const ::core::ffi::c_char);
    } else {
        print_nl_cstr(b"Error \0" as *const u8 as *const ::core::ffi::c_char);
        print_int(k as int32_t);
        print_cstr(b" (\0" as *const u8 as *const ::core::ffi::c_char);
        print_c_string(strerror(k as ::core::ffi::c_int));
        print_cstr(b") generating output;\0" as *const u8 as *const ::core::ffi::c_char);
        print_nl_cstr(b"file \0" as *const u8 as *const ::core::ffi::c_char);
        print(output_file_name as int32_t);
        print_cstr(b" may not be valid.\0" as *const u8 as *const ::core::ffi::c_char);
    };
}
unsafe extern "C" fn write_to_dvi(mut a: int32_t, mut b: int32_t) {
    let mut n: int32_t = b - a + 1 as int32_t;
    if ttstub_output_write(
        dvi_file,
        dvi_buf.offset(a as isize) as *mut eight_bits as *mut ::core::ffi::c_char,
        n as size_t,
    ) != n as size_t
    {
        _tt_abort(b"failed to write data to XDV file\0" as *const u8 as *const ::core::ffi::c_char);
    }
}
unsafe extern "C" fn dvi_flush() {
    if dvi_limit != DVI_BUF_SIZE as int32_t {
        write_to_dvi(HALF_BUF as int32_t, DVI_BUF_SIZE as int32_t - 1 as int32_t);
        dvi_gone = dvi_gone + HALF_BUF as int32_t;
    }
    write_to_dvi(0 as int32_t, dvi_ptr - 1 as int32_t);
    dvi_limit = DVI_BUF_SIZE as int32_t;
    dvi_offset = dvi_offset + dvi_ptr;
    dvi_gone = dvi_gone + dvi_ptr;
    dvi_ptr = 0 as ::core::ffi::c_int as int32_t;
}
unsafe extern "C" fn dvi_eop() {
    dvi_out(EOP as eight_bits);
    total_pages += 1;
    dvi_flush();
    ttstub_output_flush(dvi_file);
}
unsafe extern "C" fn dvi_swap() {
    if dvi_ptr > TEX_INFINITY as int32_t - dvi_offset {
        cur_s = -(2 as ::core::ffi::c_int) as int32_t;
        fatal_error(b"dvi length exceeds 0x7FFFFFFF\0" as *const u8 as *const ::core::ffi::c_char);
    }
    if dvi_limit == DVI_BUF_SIZE as int32_t {
        write_to_dvi(0 as int32_t, HALF_BUF as int32_t - 1 as int32_t);
        dvi_limit = HALF_BUF as int32_t;
        dvi_offset = dvi_offset + DVI_BUF_SIZE as int32_t;
        dvi_ptr = 0 as ::core::ffi::c_int as int32_t;
    } else {
        write_to_dvi(HALF_BUF as int32_t, DVI_BUF_SIZE as int32_t - 1 as int32_t);
        dvi_limit = DVI_BUF_SIZE as int32_t;
    }
    dvi_gone = dvi_gone + HALF_BUF as int32_t;
}
unsafe extern "C" fn dvi_four(mut x: int32_t) {
    if x >= 0 as int32_t {
        dvi_out((x / 0x1000000 as int32_t) as eight_bits);
    } else {
        x = x + 0x40000000 as int32_t;
        x = x + 0x40000000 as int32_t;
        dvi_out((x / 0x1000000 as int32_t + 128 as int32_t) as eight_bits);
    }
    x = x % 0x1000000 as int32_t;
    dvi_out((x / 0x10000 as int32_t) as eight_bits);
    x = x % 0x10000 as int32_t;
    dvi_out((x / 0x100 as int32_t) as eight_bits);
    dvi_out((x % 0x100 as int32_t) as eight_bits);
}
unsafe extern "C" fn dvi_two(mut s: UTF16_code) {
    dvi_out((s as ::core::ffi::c_int / 0x100 as ::core::ffi::c_int) as eight_bits);
    dvi_out((s as ::core::ffi::c_int % 0x100 as ::core::ffi::c_int) as eight_bits);
}
unsafe extern "C" fn dvi_pop(mut l: int32_t) {
    if l == dvi_offset + dvi_ptr && dvi_ptr > 0 as int32_t {
        dvi_ptr -= 1;
    } else {
        dvi_out(POP as eight_bits);
    };
}
pub const true_0: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const false_0: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
