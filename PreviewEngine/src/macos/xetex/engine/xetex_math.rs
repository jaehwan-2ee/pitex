/* Copyright 2016-2018 The Tectonic Project
 * Licensed under the MIT License.
 */
// Translated from xetex/engine/xetex-math.c with C2Rust 0.22.1.
extern "C" {
    pub type ttbc_diagnostic_t;
    pub type XeTeXLayoutEngine_rec;
    fn abs(_: ::core::ffi::c_int) -> ::core::ffi::c_int;
    fn usingOpenType(engine: XeTeXLayoutEngine) -> bool;
    fn isOpenTypeMathFont(engine: XeTeXLayoutEngine) -> bool;
    fn measure_native_glyph(node: *mut ::core::ffi::c_void, use_glyph_metrics: ::core::ffi::c_int);
    fn map_char_to_glyph(font: int32_t, ch: int32_t) -> int32_t;
    fn real_get_native_glyph(
        pNode: *mut ::core::ffi::c_void,
        index: ::core::ffi::c_uint,
    ) -> uint16_t;
    fn get_native_mathsy_param(f: ::core::ffi::c_int, n: ::core::ffi::c_int) -> ::core::ffi::c_int;
    fn get_native_mathex_param(f: ::core::ffi::c_int, n: ::core::ffi::c_int) -> ::core::ffi::c_int;
    fn get_ot_math_constant(f: ::core::ffi::c_int, n: ::core::ffi::c_int) -> ::core::ffi::c_int;
    fn get_ot_math_variant(
        f: ::core::ffi::c_int,
        g: ::core::ffi::c_int,
        v: ::core::ffi::c_int,
        adv: *mut int32_t,
        horiz: ::core::ffi::c_int,
    ) -> ::core::ffi::c_int;
    fn get_ot_assembly_ptr(
        f: ::core::ffi::c_int,
        g: ::core::ffi::c_int,
        horiz: ::core::ffi::c_int,
    ) -> *mut ::core::ffi::c_void;
    fn free_ot_assembly(a: *mut GlyphAssembly);
    fn get_ot_math_ital_corr(f: ::core::ffi::c_int, g: ::core::ffi::c_int) -> ::core::ffi::c_int;
    fn get_ot_math_accent_pos(f: ::core::ffi::c_int, g: ::core::ffi::c_int) -> ::core::ffi::c_int;
    fn get_ot_math_kern(
        f: ::core::ffi::c_int,
        g: ::core::ffi::c_int,
        sf: ::core::ffi::c_int,
        sg: ::core::ffi::c_int,
        cmd: ::core::ffi::c_int,
        shift: ::core::ffi::c_int,
    ) -> ::core::ffi::c_int;
    fn ot_part_count(a: *const GlyphAssembly) -> ::core::ffi::c_int;
    fn ot_part_glyph(a: *const GlyphAssembly, i: ::core::ffi::c_int) -> ::core::ffi::c_int;
    fn ot_part_is_extender(a: *const GlyphAssembly, i: ::core::ffi::c_int) -> bool;
    fn ot_part_start_connector(
        f: ::core::ffi::c_int,
        a: *const GlyphAssembly,
        i: ::core::ffi::c_int,
    ) -> ::core::ffi::c_int;
    fn ot_part_end_connector(
        f: ::core::ffi::c_int,
        a: *const GlyphAssembly,
        i: ::core::ffi::c_int,
    ) -> ::core::ffi::c_int;
    fn ot_part_full_advance(
        f: ::core::ffi::c_int,
        a: *const GlyphAssembly,
        i: ::core::ffi::c_int,
    ) -> ::core::ffi::c_int;
    fn ot_min_connector_overlap(f: ::core::ffi::c_int) -> ::core::ffi::c_int;
    static mut eqtb: *mut memory_word;
    static mut file_line_error_style_p: ::core::ffi::c_int;
    static mut insert_src_special_every_math: bool;
    static mut help_line: [*const ::core::ffi::c_char; 6];
    static mut help_ptr: ::core::ffi::c_uchar;
    static mut tex_remainder: scaled_t;
    static mut temp_ptr: int32_t;
    static mut mem: *mut memory_word;
    static mut hi_mem_min: int32_t;
    static mut avail: int32_t;
    static mut mem_end: int32_t;
    static mut nest_ptr: int32_t;
    static mut cur_list: list_state_record;
    static mut save_stack: *mut memory_word;
    static mut save_ptr: int32_t;
    static mut cur_group: group_code;
    static mut cur_cmd: eight_bits;
    static mut cur_chr: int32_t;
    static mut cur_val: int32_t;
    static mut cur_val1: int32_t;
    static mut font_info: *mut memory_word;
    static mut font_params: *mut font_index;
    static mut font_area: *mut str_number;
    static mut font_bc: *mut UTF16_code;
    static mut font_ec: *mut UTF16_code;
    static mut skew_char: *mut int32_t;
    static mut font_layout_engine: *mut *mut ::core::ffi::c_void;
    static mut char_base: *mut int32_t;
    static mut width_base: *mut int32_t;
    static mut height_base: *mut int32_t;
    static mut depth_base: *mut int32_t;
    static mut italic_base: *mut int32_t;
    static mut lig_kern_base: *mut int32_t;
    static mut kern_base: *mut int32_t;
    static mut exten_base: *mut int32_t;
    static mut param_base: *mut int32_t;
    static mut null_character: b16x4;
    static mut total_shrink: [scaled_t; 4];
    static mut adjust_tail: int32_t;
    static mut pre_adjust_tail: int32_t;
    static mut empty: b32x2;
    static mut cur_f: internal_font_number;
    static mut cur_c: int32_t;
    static mut cur_i: b16x4;
    static mut just_box: int32_t;
    static mut cur_lang: ::core::ffi::c_uchar;
    static mut LR_ptr: int32_t;
    static mut LR_problems: int32_t;
    static mut cur_dir: small_number;
    static mut xtx_ligature_present: bool;
    static mut semantic_pagination_enabled: bool;
    fn get_avail() -> int32_t;
    fn get_node(s: int32_t) -> int32_t;
    fn free_node(p: int32_t, s: int32_t);
    fn new_null_box() -> int32_t;
    fn new_rule() -> int32_t;
    fn new_math(w: scaled_t, s: small_number) -> int32_t;
    fn new_spec(p: int32_t) -> int32_t;
    fn new_param_glue(n: small_number) -> int32_t;
    fn new_glue(q: int32_t) -> int32_t;
    fn new_skip_param(n: small_number) -> int32_t;
    fn new_kern(w: scaled_t) -> int32_t;
    fn new_penalty(m: int32_t) -> int32_t;
    fn delete_glue_ref(p: int32_t);
    fn flush_node_list(p: int32_t);
    fn copy_node_list(p: int32_t) -> int32_t;
    fn push_nest();
    fn pop_nest();
    fn eq_word_define(p: int32_t, w: int32_t);
    fn unsave();
    fn begin_token_list(p: int32_t, t: uint16_t);
    fn back_input();
    fn back_error();
    fn get_token();
    fn get_x_token();
    fn scan_left_brace();
    fn scan_keyword(s: *const ::core::ffi::c_char) -> bool;
    fn scan_usv_num();
    fn scan_math_class_int();
    fn scan_math_fam_int();
    fn scan_fifteen_bit_int();
    fn scan_delimiter_int();
    fn effective_char(err_p: bool, f: internal_font_number, c: uint16_t) -> int32_t;
    fn scan_dimen(mu: bool, inf: bool, shortcut: bool);
    fn char_warning(f: internal_font_number, c: int32_t);
    fn new_native_character(f: internal_font_number, c: UnicodeScalar) -> int32_t;
    fn new_character(f: internal_font_number, c: UTF16_code) -> int32_t;
    fn hpack(p: int32_t, w: scaled_t, m: small_number) -> int32_t;
    fn vpackage(p: int32_t, h: scaled_t, m: small_number, l: scaled_t) -> int32_t;
    fn append_to_vlist(b: int32_t);
    fn new_noad() -> int32_t;
    fn new_choice() -> int32_t;
    fn line_break(d: bool);
    fn off_save();
    fn norm_min(h: int32_t) -> small_number;
    fn push_math(c: group_code);
    fn just_copy(p: int32_t, h: int32_t, t: int32_t);
    fn just_reverse(p: int32_t);
    fn scan_math(p: int32_t);
    fn insert_src_special();
    fn error();
    fn confusion(s: *const ::core::ffi::c_char) -> !;
    fn capture_to_diagnostic(diagnostic: *mut ttbc_diagnostic_t);
    fn error_here_with_diagnostic(message: *const ::core::ffi::c_char) -> *mut ttbc_diagnostic_t;
    fn print_char(s: int32_t);
    fn print(s: int32_t);
    fn print_cstr(s: *const ::core::ffi::c_char);
    fn print_nl_cstr(s: *const ::core::ffi::c_char);
    fn print_esc_cstr(s: *const ::core::ffi::c_char);
    fn print_int(n: int32_t);
    fn print_size(s: int32_t);
    fn print_file_line();
    fn build_page();
    fn tex_round(_: ::core::ffi::c_double) -> int32_t;
    fn half(x: int32_t) -> int32_t;
    fn mult_and_add(n: int32_t, x: scaled_t, y: scaled_t, max_answer: scaled_t) -> scaled_t;
    fn x_over_n(x: scaled_t, n: int32_t) -> scaled_t;
    fn xn_over_d(x: scaled_t, n: int32_t, d: int32_t) -> scaled_t;
    fn tt_insert_special(ascii_text: *const ::core::ffi::c_char);
}
pub type int32_t = i32;
pub type uint16_t = u16;
pub type uint32_t = u32;
pub type hb_codepoint_t = uint32_t;
pub type hb_position_t = int32_t;
pub type hb_ot_math_glyph_part_flags_t = ::core::ffi::c_uint;
pub const HB_OT_MATH_GLYPH_PART_FLAG_EXTENDER: hb_ot_math_glyph_part_flags_t = 1;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct hb_ot_math_glyph_part_t {
    pub glyph: hb_codepoint_t,
    pub start_connector_length: hb_position_t,
    pub end_connector_length: hb_position_t,
    pub full_advance: hb_position_t,
    pub flags: hb_ot_math_glyph_part_flags_t,
}
pub type scaled_t = int32_t;
pub type XeTeXLayoutEngine = *mut XeTeXLayoutEngine_rec;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct GlyphAssembly {
    pub count: ::core::ffi::c_uint,
    pub parts: *mut hb_ot_math_glyph_part_t,
}
pub type eight_bits = ::core::ffi::c_uchar;
pub type UTF16_code = ::core::ffi::c_ushort;
pub type UnicodeScalar = int32_t;
pub type str_number = int32_t;
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
pub type group_code = ::core::ffi::c_uchar;
pub type internal_font_number = int32_t;
pub type font_index = int32_t;
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
pub const DEFAULT_CODE: ::core::ffi::c_int = 0x40000000 as ::core::ffi::c_int;
pub const BIGGEST_USV: ::core::ffi::c_int = 0x10ffff as ::core::ffi::c_int;
pub const NUMBER_USVS: ::core::ffi::c_int = BIGGEST_USV + 1 as ::core::ffi::c_int;
pub const NUMBER_MATH_FAMILIES: ::core::ffi::c_int = 256 as ::core::ffi::c_int;
pub const TEMP_HEAD: ::core::ffi::c_int = MEM_TOP - 3 as ::core::ffi::c_int;
pub const ADJUST_HEAD: ::core::ffi::c_int = MEM_TOP - 5 as ::core::ffi::c_int;
pub const GARBAGE: ::core::ffi::c_int = MEM_TOP - 12 as ::core::ffi::c_int;
pub const PRE_ADJUST_HEAD: ::core::ffi::c_int = MEM_TOP - 14 as ::core::ffi::c_int;
pub const LEFT_TO_RIGHT: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const RIGHT_TO_LEFT: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const MEDIUM_NODE_SIZE: ::core::ffi::c_int = 3 as ::core::ffi::c_int;
pub const STYLE_NODE_SIZE: ::core::ffi::c_int = 3 as ::core::ffi::c_int;
pub const GLUE_SPEC_SIZE: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
pub const GLYPH_NODE_SIZE: ::core::ffi::c_int = 5 as ::core::ffi::c_int;
pub const BOX_NODE_SIZE: ::core::ffi::c_int = 8 as ::core::ffi::c_int;
pub const NOAD_SIZE: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
pub const ACCENT_NOAD_SIZE: ::core::ffi::c_int = 5 as ::core::ffi::c_int;
pub const RADICAL_NOAD_SIZE: ::core::ffi::c_int = 5 as ::core::ffi::c_int;
pub const FRACTION_NOAD_SIZE: ::core::ffi::c_int = 6 as ::core::ffi::c_int;
pub const L_CODE: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
pub const R_CODE: ::core::ffi::c_int = 8 as ::core::ffi::c_int;
pub const MATH_CHOICE_GROUP: ::core::ffi::c_int = 13 as ::core::ffi::c_int;
pub const MATH_SHIFT_GROUP: ::core::ffi::c_int = 15 as ::core::ffi::c_int;
pub const MATH_LEFT_GROUP: ::core::ffi::c_int = 16 as ::core::ffi::c_int;
pub const SUP_CMD: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const SUB_CMD: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const FIL: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const FILL: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const FILLL: ::core::ffi::c_int = 3 as ::core::ffi::c_int;
pub const LIG_TAG: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const LIST_TAG: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const EXT_TAG: ::core::ffi::c_int = 3 as ::core::ffi::c_int;
pub const NORMAL: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const DELIMITED_CODE: ::core::ffi::c_int = 3 as ::core::ffi::c_int;
pub const BEGIN_L_CODE: ::core::ffi::c_int = 6 as ::core::ffi::c_int;
pub const END_L_CODE: ::core::ffi::c_int = 7 as ::core::ffi::c_int;
pub const EVERY_MATH_TEXT: ::core::ffi::c_int = 9 as ::core::ffi::c_int;
pub const EVERY_DISPLAY_TEXT: ::core::ffi::c_int = 10 as ::core::ffi::c_int;
pub const EMPTY: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const EXACTLY: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const FONT_BASE: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const ADDITIONAL: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const FIXED_ACC: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const MATH_CHAR: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const STRETCHING: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const BOTTOM_ACC: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const DLIST: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const SHRINKING: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const SPACE_CODE: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const SUB_BOX: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const DISPLAYOPERATORMINHEIGHT: ::core::ffi::c_int = 3 as ::core::ffi::c_int;
pub const MATH_SHIFT: ::core::ffi::c_int = 3 as ::core::ffi::c_int;
pub const SUB_MLIST: ::core::ffi::c_int = 3 as ::core::ffi::c_int;
pub const MATH_TEXT_CHAR: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
pub const X_HEIGHT_CODE: ::core::ffi::c_int = 5 as ::core::ffi::c_int;
pub const ACCENTBASEHEIGHT: ::core::ffi::c_int = 6 as ::core::ffi::c_int;
pub const QUAD_CODE: ::core::ffi::c_int = 6 as ::core::ffi::c_int;
pub const SUP_MARK: ::core::ffi::c_int = 7 as ::core::ffi::c_int;
pub const SUBSCRIPTTOPMAX: ::core::ffi::c_int = 9 as ::core::ffi::c_int;
pub const SUPERSCRIPTBOTTOMMIN: ::core::ffi::c_int = 13 as ::core::ffi::c_int;
pub const TOTAL_MATHEX_PARAMS: ::core::ffi::c_int = 13 as ::core::ffi::c_int;
pub const SUBSUPERSCRIPTGAPMIN: ::core::ffi::c_int = 15 as ::core::ffi::c_int;
pub const SUPERSCRIPTBOTTOMMAXWITHSUBSCRIPT: ::core::ffi::c_int = 16 as ::core::ffi::c_int;
pub const TOTAL_MATHSY_PARAMS: ::core::ffi::c_int = 22 as ::core::ffi::c_int;
pub const STACKGAPMIN: ::core::ffi::c_int = 26 as ::core::ffi::c_int;
pub const STACKDISPLAYSTYLEGAPMIN: ::core::ffi::c_int = 27 as ::core::ffi::c_int;
pub const FRACTIONNUMERATORGAPMIN: ::core::ffi::c_int = 36 as ::core::ffi::c_int;
pub const FRACTIONNUMDISPLAYSTYLEGAPMIN: ::core::ffi::c_int = 37 as ::core::ffi::c_int;
pub const FRACTIONDENOMINATORGAPMIN: ::core::ffi::c_int = 39 as ::core::ffi::c_int;
pub const FRACTIONDENOMDISPLAYSTYLEGAPMIN: ::core::ffi::c_int = 40 as ::core::ffi::c_int;
pub const RADICALVERTICALGAP: ::core::ffi::c_int = 49 as ::core::ffi::c_int;
pub const RADICALDISPLAYSTYLEVERTICALGAP: ::core::ffi::c_int = 50 as ::core::ffi::c_int;
pub const RADICALRULETHICKNESS: ::core::ffi::c_int = 51 as ::core::ffi::c_int;
pub const COND_MATH_GLUE: ::core::ffi::c_int = 98 as ::core::ffi::c_int;
pub const BIGGEST_LANG: ::core::ffi::c_int = 255 as ::core::ffi::c_int;
pub const INF_PENALTY: ::core::ffi::c_int = 10000 as ::core::ffi::c_int;
pub const AAT_FONT_FLAG: ::core::ffi::c_uint = 0xffff as ::core::ffi::c_uint;
pub const OTGR_FONT_FLAG: ::core::ffi::c_uint = 0xfffe as ::core::ffi::c_uint;
pub const MEM_TOP: ::core::ffi::c_int = 4999999 as ::core::ffi::c_int;
pub const INT_PAR__bin_op_penalty: ::core::ffi::c_int = 9 as ::core::ffi::c_int;
pub const INT_PAR__rel_penalty: ::core::ffi::c_int = 10 as ::core::ffi::c_int;
pub const INT_PAR__pre_display_penalty: ::core::ffi::c_int = 11 as ::core::ffi::c_int;
pub const INT_PAR__post_display_penalty: ::core::ffi::c_int = 12 as ::core::ffi::c_int;
pub const INT_PAR__delimiter_factor: ::core::ffi::c_int = 18 as ::core::ffi::c_int;
pub const INT_PAR__hang_after: ::core::ffi::c_int = 41 as ::core::ffi::c_int;
pub const INT_PAR__cur_fam: ::core::ffi::c_int = 44 as ::core::ffi::c_int;
pub const INT_PAR__language: ::core::ffi::c_int = 50 as ::core::ffi::c_int;
pub const INT_PAR__left_hyphen_min: ::core::ffi::c_int = 51 as ::core::ffi::c_int;
pub const INT_PAR__right_hyphen_min: ::core::ffi::c_int = 52 as ::core::ffi::c_int;
pub const INT_PAR__pre_display_direction: ::core::ffi::c_int = 61 as ::core::ffi::c_int;
pub const INT_PAR__texxet: ::core::ffi::c_int = 69 as ::core::ffi::c_int;
pub const INT_PARS: ::core::ffi::c_int = 89 as ::core::ffi::c_int;
pub const DIMEN_PAR__math_surround: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const DIMEN_PAR__hsize: ::core::ffi::c_int = 3 as ::core::ffi::c_int;
pub const DIMEN_PAR__delimiter_shortfall: ::core::ffi::c_int = 10 as ::core::ffi::c_int;
pub const DIMEN_PAR__null_delimiter_space: ::core::ffi::c_int = 11 as ::core::ffi::c_int;
pub const DIMEN_PAR__script_space: ::core::ffi::c_int = 12 as ::core::ffi::c_int;
pub const DIMEN_PAR__pre_display_size: ::core::ffi::c_int = 13 as ::core::ffi::c_int;
pub const DIMEN_PAR__display_width: ::core::ffi::c_int = 14 as ::core::ffi::c_int;
pub const DIMEN_PAR__display_indent: ::core::ffi::c_int = 15 as ::core::ffi::c_int;
pub const DIMEN_PAR__hang_indent: ::core::ffi::c_int = 17 as ::core::ffi::c_int;
pub const GLUE_PAR__above_display_skip: ::core::ffi::c_int = 3 as ::core::ffi::c_int;
pub const GLUE_PAR__below_display_skip: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
pub const GLUE_PAR__above_display_short_skip: ::core::ffi::c_int = 5 as ::core::ffi::c_int;
pub const GLUE_PAR__below_display_short_skip: ::core::ffi::c_int = 6 as ::core::ffi::c_int;
pub const GLUE_PAR__left_skip: ::core::ffi::c_int = 7 as ::core::ffi::c_int;
pub const GLUE_PAR__right_skip: ::core::ffi::c_int = 8 as ::core::ffi::c_int;
pub const GLUE_PAR__thin_mu_skip: ::core::ffi::c_int = 16 as ::core::ffi::c_int;
pub const GLUE_PAR__med_mu_skip: ::core::ffi::c_int = 17 as ::core::ffi::c_int;
pub const GLUE_PAR__thick_mu_skip: ::core::ffi::c_int = 18 as ::core::ffi::c_int;
pub const LOCAL__par_shape: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const LOCAL__every_math: ::core::ffi::c_int = 3 as ::core::ffi::c_int;
pub const LOCAL__every_display: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
pub const GLUE_BASE: ::core::ffi::c_int = 2254340 as ::core::ffi::c_int;
pub const LOCAL_BASE: ::core::ffi::c_int = 2254871 as ::core::ffi::c_int;
pub const CUR_FONT_LOC: ::core::ffi::c_int = 2255400 as ::core::ffi::c_int;
pub const MATH_FONT_BASE: ::core::ffi::c_int = 2255401 as ::core::ffi::c_int;
pub const INT_BASE: ::core::ffi::c_int = 7826729 as ::core::ffi::c_int;
pub const COUNT_BASE: ::core::ffi::c_int = INT_BASE + INT_PARS;
pub const DEL_CODE_BASE: ::core::ffi::c_int = COUNT_BASE + 256 as ::core::ffi::c_int;
pub const DIMEN_BASE: ::core::ffi::c_int = DEL_CODE_BASE + NUMBER_USVS;
pub const RELAX: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const SPACER: ::core::ffi::c_int = 10 as ::core::ffi::c_int;
pub const LETTER: ::core::ffi::c_int = 11;
pub const OTHER_CHAR: ::core::ffi::c_int = 12;
pub const DELIM_NUM: ::core::ffi::c_int = 15;
pub const ACCENT: ::core::ffi::c_int = 45 as ::core::ffi::c_int;
pub const HMODE: ::core::ffi::c_int = 104 as ::core::ffi::c_int;
pub const MMODE: ::core::ffi::c_int = 207 as ::core::ffi::c_int;
pub const TEXT_SIZE: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const SCRIPT_SIZE: ::core::ffi::c_int = 256 as ::core::ffi::c_int;
pub const SCRIPT_SCRIPT_SIZE: ::core::ffi::c_int = 512 as ::core::ffi::c_int;
pub const HLIST_NODE: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const VLIST_NODE: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const RULE_NODE: ::core::ffi::c_int = 2;
pub const INS_NODE: ::core::ffi::c_int = 3;
pub const MARK_NODE: ::core::ffi::c_int = 4;
pub const ADJUST_NODE: ::core::ffi::c_int = 5;
pub const DISC_NODE: ::core::ffi::c_int = 7;
pub const WHATSIT_NODE: ::core::ffi::c_int = 8;
pub const GLUE_NODE: ::core::ffi::c_int = 10 as ::core::ffi::c_int;
pub const KERN_NODE: ::core::ffi::c_int = 11;
pub const PENALTY_NODE: ::core::ffi::c_int = 12 as ::core::ffi::c_int;
pub const STYLE_NODE: ::core::ffi::c_int = 14;
pub const CHOICE_NODE: ::core::ffi::c_int = 15 as ::core::ffi::c_int;
pub const ORD_NOAD: ::core::ffi::c_int = 16 as ::core::ffi::c_int;
pub const OP_NOAD: ::core::ffi::c_int = 17;
pub const BIN_NOAD: ::core::ffi::c_int = 18 as ::core::ffi::c_int;
pub const REL_NOAD: ::core::ffi::c_int = 19 as ::core::ffi::c_int;
pub const OPEN_NOAD: ::core::ffi::c_int = 20 as ::core::ffi::c_int;
pub const CLOSE_NOAD: ::core::ffi::c_int = 21;
pub const PUNCT_NOAD: ::core::ffi::c_int = 22;
pub const INNER_NOAD: ::core::ffi::c_int = 23;
pub const RADICAL_NOAD: ::core::ffi::c_int = 24;
pub const FRACTION_NOAD: ::core::ffi::c_int = 25;
pub const UNDER_NOAD: ::core::ffi::c_int = 26;
pub const OVER_NOAD: ::core::ffi::c_int = 27;
pub const ACCENT_NOAD: ::core::ffi::c_int = 28;
pub const VCENTER_NOAD: ::core::ffi::c_int = 29;
pub const LEFT_NOAD: ::core::ffi::c_int = 30 as ::core::ffi::c_int;
pub const RIGHT_NOAD: ::core::ffi::c_int = 31 as ::core::ffi::c_int;
pub const MU_GLUE: ::core::ffi::c_int = 99 as ::core::ffi::c_int;
pub const A_LEADERS: ::core::ffi::c_int = 100 as ::core::ffi::c_int;
pub const EXPLICIT: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const BEFORE: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const AFTER: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const BEGIN_M_CODE: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const END_M_CODE: ::core::ffi::c_int = 3 as ::core::ffi::c_int;
pub const DISPLAY_STYLE: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const TEXT_STYLE: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const SCRIPT_STYLE: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
pub const LIMITS: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const NATIVE_WORD_NODE: ::core::ffi::c_int = 40 as ::core::ffi::c_int;
pub const NATIVE_WORD_NODE_AT: ::core::ffi::c_int = 41 as ::core::ffi::c_int;
pub const GLYPH_NODE: ::core::ffi::c_int = 42 as ::core::ffi::c_int;
pub const PIC_NODE: ::core::ffi::c_int = 43 as ::core::ffi::c_int;
pub const PDF_NODE: ::core::ffi::c_int = 44 as ::core::ffi::c_int;
pub const ABOVE_CODE: ::core::ffi::c_int = 0;
pub const OVER_CODE: ::core::ffi::c_int = 1;
pub const ATOP_CODE: ::core::ffi::c_int = 2;
#[inline]
unsafe extern "C" fn is_char_node(p: int32_t) -> bool {
    return p >= hi_mem_min;
}
static mut null_delimiter: b16x4 = b16x4_le_t {
    s0: 0,
    s1: 0,
    s2: 0,
    s3: 0,
};
static mut cur_mlist: int32_t = 0;
static mut cur_style: small_number = 0;
static mut cur_size: int32_t = 0;
static mut cur_mu: scaled_t = 0;
static mut mlist_penalties: bool = false;
#[no_mangle]
pub unsafe extern "C" fn initialize_math_variables() {
    null_delimiter.s3 = 0 as uint16_t;
    null_delimiter.s2 = 0 as uint16_t;
    null_delimiter.s1 = 0 as uint16_t;
    null_delimiter.s0 = 0 as uint16_t;
}
#[no_mangle]
pub unsafe extern "C" fn init_math() {
    let mut current_block: u64;
    let mut w: scaled_t = 0;
    let mut j: int32_t = 0;
    let mut x: int32_t = 0;
    let mut l: scaled_t = 0;
    let mut s: scaled_t = 0;
    let mut p: int32_t = 0;
    let mut q: int32_t = 0;
    let mut f: internal_font_number = 0;
    let mut n: int32_t = 0;
    let mut v: scaled_t = 0;
    let mut d: scaled_t = 0;
    get_token();
    if cur_cmd as ::core::ffi::c_int == MATH_SHIFT
        && cur_list.mode as ::core::ffi::c_int > 0 as ::core::ffi::c_int
    {
        j = TEX_NULL as int32_t;
        w = -MAX_HALFWORD as scaled_t;
        if cur_list.head == cur_list.tail {
            pop_nest();
            if cur_list.eTeX_aux == TEX_NULL as int32_t {
                x = 0 as ::core::ffi::c_int as int32_t;
            } else if (*mem.offset(cur_list.eTeX_aux as isize)).b32.s0 >= R_CODE as int32_t {
                x = -(1 as ::core::ffi::c_int) as int32_t;
            } else {
                x = 1 as ::core::ffi::c_int as int32_t;
            }
        } else {
            line_break(true_0 != 0);
            if (*eqtb.offset((GLUE_BASE + GLUE_PAR__right_skip) as isize))
                .b32
                .s1
                == 0 as int32_t
            {
                j = new_kern(0 as scaled_t);
            } else {
                j = new_param_glue(GLUE_PAR__right_skip as small_number);
            }
            if (*eqtb.offset((GLUE_BASE + GLUE_PAR__left_skip) as isize))
                .b32
                .s1
                == 0 as int32_t
            {
                p = new_kern(0 as scaled_t);
            } else {
                p = new_param_glue(GLUE_PAR__left_skip as small_number);
            }
            (*mem.offset(p as isize)).b32.s1 = j;
            j = new_null_box();
            (*mem.offset((j + 1 as int32_t) as isize)).b32.s1 =
                (*mem.offset((just_box + 1 as int32_t) as isize)).b32.s1;
            (*mem.offset((j + 4 as int32_t) as isize)).b32.s1 =
                (*mem.offset((just_box + 4 as int32_t) as isize)).b32.s1;
            (*mem.offset((j + 5 as int32_t) as isize)).b32.s1 = p;
            (*mem.offset((j + 5 as int32_t) as isize)).b16.s0 =
                (*mem.offset((just_box + 5 as int32_t) as isize)).b16.s0;
            (*mem.offset((j + 5 as int32_t) as isize)).b16.s1 =
                (*mem.offset((just_box + 5 as int32_t) as isize)).b16.s1;
            (*mem.offset((j + 6 as int32_t) as isize)).gr =
                (*mem.offset((just_box + 6 as int32_t) as isize)).gr;
            v = (*mem.offset((just_box + 4 as int32_t) as isize)).b32.s1 as scaled_t;
            if cur_list.eTeX_aux == TEX_NULL as int32_t {
                x = 0 as ::core::ffi::c_int as int32_t;
            } else if (*mem.offset(cur_list.eTeX_aux as isize)).b32.s0 >= R_CODE as int32_t {
                x = -(1 as ::core::ffi::c_int) as int32_t;
            } else {
                x = 1 as ::core::ffi::c_int as int32_t;
            }
            if x >= 0 as int32_t {
                p = (*mem.offset((just_box + 5 as int32_t) as isize)).b32.s1;
                (*mem.offset(TEMP_HEAD as isize)).b32.s1 = TEX_NULL as int32_t;
            } else {
                v = (-v - (*mem.offset((just_box + 1 as int32_t) as isize)).b32.s1) as scaled_t;
                p = new_math(0 as scaled_t, BEGIN_L_CODE as small_number);
                (*mem.offset(TEMP_HEAD as isize)).b32.s1 = p;
                just_copy(
                    (*mem.offset((just_box + 5 as int32_t) as isize)).b32.s1,
                    p,
                    new_math(0 as scaled_t, END_L_CODE as small_number),
                );
                cur_dir = RIGHT_TO_LEFT as small_number;
            }
            v = v + 2 as scaled_t
                * (*font_info.offset(
                    (QUAD_CODE as int32_t
                        + *param_base.offset((*eqtb.offset(CUR_FONT_LOC as isize)).b32.s1 as isize))
                        as isize,
                ))
                .b32
                .s1 as scaled_t;
            if (*eqtb.offset((INT_BASE + INT_PAR__texxet) as isize)).b32.s1 > 0 as int32_t {
                temp_ptr = get_avail();
                (*mem.offset(temp_ptr as isize)).b32.s0 = BEFORE as int32_t;
                (*mem.offset(temp_ptr as isize)).b32.s1 = LR_ptr;
                LR_ptr = temp_ptr;
            }
            while p != TEX_NULL as int32_t {
                loop {
                    if is_char_node(p) {
                        f = (*mem.offset(p as isize)).b16.s1 as internal_font_number;
                        d = (*font_info.offset(
                            (*width_base.offset(f as isize)
                                + (*font_info.offset(
                                    (*char_base.offset(f as isize)
                                        + effective_char(
                                            1 as ::core::ffi::c_int != 0,
                                            f,
                                            (*mem.offset(p as isize)).b16.s0,
                                        )) as isize,
                                ))
                                .b16
                                .s3 as int32_t) as isize,
                        ))
                        .b32
                        .s1 as scaled_t;
                        current_block = 13021222023936379008;
                        break;
                    } else {
                        match (*mem.offset(p as isize)).b16.s1 as ::core::ffi::c_int {
                            0 | 1 | 2 => {
                                d = (*mem.offset((p + 1 as int32_t) as isize)).b32.s1 as scaled_t;
                                current_block = 13021222023936379008;
                                break;
                            }
                            6 => {
                                *mem.offset(GARBAGE as isize) =
                                    *mem.offset((p + 1 as int32_t) as isize);
                                (*mem.offset(GARBAGE as isize)).b32.s1 =
                                    (*mem.offset(p as isize)).b32.s1;
                                p = GARBAGE as int32_t;
                                xtx_ligature_present = true_0 != 0;
                            }
                            11 => {
                                d = (*mem.offset((p + 1 as int32_t) as isize)).b32.s1 as scaled_t;
                                current_block = 8723848109087415604;
                                break;
                            }
                            40 => {
                                d = (*mem.offset((p + 1 as int32_t) as isize)).b32.s1 as scaled_t;
                                current_block = 8723848109087415604;
                                break;
                            }
                            9 => {
                                d = (*mem.offset((p + 1 as int32_t) as isize)).b32.s1 as scaled_t;
                                if (*eqtb.offset((INT_BASE + INT_PAR__texxet) as isize)).b32.s1
                                    > 0 as int32_t
                                {
                                    current_block = 9353995356876505083;
                                    break;
                                } else {
                                    current_block = 10930818133215224067;
                                    break;
                                }
                            }
                            14 => {
                                d = (*mem.offset((p + 1 as int32_t) as isize)).b32.s1 as scaled_t;
                                cur_dir = (*mem.offset(p as isize)).b16.s0 as small_number;
                                current_block = 8723848109087415604;
                                break;
                            }
                            10 => {
                                q = (*mem.offset((p + 1 as int32_t) as isize)).b32.s0;
                                d = (*mem.offset((q + 1 as int32_t) as isize)).b32.s1 as scaled_t;
                                if (*mem.offset((just_box + 5 as int32_t) as isize)).b16.s1
                                    as ::core::ffi::c_int
                                    == STRETCHING
                                {
                                    if (*mem.offset((just_box + 5 as int32_t) as isize)).b16.s0
                                        as ::core::ffi::c_int
                                        == (*mem.offset(q as isize)).b16.s1 as ::core::ffi::c_int
                                        && (*mem.offset((q + 2 as int32_t) as isize)).b32.s1
                                            != 0 as int32_t
                                    {
                                        v = MAX_HALFWORD as scaled_t;
                                    }
                                } else if (*mem.offset((just_box + 5 as int32_t) as isize)).b16.s1
                                    as ::core::ffi::c_int
                                    == SHRINKING
                                {
                                    if (*mem.offset((just_box + 5 as int32_t) as isize)).b16.s0
                                        as ::core::ffi::c_int
                                        == (*mem.offset(q as isize)).b16.s0 as ::core::ffi::c_int
                                        && (*mem.offset((q + 3 as int32_t) as isize)).b32.s1
                                            != 0 as int32_t
                                    {
                                        v = MAX_HALFWORD as scaled_t;
                                    }
                                }
                                if (*mem.offset(p as isize)).b16.s0 as ::core::ffi::c_int
                                    >= A_LEADERS
                                {
                                    current_block = 13021222023936379008;
                                    break;
                                } else {
                                    current_block = 8723848109087415604;
                                    break;
                                }
                            }
                            8 => {
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
                                    current_block = 1852451392920375136;
                                    break;
                                } else {
                                    current_block = 12705158477165241210;
                                    break;
                                }
                            }
                            _ => {
                                d = 0 as ::core::ffi::c_int as scaled_t;
                                current_block = 8723848109087415604;
                                break;
                            }
                        }
                    }
                }
                match current_block {
                    9353995356876505083 => {
                        if (*mem.offset(p as isize)).b16.s0 as ::core::ffi::c_int
                            & 1 as ::core::ffi::c_int
                            != 0
                        {
                            if (*mem.offset(LR_ptr as isize)).b32.s0
                                == L_CODE as int32_t
                                    * ((*mem.offset(p as isize)).b16.s0 as int32_t
                                        / L_CODE as int32_t)
                                    + 3 as int32_t
                            {
                                temp_ptr = LR_ptr;
                                LR_ptr = (*mem.offset(temp_ptr as isize)).b32.s1;
                                (*mem.offset(temp_ptr as isize)).b32.s1 = avail;
                                avail = temp_ptr;
                            } else if (*mem.offset(p as isize)).b16.s0 as ::core::ffi::c_int
                                > L_CODE
                            {
                                w = MAX_HALFWORD as scaled_t;
                                break;
                            }
                        } else {
                            temp_ptr = get_avail();
                            (*mem.offset(temp_ptr as isize)).b32.s0 = (L_CODE
                                * ((*mem.offset(p as isize)).b16.s0 as ::core::ffi::c_int / L_CODE)
                                + 3 as ::core::ffi::c_int)
                                as int32_t;
                            (*mem.offset(temp_ptr as isize)).b32.s1 = LR_ptr;
                            LR_ptr = temp_ptr;
                            if (*mem.offset(p as isize)).b16.s0 as ::core::ffi::c_int / R_CODE
                                != cur_dir as ::core::ffi::c_int
                            {
                                just_reverse(p);
                                p = TEMP_HEAD as int32_t;
                            }
                        }
                        current_block = 8723848109087415604;
                    }
                    10930818133215224067 => {
                        if (*mem.offset(p as isize)).b16.s0 as ::core::ffi::c_int >= L_CODE {
                            w = MAX_HALFWORD as scaled_t;
                            break;
                        } else {
                            current_block = 8723848109087415604;
                        }
                    }
                    12705158477165241210 => {
                        d = 0 as ::core::ffi::c_int as scaled_t;
                        current_block = 8723848109087415604;
                    }
                    1852451392920375136 => {
                        d = (*mem.offset((p + 1 as int32_t) as isize)).b32.s1 as scaled_t;
                        current_block = 13021222023936379008;
                    }
                    _ => {}
                }
                match current_block {
                    8723848109087415604 => {
                        if v < MAX_HALFWORD as scaled_t {
                            v = v + d;
                        }
                    }
                    _ => {
                        if v < MAX_HALFWORD as scaled_t {
                            v = v + d;
                            w = v;
                        } else {
                            w = MAX_HALFWORD as scaled_t;
                            break;
                        }
                    }
                }
                p = (*mem.offset(p as isize)).b32.s1;
            }
            if (*eqtb.offset((INT_BASE + INT_PAR__texxet) as isize)).b32.s1 > 0 as int32_t {
                while LR_ptr != TEX_NULL as int32_t {
                    temp_ptr = LR_ptr;
                    LR_ptr = (*mem.offset(temp_ptr as isize)).b32.s1;
                    (*mem.offset(temp_ptr as isize)).b32.s1 = avail;
                    avail = temp_ptr;
                }
                if LR_problems != 0 as int32_t {
                    w = MAX_HALFWORD as scaled_t;
                    LR_problems = 0 as ::core::ffi::c_int as int32_t;
                }
            }
            cur_dir = LEFT_TO_RIGHT as small_number;
            flush_node_list((*mem.offset(TEMP_HEAD as isize)).b32.s1);
        }
        if (*eqtb.offset((LOCAL_BASE + LOCAL__par_shape) as isize))
            .b32
            .s1
            == TEX_NULL as int32_t
        {
            if (*eqtb.offset((DIMEN_BASE + DIMEN_PAR__hang_indent) as isize))
                .b32
                .s1
                != 0 as int32_t
                && ((*eqtb.offset((INT_BASE + INT_PAR__hang_after) as isize))
                    .b32
                    .s1
                    >= 0 as int32_t
                    && cur_list.prev_graf + 2 as int32_t
                        > (*eqtb.offset((INT_BASE + INT_PAR__hang_after) as isize))
                            .b32
                            .s1
                    || (cur_list.prev_graf + 1 as int32_t)
                        < -(*eqtb.offset((INT_BASE + INT_PAR__hang_after) as isize))
                            .b32
                            .s1)
            {
                l = ((*eqtb.offset((DIMEN_BASE + DIMEN_PAR__hsize) as isize))
                    .b32
                    .s1
                    - abs(
                        (*eqtb.offset((DIMEN_BASE + DIMEN_PAR__hang_indent) as isize))
                            .b32
                            .s1 as ::core::ffi::c_int,
                    ) as int32_t) as scaled_t;
                if (*eqtb.offset((DIMEN_BASE + DIMEN_PAR__hang_indent) as isize))
                    .b32
                    .s1
                    > 0 as int32_t
                {
                    s = (*eqtb.offset((DIMEN_BASE + DIMEN_PAR__hang_indent) as isize))
                        .b32
                        .s1 as scaled_t;
                } else {
                    s = 0 as ::core::ffi::c_int as scaled_t;
                }
            } else {
                l = (*eqtb.offset((DIMEN_BASE + DIMEN_PAR__hsize) as isize))
                    .b32
                    .s1 as scaled_t;
                s = 0 as ::core::ffi::c_int as scaled_t;
            }
        } else {
            n = (*mem.offset(
                (*eqtb.offset((LOCAL_BASE + LOCAL__par_shape) as isize))
                    .b32
                    .s1 as isize,
            ))
            .b32
            .s0;
            if cur_list.prev_graf + 2 as int32_t >= n {
                p = (*eqtb.offset((LOCAL_BASE + LOCAL__par_shape) as isize))
                    .b32
                    .s1
                    + 2 as int32_t * n;
            } else {
                p = (*eqtb.offset((LOCAL_BASE + LOCAL__par_shape) as isize))
                    .b32
                    .s1
                    + 2 as int32_t * (cur_list.prev_graf + 2 as int32_t);
            }
            s = (*mem.offset((p - 1 as int32_t) as isize)).b32.s1 as scaled_t;
            l = (*mem.offset(p as isize)).b32.s1 as scaled_t;
        }
        push_math(MATH_SHIFT_GROUP as group_code);
        cur_list.mode = MMODE as ::core::ffi::c_short;
        eq_word_define(
            INT_BASE as int32_t + INT_PAR__cur_fam as int32_t,
            -(1 as int32_t),
        );
        eq_word_define(
            DIMEN_BASE as int32_t + DIMEN_PAR__pre_display_size as int32_t,
            w as int32_t,
        );
        cur_list.eTeX_aux = j;
        eq_word_define(
            INT_BASE as int32_t + INT_PAR__pre_display_direction as int32_t,
            x,
        );
        eq_word_define(
            DIMEN_BASE as int32_t + DIMEN_PAR__display_width as int32_t,
            l as int32_t,
        );
        eq_word_define(
            DIMEN_BASE as int32_t + DIMEN_PAR__display_indent as int32_t,
            s as int32_t,
        );
        if semantic_pagination_enabled {
            tt_insert_special(b"tdux:cs dmath\0" as *const u8 as *const ::core::ffi::c_char);
        }
        if (*eqtb.offset((LOCAL_BASE + LOCAL__every_display) as isize))
            .b32
            .s1
            != TEX_NULL as int32_t
        {
            begin_token_list(
                (*eqtb.offset((LOCAL_BASE + LOCAL__every_display) as isize))
                    .b32
                    .s1,
                EVERY_DISPLAY_TEXT as uint16_t,
            );
        }
        if nest_ptr == 1 as int32_t {
            build_page();
        }
    } else {
        back_input();
        push_math(MATH_SHIFT_GROUP as group_code);
        eq_word_define(
            INT_BASE as int32_t + INT_PAR__cur_fam as int32_t,
            -(1 as int32_t),
        );
        if insert_src_special_every_math {
            insert_src_special();
        }
        if semantic_pagination_enabled {
            tt_insert_special(b"tdux:cs math\0" as *const u8 as *const ::core::ffi::c_char);
        }
        if (*eqtb.offset((LOCAL_BASE + LOCAL__every_math) as isize))
            .b32
            .s1
            != TEX_NULL as int32_t
        {
            begin_token_list(
                (*eqtb.offset((LOCAL_BASE + LOCAL__every_math) as isize))
                    .b32
                    .s1,
                EVERY_MATH_TEXT as uint16_t,
            );
        }
    };
}
#[no_mangle]
pub unsafe extern "C" fn start_eq_no() {
    (*save_stack.offset((save_ptr + 0 as int32_t) as isize))
        .b32
        .s1 = cur_chr;
    save_ptr += 1;
    push_math(MATH_SHIFT_GROUP as group_code);
    eq_word_define(
        INT_BASE as int32_t + INT_PAR__cur_fam as int32_t,
        -(1 as int32_t),
    );
    if insert_src_special_every_math {
        insert_src_special();
    }
    if (*eqtb.offset((LOCAL_BASE + LOCAL__every_math) as isize))
        .b32
        .s1
        != TEX_NULL as int32_t
    {
        begin_token_list(
            (*eqtb.offset((LOCAL_BASE + LOCAL__every_math) as isize))
                .b32
                .s1,
            EVERY_MATH_TEXT as uint16_t,
        );
    }
}
#[no_mangle]
pub unsafe extern "C" fn math_limit_switch() {
    if cur_list.head != cur_list.tail {
        if (*mem.offset(cur_list.tail as isize)).b16.s1 as ::core::ffi::c_int == OP_NOAD {
            (*mem.offset(cur_list.tail as isize)).b16.s0 = cur_chr as uint16_t;
            return;
        }
    }
    error_here_with_diagnostic(
        b"Limit controls must follow a math operator\0" as *const u8 as *const ::core::ffi::c_char,
    );
    capture_to_diagnostic(::core::ptr::null_mut::<ttbc_diagnostic_t>());
    help_ptr = 1 as ::core::ffi::c_uchar;
    help_line[0 as ::core::ffi::c_int as usize] =
        b"I'm ignoring this misplaced \\limits or \\nolimits command.\0" as *const u8
            as *const ::core::ffi::c_char;
    error();
}
unsafe extern "C" fn scan_delimiter(mut p: int32_t, mut r: bool) {
    if r {
        if cur_chr == 1 as int32_t {
            cur_val1 = 0x40000000 as ::core::ffi::c_int as int32_t;
            scan_math_fam_int();
            cur_val1 = (cur_val1 as ::core::ffi::c_int
                + (cur_val * 0x200000 as int32_t) as ::core::ffi::c_int)
                as int32_t;
            scan_usv_num();
            cur_val += cur_val1;
        } else {
            scan_delimiter_int();
        }
    } else {
        loop {
            get_x_token();
            if !(cur_cmd as ::core::ffi::c_int == SPACER || cur_cmd as ::core::ffi::c_int == RELAX)
            {
                break;
            }
        }
        match cur_cmd as ::core::ffi::c_int {
            LETTER | OTHER_CHAR => {
                cur_val = (*eqtb.offset((DEL_CODE_BASE as int32_t + cur_chr) as isize))
                    .b32
                    .s1;
            }
            DELIM_NUM => {
                if cur_chr == 1 as int32_t {
                    cur_val1 = 0x40000000 as ::core::ffi::c_int as int32_t;
                    scan_math_class_int();
                    scan_math_fam_int();
                    cur_val1 = (cur_val1 as ::core::ffi::c_int
                        + (cur_val * 0x20000 as int32_t) as ::core::ffi::c_int)
                        as int32_t;
                    scan_usv_num();
                    cur_val += cur_val1;
                } else {
                    scan_delimiter_int();
                }
            }
            _ => {
                cur_val = -(1 as ::core::ffi::c_int) as int32_t;
            }
        }
    }
    if cur_val < 0 as int32_t {
        if file_line_error_style_p != 0 {
            print_file_line();
        } else {
            print_nl_cstr(b"! \0" as *const u8 as *const ::core::ffi::c_char);
        }
        print_cstr(b"Missing delimiter (. inserted)\0" as *const u8 as *const ::core::ffi::c_char);
        help_ptr = 6 as ::core::ffi::c_uchar;
        help_line[5 as ::core::ffi::c_int as usize] =
            b"I was expecting to see something like `(' or `\\{' or\0" as *const u8
                as *const ::core::ffi::c_char;
        help_line[4 as ::core::ffi::c_int as usize] =
            b"`\\}' here. If you typed, e.g., `{' instead of `\\{', you\0" as *const u8
                as *const ::core::ffi::c_char;
        help_line[3 as ::core::ffi::c_int as usize] =
            b"should probably delete the `{' by typing `1' now, so that\0" as *const u8
                as *const ::core::ffi::c_char;
        help_line[2 as ::core::ffi::c_int as usize] =
            b"braces don't get unbalanced. Otherwise just proceed.\0" as *const u8
                as *const ::core::ffi::c_char;
        help_line[1 as ::core::ffi::c_int as usize] =
            b"Acceptable delimiters are characters whose \\delcode is\0" as *const u8
                as *const ::core::ffi::c_char;
        help_line[0 as ::core::ffi::c_int as usize] =
            b"nonnegative, or you can use `\\delimiter <delimiter code>'.\0" as *const u8
                as *const ::core::ffi::c_char;
        back_error();
        cur_val = 0 as ::core::ffi::c_int as int32_t;
    }
    if cur_val >= 0x40000000 as int32_t {
        (*mem.offset(p as isize)).b16.s3 =
            (cur_val % 0x200000 as int32_t / 0x10000 as int32_t * 0x100 as int32_t
                + cur_val / 0x200000 as int32_t % 0x100 as int32_t) as uint16_t;
        (*mem.offset(p as isize)).b16.s2 = (cur_val % 0x10000 as int32_t) as uint16_t;
        (*mem.offset(p as isize)).b16.s1 = 0 as uint16_t;
        (*mem.offset(p as isize)).b16.s0 = 0 as uint16_t;
    } else {
        (*mem.offset(p as isize)).b16.s3 =
            (cur_val / 0x100000 as int32_t % 16 as int32_t) as uint16_t;
        (*mem.offset(p as isize)).b16.s2 =
            (cur_val / 0x1000 as int32_t % 0x100 as int32_t) as uint16_t;
        (*mem.offset(p as isize)).b16.s1 = (cur_val / 0x100 as int32_t % 16 as int32_t) as uint16_t;
        (*mem.offset(p as isize)).b16.s0 = (cur_val % 0x100 as int32_t) as uint16_t;
    };
}
#[no_mangle]
pub unsafe extern "C" fn math_radical() {
    (*mem.offset(cur_list.tail as isize)).b32.s1 = get_node(RADICAL_NOAD_SIZE as int32_t);
    cur_list.tail = (*mem.offset(cur_list.tail as isize)).b32.s1;
    (*mem.offset(cur_list.tail as isize)).b16.s1 = RADICAL_NOAD as uint16_t;
    (*mem.offset(cur_list.tail as isize)).b16.s0 = NORMAL as uint16_t;
    (*mem.offset((cur_list.tail + 1 as int32_t) as isize)).b32 = empty;
    (*mem.offset((cur_list.tail + 3 as int32_t) as isize)).b32 = empty;
    (*mem.offset((cur_list.tail + 2 as int32_t) as isize)).b32 = empty;
    scan_delimiter(cur_list.tail + 4 as int32_t, true_0 != 0);
    scan_math(cur_list.tail + 1 as int32_t);
}
#[no_mangle]
pub unsafe extern "C" fn math_ac() {
    let mut c: int32_t = 0;
    if cur_cmd as ::core::ffi::c_int == ACCENT {
        error_here_with_diagnostic(b"Please use \0" as *const u8 as *const ::core::ffi::c_char);
        print_esc_cstr(b"mathaccent\0" as *const u8 as *const ::core::ffi::c_char);
        print_cstr(b" for accents in math mode\0" as *const u8 as *const ::core::ffi::c_char);
        capture_to_diagnostic(::core::ptr::null_mut::<ttbc_diagnostic_t>());
        help_ptr = 2 as ::core::ffi::c_uchar;
        help_line[1 as ::core::ffi::c_int as usize] =
            b"I'm changing \\accent to \\mathaccent here; wish me luck.\0" as *const u8
                as *const ::core::ffi::c_char;
        help_line[0 as ::core::ffi::c_int as usize] =
            b"(Accents are not the same in formulas as they are in text.)\0" as *const u8
                as *const ::core::ffi::c_char;
        error();
    }
    (*mem.offset(cur_list.tail as isize)).b32.s1 = get_node(ACCENT_NOAD_SIZE as int32_t);
    cur_list.tail = (*mem.offset(cur_list.tail as isize)).b32.s1;
    (*mem.offset(cur_list.tail as isize)).b16.s1 = ACCENT_NOAD as uint16_t;
    (*mem.offset(cur_list.tail as isize)).b16.s0 = NORMAL as uint16_t;
    (*mem.offset((cur_list.tail + 1 as int32_t) as isize)).b32 = empty;
    (*mem.offset((cur_list.tail + 3 as int32_t) as isize)).b32 = empty;
    (*mem.offset((cur_list.tail + 2 as int32_t) as isize)).b32 = empty;
    (*mem.offset((cur_list.tail + 4 as int32_t) as isize))
        .b32
        .s1 = MATH_CHAR as int32_t;
    if cur_chr == 1 as int32_t {
        if scan_keyword(b"fixed\0" as *const u8 as *const ::core::ffi::c_char) {
            (*mem.offset(cur_list.tail as isize)).b16.s0 = FIXED_ACC as uint16_t;
        } else if scan_keyword(b"bottom\0" as *const u8 as *const ::core::ffi::c_char) {
            if scan_keyword(b"fixed\0" as *const u8 as *const ::core::ffi::c_char) {
                (*mem.offset(cur_list.tail as isize)).b16.s0 =
                    (BOTTOM_ACC + 1 as ::core::ffi::c_int) as uint16_t;
            } else {
                (*mem.offset(cur_list.tail as isize)).b16.s0 = BOTTOM_ACC as uint16_t;
            }
        }
        scan_math_class_int();
        c = ((cur_val as ::core::ffi::c_uint & 0x7 as ::core::ffi::c_uint)
            << 21 as ::core::ffi::c_int) as int32_t;
        scan_math_fam_int();
        c = (c as ::core::ffi::c_uint).wrapping_add(
            (cur_val as ::core::ffi::c_uint & 0xff as ::core::ffi::c_uint)
                << 24 as ::core::ffi::c_int,
        ) as int32_t;
        scan_usv_num();
        cur_val = cur_val + c;
    } else {
        scan_fifteen_bit_int();
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
    (*mem.offset((cur_list.tail + 4 as int32_t) as isize))
        .b16
        .s0 = (cur_val as ::core::ffi::c_long % 65536 as ::core::ffi::c_long) as uint16_t;
    if cur_val as ::core::ffi::c_uint >> 21 as ::core::ffi::c_int & 0x7 as ::core::ffi::c_uint
        == 7 as ::core::ffi::c_uint
        && ((*eqtb.offset((INT_BASE + INT_PAR__cur_fam) as isize))
            .b32
            .s1
            >= 0 as int32_t
            && (*eqtb.offset((INT_BASE + INT_PAR__cur_fam) as isize))
                .b32
                .s1
                < NUMBER_MATH_FAMILIES as int32_t)
    {
        (*mem.offset((cur_list.tail + 4 as int32_t) as isize))
            .b16
            .s1 = (*eqtb.offset((INT_BASE + INT_PAR__cur_fam) as isize))
            .b32
            .s1 as uint16_t;
    } else {
        (*mem.offset((cur_list.tail + 4 as int32_t) as isize))
            .b16
            .s1 = (cur_val as ::core::ffi::c_uint >> 24 as ::core::ffi::c_int
            & 0xff as ::core::ffi::c_uint) as uint16_t;
    }
    (*mem.offset((cur_list.tail + 4 as int32_t) as isize))
        .b16
        .s1 = ((*mem.offset((cur_list.tail + 4 as int32_t) as isize))
        .b16
        .s1 as ::core::ffi::c_long
        + (cur_val as ::core::ffi::c_uint & 0x1fffff as ::core::ffi::c_int as ::core::ffi::c_uint)
            as ::core::ffi::c_long
            / 65536 as ::core::ffi::c_long
            * 256 as ::core::ffi::c_long) as uint16_t;
    scan_math(cur_list.tail + 1 as int32_t);
}
#[no_mangle]
pub unsafe extern "C" fn append_choices() {
    (*mem.offset(cur_list.tail as isize)).b32.s1 = new_choice();
    cur_list.tail = (*mem.offset(cur_list.tail as isize)).b32.s1;
    save_ptr += 1;
    (*save_stack.offset((save_ptr - 1 as int32_t) as isize))
        .b32
        .s1 = 0 as ::core::ffi::c_int as int32_t;
    push_math(MATH_CHOICE_GROUP as group_code);
    scan_left_brace();
}
#[no_mangle]
pub unsafe extern "C" fn fin_mlist(mut p: int32_t) -> int32_t {
    let mut q: int32_t = 0;
    if cur_list.aux.b32.s1 != TEX_NULL as int32_t {
        (*mem.offset((cur_list.aux.b32.s1 + 3 as int32_t) as isize))
            .b32
            .s1 = SUB_MLIST as int32_t;
        (*mem.offset((cur_list.aux.b32.s1 + 3 as int32_t) as isize))
            .b32
            .s0 = (*mem.offset(cur_list.head as isize)).b32.s1;
        if p == TEX_NULL as int32_t {
            q = cur_list.aux.b32.s1;
        } else {
            q = (*mem.offset((cur_list.aux.b32.s1 + 2 as int32_t) as isize))
                .b32
                .s0;
            if (*mem.offset(q as isize)).b16.s1 as ::core::ffi::c_int != LEFT_NOAD
                || cur_list.eTeX_aux == TEX_NULL as int32_t
            {
                confusion(b"right\0" as *const u8 as *const ::core::ffi::c_char);
            }
            (*mem.offset((cur_list.aux.b32.s1 + 2 as int32_t) as isize))
                .b32
                .s0 = (*mem.offset(cur_list.eTeX_aux as isize)).b32.s1;
            (*mem.offset(cur_list.eTeX_aux as isize)).b32.s1 = cur_list.aux.b32.s1;
            (*mem.offset(cur_list.aux.b32.s1 as isize)).b32.s1 = p;
        }
    } else {
        (*mem.offset(cur_list.tail as isize)).b32.s1 = p;
        q = (*mem.offset(cur_list.head as isize)).b32.s1;
    }
    pop_nest();
    return q;
}
#[no_mangle]
pub unsafe extern "C" fn build_choices() {
    let mut p: int32_t = 0;
    unsave();
    p = fin_mlist(TEX_NULL as int32_t);
    match (*save_stack.offset((save_ptr - 1 as int32_t) as isize))
        .b32
        .s1
    {
        0 => {
            (*mem.offset((cur_list.tail + 1 as int32_t) as isize))
                .b32
                .s0 = p;
        }
        1 => {
            (*mem.offset((cur_list.tail + 1 as int32_t) as isize))
                .b32
                .s1 = p;
        }
        2 => {
            (*mem.offset((cur_list.tail + 2 as int32_t) as isize))
                .b32
                .s0 = p;
        }
        3 => {
            (*mem.offset((cur_list.tail + 2 as int32_t) as isize))
                .b32
                .s1 = p;
            save_ptr -= 1;
            return;
        }
        _ => {}
    }
    let ref mut fresh6 = (*save_stack.offset((save_ptr - 1 as int32_t) as isize))
        .b32
        .s1;
    *fresh6 += 1;
    push_math(MATH_CHOICE_GROUP as group_code);
    scan_left_brace();
}
#[no_mangle]
pub unsafe extern "C" fn sub_sup() {
    let mut t: small_number = 0;
    let mut p: int32_t = 0;
    t = EMPTY as small_number;
    p = TEX_NULL as int32_t;
    if cur_list.tail != cur_list.head {
        if (*mem.offset(cur_list.tail as isize)).b16.s1 as ::core::ffi::c_int >= ORD_NOAD
            && ((*mem.offset(cur_list.tail as isize)).b16.s1 as ::core::ffi::c_int) < LEFT_NOAD
        {
            p = cur_list.tail + 2 as int32_t + cur_cmd as int32_t - 7 as int32_t;
            t = (*mem.offset(p as isize)).b32.s1 as small_number;
        }
    }
    if p == TEX_NULL as int32_t || t as ::core::ffi::c_int != EMPTY {
        (*mem.offset(cur_list.tail as isize)).b32.s1 = new_noad();
        cur_list.tail = (*mem.offset(cur_list.tail as isize)).b32.s1;
        p = cur_list.tail + 2 as int32_t + cur_cmd as int32_t - 7 as int32_t;
        if t as ::core::ffi::c_int != EMPTY {
            if cur_cmd as ::core::ffi::c_int == SUP_MARK {
                error_here_with_diagnostic(
                    b"Double superscript\0" as *const u8 as *const ::core::ffi::c_char,
                );
                capture_to_diagnostic(::core::ptr::null_mut::<ttbc_diagnostic_t>());
                help_ptr = 1 as ::core::ffi::c_uchar;
                help_line[0 as ::core::ffi::c_int as usize] =
                    b"I treat `x^1^2' essentially like `x^1{}^2'.\0" as *const u8
                        as *const ::core::ffi::c_char;
            } else {
                error_here_with_diagnostic(
                    b"Double subscript\0" as *const u8 as *const ::core::ffi::c_char,
                );
                capture_to_diagnostic(::core::ptr::null_mut::<ttbc_diagnostic_t>());
                help_ptr = 1 as ::core::ffi::c_uchar;
                help_line[0 as ::core::ffi::c_int as usize] =
                    b"I treat `x_1_2' essentially like `x_1{}_2'.\0" as *const u8
                        as *const ::core::ffi::c_char;
            }
            error();
        }
    }
    scan_math(p);
}
#[no_mangle]
pub unsafe extern "C" fn math_fraction() {
    let mut c: small_number = 0;
    c = cur_chr as small_number;
    if cur_list.aux.b32.s1 != TEX_NULL as int32_t {
        if c as ::core::ffi::c_int >= DELIMITED_CODE {
            scan_delimiter(GARBAGE as int32_t, false_0 != 0);
            scan_delimiter(GARBAGE as int32_t, false_0 != 0);
        }
        if c as ::core::ffi::c_int % DELIMITED_CODE == ABOVE_CODE {
            scan_dimen(false_0 != 0, false_0 != 0, false_0 != 0);
        }
        if file_line_error_style_p != 0 {
            print_file_line();
        } else {
            print_nl_cstr(b"! \0" as *const u8 as *const ::core::ffi::c_char);
        }
        print_cstr(
            b"Ambiguous; you need another { and }\0" as *const u8 as *const ::core::ffi::c_char,
        );
        help_ptr = 3 as ::core::ffi::c_uchar;
        help_line[2 as ::core::ffi::c_int as usize] =
            b"I'm ignoring this fraction specification, since I don't\0" as *const u8
                as *const ::core::ffi::c_char;
        help_line[1 as ::core::ffi::c_int as usize] =
            b"know whether a construction like `x \\over y \\over z'\0" as *const u8
                as *const ::core::ffi::c_char;
        help_line[0 as ::core::ffi::c_int as usize] =
            b"means `{x \\over y} \\over z' or `x \\over {y \\over z}'.\0" as *const u8
                as *const ::core::ffi::c_char;
        error();
    } else {
        cur_list.aux.b32.s1 = get_node(FRACTION_NOAD_SIZE as int32_t);
        (*mem.offset(cur_list.aux.b32.s1 as isize)).b16.s1 = FRACTION_NOAD as uint16_t;
        (*mem.offset(cur_list.aux.b32.s1 as isize)).b16.s0 = NORMAL as uint16_t;
        (*mem.offset((cur_list.aux.b32.s1 + 2 as int32_t) as isize))
            .b32
            .s1 = SUB_MLIST as int32_t;
        (*mem.offset((cur_list.aux.b32.s1 + 2 as int32_t) as isize))
            .b32
            .s0 = (*mem.offset(cur_list.head as isize)).b32.s1;
        (*mem.offset((cur_list.aux.b32.s1 + 3 as int32_t) as isize)).b32 = empty;
        (*mem.offset((cur_list.aux.b32.s1 + 4 as int32_t) as isize)).b16 = null_delimiter;
        (*mem.offset((cur_list.aux.b32.s1 + 5 as int32_t) as isize)).b16 = null_delimiter;
        (*mem.offset(cur_list.head as isize)).b32.s1 = TEX_NULL as int32_t;
        cur_list.tail = cur_list.head;
        if c as ::core::ffi::c_int >= DELIMITED_CODE {
            scan_delimiter(cur_list.aux.b32.s1 + 4 as int32_t, false_0 != 0);
            scan_delimiter(cur_list.aux.b32.s1 + 5 as int32_t, false_0 != 0);
        }
        match c as ::core::ffi::c_int % DELIMITED_CODE {
            ABOVE_CODE => {
                scan_dimen(false_0 != 0, false_0 != 0, false_0 != 0);
                (*mem.offset((cur_list.aux.b32.s1 + 1 as int32_t) as isize))
                    .b32
                    .s1 = cur_val;
            }
            OVER_CODE => {
                (*mem.offset((cur_list.aux.b32.s1 + 1 as int32_t) as isize))
                    .b32
                    .s1 = DEFAULT_CODE as int32_t;
            }
            ATOP_CODE => {
                (*mem.offset((cur_list.aux.b32.s1 + 1 as int32_t) as isize))
                    .b32
                    .s1 = 0 as ::core::ffi::c_int as int32_t;
            }
            _ => {}
        }
    };
}
#[no_mangle]
pub unsafe extern "C" fn math_left_right() {
    let mut t: small_number = 0;
    let mut p: int32_t = 0;
    let mut q: int32_t = 0;
    t = cur_chr as small_number;
    if t as ::core::ffi::c_int != LEFT_NOAD && cur_group as ::core::ffi::c_int != MATH_LEFT_GROUP {
        if cur_group as ::core::ffi::c_int == MATH_SHIFT_GROUP {
            scan_delimiter(GARBAGE as int32_t, false_0 != 0);
            error_here_with_diagnostic(b"Extra \0" as *const u8 as *const ::core::ffi::c_char);
            if t as ::core::ffi::c_int == 1 as ::core::ffi::c_int {
                print_esc_cstr(b"middle\0" as *const u8 as *const ::core::ffi::c_char);
                help_ptr = 1 as ::core::ffi::c_uchar;
                help_line[0 as ::core::ffi::c_int as usize] =
                    b"I'm ignoring a \\middle that had no matching \\left.\0" as *const u8
                        as *const ::core::ffi::c_char;
            } else {
                print_esc_cstr(b"right\0" as *const u8 as *const ::core::ffi::c_char);
                help_ptr = 1 as ::core::ffi::c_uchar;
                help_line[0 as ::core::ffi::c_int as usize] =
                    b"I'm ignoring a \\right that had no matching \\left.\0" as *const u8
                        as *const ::core::ffi::c_char;
            }
            capture_to_diagnostic(::core::ptr::null_mut::<ttbc_diagnostic_t>());
            error();
        } else {
            off_save();
        }
    } else {
        p = new_noad();
        (*mem.offset(p as isize)).b16.s1 = t as uint16_t;
        scan_delimiter(p + 1 as int32_t, false_0 != 0);
        if t as ::core::ffi::c_int == 1 as ::core::ffi::c_int {
            (*mem.offset(p as isize)).b16.s1 = RIGHT_NOAD as uint16_t;
            (*mem.offset(p as isize)).b16.s0 = 1 as uint16_t;
        }
        if t as ::core::ffi::c_int == LEFT_NOAD {
            q = p;
        } else {
            q = fin_mlist(p);
            unsave();
        }
        if t as ::core::ffi::c_int != RIGHT_NOAD {
            push_math(MATH_LEFT_GROUP as group_code);
            (*mem.offset(cur_list.head as isize)).b32.s1 = q;
            cur_list.tail = p;
            cur_list.eTeX_aux = p;
        } else {
            (*mem.offset(cur_list.tail as isize)).b32.s1 = new_noad();
            cur_list.tail = (*mem.offset(cur_list.tail as isize)).b32.s1;
            (*mem.offset(cur_list.tail as isize)).b16.s1 = INNER_NOAD as uint16_t;
            (*mem.offset((cur_list.tail + 1 as int32_t) as isize))
                .b32
                .s1 = SUB_MLIST as int32_t;
            (*mem.offset((cur_list.tail + 1 as int32_t) as isize))
                .b32
                .s0 = q;
        }
    };
}
unsafe extern "C" fn app_display(mut j: int32_t, mut b: int32_t, mut d: scaled_t) {
    let mut z: scaled_t = 0;
    let mut s: scaled_t = 0;
    let mut e: scaled_t = 0;
    let mut x: int32_t = 0;
    let mut p: int32_t = 0;
    let mut q: int32_t = 0;
    let mut r: int32_t = 0;
    let mut t: int32_t = 0;
    let mut u: int32_t = 0;
    s = (*eqtb.offset((DIMEN_BASE + DIMEN_PAR__display_indent) as isize))
        .b32
        .s1 as scaled_t;
    x = (*eqtb.offset((INT_BASE + INT_PAR__pre_display_direction) as isize))
        .b32
        .s1;
    if x == 0 as int32_t {
        (*mem.offset((b + 4 as int32_t) as isize)).b32.s1 = (s + d) as int32_t;
    } else {
        z = (*eqtb.offset((DIMEN_BASE + DIMEN_PAR__display_width) as isize))
            .b32
            .s1 as scaled_t;
        p = b;
        if x > 0 as int32_t {
            e = z - d - (*mem.offset((p + 1 as int32_t) as isize)).b32.s1 as scaled_t;
        } else {
            e = d;
            d = z - e - (*mem.offset((p + 1 as int32_t) as isize)).b32.s1 as scaled_t;
        }
        if j != TEX_NULL as int32_t {
            b = copy_node_list(j);
            (*mem.offset((b + 3 as int32_t) as isize)).b32.s1 =
                (*mem.offset((p + 3 as int32_t) as isize)).b32.s1;
            (*mem.offset((b + 2 as int32_t) as isize)).b32.s1 =
                (*mem.offset((p + 2 as int32_t) as isize)).b32.s1;
            s = s - (*mem.offset((b + 4 as int32_t) as isize)).b32.s1 as scaled_t;
            d = d + s;
            e = e + (*mem.offset((b + 1 as int32_t) as isize)).b32.s1 as scaled_t - z - s;
        }
        if (*mem.offset(p as isize)).b16.s0 as ::core::ffi::c_int == DLIST {
            q = p;
        } else {
            r = (*mem.offset((p + 5 as int32_t) as isize)).b32.s1;
            free_node(p, BOX_NODE_SIZE as int32_t);
            if r == TEX_NULL as int32_t {
                confusion(b"LR4\0" as *const u8 as *const ::core::ffi::c_char);
            }
            if x > 0 as int32_t {
                p = r;
                loop {
                    q = r;
                    r = (*mem.offset(r as isize)).b32.s1;
                    if r == TEX_NULL as int32_t {
                        break;
                    }
                }
            } else {
                p = TEX_NULL as int32_t;
                q = r;
                loop {
                    t = (*mem.offset(r as isize)).b32.s1;
                    (*mem.offset(r as isize)).b32.s1 = p;
                    p = r;
                    r = t;
                    if r == TEX_NULL as int32_t {
                        break;
                    }
                }
            }
        }
        if j == TEX_NULL as int32_t {
            r = new_kern(0 as scaled_t);
            t = new_kern(0 as scaled_t);
        } else {
            r = (*mem.offset((b + 5 as int32_t) as isize)).b32.s1;
            t = (*mem.offset(r as isize)).b32.s1;
        }
        u = new_math(0 as scaled_t, END_M_CODE as small_number);
        if (*mem.offset(t as isize)).b16.s1 as ::core::ffi::c_int == GLUE_NODE {
            j = new_skip_param(GLUE_PAR__right_skip as small_number);
            (*mem.offset(q as isize)).b32.s1 = j;
            (*mem.offset(j as isize)).b32.s1 = u;
            j = (*mem.offset((t + 1 as int32_t) as isize)).b32.s0;
            (*mem.offset(temp_ptr as isize)).b16.s1 = (*mem.offset(j as isize)).b16.s1;
            (*mem.offset(temp_ptr as isize)).b16.s0 = (*mem.offset(j as isize)).b16.s0;
            (*mem.offset((temp_ptr + 1 as int32_t) as isize)).b32.s1 =
                (e - (*mem.offset((j + 1 as int32_t) as isize)).b32.s1 as scaled_t) as int32_t;
            (*mem.offset((temp_ptr + 2 as int32_t) as isize)).b32.s1 =
                -(*mem.offset((j + 2 as int32_t) as isize)).b32.s1;
            (*mem.offset((temp_ptr + 3 as int32_t) as isize)).b32.s1 =
                -(*mem.offset((j + 3 as int32_t) as isize)).b32.s1;
            (*mem.offset(u as isize)).b32.s1 = t;
        } else {
            (*mem.offset((t + 1 as int32_t) as isize)).b32.s1 = e as int32_t;
            (*mem.offset(t as isize)).b32.s1 = u;
            (*mem.offset(q as isize)).b32.s1 = t;
        }
        u = new_math(0 as scaled_t, BEGIN_M_CODE as small_number);
        if (*mem.offset(r as isize)).b16.s1 as ::core::ffi::c_int == GLUE_NODE {
            j = new_skip_param(GLUE_PAR__left_skip as small_number);
            (*mem.offset(u as isize)).b32.s1 = j;
            (*mem.offset(j as isize)).b32.s1 = p;
            j = (*mem.offset((r + 1 as int32_t) as isize)).b32.s0;
            (*mem.offset(temp_ptr as isize)).b16.s1 = (*mem.offset(j as isize)).b16.s1;
            (*mem.offset(temp_ptr as isize)).b16.s0 = (*mem.offset(j as isize)).b16.s0;
            (*mem.offset((temp_ptr + 1 as int32_t) as isize)).b32.s1 =
                (d - (*mem.offset((j + 1 as int32_t) as isize)).b32.s1 as scaled_t) as int32_t;
            (*mem.offset((temp_ptr + 2 as int32_t) as isize)).b32.s1 =
                -(*mem.offset((j + 2 as int32_t) as isize)).b32.s1;
            (*mem.offset((temp_ptr + 3 as int32_t) as isize)).b32.s1 =
                -(*mem.offset((j + 3 as int32_t) as isize)).b32.s1;
            (*mem.offset(r as isize)).b32.s1 = u;
        } else {
            (*mem.offset((r + 1 as int32_t) as isize)).b32.s1 = d as int32_t;
            (*mem.offset(r as isize)).b32.s1 = p;
            (*mem.offset(u as isize)).b32.s1 = r;
            if j == TEX_NULL as int32_t {
                b = hpack(u, 0 as scaled_t, ADDITIONAL as small_number);
                (*mem.offset((b + 4 as int32_t) as isize)).b32.s1 = s as int32_t;
            } else {
                (*mem.offset((b + 5 as int32_t) as isize)).b32.s1 = u;
            }
        }
    }
    append_to_vlist(b);
}
#[no_mangle]
pub unsafe extern "C" fn after_math() {
    let mut l: bool = false;
    let mut danger: bool = false;
    let mut m: int32_t = 0;
    let mut p: int32_t = 0;
    let mut a: int32_t = 0;
    let mut b: int32_t = 0;
    let mut w: scaled_t = 0;
    let mut z: scaled_t = 0;
    let mut e: scaled_t = 0;
    let mut q: scaled_t = 0;
    let mut d: scaled_t = 0;
    let mut s: scaled_t = 0;
    let mut g1: small_number = 0;
    let mut g2: small_number = 0;
    let mut r: int32_t = 0;
    let mut t: int32_t = 0;
    let mut pre_t: int32_t = 0;
    let mut j: int32_t = TEX_NULL as int32_t;
    danger = false_0 != 0;
    if cur_list.mode as ::core::ffi::c_int == MMODE {
        j = cur_list.eTeX_aux;
    }
    if *font_params.offset(
        (*eqtb.offset((MATH_FONT_BASE + 2 as ::core::ffi::c_int) as isize))
            .b32
            .s1 as isize,
    ) < TOTAL_MATHSY_PARAMS as font_index
        && !(*font_area.offset(
            (*eqtb.offset((MATH_FONT_BASE + 2 as ::core::ffi::c_int) as isize))
                .b32
                .s1 as isize,
        ) as ::core::ffi::c_uint
            == OTGR_FONT_FLAG
            && isOpenTypeMathFont(
                *font_layout_engine.offset(
                    (*eqtb.offset((MATH_FONT_BASE + 2 as ::core::ffi::c_int) as isize))
                        .b32
                        .s1 as isize,
                ) as XeTeXLayoutEngine,
            ) as ::core::ffi::c_int
                != 0)
        || *font_params.offset(
            (*eqtb.offset(
                (MATH_FONT_BASE + (2 as ::core::ffi::c_int + 256 as ::core::ffi::c_int)) as isize,
            ))
            .b32
            .s1 as isize,
        ) < TOTAL_MATHSY_PARAMS as font_index
            && !(*font_area.offset(
                (*eqtb.offset(
                    (MATH_FONT_BASE + (2 as ::core::ffi::c_int + 256 as ::core::ffi::c_int))
                        as isize,
                ))
                .b32
                .s1 as isize,
            ) as ::core::ffi::c_uint
                == OTGR_FONT_FLAG
                && isOpenTypeMathFont(
                    *font_layout_engine.offset(
                        (*eqtb.offset(
                            (MATH_FONT_BASE + (2 as ::core::ffi::c_int + 256 as ::core::ffi::c_int))
                                as isize,
                        ))
                        .b32
                        .s1 as isize,
                    ) as XeTeXLayoutEngine,
                ) as ::core::ffi::c_int
                    != 0)
        || *font_params.offset(
            (*eqtb.offset(
                (MATH_FONT_BASE + (2 as ::core::ffi::c_int + 512 as ::core::ffi::c_int)) as isize,
            ))
            .b32
            .s1 as isize,
        ) < TOTAL_MATHSY_PARAMS as font_index
            && !(*font_area.offset(
                (*eqtb.offset(
                    (MATH_FONT_BASE + (2 as ::core::ffi::c_int + 512 as ::core::ffi::c_int))
                        as isize,
                ))
                .b32
                .s1 as isize,
            ) as ::core::ffi::c_uint
                == OTGR_FONT_FLAG
                && isOpenTypeMathFont(
                    *font_layout_engine.offset(
                        (*eqtb.offset(
                            (MATH_FONT_BASE + (2 as ::core::ffi::c_int + 512 as ::core::ffi::c_int))
                                as isize,
                        ))
                        .b32
                        .s1 as isize,
                    ) as XeTeXLayoutEngine,
                ) as ::core::ffi::c_int
                    != 0)
    {
        error_here_with_diagnostic(
            b"Math formula deleted: Insufficient symbol fonts\0" as *const u8
                as *const ::core::ffi::c_char,
        );
        capture_to_diagnostic(::core::ptr::null_mut::<ttbc_diagnostic_t>());
        help_ptr = 3 as ::core::ffi::c_uchar;
        help_line[2 as ::core::ffi::c_int as usize] =
            b"Sorry, but I can't typeset math unless \\textfont 2\0" as *const u8
                as *const ::core::ffi::c_char;
        help_line[1 as ::core::ffi::c_int as usize] =
            b"and \\scriptfont 2 and \\scriptscriptfont 2 have all\0" as *const u8
                as *const ::core::ffi::c_char;
        help_line[0 as ::core::ffi::c_int as usize] =
            b"the \\fontdimen values needed in math symbol fonts.\0" as *const u8
                as *const ::core::ffi::c_char;
        error();
        flush_math();
        danger = true_0 != 0;
    } else if *font_params.offset(
        (*eqtb.offset(
            (MATH_FONT_BASE + (3 as ::core::ffi::c_int + 0 as ::core::ffi::c_int)) as isize,
        ))
        .b32
        .s1 as isize,
    ) < TOTAL_MATHEX_PARAMS as font_index
        && !(*font_area.offset(
            (*eqtb.offset(
                (MATH_FONT_BASE + (3 as ::core::ffi::c_int + 0 as ::core::ffi::c_int)) as isize,
            ))
            .b32
            .s1 as isize,
        ) as ::core::ffi::c_uint
            == OTGR_FONT_FLAG
            && isOpenTypeMathFont(
                *font_layout_engine.offset(
                    (*eqtb.offset(
                        (MATH_FONT_BASE + (3 as ::core::ffi::c_int + 0 as ::core::ffi::c_int))
                            as isize,
                    ))
                    .b32
                    .s1 as isize,
                ) as XeTeXLayoutEngine,
            ) as ::core::ffi::c_int
                != 0)
        || *font_params.offset(
            (*eqtb.offset(
                (MATH_FONT_BASE + (3 as ::core::ffi::c_int + 256 as ::core::ffi::c_int)) as isize,
            ))
            .b32
            .s1 as isize,
        ) < TOTAL_MATHEX_PARAMS as font_index
            && !(*font_area.offset(
                (*eqtb.offset(
                    (MATH_FONT_BASE + (3 as ::core::ffi::c_int + 256 as ::core::ffi::c_int))
                        as isize,
                ))
                .b32
                .s1 as isize,
            ) as ::core::ffi::c_uint
                == OTGR_FONT_FLAG
                && isOpenTypeMathFont(
                    *font_layout_engine.offset(
                        (*eqtb.offset(
                            (MATH_FONT_BASE + (3 as ::core::ffi::c_int + 256 as ::core::ffi::c_int))
                                as isize,
                        ))
                        .b32
                        .s1 as isize,
                    ) as XeTeXLayoutEngine,
                ) as ::core::ffi::c_int
                    != 0)
        || *font_params.offset(
            (*eqtb.offset(
                (MATH_FONT_BASE + (3 as ::core::ffi::c_int + 512 as ::core::ffi::c_int)) as isize,
            ))
            .b32
            .s1 as isize,
        ) < TOTAL_MATHEX_PARAMS as font_index
            && !(*font_area.offset(
                (*eqtb.offset(
                    (MATH_FONT_BASE + (3 as ::core::ffi::c_int + 512 as ::core::ffi::c_int))
                        as isize,
                ))
                .b32
                .s1 as isize,
            ) as ::core::ffi::c_uint
                == OTGR_FONT_FLAG
                && isOpenTypeMathFont(
                    *font_layout_engine.offset(
                        (*eqtb.offset(
                            (MATH_FONT_BASE + (3 as ::core::ffi::c_int + 512 as ::core::ffi::c_int))
                                as isize,
                        ))
                        .b32
                        .s1 as isize,
                    ) as XeTeXLayoutEngine,
                ) as ::core::ffi::c_int
                    != 0)
    {
        error_here_with_diagnostic(
            b"Math formula deleted: Insufficient extension fonts\0" as *const u8
                as *const ::core::ffi::c_char,
        );
        capture_to_diagnostic(::core::ptr::null_mut::<ttbc_diagnostic_t>());
        help_ptr = 3 as ::core::ffi::c_uchar;
        help_line[2 as ::core::ffi::c_int as usize] =
            b"Sorry, but I can't typeset math unless \\textfont 3\0" as *const u8
                as *const ::core::ffi::c_char;
        help_line[1 as ::core::ffi::c_int as usize] =
            b"and \\scriptfont 3 and \\scriptscriptfont 3 have all\0" as *const u8
                as *const ::core::ffi::c_char;
        help_line[0 as ::core::ffi::c_int as usize] =
            b"the \\fontdimen values needed in math extension fonts.\0" as *const u8
                as *const ::core::ffi::c_char;
        error();
        flush_math();
        danger = true_0 != 0;
    }
    m = cur_list.mode as int32_t;
    l = false_0 != 0;
    p = fin_mlist(TEX_NULL as int32_t);
    if cur_list.mode as int32_t == -m {
        get_x_token();
        if cur_cmd as ::core::ffi::c_int != MATH_SHIFT {
            error_here_with_diagnostic(
                b"Display math should end with $$\0" as *const u8 as *const ::core::ffi::c_char,
            );
            capture_to_diagnostic(::core::ptr::null_mut::<ttbc_diagnostic_t>());
            help_ptr = 2 as ::core::ffi::c_uchar;
            help_line[1 as ::core::ffi::c_int as usize] =
                b"The `$' that I just saw supposedly matches a previous `$$'.\0" as *const u8
                    as *const ::core::ffi::c_char;
            help_line[0 as ::core::ffi::c_int as usize] =
                b"So I shall assume that you typed `$$' both times.\0" as *const u8
                    as *const ::core::ffi::c_char;
            back_error();
        }
        cur_mlist = p;
        cur_style = TEXT_STYLE as small_number;
        mlist_penalties = false_0 != 0;
        mlist_to_hlist();
        a = hpack(
            (*mem.offset(TEMP_HEAD as isize)).b32.s1,
            0 as scaled_t,
            ADDITIONAL as small_number,
        );
        (*mem.offset(a as isize)).b16.s0 = DLIST as uint16_t;
        unsave();
        save_ptr -= 1;
        if (*save_stack.offset((save_ptr + 0 as int32_t) as isize))
            .b32
            .s1
            == 1 as int32_t
        {
            l = true_0 != 0;
        }
        danger = false_0 != 0;
        if cur_list.mode as ::core::ffi::c_int == MMODE {
            j = cur_list.eTeX_aux;
        }
        if *font_params.offset(
            (*eqtb.offset((MATH_FONT_BASE + 2 as ::core::ffi::c_int) as isize))
                .b32
                .s1 as isize,
        ) < TOTAL_MATHSY_PARAMS as font_index
            && !(*font_area.offset(
                (*eqtb.offset((MATH_FONT_BASE + 2 as ::core::ffi::c_int) as isize))
                    .b32
                    .s1 as isize,
            ) as ::core::ffi::c_uint
                == OTGR_FONT_FLAG
                && isOpenTypeMathFont(
                    *font_layout_engine.offset(
                        (*eqtb.offset((MATH_FONT_BASE + 2 as ::core::ffi::c_int) as isize))
                            .b32
                            .s1 as isize,
                    ) as XeTeXLayoutEngine,
                ) as ::core::ffi::c_int
                    != 0)
            || *font_params.offset(
                (*eqtb.offset(
                    (MATH_FONT_BASE + (2 as ::core::ffi::c_int + 256 as ::core::ffi::c_int))
                        as isize,
                ))
                .b32
                .s1 as isize,
            ) < TOTAL_MATHSY_PARAMS as font_index
                && !(*font_area.offset(
                    (*eqtb.offset(
                        (MATH_FONT_BASE + (2 as ::core::ffi::c_int + 256 as ::core::ffi::c_int))
                            as isize,
                    ))
                    .b32
                    .s1 as isize,
                ) as ::core::ffi::c_uint
                    == OTGR_FONT_FLAG
                    && isOpenTypeMathFont(
                        *font_layout_engine.offset(
                            (*eqtb.offset(
                                (MATH_FONT_BASE
                                    + (2 as ::core::ffi::c_int + 256 as ::core::ffi::c_int))
                                    as isize,
                            ))
                            .b32
                            .s1 as isize,
                        ) as XeTeXLayoutEngine,
                    ) as ::core::ffi::c_int
                        != 0)
            || *font_params.offset(
                (*eqtb.offset(
                    (MATH_FONT_BASE + (2 as ::core::ffi::c_int + 512 as ::core::ffi::c_int))
                        as isize,
                ))
                .b32
                .s1 as isize,
            ) < TOTAL_MATHSY_PARAMS as font_index
                && !(*font_area.offset(
                    (*eqtb.offset(
                        (MATH_FONT_BASE + (2 as ::core::ffi::c_int + 512 as ::core::ffi::c_int))
                            as isize,
                    ))
                    .b32
                    .s1 as isize,
                ) as ::core::ffi::c_uint
                    == OTGR_FONT_FLAG
                    && isOpenTypeMathFont(
                        *font_layout_engine.offset(
                            (*eqtb.offset(
                                (MATH_FONT_BASE
                                    + (2 as ::core::ffi::c_int + 512 as ::core::ffi::c_int))
                                    as isize,
                            ))
                            .b32
                            .s1 as isize,
                        ) as XeTeXLayoutEngine,
                    ) as ::core::ffi::c_int
                        != 0)
        {
            error_here_with_diagnostic(
                b"Math formula deleted: Insufficient symbol fonts\0" as *const u8
                    as *const ::core::ffi::c_char,
            );
            capture_to_diagnostic(::core::ptr::null_mut::<ttbc_diagnostic_t>());
            help_ptr = 3 as ::core::ffi::c_uchar;
            help_line[2 as ::core::ffi::c_int as usize] =
                b"Sorry, but I can't typeset math unless \\textfont 2\0" as *const u8
                    as *const ::core::ffi::c_char;
            help_line[1 as ::core::ffi::c_int as usize] =
                b"and \\scriptfont 2 and \\scriptscriptfont 2 have all\0" as *const u8
                    as *const ::core::ffi::c_char;
            help_line[0 as ::core::ffi::c_int as usize] =
                b"the \\fontdimen values needed in math symbol fonts.\0" as *const u8
                    as *const ::core::ffi::c_char;
            error();
            flush_math();
            danger = true_0 != 0;
        } else if *font_params.offset(
            (*eqtb.offset(
                (MATH_FONT_BASE + (3 as ::core::ffi::c_int + 0 as ::core::ffi::c_int)) as isize,
            ))
            .b32
            .s1 as isize,
        ) < TOTAL_MATHEX_PARAMS as font_index
            && !(*font_area.offset(
                (*eqtb.offset(
                    (MATH_FONT_BASE + (3 as ::core::ffi::c_int + 0 as ::core::ffi::c_int)) as isize,
                ))
                .b32
                .s1 as isize,
            ) as ::core::ffi::c_uint
                == OTGR_FONT_FLAG
                && isOpenTypeMathFont(
                    *font_layout_engine.offset(
                        (*eqtb.offset(
                            (MATH_FONT_BASE + (3 as ::core::ffi::c_int + 0 as ::core::ffi::c_int))
                                as isize,
                        ))
                        .b32
                        .s1 as isize,
                    ) as XeTeXLayoutEngine,
                ) as ::core::ffi::c_int
                    != 0)
            || *font_params.offset(
                (*eqtb.offset(
                    (MATH_FONT_BASE + (3 as ::core::ffi::c_int + 256 as ::core::ffi::c_int))
                        as isize,
                ))
                .b32
                .s1 as isize,
            ) < TOTAL_MATHEX_PARAMS as font_index
                && !(*font_area.offset(
                    (*eqtb.offset(
                        (MATH_FONT_BASE + (3 as ::core::ffi::c_int + 256 as ::core::ffi::c_int))
                            as isize,
                    ))
                    .b32
                    .s1 as isize,
                ) as ::core::ffi::c_uint
                    == OTGR_FONT_FLAG
                    && isOpenTypeMathFont(
                        *font_layout_engine.offset(
                            (*eqtb.offset(
                                (MATH_FONT_BASE
                                    + (3 as ::core::ffi::c_int + 256 as ::core::ffi::c_int))
                                    as isize,
                            ))
                            .b32
                            .s1 as isize,
                        ) as XeTeXLayoutEngine,
                    ) as ::core::ffi::c_int
                        != 0)
            || *font_params.offset(
                (*eqtb.offset(
                    (MATH_FONT_BASE + (3 as ::core::ffi::c_int + 512 as ::core::ffi::c_int))
                        as isize,
                ))
                .b32
                .s1 as isize,
            ) < TOTAL_MATHEX_PARAMS as font_index
                && !(*font_area.offset(
                    (*eqtb.offset(
                        (MATH_FONT_BASE + (3 as ::core::ffi::c_int + 512 as ::core::ffi::c_int))
                            as isize,
                    ))
                    .b32
                    .s1 as isize,
                ) as ::core::ffi::c_uint
                    == OTGR_FONT_FLAG
                    && isOpenTypeMathFont(
                        *font_layout_engine.offset(
                            (*eqtb.offset(
                                (MATH_FONT_BASE
                                    + (3 as ::core::ffi::c_int + 512 as ::core::ffi::c_int))
                                    as isize,
                            ))
                            .b32
                            .s1 as isize,
                        ) as XeTeXLayoutEngine,
                    ) as ::core::ffi::c_int
                        != 0)
        {
            error_here_with_diagnostic(
                b"Math formula deleted: Insufficient extension fonts\0" as *const u8
                    as *const ::core::ffi::c_char,
            );
            capture_to_diagnostic(::core::ptr::null_mut::<ttbc_diagnostic_t>());
            help_ptr = 3 as ::core::ffi::c_uchar;
            help_line[2 as ::core::ffi::c_int as usize] =
                b"Sorry, but I can't typeset math unless \\textfont 3\0" as *const u8
                    as *const ::core::ffi::c_char;
            help_line[1 as ::core::ffi::c_int as usize] =
                b"and \\scriptfont 3 and \\scriptscriptfont 3 have all\0" as *const u8
                    as *const ::core::ffi::c_char;
            help_line[0 as ::core::ffi::c_int as usize] =
                b"the \\fontdimen values needed in math extension fonts.\0" as *const u8
                    as *const ::core::ffi::c_char;
            error();
            flush_math();
            danger = true_0 != 0;
        }
        m = cur_list.mode as int32_t;
        p = fin_mlist(TEX_NULL as int32_t);
    } else {
        a = TEX_NULL as int32_t;
    }
    if m < 0 as int32_t {
        (*mem.offset(cur_list.tail as isize)).b32.s1 = new_math(
            (*eqtb.offset((DIMEN_BASE + DIMEN_PAR__math_surround) as isize))
                .b32
                .s1 as scaled_t,
            BEFORE as small_number,
        );
        cur_list.tail = (*mem.offset(cur_list.tail as isize)).b32.s1;
        cur_mlist = p;
        cur_style = TEXT_STYLE as small_number;
        mlist_penalties = cur_list.mode as ::core::ffi::c_int > 0 as ::core::ffi::c_int;
        mlist_to_hlist();
        (*mem.offset(cur_list.tail as isize)).b32.s1 = (*mem.offset(TEMP_HEAD as isize)).b32.s1;
        while (*mem.offset(cur_list.tail as isize)).b32.s1 != TEX_NULL as int32_t {
            cur_list.tail = (*mem.offset(cur_list.tail as isize)).b32.s1;
        }
        (*mem.offset(cur_list.tail as isize)).b32.s1 = new_math(
            (*eqtb.offset((DIMEN_BASE + DIMEN_PAR__math_surround) as isize))
                .b32
                .s1 as scaled_t,
            AFTER as small_number,
        );
        cur_list.tail = (*mem.offset(cur_list.tail as isize)).b32.s1;
        cur_list.aux.b32.s0 = 1000 as ::core::ffi::c_int as int32_t;
        if semantic_pagination_enabled {
            tt_insert_special(b"tdux:ce math\0" as *const u8 as *const ::core::ffi::c_char);
        }
        unsave();
    } else {
        if a == TEX_NULL as int32_t {
            get_x_token();
            if cur_cmd as ::core::ffi::c_int != MATH_SHIFT {
                error_here_with_diagnostic(
                    b"Display math should end with $$\0" as *const u8 as *const ::core::ffi::c_char,
                );
                capture_to_diagnostic(::core::ptr::null_mut::<ttbc_diagnostic_t>());
                help_ptr = 2 as ::core::ffi::c_uchar;
                help_line[1 as ::core::ffi::c_int as usize] =
                    b"The `$' that I just saw supposedly matches a previous `$$'.\0" as *const u8
                        as *const ::core::ffi::c_char;
                help_line[0 as ::core::ffi::c_int as usize] =
                    b"So I shall assume that you typed `$$' both times.\0" as *const u8
                        as *const ::core::ffi::c_char;
                back_error();
            }
        }
        cur_mlist = p;
        cur_style = DISPLAY_STYLE as small_number;
        mlist_penalties = false_0 != 0;
        mlist_to_hlist();
        p = (*mem.offset(TEMP_HEAD as isize)).b32.s1;
        adjust_tail = ADJUST_HEAD as int32_t;
        pre_adjust_tail = PRE_ADJUST_HEAD as int32_t;
        b = hpack(p, 0 as scaled_t, ADDITIONAL as small_number);
        p = (*mem.offset((b + 5 as int32_t) as isize)).b32.s1;
        t = adjust_tail;
        adjust_tail = TEX_NULL as int32_t;
        pre_t = pre_adjust_tail;
        pre_adjust_tail = TEX_NULL as int32_t;
        w = (*mem.offset((b + 1 as int32_t) as isize)).b32.s1 as scaled_t;
        z = (*eqtb.offset((DIMEN_BASE + DIMEN_PAR__display_width) as isize))
            .b32
            .s1 as scaled_t;
        s = (*eqtb.offset((DIMEN_BASE + DIMEN_PAR__display_indent) as isize))
            .b32
            .s1 as scaled_t;
        if (*eqtb.offset((INT_BASE + INT_PAR__pre_display_direction) as isize))
            .b32
            .s1
            < 0 as int32_t
        {
            s = (-s - z as int32_t) as scaled_t;
        }
        if a == TEX_NULL as int32_t || danger as ::core::ffi::c_int != 0 {
            e = 0 as ::core::ffi::c_int as scaled_t;
            q = 0 as ::core::ffi::c_int as scaled_t;
        } else {
            e = (*mem.offset((a + 1 as int32_t) as isize)).b32.s1 as scaled_t;
            q = e + math_quad(TEXT_SIZE as int32_t);
        }
        if w + q > z {
            if e != 0 as scaled_t
                && (w - total_shrink[NORMAL as usize] + q <= z
                    || total_shrink[FIL as usize] != 0 as scaled_t
                    || total_shrink[FILL as usize] != 0 as scaled_t
                    || total_shrink[FILLL as usize] != 0 as scaled_t)
            {
                free_node(b, BOX_NODE_SIZE as int32_t);
                b = hpack(p, z - q, EXACTLY as small_number);
            } else {
                e = 0 as ::core::ffi::c_int as scaled_t;
                if w > z {
                    free_node(b, BOX_NODE_SIZE as int32_t);
                    b = hpack(p, z, EXACTLY as small_number);
                }
            }
            w = (*mem.offset((b + 1 as int32_t) as isize)).b32.s1 as scaled_t;
        }
        (*mem.offset(b as isize)).b16.s0 = DLIST as uint16_t;
        d = half(z as int32_t - w as int32_t) as scaled_t;
        if e > 0 as scaled_t && d < 2 as scaled_t * e {
            d = half(z as int32_t - w as int32_t - e as int32_t) as scaled_t;
            if p != TEX_NULL as int32_t {
                if !is_char_node(p) {
                    if (*mem.offset(p as isize)).b16.s1 as ::core::ffi::c_int == GLUE_NODE {
                        d = 0 as ::core::ffi::c_int as scaled_t;
                    }
                }
            }
        }
        (*mem.offset(cur_list.tail as isize)).b32.s1 = new_penalty(
            (*eqtb.offset((INT_BASE + INT_PAR__pre_display_penalty) as isize))
                .b32
                .s1,
        );
        cur_list.tail = (*mem.offset(cur_list.tail as isize)).b32.s1;
        if d + s
            <= (*eqtb.offset((DIMEN_BASE + DIMEN_PAR__pre_display_size) as isize))
                .b32
                .s1
            || l as ::core::ffi::c_int != 0
        {
            g1 = GLUE_PAR__above_display_skip as small_number;
            g2 = GLUE_PAR__below_display_skip as small_number;
        } else {
            g1 = GLUE_PAR__above_display_short_skip as small_number;
            g2 = GLUE_PAR__below_display_short_skip as small_number;
        }
        if l as ::core::ffi::c_int != 0 && e == 0 as scaled_t {
            app_display(j, a, 0 as scaled_t);
            (*mem.offset(cur_list.tail as isize)).b32.s1 = new_penalty(INF_PENALTY as int32_t);
            cur_list.tail = (*mem.offset(cur_list.tail as isize)).b32.s1;
        } else {
            (*mem.offset(cur_list.tail as isize)).b32.s1 = new_param_glue(g1);
            cur_list.tail = (*mem.offset(cur_list.tail as isize)).b32.s1;
        }
        if e != 0 as scaled_t {
            r = new_kern(z - w - e - d);
            if l {
                (*mem.offset(a as isize)).b32.s1 = r;
                (*mem.offset(r as isize)).b32.s1 = b;
                b = a;
                d = 0 as ::core::ffi::c_int as scaled_t;
            } else {
                (*mem.offset(b as isize)).b32.s1 = r;
                (*mem.offset(r as isize)).b32.s1 = a;
            }
            b = hpack(b, 0 as scaled_t, ADDITIONAL as small_number);
        }
        app_display(j, b, d);
        if a != TEX_NULL as int32_t && e == 0 as scaled_t && !l {
            (*mem.offset(cur_list.tail as isize)).b32.s1 = new_penalty(INF_PENALTY as int32_t);
            cur_list.tail = (*mem.offset(cur_list.tail as isize)).b32.s1;
            app_display(
                j,
                a,
                z - (*mem.offset((a + 1 as int32_t) as isize)).b32.s1 as scaled_t,
            );
            g2 = 0 as small_number;
        }
        if t != ADJUST_HEAD as int32_t {
            (*mem.offset(cur_list.tail as isize)).b32.s1 =
                (*mem.offset(ADJUST_HEAD as isize)).b32.s1;
            cur_list.tail = t;
        }
        if pre_t != PRE_ADJUST_HEAD as int32_t {
            (*mem.offset(cur_list.tail as isize)).b32.s1 =
                (*mem.offset(PRE_ADJUST_HEAD as isize)).b32.s1;
            cur_list.tail = pre_t;
        }
        (*mem.offset(cur_list.tail as isize)).b32.s1 = new_penalty(
            (*eqtb.offset((INT_BASE + INT_PAR__post_display_penalty) as isize))
                .b32
                .s1,
        );
        cur_list.tail = (*mem.offset(cur_list.tail as isize)).b32.s1;
        if g2 as ::core::ffi::c_int > 0 as ::core::ffi::c_int {
            (*mem.offset(cur_list.tail as isize)).b32.s1 = new_param_glue(g2);
            cur_list.tail = (*mem.offset(cur_list.tail as isize)).b32.s1;
        }
        flush_node_list(j);
        if semantic_pagination_enabled {
            tt_insert_special(b"tdux:ce dmath\0" as *const u8 as *const ::core::ffi::c_char);
        }
        resume_after_display();
    };
}
#[no_mangle]
pub unsafe extern "C" fn resume_after_display() {
    if cur_group as ::core::ffi::c_int != MATH_SHIFT_GROUP {
        confusion(b"display\0" as *const u8 as *const ::core::ffi::c_char);
    }
    unsave();
    cur_list.prev_graf = cur_list.prev_graf + 3 as int32_t;
    push_nest();
    cur_list.mode = HMODE as ::core::ffi::c_short;
    cur_list.aux.b32.s0 = 1000 as ::core::ffi::c_int as int32_t;
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
    cur_list.aux.b32.s1 = cur_lang as int32_t;
    cur_list.prev_graf = ((norm_min(
        (*eqtb.offset((INT_BASE + INT_PAR__left_hyphen_min) as isize))
            .b32
            .s1,
    ) as ::core::ffi::c_int
        * 64 as ::core::ffi::c_int
        + norm_min(
            (*eqtb.offset((INT_BASE + INT_PAR__right_hyphen_min) as isize))
                .b32
                .s1,
        ) as ::core::ffi::c_int) as ::core::ffi::c_long
        * 65536 as ::core::ffi::c_long
        + cur_lang as ::core::ffi::c_long) as int32_t;
    get_x_token();
    if cur_cmd as ::core::ffi::c_int != SPACER {
        back_input();
    }
    if nest_ptr == 1 as int32_t {
        build_page();
    }
}
unsafe extern "C" fn math_x_height(mut size_code: int32_t) -> scaled_t {
    let mut f: int32_t = 0;
    let mut rval: scaled_t = 0;
    f = (*eqtb.offset((MATH_FONT_BASE as int32_t + (2 as int32_t + size_code)) as isize))
        .b32
        .s1;
    if *font_area.offset(f as isize) as ::core::ffi::c_uint == OTGR_FONT_FLAG
        && isOpenTypeMathFont(*font_layout_engine.offset(f as isize) as XeTeXLayoutEngine)
            as ::core::ffi::c_int
            != 0
    {
        rval =
            get_native_mathsy_param(f as ::core::ffi::c_int, 5 as ::core::ffi::c_int) as scaled_t;
    } else {
        rval = (*font_info.offset((5 as int32_t + *param_base.offset(f as isize)) as isize))
            .b32
            .s1 as scaled_t;
    }
    return rval;
}
unsafe extern "C" fn math_quad(mut size_code: int32_t) -> scaled_t {
    let mut f: int32_t = 0;
    let mut rval: scaled_t = 0;
    f = (*eqtb.offset((MATH_FONT_BASE as int32_t + (2 as int32_t + size_code)) as isize))
        .b32
        .s1;
    if *font_area.offset(f as isize) as ::core::ffi::c_uint == OTGR_FONT_FLAG
        && isOpenTypeMathFont(*font_layout_engine.offset(f as isize) as XeTeXLayoutEngine)
            as ::core::ffi::c_int
            != 0
    {
        rval =
            get_native_mathsy_param(f as ::core::ffi::c_int, 6 as ::core::ffi::c_int) as scaled_t;
    } else {
        rval = (*font_info.offset((6 as int32_t + *param_base.offset(f as isize)) as isize))
            .b32
            .s1 as scaled_t;
    }
    return rval;
}
unsafe extern "C" fn num1(mut size_code: int32_t) -> scaled_t {
    let mut f: int32_t = 0;
    let mut rval: scaled_t = 0;
    f = (*eqtb.offset((MATH_FONT_BASE as int32_t + (2 as int32_t + size_code)) as isize))
        .b32
        .s1;
    if *font_area.offset(f as isize) as ::core::ffi::c_uint == OTGR_FONT_FLAG
        && isOpenTypeMathFont(*font_layout_engine.offset(f as isize) as XeTeXLayoutEngine)
            as ::core::ffi::c_int
            != 0
    {
        rval =
            get_native_mathsy_param(f as ::core::ffi::c_int, 8 as ::core::ffi::c_int) as scaled_t;
    } else {
        rval = (*font_info.offset((8 as int32_t + *param_base.offset(f as isize)) as isize))
            .b32
            .s1 as scaled_t;
    }
    return rval;
}
unsafe extern "C" fn num2(mut size_code: int32_t) -> scaled_t {
    let mut f: int32_t = 0;
    let mut rval: scaled_t = 0;
    f = (*eqtb.offset((MATH_FONT_BASE as int32_t + (2 as int32_t + size_code)) as isize))
        .b32
        .s1;
    if *font_area.offset(f as isize) as ::core::ffi::c_uint == OTGR_FONT_FLAG
        && isOpenTypeMathFont(*font_layout_engine.offset(f as isize) as XeTeXLayoutEngine)
            as ::core::ffi::c_int
            != 0
    {
        rval =
            get_native_mathsy_param(f as ::core::ffi::c_int, 9 as ::core::ffi::c_int) as scaled_t;
    } else {
        rval = (*font_info.offset((9 as int32_t + *param_base.offset(f as isize)) as isize))
            .b32
            .s1 as scaled_t;
    }
    return rval;
}
unsafe extern "C" fn num3(mut size_code: int32_t) -> scaled_t {
    let mut f: int32_t = 0;
    let mut rval: scaled_t = 0;
    f = (*eqtb.offset((MATH_FONT_BASE as int32_t + (2 as int32_t + size_code)) as isize))
        .b32
        .s1;
    if *font_area.offset(f as isize) as ::core::ffi::c_uint == OTGR_FONT_FLAG
        && isOpenTypeMathFont(*font_layout_engine.offset(f as isize) as XeTeXLayoutEngine)
            as ::core::ffi::c_int
            != 0
    {
        rval =
            get_native_mathsy_param(f as ::core::ffi::c_int, 10 as ::core::ffi::c_int) as scaled_t;
    } else {
        rval = (*font_info.offset((10 as int32_t + *param_base.offset(f as isize)) as isize))
            .b32
            .s1 as scaled_t;
    }
    return rval;
}
unsafe extern "C" fn denom1(mut size_code: int32_t) -> scaled_t {
    let mut f: int32_t = 0;
    let mut rval: scaled_t = 0;
    f = (*eqtb.offset((MATH_FONT_BASE as int32_t + (2 as int32_t + size_code)) as isize))
        .b32
        .s1;
    if *font_area.offset(f as isize) as ::core::ffi::c_uint == OTGR_FONT_FLAG
        && isOpenTypeMathFont(*font_layout_engine.offset(f as isize) as XeTeXLayoutEngine)
            as ::core::ffi::c_int
            != 0
    {
        rval =
            get_native_mathsy_param(f as ::core::ffi::c_int, 11 as ::core::ffi::c_int) as scaled_t;
    } else {
        rval = (*font_info.offset((11 as int32_t + *param_base.offset(f as isize)) as isize))
            .b32
            .s1 as scaled_t;
    }
    return rval;
}
unsafe extern "C" fn denom2(mut size_code: int32_t) -> scaled_t {
    let mut f: int32_t = 0;
    let mut rval: scaled_t = 0;
    f = (*eqtb.offset((MATH_FONT_BASE as int32_t + (2 as int32_t + size_code)) as isize))
        .b32
        .s1;
    if *font_area.offset(f as isize) as ::core::ffi::c_uint == OTGR_FONT_FLAG
        && isOpenTypeMathFont(*font_layout_engine.offset(f as isize) as XeTeXLayoutEngine)
            as ::core::ffi::c_int
            != 0
    {
        rval =
            get_native_mathsy_param(f as ::core::ffi::c_int, 12 as ::core::ffi::c_int) as scaled_t;
    } else {
        rval = (*font_info.offset((12 as int32_t + *param_base.offset(f as isize)) as isize))
            .b32
            .s1 as scaled_t;
    }
    return rval;
}
unsafe extern "C" fn sup1(mut size_code: int32_t) -> scaled_t {
    let mut f: int32_t = 0;
    let mut rval: scaled_t = 0;
    f = (*eqtb.offset((MATH_FONT_BASE as int32_t + (2 as int32_t + size_code)) as isize))
        .b32
        .s1;
    if *font_area.offset(f as isize) as ::core::ffi::c_uint == OTGR_FONT_FLAG
        && isOpenTypeMathFont(*font_layout_engine.offset(f as isize) as XeTeXLayoutEngine)
            as ::core::ffi::c_int
            != 0
    {
        rval =
            get_native_mathsy_param(f as ::core::ffi::c_int, 13 as ::core::ffi::c_int) as scaled_t;
    } else {
        rval = (*font_info.offset((13 as int32_t + *param_base.offset(f as isize)) as isize))
            .b32
            .s1 as scaled_t;
    }
    return rval;
}
unsafe extern "C" fn sup2(mut size_code: int32_t) -> scaled_t {
    let mut f: int32_t = 0;
    let mut rval: scaled_t = 0;
    f = (*eqtb.offset((MATH_FONT_BASE as int32_t + (2 as int32_t + size_code)) as isize))
        .b32
        .s1;
    if *font_area.offset(f as isize) as ::core::ffi::c_uint == OTGR_FONT_FLAG
        && isOpenTypeMathFont(*font_layout_engine.offset(f as isize) as XeTeXLayoutEngine)
            as ::core::ffi::c_int
            != 0
    {
        rval =
            get_native_mathsy_param(f as ::core::ffi::c_int, 14 as ::core::ffi::c_int) as scaled_t;
    } else {
        rval = (*font_info.offset((14 as int32_t + *param_base.offset(f as isize)) as isize))
            .b32
            .s1 as scaled_t;
    }
    return rval;
}
unsafe extern "C" fn sup3(mut size_code: int32_t) -> scaled_t {
    let mut f: int32_t = 0;
    let mut rval: scaled_t = 0;
    f = (*eqtb.offset((MATH_FONT_BASE as int32_t + (2 as int32_t + size_code)) as isize))
        .b32
        .s1;
    if *font_area.offset(f as isize) as ::core::ffi::c_uint == OTGR_FONT_FLAG
        && isOpenTypeMathFont(*font_layout_engine.offset(f as isize) as XeTeXLayoutEngine)
            as ::core::ffi::c_int
            != 0
    {
        rval =
            get_native_mathsy_param(f as ::core::ffi::c_int, 15 as ::core::ffi::c_int) as scaled_t;
    } else {
        rval = (*font_info.offset((15 as int32_t + *param_base.offset(f as isize)) as isize))
            .b32
            .s1 as scaled_t;
    }
    return rval;
}
unsafe extern "C" fn sub1(mut size_code: int32_t) -> scaled_t {
    let mut f: int32_t = 0;
    let mut rval: scaled_t = 0;
    f = (*eqtb.offset((MATH_FONT_BASE as int32_t + (2 as int32_t + size_code)) as isize))
        .b32
        .s1;
    if *font_area.offset(f as isize) as ::core::ffi::c_uint == OTGR_FONT_FLAG
        && isOpenTypeMathFont(*font_layout_engine.offset(f as isize) as XeTeXLayoutEngine)
            as ::core::ffi::c_int
            != 0
    {
        rval =
            get_native_mathsy_param(f as ::core::ffi::c_int, 16 as ::core::ffi::c_int) as scaled_t;
    } else {
        rval = (*font_info.offset((16 as int32_t + *param_base.offset(f as isize)) as isize))
            .b32
            .s1 as scaled_t;
    }
    return rval;
}
unsafe extern "C" fn sub2(mut size_code: int32_t) -> scaled_t {
    let mut f: int32_t = 0;
    let mut rval: scaled_t = 0;
    f = (*eqtb.offset((MATH_FONT_BASE as int32_t + (2 as int32_t + size_code)) as isize))
        .b32
        .s1;
    if *font_area.offset(f as isize) as ::core::ffi::c_uint == OTGR_FONT_FLAG
        && isOpenTypeMathFont(*font_layout_engine.offset(f as isize) as XeTeXLayoutEngine)
            as ::core::ffi::c_int
            != 0
    {
        rval =
            get_native_mathsy_param(f as ::core::ffi::c_int, 17 as ::core::ffi::c_int) as scaled_t;
    } else {
        rval = (*font_info.offset((17 as int32_t + *param_base.offset(f as isize)) as isize))
            .b32
            .s1 as scaled_t;
    }
    return rval;
}
unsafe extern "C" fn sup_drop(mut size_code: int32_t) -> scaled_t {
    let mut f: int32_t = 0;
    let mut rval: scaled_t = 0;
    f = (*eqtb.offset((MATH_FONT_BASE as int32_t + (2 as int32_t + size_code)) as isize))
        .b32
        .s1;
    if *font_area.offset(f as isize) as ::core::ffi::c_uint == OTGR_FONT_FLAG
        && isOpenTypeMathFont(*font_layout_engine.offset(f as isize) as XeTeXLayoutEngine)
            as ::core::ffi::c_int
            != 0
    {
        rval =
            get_native_mathsy_param(f as ::core::ffi::c_int, 18 as ::core::ffi::c_int) as scaled_t;
    } else {
        rval = (*font_info.offset((18 as int32_t + *param_base.offset(f as isize)) as isize))
            .b32
            .s1 as scaled_t;
    }
    return rval;
}
unsafe extern "C" fn sub_drop(mut size_code: int32_t) -> scaled_t {
    let mut f: int32_t = 0;
    let mut rval: scaled_t = 0;
    f = (*eqtb.offset((MATH_FONT_BASE as int32_t + (2 as int32_t + size_code)) as isize))
        .b32
        .s1;
    if *font_area.offset(f as isize) as ::core::ffi::c_uint == OTGR_FONT_FLAG
        && isOpenTypeMathFont(*font_layout_engine.offset(f as isize) as XeTeXLayoutEngine)
            as ::core::ffi::c_int
            != 0
    {
        rval =
            get_native_mathsy_param(f as ::core::ffi::c_int, 19 as ::core::ffi::c_int) as scaled_t;
    } else {
        rval = (*font_info.offset((19 as int32_t + *param_base.offset(f as isize)) as isize))
            .b32
            .s1 as scaled_t;
    }
    return rval;
}
unsafe extern "C" fn delim1(mut size_code: int32_t) -> scaled_t {
    let mut f: int32_t = 0;
    let mut rval: scaled_t = 0;
    f = (*eqtb.offset((MATH_FONT_BASE as int32_t + (2 as int32_t + size_code)) as isize))
        .b32
        .s1;
    if *font_area.offset(f as isize) as ::core::ffi::c_uint == OTGR_FONT_FLAG
        && isOpenTypeMathFont(*font_layout_engine.offset(f as isize) as XeTeXLayoutEngine)
            as ::core::ffi::c_int
            != 0
    {
        rval =
            get_native_mathsy_param(f as ::core::ffi::c_int, 20 as ::core::ffi::c_int) as scaled_t;
    } else {
        rval = (*font_info.offset((20 as int32_t + *param_base.offset(f as isize)) as isize))
            .b32
            .s1 as scaled_t;
    }
    return rval;
}
unsafe extern "C" fn delim2(mut size_code: int32_t) -> scaled_t {
    let mut f: int32_t = 0;
    let mut rval: scaled_t = 0;
    f = (*eqtb.offset((MATH_FONT_BASE as int32_t + (2 as int32_t + size_code)) as isize))
        .b32
        .s1;
    if *font_area.offset(f as isize) as ::core::ffi::c_uint == OTGR_FONT_FLAG
        && isOpenTypeMathFont(*font_layout_engine.offset(f as isize) as XeTeXLayoutEngine)
            as ::core::ffi::c_int
            != 0
    {
        rval =
            get_native_mathsy_param(f as ::core::ffi::c_int, 21 as ::core::ffi::c_int) as scaled_t;
    } else {
        rval = (*font_info.offset((21 as int32_t + *param_base.offset(f as isize)) as isize))
            .b32
            .s1 as scaled_t;
    }
    return rval;
}
unsafe extern "C" fn axis_height(mut size_code: int32_t) -> scaled_t {
    let mut f: int32_t = 0;
    let mut rval: scaled_t = 0;
    f = (*eqtb.offset((MATH_FONT_BASE as int32_t + (2 as int32_t + size_code)) as isize))
        .b32
        .s1;
    if *font_area.offset(f as isize) as ::core::ffi::c_uint == OTGR_FONT_FLAG
        && isOpenTypeMathFont(*font_layout_engine.offset(f as isize) as XeTeXLayoutEngine)
            as ::core::ffi::c_int
            != 0
    {
        rval =
            get_native_mathsy_param(f as ::core::ffi::c_int, 22 as ::core::ffi::c_int) as scaled_t;
    } else {
        rval = (*font_info.offset((22 as int32_t + *param_base.offset(f as isize)) as isize))
            .b32
            .s1 as scaled_t;
    }
    return rval;
}
unsafe extern "C" fn default_rule_thickness() -> scaled_t {
    let mut f: int32_t = 0;
    let mut rval: scaled_t = 0;
    f = (*eqtb.offset((MATH_FONT_BASE as int32_t + (3 as int32_t + cur_size)) as isize))
        .b32
        .s1;
    if *font_area.offset(f as isize) as ::core::ffi::c_uint == OTGR_FONT_FLAG
        && isOpenTypeMathFont(*font_layout_engine.offset(f as isize) as XeTeXLayoutEngine)
            as ::core::ffi::c_int
            != 0
    {
        rval =
            get_native_mathex_param(f as ::core::ffi::c_int, 8 as ::core::ffi::c_int) as scaled_t;
    } else {
        rval = (*font_info.offset((8 as int32_t + *param_base.offset(f as isize)) as isize))
            .b32
            .s1 as scaled_t;
    }
    return rval;
}
unsafe extern "C" fn big_op_spacing1() -> scaled_t {
    let mut f: int32_t = 0;
    let mut rval: scaled_t = 0;
    f = (*eqtb.offset((MATH_FONT_BASE as int32_t + (3 as int32_t + cur_size)) as isize))
        .b32
        .s1;
    if *font_area.offset(f as isize) as ::core::ffi::c_uint == OTGR_FONT_FLAG
        && isOpenTypeMathFont(*font_layout_engine.offset(f as isize) as XeTeXLayoutEngine)
            as ::core::ffi::c_int
            != 0
    {
        rval =
            get_native_mathex_param(f as ::core::ffi::c_int, 9 as ::core::ffi::c_int) as scaled_t;
    } else {
        rval = (*font_info.offset((9 as int32_t + *param_base.offset(f as isize)) as isize))
            .b32
            .s1 as scaled_t;
    }
    return rval;
}
unsafe extern "C" fn big_op_spacing2() -> scaled_t {
    let mut f: int32_t = 0;
    let mut rval: scaled_t = 0;
    f = (*eqtb.offset((MATH_FONT_BASE as int32_t + (3 as int32_t + cur_size)) as isize))
        .b32
        .s1;
    if *font_area.offset(f as isize) as ::core::ffi::c_uint == OTGR_FONT_FLAG
        && isOpenTypeMathFont(*font_layout_engine.offset(f as isize) as XeTeXLayoutEngine)
            as ::core::ffi::c_int
            != 0
    {
        rval =
            get_native_mathex_param(f as ::core::ffi::c_int, 10 as ::core::ffi::c_int) as scaled_t;
    } else {
        rval = (*font_info.offset((10 as int32_t + *param_base.offset(f as isize)) as isize))
            .b32
            .s1 as scaled_t;
    }
    return rval;
}
unsafe extern "C" fn big_op_spacing3() -> scaled_t {
    let mut f: int32_t = 0;
    let mut rval: scaled_t = 0;
    f = (*eqtb.offset((MATH_FONT_BASE as int32_t + (3 as int32_t + cur_size)) as isize))
        .b32
        .s1;
    if *font_area.offset(f as isize) as ::core::ffi::c_uint == OTGR_FONT_FLAG
        && isOpenTypeMathFont(*font_layout_engine.offset(f as isize) as XeTeXLayoutEngine)
            as ::core::ffi::c_int
            != 0
    {
        rval =
            get_native_mathex_param(f as ::core::ffi::c_int, 11 as ::core::ffi::c_int) as scaled_t;
    } else {
        rval = (*font_info.offset((11 as int32_t + *param_base.offset(f as isize)) as isize))
            .b32
            .s1 as scaled_t;
    }
    return rval;
}
unsafe extern "C" fn big_op_spacing4() -> scaled_t {
    let mut f: int32_t = 0;
    let mut rval: scaled_t = 0;
    f = (*eqtb.offset((MATH_FONT_BASE as int32_t + (3 as int32_t + cur_size)) as isize))
        .b32
        .s1;
    if *font_area.offset(f as isize) as ::core::ffi::c_uint == OTGR_FONT_FLAG
        && isOpenTypeMathFont(*font_layout_engine.offset(f as isize) as XeTeXLayoutEngine)
            as ::core::ffi::c_int
            != 0
    {
        rval =
            get_native_mathex_param(f as ::core::ffi::c_int, 12 as ::core::ffi::c_int) as scaled_t;
    } else {
        rval = (*font_info.offset((12 as int32_t + *param_base.offset(f as isize)) as isize))
            .b32
            .s1 as scaled_t;
    }
    return rval;
}
unsafe extern "C" fn big_op_spacing5() -> scaled_t {
    let mut f: int32_t = 0;
    let mut rval: scaled_t = 0;
    f = (*eqtb.offset((MATH_FONT_BASE as int32_t + (3 as int32_t + cur_size)) as isize))
        .b32
        .s1;
    if *font_area.offset(f as isize) as ::core::ffi::c_uint == OTGR_FONT_FLAG
        && isOpenTypeMathFont(*font_layout_engine.offset(f as isize) as XeTeXLayoutEngine)
            as ::core::ffi::c_int
            != 0
    {
        rval =
            get_native_mathex_param(f as ::core::ffi::c_int, 13 as ::core::ffi::c_int) as scaled_t;
    } else {
        rval = (*font_info.offset((13 as int32_t + *param_base.offset(f as isize)) as isize))
            .b32
            .s1 as scaled_t;
    }
    return rval;
}
unsafe extern "C" fn fraction_rule(mut t: scaled_t) -> int32_t {
    let mut p: int32_t = 0;
    p = new_rule();
    (*mem.offset((p + 3 as int32_t) as isize)).b32.s1 = t as int32_t;
    (*mem.offset((p + 2 as int32_t) as isize)).b32.s1 = 0 as ::core::ffi::c_int as int32_t;
    return p;
}
unsafe extern "C" fn overbar(mut b: int32_t, mut k: scaled_t, mut t: scaled_t) -> int32_t {
    let mut p: int32_t = 0;
    let mut q: int32_t = 0;
    p = new_kern(k);
    (*mem.offset(p as isize)).b32.s1 = b;
    q = fraction_rule(t);
    (*mem.offset(q as isize)).b32.s1 = p;
    p = new_kern(t);
    (*mem.offset(p as isize)).b32.s1 = q;
    return vpackage(
        p,
        0 as scaled_t,
        ADDITIONAL as small_number,
        MAX_HALFWORD as scaled_t,
    );
}
unsafe extern "C" fn math_glue(mut g: int32_t, mut m: scaled_t) -> int32_t {
    let mut p: int32_t = 0;
    let mut n: int32_t = 0;
    let mut f: scaled_t = 0;
    n = x_over_n(m, 65536 as int32_t) as int32_t;
    f = tex_remainder;
    if f < 0 as scaled_t {
        n -= 1;
        f = (f as ::core::ffi::c_long + 65536 as ::core::ffi::c_long) as scaled_t;
    }
    p = get_node(GLUE_SPEC_SIZE as int32_t);
    (*mem.offset((p + 1 as int32_t) as isize)).b32.s1 = mult_and_add(
        n,
        (*mem.offset((g + 1 as int32_t) as isize)).b32.s1 as scaled_t,
        xn_over_d(
            (*mem.offset((g + 1 as int32_t) as isize)).b32.s1 as scaled_t,
            f as int32_t,
            65536 as int32_t,
        ),
        MAX_HALFWORD as scaled_t,
    ) as int32_t;
    (*mem.offset(p as isize)).b16.s1 = (*mem.offset(g as isize)).b16.s1;
    if (*mem.offset(p as isize)).b16.s1 as ::core::ffi::c_int == NORMAL {
        (*mem.offset((p + 2 as int32_t) as isize)).b32.s1 = mult_and_add(
            n,
            (*mem.offset((g + 2 as int32_t) as isize)).b32.s1 as scaled_t,
            xn_over_d(
                (*mem.offset((g + 2 as int32_t) as isize)).b32.s1 as scaled_t,
                f as int32_t,
                65536 as int32_t,
            ),
            MAX_HALFWORD as scaled_t,
        ) as int32_t;
    } else {
        (*mem.offset((p + 2 as int32_t) as isize)).b32.s1 =
            (*mem.offset((g + 2 as int32_t) as isize)).b32.s1;
    }
    (*mem.offset(p as isize)).b16.s0 = (*mem.offset(g as isize)).b16.s0;
    if (*mem.offset(p as isize)).b16.s0 as ::core::ffi::c_int == NORMAL {
        (*mem.offset((p + 3 as int32_t) as isize)).b32.s1 = mult_and_add(
            n,
            (*mem.offset((g + 3 as int32_t) as isize)).b32.s1 as scaled_t,
            xn_over_d(
                (*mem.offset((g + 3 as int32_t) as isize)).b32.s1 as scaled_t,
                f as int32_t,
                65536 as int32_t,
            ),
            MAX_HALFWORD as scaled_t,
        ) as int32_t;
    } else {
        (*mem.offset((p + 3 as int32_t) as isize)).b32.s1 =
            (*mem.offset((g + 3 as int32_t) as isize)).b32.s1;
    }
    return p;
}
unsafe extern "C" fn math_kern(mut p: int32_t, mut m: scaled_t) {
    let mut n: int32_t = 0;
    let mut f: scaled_t = 0;
    if (*mem.offset(p as isize)).b16.s0 as ::core::ffi::c_int == MU_GLUE {
        n = x_over_n(m, 65536 as int32_t) as int32_t;
        f = tex_remainder;
        if f < 0 as scaled_t {
            n -= 1;
            f = (f as ::core::ffi::c_long + 65536 as ::core::ffi::c_long) as scaled_t;
        }
        (*mem.offset((p + 1 as int32_t) as isize)).b32.s1 = mult_and_add(
            n,
            (*mem.offset((p + 1 as int32_t) as isize)).b32.s1 as scaled_t,
            xn_over_d(
                (*mem.offset((p + 1 as int32_t) as isize)).b32.s1 as scaled_t,
                f as int32_t,
                65536 as int32_t,
            ),
            MAX_HALFWORD as scaled_t,
        ) as int32_t;
        (*mem.offset(p as isize)).b16.s0 = EXPLICIT as uint16_t;
    }
}
#[no_mangle]
pub unsafe extern "C" fn flush_math() {
    flush_node_list((*mem.offset(cur_list.head as isize)).b32.s1);
    flush_node_list(cur_list.aux.b32.s1);
    (*mem.offset(cur_list.head as isize)).b32.s1 = TEX_NULL as int32_t;
    cur_list.tail = cur_list.head;
    cur_list.aux.b32.s1 = TEX_NULL as int32_t;
}
unsafe extern "C" fn clean_box(mut p: int32_t, mut s: small_number) -> int32_t {
    let mut current_block: u64;
    let mut q: int32_t = 0;
    let mut save_style: small_number = 0;
    let mut x: int32_t = 0;
    let mut r: int32_t = 0;
    match (*mem.offset(p as isize)).b32.s1 {
        1 => {
            cur_mlist = new_noad();
            *mem.offset((cur_mlist + 1 as int32_t) as isize) = *mem.offset(p as isize);
            current_block = 13183875560443969876;
        }
        2 => {
            q = (*mem.offset(p as isize)).b32.s0;
            current_block = 1043924212389387875;
        }
        3 => {
            cur_mlist = (*mem.offset(p as isize)).b32.s0;
            current_block = 13183875560443969876;
        }
        _ => {
            q = new_null_box();
            current_block = 1043924212389387875;
        }
    }
    match current_block {
        13183875560443969876 => {
            save_style = cur_style;
            cur_style = s;
            mlist_penalties = false_0 != 0;
            mlist_to_hlist();
            q = (*mem.offset(TEMP_HEAD as isize)).b32.s1;
            cur_style = save_style;
            if (cur_style as ::core::ffi::c_int) < SCRIPT_STYLE {
                cur_size = TEXT_SIZE as int32_t;
            } else {
                cur_size = (SCRIPT_SIZE
                    * ((cur_style as ::core::ffi::c_int - 2 as ::core::ffi::c_int)
                        / 2 as ::core::ffi::c_int)) as int32_t;
            }
            cur_mu = x_over_n(math_quad(cur_size), 18 as int32_t);
        }
        _ => {}
    }
    if is_char_node(q) as ::core::ffi::c_int != 0 || q == TEX_NULL as int32_t {
        x = hpack(q, 0 as scaled_t, ADDITIONAL as small_number);
    } else if (*mem.offset(q as isize)).b32.s1 == TEX_NULL as int32_t
        && (*mem.offset(q as isize)).b16.s1 as ::core::ffi::c_int <= VLIST_NODE
        && (*mem.offset((q + 4 as int32_t) as isize)).b32.s1 == 0 as int32_t
    {
        x = q;
    } else {
        x = hpack(q, 0 as scaled_t, ADDITIONAL as small_number);
    }
    q = (*mem.offset((x + 5 as int32_t) as isize)).b32.s1;
    if is_char_node(q) {
        r = (*mem.offset(q as isize)).b32.s1;
        if r != TEX_NULL as int32_t {
            if (*mem.offset(r as isize)).b32.s1 == TEX_NULL as int32_t {
                if !is_char_node(r) {
                    if (*mem.offset(r as isize)).b16.s1 as ::core::ffi::c_int == KERN_NODE {
                        free_node(r, MEDIUM_NODE_SIZE as int32_t);
                        (*mem.offset(q as isize)).b32.s1 = TEX_NULL as int32_t;
                    }
                }
            }
        }
    }
    return x;
}
unsafe extern "C" fn fetch(mut a: int32_t) {
    cur_c = (*mem.offset(a as isize)).b16.s0 as ::core::ffi::c_ushort as int32_t;
    cur_f = (*eqtb.offset(
        (MATH_FONT_BASE as int32_t
            + ((*mem.offset(a as isize)).b16.s1 as int32_t % 256 as int32_t + cur_size))
            as isize,
    ))
    .b32
    .s1 as internal_font_number;
    cur_c = (cur_c as ::core::ffi::c_long
        + ((*mem.offset(a as isize)).b16.s1 as ::core::ffi::c_int / 256 as ::core::ffi::c_int)
            as ::core::ffi::c_long
            * 65536 as ::core::ffi::c_long) as int32_t;
    if cur_f == FONT_BASE as internal_font_number {
        error_here_with_diagnostic(b"\0" as *const u8 as *const ::core::ffi::c_char);
        print_size(cur_size);
        print_char(' ' as i32);
        print_int((*mem.offset(a as isize)).b16.s1 as int32_t % 256 as int32_t);
        print_cstr(b" is undefined (character \0" as *const u8 as *const ::core::ffi::c_char);
        print(cur_c);
        print_char(')' as i32);
        capture_to_diagnostic(::core::ptr::null_mut::<ttbc_diagnostic_t>());
        help_ptr = 4 as ::core::ffi::c_uchar;
        help_line[3 as ::core::ffi::c_int as usize] =
            b"Somewhere in the math formula just ended, you used the\0" as *const u8
                as *const ::core::ffi::c_char;
        help_line[2 as ::core::ffi::c_int as usize] =
            b"stated character from an undefined font family. For example,\0" as *const u8
                as *const ::core::ffi::c_char;
        help_line[1 as ::core::ffi::c_int as usize] =
            b"plain TeX doesn't allow \\it or \\sl in subscripts. Proceed,\0" as *const u8
                as *const ::core::ffi::c_char;
        help_line[0 as ::core::ffi::c_int as usize] =
            b"and I'll try to forget that I needed that character.\0" as *const u8
                as *const ::core::ffi::c_char;
        error();
        cur_i = null_character;
        (*mem.offset(a as isize)).b32.s1 = EMPTY as int32_t;
    } else if *font_area.offset(cur_f as isize) as ::core::ffi::c_uint == AAT_FONT_FLAG
        || *font_area.offset(cur_f as isize) as ::core::ffi::c_uint == OTGR_FONT_FLAG
    {
        cur_i = null_character;
    } else {
        if cur_c >= *font_bc.offset(cur_f as isize) as int32_t
            && cur_c <= *font_ec.offset(cur_f as isize) as int32_t
        {
            cur_i = (*font_info.offset((*char_base.offset(cur_f as isize) + cur_c) as isize)).b16;
        } else {
            cur_i = null_character;
        }
        if !(cur_i.s3 as ::core::ffi::c_int > 0 as ::core::ffi::c_int) {
            char_warning(cur_f, cur_c);
            (*mem.offset(a as isize)).b32.s1 = EMPTY as int32_t;
            cur_i = null_character;
        }
    };
}
unsafe extern "C" fn make_over(mut q: int32_t) {
    (*mem.offset((q + 1 as int32_t) as isize)).b32.s0 = overbar(
        clean_box(
            q + 1 as int32_t,
            (2 as ::core::ffi::c_int * (cur_style as ::core::ffi::c_int / 2 as ::core::ffi::c_int)
                + 1 as ::core::ffi::c_int) as small_number,
        ),
        3 as scaled_t * default_rule_thickness(),
        default_rule_thickness(),
    );
    (*mem.offset((q + 1 as int32_t) as isize)).b32.s1 = SUB_BOX as int32_t;
}
unsafe extern "C" fn make_under(mut q: int32_t) {
    let mut p: int32_t = 0;
    let mut x: int32_t = 0;
    let mut y: int32_t = 0;
    let mut delta: scaled_t = 0;
    x = clean_box(q + 1 as int32_t, cur_style);
    p = new_kern(3 as scaled_t * default_rule_thickness());
    (*mem.offset(x as isize)).b32.s1 = p;
    (*mem.offset(p as isize)).b32.s1 = fraction_rule(default_rule_thickness());
    y = vpackage(
        x,
        0 as scaled_t,
        ADDITIONAL as small_number,
        MAX_HALFWORD as scaled_t,
    );
    delta = ((*mem.offset((y + 3 as int32_t) as isize)).b32.s1
        + (*mem.offset((y + 2 as int32_t) as isize)).b32.s1
        + default_rule_thickness() as int32_t) as scaled_t;
    (*mem.offset((y + 3 as int32_t) as isize)).b32.s1 =
        (*mem.offset((x + 3 as int32_t) as isize)).b32.s1;
    (*mem.offset((y + 2 as int32_t) as isize)).b32.s1 =
        (delta - (*mem.offset((y + 3 as int32_t) as isize)).b32.s1 as scaled_t) as int32_t;
    (*mem.offset((q + 1 as int32_t) as isize)).b32.s0 = y;
    (*mem.offset((q + 1 as int32_t) as isize)).b32.s1 = SUB_BOX as int32_t;
}
unsafe extern "C" fn make_vcenter(mut q: int32_t) {
    let mut v: int32_t = 0;
    let mut delta: scaled_t = 0;
    v = (*mem.offset((q + 1 as int32_t) as isize)).b32.s0;
    if (*mem.offset(v as isize)).b16.s1 as ::core::ffi::c_int != VLIST_NODE {
        confusion(b"vcenter\0" as *const u8 as *const ::core::ffi::c_char);
    }
    delta = ((*mem.offset((v + 3 as int32_t) as isize)).b32.s1
        + (*mem.offset((v + 2 as int32_t) as isize)).b32.s1) as scaled_t;
    (*mem.offset((v + 3 as int32_t) as isize)).b32.s1 =
        (axis_height(cur_size) + half(delta as int32_t) as scaled_t) as int32_t;
    (*mem.offset((v + 2 as int32_t) as isize)).b32.s1 =
        (delta - (*mem.offset((v + 3 as int32_t) as isize)).b32.s1 as scaled_t) as int32_t;
}
unsafe extern "C" fn make_radical(mut q: int32_t) {
    let mut x: int32_t = 0;
    let mut y: int32_t = 0;
    let mut f: internal_font_number = 0;
    let mut rule_thickness: scaled_t = 0;
    let mut delta: scaled_t = 0;
    let mut clr: scaled_t = 0;
    f = (*eqtb.offset(
        (MATH_FONT_BASE as int32_t
            + ((*mem.offset((q + 4 as int32_t) as isize)).b16.s3 as int32_t % 256 as int32_t
                + cur_size)) as isize,
    ))
    .b32
    .s1 as internal_font_number;
    if *font_area.offset(f as isize) as ::core::ffi::c_uint == OTGR_FONT_FLAG
        && isOpenTypeMathFont(*font_layout_engine.offset(f as isize) as XeTeXLayoutEngine)
            as ::core::ffi::c_int
            != 0
    {
        rule_thickness =
            get_ot_math_constant(f as ::core::ffi::c_int, RADICALRULETHICKNESS) as scaled_t;
    } else {
        rule_thickness = default_rule_thickness();
    }
    x = clean_box(
        q + 1 as int32_t,
        (2 as ::core::ffi::c_int * (cur_style as ::core::ffi::c_int / 2 as ::core::ffi::c_int)
            + 1 as ::core::ffi::c_int) as small_number,
    );
    if *font_area.offset(f as isize) as ::core::ffi::c_uint == OTGR_FONT_FLAG
        && isOpenTypeMathFont(*font_layout_engine.offset(f as isize) as XeTeXLayoutEngine)
            as ::core::ffi::c_int
            != 0
    {
        if (cur_style as ::core::ffi::c_int) < TEXT_STYLE {
            clr = get_ot_math_constant(f as ::core::ffi::c_int, RADICALDISPLAYSTYLEVERTICALGAP)
                as scaled_t;
        } else {
            clr = get_ot_math_constant(f as ::core::ffi::c_int, RADICALVERTICALGAP) as scaled_t;
        }
    } else if (cur_style as ::core::ffi::c_int) < TEXT_STYLE {
        clr = rule_thickness
            + abs(math_x_height(cur_size) as ::core::ffi::c_int) as scaled_t / 4 as scaled_t;
    } else {
        clr = rule_thickness;
        clr = clr + abs(clr as ::core::ffi::c_int) as scaled_t / 4 as scaled_t;
    }
    y = var_delimiter(
        q + 4 as int32_t,
        cur_size,
        (*mem.offset((x + 3 as int32_t) as isize)).b32.s1 as scaled_t
            + (*mem.offset((x + 2 as int32_t) as isize)).b32.s1 as scaled_t
            + clr
            + rule_thickness,
    );
    if *font_area.offset(f as isize) as ::core::ffi::c_uint == OTGR_FONT_FLAG
        && isOpenTypeMathFont(*font_layout_engine.offset(f as isize) as XeTeXLayoutEngine)
            as ::core::ffi::c_int
            != 0
    {
        (*mem.offset((y + 2 as int32_t) as isize)).b32.s1 =
            (*mem.offset((y + 3 as int32_t) as isize)).b32.s1
                + (*mem.offset((y + 2 as int32_t) as isize)).b32.s1
                - rule_thickness as int32_t;
        (*mem.offset((y + 3 as int32_t) as isize)).b32.s1 = rule_thickness as int32_t;
    }
    delta = ((*mem.offset((y + 2 as int32_t) as isize)).b32.s1
        - ((*mem.offset((x + 3 as int32_t) as isize)).b32.s1
            + (*mem.offset((x + 2 as int32_t) as isize)).b32.s1
            + clr as int32_t)) as scaled_t;
    if delta > 0 as scaled_t {
        clr = clr + half(delta as int32_t) as scaled_t;
    }
    (*mem.offset((y + 4 as int32_t) as isize)).b32.s1 =
        -((*mem.offset((x + 3 as int32_t) as isize)).b32.s1 + clr as int32_t);
    (*mem.offset(y as isize)).b32.s1 = overbar(
        x,
        clr,
        (*mem.offset((y + 3 as int32_t) as isize)).b32.s1 as scaled_t,
    );
    (*mem.offset((q + 1 as int32_t) as isize)).b32.s0 =
        hpack(y, 0 as scaled_t, ADDITIONAL as small_number);
    (*mem.offset((q + 1 as int32_t) as isize)).b32.s1 = SUB_BOX as int32_t;
}
unsafe extern "C" fn compute_ot_math_accent_pos(mut p: int32_t) -> scaled_t {
    let mut q: int32_t = 0;
    let mut r: int32_t = 0;
    let mut s: scaled_t = 0;
    let mut g: scaled_t = 0;
    if (*mem.offset((p + 1 as int32_t) as isize)).b32.s1 == MATH_CHAR as int32_t {
        fetch(p + 1 as int32_t);
        q = new_native_character(cur_f, cur_c as UnicodeScalar);
        g = real_get_native_glyph(
            mem.offset(q as isize) as *mut memory_word as *mut ::core::ffi::c_void,
            0 as ::core::ffi::c_uint,
        ) as scaled_t;
        s = get_ot_math_accent_pos(cur_f as ::core::ffi::c_int, g as ::core::ffi::c_int)
            as scaled_t;
    } else if (*mem.offset((p + 1 as int32_t) as isize)).b32.s1 == SUB_MLIST as int32_t {
        r = (*mem.offset((p + 1 as int32_t) as isize)).b32.s0;
        if r != TEX_NULL as int32_t
            && (*mem.offset(r as isize)).b16.s1 as ::core::ffi::c_int == ACCENT_NOAD
        {
            s = compute_ot_math_accent_pos(r);
        } else {
            s = TEX_INFINITY as scaled_t;
        }
    } else {
        s = TEX_INFINITY as scaled_t;
    }
    return s;
}
unsafe extern "C" fn make_math_accent(mut q: int32_t) {
    let mut p: int32_t = 0;
    let mut x: int32_t = 0;
    let mut y: int32_t = 0;
    let mut a: int32_t = 0;
    let mut c: int32_t = 0;
    let mut g: int32_t = 0;
    let mut f: internal_font_number = 0;
    let mut i: b16x4 = b16x4_le_t {
        s0: 0,
        s1: 0,
        s2: 0,
        s3: 0,
    };
    let mut s: scaled_t = 0;
    let mut sa: scaled_t = 0;
    let mut h: scaled_t = 0;
    let mut delta: scaled_t = 0;
    let mut w: scaled_t = 0;
    let mut w2: scaled_t = 0;
    let mut ot_assembly_ptr: *mut ::core::ffi::c_void =
        ::core::ptr::null_mut::<::core::ffi::c_void>();
    fetch(q + 4 as int32_t);
    x = TEX_NULL as int32_t;
    ot_assembly_ptr = NULL;
    if *font_area.offset(cur_f as isize) as ::core::ffi::c_uint == AAT_FONT_FLAG
        || *font_area.offset(cur_f as isize) as ::core::ffi::c_uint == OTGR_FONT_FLAG
    {
        c = cur_c;
        f = cur_f;
        if !((*mem.offset(q as isize)).b16.s0 as ::core::ffi::c_int == BOTTOM_ACC
            || (*mem.offset(q as isize)).b16.s0 as ::core::ffi::c_int
                == BOTTOM_ACC + 1 as ::core::ffi::c_int)
        {
            s = compute_ot_math_accent_pos(q);
        } else {
            s = 0 as ::core::ffi::c_int as scaled_t;
        }
        x = clean_box(
            q + 1 as int32_t,
            (2 as ::core::ffi::c_int * (cur_style as ::core::ffi::c_int / 2 as ::core::ffi::c_int)
                + 1 as ::core::ffi::c_int) as small_number,
        );
        w = (*mem.offset((x + 1 as int32_t) as isize)).b32.s1 as scaled_t;
        h = (*mem.offset((x + 3 as int32_t) as isize)).b32.s1 as scaled_t;
    } else if cur_i.s3 as ::core::ffi::c_int > 0 as ::core::ffi::c_int {
        i = cur_i;
        c = cur_c;
        f = cur_f;
        s = 0 as ::core::ffi::c_int as scaled_t;
        if (*mem.offset((q + 1 as int32_t) as isize)).b32.s1 == MATH_CHAR as int32_t {
            fetch(q + 1 as int32_t);
            if cur_i.s1 as ::core::ffi::c_int % 4 as ::core::ffi::c_int == LIG_TAG {
                a = *lig_kern_base.offset(cur_f as isize) + cur_i.s0 as int32_t;
                cur_i = (*font_info.offset(a as isize)).b16;
                if cur_i.s3 as ::core::ffi::c_int > 128 as ::core::ffi::c_int {
                    a = ((*lig_kern_base.offset(cur_f as isize)
                        + 256 as int32_t * cur_i.s1 as int32_t
                        + cur_i.s0 as int32_t) as ::core::ffi::c_long
                        + 32768 as ::core::ffi::c_long
                        - (256 as ::core::ffi::c_int * 128 as ::core::ffi::c_int)
                            as ::core::ffi::c_long) as int32_t;
                    cur_i = (*font_info.offset(a as isize)).b16;
                }
                loop {
                    if cur_i.s2 as int32_t == *skew_char.offset(cur_f as isize) {
                        if cur_i.s1 as ::core::ffi::c_int >= 128 as ::core::ffi::c_int {
                            if cur_i.s3 as ::core::ffi::c_int <= 128 as ::core::ffi::c_int {
                                s = (*font_info.offset(
                                    (*kern_base.offset(cur_f as isize)
                                        + 256 as int32_t * cur_i.s1 as int32_t
                                        + cur_i.s0 as int32_t)
                                        as isize,
                                ))
                                .b32
                                .s1 as scaled_t;
                            }
                        }
                        break;
                    } else {
                        if cur_i.s3 as ::core::ffi::c_int >= 128 as ::core::ffi::c_int {
                            break;
                        }
                        a = a + cur_i.s3 as int32_t + 1 as int32_t;
                        cur_i = (*font_info.offset(a as isize)).b16;
                    }
                }
            }
        }
        x = clean_box(
            q + 1 as int32_t,
            (2 as ::core::ffi::c_int * (cur_style as ::core::ffi::c_int / 2 as ::core::ffi::c_int)
                + 1 as ::core::ffi::c_int) as small_number,
        );
        w = (*mem.offset((x + 1 as int32_t) as isize)).b32.s1 as scaled_t;
        h = (*mem.offset((x + 3 as int32_t) as isize)).b32.s1 as scaled_t;
        while !(i.s1 as ::core::ffi::c_int % 4 as ::core::ffi::c_int != LIST_TAG) {
            y = i.s0 as int32_t;
            i = (*font_info.offset((*char_base.offset(f as isize) + y) as isize)).b16;
            if !(i.s3 as ::core::ffi::c_int > 0 as ::core::ffi::c_int) {
                break;
            }
            if (*font_info.offset((*width_base.offset(f as isize) + i.s3 as int32_t) as isize))
                .b32
                .s1
                > w
            {
                break;
            }
            c = y;
        }
    }
    if x != TEX_NULL as int32_t {
        if *font_area.offset(f as isize) as ::core::ffi::c_uint == OTGR_FONT_FLAG
            && isOpenTypeMathFont(*font_layout_engine.offset(f as isize) as XeTeXLayoutEngine)
                as ::core::ffi::c_int
                != 0
        {
            if (*mem.offset(q as isize)).b16.s0 as ::core::ffi::c_int == BOTTOM_ACC
                || (*mem.offset(q as isize)).b16.s0 as ::core::ffi::c_int
                    == BOTTOM_ACC + 1 as ::core::ffi::c_int
            {
                delta = 0 as ::core::ffi::c_int as scaled_t;
            } else if h < get_ot_math_constant(f as ::core::ffi::c_int, ACCENTBASEHEIGHT)
                as scaled_t
            {
                delta = h;
            } else {
                delta = get_ot_math_constant(f as ::core::ffi::c_int, ACCENTBASEHEIGHT) as scaled_t;
            }
        } else if h
            < (*font_info
                .offset((X_HEIGHT_CODE as int32_t + *param_base.offset(f as isize)) as isize))
            .b32
            .s1
        {
            delta = h;
        } else {
            delta = (*font_info
                .offset((X_HEIGHT_CODE as int32_t + *param_base.offset(f as isize)) as isize))
            .b32
            .s1 as scaled_t;
        }
        if (*mem.offset((q + 2 as int32_t) as isize)).b32.s1 != EMPTY as int32_t
            || (*mem.offset((q + 3 as int32_t) as isize)).b32.s1 != EMPTY as int32_t
        {
            if (*mem.offset((q + 1 as int32_t) as isize)).b32.s1 == MATH_CHAR as int32_t {
                flush_node_list(x);
                x = new_noad();
                *mem.offset((x + 1 as int32_t) as isize) = *mem.offset((q + 1 as int32_t) as isize);
                *mem.offset((x + 2 as int32_t) as isize) = *mem.offset((q + 2 as int32_t) as isize);
                *mem.offset((x + 3 as int32_t) as isize) = *mem.offset((q + 3 as int32_t) as isize);
                (*mem.offset((q + 2 as int32_t) as isize)).b32 = empty;
                (*mem.offset((q + 3 as int32_t) as isize)).b32 = empty;
                (*mem.offset((q + 1 as int32_t) as isize)).b32.s1 = SUB_MLIST as int32_t;
                (*mem.offset((q + 1 as int32_t) as isize)).b32.s0 = x;
                x = clean_box(q + 1 as int32_t, cur_style);
                delta = delta + (*mem.offset((x + 3 as int32_t) as isize)).b32.s1 as scaled_t - h;
                h = (*mem.offset((x + 3 as int32_t) as isize)).b32.s1 as scaled_t;
            }
        }
        y = char_box(f, c);
        if *font_area.offset(f as isize) as ::core::ffi::c_uint == AAT_FONT_FLAG
            || *font_area.offset(f as isize) as ::core::ffi::c_uint == OTGR_FONT_FLAG
        {
            p = get_node(GLYPH_NODE_SIZE as int32_t);
            (*mem.offset(p as isize)).b16.s1 = WHATSIT_NODE as uint16_t;
            (*mem.offset(p as isize)).b16.s0 = GLYPH_NODE as uint16_t;
            (*mem.offset((p + 4 as int32_t) as isize)).b16.s2 = f as uint16_t;
            (*mem.offset((p + 4 as int32_t) as isize)).b16.s1 = real_get_native_glyph(
                mem.offset((*mem.offset((y + 5 as int32_t) as isize)).b32.s1 as isize)
                    as *mut memory_word as *mut ::core::ffi::c_void,
                0 as ::core::ffi::c_uint,
            );
            measure_native_glyph(
                mem.offset(p as isize) as *mut memory_word as *mut ::core::ffi::c_void,
                1 as ::core::ffi::c_int,
            );
            free_node(
                (*mem.offset((y + 5 as int32_t) as isize)).b32.s1,
                (*mem.offset(
                    ((*mem.offset((y + 5 as int32_t) as isize)).b32.s1 + 4 as int32_t) as isize,
                ))
                .b16
                .s3 as int32_t,
            );
            (*mem.offset((y + 5 as int32_t) as isize)).b32.s1 = p;
            if (*mem.offset(q as isize)).b16.s0 as ::core::ffi::c_int & 1 as ::core::ffi::c_int != 0
            {
                measure_native_glyph(
                    mem.offset(p as isize) as *mut memory_word as *mut ::core::ffi::c_void,
                    1 as ::core::ffi::c_int,
                );
            } else {
                c = (*mem.offset((p + 4 as int32_t) as isize)).b16.s1 as int32_t;
                a = 0 as ::core::ffi::c_int as int32_t;
                loop {
                    g = get_ot_math_variant(
                        f as ::core::ffi::c_int,
                        c as ::core::ffi::c_int,
                        a as ::core::ffi::c_int,
                        &raw mut w2,
                        1 as ::core::ffi::c_int,
                    ) as int32_t;
                    if w2 > 0 as scaled_t && w2 <= w {
                        (*mem.offset((p + 4 as int32_t) as isize)).b16.s1 = g as uint16_t;
                        measure_native_glyph(
                            mem.offset(p as isize) as *mut memory_word as *mut ::core::ffi::c_void,
                            1 as ::core::ffi::c_int,
                        );
                        a += 1;
                    }
                    if w2 < 0 as scaled_t || w2 >= w {
                        break;
                    }
                }
                if w2 < 0 as scaled_t {
                    ot_assembly_ptr = get_ot_assembly_ptr(
                        f as ::core::ffi::c_int,
                        c as ::core::ffi::c_int,
                        1 as ::core::ffi::c_int,
                    );
                    if !ot_assembly_ptr.is_null() {
                        free_node(p, GLYPH_NODE_SIZE as int32_t);
                        p = build_opentype_assembly(
                            f,
                            ot_assembly_ptr,
                            w,
                            1 as ::core::ffi::c_int != 0,
                        );
                        (*mem.offset((y + 5 as int32_t) as isize)).b32.s1 = p;
                    }
                } else {
                    measure_native_glyph(
                        mem.offset(p as isize) as *mut memory_word as *mut ::core::ffi::c_void,
                        1 as ::core::ffi::c_int,
                    );
                }
            }
            (*mem.offset((y + 1 as int32_t) as isize)).b32.s1 =
                (*mem.offset((p + 1 as int32_t) as isize)).b32.s1;
            (*mem.offset((y + 3 as int32_t) as isize)).b32.s1 =
                (*mem.offset((p + 3 as int32_t) as isize)).b32.s1;
            (*mem.offset((y + 2 as int32_t) as isize)).b32.s1 =
                (*mem.offset((p + 2 as int32_t) as isize)).b32.s1;
            if (*mem.offset(q as isize)).b16.s0 as ::core::ffi::c_int == BOTTOM_ACC
                || (*mem.offset(q as isize)).b16.s0 as ::core::ffi::c_int
                    == BOTTOM_ACC + 1 as ::core::ffi::c_int
            {
                if (*mem.offset((y + 3 as int32_t) as isize)).b32.s1 < 0 as int32_t {
                    (*mem.offset((y + 3 as int32_t) as isize)).b32.s1 =
                        0 as ::core::ffi::c_int as int32_t;
                }
            } else if (*mem.offset((y + 2 as int32_t) as isize)).b32.s1 < 0 as int32_t {
                (*mem.offset((y + 2 as int32_t) as isize)).b32.s1 =
                    0 as ::core::ffi::c_int as int32_t;
            }
            if p != TEX_NULL as int32_t
                && !is_char_node(p)
                && (*mem.offset(p as isize)).b16.s1 as ::core::ffi::c_int == WHATSIT_NODE
                && (*mem.offset(p as isize)).b16.s0 as ::core::ffi::c_int == GLYPH_NODE
            {
                sa = get_ot_math_accent_pos(
                    f as ::core::ffi::c_int,
                    (*mem.offset((p + 4 as int32_t) as isize)).b16.s1 as ::core::ffi::c_int,
                ) as scaled_t;
                if sa == TEX_INFINITY as scaled_t {
                    sa = half((*mem.offset((y + 1 as int32_t) as isize)).b32.s1) as scaled_t;
                }
            } else {
                sa = half((*mem.offset((y + 1 as int32_t) as isize)).b32.s1) as scaled_t;
            }
            if (*mem.offset(q as isize)).b16.s0 as ::core::ffi::c_int == BOTTOM_ACC
                || (*mem.offset(q as isize)).b16.s0 as ::core::ffi::c_int
                    == BOTTOM_ACC + 1 as ::core::ffi::c_int
                || s == TEX_INFINITY as scaled_t
            {
                s = half(w as int32_t) as scaled_t;
            }
            (*mem.offset((y + 4 as int32_t) as isize)).b32.s1 = (s - sa) as int32_t;
        } else {
            (*mem.offset((y + 4 as int32_t) as isize)).b32.s1 =
                (s + half(w as int32_t - (*mem.offset((y + 1 as int32_t) as isize)).b32.s1)
                    as scaled_t) as int32_t;
        }
        (*mem.offset((y + 1 as int32_t) as isize)).b32.s1 = 0 as ::core::ffi::c_int as int32_t;
        if (*mem.offset(q as isize)).b16.s0 as ::core::ffi::c_int == BOTTOM_ACC
            || (*mem.offset(q as isize)).b16.s0 as ::core::ffi::c_int
                == BOTTOM_ACC + 1 as ::core::ffi::c_int
        {
            (*mem.offset(x as isize)).b32.s1 = y;
            y = vpackage(
                x,
                0 as scaled_t,
                ADDITIONAL as small_number,
                MAX_HALFWORD as scaled_t,
            );
            (*mem.offset((y + 4 as int32_t) as isize)).b32.s1 =
                -(h - (*mem.offset((y + 3 as int32_t) as isize)).b32.s1 as scaled_t);
        } else {
            p = new_kern(-delta);
            (*mem.offset(p as isize)).b32.s1 = x;
            (*mem.offset(y as isize)).b32.s1 = p;
            y = vpackage(
                y,
                0 as scaled_t,
                ADDITIONAL as small_number,
                MAX_HALFWORD as scaled_t,
            );
            if (*mem.offset((y + 3 as int32_t) as isize)).b32.s1 < h {
                p = new_kern(h - (*mem.offset((y + 3 as int32_t) as isize)).b32.s1 as scaled_t);
                (*mem.offset(p as isize)).b32.s1 =
                    (*mem.offset((y + 5 as int32_t) as isize)).b32.s1;
                (*mem.offset((y + 5 as int32_t) as isize)).b32.s1 = p;
                (*mem.offset((y + 3 as int32_t) as isize)).b32.s1 = h as int32_t;
            }
        }
        (*mem.offset((y + 1 as int32_t) as isize)).b32.s1 =
            (*mem.offset((x + 1 as int32_t) as isize)).b32.s1;
        (*mem.offset((q + 1 as int32_t) as isize)).b32.s0 = y;
        (*mem.offset((q + 1 as int32_t) as isize)).b32.s1 = SUB_BOX as int32_t;
    }
    free_ot_assembly(ot_assembly_ptr as *mut GlyphAssembly);
}
unsafe extern "C" fn make_fraction(mut q: int32_t) {
    let mut p: int32_t = 0;
    let mut v: int32_t = 0;
    let mut x: int32_t = 0;
    let mut y: int32_t = 0;
    let mut z: int32_t = 0;
    let mut delta: scaled_t = 0;
    let mut delta1: scaled_t = 0;
    let mut delta2: scaled_t = 0;
    let mut shift_up: scaled_t = 0;
    let mut shift_down: scaled_t = 0;
    let mut clr: scaled_t = 0;
    if (*mem.offset((q + 1 as int32_t) as isize)).b32.s1 == DEFAULT_CODE as int32_t {
        (*mem.offset((q + 1 as int32_t) as isize)).b32.s1 = default_rule_thickness() as int32_t;
    }
    x = clean_box(
        q + 2 as int32_t,
        (cur_style as ::core::ffi::c_int + 2 as ::core::ffi::c_int
            - 2 as ::core::ffi::c_int * (cur_style as ::core::ffi::c_int / 6 as ::core::ffi::c_int))
            as small_number,
    );
    z = clean_box(
        q + 3 as int32_t,
        (2 as ::core::ffi::c_int * (cur_style as ::core::ffi::c_int / 2 as ::core::ffi::c_int)
            + 3 as ::core::ffi::c_int
            - 2 as ::core::ffi::c_int * (cur_style as ::core::ffi::c_int / 6 as ::core::ffi::c_int))
            as small_number,
    );
    if (*mem.offset((x + 1 as int32_t) as isize)).b32.s1
        < (*mem.offset((z + 1 as int32_t) as isize)).b32.s1
    {
        x = rebox(
            x,
            (*mem.offset((z + 1 as int32_t) as isize)).b32.s1 as scaled_t,
        );
    } else {
        z = rebox(
            z,
            (*mem.offset((x + 1 as int32_t) as isize)).b32.s1 as scaled_t,
        );
    }
    if (cur_style as ::core::ffi::c_int) < TEXT_STYLE {
        shift_up = num1(cur_size);
        shift_down = denom1(cur_size);
    } else {
        shift_down = denom2(cur_size);
        if (*mem.offset((q + 1 as int32_t) as isize)).b32.s1 != 0 as int32_t {
            shift_up = num2(cur_size);
        } else {
            shift_up = num3(cur_size);
        }
    }
    if (*mem.offset((q + 1 as int32_t) as isize)).b32.s1 == 0 as int32_t {
        if *font_area.offset(cur_f as isize) as ::core::ffi::c_uint == OTGR_FONT_FLAG
            && isOpenTypeMathFont(*font_layout_engine.offset(cur_f as isize) as XeTeXLayoutEngine)
                as ::core::ffi::c_int
                != 0
        {
            if (cur_style as ::core::ffi::c_int) < TEXT_STYLE {
                clr = get_ot_math_constant(cur_f as ::core::ffi::c_int, STACKDISPLAYSTYLEGAPMIN)
                    as scaled_t;
            } else {
                clr = get_ot_math_constant(cur_f as ::core::ffi::c_int, STACKGAPMIN) as scaled_t;
            }
        } else if (cur_style as ::core::ffi::c_int) < TEXT_STYLE {
            clr = 7 as scaled_t * default_rule_thickness();
        } else {
            clr = 3 as scaled_t * default_rule_thickness();
        }
        delta = half(
            clr as int32_t
                - (shift_up as int32_t
                    - (*mem.offset((x + 2 as int32_t) as isize)).b32.s1
                    - ((*mem.offset((z + 3 as int32_t) as isize)).b32.s1 - shift_down as int32_t)),
        ) as scaled_t;
        if delta > 0 as scaled_t {
            shift_up = shift_up + delta;
            shift_down = shift_down + delta;
        }
    } else {
        if *font_area.offset(cur_f as isize) as ::core::ffi::c_uint == OTGR_FONT_FLAG
            && isOpenTypeMathFont(*font_layout_engine.offset(cur_f as isize) as XeTeXLayoutEngine)
                as ::core::ffi::c_int
                != 0
        {
            delta = half((*mem.offset((q + 1 as int32_t) as isize)).b32.s1) as scaled_t;
            if (cur_style as ::core::ffi::c_int) < TEXT_STYLE {
                clr = get_ot_math_constant(
                    cur_f as ::core::ffi::c_int,
                    FRACTIONNUMDISPLAYSTYLEGAPMIN,
                ) as scaled_t;
            } else {
                clr = get_ot_math_constant(cur_f as ::core::ffi::c_int, FRACTIONNUMERATORGAPMIN)
                    as scaled_t;
            }
            delta1 = clr
                - (shift_up
                    - (*mem.offset((x + 2 as int32_t) as isize)).b32.s1 as scaled_t
                    - (axis_height(cur_size) + delta));
            if (cur_style as ::core::ffi::c_int) < TEXT_STYLE {
                clr = get_ot_math_constant(
                    cur_f as ::core::ffi::c_int,
                    FRACTIONDENOMDISPLAYSTYLEGAPMIN,
                ) as scaled_t;
            } else {
                clr = get_ot_math_constant(cur_f as ::core::ffi::c_int, FRACTIONDENOMINATORGAPMIN)
                    as scaled_t;
            }
            delta2 = clr
                - (axis_height(cur_size)
                    - delta
                    - ((*mem.offset((z + 3 as int32_t) as isize)).b32.s1 as scaled_t - shift_down));
        } else {
            if (cur_style as ::core::ffi::c_int) < TEXT_STYLE {
                clr =
                    (3 as int32_t * (*mem.offset((q + 1 as int32_t) as isize)).b32.s1) as scaled_t;
            } else {
                clr = (*mem.offset((q + 1 as int32_t) as isize)).b32.s1 as scaled_t;
            }
            delta = half((*mem.offset((q + 1 as int32_t) as isize)).b32.s1) as scaled_t;
            delta1 = clr
                - (shift_up
                    - (*mem.offset((x + 2 as int32_t) as isize)).b32.s1 as scaled_t
                    - (axis_height(cur_size) + delta));
            delta2 = clr
                - (axis_height(cur_size)
                    - delta
                    - ((*mem.offset((z + 3 as int32_t) as isize)).b32.s1 as scaled_t - shift_down));
        }
        if delta1 > 0 as scaled_t {
            shift_up = shift_up + delta1;
        }
        if delta2 > 0 as scaled_t {
            shift_down = shift_down + delta2;
        }
    }
    v = new_null_box();
    (*mem.offset(v as isize)).b16.s1 = VLIST_NODE as uint16_t;
    (*mem.offset((v + 3 as int32_t) as isize)).b32.s1 =
        (shift_up + (*mem.offset((x + 3 as int32_t) as isize)).b32.s1 as scaled_t) as int32_t;
    (*mem.offset((v + 2 as int32_t) as isize)).b32.s1 =
        (*mem.offset((z + 2 as int32_t) as isize)).b32.s1 + shift_down as int32_t;
    (*mem.offset((v + 1 as int32_t) as isize)).b32.s1 =
        (*mem.offset((x + 1 as int32_t) as isize)).b32.s1;
    if (*mem.offset((q + 1 as int32_t) as isize)).b32.s1 == 0 as int32_t {
        p = new_kern(
            shift_up
                - (*mem.offset((x + 2 as int32_t) as isize)).b32.s1 as scaled_t
                - ((*mem.offset((z + 3 as int32_t) as isize)).b32.s1 as scaled_t - shift_down),
        );
        (*mem.offset(p as isize)).b32.s1 = z;
    } else {
        y = fraction_rule((*mem.offset((q + 1 as int32_t) as isize)).b32.s1 as scaled_t);
        p = new_kern(
            axis_height(cur_size)
                - delta
                - ((*mem.offset((z + 3 as int32_t) as isize)).b32.s1 as scaled_t - shift_down),
        );
        (*mem.offset(y as isize)).b32.s1 = p;
        (*mem.offset(p as isize)).b32.s1 = z;
        p = new_kern(
            shift_up
                - (*mem.offset((x + 2 as int32_t) as isize)).b32.s1 as scaled_t
                - (axis_height(cur_size) + delta),
        );
        (*mem.offset(p as isize)).b32.s1 = y;
    }
    (*mem.offset(x as isize)).b32.s1 = p;
    (*mem.offset((v + 5 as int32_t) as isize)).b32.s1 = x;
    if (cur_style as ::core::ffi::c_int) < TEXT_STYLE {
        delta = delim1(cur_size);
    } else {
        delta = delim2(cur_size);
    }
    x = var_delimiter(q + 4 as int32_t, cur_size, delta);
    (*mem.offset(x as isize)).b32.s1 = v;
    z = var_delimiter(q + 5 as int32_t, cur_size, delta);
    (*mem.offset(v as isize)).b32.s1 = z;
    (*mem.offset((q + 1 as int32_t) as isize)).b32.s1 =
        hpack(x, 0 as scaled_t, ADDITIONAL as small_number);
}
unsafe extern "C" fn make_op(mut q: int32_t) -> scaled_t {
    let mut delta: scaled_t = 0;
    let mut p: int32_t = 0;
    let mut v: int32_t = 0;
    let mut x: int32_t = 0;
    let mut y: int32_t = 0;
    let mut z: int32_t = 0;
    let mut c: uint16_t = 0;
    let mut i: b16x4 = b16x4_le_t {
        s0: 0,
        s1: 0,
        s2: 0,
        s3: 0,
    };
    let mut shift_up: scaled_t = 0;
    let mut shift_down: scaled_t = 0;
    let mut h1: scaled_t = 0;
    let mut h2: scaled_t = 0;
    let mut n: int32_t = 0;
    let mut g: int32_t = 0;
    let mut ot_assembly_ptr: *mut ::core::ffi::c_void =
        ::core::ptr::null_mut::<::core::ffi::c_void>();
    let mut save_f: internal_font_number = 0;
    if (*mem.offset(q as isize)).b16.s0 as ::core::ffi::c_int == NORMAL
        && (cur_style as ::core::ffi::c_int) < TEXT_STYLE
    {
        (*mem.offset(q as isize)).b16.s0 = LIMITS as uint16_t;
    }
    delta = 0 as ::core::ffi::c_int as scaled_t;
    ot_assembly_ptr = NULL;
    if (*mem.offset((q + 1 as int32_t) as isize)).b32.s1 == MATH_CHAR as int32_t {
        fetch(q + 1 as int32_t);
        if !(*font_area.offset(cur_f as isize) as ::core::ffi::c_uint == OTGR_FONT_FLAG
            && usingOpenType(*font_layout_engine.offset(cur_f as isize) as XeTeXLayoutEngine)
                as ::core::ffi::c_int
                != 0)
        {
            if (cur_style as ::core::ffi::c_int) < TEXT_STYLE
                && cur_i.s1 as ::core::ffi::c_int % 4 as ::core::ffi::c_int == LIST_TAG
            {
                c = cur_i.s0;
                i = (*font_info
                    .offset((*char_base.offset(cur_f as isize) + c as int32_t) as isize))
                .b16;
                if i.s3 as ::core::ffi::c_int > 0 as ::core::ffi::c_int {
                    cur_c = c as int32_t;
                    cur_i = i;
                    (*mem.offset((q + 1 as int32_t) as isize)).b16.s0 = c;
                }
            }
            delta = (*font_info.offset(
                (*italic_base.offset(cur_f as isize) + cur_i.s1 as int32_t / 4 as int32_t) as isize,
            ))
            .b32
            .s1 as scaled_t;
        }
        x = clean_box(q + 1 as int32_t, cur_style);
        if *font_area.offset(cur_f as isize) as ::core::ffi::c_uint == OTGR_FONT_FLAG
            && isOpenTypeMathFont(*font_layout_engine.offset(cur_f as isize) as XeTeXLayoutEngine)
                as ::core::ffi::c_int
                != 0
        {
            p = (*mem.offset((x + 5 as int32_t) as isize)).b32.s1;
            if p != TEX_NULL as int32_t
                && !is_char_node(p)
                && (*mem.offset(p as isize)).b16.s1 as ::core::ffi::c_int == WHATSIT_NODE
                && (*mem.offset(p as isize)).b16.s0 as ::core::ffi::c_int == GLYPH_NODE
            {
                let mut current_block_41: u64;
                if (cur_style as ::core::ffi::c_int) < TEXT_STYLE {
                    h1 = get_ot_math_constant(cur_f as ::core::ffi::c_int, DISPLAYOPERATORMINHEIGHT)
                        as scaled_t;
                    if (h1 as ::core::ffi::c_double)
                        < (((*mem.offset((p + 3 as int32_t) as isize)).b32.s1
                            + (*mem.offset((p + 2 as int32_t) as isize)).b32.s1)
                            * 5 as int32_t) as ::core::ffi::c_double
                            / 4 as ::core::ffi::c_int as ::core::ffi::c_double
                    {
                        h1 = ((((*mem.offset((p + 3 as int32_t) as isize)).b32.s1
                            + (*mem.offset((p + 2 as int32_t) as isize)).b32.s1)
                            * 5 as int32_t) as ::core::ffi::c_double
                            / 4 as ::core::ffi::c_int as ::core::ffi::c_double)
                            as scaled_t;
                    }
                    c = (*mem.offset((p + 4 as int32_t) as isize)).b16.s1;
                    n = 0 as ::core::ffi::c_int as int32_t;
                    loop {
                        g = get_ot_math_variant(
                            cur_f as ::core::ffi::c_int,
                            c as ::core::ffi::c_int,
                            n as ::core::ffi::c_int,
                            &raw mut h2,
                            0 as ::core::ffi::c_int,
                        ) as int32_t;
                        if h2 > 0 as scaled_t {
                            (*mem.offset((p + 4 as int32_t) as isize)).b16.s1 = g as uint16_t;
                            measure_native_glyph(
                                mem.offset(p as isize) as *mut memory_word
                                    as *mut ::core::ffi::c_void,
                                1 as ::core::ffi::c_int,
                            );
                        }
                        n += 1;
                        if h2 < 0 as scaled_t || h2 >= h1 {
                            break;
                        }
                    }
                    if h2 < 0 as scaled_t {
                        ot_assembly_ptr = get_ot_assembly_ptr(
                            cur_f as ::core::ffi::c_int,
                            c as ::core::ffi::c_int,
                            0 as ::core::ffi::c_int,
                        );
                        if !ot_assembly_ptr.is_null() {
                            free_node(p, GLYPH_NODE_SIZE as int32_t);
                            p = build_opentype_assembly(
                                cur_f,
                                ot_assembly_ptr,
                                h1,
                                0 as ::core::ffi::c_int != 0,
                            );
                            (*mem.offset((x + 5 as int32_t) as isize)).b32.s1 = p;
                            delta = 0 as ::core::ffi::c_int as scaled_t;
                            current_block_41 = 5431329424827657200;
                        } else {
                            current_block_41 = 1836292691772056875;
                        }
                    } else {
                        measure_native_glyph(
                            mem.offset(p as isize) as *mut memory_word as *mut ::core::ffi::c_void,
                            1 as ::core::ffi::c_int,
                        );
                        current_block_41 = 1836292691772056875;
                    }
                } else {
                    current_block_41 = 1836292691772056875;
                }
                match current_block_41 {
                    1836292691772056875 => {
                        delta = get_ot_math_ital_corr(
                            cur_f as ::core::ffi::c_int,
                            (*mem.offset((p + 4 as int32_t) as isize)).b16.s1 as ::core::ffi::c_int,
                        ) as scaled_t;
                    }
                    _ => {}
                }
                (*mem.offset((x + 1 as int32_t) as isize)).b32.s1 =
                    (*mem.offset((p + 1 as int32_t) as isize)).b32.s1;
                (*mem.offset((x + 3 as int32_t) as isize)).b32.s1 =
                    (*mem.offset((p + 3 as int32_t) as isize)).b32.s1;
                (*mem.offset((x + 2 as int32_t) as isize)).b32.s1 =
                    (*mem.offset((p + 2 as int32_t) as isize)).b32.s1;
            }
        }
        if (*mem.offset((q + 3 as int32_t) as isize)).b32.s1 != EMPTY as int32_t
            && (*mem.offset(q as isize)).b16.s0 as ::core::ffi::c_int != LIMITS
        {
            (*mem.offset((x + 1 as int32_t) as isize)).b32.s1 =
                (*mem.offset((x + 1 as int32_t) as isize)).b32.s1 - delta as int32_t;
        }
        (*mem.offset((x + 4 as int32_t) as isize)).b32.s1 = half(
            (*mem.offset((x + 3 as int32_t) as isize)).b32.s1
                - (*mem.offset((x + 2 as int32_t) as isize)).b32.s1,
        ) - axis_height(cur_size) as int32_t;
        (*mem.offset((q + 1 as int32_t) as isize)).b32.s1 = SUB_BOX as int32_t;
        (*mem.offset((q + 1 as int32_t) as isize)).b32.s0 = x;
    }
    save_f = cur_f;
    if (*mem.offset(q as isize)).b16.s0 as ::core::ffi::c_int == LIMITS {
        x = clean_box(
            q + 2 as int32_t,
            (2 as ::core::ffi::c_int * (cur_style as ::core::ffi::c_int / 4 as ::core::ffi::c_int)
                + 4 as ::core::ffi::c_int
                + cur_style as ::core::ffi::c_int % 2 as ::core::ffi::c_int)
                as small_number,
        );
        y = clean_box(q + 1 as int32_t, cur_style);
        z = clean_box(
            q + 3 as int32_t,
            (2 as ::core::ffi::c_int * (cur_style as ::core::ffi::c_int / 4 as ::core::ffi::c_int)
                + 5 as ::core::ffi::c_int) as small_number,
        );
        v = new_null_box();
        (*mem.offset(v as isize)).b16.s1 = VLIST_NODE as uint16_t;
        (*mem.offset((v + 1 as int32_t) as isize)).b32.s1 =
            (*mem.offset((y + 1 as int32_t) as isize)).b32.s1;
        if (*mem.offset((x + 1 as int32_t) as isize)).b32.s1
            > (*mem.offset((v + 1 as int32_t) as isize)).b32.s1
        {
            (*mem.offset((v + 1 as int32_t) as isize)).b32.s1 =
                (*mem.offset((x + 1 as int32_t) as isize)).b32.s1;
        }
        if (*mem.offset((z + 1 as int32_t) as isize)).b32.s1
            > (*mem.offset((v + 1 as int32_t) as isize)).b32.s1
        {
            (*mem.offset((v + 1 as int32_t) as isize)).b32.s1 =
                (*mem.offset((z + 1 as int32_t) as isize)).b32.s1;
        }
        x = rebox(
            x,
            (*mem.offset((v + 1 as int32_t) as isize)).b32.s1 as scaled_t,
        );
        y = rebox(
            y,
            (*mem.offset((v + 1 as int32_t) as isize)).b32.s1 as scaled_t,
        );
        z = rebox(
            z,
            (*mem.offset((v + 1 as int32_t) as isize)).b32.s1 as scaled_t,
        );
        (*mem.offset((x + 4 as int32_t) as isize)).b32.s1 = half(delta as int32_t);
        (*mem.offset((z + 4 as int32_t) as isize)).b32.s1 =
            -(*mem.offset((x + 4 as int32_t) as isize)).b32.s1;
        (*mem.offset((v + 3 as int32_t) as isize)).b32.s1 =
            (*mem.offset((y + 3 as int32_t) as isize)).b32.s1;
        (*mem.offset((v + 2 as int32_t) as isize)).b32.s1 =
            (*mem.offset((y + 2 as int32_t) as isize)).b32.s1;
        cur_f = save_f;
        if (*mem.offset((q + 2 as int32_t) as isize)).b32.s1 == EMPTY as int32_t {
            free_node(x, BOX_NODE_SIZE as int32_t);
            (*mem.offset((v + 5 as int32_t) as isize)).b32.s1 = y;
        } else {
            shift_up =
                big_op_spacing3() - (*mem.offset((x + 2 as int32_t) as isize)).b32.s1 as scaled_t;
            if shift_up < big_op_spacing1() {
                shift_up = big_op_spacing1();
            }
            p = new_kern(shift_up);
            (*mem.offset(p as isize)).b32.s1 = y;
            (*mem.offset(x as isize)).b32.s1 = p;
            p = new_kern(big_op_spacing5());
            (*mem.offset(p as isize)).b32.s1 = x;
            (*mem.offset((v + 5 as int32_t) as isize)).b32.s1 = p;
            (*mem.offset((v + 3 as int32_t) as isize)).b32.s1 =
                (*mem.offset((v + 3 as int32_t) as isize)).b32.s1
                    + big_op_spacing5() as int32_t
                    + (*mem.offset((x + 3 as int32_t) as isize)).b32.s1
                    + (*mem.offset((x + 2 as int32_t) as isize)).b32.s1
                    + shift_up as int32_t;
        }
        if (*mem.offset((q + 3 as int32_t) as isize)).b32.s1 == EMPTY as int32_t {
            free_node(z, BOX_NODE_SIZE as int32_t);
        } else {
            shift_down =
                big_op_spacing4() - (*mem.offset((z + 3 as int32_t) as isize)).b32.s1 as scaled_t;
            if shift_down < big_op_spacing2() {
                shift_down = big_op_spacing2();
            }
            p = new_kern(shift_down);
            (*mem.offset(y as isize)).b32.s1 = p;
            (*mem.offset(p as isize)).b32.s1 = z;
            p = new_kern(big_op_spacing5());
            (*mem.offset(z as isize)).b32.s1 = p;
            (*mem.offset((v + 2 as int32_t) as isize)).b32.s1 =
                (*mem.offset((v + 2 as int32_t) as isize)).b32.s1
                    + big_op_spacing5() as int32_t
                    + (*mem.offset((z + 3 as int32_t) as isize)).b32.s1
                    + (*mem.offset((z + 2 as int32_t) as isize)).b32.s1
                    + shift_down as int32_t;
        }
        (*mem.offset((q + 1 as int32_t) as isize)).b32.s1 = v;
    }
    free_ot_assembly(ot_assembly_ptr as *mut GlyphAssembly);
    return delta;
}
unsafe extern "C" fn make_ord(mut q: int32_t) {
    let mut a: int32_t = 0;
    let mut p: int32_t = 0;
    let mut r: int32_t = 0;
    while (*mem.offset((q + 3 as int32_t) as isize)).b32.s1 == EMPTY as int32_t {
        if !((*mem.offset((q + 2 as int32_t) as isize)).b32.s1 == EMPTY as int32_t) {
            break;
        }
        if !((*mem.offset((q + 1 as int32_t) as isize)).b32.s1 == MATH_CHAR as int32_t) {
            break;
        }
        p = (*mem.offset(q as isize)).b32.s1;
        if !(p != TEX_NULL as int32_t) {
            break;
        }
        if !((*mem.offset(p as isize)).b16.s1 as ::core::ffi::c_int >= ORD_NOAD
            && (*mem.offset(p as isize)).b16.s1 as ::core::ffi::c_int <= PUNCT_NOAD)
        {
            break;
        }
        if !((*mem.offset((p + 1 as int32_t) as isize)).b32.s1 == MATH_CHAR as int32_t) {
            break;
        }
        if !((*mem.offset((p + 1 as int32_t) as isize)).b16.s1 as ::core::ffi::c_int
            % 256 as ::core::ffi::c_int
            == (*mem.offset((q + 1 as int32_t) as isize)).b16.s1 as ::core::ffi::c_int
                % 256 as ::core::ffi::c_int)
        {
            break;
        }
        (*mem.offset((q + 1 as int32_t) as isize)).b32.s1 = MATH_TEXT_CHAR as int32_t;
        fetch(q + 1 as int32_t);
        if !(cur_i.s1 as ::core::ffi::c_int % 4 as ::core::ffi::c_int == LIG_TAG) {
            break;
        }
        a = *lig_kern_base.offset(cur_f as isize) + cur_i.s0 as int32_t;
        cur_c = (*mem.offset((p + 1 as int32_t) as isize)).b16.s0 as int32_t;
        cur_i = (*font_info.offset(a as isize)).b16;
        if cur_i.s3 as ::core::ffi::c_int > 128 as ::core::ffi::c_int {
            a = ((*lig_kern_base.offset(cur_f as isize)
                + 256 as int32_t * cur_i.s1 as int32_t
                + cur_i.s0 as int32_t) as ::core::ffi::c_long
                + 32768 as ::core::ffi::c_long
                - (256 as ::core::ffi::c_int * 128 as ::core::ffi::c_int) as ::core::ffi::c_long)
                as int32_t;
            cur_i = (*font_info.offset(a as isize)).b16;
        }
        loop {
            if cur_i.s2 as int32_t == cur_c {
                if cur_i.s3 as ::core::ffi::c_int <= 128 as ::core::ffi::c_int {
                    if cur_i.s1 as ::core::ffi::c_int >= 128 as ::core::ffi::c_int {
                        p = new_kern(
                            (*font_info.offset(
                                (*kern_base.offset(cur_f as isize)
                                    + 256 as int32_t * cur_i.s1 as int32_t
                                    + cur_i.s0 as int32_t) as isize,
                            ))
                            .b32
                            .s1 as scaled_t,
                        );
                        (*mem.offset(p as isize)).b32.s1 = (*mem.offset(q as isize)).b32.s1;
                        (*mem.offset(q as isize)).b32.s1 = p;
                        return;
                    } else {
                        match cur_i.s1 as ::core::ffi::c_int {
                            1 | 5 => {
                                (*mem.offset((q + 1 as int32_t) as isize)).b16.s0 = cur_i.s0;
                            }
                            2 | 6 => {
                                (*mem.offset((p + 1 as int32_t) as isize)).b16.s0 = cur_i.s0;
                            }
                            3 | 7 | 11 => {
                                r = new_noad();
                                (*mem.offset((r + 1 as int32_t) as isize)).b16.s0 = cur_i.s0;
                                (*mem.offset((r + 1 as int32_t) as isize)).b16.s1 =
                                    ((*mem.offset((q + 1 as int32_t) as isize)).b16.s1
                                        as ::core::ffi::c_int
                                        % 256 as ::core::ffi::c_int)
                                        as uint16_t;
                                (*mem.offset(q as isize)).b32.s1 = r;
                                (*mem.offset(r as isize)).b32.s1 = p;
                                if (cur_i.s1 as ::core::ffi::c_int) < 11 as ::core::ffi::c_int {
                                    (*mem.offset((r + 1 as int32_t) as isize)).b32.s1 =
                                        MATH_CHAR as int32_t;
                                } else {
                                    (*mem.offset((r + 1 as int32_t) as isize)).b32.s1 =
                                        MATH_TEXT_CHAR as int32_t;
                                }
                            }
                            _ => {
                                (*mem.offset(q as isize)).b32.s1 = (*mem.offset(p as isize)).b32.s1;
                                (*mem.offset((q + 1 as int32_t) as isize)).b16.s0 = cur_i.s0;
                                *mem.offset((q + 3 as int32_t) as isize) =
                                    *mem.offset((p + 3 as int32_t) as isize);
                                *mem.offset((q + 2 as int32_t) as isize) =
                                    *mem.offset((p + 2 as int32_t) as isize);
                                free_node(p, NOAD_SIZE as int32_t);
                            }
                        }
                        if cur_i.s1 as ::core::ffi::c_int > 3 as ::core::ffi::c_int {
                            return;
                        }
                        (*mem.offset((q + 1 as int32_t) as isize)).b32.s1 = MATH_CHAR as int32_t;
                        break;
                    }
                }
            }
            if cur_i.s3 as ::core::ffi::c_int >= 128 as ::core::ffi::c_int {
                return;
            }
            a = a + cur_i.s3 as int32_t + 1 as int32_t;
            cur_i = (*font_info.offset(a as isize)).b16;
        }
    }
}
unsafe extern "C" fn attach_hkern_to_new_hlist(mut q: int32_t, mut delta: scaled_t) -> int32_t {
    let mut y: int32_t = 0;
    let mut z: int32_t = 0;
    z = new_kern(delta);
    if (*mem.offset((q + 1 as int32_t) as isize)).b32.s1 == TEX_NULL as int32_t {
        (*mem.offset((q + 1 as int32_t) as isize)).b32.s1 = z;
    } else {
        y = (*mem.offset((q + 1 as int32_t) as isize)).b32.s1;
        while (*mem.offset(y as isize)).b32.s1 != TEX_NULL as int32_t {
            y = (*mem.offset(y as isize)).b32.s1;
        }
        (*mem.offset(y as isize)).b32.s1 = z;
    }
    return (*mem.offset((q + 1 as int32_t) as isize)).b32.s1;
}
unsafe extern "C" fn make_scripts(mut q: int32_t, mut delta: scaled_t) {
    let mut p: int32_t = 0;
    let mut x: int32_t = 0;
    let mut y: int32_t = 0;
    let mut z: int32_t = 0;
    let mut shift_up: scaled_t = 0;
    let mut shift_down: scaled_t = 0;
    let mut clr: scaled_t = 0;
    let mut sub_kern: scaled_t = 0;
    let mut sup_kern: scaled_t = 0;
    let mut script_c: int32_t = 0;
    let mut script_g: uint16_t = 0;
    let mut script_f: internal_font_number = 0;
    let mut sup_g: uint16_t = 0;
    let mut sup_f: internal_font_number = 0;
    let mut sub_g: uint16_t = 0;
    let mut sub_f: internal_font_number = 0;
    let mut t: int32_t = 0;
    let mut save_f: internal_font_number = 0;
    let mut script_head: int32_t = 0;
    let mut script_ptr: int32_t = 0;
    let mut saved_math_style: small_number = 0;
    let mut this_math_style: small_number = 0;
    p = (*mem.offset((q + 1 as int32_t) as isize)).b32.s1;
    script_c = TEX_NULL as int32_t;
    script_g = 0 as uint16_t;
    script_f = 0 as ::core::ffi::c_int as internal_font_number;
    sup_kern = 0 as ::core::ffi::c_int as scaled_t;
    sub_kern = 0 as ::core::ffi::c_int as scaled_t;
    if is_char_node(p) as ::core::ffi::c_int != 0
        || p != TEX_NULL as int32_t
            && !is_char_node(p)
            && (*mem.offset(p as isize)).b16.s1 as ::core::ffi::c_int == WHATSIT_NODE
            && (*mem.offset(p as isize)).b16.s0 as ::core::ffi::c_int == GLYPH_NODE
    {
        shift_up = 0 as ::core::ffi::c_int as scaled_t;
        shift_down = 0 as ::core::ffi::c_int as scaled_t;
    } else {
        z = hpack(p, 0 as scaled_t, ADDITIONAL as small_number);
        if (cur_style as ::core::ffi::c_int) < SCRIPT_STYLE {
            t = SCRIPT_SIZE as int32_t;
        } else {
            t = SCRIPT_SCRIPT_SIZE as int32_t;
        }
        shift_up = ((*mem.offset((z + 3 as int32_t) as isize)).b32.s1 - sup_drop(t) as int32_t)
            as scaled_t;
        shift_down = ((*mem.offset((z + 2 as int32_t) as isize)).b32.s1 + sub_drop(t) as int32_t)
            as scaled_t;
        free_node(z, BOX_NODE_SIZE as int32_t);
    }
    if (*mem.offset((q + 2 as int32_t) as isize)).b32.s1 == EMPTY as int32_t {
        script_head = q + 3 as int32_t;
        script_c = TEX_NULL as int32_t;
        script_g = 0 as uint16_t;
        script_f = 0 as ::core::ffi::c_int as internal_font_number;
        this_math_style = (2 as ::core::ffi::c_int
            * (cur_style as ::core::ffi::c_int / 4 as ::core::ffi::c_int)
            + 5 as ::core::ffi::c_int) as small_number;
        if (*mem.offset(script_head as isize)).b32.s1 == SUB_MLIST as int32_t {
            script_ptr = (*mem.offset(script_head as isize)).b32.s0;
            script_head = TEX_NULL as int32_t;
            while script_ptr >= 0 as int32_t && script_ptr <= mem_end {
                match (*mem.offset(script_ptr as isize)).b16.s1 as ::core::ffi::c_int {
                    KERN_NODE | GLUE_NODE | CHOICE_NODE => {}
                    STYLE_NODE => {
                        this_math_style = (*mem.offset(script_ptr as isize)).b16.s0 as small_number;
                    }
                    ORD_NOAD | OP_NOAD | BIN_NOAD | REL_NOAD | OPEN_NOAD | CLOSE_NOAD
                    | PUNCT_NOAD => {
                        script_head = script_ptr + 1 as int32_t;
                        script_ptr = TEX_NULL as int32_t;
                    }
                    _ => {
                        script_ptr = TEX_NULL as int32_t;
                    }
                }
                if script_ptr >= 0 as int32_t && script_ptr <= mem_end {
                    if (*mem.offset(script_ptr as isize)).b16.s1 as ::core::ffi::c_int
                        == CHOICE_NODE
                    {
                        match this_math_style as ::core::ffi::c_int / 2 as ::core::ffi::c_int {
                            0 => {
                                script_ptr =
                                    (*mem.offset((script_ptr + 1 as int32_t) as isize)).b32.s0;
                            }
                            1 => {
                                script_ptr =
                                    (*mem.offset((script_ptr + 1 as int32_t) as isize)).b32.s1;
                            }
                            2 => {
                                script_ptr =
                                    (*mem.offset((script_ptr + 2 as int32_t) as isize)).b32.s0;
                            }
                            3 => {
                                script_ptr =
                                    (*mem.offset((script_ptr + 2 as int32_t) as isize)).b32.s1;
                            }
                            _ => {}
                        }
                    } else {
                        script_ptr = (*mem.offset(script_ptr as isize)).b32.s1;
                    }
                }
            }
        }
        if script_head >= 0 as int32_t
            && script_head <= mem_end
            && (*mem.offset(script_head as isize)).b32.s1 == MATH_CHAR as int32_t
        {
            save_f = cur_f;
            saved_math_style = cur_style;
            cur_style = this_math_style;
            if (cur_style as ::core::ffi::c_int) < SCRIPT_STYLE {
                cur_size = TEXT_SIZE as int32_t;
            } else {
                cur_size = (SCRIPT_SIZE
                    * ((cur_style as ::core::ffi::c_int - 2 as ::core::ffi::c_int)
                        / 2 as ::core::ffi::c_int)) as int32_t;
            }
            cur_mu = x_over_n(math_quad(cur_size), 18 as int32_t);
            fetch(script_head);
            if *font_area.offset(cur_f as isize) as ::core::ffi::c_uint == OTGR_FONT_FLAG
                && isOpenTypeMathFont(
                    *font_layout_engine.offset(cur_f as isize) as XeTeXLayoutEngine
                ) as ::core::ffi::c_int
                    != 0
            {
                script_c = new_native_character(cur_f, cur_c as UnicodeScalar);
                script_g = real_get_native_glyph(
                    mem.offset(script_c as isize) as *mut memory_word as *mut ::core::ffi::c_void,
                    0 as ::core::ffi::c_uint,
                );
                script_f = cur_f;
            }
            cur_f = save_f;
            cur_style = saved_math_style;
            if (cur_style as ::core::ffi::c_int) < SCRIPT_STYLE {
                cur_size = TEXT_SIZE as int32_t;
            } else {
                cur_size = (SCRIPT_SIZE
                    * ((cur_style as ::core::ffi::c_int - 2 as ::core::ffi::c_int)
                        / 2 as ::core::ffi::c_int)) as int32_t;
            }
            cur_mu = x_over_n(math_quad(cur_size), 18 as int32_t);
        }
        sub_g = script_g;
        sub_f = script_f;
        save_f = cur_f;
        x = clean_box(
            q + 3 as int32_t,
            (2 as ::core::ffi::c_int * (cur_style as ::core::ffi::c_int / 4 as ::core::ffi::c_int)
                + 5 as ::core::ffi::c_int) as small_number,
        );
        cur_f = save_f;
        (*mem.offset((x + 1 as int32_t) as isize)).b32.s1 =
            (*mem.offset((x + 1 as int32_t) as isize)).b32.s1
                + (*eqtb.offset((DIMEN_BASE + DIMEN_PAR__script_space) as isize))
                    .b32
                    .s1;
        if shift_down < sub1(cur_size) {
            shift_down = sub1(cur_size);
        }
        if *font_area.offset(cur_f as isize) as ::core::ffi::c_uint == OTGR_FONT_FLAG
            && isOpenTypeMathFont(*font_layout_engine.offset(cur_f as isize) as XeTeXLayoutEngine)
                as ::core::ffi::c_int
                != 0
        {
            clr = ((*mem.offset((x + 3 as int32_t) as isize)).b32.s1
                - get_ot_math_constant(cur_f as ::core::ffi::c_int, SUBSCRIPTTOPMAX) as int32_t)
                as scaled_t;
        } else {
            clr = ((*mem.offset((x + 3 as int32_t) as isize)).b32.s1
                - abs(math_x_height(cur_size) as ::core::ffi::c_int * 4 as ::core::ffi::c_int)
                    as int32_t
                    / 5 as int32_t) as scaled_t;
        }
        if shift_down < clr {
            shift_down = clr;
        }
        (*mem.offset((x + 4 as int32_t) as isize)).b32.s1 = shift_down as int32_t;
        if *font_area.offset(cur_f as isize) as ::core::ffi::c_uint == OTGR_FONT_FLAG
            && isOpenTypeMathFont(*font_layout_engine.offset(cur_f as isize) as XeTeXLayoutEngine)
                as ::core::ffi::c_int
                != 0
        {
            if p != TEX_NULL as int32_t
                && !is_char_node(p)
                && (*mem.offset(p as isize)).b16.s1 as ::core::ffi::c_int == WHATSIT_NODE
                && (*mem.offset(p as isize)).b16.s0 as ::core::ffi::c_int == GLYPH_NODE
            {
                sub_kern = get_ot_math_kern(
                    (*mem.offset((p + 4 as int32_t) as isize)).b16.s2 as ::core::ffi::c_int,
                    (*mem.offset((p + 4 as int32_t) as isize)).b16.s1 as ::core::ffi::c_int,
                    sub_f as ::core::ffi::c_int,
                    sub_g as ::core::ffi::c_int,
                    SUB_CMD,
                    shift_down as ::core::ffi::c_int,
                ) as scaled_t;
                if sub_kern != 0 as scaled_t {
                    p = attach_hkern_to_new_hlist(q, sub_kern);
                }
            }
        }
    } else {
        script_head = q + 2 as int32_t;
        script_c = TEX_NULL as int32_t;
        script_g = 0 as uint16_t;
        script_f = 0 as ::core::ffi::c_int as internal_font_number;
        this_math_style = (2 as ::core::ffi::c_int
            * (cur_style as ::core::ffi::c_int / 4 as ::core::ffi::c_int)
            + 5 as ::core::ffi::c_int) as small_number;
        if (*mem.offset(script_head as isize)).b32.s1 == SUB_MLIST as int32_t {
            script_ptr = (*mem.offset(script_head as isize)).b32.s0;
            script_head = TEX_NULL as int32_t;
            while script_ptr >= 0 as int32_t && script_ptr <= mem_end {
                match (*mem.offset(script_ptr as isize)).b16.s1 as ::core::ffi::c_int {
                    KERN_NODE | GLUE_NODE | CHOICE_NODE => {}
                    STYLE_NODE => {
                        this_math_style = (*mem.offset(script_ptr as isize)).b16.s0 as small_number;
                    }
                    ORD_NOAD | OP_NOAD | BIN_NOAD | REL_NOAD | OPEN_NOAD | CLOSE_NOAD
                    | PUNCT_NOAD => {
                        script_head = script_ptr + 1 as int32_t;
                        script_ptr = TEX_NULL as int32_t;
                    }
                    _ => {
                        script_ptr = TEX_NULL as int32_t;
                    }
                }
                if script_ptr >= 0 as int32_t && script_ptr <= mem_end {
                    if (*mem.offset(script_ptr as isize)).b16.s1 as ::core::ffi::c_int
                        == CHOICE_NODE
                    {
                        match this_math_style as ::core::ffi::c_int / 2 as ::core::ffi::c_int {
                            0 => {
                                script_ptr =
                                    (*mem.offset((script_ptr + 1 as int32_t) as isize)).b32.s0;
                            }
                            1 => {
                                script_ptr =
                                    (*mem.offset((script_ptr + 1 as int32_t) as isize)).b32.s1;
                            }
                            2 => {
                                script_ptr =
                                    (*mem.offset((script_ptr + 2 as int32_t) as isize)).b32.s0;
                            }
                            3 => {
                                script_ptr =
                                    (*mem.offset((script_ptr + 2 as int32_t) as isize)).b32.s1;
                            }
                            _ => {}
                        }
                    } else {
                        script_ptr = (*mem.offset(script_ptr as isize)).b32.s1;
                    }
                }
            }
        }
        if script_head >= 0 as int32_t
            && script_head <= mem_end
            && (*mem.offset(script_head as isize)).b32.s1 == MATH_CHAR as int32_t
        {
            save_f = cur_f;
            saved_math_style = cur_style;
            cur_style = this_math_style;
            if (cur_style as ::core::ffi::c_int) < SCRIPT_STYLE {
                cur_size = TEXT_SIZE as int32_t;
            } else {
                cur_size = (SCRIPT_SIZE
                    * ((cur_style as ::core::ffi::c_int - 2 as ::core::ffi::c_int)
                        / 2 as ::core::ffi::c_int)) as int32_t;
            }
            cur_mu = x_over_n(math_quad(cur_size), 18 as int32_t);
            fetch(script_head);
            if *font_area.offset(cur_f as isize) as ::core::ffi::c_uint == OTGR_FONT_FLAG
                && isOpenTypeMathFont(
                    *font_layout_engine.offset(cur_f as isize) as XeTeXLayoutEngine
                ) as ::core::ffi::c_int
                    != 0
            {
                script_c = new_native_character(cur_f, cur_c as UnicodeScalar);
                script_g = real_get_native_glyph(
                    mem.offset(script_c as isize) as *mut memory_word as *mut ::core::ffi::c_void,
                    0 as ::core::ffi::c_uint,
                );
                script_f = cur_f;
            }
            cur_f = save_f;
            cur_style = saved_math_style;
            if (cur_style as ::core::ffi::c_int) < SCRIPT_STYLE {
                cur_size = TEXT_SIZE as int32_t;
            } else {
                cur_size = (SCRIPT_SIZE
                    * ((cur_style as ::core::ffi::c_int - 2 as ::core::ffi::c_int)
                        / 2 as ::core::ffi::c_int)) as int32_t;
            }
            cur_mu = x_over_n(math_quad(cur_size), 18 as int32_t);
        }
        sup_g = script_g;
        sup_f = script_f;
        save_f = cur_f;
        x = clean_box(
            q + 2 as int32_t,
            (2 as ::core::ffi::c_int * (cur_style as ::core::ffi::c_int / 4 as ::core::ffi::c_int)
                + 4 as ::core::ffi::c_int
                + cur_style as ::core::ffi::c_int % 2 as ::core::ffi::c_int)
                as small_number,
        );
        cur_f = save_f;
        (*mem.offset((x + 1 as int32_t) as isize)).b32.s1 =
            (*mem.offset((x + 1 as int32_t) as isize)).b32.s1
                + (*eqtb.offset((DIMEN_BASE + DIMEN_PAR__script_space) as isize))
                    .b32
                    .s1;
        if cur_style as ::core::ffi::c_int & 1 as ::core::ffi::c_int != 0 {
            clr = sup3(cur_size);
        } else if (cur_style as ::core::ffi::c_int) < TEXT_STYLE {
            clr = sup1(cur_size);
        } else {
            clr = sup2(cur_size);
        }
        if shift_up < clr {
            shift_up = clr;
        }
        if *font_area.offset(cur_f as isize) as ::core::ffi::c_uint == OTGR_FONT_FLAG
            && isOpenTypeMathFont(*font_layout_engine.offset(cur_f as isize) as XeTeXLayoutEngine)
                as ::core::ffi::c_int
                != 0
        {
            clr = ((*mem.offset((x + 2 as int32_t) as isize)).b32.s1
                + get_ot_math_constant(cur_f as ::core::ffi::c_int, SUPERSCRIPTBOTTOMMIN)
                    as int32_t) as scaled_t;
        } else {
            clr = ((*mem.offset((x + 2 as int32_t) as isize)).b32.s1
                + abs(math_x_height(cur_size) as ::core::ffi::c_int) as int32_t / 4 as int32_t)
                as scaled_t;
        }
        if shift_up < clr {
            shift_up = clr;
        }
        if *font_area.offset(cur_f as isize) as ::core::ffi::c_uint == OTGR_FONT_FLAG
            && isOpenTypeMathFont(*font_layout_engine.offset(cur_f as isize) as XeTeXLayoutEngine)
                as ::core::ffi::c_int
                != 0
        {
            if (*mem.offset((q + 3 as int32_t) as isize)).b32.s1 == EMPTY as int32_t {
                if p != TEX_NULL as int32_t
                    && !is_char_node(p)
                    && (*mem.offset(p as isize)).b16.s1 as ::core::ffi::c_int == WHATSIT_NODE
                    && (*mem.offset(p as isize)).b16.s0 as ::core::ffi::c_int == GLYPH_NODE
                {
                    sup_kern = get_ot_math_kern(
                        (*mem.offset((p + 4 as int32_t) as isize)).b16.s2 as ::core::ffi::c_int,
                        (*mem.offset((p + 4 as int32_t) as isize)).b16.s1 as ::core::ffi::c_int,
                        sup_f as ::core::ffi::c_int,
                        sup_g as ::core::ffi::c_int,
                        SUP_CMD,
                        shift_up as ::core::ffi::c_int,
                    ) as scaled_t;
                    if sup_kern != 0 as scaled_t {
                        p = attach_hkern_to_new_hlist(q, sup_kern);
                    }
                }
            }
        }
        if (*mem.offset((q + 3 as int32_t) as isize)).b32.s1 == EMPTY as int32_t {
            (*mem.offset((x + 4 as int32_t) as isize)).b32.s1 = -shift_up;
        } else {
            save_f = cur_f;
            script_head = q + 3 as int32_t;
            script_c = TEX_NULL as int32_t;
            script_g = 0 as uint16_t;
            script_f = 0 as ::core::ffi::c_int as internal_font_number;
            this_math_style = (2 as ::core::ffi::c_int
                * (cur_style as ::core::ffi::c_int / 4 as ::core::ffi::c_int)
                + 5 as ::core::ffi::c_int) as small_number;
            if (*mem.offset(script_head as isize)).b32.s1 == SUB_MLIST as int32_t {
                script_ptr = (*mem.offset(script_head as isize)).b32.s0;
                script_head = TEX_NULL as int32_t;
                while script_ptr >= 0 as int32_t && script_ptr <= mem_end {
                    match (*mem.offset(script_ptr as isize)).b16.s1 as ::core::ffi::c_int {
                        KERN_NODE | GLUE_NODE | CHOICE_NODE => {}
                        STYLE_NODE => {
                            this_math_style =
                                (*mem.offset(script_ptr as isize)).b16.s0 as small_number;
                        }
                        ORD_NOAD | OP_NOAD | BIN_NOAD | REL_NOAD | OPEN_NOAD | CLOSE_NOAD
                        | PUNCT_NOAD => {
                            script_head = script_ptr + 1 as int32_t;
                            script_ptr = TEX_NULL as int32_t;
                        }
                        _ => {
                            script_ptr = TEX_NULL as int32_t;
                        }
                    }
                    if script_ptr >= 0 as int32_t && script_ptr <= mem_end {
                        if (*mem.offset(script_ptr as isize)).b16.s1 as ::core::ffi::c_int
                            == CHOICE_NODE
                        {
                            match this_math_style as ::core::ffi::c_int / 2 as ::core::ffi::c_int {
                                0 => {
                                    script_ptr =
                                        (*mem.offset((script_ptr + 1 as int32_t) as isize)).b32.s0;
                                }
                                1 => {
                                    script_ptr =
                                        (*mem.offset((script_ptr + 1 as int32_t) as isize)).b32.s1;
                                }
                                2 => {
                                    script_ptr =
                                        (*mem.offset((script_ptr + 2 as int32_t) as isize)).b32.s0;
                                }
                                3 => {
                                    script_ptr =
                                        (*mem.offset((script_ptr + 2 as int32_t) as isize)).b32.s1;
                                }
                                _ => {}
                            }
                        } else {
                            script_ptr = (*mem.offset(script_ptr as isize)).b32.s1;
                        }
                    }
                }
            }
            if script_head >= 0 as int32_t
                && script_head <= mem_end
                && (*mem.offset(script_head as isize)).b32.s1 == MATH_CHAR as int32_t
            {
                save_f = cur_f;
                saved_math_style = cur_style;
                cur_style = this_math_style;
                if (cur_style as ::core::ffi::c_int) < SCRIPT_STYLE {
                    cur_size = TEXT_SIZE as int32_t;
                } else {
                    cur_size = (SCRIPT_SIZE
                        * ((cur_style as ::core::ffi::c_int - 2 as ::core::ffi::c_int)
                            / 2 as ::core::ffi::c_int)) as int32_t;
                }
                cur_mu = x_over_n(math_quad(cur_size), 18 as int32_t);
                fetch(script_head);
                if *font_area.offset(cur_f as isize) as ::core::ffi::c_uint == OTGR_FONT_FLAG
                    && isOpenTypeMathFont(
                        *font_layout_engine.offset(cur_f as isize) as XeTeXLayoutEngine
                    ) as ::core::ffi::c_int
                        != 0
                {
                    script_c = new_native_character(cur_f, cur_c as UnicodeScalar);
                    script_g = real_get_native_glyph(
                        mem.offset(script_c as isize) as *mut memory_word
                            as *mut ::core::ffi::c_void,
                        0 as ::core::ffi::c_uint,
                    );
                    script_f = cur_f;
                }
                cur_f = save_f;
                cur_style = saved_math_style;
                if (cur_style as ::core::ffi::c_int) < SCRIPT_STYLE {
                    cur_size = TEXT_SIZE as int32_t;
                } else {
                    cur_size = (SCRIPT_SIZE
                        * ((cur_style as ::core::ffi::c_int - 2 as ::core::ffi::c_int)
                            / 2 as ::core::ffi::c_int)) as int32_t;
                }
                cur_mu = x_over_n(math_quad(cur_size), 18 as int32_t);
            }
            sub_g = script_g;
            sub_f = script_f;
            y = clean_box(
                q + 3 as int32_t,
                (2 as ::core::ffi::c_int
                    * (cur_style as ::core::ffi::c_int / 4 as ::core::ffi::c_int)
                    + 5 as ::core::ffi::c_int) as small_number,
            );
            cur_f = save_f;
            (*mem.offset((y + 1 as int32_t) as isize)).b32.s1 =
                (*mem.offset((y + 1 as int32_t) as isize)).b32.s1
                    + (*eqtb.offset((DIMEN_BASE + DIMEN_PAR__script_space) as isize))
                        .b32
                        .s1;
            if shift_down < sub2(cur_size) {
                shift_down = sub2(cur_size);
            }
            if *font_area.offset(cur_f as isize) as ::core::ffi::c_uint == OTGR_FONT_FLAG
                && isOpenTypeMathFont(
                    *font_layout_engine.offset(cur_f as isize) as XeTeXLayoutEngine
                ) as ::core::ffi::c_int
                    != 0
            {
                clr = get_ot_math_constant(cur_f as ::core::ffi::c_int, SUBSUPERSCRIPTGAPMIN)
                    as scaled_t
                    - (shift_up
                        - (*mem.offset((x + 2 as int32_t) as isize)).b32.s1 as scaled_t
                        - ((*mem.offset((y + 3 as int32_t) as isize)).b32.s1 as scaled_t
                            - shift_down));
            } else {
                clr = 4 as scaled_t * default_rule_thickness()
                    - (shift_up
                        - (*mem.offset((x + 2 as int32_t) as isize)).b32.s1 as scaled_t
                        - ((*mem.offset((y + 3 as int32_t) as isize)).b32.s1 as scaled_t
                            - shift_down));
            }
            if clr > 0 as scaled_t {
                shift_down = shift_down + clr;
                if *font_area.offset(cur_f as isize) as ::core::ffi::c_uint == OTGR_FONT_FLAG
                    && isOpenTypeMathFont(
                        *font_layout_engine.offset(cur_f as isize) as XeTeXLayoutEngine
                    ) as ::core::ffi::c_int
                        != 0
                {
                    clr = get_ot_math_constant(
                        cur_f as ::core::ffi::c_int,
                        SUPERSCRIPTBOTTOMMAXWITHSUBSCRIPT,
                    ) as scaled_t
                        - (shift_up
                            - (*mem.offset((x + 2 as int32_t) as isize)).b32.s1 as scaled_t);
                } else {
                    clr = abs(
                        math_x_height(cur_size) as ::core::ffi::c_int * 4 as ::core::ffi::c_int
                    ) as scaled_t
                        / 5 as scaled_t
                        - (shift_up
                            - (*mem.offset((x + 2 as int32_t) as isize)).b32.s1 as scaled_t);
                }
                if clr > 0 as scaled_t {
                    shift_up = shift_up + clr;
                    shift_down = shift_down - clr;
                }
            }
            if *font_area.offset(cur_f as isize) as ::core::ffi::c_uint == OTGR_FONT_FLAG
                && isOpenTypeMathFont(
                    *font_layout_engine.offset(cur_f as isize) as XeTeXLayoutEngine
                ) as ::core::ffi::c_int
                    != 0
            {
                if p != TEX_NULL as int32_t
                    && !is_char_node(p)
                    && (*mem.offset(p as isize)).b16.s1 as ::core::ffi::c_int == WHATSIT_NODE
                    && (*mem.offset(p as isize)).b16.s0 as ::core::ffi::c_int == GLYPH_NODE
                {
                    sub_kern = get_ot_math_kern(
                        (*mem.offset((p + 4 as int32_t) as isize)).b16.s2 as ::core::ffi::c_int,
                        (*mem.offset((p + 4 as int32_t) as isize)).b16.s1 as ::core::ffi::c_int,
                        sub_f as ::core::ffi::c_int,
                        sub_g as ::core::ffi::c_int,
                        SUB_CMD,
                        shift_down as ::core::ffi::c_int,
                    ) as scaled_t;
                    if sub_kern != 0 as scaled_t {
                        p = attach_hkern_to_new_hlist(q, sub_kern);
                    }
                }
                if (*mem.offset((q + 3 as int32_t) as isize)).b32.s1 == EMPTY as int32_t {
                    if p != TEX_NULL as int32_t
                        && !is_char_node(p)
                        && (*mem.offset(p as isize)).b16.s1 as ::core::ffi::c_int == WHATSIT_NODE
                        && (*mem.offset(p as isize)).b16.s0 as ::core::ffi::c_int == GLYPH_NODE
                    {
                        sup_kern = get_ot_math_kern(
                            (*mem.offset((p + 4 as int32_t) as isize)).b16.s2 as ::core::ffi::c_int,
                            (*mem.offset((p + 4 as int32_t) as isize)).b16.s1 as ::core::ffi::c_int,
                            sup_f as ::core::ffi::c_int,
                            sup_g as ::core::ffi::c_int,
                            SUP_CMD,
                            shift_up as ::core::ffi::c_int,
                        ) as scaled_t;
                        if sup_kern != 0 as scaled_t {
                            p = attach_hkern_to_new_hlist(q, sup_kern);
                        }
                    }
                }
            } else {
                sup_kern = 0 as ::core::ffi::c_int as scaled_t;
                sub_kern = 0 as ::core::ffi::c_int as scaled_t;
            }
            (*mem.offset((x + 4 as int32_t) as isize)).b32.s1 =
                (sup_kern + delta - sub_kern) as int32_t;
            p = new_kern(
                shift_up
                    - (*mem.offset((x + 2 as int32_t) as isize)).b32.s1 as scaled_t
                    - ((*mem.offset((y + 3 as int32_t) as isize)).b32.s1 as scaled_t - shift_down),
            );
            (*mem.offset(x as isize)).b32.s1 = p;
            (*mem.offset(p as isize)).b32.s1 = y;
            x = vpackage(
                x,
                0 as scaled_t,
                ADDITIONAL as small_number,
                MAX_HALFWORD as scaled_t,
            );
            (*mem.offset((x + 4 as int32_t) as isize)).b32.s1 = shift_down as int32_t;
        }
    }
    if (*mem.offset((q + 1 as int32_t) as isize)).b32.s1 == TEX_NULL as int32_t {
        (*mem.offset((q + 1 as int32_t) as isize)).b32.s1 = x;
    } else {
        p = (*mem.offset((q + 1 as int32_t) as isize)).b32.s1;
        while (*mem.offset(p as isize)).b32.s1 != TEX_NULL as int32_t {
            p = (*mem.offset(p as isize)).b32.s1;
        }
        (*mem.offset(p as isize)).b32.s1 = x;
    };
}
unsafe extern "C" fn make_left_right(
    mut q: int32_t,
    mut style: small_number,
    mut max_d: scaled_t,
    mut max_h: scaled_t,
) -> small_number {
    let mut delta: scaled_t = 0;
    let mut delta1: scaled_t = 0;
    let mut delta2: scaled_t = 0;
    cur_style = style;
    if (cur_style as ::core::ffi::c_int) < SCRIPT_STYLE {
        cur_size = TEXT_SIZE as int32_t;
    } else {
        cur_size = (SCRIPT_SIZE
            * ((cur_style as ::core::ffi::c_int - 2 as ::core::ffi::c_int)
                / 2 as ::core::ffi::c_int)) as int32_t;
    }
    cur_mu = x_over_n(math_quad(cur_size), 18 as int32_t);
    delta2 = max_d + axis_height(cur_size);
    delta1 = max_h + max_d - delta2;
    if delta2 > delta1 {
        delta1 = delta2;
    }
    delta = delta1 / 500 as scaled_t
        * (*eqtb.offset((INT_BASE + INT_PAR__delimiter_factor) as isize))
            .b32
            .s1 as scaled_t;
    delta2 = delta1 + delta1
        - (*eqtb.offset((DIMEN_BASE + DIMEN_PAR__delimiter_shortfall) as isize))
            .b32
            .s1 as scaled_t;
    if delta < delta2 {
        delta = delta2;
    }
    (*mem.offset((q + 1 as int32_t) as isize)).b32.s1 =
        var_delimiter(q + 1 as int32_t, cur_size, delta);
    return ((*mem.offset(q as isize)).b16.s1 as ::core::ffi::c_int
        - (LEFT_NOAD - 20 as ::core::ffi::c_int)) as small_number;
}
unsafe extern "C" fn mlist_to_hlist() {
    let mut current_block: u64;
    let mut mlist: int32_t = 0;
    let mut penalties: bool = false;
    let mut style: small_number = 0;
    let mut save_style: small_number = 0;
    let mut q: int32_t = 0;
    let mut r: int32_t = 0;
    let mut r_type: small_number = 0;
    let mut t: small_number = 0;
    let mut p: int32_t = TEX_NULL as int32_t;
    let mut x: int32_t = 0;
    let mut y: int32_t = 0;
    let mut z: int32_t = 0;
    let mut pen: int32_t = 0;
    let mut s: small_number = 0;
    let mut max_h: scaled_t = 0;
    let mut max_d: scaled_t = 0;
    let mut delta: scaled_t = 0;
    mlist = cur_mlist;
    penalties = mlist_penalties;
    style = cur_style;
    q = mlist;
    r = TEX_NULL as int32_t;
    r_type = OP_NOAD as small_number;
    max_h = 0 as ::core::ffi::c_int as scaled_t;
    max_d = 0 as ::core::ffi::c_int as scaled_t;
    if (cur_style as ::core::ffi::c_int) < SCRIPT_STYLE {
        cur_size = TEXT_SIZE as int32_t;
    } else {
        cur_size = (SCRIPT_SIZE
            * ((cur_style as ::core::ffi::c_int - 2 as ::core::ffi::c_int)
                / 2 as ::core::ffi::c_int)) as int32_t;
    }
    cur_mu = x_over_n(math_quad(cur_size), 18 as int32_t);
    while q != TEX_NULL as int32_t {
        loop {
            delta = 0 as ::core::ffi::c_int as scaled_t;
            match (*mem.offset(q as isize)).b16.s1 as ::core::ffi::c_int {
                BIN_NOAD => {
                    match r_type as ::core::ffi::c_int {
                        BIN_NOAD | OP_NOAD | REL_NOAD | OPEN_NOAD | PUNCT_NOAD | LEFT_NOAD => {}
                        _ => {
                            current_block = 517042441694919077;
                            break;
                        }
                    }
                    (*mem.offset(q as isize)).b16.s1 = ORD_NOAD as uint16_t;
                }
                REL_NOAD | CLOSE_NOAD | PUNCT_NOAD | RIGHT_NOAD => {
                    if r_type as ::core::ffi::c_int == BIN_NOAD {
                        (*mem.offset(r as isize)).b16.s1 = ORD_NOAD as uint16_t;
                    }
                    if (*mem.offset(q as isize)).b16.s1 as ::core::ffi::c_int == RIGHT_NOAD {
                        current_block = 4977813590126011707;
                        break;
                    } else {
                        current_block = 517042441694919077;
                        break;
                    }
                }
                LEFT_NOAD => {
                    current_block = 4977813590126011707;
                    break;
                }
                FRACTION_NOAD => {
                    make_fraction(q);
                    current_block = 3615033227452819297;
                    break;
                }
                OP_NOAD => {
                    delta = make_op(q);
                    if (*mem.offset(q as isize)).b16.s0 as ::core::ffi::c_int == LIMITS {
                        current_block = 3615033227452819297;
                        break;
                    } else {
                        current_block = 517042441694919077;
                        break;
                    }
                }
                ORD_NOAD => {
                    make_ord(q);
                    current_block = 517042441694919077;
                    break;
                }
                OPEN_NOAD | INNER_NOAD => {
                    current_block = 517042441694919077;
                    break;
                }
                RADICAL_NOAD => {
                    make_radical(q);
                    current_block = 517042441694919077;
                    break;
                }
                OVER_NOAD => {
                    make_over(q);
                    current_block = 517042441694919077;
                    break;
                }
                UNDER_NOAD => {
                    make_under(q);
                    current_block = 517042441694919077;
                    break;
                }
                ACCENT_NOAD => {
                    make_math_accent(q);
                    current_block = 517042441694919077;
                    break;
                }
                VCENTER_NOAD => {
                    make_vcenter(q);
                    current_block = 517042441694919077;
                    break;
                }
                STYLE_NODE => {
                    cur_style = (*mem.offset(q as isize)).b16.s0 as small_number;
                    if (cur_style as ::core::ffi::c_int) < SCRIPT_STYLE {
                        cur_size = TEXT_SIZE as int32_t;
                    } else {
                        cur_size = (SCRIPT_SIZE
                            * ((cur_style as ::core::ffi::c_int - 2 as ::core::ffi::c_int)
                                / 2 as ::core::ffi::c_int))
                            as int32_t;
                    }
                    cur_mu = x_over_n(math_quad(cur_size), 18 as int32_t);
                    current_block = 13885369121984515127;
                    break;
                }
                CHOICE_NODE => {
                    match cur_style as ::core::ffi::c_int / 2 as ::core::ffi::c_int {
                        0 => {
                            p = (*mem.offset((q + 1 as int32_t) as isize)).b32.s0;
                            (*mem.offset((q + 1 as int32_t) as isize)).b32.s0 = TEX_NULL as int32_t;
                        }
                        1 => {
                            p = (*mem.offset((q + 1 as int32_t) as isize)).b32.s1;
                            (*mem.offset((q + 1 as int32_t) as isize)).b32.s1 = TEX_NULL as int32_t;
                        }
                        2 => {
                            p = (*mem.offset((q + 2 as int32_t) as isize)).b32.s0;
                            (*mem.offset((q + 2 as int32_t) as isize)).b32.s0 = TEX_NULL as int32_t;
                        }
                        3 => {
                            p = (*mem.offset((q + 2 as int32_t) as isize)).b32.s1;
                            (*mem.offset((q + 2 as int32_t) as isize)).b32.s1 = TEX_NULL as int32_t;
                        }
                        _ => {}
                    }
                    flush_node_list((*mem.offset((q + 1 as int32_t) as isize)).b32.s0);
                    flush_node_list((*mem.offset((q + 1 as int32_t) as isize)).b32.s1);
                    flush_node_list((*mem.offset((q + 2 as int32_t) as isize)).b32.s0);
                    flush_node_list((*mem.offset((q + 2 as int32_t) as isize)).b32.s1);
                    (*mem.offset(q as isize)).b16.s1 = STYLE_NODE as uint16_t;
                    (*mem.offset(q as isize)).b16.s0 = cur_style as uint16_t;
                    (*mem.offset((q + 1 as int32_t) as isize)).b32.s1 =
                        0 as ::core::ffi::c_int as int32_t;
                    (*mem.offset((q + 2 as int32_t) as isize)).b32.s1 =
                        0 as ::core::ffi::c_int as int32_t;
                    if p != TEX_NULL as int32_t {
                        z = (*mem.offset(q as isize)).b32.s1;
                        (*mem.offset(q as isize)).b32.s1 = p;
                        while (*mem.offset(p as isize)).b32.s1 != TEX_NULL as int32_t {
                            p = (*mem.offset(p as isize)).b32.s1;
                        }
                        (*mem.offset(p as isize)).b32.s1 = z;
                    }
                    current_block = 13885369121984515127;
                    break;
                }
                INS_NODE | MARK_NODE | ADJUST_NODE | WHATSIT_NODE | PENALTY_NODE | DISC_NODE => {
                    current_block = 13885369121984515127;
                    break;
                }
                RULE_NODE => {
                    if (*mem.offset((q + 3 as int32_t) as isize)).b32.s1 > max_h {
                        max_h = (*mem.offset((q + 3 as int32_t) as isize)).b32.s1 as scaled_t;
                    }
                    if (*mem.offset((q + 2 as int32_t) as isize)).b32.s1 > max_d {
                        max_d = (*mem.offset((q + 2 as int32_t) as isize)).b32.s1 as scaled_t;
                    }
                    current_block = 13885369121984515127;
                    break;
                }
                GLUE_NODE => {
                    if (*mem.offset(q as isize)).b16.s0 as ::core::ffi::c_int == MU_GLUE {
                        x = (*mem.offset((q + 1 as int32_t) as isize)).b32.s0;
                        y = math_glue(x, cur_mu);
                        delete_glue_ref(x);
                        (*mem.offset((q + 1 as int32_t) as isize)).b32.s0 = y;
                        (*mem.offset(q as isize)).b16.s0 = NORMAL as uint16_t;
                    } else if cur_size != TEXT_SIZE as int32_t
                        && (*mem.offset(q as isize)).b16.s0 as ::core::ffi::c_int == COND_MATH_GLUE
                    {
                        p = (*mem.offset(q as isize)).b32.s1;
                        if p != TEX_NULL as int32_t {
                            if (*mem.offset(p as isize)).b16.s1 as ::core::ffi::c_int == GLUE_NODE
                                || (*mem.offset(p as isize)).b16.s1 as ::core::ffi::c_int
                                    == KERN_NODE
                            {
                                (*mem.offset(q as isize)).b32.s1 = (*mem.offset(p as isize)).b32.s1;
                                (*mem.offset(p as isize)).b32.s1 = TEX_NULL as int32_t;
                                flush_node_list(p);
                            }
                        }
                    }
                    current_block = 13885369121984515127;
                    break;
                }
                KERN_NODE => {
                    math_kern(q, cur_mu);
                    current_block = 13885369121984515127;
                    break;
                }
                _ => {
                    confusion(b"mlist1\0" as *const u8 as *const ::core::ffi::c_char);
                }
            }
        }
        match current_block {
            517042441694919077 => {
                match (*mem.offset((q + 1 as int32_t) as isize)).b32.s1 {
                    1 | 4 => {
                        fetch(q + 1 as int32_t);
                        if *font_area.offset(cur_f as isize) as ::core::ffi::c_uint == AAT_FONT_FLAG
                            || *font_area.offset(cur_f as isize) as ::core::ffi::c_uint
                                == OTGR_FONT_FLAG
                        {
                            z = new_native_character(cur_f, cur_c as UnicodeScalar);
                            p = get_node(GLYPH_NODE_SIZE as int32_t);
                            (*mem.offset(p as isize)).b16.s1 = WHATSIT_NODE as uint16_t;
                            (*mem.offset(p as isize)).b16.s0 = GLYPH_NODE as uint16_t;
                            (*mem.offset((p + 4 as int32_t) as isize)).b16.s2 = cur_f as uint16_t;
                            (*mem.offset((p + 4 as int32_t) as isize)).b16.s1 =
                                real_get_native_glyph(
                                    mem.offset(z as isize) as *mut memory_word
                                        as *mut ::core::ffi::c_void,
                                    0 as ::core::ffi::c_uint,
                                );
                            measure_native_glyph(
                                mem.offset(p as isize) as *mut memory_word
                                    as *mut ::core::ffi::c_void,
                                1 as ::core::ffi::c_int,
                            );
                            free_node(
                                z,
                                (*mem.offset((z + 4 as int32_t) as isize)).b16.s3 as int32_t,
                            );
                            delta = get_ot_math_ital_corr(
                                cur_f as ::core::ffi::c_int,
                                (*mem.offset((p + 4 as int32_t) as isize)).b16.s1
                                    as ::core::ffi::c_int,
                            ) as scaled_t;
                            if (*mem.offset((q + 1 as int32_t) as isize)).b32.s1
                                == MATH_TEXT_CHAR as int32_t
                                && !(*font_area.offset(cur_f as isize) as ::core::ffi::c_uint
                                    == OTGR_FONT_FLAG
                                    && isOpenTypeMathFont(
                                        *font_layout_engine.offset(cur_f as isize)
                                            as XeTeXLayoutEngine,
                                    ) as ::core::ffi::c_int
                                        != 0)
                                    as ::core::ffi::c_int
                                    != 0 as ::core::ffi::c_int
                            {
                                delta = 0 as ::core::ffi::c_int as scaled_t;
                            }
                            if (*mem.offset((q + 3 as int32_t) as isize)).b32.s1 == EMPTY as int32_t
                                && delta != 0 as scaled_t
                            {
                                (*mem.offset(p as isize)).b32.s1 = new_kern(delta);
                                delta = 0 as ::core::ffi::c_int as scaled_t;
                            }
                        } else if cur_i.s3 as ::core::ffi::c_int > 0 as ::core::ffi::c_int {
                            delta = (*font_info.offset(
                                (*italic_base.offset(cur_f as isize)
                                    + cur_i.s1 as int32_t / 4 as int32_t)
                                    as isize,
                            ))
                            .b32
                            .s1 as scaled_t;
                            p = new_character(cur_f, cur_c as UTF16_code);
                            if (*mem.offset((q + 1 as int32_t) as isize)).b32.s1
                                == MATH_TEXT_CHAR as int32_t
                                && (*font_info.offset(
                                    (SPACE_CODE as int32_t + *param_base.offset(cur_f as isize))
                                        as isize,
                                ))
                                .b32
                                .s1 != 0 as int32_t
                            {
                                delta = 0 as ::core::ffi::c_int as scaled_t;
                            }
                            if (*mem.offset((q + 3 as int32_t) as isize)).b32.s1 == EMPTY as int32_t
                                && delta != 0 as scaled_t
                            {
                                (*mem.offset(p as isize)).b32.s1 = new_kern(delta);
                                delta = 0 as ::core::ffi::c_int as scaled_t;
                            }
                        } else {
                            p = TEX_NULL as int32_t;
                        }
                    }
                    0 => {
                        p = TEX_NULL as int32_t;
                    }
                    2 => {
                        p = (*mem.offset((q + 1 as int32_t) as isize)).b32.s0;
                    }
                    3 => {
                        cur_mlist = (*mem.offset((q + 1 as int32_t) as isize)).b32.s0;
                        save_style = cur_style;
                        mlist_penalties = false_0 != 0;
                        mlist_to_hlist();
                        cur_style = save_style;
                        if (cur_style as ::core::ffi::c_int) < SCRIPT_STYLE {
                            cur_size = TEXT_SIZE as int32_t;
                        } else {
                            cur_size = (SCRIPT_SIZE
                                * ((cur_style as ::core::ffi::c_int - 2 as ::core::ffi::c_int)
                                    / 2 as ::core::ffi::c_int))
                                as int32_t;
                        }
                        cur_mu = x_over_n(math_quad(cur_size), 18 as int32_t);
                        p = hpack(
                            (*mem.offset(TEMP_HEAD as isize)).b32.s1,
                            0 as scaled_t,
                            ADDITIONAL as small_number,
                        );
                    }
                    _ => {
                        confusion(b"mlist2\0" as *const u8 as *const ::core::ffi::c_char);
                    }
                }
                (*mem.offset((q + 1 as int32_t) as isize)).b32.s1 = p;
                if (*mem.offset((q + 3 as int32_t) as isize)).b32.s1 == EMPTY as int32_t
                    && (*mem.offset((q + 2 as int32_t) as isize)).b32.s1 == EMPTY as int32_t
                {
                    current_block = 3615033227452819297;
                } else {
                    make_scripts(q, delta);
                    current_block = 3615033227452819297;
                }
            }
            _ => {}
        }
        match current_block {
            3615033227452819297 => {
                z = hpack(
                    (*mem.offset((q + 1 as int32_t) as isize)).b32.s1,
                    0 as scaled_t,
                    ADDITIONAL as small_number,
                );
                if (*mem.offset((z + 3 as int32_t) as isize)).b32.s1 > max_h {
                    max_h = (*mem.offset((z + 3 as int32_t) as isize)).b32.s1 as scaled_t;
                }
                if (*mem.offset((z + 2 as int32_t) as isize)).b32.s1 > max_d {
                    max_d = (*mem.offset((z + 2 as int32_t) as isize)).b32.s1 as scaled_t;
                }
                free_node(z, BOX_NODE_SIZE as int32_t);
                current_block = 4977813590126011707;
            }
            _ => {}
        }
        match current_block {
            4977813590126011707 => {
                r = q;
                r_type = (*mem.offset(r as isize)).b16.s1 as small_number;
                if r_type as ::core::ffi::c_int == RIGHT_NOAD {
                    r_type = LEFT_NOAD as small_number;
                    cur_style = style;
                    if (cur_style as ::core::ffi::c_int) < SCRIPT_STYLE {
                        cur_size = TEXT_SIZE as int32_t;
                    } else {
                        cur_size = (SCRIPT_SIZE
                            * ((cur_style as ::core::ffi::c_int - 2 as ::core::ffi::c_int)
                                / 2 as ::core::ffi::c_int))
                            as int32_t;
                    }
                    cur_mu = x_over_n(math_quad(cur_size), 18 as int32_t);
                }
            }
            _ => {}
        }
        q = (*mem.offset(q as isize)).b32.s1;
    }
    if r_type as ::core::ffi::c_int == BIN_NOAD {
        (*mem.offset(r as isize)).b16.s1 = 16 as uint16_t;
    }
    p = TEMP_HEAD as int32_t;
    (*mem.offset(p as isize)).b32.s1 = TEX_NULL as int32_t;
    q = mlist;
    r_type = 0 as small_number;
    cur_style = style;
    if (cur_style as ::core::ffi::c_int) < SCRIPT_STYLE {
        cur_size = TEXT_SIZE as int32_t;
    } else {
        cur_size = (SCRIPT_SIZE
            * ((cur_style as ::core::ffi::c_int - 2 as ::core::ffi::c_int)
                / 2 as ::core::ffi::c_int)) as int32_t;
    }
    cur_mu = x_over_n(math_quad(cur_size), 18 as int32_t);
    while q != TEX_NULL as int32_t {
        let mut current_block_235: u64;
        t = ORD_NOAD as small_number;
        s = NOAD_SIZE as small_number;
        pen = INF_PENALTY as int32_t;
        match (*mem.offset(q as isize)).b16.s1 as ::core::ffi::c_int {
            OP_NOAD | OPEN_NOAD | CLOSE_NOAD | PUNCT_NOAD | INNER_NOAD => {
                t = (*mem.offset(q as isize)).b16.s1 as small_number;
                current_block_235 = 11202235766349324107;
            }
            BIN_NOAD => {
                t = BIN_NOAD as small_number;
                pen = (*eqtb.offset((INT_BASE + INT_PAR__bin_op_penalty) as isize))
                    .b32
                    .s1;
                current_block_235 = 11202235766349324107;
            }
            REL_NOAD => {
                t = REL_NOAD as small_number;
                pen = (*eqtb.offset((INT_BASE + INT_PAR__rel_penalty) as isize))
                    .b32
                    .s1;
                current_block_235 = 11202235766349324107;
            }
            ORD_NOAD | VCENTER_NOAD | OVER_NOAD | UNDER_NOAD => {
                current_block_235 = 11202235766349324107;
            }
            RADICAL_NOAD => {
                s = RADICAL_NOAD_SIZE as small_number;
                current_block_235 = 11202235766349324107;
            }
            ACCENT_NOAD => {
                s = ACCENT_NOAD_SIZE as small_number;
                current_block_235 = 11202235766349324107;
            }
            FRACTION_NOAD => {
                s = FRACTION_NOAD_SIZE as small_number;
                current_block_235 = 11202235766349324107;
            }
            LEFT_NOAD | RIGHT_NOAD => {
                t = make_left_right(q, style, max_d, max_h);
                current_block_235 = 11202235766349324107;
            }
            STYLE_NODE => {
                cur_style = (*mem.offset(q as isize)).b16.s0 as small_number;
                s = STYLE_NODE_SIZE as small_number;
                if (cur_style as ::core::ffi::c_int) < SCRIPT_STYLE {
                    cur_size = TEXT_SIZE as int32_t;
                } else {
                    cur_size = (SCRIPT_SIZE
                        * ((cur_style as ::core::ffi::c_int - 2 as ::core::ffi::c_int)
                            / 2 as ::core::ffi::c_int)) as int32_t;
                }
                cur_mu = x_over_n(math_quad(cur_size), 18 as int32_t);
                current_block_235 = 13399908265510761917;
            }
            WHATSIT_NODE | PENALTY_NODE | RULE_NODE | DISC_NODE | ADJUST_NODE | INS_NODE
            | MARK_NODE | GLUE_NODE | KERN_NODE => {
                (*mem.offset(p as isize)).b32.s1 = q;
                p = q;
                q = (*mem.offset(q as isize)).b32.s1;
                (*mem.offset(p as isize)).b32.s1 = TEX_NULL as int32_t;
                current_block_235 = 3240127892612706009;
            }
            _ => {
                confusion(b"mlist3\0" as *const u8 as *const ::core::ffi::c_char);
            }
        }
        match current_block_235 {
            11202235766349324107 => {
                if r_type as ::core::ffi::c_int > 0 as ::core::ffi::c_int {
                    let mut offset_table: [*const ::core::ffi::c_char; 8] = [
                        b"02340001\0" as *const u8 as *const ::core::ffi::c_char,
                        b"22*40001\0" as *const u8 as *const ::core::ffi::c_char,
                        b"33**3**3\0" as *const u8 as *const ::core::ffi::c_char,
                        b"44*04004\0" as *const u8 as *const ::core::ffi::c_char,
                        b"00*00000\0" as *const u8 as *const ::core::ffi::c_char,
                        b"02340001\0" as *const u8 as *const ::core::ffi::c_char,
                        b"11*11111\0" as *const u8 as *const ::core::ffi::c_char,
                        b"12341011\0" as *const u8 as *const ::core::ffi::c_char,
                    ];
                    match *offset_table[(r_type as ::core::ffi::c_int - ORD_NOAD) as usize]
                        .offset((t as ::core::ffi::c_int - ORD_NOAD) as isize)
                        as ::core::ffi::c_int
                    {
                        48 => {
                            x = 0 as ::core::ffi::c_int as int32_t;
                        }
                        49 => {
                            if (cur_style as ::core::ffi::c_int) < SCRIPT_STYLE {
                                x = GLUE_PAR__thin_mu_skip as int32_t;
                            } else {
                                x = 0 as ::core::ffi::c_int as int32_t;
                            }
                        }
                        50 => {
                            x = GLUE_PAR__thin_mu_skip as int32_t;
                        }
                        51 => {
                            if (cur_style as ::core::ffi::c_int) < SCRIPT_STYLE {
                                x = GLUE_PAR__med_mu_skip as int32_t;
                            } else {
                                x = 0 as ::core::ffi::c_int as int32_t;
                            }
                        }
                        52 => {
                            if (cur_style as ::core::ffi::c_int) < SCRIPT_STYLE {
                                x = GLUE_PAR__thick_mu_skip as int32_t;
                            } else {
                                x = 0 as ::core::ffi::c_int as int32_t;
                            }
                        }
                        _ => {
                            confusion(b"mlist4\0" as *const u8 as *const ::core::ffi::c_char);
                        }
                    }
                    if x != 0 as int32_t {
                        y = math_glue(
                            (*eqtb.offset((GLUE_BASE as int32_t + x) as isize)).b32.s1,
                            cur_mu,
                        );
                        z = new_glue(y);
                        (*mem.offset(y as isize)).b32.s1 = TEX_NULL as int32_t;
                        (*mem.offset(p as isize)).b32.s1 = z;
                        p = z;
                        (*mem.offset(z as isize)).b16.s0 = (x + 1 as int32_t) as uint16_t;
                    }
                }
                if (*mem.offset((q + 1 as int32_t) as isize)).b32.s1 != TEX_NULL as int32_t {
                    (*mem.offset(p as isize)).b32.s1 =
                        (*mem.offset((q + 1 as int32_t) as isize)).b32.s1;
                    loop {
                        p = (*mem.offset(p as isize)).b32.s1;
                        if (*mem.offset(p as isize)).b32.s1 == TEX_NULL as int32_t {
                            break;
                        }
                    }
                }
                if penalties {
                    if (*mem.offset(q as isize)).b32.s1 != TEX_NULL as int32_t {
                        if pen < INF_PENALTY as int32_t {
                            r_type = (*mem.offset((*mem.offset(q as isize)).b32.s1 as isize))
                                .b16
                                .s1 as small_number;
                            if r_type as ::core::ffi::c_int != PENALTY_NODE {
                                if r_type as ::core::ffi::c_int != REL_NOAD {
                                    z = new_penalty(pen);
                                    (*mem.offset(p as isize)).b32.s1 = z;
                                    p = z;
                                }
                            }
                        }
                    }
                }
                if (*mem.offset(q as isize)).b16.s1 as ::core::ffi::c_int == RIGHT_NOAD {
                    t = OPEN_NOAD as small_number;
                }
                r_type = t;
                current_block_235 = 13399908265510761917;
            }
            _ => {}
        }
        match current_block_235 {
            13399908265510761917 => {
                r = q;
                q = (*mem.offset(q as isize)).b32.s1;
                free_node(r, s as int32_t);
            }
            _ => {}
        }
    }
}
unsafe extern "C" fn var_delimiter(mut d: int32_t, mut s: int32_t, mut v: scaled_t) -> int32_t {
    let mut b: int32_t = 0;
    let mut ot_assembly_ptr: *mut ::core::ffi::c_void =
        ::core::ptr::null_mut::<::core::ffi::c_void>();
    let mut f: internal_font_number = 0;
    let mut g: internal_font_number = 0;
    let mut c: uint16_t = 0 as uint16_t;
    let mut x: uint16_t = 0;
    let mut y: uint16_t = 0;
    let mut m: int32_t = 0;
    let mut n: int32_t = 0;
    let mut u: scaled_t = 0;
    let mut w: scaled_t = 0;
    let mut q: b16x4 = b16x4_le_t {
        s0: 0 as uint16_t,
        s1: 0 as uint16_t,
        s2: 0 as uint16_t,
        s3: 0 as uint16_t,
    };
    let mut r: b16x4 = b16x4_le_t {
        s0: 0,
        s1: 0,
        s2: 0,
        s3: 0,
    };
    let mut z: int32_t = 0;
    let mut large_attempt: bool = false;
    f = FONT_BASE as internal_font_number;
    w = 0 as ::core::ffi::c_int as scaled_t;
    large_attempt = false_0 != 0;
    z = ((*mem.offset(d as isize)).b16.s3 as ::core::ffi::c_int % 256 as ::core::ffi::c_int)
        as int32_t;
    x = ((*mem.offset(d as isize)).b16.s2 as ::core::ffi::c_long
        + ((*mem.offset(d as isize)).b16.s3 as ::core::ffi::c_int / 256 as ::core::ffi::c_int)
            as ::core::ffi::c_long
            * 65536 as ::core::ffi::c_long) as uint16_t;
    ot_assembly_ptr = NULL;
    's_44: loop {
        if z != 0 as int32_t || x as ::core::ffi::c_int != 0 as ::core::ffi::c_int {
            z = z + s + 256 as int32_t;
            loop {
                z = z - 256 as int32_t;
                g = (*eqtb.offset((MATH_FONT_BASE as int32_t + z) as isize))
                    .b32
                    .s1 as internal_font_number;
                if g != FONT_BASE as internal_font_number {
                    if *font_area.offset(g as isize) as ::core::ffi::c_uint == OTGR_FONT_FLAG
                        && usingOpenType(*font_layout_engine.offset(g as isize) as XeTeXLayoutEngine)
                            as ::core::ffi::c_int
                            != 0
                    {
                        x = map_char_to_glyph(g as int32_t, x as int32_t) as uint16_t;
                        f = g;
                        c = x;
                        w = 0 as ::core::ffi::c_int as scaled_t;
                        n = 0 as ::core::ffi::c_int as int32_t;
                        loop {
                            y = get_ot_math_variant(
                                g as ::core::ffi::c_int,
                                x as ::core::ffi::c_int,
                                n as ::core::ffi::c_int,
                                &raw mut u,
                                0 as ::core::ffi::c_int,
                            ) as uint16_t;
                            if u > w {
                                c = y;
                                w = u;
                                if u >= v {
                                    break 's_44;
                                }
                            }
                            n = n + 1 as int32_t;
                            if u < 0 as scaled_t {
                                break;
                            }
                        }
                        ot_assembly_ptr = get_ot_assembly_ptr(
                            g as ::core::ffi::c_int,
                            x as ::core::ffi::c_int,
                            0 as ::core::ffi::c_int,
                        );
                        if !ot_assembly_ptr.is_null() {
                            break 's_44;
                        }
                    } else {
                        y = x;
                        if y as ::core::ffi::c_int
                            >= *font_bc.offset(g as isize) as ::core::ffi::c_int
                            && y as ::core::ffi::c_int
                                <= *font_ec.offset(g as isize) as ::core::ffi::c_int
                        {
                            loop {
                                q = (*font_info.offset(
                                    (*char_base.offset(g as isize) + y as int32_t) as isize,
                                ))
                                .b16;
                                if !(q.s3 as ::core::ffi::c_int > 0 as ::core::ffi::c_int) {
                                    break;
                                }
                                if q.s1 as ::core::ffi::c_int % 4 as ::core::ffi::c_int == EXT_TAG {
                                    f = g;
                                    c = y;
                                    break 's_44;
                                } else {
                                    u = ((*font_info.offset(
                                        (*height_base.offset(g as isize)
                                            + q.s2 as int32_t / 16 as int32_t)
                                            as isize,
                                    ))
                                    .b32
                                    .s1 + (*font_info.offset(
                                        (*depth_base.offset(g as isize)
                                            + q.s2 as int32_t % 16 as int32_t)
                                            as isize,
                                    ))
                                    .b32
                                    .s1) as scaled_t;
                                    if u > w {
                                        f = g;
                                        c = y;
                                        w = u;
                                        if u >= v {
                                            break 's_44;
                                        }
                                    }
                                    if !(q.s1 as ::core::ffi::c_int % 4 as ::core::ffi::c_int
                                        == LIST_TAG)
                                    {
                                        break;
                                    }
                                    y = q.s0;
                                }
                            }
                        }
                    }
                }
                if z < SCRIPT_SIZE as int32_t {
                    break;
                }
            }
        }
        if large_attempt {
            break;
        }
        large_attempt = true_0 != 0;
        z = ((*mem.offset(d as isize)).b16.s1 as ::core::ffi::c_int % 256 as ::core::ffi::c_int)
            as int32_t;
        x = ((*mem.offset(d as isize)).b16.s0 as ::core::ffi::c_long
            + ((*mem.offset(d as isize)).b16.s1 as ::core::ffi::c_int / 256 as ::core::ffi::c_int)
                as ::core::ffi::c_long
                * 65536 as ::core::ffi::c_long) as uint16_t;
    }
    if f != FONT_BASE as internal_font_number {
        if !(*font_area.offset(f as isize) as ::core::ffi::c_uint == OTGR_FONT_FLAG
            && usingOpenType(*font_layout_engine.offset(f as isize) as XeTeXLayoutEngine)
                as ::core::ffi::c_int
                != 0)
        {
            if q.s1 as ::core::ffi::c_int % 4 as ::core::ffi::c_int == EXT_TAG {
                b = new_null_box();
                (*mem.offset(b as isize)).b16.s1 = VLIST_NODE as uint16_t;
                r = (*font_info
                    .offset((*exten_base.offset(f as isize) + q.s0 as int32_t) as isize))
                .b16;
                c = r.s0;
                u = height_plus_depth(f, c);
                w = 0 as ::core::ffi::c_int as scaled_t;
                q = (*font_info.offset(
                    (*char_base.offset(f as isize)
                        + effective_char(1 as ::core::ffi::c_int != 0, f, c))
                        as isize,
                ))
                .b16;
                (*mem.offset((b + 1 as int32_t) as isize)).b32.s1 = (*font_info
                    .offset((*width_base.offset(f as isize) + q.s3 as int32_t) as isize))
                .b32
                .s1 + (*font_info.offset(
                    (*italic_base.offset(f as isize) + q.s1 as int32_t / 4 as int32_t) as isize,
                ))
                .b32
                .s1;
                c = r.s1;
                if c as ::core::ffi::c_int != 0 as ::core::ffi::c_int {
                    w = w + height_plus_depth(f, c);
                }
                c = r.s2;
                if c as ::core::ffi::c_int != 0 as ::core::ffi::c_int {
                    w = w + height_plus_depth(f, c);
                }
                c = r.s3;
                if c as ::core::ffi::c_int != 0 as ::core::ffi::c_int {
                    w = w + height_plus_depth(f, c);
                }
                n = 0 as ::core::ffi::c_int as int32_t;
                if u > 0 as scaled_t {
                    while w < v {
                        w = w + u;
                        n += 1;
                        if r.s2 as ::core::ffi::c_int != 0 as ::core::ffi::c_int {
                            w = w + u;
                        }
                    }
                }
                c = r.s1;
                if c as ::core::ffi::c_int != 0 as ::core::ffi::c_int {
                    stack_into_box(b, f, c);
                }
                c = r.s0;
                let mut for_end: int32_t = 0;
                m = 1 as ::core::ffi::c_int as int32_t;
                for_end = n;
                if m <= for_end {
                    loop {
                        stack_into_box(b, f, c);
                        let fresh0 = m;
                        m = m + 1;
                        if !(fresh0 < for_end) {
                            break;
                        }
                    }
                }
                c = r.s2;
                if c as ::core::ffi::c_int != 0 as ::core::ffi::c_int {
                    stack_into_box(b, f, c);
                    c = r.s0;
                    let mut for_end_0: int32_t = 0;
                    m = 1 as ::core::ffi::c_int as int32_t;
                    for_end_0 = n;
                    if m <= for_end_0 {
                        loop {
                            stack_into_box(b, f, c);
                            let fresh1 = m;
                            m = m + 1;
                            if !(fresh1 < for_end_0) {
                                break;
                            }
                        }
                    }
                }
                c = r.s3;
                if c as ::core::ffi::c_int != 0 as ::core::ffi::c_int {
                    stack_into_box(b, f, c);
                }
                (*mem.offset((b + 2 as int32_t) as isize)).b32.s1 =
                    (w - (*mem.offset((b + 3 as int32_t) as isize)).b32.s1 as scaled_t) as int32_t;
            } else {
                b = char_box(f, c as int32_t);
            }
        } else if !ot_assembly_ptr.is_null() {
            b = build_opentype_assembly(f, ot_assembly_ptr, v, 0 as ::core::ffi::c_int != 0);
        } else {
            b = new_null_box();
            (*mem.offset(b as isize)).b16.s1 = VLIST_NODE as uint16_t;
            (*mem.offset((b + 5 as int32_t) as isize)).b32.s1 =
                get_node(GLYPH_NODE_SIZE as int32_t);
            (*mem.offset((*mem.offset((b + 5 as int32_t) as isize)).b32.s1 as isize))
                .b16
                .s1 = WHATSIT_NODE as uint16_t;
            (*mem.offset((*mem.offset((b + 5 as int32_t) as isize)).b32.s1 as isize))
                .b16
                .s0 = GLYPH_NODE as uint16_t;
            (*mem.offset(
                ((*mem.offset((b + 5 as int32_t) as isize)).b32.s1 + 4 as int32_t) as isize,
            ))
            .b16
            .s2 = f as uint16_t;
            (*mem.offset(
                ((*mem.offset((b + 5 as int32_t) as isize)).b32.s1 + 4 as int32_t) as isize,
            ))
            .b16
            .s1 = c;
            measure_native_glyph(
                mem.offset((*mem.offset((b + 5 as int32_t) as isize)).b32.s1 as isize)
                    as *mut memory_word as *mut ::core::ffi::c_void,
                1 as ::core::ffi::c_int,
            );
            (*mem.offset((b + 1 as int32_t) as isize)).b32.s1 = (*mem.offset(
                ((*mem.offset((b + 5 as int32_t) as isize)).b32.s1 + 1 as int32_t) as isize,
            ))
            .b32
            .s1;
            (*mem.offset((b + 3 as int32_t) as isize)).b32.s1 = (*mem.offset(
                ((*mem.offset((b + 5 as int32_t) as isize)).b32.s1 + 3 as int32_t) as isize,
            ))
            .b32
            .s1;
            (*mem.offset((b + 2 as int32_t) as isize)).b32.s1 = (*mem.offset(
                ((*mem.offset((b + 5 as int32_t) as isize)).b32.s1 + 2 as int32_t) as isize,
            ))
            .b32
            .s1;
        }
    } else {
        b = new_null_box();
        (*mem.offset((b + 1 as int32_t) as isize)).b32.s1 = (*eqtb
            .offset((DIMEN_BASE + DIMEN_PAR__null_delimiter_space) as isize))
        .b32
        .s1;
    }
    (*mem.offset((b + 4 as int32_t) as isize)).b32.s1 = half(
        (*mem.offset((b + 3 as int32_t) as isize)).b32.s1
            - (*mem.offset((b + 2 as int32_t) as isize)).b32.s1,
    ) - axis_height(s) as int32_t;
    free_ot_assembly(ot_assembly_ptr as *mut GlyphAssembly);
    return b;
}
unsafe extern "C" fn char_box(mut f: internal_font_number, mut c: int32_t) -> int32_t {
    let mut q: b16x4 = b16x4_le_t {
        s0: 0,
        s1: 0,
        s2: 0,
        s3: 0,
    };
    let mut b: int32_t = 0;
    let mut p: int32_t = 0;
    if *font_area.offset(f as isize) as ::core::ffi::c_uint == AAT_FONT_FLAG
        || *font_area.offset(f as isize) as ::core::ffi::c_uint == OTGR_FONT_FLAG
    {
        b = new_null_box();
        p = new_native_character(f, c as UnicodeScalar);
        (*mem.offset((b + 5 as int32_t) as isize)).b32.s1 = p;
        (*mem.offset((b + 3 as int32_t) as isize)).b32.s1 =
            (*mem.offset((p + 3 as int32_t) as isize)).b32.s1;
        (*mem.offset((b + 1 as int32_t) as isize)).b32.s1 =
            (*mem.offset((p + 1 as int32_t) as isize)).b32.s1;
        if (*mem.offset((p + 2 as int32_t) as isize)).b32.s1 < 0 as int32_t {
            (*mem.offset((b + 2 as int32_t) as isize)).b32.s1 = 0 as ::core::ffi::c_int as int32_t;
        } else {
            (*mem.offset((b + 2 as int32_t) as isize)).b32.s1 =
                (*mem.offset((p + 2 as int32_t) as isize)).b32.s1;
        }
    } else {
        q = (*font_info.offset(
            (*char_base.offset(f as isize)
                + effective_char(1 as ::core::ffi::c_int != 0, f, c as uint16_t))
                as isize,
        ))
        .b16;
        b = new_null_box();
        (*mem.offset((b + 1 as int32_t) as isize)).b32.s1 = (*font_info
            .offset((*width_base.offset(f as isize) + q.s3 as int32_t) as isize))
        .b32
        .s1 + (*font_info
            .offset((*italic_base.offset(f as isize) + q.s1 as int32_t / 4 as int32_t) as isize))
        .b32
        .s1;
        (*mem.offset((b + 3 as int32_t) as isize)).b32.s1 = (*font_info
            .offset((*height_base.offset(f as isize) + q.s2 as int32_t / 16 as int32_t) as isize))
        .b32
        .s1;
        (*mem.offset((b + 2 as int32_t) as isize)).b32.s1 = (*font_info
            .offset((*depth_base.offset(f as isize) + q.s2 as int32_t % 16 as int32_t) as isize))
        .b32
        .s1;
        p = get_avail();
        (*mem.offset(p as isize)).b16.s0 = c as uint16_t;
        (*mem.offset(p as isize)).b16.s1 = f as uint16_t;
    }
    (*mem.offset((b + 5 as int32_t) as isize)).b32.s1 = p;
    return b;
}
unsafe extern "C" fn stack_into_box(mut b: int32_t, mut f: internal_font_number, mut c: uint16_t) {
    let mut p: int32_t = 0;
    p = char_box(f, c as int32_t);
    (*mem.offset(p as isize)).b32.s1 = (*mem.offset((b + 5 as int32_t) as isize)).b32.s1;
    (*mem.offset((b + 5 as int32_t) as isize)).b32.s1 = p;
    (*mem.offset((b + 3 as int32_t) as isize)).b32.s1 =
        (*mem.offset((p + 3 as int32_t) as isize)).b32.s1;
}
unsafe extern "C" fn height_plus_depth(mut f: internal_font_number, mut c: uint16_t) -> scaled_t {
    let mut q: b16x4 = (*font_info.offset(
        (*char_base.offset(f as isize) + effective_char(1 as ::core::ffi::c_int != 0, f, c))
            as isize,
    ))
    .b16;
    return (*font_info
        .offset((*height_base.offset(f as isize) + q.s2 as int32_t / 16 as int32_t) as isize))
    .b32
    .s1 as scaled_t
        + (*font_info
            .offset((*depth_base.offset(f as isize) + q.s2 as int32_t % 16 as int32_t) as isize))
        .b32
        .s1 as scaled_t;
}
unsafe extern "C" fn stack_glyph_into_box(
    mut b: int32_t,
    mut f: internal_font_number,
    mut g: int32_t,
) {
    let mut p: int32_t = 0;
    let mut q: int32_t = 0;
    p = get_node(GLYPH_NODE_SIZE as int32_t);
    (*mem.offset(p as isize)).b16.s1 = WHATSIT_NODE as uint16_t;
    (*mem.offset(p as isize)).b16.s0 = GLYPH_NODE as uint16_t;
    (*mem.offset((p + 4 as int32_t) as isize)).b16.s2 = f as uint16_t;
    (*mem.offset((p + 4 as int32_t) as isize)).b16.s1 = g as uint16_t;
    measure_native_glyph(
        mem.offset(p as isize) as *mut memory_word as *mut ::core::ffi::c_void,
        1 as ::core::ffi::c_int,
    );
    if (*mem.offset(b as isize)).b16.s1 as ::core::ffi::c_int == HLIST_NODE {
        q = (*mem.offset((b + 5 as int32_t) as isize)).b32.s1;
        if q == TEX_NULL as int32_t {
            (*mem.offset((b + 5 as int32_t) as isize)).b32.s1 = p;
        } else {
            while (*mem.offset(q as isize)).b32.s1 != TEX_NULL as int32_t {
                q = (*mem.offset(q as isize)).b32.s1;
            }
            (*mem.offset(q as isize)).b32.s1 = p;
            if (*mem.offset((b + 3 as int32_t) as isize)).b32.s1
                < (*mem.offset((p + 3 as int32_t) as isize)).b32.s1
            {
                (*mem.offset((b + 3 as int32_t) as isize)).b32.s1 =
                    (*mem.offset((p + 3 as int32_t) as isize)).b32.s1;
            }
            if (*mem.offset((b + 2 as int32_t) as isize)).b32.s1
                < (*mem.offset((p + 2 as int32_t) as isize)).b32.s1
            {
                (*mem.offset((b + 2 as int32_t) as isize)).b32.s1 =
                    (*mem.offset((p + 2 as int32_t) as isize)).b32.s1;
            }
        }
    } else {
        (*mem.offset(p as isize)).b32.s1 = (*mem.offset((b + 5 as int32_t) as isize)).b32.s1;
        (*mem.offset((b + 5 as int32_t) as isize)).b32.s1 = p;
        (*mem.offset((b + 3 as int32_t) as isize)).b32.s1 =
            (*mem.offset((p + 3 as int32_t) as isize)).b32.s1;
        if (*mem.offset((b + 1 as int32_t) as isize)).b32.s1
            < (*mem.offset((p + 1 as int32_t) as isize)).b32.s1
        {
            (*mem.offset((b + 1 as int32_t) as isize)).b32.s1 =
                (*mem.offset((p + 1 as int32_t) as isize)).b32.s1;
        }
    };
}
unsafe extern "C" fn stack_glue_into_box(mut b: int32_t, mut min: scaled_t, mut max: scaled_t) {
    let mut p: int32_t = 0;
    let mut q: int32_t = 0;
    q = new_spec(0 as int32_t);
    (*mem.offset((q + 1 as int32_t) as isize)).b32.s1 = min as int32_t;
    (*mem.offset((q + 2 as int32_t) as isize)).b32.s1 = (max - min) as int32_t;
    p = new_glue(q);
    if (*mem.offset(b as isize)).b16.s1 as ::core::ffi::c_int == HLIST_NODE {
        q = (*mem.offset((b + 5 as int32_t) as isize)).b32.s1;
        if q == TEX_NULL as int32_t {
            (*mem.offset((b + 5 as int32_t) as isize)).b32.s1 = p;
        } else {
            while (*mem.offset(q as isize)).b32.s1 != TEX_NULL as int32_t {
                q = (*mem.offset(q as isize)).b32.s1;
            }
            (*mem.offset(q as isize)).b32.s1 = p;
        }
    } else {
        (*mem.offset(p as isize)).b32.s1 = (*mem.offset((b + 5 as int32_t) as isize)).b32.s1;
        (*mem.offset((b + 5 as int32_t) as isize)).b32.s1 = p;
        (*mem.offset((b + 3 as int32_t) as isize)).b32.s1 =
            (*mem.offset((p + 3 as int32_t) as isize)).b32.s1;
        (*mem.offset((b + 1 as int32_t) as isize)).b32.s1 =
            (*mem.offset((p + 1 as int32_t) as isize)).b32.s1;
    };
}
unsafe extern "C" fn build_opentype_assembly(
    mut f: internal_font_number,
    mut a: *mut ::core::ffi::c_void,
    mut s: scaled_t,
    mut horiz: bool,
) -> int32_t {
    let mut b: int32_t = 0;
    let mut n: int32_t = 0;
    let mut i: int32_t = 0;
    let mut j: int32_t = 0;
    let mut g: int32_t = 0;
    let mut p: int32_t = 0;
    let mut s_max: scaled_t = 0;
    let mut o: scaled_t = 0;
    let mut oo: scaled_t = 0;
    let mut prev_o: scaled_t = 0;
    let mut min_o: scaled_t = 0;
    let mut no_extenders: bool = false;
    let mut nat: scaled_t = 0;
    let mut str: scaled_t = 0;
    b = new_null_box();
    if horiz {
        (*mem.offset(b as isize)).b16.s1 = HLIST_NODE as uint16_t;
    } else {
        (*mem.offset(b as isize)).b16.s1 = VLIST_NODE as uint16_t;
    }
    n = -(1 as ::core::ffi::c_int) as int32_t;
    no_extenders = true_0 != 0;
    min_o = ot_min_connector_overlap(f as ::core::ffi::c_int) as scaled_t;
    loop {
        n = n + 1 as int32_t;
        s_max = 0 as ::core::ffi::c_int as scaled_t;
        prev_o = 0 as ::core::ffi::c_int as scaled_t;
        let mut for_end: int32_t = 0;
        i = 0 as ::core::ffi::c_int as int32_t;
        for_end = (ot_part_count(a as *const GlyphAssembly) - 1 as ::core::ffi::c_int) as int32_t;
        if i <= for_end {
            loop {
                if ot_part_is_extender(a as *const GlyphAssembly, i as ::core::ffi::c_int) {
                    no_extenders = false_0 != 0;
                    let mut for_end_0: int32_t = 0;
                    j = 1 as ::core::ffi::c_int as int32_t;
                    for_end_0 = n;
                    if j <= for_end_0 {
                        loop {
                            o = ot_part_start_connector(
                                f as ::core::ffi::c_int,
                                a as *const GlyphAssembly,
                                i as ::core::ffi::c_int,
                            ) as scaled_t;
                            if min_o < o {
                                o = min_o;
                            }
                            if prev_o < o {
                                o = prev_o;
                            }
                            s_max = s_max - o
                                + ot_part_full_advance(
                                    f as ::core::ffi::c_int,
                                    a as *const GlyphAssembly,
                                    i as ::core::ffi::c_int,
                                ) as scaled_t;
                            prev_o = ot_part_end_connector(
                                f as ::core::ffi::c_int,
                                a as *const GlyphAssembly,
                                i as ::core::ffi::c_int,
                            ) as scaled_t;
                            let fresh2 = j;
                            j = j + 1;
                            if !(fresh2 < for_end_0) {
                                break;
                            }
                        }
                    }
                } else {
                    o = ot_part_start_connector(
                        f as ::core::ffi::c_int,
                        a as *const GlyphAssembly,
                        i as ::core::ffi::c_int,
                    ) as scaled_t;
                    if min_o < o {
                        o = min_o;
                    }
                    if prev_o < o {
                        o = prev_o;
                    }
                    s_max = s_max - o
                        + ot_part_full_advance(
                            f as ::core::ffi::c_int,
                            a as *const GlyphAssembly,
                            i as ::core::ffi::c_int,
                        ) as scaled_t;
                    prev_o = ot_part_end_connector(
                        f as ::core::ffi::c_int,
                        a as *const GlyphAssembly,
                        i as ::core::ffi::c_int,
                    ) as scaled_t;
                }
                let fresh3 = i;
                i = i + 1;
                if !(fresh3 < for_end) {
                    break;
                }
            }
        }
        if s_max >= s || no_extenders as ::core::ffi::c_int != 0 {
            break;
        }
    }
    prev_o = 0 as ::core::ffi::c_int as scaled_t;
    let mut for_end_1: int32_t = 0;
    i = 0 as ::core::ffi::c_int as int32_t;
    for_end_1 = (ot_part_count(a as *const GlyphAssembly) - 1 as ::core::ffi::c_int) as int32_t;
    if i <= for_end_1 {
        loop {
            if ot_part_is_extender(a as *const GlyphAssembly, i as ::core::ffi::c_int) {
                let mut for_end_2: int32_t = 0;
                j = 1 as ::core::ffi::c_int as int32_t;
                for_end_2 = n;
                if j <= for_end_2 {
                    loop {
                        o = ot_part_start_connector(
                            f as ::core::ffi::c_int,
                            a as *const GlyphAssembly,
                            i as ::core::ffi::c_int,
                        ) as scaled_t;
                        if prev_o < o {
                            o = prev_o;
                        }
                        oo = o;
                        if min_o < o {
                            o = min_o;
                        }
                        if oo > 0 as scaled_t {
                            stack_glue_into_box(b, -oo, -o);
                        }
                        g = ot_part_glyph(a as *const GlyphAssembly, i as ::core::ffi::c_int)
                            as int32_t;
                        stack_glyph_into_box(b, f, g);
                        prev_o = ot_part_end_connector(
                            f as ::core::ffi::c_int,
                            a as *const GlyphAssembly,
                            i as ::core::ffi::c_int,
                        ) as scaled_t;
                        let fresh4 = j;
                        j = j + 1;
                        if !(fresh4 < for_end_2) {
                            break;
                        }
                    }
                }
            } else {
                o = ot_part_start_connector(
                    f as ::core::ffi::c_int,
                    a as *const GlyphAssembly,
                    i as ::core::ffi::c_int,
                ) as scaled_t;
                if prev_o < o {
                    o = prev_o;
                }
                oo = o;
                if min_o < o {
                    o = min_o;
                }
                if oo > 0 as scaled_t {
                    stack_glue_into_box(b, -oo, -o);
                }
                g = ot_part_glyph(a as *const GlyphAssembly, i as ::core::ffi::c_int) as int32_t;
                stack_glyph_into_box(b, f, g);
                prev_o = ot_part_end_connector(
                    f as ::core::ffi::c_int,
                    a as *const GlyphAssembly,
                    i as ::core::ffi::c_int,
                ) as scaled_t;
            }
            let fresh5 = i;
            i = i + 1;
            if !(fresh5 < for_end_1) {
                break;
            }
        }
    }
    p = (*mem.offset((b + 5 as int32_t) as isize)).b32.s1;
    nat = 0 as ::core::ffi::c_int as scaled_t;
    str = 0 as ::core::ffi::c_int as scaled_t;
    while p != TEX_NULL as int32_t {
        if (*mem.offset(p as isize)).b16.s1 as ::core::ffi::c_int == WHATSIT_NODE {
            if horiz {
                nat = nat + (*mem.offset((p + 1 as int32_t) as isize)).b32.s1 as scaled_t;
            } else {
                nat = nat
                    + (*mem.offset((p + 3 as int32_t) as isize)).b32.s1 as scaled_t
                    + (*mem.offset((p + 2 as int32_t) as isize)).b32.s1 as scaled_t;
            }
        } else if (*mem.offset(p as isize)).b16.s1 as ::core::ffi::c_int == GLUE_NODE {
            nat = nat
                + (*mem.offset(
                    ((*mem.offset((p + 1 as int32_t) as isize)).b32.s0 + 1 as int32_t) as isize,
                ))
                .b32
                .s1 as scaled_t;
            str = str
                + (*mem.offset(
                    ((*mem.offset((p + 1 as int32_t) as isize)).b32.s0 + 2 as int32_t) as isize,
                ))
                .b32
                .s1 as scaled_t;
        }
        p = (*mem.offset(p as isize)).b32.s1;
    }
    o = 0 as ::core::ffi::c_int as scaled_t;
    if s > nat && str > 0 as scaled_t {
        o = s - nat;
        if o > str {
            o = str;
        }
        (*mem.offset((b + 5 as int32_t) as isize)).b16.s0 = NORMAL as uint16_t;
        (*mem.offset((b + 5 as int32_t) as isize)).b16.s1 = STRETCHING as uint16_t;
        (*mem.offset((b + 6 as int32_t) as isize)).gr =
            o as ::core::ffi::c_double / str as ::core::ffi::c_double;
        if horiz {
            (*mem.offset((b + 1 as int32_t) as isize)).b32.s1 =
                (nat + tex_round(
                    str as ::core::ffi::c_double * (*mem.offset((b + 6 as int32_t) as isize)).gr,
                ) as scaled_t) as int32_t;
        } else {
            (*mem.offset((b + 3 as int32_t) as isize)).b32.s1 =
                (nat + tex_round(
                    str as ::core::ffi::c_double * (*mem.offset((b + 6 as int32_t) as isize)).gr,
                ) as scaled_t) as int32_t;
        }
    } else if horiz {
        (*mem.offset((b + 1 as int32_t) as isize)).b32.s1 = nat as int32_t;
    } else {
        (*mem.offset((b + 3 as int32_t) as isize)).b32.s1 = nat as int32_t;
    }
    return b;
}
unsafe extern "C" fn rebox(mut b: int32_t, mut w: scaled_t) -> int32_t {
    let mut p: int32_t = 0;
    let mut f: internal_font_number = 0;
    let mut v: scaled_t = 0;
    if (*mem.offset((b + 1 as int32_t) as isize)).b32.s1 != w
        && (*mem.offset((b + 5 as int32_t) as isize)).b32.s1 != TEX_NULL as int32_t
    {
        if (*mem.offset(b as isize)).b16.s1 as ::core::ffi::c_int == VLIST_NODE {
            b = hpack(b, 0 as scaled_t, ADDITIONAL as small_number);
        }
        p = (*mem.offset((b + 5 as int32_t) as isize)).b32.s1;
        if is_char_node(p) as ::core::ffi::c_int != 0
            && (*mem.offset(p as isize)).b32.s1 == TEX_NULL as int32_t
        {
            f = (*mem.offset(p as isize)).b16.s1 as internal_font_number;
            v = (*font_info.offset(
                (*width_base.offset(f as isize)
                    + (*font_info.offset(
                        (*char_base.offset(f as isize)
                            + effective_char(
                                1 as ::core::ffi::c_int != 0,
                                f,
                                (*mem.offset(p as isize)).b16.s0,
                            )) as isize,
                    ))
                    .b16
                    .s3 as int32_t) as isize,
            ))
            .b32
            .s1 as scaled_t;
            if v != (*mem.offset((b + 1 as int32_t) as isize)).b32.s1 {
                (*mem.offset(p as isize)).b32.s1 =
                    new_kern((*mem.offset((b + 1 as int32_t) as isize)).b32.s1 as scaled_t - v);
            }
        }
        free_node(b, BOX_NODE_SIZE as int32_t);
        b = new_glue(12 as int32_t);
        (*mem.offset(b as isize)).b32.s1 = p;
        while (*mem.offset(p as isize)).b32.s1 != TEX_NULL as int32_t {
            p = (*mem.offset(p as isize)).b32.s1;
        }
        (*mem.offset(p as isize)).b32.s1 = new_glue(12 as int32_t);
        return hpack(b, w, EXACTLY as small_number);
    } else {
        (*mem.offset((b + 1 as int32_t) as isize)).b32.s1 = w as int32_t;
        return b;
    };
}
pub const true_0: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const false_0: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
