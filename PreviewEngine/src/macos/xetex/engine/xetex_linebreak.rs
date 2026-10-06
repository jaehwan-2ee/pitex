/* Copyright 2016-2018 The Tectonic Project
 * Licensed under the MIT License.
 */
// Translated from xetex/engine/xetex-linebreak.c with C2Rust 0.22.1.
extern "C" {
    pub type ttbc_diagnostic_t;
    fn abs(_: ::core::ffi::c_int) -> ::core::ffi::c_int;
    fn measure_native_node(node: *mut ::core::ffi::c_void, use_glyph_metrics: ::core::ffi::c_int);
    static mut eqtb: *mut memory_word;
    static mut str_pool: *mut packed_UTF16_code;
    static mut str_start: *mut pool_pointer;
    static mut help_line: [*const ::core::ffi::c_char; 6];
    static mut help_ptr: ::core::ffi::c_uchar;
    static mut arith_error: bool;
    static mut temp_ptr: int32_t;
    static mut mem: *mut memory_word;
    static mut hi_mem_min: int32_t;
    static mut avail: int32_t;
    static mut last_leftmost_char: int32_t;
    static mut last_rightmost_char: int32_t;
    static mut hlist_stack: [int32_t; 513];
    static mut hlist_stack_level: ::core::ffi::c_short;
    static mut first_p: int32_t;
    static mut global_prev_p: int32_t;
    static mut font_in_short_display: int32_t;
    static mut cur_list: list_state_record;
    static mut font_info: *mut memory_word;
    static mut hyphen_char: *mut int32_t;
    static mut bchar_label: *mut font_index;
    static mut font_bchar: *mut nine_bits;
    static mut char_base: *mut int32_t;
    static mut width_base: *mut int32_t;
    static mut lig_kern_base: *mut int32_t;
    static mut kern_base: *mut int32_t;
    static mut adjust_tail: int32_t;
    static mut pre_adjust_tail: int32_t;
    static mut pack_begin_line: int32_t;
    static mut just_box: int32_t;
    static mut active_width: [scaled_t; 7];
    static mut hc: [int32_t; 4099];
    static mut hf: internal_font_number;
    static mut hu: [int32_t; 4097];
    static mut cur_lang: ::core::ffi::c_uchar;
    static mut max_hyph_char: int32_t;
    static mut hyf: [::core::ffi::c_uchar; 4097];
    static mut init_list: int32_t;
    static mut init_lig: bool;
    static mut init_lft: bool;
    static mut hyphen_passed: small_number;
    static mut cur_r: int32_t;
    static mut cur_l: int32_t;
    static mut cur_q: int32_t;
    static mut lig_stack: int32_t;
    static mut ligature_present: bool;
    static mut rt_hit: bool;
    static mut lft_hit: bool;
    static mut trie_trl: *mut trie_pointer;
    static mut trie_tro: *mut trie_pointer;
    static mut trie_trc: *mut uint16_t;
    static mut hyf_distance: [small_number; 35112];
    static mut hyf_num: [small_number; 35112];
    static mut hyf_next: [trie_opcode; 35112];
    static mut op_start: [int32_t; 256];
    static mut hyph_word: *mut str_number;
    static mut hyph_list: *mut int32_t;
    static mut hyph_link: *mut hyph_pointer;
    static mut trie_not_ready: bool;
    static mut hyph_start: trie_pointer;
    static mut hyph_index: trie_pointer;
    static mut xtx_ligature_present: bool;
    static mut semantic_pagination_enabled: bool;
    fn badness(t: scaled_t, s: scaled_t) -> int32_t;
    fn get_avail() -> int32_t;
    fn flush_list(p: int32_t);
    fn get_node(s: int32_t) -> int32_t;
    fn free_node(p: int32_t, s: int32_t);
    fn new_ligature(f: internal_font_number, c: uint16_t, q: int32_t) -> int32_t;
    fn new_lig_item(c: uint16_t) -> int32_t;
    fn new_disc() -> int32_t;
    fn new_math(w: scaled_t, s: small_number) -> int32_t;
    fn new_spec(p: int32_t) -> int32_t;
    fn new_param_glue(n: small_number) -> int32_t;
    fn new_kern(w: scaled_t) -> int32_t;
    fn new_penalty(m: int32_t) -> int32_t;
    fn prev_rightmost(s: int32_t, e: int32_t) -> int32_t;
    fn delete_glue_ref(p: int32_t);
    fn flush_node_list(p: int32_t);
    fn pop_nest();
    fn effective_char(err_p: bool, f: internal_font_number, c: uint16_t) -> int32_t;
    fn fract(x: int32_t, n: int32_t, d: int32_t, max_answer: int32_t) -> int32_t;
    fn new_native_word_node(f: internal_font_number, n: int32_t) -> int32_t;
    fn new_native_character(f: internal_font_number, c: UnicodeScalar) -> int32_t;
    fn new_character(f: internal_font_number, c: UTF16_code) -> int32_t;
    fn char_pw(p: int32_t, side: small_number) -> scaled_t;
    fn new_margin_kern(w: scaled_t, p: int32_t, side: small_number) -> int32_t;
    fn hpack(p: int32_t, w: scaled_t, m: small_number) -> int32_t;
    fn pitex_special_node(text: *const ::core::ffi::c_char) -> int32_t;
    fn append_to_vlist(b: int32_t);
    fn max_hyphenatable_length() -> int32_t;
    fn init_trie();
    fn error();
    fn confusion(s: *const ::core::ffi::c_char) -> !;
    fn pdf_error(t: *const ::core::ffi::c_char, p: *const ::core::ffi::c_char) -> !;
    fn capture_to_diagnostic(diagnostic: *mut ttbc_diagnostic_t);
    fn error_here_with_diagnostic(message: *const ::core::ffi::c_char) -> *mut ttbc_diagnostic_t;
    fn length(s: str_number) -> int32_t;
}
pub type int32_t = i32;
pub type uint16_t = u16;
pub type scaled_t = int32_t;
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
pub type internal_font_number = int32_t;
pub type font_index = int32_t;
pub type nine_bits = int32_t;
pub type trie_pointer = int32_t;
pub type trie_opcode = ::core::ffi::c_ushort;
pub type hyph_pointer = ::core::ffi::c_ushort;
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
pub const BIGGEST_CHAR: ::core::ffi::c_int = 0xffff as ::core::ffi::c_int;
pub const BIGGEST_USV: ::core::ffi::c_int = 0x10ffff as ::core::ffi::c_int;
pub const NUMBER_USVS: ::core::ffi::c_int = BIGGEST_USV + 1 as ::core::ffi::c_int;
pub const TEMP_HEAD: ::core::ffi::c_int = MEM_TOP - 3 as ::core::ffi::c_int;
pub const HOLD_HEAD: ::core::ffi::c_int = MEM_TOP - 4 as ::core::ffi::c_int;
pub const ADJUST_HEAD: ::core::ffi::c_int = MEM_TOP - 5 as ::core::ffi::c_int;
pub const ACTIVE_LIST: ::core::ffi::c_int = MEM_TOP - 7 as ::core::ffi::c_int;
pub const PRE_ADJUST_HEAD: ::core::ffi::c_int = MEM_TOP - 14 as ::core::ffi::c_int;
pub const DELTA_NODE: ::core::ffi::c_int = RULE_NODE;
pub const PASSIVE_NODE_SIZE: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const SMALL_NODE_SIZE: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const ACTIVE_NODE_SIZE_NORMAL: ::core::ffi::c_int = 3 as ::core::ffi::c_int;
pub const ACTIVE_NODE_SIZE_EXTENDED: ::core::ffi::c_int = 5 as ::core::ffi::c_int;
pub const NATIVE_NODE_SIZE: ::core::ffi::c_int = 6 as ::core::ffi::c_int;
pub const DELTA_NODE_SIZE: ::core::ffi::c_int = 7 as ::core::ffi::c_int;
pub const L_CODE: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
pub const LIG_TAG: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const NORMAL: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const EXACTLY: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const NON_ADDRESS: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const UNHYPHENATED: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const ADDITIONAL: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const HYPHENATED: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const MAX_HLIST_STACK: ::core::ffi::c_int = 512 as ::core::ffi::c_int;
pub const HYPH_PRIME: ::core::ffi::c_int = 607 as ::core::ffi::c_int;
pub const EJECT_PENALTY: ::core::ffi::c_int = -(10000 as ::core::ffi::c_int);
pub const INF_BAD: ::core::ffi::c_int = 10000 as ::core::ffi::c_int;
pub const INF_PENALTY: ::core::ffi::c_int = 10000 as ::core::ffi::c_int;
pub const TOO_BIG_CHAR: ::core::ffi::c_int = 65536 as ::core::ffi::c_int;
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
pub const INT_PAR__inter_line_penalty: ::core::ffi::c_int = 13 as ::core::ffi::c_int;
pub const INT_PAR__double_hyphen_demerits: ::core::ffi::c_int = 14 as ::core::ffi::c_int;
pub const INT_PAR__final_hyphen_demerits: ::core::ffi::c_int = 15 as ::core::ffi::c_int;
pub const INT_PAR__adj_demerits: ::core::ffi::c_int = 16 as ::core::ffi::c_int;
pub const INT_PAR__looseness: ::core::ffi::c_int = 19 as ::core::ffi::c_int;
pub const INT_PAR__uc_hyph: ::core::ffi::c_int = 38 as ::core::ffi::c_int;
pub const INT_PAR__hang_after: ::core::ffi::c_int = 41 as ::core::ffi::c_int;
pub const INT_PAR__last_line_fit: ::core::ffi::c_int = 62 as ::core::ffi::c_int;
pub const INT_PAR__xetex_protrude_chars: ::core::ffi::c_int = 68 as ::core::ffi::c_int;
pub const INT_PAR__texxet: ::core::ffi::c_int = 69 as ::core::ffi::c_int;
pub const INT_PARS: ::core::ffi::c_int = 89 as ::core::ffi::c_int;
pub const DIMEN_PAR__hsize: ::core::ffi::c_int = 3 as ::core::ffi::c_int;
pub const DIMEN_PAR__hang_indent: ::core::ffi::c_int = 17 as ::core::ffi::c_int;
pub const DIMEN_PAR__emergency_stretch: ::core::ffi::c_int = 20 as ::core::ffi::c_int;
pub const GLUE_PAR__left_skip: ::core::ffi::c_int = 7 as ::core::ffi::c_int;
pub const GLUE_PAR__right_skip: ::core::ffi::c_int = 8 as ::core::ffi::c_int;
pub const GLUE_PAR__par_fill_skip: ::core::ffi::c_int = 14 as ::core::ffi::c_int;
pub const LOCAL__par_shape: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const ETEX_PENALTIES_PAR__inter_line_penalties: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const ETEX_PENALTIES_PAR__club_penalties: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const ETEX_PENALTIES_PAR__widow_penalties: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const ETEX_PENALTIES_PAR__display_widow_penalties: ::core::ffi::c_int = 3 as ::core::ffi::c_int;
pub const GLUE_BASE: ::core::ffi::c_int = 2254340 as ::core::ffi::c_int;
pub const LOCAL_BASE: ::core::ffi::c_int = 2254871 as ::core::ffi::c_int;
pub const ETEX_PEN_BASE: ::core::ffi::c_int = 2255140 as ::core::ffi::c_int;
pub const LC_CODE_BASE: ::core::ffi::c_int = 3370281 as ::core::ffi::c_int;
pub const INT_BASE: ::core::ffi::c_int = 7826729 as ::core::ffi::c_int;
pub const COUNT_BASE: ::core::ffi::c_int = INT_BASE + INT_PARS;
pub const DEL_CODE_BASE: ::core::ffi::c_int = COUNT_BASE + 256 as ::core::ffi::c_int;
pub const DIMEN_BASE: ::core::ffi::c_int = DEL_CODE_BASE + NUMBER_USVS;
pub const HLIST_NODE: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const VLIST_NODE: ::core::ffi::c_int = 1;
pub const RULE_NODE: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const INS_NODE: ::core::ffi::c_int = 3 as ::core::ffi::c_int;
pub const MARK_NODE: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
pub const ADJUST_NODE: ::core::ffi::c_int = 5 as ::core::ffi::c_int;
pub const LIGATURE_NODE: ::core::ffi::c_int = 6;
pub const DISC_NODE: ::core::ffi::c_int = 7 as ::core::ffi::c_int;
pub const WHATSIT_NODE: ::core::ffi::c_int = 8;
pub const MATH_NODE: ::core::ffi::c_int = 9 as ::core::ffi::c_int;
pub const GLUE_NODE: ::core::ffi::c_int = 10 as ::core::ffi::c_int;
pub const KERN_NODE: ::core::ffi::c_int = 11 as ::core::ffi::c_int;
pub const PENALTY_NODE: ::core::ffi::c_int = 12 as ::core::ffi::c_int;
pub const EXPLICIT: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const SPACE_ADJUSTMENT: ::core::ffi::c_int = 3 as ::core::ffi::c_int;
pub const LANGUAGE_NODE: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
pub const NATIVE_WORD_NODE: ::core::ffi::c_int = 40 as ::core::ffi::c_int;
pub const NATIVE_WORD_NODE_AT: ::core::ffi::c_int = 41 as ::core::ffi::c_int;
pub const GLYPH_NODE: ::core::ffi::c_int = 42 as ::core::ffi::c_int;
pub const PIC_NODE: ::core::ffi::c_int = 43 as ::core::ffi::c_int;
pub const PDF_NODE: ::core::ffi::c_int = 44 as ::core::ffi::c_int;
pub const MIN_TRIE_OP: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
#[inline]
unsafe extern "C" fn is_char_node(p: int32_t) -> bool {
    return p >= hi_mem_min;
}
#[inline]
unsafe extern "C" fn is_non_discardable_node(p: int32_t) -> bool {
    return ((*mem.offset(p as isize)).b16.s1 as ::core::ffi::c_int) < MATH_NODE;
}
pub const AWFUL_BAD: ::core::ffi::c_int = 0x3fffffff as ::core::ffi::c_int;
pub const VERY_LOOSE_FIT: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const LOOSE_FIT: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const DECENT_FIT: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const TIGHT_FIT: ::core::ffi::c_int = 3 as ::core::ffi::c_int;
static mut passive: int32_t = 0;
static mut cur_active_width: [scaled_t; 7] = [0; 7];
static mut background: [scaled_t; 7] = [0; 7];
static mut break_width: [scaled_t; 7] = [0; 7];
static mut best_place: [int32_t; 4] = [0; 4];
static mut best_pl_line: [int32_t; 4] = [0; 4];
static mut disc_width: scaled_t = 0;
static mut no_shrink_error_yet: bool = false;
static mut cur_p: int32_t = 0;
static mut second_pass: bool = false;
static mut final_pass: bool = false;
static mut threshold: int32_t = 0;
static mut minimal_demerits: [int32_t; 4] = [0; 4];
static mut minimum_demerits: int32_t = 0;
static mut easy_line: int32_t = 0;
static mut last_special_line: int32_t = 0;
static mut first_width: scaled_t = 0;
static mut second_width: scaled_t = 0;
static mut first_indent: scaled_t = 0;
static mut second_indent: scaled_t = 0;
static mut best_bet: int32_t = 0;
static mut fewest_demerits: int32_t = 0;
static mut best_line: int32_t = 0;
static mut actual_looseness: int32_t = 0;
static mut line_diff: int32_t = 0;
static mut hn: small_number = 0;
static mut ha: int32_t = 0;
static mut hb: int32_t = 0;
static mut hyf_char: int32_t = 0;
static mut init_cur_lang: ::core::ffi::c_uchar = 0;
static mut r_hyf: int32_t = 0;
static mut l_hyf: int32_t = 0;
static mut init_r_hyf: int32_t = 0;
static mut init_l_hyf: int32_t = 0;
static mut hyf_bchar: int32_t = 0;
static mut last_line_fill: int32_t = 0;
static mut do_last_line_fit: bool = false;
static mut active_node_size: small_number = 0;
static mut fill_width: [scaled_t; 3] = [0; 3];
static mut best_pl_short: [scaled_t; 4] = [0; 4];
static mut best_pl_glue: [scaled_t; 4] = [0; 4];
pub const LAST_ACTIVE: ::core::ffi::c_int = ACTIVE_LIST;
#[inline]
unsafe extern "C" fn get_native_usv(mut p: int32_t, mut i: int32_t) -> UnicodeScalar {
    let mut c: ::core::ffi::c_ushort = *(mem.offset((p + NATIVE_NODE_SIZE as int32_t) as isize)
        as *mut memory_word as *mut ::core::ffi::c_ushort)
        .offset(i as isize);
    if c as ::core::ffi::c_int >= 0xd800 as ::core::ffi::c_int
        && (c as ::core::ffi::c_int) < 0xdc00 as ::core::ffi::c_int
    {
        return 0x10000 as UnicodeScalar
            + (c as UnicodeScalar - 0xd800 as UnicodeScalar) * 0x400 as UnicodeScalar
            + *(mem.offset((p + NATIVE_NODE_SIZE as int32_t) as isize) as *mut memory_word
                as *mut ::core::ffi::c_ushort)
                .offset((i + 1 as int32_t) as isize) as UnicodeScalar
            - 0xdc00 as UnicodeScalar;
    }
    return c as UnicodeScalar;
}
#[no_mangle]
pub unsafe extern "C" fn line_break(mut d: bool) {
    let mut current_block: u64;
    let mut auto_breaking: bool = false;
    let mut prev_p: int32_t = 0;
    let mut q: int32_t = 0;
    let mut r: int32_t = 0;
    let mut s: int32_t = 0;
    let mut prev_s: int32_t = 0;
    let mut f: internal_font_number = 0;
    let mut j: small_number = 0;
    let mut c: UnicodeScalar = 0;
    let mut l: int32_t = 0;
    let mut i: int32_t = 0;
    let mut for_end_1: int32_t = 0;
    let typography_head = crate::engine_fonts::apply_spacing((*mem.offset(cur_list.head as isize)).b32.s1);
    (*mem.offset(cur_list.head as isize)).b32.s1 = typography_head;
    let mut typography_tail = typography_head;
    if typography_tail != TEX_NULL { while (*mem.offset(typography_tail as isize)).b32.s1 != TEX_NULL { typography_tail = (*mem.offset(typography_tail as isize)).b32.s1; } cur_list.tail = typography_tail; }
    pack_begin_line = cur_list.mode_line;
    (*mem.offset((4999999 as ::core::ffi::c_int - 3 as ::core::ffi::c_int) as isize))
        .b32
        .s1 = (*mem.offset(cur_list.head as isize)).b32.s1;
    if is_char_node(cur_list.tail) {
        let ref mut fresh0 = (*mem.offset(cur_list.tail as isize)).b32.s1;
        *fresh0 = new_penalty(INF_PENALTY as int32_t);
        cur_list.tail = *fresh0;
    } else if (*mem.offset(cur_list.tail as isize)).b16.s1 as ::core::ffi::c_int != GLUE_NODE {
        let ref mut fresh1 = (*mem.offset(cur_list.tail as isize)).b32.s1;
        *fresh1 = new_penalty(INF_PENALTY as int32_t);
        cur_list.tail = *fresh1;
    } else {
        (*mem.offset(cur_list.tail as isize)).b16.s1 = PENALTY_NODE as uint16_t;
        delete_glue_ref(
            (*mem.offset((cur_list.tail + 1 as int32_t) as isize))
                .b32
                .s0,
        );
        flush_node_list(
            (*mem.offset((cur_list.tail + 1 as int32_t) as isize))
                .b32
                .s1,
        );
        (*mem.offset((cur_list.tail + 1 as int32_t) as isize))
            .b32
            .s1 = INF_PENALTY as int32_t;
    }
    let ref mut fresh2 = (*mem.offset(cur_list.tail as isize)).b32.s1;
    *fresh2 = new_param_glue(GLUE_PAR__par_fill_skip as small_number);
    last_line_fill = *fresh2;
    init_cur_lang = (cur_list.prev_graf as ::core::ffi::c_long % 65536 as ::core::ffi::c_long)
        as ::core::ffi::c_uchar;
    init_l_hyf = cur_list.prev_graf / 0x400000 as int32_t;
    init_r_hyf = (cur_list.prev_graf as ::core::ffi::c_long / 65536 as ::core::ffi::c_long
        % 64 as ::core::ffi::c_long) as int32_t;
    pop_nest();
    no_shrink_error_yet = true_0 != 0;
    if (*mem.offset(
        (*eqtb.offset((2254340 as ::core::ffi::c_int + 7 as ::core::ffi::c_int) as isize))
            .b32
            .s1 as isize,
    ))
    .b16
    .s0 as ::core::ffi::c_int
        != NORMAL
        && (*mem.offset(
            ((*eqtb.offset((2254340 as ::core::ffi::c_int + 7 as ::core::ffi::c_int) as isize))
                .b32
                .s1
                + 3 as int32_t) as isize,
        ))
        .b32
        .s1 != 0 as int32_t
    {
        (*eqtb.offset((GLUE_BASE + GLUE_PAR__left_skip) as isize))
            .b32
            .s1 = finite_shrink(
            (*eqtb.offset((GLUE_BASE + GLUE_PAR__left_skip) as isize))
                .b32
                .s1,
        );
    }
    if (*mem.offset(
        (*eqtb.offset((2254340 as ::core::ffi::c_int + 8 as ::core::ffi::c_int) as isize))
            .b32
            .s1 as isize,
    ))
    .b16
    .s0 as ::core::ffi::c_int
        != NORMAL
        && (*mem.offset(
            ((*eqtb.offset((2254340 as ::core::ffi::c_int + 8 as ::core::ffi::c_int) as isize))
                .b32
                .s1
                + 3 as int32_t) as isize,
        ))
        .b32
        .s1 != 0 as int32_t
    {
        (*eqtb.offset((GLUE_BASE + GLUE_PAR__right_skip) as isize))
            .b32
            .s1 = finite_shrink(
            (*eqtb.offset((GLUE_BASE + GLUE_PAR__right_skip) as isize))
                .b32
                .s1,
        );
    }
    q = (*eqtb.offset((GLUE_BASE + GLUE_PAR__left_skip) as isize))
        .b32
        .s1;
    r = (*eqtb.offset((GLUE_BASE + GLUE_PAR__right_skip) as isize))
        .b32
        .s1;
    background[1 as ::core::ffi::c_int as usize] =
        ((*mem.offset((q + 1 as int32_t) as isize)).b32.s1
            + (*mem.offset((r + 1 as int32_t) as isize)).b32.s1) as scaled_t;
    background[2 as ::core::ffi::c_int as usize] = 0 as ::core::ffi::c_int as scaled_t;
    background[3 as ::core::ffi::c_int as usize] = 0 as ::core::ffi::c_int as scaled_t;
    background[4 as ::core::ffi::c_int as usize] = 0 as ::core::ffi::c_int as scaled_t;
    background[5 as ::core::ffi::c_int as usize] = 0 as ::core::ffi::c_int as scaled_t;
    background[(2 as ::core::ffi::c_int + (*mem.offset(q as isize)).b16.s1 as ::core::ffi::c_int)
        as usize] = (*mem.offset((q + 2 as int32_t) as isize)).b32.s1 as scaled_t;
    background[(2 as ::core::ffi::c_int + (*mem.offset(r as isize)).b16.s1 as ::core::ffi::c_int)
        as usize] += (*mem.offset((r + 2 as int32_t) as isize)).b32.s1;
    background[6 as ::core::ffi::c_int as usize] =
        ((*mem.offset((q + 3 as int32_t) as isize)).b32.s1
            + (*mem.offset((r + 3 as int32_t) as isize)).b32.s1) as scaled_t;
    do_last_line_fit = false_0 != 0;
    active_node_size = ACTIVE_NODE_SIZE_NORMAL as small_number;
    if (*eqtb.offset((INT_BASE + INT_PAR__last_line_fit) as isize))
        .b32
        .s1
        > 0 as int32_t
    {
        q = (*mem.offset((last_line_fill + 1 as int32_t) as isize))
            .b32
            .s0;
        if (*mem.offset((q + 2 as int32_t) as isize)).b32.s1 > 0 as int32_t
            && (*mem.offset(q as isize)).b16.s1 as ::core::ffi::c_int > NORMAL
        {
            if background[3 as ::core::ffi::c_int as usize] == 0 as scaled_t
                && background[4 as ::core::ffi::c_int as usize] == 0 as scaled_t
                && background[5 as ::core::ffi::c_int as usize] == 0 as scaled_t
            {
                do_last_line_fit = true_0 != 0;
                active_node_size = ACTIVE_NODE_SIZE_EXTENDED as small_number;
                fill_width[0 as ::core::ffi::c_int as usize] = 0 as ::core::ffi::c_int as scaled_t;
                fill_width[1 as ::core::ffi::c_int as usize] = 0 as ::core::ffi::c_int as scaled_t;
                fill_width[2 as ::core::ffi::c_int as usize] = 0 as ::core::ffi::c_int as scaled_t;
                fill_width[((*mem.offset(q as isize)).b16.s1 as ::core::ffi::c_int
                    - 1 as ::core::ffi::c_int) as usize] =
                    (*mem.offset((q + 2 as int32_t) as isize)).b32.s1 as scaled_t;
            }
        }
    }
    minimum_demerits = AWFUL_BAD as int32_t;
    minimal_demerits[TIGHT_FIT as usize] = AWFUL_BAD as int32_t;
    minimal_demerits[DECENT_FIT as usize] = AWFUL_BAD as int32_t;
    minimal_demerits[LOOSE_FIT as usize] = AWFUL_BAD as int32_t;
    minimal_demerits[VERY_LOOSE_FIT as usize] = AWFUL_BAD as int32_t;
    if (*eqtb.offset((LOCAL_BASE + LOCAL__par_shape) as isize))
        .b32
        .s1
        == TEX_NULL as int32_t
    {
        if (*eqtb.offset((DIMEN_BASE + DIMEN_PAR__hang_indent) as isize))
            .b32
            .s1
            == 0 as int32_t
        {
            last_special_line = 0 as ::core::ffi::c_int as int32_t;
            second_width = (*eqtb.offset((DIMEN_BASE + DIMEN_PAR__hsize) as isize))
                .b32
                .s1 as scaled_t;
            second_indent = 0 as ::core::ffi::c_int as scaled_t;
        } else {
            last_special_line = abs((*eqtb.offset((INT_BASE + INT_PAR__hang_after) as isize))
                .b32
                .s1 as ::core::ffi::c_int) as int32_t;
            if (*eqtb.offset((INT_BASE + INT_PAR__hang_after) as isize))
                .b32
                .s1
                < 0 as int32_t
            {
                first_width = ((*eqtb.offset((DIMEN_BASE + DIMEN_PAR__hsize) as isize))
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
                    >= 0 as int32_t
                {
                    first_indent = (*eqtb.offset((DIMEN_BASE + DIMEN_PAR__hang_indent) as isize))
                        .b32
                        .s1 as scaled_t;
                } else {
                    first_indent = 0 as ::core::ffi::c_int as scaled_t;
                }
                second_width = (*eqtb.offset((DIMEN_BASE + DIMEN_PAR__hsize) as isize))
                    .b32
                    .s1 as scaled_t;
                second_indent = 0 as ::core::ffi::c_int as scaled_t;
            } else {
                first_width = (*eqtb.offset((DIMEN_BASE + DIMEN_PAR__hsize) as isize))
                    .b32
                    .s1 as scaled_t;
                first_indent = 0 as ::core::ffi::c_int as scaled_t;
                second_width = ((*eqtb.offset((DIMEN_BASE + DIMEN_PAR__hsize) as isize))
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
                    >= 0 as int32_t
                {
                    second_indent = (*eqtb.offset((DIMEN_BASE + DIMEN_PAR__hang_indent) as isize))
                        .b32
                        .s1 as scaled_t;
                } else {
                    second_indent = 0 as ::core::ffi::c_int as scaled_t;
                }
            }
        }
    } else {
        last_special_line = (*mem.offset(
            (*eqtb.offset((2254871 as ::core::ffi::c_int + 0 as ::core::ffi::c_int) as isize))
                .b32
                .s1 as isize,
        ))
        .b32
        .s0 - 1 as int32_t;
        second_width = (*mem.offset(
            ((*eqtb.offset((LOCAL_BASE + LOCAL__par_shape) as isize))
                .b32
                .s1
                + 2 as int32_t * (last_special_line + 1 as int32_t)) as isize,
        ))
        .b32
        .s1 as scaled_t;
        second_indent = (*mem.offset(
            ((*eqtb.offset((LOCAL_BASE + LOCAL__par_shape) as isize))
                .b32
                .s1
                + 2 as int32_t * last_special_line
                + 1 as int32_t) as isize,
        ))
        .b32
        .s1 as scaled_t;
    }
    if (*eqtb.offset((INT_BASE + INT_PAR__looseness) as isize))
        .b32
        .s1
        == 0 as int32_t
    {
        easy_line = last_special_line;
    } else {
        easy_line = MAX_HALFWORD as int32_t;
    }
    threshold = (*eqtb.offset((INT_BASE + INT_PAR__pretolerance) as isize))
        .b32
        .s1;
    if threshold >= 0 as int32_t {
        second_pass = false_0 != 0;
        final_pass = false_0 != 0;
    } else {
        threshold = (*eqtb.offset((INT_BASE + INT_PAR__tolerance) as isize))
            .b32
            .s1;
        second_pass = true_0 != 0;
        final_pass = (*eqtb.offset((DIMEN_BASE + DIMEN_PAR__emergency_stretch) as isize))
            .b32
            .s1
            <= 0 as int32_t;
    }
    loop {
        if threshold > INF_BAD as int32_t {
            threshold = INF_BAD as int32_t;
        }
        if second_pass {
            if trie_not_ready {
                init_trie();
            }
            cur_lang = init_cur_lang;
            l_hyf = init_l_hyf;
            r_hyf = init_r_hyf;
            if *trie_trc.offset((hyph_start + cur_lang as trie_pointer) as isize)
                as ::core::ffi::c_int
                != cur_lang as ::core::ffi::c_int
            {
                hyph_index = 0 as ::core::ffi::c_int as trie_pointer;
            } else {
                hyph_index = *trie_trl.offset((hyph_start + cur_lang as trie_pointer) as isize);
            }
        }
        q = get_node(active_node_size as int32_t);
        (*mem.offset(q as isize)).b16.s1 = UNHYPHENATED as uint16_t;
        (*mem.offset(q as isize)).b16.s0 = DECENT_FIT as uint16_t;
        (*mem.offset(q as isize)).b32.s1 = LAST_ACTIVE as int32_t;
        (*mem.offset((q + 1 as int32_t) as isize)).b32.s1 = TEX_NULL as int32_t;
        (*mem.offset((q + 1 as int32_t) as isize)).b32.s0 = cur_list.prev_graf + 1 as int32_t;
        (*mem.offset((q + 2 as int32_t) as isize)).b32.s1 = 0 as ::core::ffi::c_int as int32_t;
        (*mem.offset((4999999 as ::core::ffi::c_int - 7 as ::core::ffi::c_int) as isize))
            .b32
            .s1 = q;
        if do_last_line_fit {
            (*mem.offset((q + 3 as int32_t) as isize)).b32.s1 = 0 as ::core::ffi::c_int as int32_t;
            (*mem.offset((q + 4 as int32_t) as isize)).b32.s1 = 0 as ::core::ffi::c_int as int32_t;
        }
        active_width[1 as ::core::ffi::c_int as usize] =
            background[1 as ::core::ffi::c_int as usize];
        active_width[2 as ::core::ffi::c_int as usize] =
            background[2 as ::core::ffi::c_int as usize];
        active_width[3 as ::core::ffi::c_int as usize] =
            background[3 as ::core::ffi::c_int as usize];
        active_width[4 as ::core::ffi::c_int as usize] =
            background[4 as ::core::ffi::c_int as usize];
        active_width[5 as ::core::ffi::c_int as usize] =
            background[5 as ::core::ffi::c_int as usize];
        active_width[6 as ::core::ffi::c_int as usize] =
            background[6 as ::core::ffi::c_int as usize];
        passive = TEX_NULL as int32_t;
        font_in_short_display = 0 as ::core::ffi::c_int as int32_t;
        cur_p = (*mem.offset((4999999 as ::core::ffi::c_int - 3 as ::core::ffi::c_int) as isize))
            .b32
            .s1;
        auto_breaking = true_0 != 0;
        global_prev_p = cur_p;
        prev_p = global_prev_p;
        first_p = cur_p;
        while cur_p != TEX_NULL as int32_t
            && (*mem.offset((4999999 as ::core::ffi::c_int - 7 as ::core::ffi::c_int) as isize))
                .b32
                .s1
                != LAST_ACTIVE as int32_t
        {
            if is_char_node(cur_p) {
                global_prev_p = cur_p;
                prev_p = global_prev_p;
                loop {
                    let mut eff_char: int32_t = 0;
                    f = (*mem.offset(cur_p as isize)).b16.s1 as internal_font_number;
                    eff_char = effective_char(true_0 != 0, f, (*mem.offset(cur_p as isize)).b16.s0);
                    active_width[1 as ::core::ffi::c_int as usize] += (*font_info.offset(
                        (*width_base.offset(f as isize)
                            + (*font_info
                                .offset((*char_base.offset(f as isize) + eff_char) as isize))
                            .b16
                            .s3 as int32_t) as isize,
                    ))
                    .b32
                    .s1;
                    cur_p = (*mem.offset(cur_p as isize)).b32.s1;
                    if !is_char_node(cur_p) {
                        break;
                    }
                }
            }
            match (*mem.offset(cur_p as isize)).b16.s1 as ::core::ffi::c_int {
                HLIST_NODE | VLIST_NODE | RULE_NODE => {
                    active_width[1 as ::core::ffi::c_int as usize] +=
                        (*mem.offset((cur_p + 1 as int32_t) as isize)).b32.s1;
                }
                WHATSIT_NODE => {
                    if (*mem.offset(cur_p as isize)).b16.s0 as ::core::ffi::c_int == LANGUAGE_NODE {
                        cur_lang = (*mem.offset((cur_p + 1 as int32_t) as isize)).b32.s1
                            as ::core::ffi::c_uchar;
                        l_hyf = (*mem.offset((cur_p + 1 as int32_t) as isize)).b16.s1 as int32_t;
                        r_hyf = (*mem.offset((cur_p + 1 as int32_t) as isize)).b16.s0 as int32_t;
                        if *trie_trc.offset((hyph_start + cur_lang as trie_pointer) as isize)
                            as ::core::ffi::c_int
                            != cur_lang as ::core::ffi::c_int
                        {
                            hyph_index = 0 as ::core::ffi::c_int as trie_pointer;
                        } else {
                            hyph_index =
                                *trie_trl.offset((hyph_start + cur_lang as trie_pointer) as isize);
                        }
                    } else if (*mem.offset(cur_p as isize)).b16.s0 as ::core::ffi::c_int
                        == NATIVE_WORD_NODE
                        || (*mem.offset(cur_p as isize)).b16.s0 as ::core::ffi::c_int
                            == NATIVE_WORD_NODE_AT
                        || (*mem.offset(cur_p as isize)).b16.s0 as ::core::ffi::c_int == GLYPH_NODE
                        || (*mem.offset(cur_p as isize)).b16.s0 as ::core::ffi::c_int == PIC_NODE
                        || (*mem.offset(cur_p as isize)).b16.s0 as ::core::ffi::c_int == PDF_NODE
                    {
                        active_width[1 as ::core::ffi::c_int as usize] +=
                            (*mem.offset((cur_p + 1 as int32_t) as isize)).b32.s1;
                    }
                }
                GLUE_NODE => {
                    if auto_breaking {
                        if is_char_node(prev_p) {
                            try_break(0 as int32_t, UNHYPHENATED as small_number);
                        } else if is_non_discardable_node(prev_p) {
                            try_break(0 as int32_t, UNHYPHENATED as small_number);
                        } else if (*mem.offset(prev_p as isize)).b16.s1 as ::core::ffi::c_int
                            == KERN_NODE
                            && (*mem.offset(prev_p as isize)).b16.s0 as ::core::ffi::c_int
                                != EXPLICIT
                        {
                            try_break(0 as int32_t, UNHYPHENATED as small_number);
                        }
                    }
                    q = (*mem.offset((cur_p + 1 as int32_t) as isize)).b32.s0;
                    if (*mem.offset(q as isize)).b16.s0 as ::core::ffi::c_int != NORMAL
                        && (*mem.offset((q + 3 as int32_t) as isize)).b32.s1 != 0 as int32_t
                    {
                        let ref mut fresh3 = (*mem.offset((cur_p + 1 as int32_t) as isize)).b32.s0;
                        *fresh3 = finite_shrink(q);
                        q = *fresh3;
                    }
                    active_width[1 as ::core::ffi::c_int as usize] +=
                        (*mem.offset((q + 1 as int32_t) as isize)).b32.s1;
                    active_width[(2 as ::core::ffi::c_int
                        + (*mem.offset(q as isize)).b16.s1 as ::core::ffi::c_int)
                        as usize] += (*mem.offset((q + 2 as int32_t) as isize)).b32.s1;
                    active_width[6 as ::core::ffi::c_int as usize] +=
                        (*mem.offset((q + 3 as int32_t) as isize)).b32.s1;
                    if second_pass as ::core::ffi::c_int != 0
                        && auto_breaking as ::core::ffi::c_int != 0
                    {
                        prev_s = cur_p;
                        s = (*mem.offset(prev_s as isize)).b32.s1;
                        if s != TEX_NULL as int32_t {
                            's_647: loop {
                                if is_char_node(s) {
                                    c = (*mem.offset(s as isize)).b16.s0 as UnicodeScalar;
                                    hf = (*mem.offset(s as isize)).b16.s1 as internal_font_number;
                                    current_block = 2415422468722899689;
                                } else if (*mem.offset(s as isize)).b16.s1 as ::core::ffi::c_int
                                    == LIGATURE_NODE
                                {
                                    if (*mem.offset((s + 1 as int32_t) as isize)).b32.s1
                                        == TEX_NULL as int32_t
                                    {
                                        current_block = 3644428341217566432;
                                    } else {
                                        q = (*mem.offset((s + 1 as int32_t) as isize)).b32.s1;
                                        c = (*mem.offset(q as isize)).b16.s0 as UnicodeScalar;
                                        hf = (*mem.offset(q as isize)).b16.s1
                                            as internal_font_number;
                                        current_block = 2415422468722899689;
                                    }
                                } else if (*mem.offset(s as isize)).b16.s1 as ::core::ffi::c_int
                                    == KERN_NODE
                                    && (*mem.offset(s as isize)).b16.s0 as ::core::ffi::c_int
                                        == NORMAL
                                {
                                    current_block = 3644428341217566432;
                                } else if (*mem.offset(s as isize)).b16.s1 as ::core::ffi::c_int
                                    == MATH_NODE
                                    && (*mem.offset(s as isize)).b16.s0 as ::core::ffi::c_int
                                        >= L_CODE
                                {
                                    current_block = 3644428341217566432;
                                } else {
                                    if !((*mem.offset(s as isize)).b16.s1 as ::core::ffi::c_int
                                        == WHATSIT_NODE)
                                    {
                                        current_block = 12167801934813671140;
                                        break;
                                    }
                                    if (*mem.offset(s as isize)).b16.s0 as ::core::ffi::c_int
                                        == NATIVE_WORD_NODE
                                        || (*mem.offset(s as isize)).b16.s0 as ::core::ffi::c_int
                                            == NATIVE_WORD_NODE_AT
                                    {
                                        l = 0 as ::core::ffi::c_int as int32_t;
                                        while l
                                            < (*mem.offset((s + 4 as int32_t) as isize)).b16.s1
                                                as int32_t
                                        {
                                            c = get_native_usv(s, l);
                                            if (*eqtb.offset(
                                                (LC_CODE_BASE as UnicodeScalar + c) as isize,
                                            ))
                                            .b32
                                            .s1 != 0 as int32_t
                                            {
                                                hf = (*mem.offset((s + 4 as int32_t) as isize))
                                                    .b16
                                                    .s2
                                                    as internal_font_number;
                                                prev_s = s;
                                                if (*eqtb.offset(
                                                    (LC_CODE_BASE as UnicodeScalar + c) as isize,
                                                ))
                                                .b32
                                                .s1 == c
                                                    || (*eqtb.offset(
                                                        (INT_BASE + INT_PAR__uc_hyph) as isize,
                                                    ))
                                                    .b32
                                                    .s1 > 0 as int32_t
                                                {
                                                    current_block = 13248996571649232331;
                                                    break 's_647;
                                                } else {
                                                    current_block = 12167801934813671140;
                                                    break 's_647;
                                                }
                                            } else {
                                                if c as ::core::ffi::c_long
                                                    >= 65536 as ::core::ffi::c_long
                                                {
                                                    l += 1;
                                                }
                                                l += 1;
                                            }
                                        }
                                    }
                                    if (*mem.offset(s as isize)).b16.s0 as ::core::ffi::c_int
                                        == LANGUAGE_NODE
                                    {
                                        cur_lang = (*mem.offset((s + 1 as int32_t) as isize)).b32.s1
                                            as ::core::ffi::c_uchar;
                                        l_hyf = (*mem.offset((s + 1 as int32_t) as isize)).b16.s1
                                            as int32_t;
                                        r_hyf = (*mem.offset((s + 1 as int32_t) as isize)).b16.s0
                                            as int32_t;
                                        if *trie_trc.offset(
                                            (hyph_start + cur_lang as trie_pointer) as isize,
                                        )
                                            as ::core::ffi::c_int
                                            != cur_lang as ::core::ffi::c_int
                                        {
                                            hyph_index = 0 as ::core::ffi::c_int as trie_pointer;
                                        } else {
                                            hyph_index = *trie_trl.offset(
                                                (hyph_start + cur_lang as trie_pointer) as isize,
                                            );
                                        }
                                    }
                                    current_block = 3644428341217566432;
                                }
                                match current_block {
                                    2415422468722899689 => {
                                        if hyph_index == 0 as trie_pointer
                                            || c > 255 as UnicodeScalar
                                        {
                                            hc[0 as ::core::ffi::c_int as usize] = (*eqtb.offset(
                                                (LC_CODE_BASE as UnicodeScalar + c) as isize,
                                            ))
                                            .b32
                                            .s1;
                                        } else if *trie_trc
                                            .offset((hyph_index + c as trie_pointer) as isize)
                                            as UnicodeScalar
                                            != c
                                        {
                                            hc[0 as ::core::ffi::c_int as usize] =
                                                0 as ::core::ffi::c_int as int32_t;
                                        } else {
                                            hc[0 as ::core::ffi::c_int as usize] = *trie_tro
                                                .offset((hyph_index + c as trie_pointer) as isize)
                                                as int32_t;
                                        }
                                        if hc[0 as ::core::ffi::c_int as usize] != 0 as int32_t {
                                            if hc[0 as ::core::ffi::c_int as usize] == c
                                                || (*eqtb
                                                    .offset((INT_BASE + INT_PAR__uc_hyph) as isize))
                                                .b32
                                                .s1 > 0 as int32_t
                                            {
                                                current_block = 13248996571649232331;
                                                break;
                                            } else {
                                                current_block = 12167801934813671140;
                                                break;
                                            }
                                        }
                                    }
                                    _ => {}
                                }
                                prev_s = s;
                                s = (*mem.offset(prev_s as isize)).b32.s1;
                            }
                            match current_block {
                                12167801934813671140 => {}
                                _ => {
                                    hyf_char = *hyphen_char.offset(hf as isize);
                                    if !(hyf_char < 0 as int32_t) {
                                        if !(hyf_char > BIGGEST_CHAR as int32_t) {
                                            ha = prev_s;
                                            if !(l_hyf + r_hyf > max_hyphenatable_length()) {
                                                if ha != TEX_NULL as int32_t
                                                    && ha < hi_mem_min
                                                    && (*mem.offset(ha as isize)).b16.s1
                                                        as ::core::ffi::c_int
                                                        == WHATSIT_NODE
                                                    && ((*mem.offset(ha as isize)).b16.s0
                                                        as ::core::ffi::c_int
                                                        == NATIVE_WORD_NODE
                                                        || (*mem.offset(ha as isize)).b16.s0
                                                            as ::core::ffi::c_int
                                                            == NATIVE_WORD_NODE_AT)
                                                {
                                                    s = (*mem.offset(ha as isize)).b32.s1;
                                                    loop {
                                                        if !is_char_node(s) {
                                                            match (*mem.offset(s as isize)).b16.s1
                                                                as ::core::ffi::c_int
                                                            {
                                                                LIGATURE_NODE => {}
                                                                KERN_NODE => {
                                                                    if (*mem.offset(s as isize))
                                                                        .b16
                                                                        .s0
                                                                        as ::core::ffi::c_int
                                                                        != NORMAL
                                                                    {
                                                                        current_block =
                                                                            14302005989611905025;
                                                                        break;
                                                                    }
                                                                }
                                                                WHATSIT_NODE | GLUE_NODE
                                                                | PENALTY_NODE | INS_NODE
                                                                | ADJUST_NODE | MARK_NODE => {
                                                                    current_block =
                                                                        14302005989611905025;
                                                                    break;
                                                                }
                                                                _ => {
                                                                    current_block =
                                                                        12167801934813671140;
                                                                    break;
                                                                }
                                                            }
                                                        }
                                                        s = (*mem.offset(s as isize)).b32.s1;
                                                    }
                                                    match current_block {
                                                        12167801934813671140 => {}
                                                        _ => {
                                                            hn = 0 as small_number;
                                                            '_restart: loop {
                                                                for_end_1 = (*mem.offset(
                                                                    (ha + 4 as int32_t) as isize,
                                                                ))
                                                                .b16
                                                                .s1
                                                                    as int32_t;
                                                                l = 0 as ::core::ffi::c_int
                                                                    as int32_t;
                                                                loop {
                                                                    if !(l < for_end_1) {
                                                                        break '_restart;
                                                                    }
                                                                    c = get_native_usv(ha, l);
                                                                    if hyph_index
                                                                        == 0 as trie_pointer
                                                                        || c > 255 as UnicodeScalar
                                                                    {
                                                                        hc[0 as ::core::ffi::c_int
                                                                            as usize] = (*eqtb
                                                                            .offset(
                                                                                (LC_CODE_BASE
                                                                                    as UnicodeScalar
                                                                                    + c)
                                                                                    as isize,
                                                                            ))
                                                                        .b32
                                                                        .s1;
                                                                    } else if *trie_trc.offset(
                                                                        (hyph_index
                                                                            + c as trie_pointer)
                                                                            as isize,
                                                                    )
                                                                        as UnicodeScalar
                                                                        != c
                                                                    {
                                                                        hc[0 as ::core::ffi::c_int
                                                                            as usize] = 0
                                                                            as ::core::ffi::c_int
                                                                            as int32_t;
                                                                    } else {
                                                                        hc[0 as ::core::ffi::c_int
                                                                            as usize] = *trie_tro
                                                                            .offset(
                                                                            (hyph_index
                                                                                + c as trie_pointer)
                                                                                as isize,
                                                                        )
                                                                            as int32_t;
                                                                    }
                                                                    if hc[0 as ::core::ffi::c_int
                                                                        as usize]
                                                                        == 0 as int32_t
                                                                    {
                                                                        if hn as ::core::ffi::c_int > 0 as ::core::ffi::c_int {
                                                                            q = new_native_word_node(
                                                                                hf,
                                                                                (*mem.offset((ha + 4 as int32_t) as isize)).b16.s1
                                                                                    as int32_t - l,
                                                                            );
                                                                            (*mem.offset(q as isize)).b16.s0 = (*mem
                                                                                .offset(ha as isize))
                                                                                .b16
                                                                                .s0;
                                                                            i = l;
                                                                            while i
                                                                                < (*mem.offset((ha + 4 as int32_t) as isize)).b16.s1
                                                                                    as int32_t
                                                                            {
                                                                                *(mem.offset((q + NATIVE_NODE_SIZE as int32_t) as isize)
                                                                                    as *mut memory_word as *mut ::core::ffi::c_ushort)
                                                                                    .offset((i - l) as isize) = *(mem
                                                                                    .offset((ha + NATIVE_NODE_SIZE as int32_t) as isize)
                                                                                    as *mut memory_word as *mut ::core::ffi::c_ushort)
                                                                                    .offset(i as isize);
                                                                                i += 1;
                                                                            }
                                                                            measure_native_node(
                                                                                mem.offset(q as isize) as *mut memory_word
                                                                                    as *mut ::core::ffi::c_void,
                                                                                ((*eqtb
                                                                                    .offset(
                                                                                        (7826729 as ::core::ffi::c_int + 72 as ::core::ffi::c_int)
                                                                                            as isize,
                                                                                    ))
                                                                                    .b32
                                                                                    .s1 > 0 as int32_t) as ::core::ffi::c_int,
                                                                            );
                                                                            (*mem.offset(q as isize)).b32.s1 = (*mem
                                                                                .offset(ha as isize))
                                                                                .b32
                                                                                .s1;
                                                                            (*mem.offset(ha as isize)).b32.s1 = q;
                                                                            (*mem.offset((ha + 4 as int32_t) as isize)).b16.s1 = l
                                                                                as uint16_t;
                                                                            measure_native_node(
                                                                                mem.offset(ha as isize) as *mut memory_word
                                                                                    as *mut ::core::ffi::c_void,
                                                                                ((*eqtb
                                                                                    .offset(
                                                                                        (7826729 as ::core::ffi::c_int + 72 as ::core::ffi::c_int)
                                                                                            as isize,
                                                                                    ))
                                                                                    .b32
                                                                                    .s1 > 0 as int32_t) as ::core::ffi::c_int,
                                                                            );
                                                                            break '_restart;
                                                                        }
                                                                    } else if hn
                                                                        as ::core::ffi::c_int
                                                                        == 0 as ::core::ffi::c_int
                                                                        && l > 0 as int32_t
                                                                    {
                                                                        q = new_native_word_node(
                                                                            hf,
                                                                            (*mem.offset(
                                                                                (ha + 4 as int32_t)
                                                                                    as isize,
                                                                            ))
                                                                            .b16
                                                                            .s1
                                                                                as int32_t
                                                                                - l,
                                                                        );
                                                                        (*mem.offset(q as isize))
                                                                            .b16
                                                                            .s0 = (*mem
                                                                            .offset(ha as isize))
                                                                        .b16
                                                                        .s0;
                                                                        i = l;
                                                                        while i
                                                                            < (*mem.offset(
                                                                                (ha + 4 as int32_t)
                                                                                    as isize,
                                                                            ))
                                                                            .b16
                                                                            .s1
                                                                                as int32_t
                                                                        {
                                                                            *(mem.offset((q + NATIVE_NODE_SIZE as int32_t) as isize)
                                                                                as *mut memory_word as *mut ::core::ffi::c_ushort)
                                                                                .offset((i - l) as isize) = *(mem
                                                                                .offset((ha + NATIVE_NODE_SIZE as int32_t) as isize)
                                                                                as *mut memory_word as *mut ::core::ffi::c_ushort)
                                                                                .offset(i as isize);
                                                                            i += 1;
                                                                        }
                                                                        measure_native_node(
                                                                            mem.offset(q as isize) as *mut memory_word
                                                                                as *mut ::core::ffi::c_void,
                                                                            ((*eqtb
                                                                                .offset(
                                                                                    (7826729 as ::core::ffi::c_int + 72 as ::core::ffi::c_int)
                                                                                        as isize,
                                                                                ))
                                                                                .b32
                                                                                .s1 > 0 as int32_t) as ::core::ffi::c_int,
                                                                        );
                                                                        (*mem.offset(q as isize))
                                                                            .b32
                                                                            .s1 = (*mem
                                                                            .offset(ha as isize))
                                                                        .b32
                                                                        .s1;
                                                                        (*mem
                                                                            .offset(ha as isize))
                                                                        .b32
                                                                        .s1 = q;
                                                                        (*mem.offset(
                                                                            (ha + 4 as int32_t)
                                                                                as isize,
                                                                        ))
                                                                        .b16
                                                                        .s1 = l as uint16_t;
                                                                        measure_native_node(
                                                                            mem.offset(ha as isize) as *mut memory_word
                                                                                as *mut ::core::ffi::c_void,
                                                                            ((*eqtb
                                                                                .offset(
                                                                                    (7826729 as ::core::ffi::c_int + 72 as ::core::ffi::c_int)
                                                                                        as isize,
                                                                                ))
                                                                                .b32
                                                                                .s1 > 0 as int32_t) as ::core::ffi::c_int,
                                                                        );
                                                                        ha = (*mem
                                                                            .offset(ha as isize))
                                                                        .b32
                                                                        .s1;
                                                                        break;
                                                                    } else {
                                                                        if hn as int32_t == max_hyphenatable_length() {
                                                                            break '_restart;
                                                                        }
                                                                        hn += 1;
                                                                        if (c as ::core::ffi::c_long) < 65536 as ::core::ffi::c_long
                                                                        {
                                                                            hu[hn as usize] = c as int32_t;
                                                                            hc[hn as usize] = hc[0 as ::core::ffi::c_int as usize];
                                                                        } else {
                                                                            hu[hn as usize] = ((c as ::core::ffi::c_long
                                                                                - 65536 as ::core::ffi::c_long)
                                                                                / 1024 as ::core::ffi::c_long
                                                                                + 0xd800 as ::core::ffi::c_long) as int32_t;
                                                                            hc[hn as usize] = ((hc[0 as ::core::ffi::c_int as usize]
                                                                                as ::core::ffi::c_long - 65536 as ::core::ffi::c_long)
                                                                                / 1024 as ::core::ffi::c_long
                                                                                + 0xd800 as ::core::ffi::c_long) as int32_t;
                                                                            hn += 1;
                                                                            hu[hn as usize] = (c % 1024 as UnicodeScalar
                                                                                + 0xdc00 as UnicodeScalar) as int32_t;
                                                                            hc[hn as usize] = hc[0 as ::core::ffi::c_int as usize]
                                                                                % 1024 as int32_t + 0xdc00 as int32_t;
                                                                            l += 1;
                                                                        }
                                                                        hyf_bchar =
                                                                            TOO_BIG_CHAR as int32_t;
                                                                    }
                                                                    l += 1;
                                                                }
                                                            }
                                                            current_block = 10453323034968249808;
                                                        }
                                                    }
                                                } else {
                                                    hn = 0 as small_number;
                                                    's_1128: loop {
                                                        if is_char_node(s) {
                                                            if (*mem.offset(s as isize)).b16.s1
                                                                as internal_font_number
                                                                != hf
                                                            {
                                                                break;
                                                            }
                                                            hyf_bchar =
                                                                (*mem.offset(s as isize)).b16.s0
                                                                    as int32_t;
                                                            c = hyf_bchar as UnicodeScalar;
                                                            if hyph_index == 0 as trie_pointer
                                                                || c > 255 as UnicodeScalar
                                                            {
                                                                hc[0 as ::core::ffi::c_int
                                                                    as usize] = (*eqtb.offset(
                                                                    (LC_CODE_BASE as UnicodeScalar
                                                                        + c)
                                                                        as isize,
                                                                ))
                                                                .b32
                                                                .s1;
                                                            } else if *trie_trc.offset(
                                                                (hyph_index + c as trie_pointer)
                                                                    as isize,
                                                            )
                                                                as UnicodeScalar
                                                                != c
                                                            {
                                                                hc[0 as ::core::ffi::c_int
                                                                    as usize] = 0
                                                                    as ::core::ffi::c_int
                                                                    as int32_t;
                                                            } else {
                                                                hc[0 as ::core::ffi::c_int
                                                                    as usize] = *trie_tro.offset(
                                                                    (hyph_index + c as trie_pointer)
                                                                        as isize,
                                                                )
                                                                    as int32_t;
                                                            }
                                                            if hc[0 as ::core::ffi::c_int as usize]
                                                                == 0 as int32_t
                                                            {
                                                                break;
                                                            }
                                                            if hc[0 as ::core::ffi::c_int as usize]
                                                                > max_hyph_char
                                                            {
                                                                break;
                                                            }
                                                            if hn as int32_t
                                                                == max_hyphenatable_length()
                                                            {
                                                                break;
                                                            }
                                                            hb = s;
                                                            hn += 1;
                                                            hu[hn as usize] = c as int32_t;
                                                            hc[hn as usize] = hc
                                                                [0 as ::core::ffi::c_int as usize];
                                                            hyf_bchar = TOO_BIG_CHAR as int32_t;
                                                        } else if (*mem.offset(s as isize)).b16.s1
                                                            as ::core::ffi::c_int
                                                            == LIGATURE_NODE
                                                        {
                                                            if (*mem.offset(
                                                                (s + 1 as int32_t) as isize,
                                                            ))
                                                            .b16
                                                            .s1
                                                                as internal_font_number
                                                                != hf
                                                            {
                                                                break;
                                                            }
                                                            j = hn;
                                                            q = (*mem.offset(
                                                                (s + 1 as int32_t) as isize,
                                                            ))
                                                            .b32
                                                            .s1;
                                                            if q > TEX_NULL as int32_t {
                                                                hyf_bchar = (*mem
                                                                    .offset(q as isize))
                                                                .b16
                                                                .s0
                                                                    as int32_t;
                                                            }
                                                            while q > TEX_NULL as int32_t {
                                                                c = (*mem.offset(q as isize)).b16.s0
                                                                    as UnicodeScalar;
                                                                if hyph_index == 0 as trie_pointer
                                                                    || c > 255 as UnicodeScalar
                                                                {
                                                                    hc[0 as ::core::ffi::c_int
                                                                        as usize] = (*eqtb.offset(
                                                                        (LC_CODE_BASE
                                                                            as UnicodeScalar
                                                                            + c)
                                                                            as isize,
                                                                    ))
                                                                    .b32
                                                                    .s1;
                                                                } else if *trie_trc.offset(
                                                                    (hyph_index + c as trie_pointer)
                                                                        as isize,
                                                                )
                                                                    as UnicodeScalar
                                                                    != c
                                                                {
                                                                    hc[0 as ::core::ffi::c_int
                                                                        as usize] = 0
                                                                        as ::core::ffi::c_int
                                                                        as int32_t;
                                                                } else {
                                                                    hc[0 as ::core::ffi::c_int
                                                                        as usize] = *trie_tro
                                                                        .offset(
                                                                            (hyph_index
                                                                                + c as trie_pointer)
                                                                                as isize,
                                                                        )
                                                                        as int32_t;
                                                                }
                                                                if hc[0 as ::core::ffi::c_int
                                                                    as usize]
                                                                    == 0 as int32_t
                                                                {
                                                                    break 's_1128;
                                                                }
                                                                if hc[0 as ::core::ffi::c_int
                                                                    as usize]
                                                                    > max_hyph_char
                                                                {
                                                                    break 's_1128;
                                                                }
                                                                if j as int32_t
                                                                    == max_hyphenatable_length()
                                                                {
                                                                    break 's_1128;
                                                                }
                                                                j += 1;
                                                                hu[j as usize] = c as int32_t;
                                                                hc[j as usize] = hc[0
                                                                    as ::core::ffi::c_int
                                                                    as usize];
                                                                q = (*mem.offset(q as isize))
                                                                    .b32
                                                                    .s1;
                                                            }
                                                            hb = s;
                                                            hn = j;
                                                            if (*mem.offset(s as isize)).b16.s0
                                                                as ::core::ffi::c_int
                                                                & 1 as ::core::ffi::c_int
                                                                != 0
                                                            {
                                                                hyf_bchar = *font_bchar
                                                                    .offset(hf as isize)
                                                                    as int32_t;
                                                            } else {
                                                                hyf_bchar = TOO_BIG_CHAR as int32_t;
                                                            }
                                                        } else {
                                                            if !((*mem.offset(s as isize)).b16.s1
                                                                as ::core::ffi::c_int
                                                                == KERN_NODE
                                                                && (*mem.offset(s as isize)).b16.s0
                                                                    as ::core::ffi::c_int
                                                                    == NORMAL)
                                                            {
                                                                break;
                                                            }
                                                            hb = s;
                                                            hyf_bchar = *font_bchar
                                                                .offset(hf as isize)
                                                                as int32_t;
                                                        }
                                                        s = (*mem.offset(s as isize)).b32.s1;
                                                    }
                                                    current_block = 10453323034968249808;
                                                }
                                                match current_block {
                                                    12167801934813671140 => {}
                                                    _ => {
                                                        if !((hn as int32_t) < l_hyf + r_hyf) {
                                                            loop {
                                                                if !is_char_node(s) {
                                                                    match (*mem.offset(s as isize))
                                                                        .b16
                                                                        .s1
                                                                        as ::core::ffi::c_int
                                                                    {
                                                                        LIGATURE_NODE => {}
                                                                        KERN_NODE => {
                                                                            current_block = 17540340720660137713;
                                                                            match current_block {
                                                                                3396755654921142388 => {
                                                                                    if (*mem.offset(s as isize)).b16.s0 as ::core::ffi::c_int
                                                                                        >= L_CODE
                                                                                    {
                                                                                        current_block = 18142989690447781916;
                                                                                        break;
                                                                                    } else {
                                                                                        current_block = 12167801934813671140;
                                                                                        break;
                                                                                    }
                                                                                }
                                                                                _ => {
                                                                                    if (*mem.offset(s as isize)).b16.s0 as ::core::ffi::c_int
                                                                                        != NORMAL
                                                                                    {
                                                                                        current_block = 18142989690447781916;
                                                                                        break;
                                                                                    }
                                                                                }
                                                                            }
                                                                        }
                                                                        WHATSIT_NODE
                                                                        | GLUE_NODE
                                                                        | PENALTY_NODE
                                                                        | INS_NODE
                                                                        | ADJUST_NODE
                                                                        | MARK_NODE => {
                                                                            current_block = 18142989690447781916;
                                                                            break;
                                                                        }
                                                                        MATH_NODE => {
                                                                            current_block =
                                                                                3396755654921142388;
                                                                            match current_block {
                                                                                3396755654921142388 => {
                                                                                    if (*mem.offset(s as isize)).b16.s0 as ::core::ffi::c_int
                                                                                        >= L_CODE
                                                                                    {
                                                                                        current_block = 18142989690447781916;
                                                                                        break;
                                                                                    } else {
                                                                                        current_block = 12167801934813671140;
                                                                                        break;
                                                                                    }
                                                                                }
                                                                                _ => {
                                                                                    if (*mem.offset(s as isize)).b16.s0 as ::core::ffi::c_int
                                                                                        != NORMAL
                                                                                    {
                                                                                        current_block = 18142989690447781916;
                                                                                        break;
                                                                                    }
                                                                                }
                                                                            }
                                                                        }
                                                                        _ => {
                                                                            current_block = 12167801934813671140;
                                                                            break;
                                                                        }
                                                                    }
                                                                }
                                                                s = (*mem.offset(s as isize))
                                                                    .b32
                                                                    .s1;
                                                            }
                                                            match current_block {
                                                                12167801934813671140 => {}
                                                                _ => {
                                                                    hyphenate();
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
                KERN_NODE => {
                    if (*mem.offset(cur_p as isize)).b16.s0 as ::core::ffi::c_int == EXPLICIT {
                        if !is_char_node((*mem.offset(cur_p as isize)).b32.s1)
                            && auto_breaking as ::core::ffi::c_int != 0
                        {
                            if (*mem.offset((*mem.offset(cur_p as isize)).b32.s1 as isize))
                                .b16
                                .s1 as ::core::ffi::c_int
                                == GLUE_NODE
                            {
                                try_break(0 as int32_t, UNHYPHENATED as small_number);
                            }
                        }
                        active_width[1 as ::core::ffi::c_int as usize] +=
                            (*mem.offset((cur_p + 1 as int32_t) as isize)).b32.s1;
                    } else {
                        active_width[1 as ::core::ffi::c_int as usize] +=
                            (*mem.offset((cur_p + 1 as int32_t) as isize)).b32.s1;
                    }
                }
                LIGATURE_NODE => {
                    f = (*mem.offset((cur_p + 1 as int32_t) as isize)).b16.s1
                        as internal_font_number;
                    xtx_ligature_present = true_0 != 0;
                    active_width[1 as ::core::ffi::c_int as usize] += (*font_info.offset(
                        (*width_base.offset(f as isize)
                            + (*font_info.offset(
                                (*char_base.offset(f as isize)
                                    + effective_char(
                                        1 as ::core::ffi::c_int != 0,
                                        f,
                                        (*mem.offset((cur_p + 1 as int32_t) as isize)).b16.s0,
                                    )) as isize,
                            ))
                            .b16
                            .s3 as int32_t) as isize,
                    ))
                    .b32
                    .s1;
                }
                DISC_NODE => {
                    s = (*mem.offset((cur_p + 1 as int32_t) as isize)).b32.s0;
                    disc_width = 0 as ::core::ffi::c_int as scaled_t;
                    if s == TEX_NULL as int32_t {
                        try_break(
                            (*eqtb.offset((INT_BASE + INT_PAR__ex_hyphen_penalty) as isize))
                                .b32
                                .s1,
                            HYPHENATED as small_number,
                        );
                    } else {
                        loop {
                            if is_char_node(s) {
                                let mut eff_char_0: int32_t = 0;
                                f = (*mem.offset(s as isize)).b16.s1 as internal_font_number;
                                eff_char_0 = effective_char(
                                    true_0 != 0,
                                    f,
                                    (*mem.offset(s as isize)).b16.s0,
                                );
                                disc_width += (*font_info.offset(
                                    (*width_base.offset(f as isize)
                                        + (*font_info.offset(
                                            (*char_base.offset(f as isize) + eff_char_0) as isize,
                                        ))
                                        .b16
                                        .s3 as int32_t)
                                        as isize,
                                ))
                                .b32
                                .s1;
                            } else {
                                match (*mem.offset(s as isize)).b16.s1 as ::core::ffi::c_int {
                                    LIGATURE_NODE => {
                                        let mut eff_char_1: int32_t = 0;
                                        f = (*mem.offset((s + 1 as int32_t) as isize)).b16.s1
                                            as internal_font_number;
                                        xtx_ligature_present = true_0 != 0;
                                        eff_char_1 = effective_char(
                                            true_0 != 0,
                                            f,
                                            (*mem.offset((s + 1 as int32_t) as isize)).b16.s0,
                                        );
                                        disc_width += (*font_info.offset(
                                            (*width_base.offset(f as isize)
                                                + (*font_info.offset(
                                                    (*char_base.offset(f as isize) + eff_char_1)
                                                        as isize,
                                                ))
                                                .b16
                                                .s3
                                                    as int32_t)
                                                as isize,
                                        ))
                                        .b32
                                        .s1;
                                    }
                                    HLIST_NODE | VLIST_NODE | RULE_NODE | KERN_NODE => {
                                        disc_width +=
                                            (*mem.offset((s + 1 as int32_t) as isize)).b32.s1;
                                    }
                                    WHATSIT_NODE => {
                                        if (*mem.offset(s as isize)).b16.s0 as ::core::ffi::c_int
                                            == NATIVE_WORD_NODE
                                            || (*mem.offset(s as isize)).b16.s0
                                                as ::core::ffi::c_int
                                                == NATIVE_WORD_NODE_AT
                                            || (*mem.offset(s as isize)).b16.s0
                                                as ::core::ffi::c_int
                                                == GLYPH_NODE
                                            || (*mem.offset(s as isize)).b16.s0
                                                as ::core::ffi::c_int
                                                == PIC_NODE
                                            || (*mem.offset(s as isize)).b16.s0
                                                as ::core::ffi::c_int
                                                == PDF_NODE
                                        {
                                            disc_width +=
                                                (*mem.offset((s + 1 as int32_t) as isize)).b32.s1;
                                        } else {
                                            confusion(
                                                b"disc3a\0" as *const u8
                                                    as *const ::core::ffi::c_char,
                                            );
                                        }
                                    }
                                    _ => {
                                        confusion(
                                            b"disc3\0" as *const u8 as *const ::core::ffi::c_char,
                                        );
                                    }
                                }
                            }
                            s = (*mem.offset(s as isize)).b32.s1;
                            if !(s != TEX_NULL as int32_t) {
                                break;
                            }
                        }
                        active_width[1 as ::core::ffi::c_int as usize] += disc_width;
                        try_break(
                            (*eqtb.offset((INT_BASE + INT_PAR__hyphen_penalty) as isize))
                                .b32
                                .s1,
                            HYPHENATED as small_number,
                        );
                        active_width[1 as ::core::ffi::c_int as usize] -= disc_width;
                    }
                    r = (*mem.offset(cur_p as isize)).b16.s0 as int32_t;
                    s = (*mem.offset(cur_p as isize)).b32.s1;
                    while r > 0 as int32_t {
                        if is_char_node(s) {
                            let mut eff_char_2: int32_t = 0;
                            f = (*mem.offset(s as isize)).b16.s1 as internal_font_number;
                            eff_char_2 =
                                effective_char(true_0 != 0, f, (*mem.offset(s as isize)).b16.s0);
                            active_width[1 as ::core::ffi::c_int as usize] += (*font_info.offset(
                                (*width_base.offset(f as isize)
                                    + (*font_info.offset(
                                        (*char_base.offset(f as isize) + eff_char_2) as isize,
                                    ))
                                    .b16
                                    .s3 as int32_t) as isize,
                            ))
                            .b32
                            .s1;
                        } else {
                            match (*mem.offset(s as isize)).b16.s1 as ::core::ffi::c_int {
                                LIGATURE_NODE => {
                                    let mut eff_char_3: int32_t = 0;
                                    f = (*mem.offset((s + 1 as int32_t) as isize)).b16.s1
                                        as internal_font_number;
                                    xtx_ligature_present = true_0 != 0;
                                    eff_char_3 = effective_char(
                                        true_0 != 0,
                                        f,
                                        (*mem.offset((s + 1 as int32_t) as isize)).b16.s0,
                                    );
                                    active_width[1 as ::core::ffi::c_int as usize] += (*font_info
                                        .offset(
                                            (*width_base.offset(f as isize)
                                                + (*font_info.offset(
                                                    (*char_base.offset(f as isize) + eff_char_3)
                                                        as isize,
                                                ))
                                                .b16
                                                .s3
                                                    as int32_t)
                                                as isize,
                                        ))
                                    .b32
                                    .s1;
                                }
                                HLIST_NODE | VLIST_NODE | RULE_NODE | KERN_NODE => {
                                    active_width[1 as ::core::ffi::c_int as usize] +=
                                        (*mem.offset((s + 1 as int32_t) as isize)).b32.s1;
                                }
                                WHATSIT_NODE => {
                                    if (*mem.offset(s as isize)).b16.s0 as ::core::ffi::c_int
                                        == NATIVE_WORD_NODE
                                        || (*mem.offset(s as isize)).b16.s0 as ::core::ffi::c_int
                                            == NATIVE_WORD_NODE_AT
                                        || (*mem.offset(s as isize)).b16.s0 as ::core::ffi::c_int
                                            == GLYPH_NODE
                                        || (*mem.offset(s as isize)).b16.s0 as ::core::ffi::c_int
                                            == PIC_NODE
                                        || (*mem.offset(s as isize)).b16.s0 as ::core::ffi::c_int
                                            == PDF_NODE
                                    {
                                        active_width[1 as ::core::ffi::c_int as usize] +=
                                            (*mem.offset((s + 1 as int32_t) as isize)).b32.s1;
                                    } else {
                                        confusion(
                                            b"disc4a\0" as *const u8 as *const ::core::ffi::c_char,
                                        );
                                    }
                                }
                                _ => {
                                    confusion(
                                        b"disc4\0" as *const u8 as *const ::core::ffi::c_char,
                                    );
                                }
                            }
                        }
                        r -= 1;
                        s = (*mem.offset(s as isize)).b32.s1;
                    }
                    global_prev_p = cur_p;
                    prev_p = global_prev_p;
                    cur_p = s;
                    continue;
                }
                MATH_NODE => {
                    if ((*mem.offset(cur_p as isize)).b16.s0 as ::core::ffi::c_int) < L_CODE {
                        auto_breaking = (*mem.offset(cur_p as isize)).b16.s0 as ::core::ffi::c_int
                            & 1 as ::core::ffi::c_int
                            != 0;
                    }
                    if !is_char_node((*mem.offset(cur_p as isize)).b32.s1)
                        && auto_breaking as ::core::ffi::c_int != 0
                    {
                        if (*mem.offset((*mem.offset(cur_p as isize)).b32.s1 as isize))
                            .b16
                            .s1 as ::core::ffi::c_int
                            == GLUE_NODE
                        {
                            try_break(0 as int32_t, UNHYPHENATED as small_number);
                        }
                    }
                    active_width[1 as ::core::ffi::c_int as usize] +=
                        (*mem.offset((cur_p + 1 as int32_t) as isize)).b32.s1;
                }
                PENALTY_NODE => {
                    try_break(
                        (*mem.offset((cur_p + 1 as int32_t) as isize)).b32.s1,
                        UNHYPHENATED as small_number,
                    );
                }
                MARK_NODE | INS_NODE | ADJUST_NODE => {}
                _ => {
                    confusion(b"paragraph\0" as *const u8 as *const ::core::ffi::c_char);
                }
            }
            global_prev_p = cur_p;
            prev_p = global_prev_p;
            cur_p = (*mem.offset(cur_p as isize)).b32.s1;
        }
        if cur_p == TEX_NULL as int32_t {
            try_break(EJECT_PENALTY as int32_t, HYPHENATED as small_number);
            if (*mem.offset((4999999 as ::core::ffi::c_int - 7 as ::core::ffi::c_int) as isize))
                .b32
                .s1
                != LAST_ACTIVE as int32_t
            {
                r = (*mem
                    .offset((4999999 as ::core::ffi::c_int - 7 as ::core::ffi::c_int) as isize))
                .b32
                .s1;
                fewest_demerits = MAX_HALFWORD as int32_t;
                loop {
                    if (*mem.offset(r as isize)).b16.s1 as ::core::ffi::c_int != DELTA_NODE {
                        if (*mem.offset((r + 2 as int32_t) as isize)).b32.s1 < fewest_demerits {
                            fewest_demerits = (*mem.offset((r + 2 as int32_t) as isize)).b32.s1;
                            best_bet = r;
                        }
                    }
                    r = (*mem.offset(r as isize)).b32.s1;
                    if !(r != LAST_ACTIVE as int32_t) {
                        break;
                    }
                }
                best_line = (*mem.offset((best_bet + 1 as int32_t) as isize)).b32.s0;
                if (*eqtb.offset((INT_BASE + INT_PAR__looseness) as isize))
                    .b32
                    .s1
                    == 0 as int32_t
                {
                    break;
                }
                r = (*mem
                    .offset((4999999 as ::core::ffi::c_int - 7 as ::core::ffi::c_int) as isize))
                .b32
                .s1;
                actual_looseness = 0 as ::core::ffi::c_int as int32_t;
                loop {
                    if (*mem.offset(r as isize)).b16.s1 as ::core::ffi::c_int != DELTA_NODE {
                        line_diff = (*mem.offset((r + 1 as int32_t) as isize)).b32.s0 - best_line;
                        if line_diff < actual_looseness
                            && (*eqtb.offset((INT_BASE + INT_PAR__looseness) as isize))
                                .b32
                                .s1
                                <= line_diff
                            || line_diff > actual_looseness
                                && (*eqtb.offset((INT_BASE + INT_PAR__looseness) as isize))
                                    .b32
                                    .s1
                                    >= line_diff
                        {
                            best_bet = r;
                            actual_looseness = line_diff;
                            fewest_demerits = (*mem.offset((r + 2 as int32_t) as isize)).b32.s1;
                        } else if line_diff == actual_looseness
                            && (*mem.offset((r + 2 as int32_t) as isize)).b32.s1 < fewest_demerits
                        {
                            best_bet = r;
                            fewest_demerits = (*mem.offset((r + 2 as int32_t) as isize)).b32.s1;
                        }
                    }
                    r = (*mem.offset(r as isize)).b32.s1;
                    if !(r != LAST_ACTIVE as int32_t) {
                        break;
                    }
                }
                best_line = (*mem.offset((best_bet + 1 as int32_t) as isize)).b32.s0;
                if actual_looseness
                    == (*eqtb.offset((INT_BASE + INT_PAR__looseness) as isize))
                        .b32
                        .s1
                    || final_pass as ::core::ffi::c_int != 0
                {
                    break;
                }
            }
        }
        q = (*mem.offset((4999999 as ::core::ffi::c_int - 7 as ::core::ffi::c_int) as isize))
            .b32
            .s1;
        while q != LAST_ACTIVE as int32_t {
            cur_p = (*mem.offset(q as isize)).b32.s1;
            if (*mem.offset(q as isize)).b16.s1 as ::core::ffi::c_int == DELTA_NODE {
                free_node(q, DELTA_NODE_SIZE as int32_t);
            } else {
                free_node(q, active_node_size as int32_t);
            }
            q = cur_p;
        }
        q = passive;
        while q != TEX_NULL as int32_t {
            cur_p = (*mem.offset(q as isize)).b32.s1;
            free_node(q, PASSIVE_NODE_SIZE as int32_t);
            q = cur_p;
        }
        if !second_pass {
            threshold = (*eqtb.offset((INT_BASE + INT_PAR__tolerance) as isize))
                .b32
                .s1;
            second_pass = true_0 != 0;
            final_pass = (*eqtb.offset((DIMEN_BASE + DIMEN_PAR__emergency_stretch) as isize))
                .b32
                .s1
                <= 0 as int32_t;
        } else {
            background[2 as ::core::ffi::c_int as usize] = background
                [2 as ::core::ffi::c_int as usize]
                + (*eqtb.offset((DIMEN_BASE + DIMEN_PAR__emergency_stretch) as isize))
                    .b32
                    .s1 as scaled_t;
            final_pass = true_0 != 0;
        }
    }
    if do_last_line_fit {
        if (*mem.offset((best_bet + 3 as int32_t) as isize)).b32.s1 == 0 as int32_t {
            do_last_line_fit = false_0 != 0;
        } else {
            q = new_spec(
                (*mem.offset((last_line_fill + 1 as int32_t) as isize))
                    .b32
                    .s0,
            );
            delete_glue_ref(
                (*mem.offset((last_line_fill + 1 as int32_t) as isize))
                    .b32
                    .s0,
            );
            (*mem.offset((q + 1 as int32_t) as isize)).b32.s1 +=
                (*mem.offset((best_bet + 3 as int32_t) as isize)).b32.s1
                    - (*mem.offset((best_bet + 4 as int32_t) as isize)).b32.s1;
            (*mem.offset((q + 2 as int32_t) as isize)).b32.s1 = 0 as ::core::ffi::c_int as int32_t;
            (*mem.offset((last_line_fill + 1 as int32_t) as isize))
                .b32
                .s0 = q;
        }
    }
    post_line_break(d);
    q = (*mem.offset((4999999 as ::core::ffi::c_int - 7 as ::core::ffi::c_int) as isize))
        .b32
        .s1;
    while q != ACTIVE_LIST as int32_t {
        let mut next: int32_t = (*mem.offset(q as isize)).b32.s1;
        if (*mem.offset(q as isize)).b16.s1 as ::core::ffi::c_int == DELTA_NODE {
            free_node(q, DELTA_NODE_SIZE as int32_t);
        } else {
            free_node(q, active_node_size as int32_t);
        }
        q = next;
    }
    q = passive;
    while q != TEX_NULL as int32_t {
        let mut next_0: int32_t = (*mem.offset(q as isize)).b32.s1;
        free_node(q, PASSIVE_NODE_SIZE as int32_t);
        q = next_0;
    }
    pack_begin_line = 0 as ::core::ffi::c_int as int32_t;
}

// Original Pitex font expansion. Capacities and steps use per-mille units.
// Font outlines are expanded by the PDF writer; node advances are adjusted here.
const MARGIN_KERN_NODE: i32 = 40;
const PITEX_EXPANSION_PARAMETER: i32 = 84 + 1;
const PITEX_STRETCH_PARAMETER: i32 = 84 + 2;
const PITEX_SHRINK_PARAMETER: i32 = 84 + 3;
const PITEX_STEP_PARAMETER: i32 = 84 + 4;

unsafe fn pitex_expansion_parameter(n: i32) -> i32 {
    (*eqtb.offset((INT_BASE + n) as isize)).b32.s1
}

unsafe fn pitex_glyph_width(node: int32_t) -> scaled_t {
    let glyph = if is_char_node(node) { node }
        else if (*mem.offset(node as isize)).b16.s1 as i32 == LIGATURE_NODE { node + 1 }
        else { return 0 };
    let font = (*mem.offset(glyph as isize)).b16.s1 as i32;
    let character = effective_char(true, font, (*mem.offset(glyph as isize)).b16.s0);
    let index = (*char_base.offset(font as isize)) + character;
    let width = (*font_info.offset(index as isize)).b16.s3 as i32;
    (*font_info.offset(((*width_base.offset(font as isize)) + width) as isize)).b32.s1
}

unsafe fn pitex_math_transition(node: int32_t, in_math: &mut bool) {
    if !is_char_node(node) && (*mem.offset(node as isize)).b16.s1 as i32 == MATH_NODE {
        match (*mem.offset(node as isize)).b16.s0 as i32 { 0 => *in_math = true, 1 => *in_math = false, _ => {} }
    }
}

unsafe fn pitex_font_expansion(font:i32) -> crate::engine_fonts::Expansion {
    crate::engine_fonts::expansion(font,crate::engine_fonts::Expansion {
        stretch:pitex_expansion_parameter(PITEX_STRETCH_PARAMETER).clamp(0,1000),
        shrink:pitex_expansion_parameter(PITEX_SHRINK_PARAMETER).clamp(0,1000),
        step:pitex_expansion_parameter(PITEX_STEP_PARAMETER).clamp(1,1000),
    })
}
unsafe fn pitex_expansion_glyph(node:i32) -> Option<(i32,i32,i64,bool)> {
    let width=pitex_glyph_width(node) as i64;
    if is_char_node(node) || (*mem.offset(node as isize)).b16.s1 as i32==LIGATURE_NODE {
        let character=if is_char_node(node) {node}else{node+1};
        return Some(((*mem.offset(character as isize)).b16.s1 as i32,(*mem.offset(character as isize)).b16.s0 as i32,width,false));
    }
    if (*mem.offset(node as isize)).b16.s1 as i32==WHATSIT_NODE
        && matches!((*mem.offset(node as isize)).b16.s0 as i32,NATIVE_WORD_NODE|NATIVE_WORD_NODE_AT|GLYPH_NODE) {
        return Some(((*mem.offset((node+4) as isize)).b16.s2 as i32,-1,(*mem.offset((node+1) as isize)).b32.s1 as i64,true));
    }
    None
}
unsafe fn pitex_glyph_capacity(font:i32,character:i32,width:i64,stretching:bool) -> i64 {
    let limits=pitex_font_expansion(font);
    let limit=if stretching {limits.stretch}else{limits.shrink};
    let weight=if character<0 {1000}else{crate::engine_fonts::code(100,font,character)};
    width.max(0)*limit as i64*weight as i64/1_000_000
}
unsafe fn pitex_expand_line(head:int32_t,desired:scaled_t) -> int32_t {
    if pitex_expansion_parameter(PITEX_EXPANSION_PARAMETER)<=0 {return head;}
    let mut node=head;let mut natural=0i64;let mut stretch=0i64;let mut shrink=0i64;
    let mut capacities=[0i64;2];let mut infinite=[false;2];let mut in_math=false;
    while node!=TEX_NULL {
        pitex_math_transition(node,&mut in_math);
        if let Some((font,character,width,_))=pitex_expansion_glyph(node) {
            natural+=width;
            if !in_math {capacities[0]+=pitex_glyph_capacity(font,character,width,true);capacities[1]+=pitex_glyph_capacity(font,character,width,false);}
        }else{
            match (*mem.offset(node as isize)).b16.s1 as i32 {
                HLIST_NODE|VLIST_NODE|RULE_NODE|KERN_NODE|MATH_NODE|MARGIN_KERN_NODE=>natural+=(*mem.offset((node+1) as isize)).b32.s1 as i64,
                GLUE_NODE=>{
                    let glue=(*mem.offset((node+1) as isize)).b32.s0;
                    natural+=(*mem.offset((glue+1) as isize)).b32.s1 as i64;
                    let plus=(*mem.offset((glue+2) as isize)).b32.s1 as i64;
                    let minus=(*mem.offset((glue+3) as isize)).b32.s1 as i64;
                    if (*mem.offset(glue as isize)).b16.s1 as i32==NORMAL {stretch+=plus;}else if plus!=0 {infinite[0]=true;}
                    if (*mem.offset(glue as isize)).b16.s0 as i32==NORMAL {shrink+=minus;}else if minus!=0 {infinite[1]=true;}
                }
                WHATSIT_NODE=>{
                    if matches!((*mem.offset(node as isize)).b16.s0 as i32,PIC_NODE|PDF_NODE) {natural+=(*mem.offset((node+1) as isize)).b32.s1 as i64;}
                }
                _=>{}
            }
        }
        node=(*mem.offset(node as isize)).b32.s1;
    }
    let shortfall=desired as i64-natural;let stretching=shortfall>0;let direction=if stretching {0}else{1};
    let capacity=capacities[direction];
    if shortfall==0 || capacity<=0 || infinite[direction] {return head;}
    let glue=if stretching {stretch.max(0)}else{shrink.max(0)};
    let amount=shortfall.abs().saturating_mul(capacity)/(capacity+glue).max(1);
    let utilization=(amount as f64/capacity as f64).min(1.);
    let sign=if stretching {1i64}else{-1};
    let mut result=head;let mut tail=TEX_NULL;let mut ratio=1.;in_math=false;node=head;
    while node!=TEX_NULL {
        let next=(*mem.offset(node as isize)).b32.s1;
        pitex_math_transition(node,&mut in_math);
        let glyph=if in_math {None}else{pitex_expansion_glyph(node)};
        let node_type=if is_char_node(node) {-1}else{(*mem.offset(node as isize)).b16.s1 as i32};
        let mut change=0i64;
        let wanted=if let Some((font,character,width,_))=glyph {
            let config=pitex_font_expansion(font);
            let limit=if stretching {config.stretch}else{config.shrink};
            let weight=if character<0 {1000}else{crate::engine_fonts::code(100,font,character)};
            let step=config.step.max(1) as i64;
            change=((limit as f64*weight as f64/1000.*utilization) as i64/step*step)*sign;
            1.+change as f64/1000.
        }else if in_math || matches!(node_type,HLIST_NODE|VLIST_NODE){1.}else{ratio};
        if (wanted-ratio).abs()>0.0000001 {
            let text=std::ffi::CString::new(format!("pitex:font-expansion {wanted:.6}")).unwrap();
            let special=pitex_special_node(text.as_ptr());(*mem.offset(special as isize)).b32.s1=node;
            if tail==TEX_NULL {result=special;}else{(*mem.offset(tail as isize)).b32.s1=special;}
            ratio=wanted;
        }
        if let Some((_,_,width,native))=glyph {
            if native {
                (*mem.offset((node+1) as isize)).b32.s1=(width*(1000+change)/1000) as i32;
                if matches!((*mem.offset(node as isize)).b16.s0 as i32,NATIVE_WORD_NODE|NATIVE_WORD_NODE_AT) {
                    let count=(*mem.offset((node+4) as isize)).b16.s0 as usize;
                    let positions=(*mem.offset((node+5) as isize)).ptr as *mut i32;
                    if !positions.is_null() {for i in 0..count {let x=positions.add(i*2);*x=(*x as i64*(1000+change)/1000) as i32;}}
                }
                tail=node;
            }else if change!=0 {
                let kern=new_kern((width*change/1000) as i32);(*mem.offset(kern as isize)).b16.s0=126;(*mem.offset(node as isize)).b32.s1=kern;
                (*mem.offset(kern as isize)).b32.s1=next;tail=kern;
            }else{tail=node;}
        }else{
            if node_type==KERN_NODE && (*mem.offset(node as isize)).b16.s0==0 && !in_math {
                let metric=mem.offset((node+1) as isize);
                (*metric).b32.s1=((*metric).b32.s1 as f64*ratio).round() as i32;
            }
            tail=node;
        }
        node=next;
    }
    if (ratio-1.).abs()>0.0000001 {
        let special=pitex_special_node(b"pitex:font-expansion 1\0".as_ptr().cast());
        (*mem.offset(tail as isize)).b32.s1=special;
    }
    result
}
unsafe fn pitex_list_expansion_capacity(mut node:i32,stop:i32,stretching:bool) -> i64 {
    let mut capacity=0i64;let mut in_math=false;
    while node!=TEX_NULL && node!=stop {
        pitex_math_transition(node,&mut in_math);
        if !in_math {if let Some((font,character,width,_))=pitex_expansion_glyph(node){capacity+=pitex_glyph_capacity(font,character,width,stretching);}}
        node=(*mem.offset(node as isize)).b32.s1;
    }
    capacity
}
unsafe fn pitex_candidate_expansion_capacity(active:int32_t,breakpoint:int32_t,stretching:bool) -> scaled_t {
    if pitex_expansion_parameter(PITEX_EXPANSION_PARAMETER)<2 {return 0;}
    let passive_record=(*mem.offset((active+1) as isize)).b32.s1;
    let previous=if passive_record==TEX_NULL {TEX_NULL}else{(*mem.offset((passive_record+1) as isize)).b32.s1};
    let mut start=if previous==TEX_NULL {(*mem.offset(TEMP_HEAD as isize)).b32.s1}else{(*mem.offset(previous as isize)).b32.s1};
    let mut capacity=0i64;
    if previous!=TEX_NULL && !is_char_node(previous) && (*mem.offset(previous as isize)).b16.s1 as i32==DISC_NODE {
        capacity+=pitex_list_expansion_capacity((*mem.offset((previous+1) as isize)).b32.s1,TEX_NULL,stretching);
        let replacement=(*mem.offset(previous as isize)).b16.s0;
        for _ in 0..replacement {if start!=TEX_NULL {start=(*mem.offset(start as isize)).b32.s1;}}
    }
    capacity+=pitex_list_expansion_capacity(start,breakpoint,stretching);
    if breakpoint!=TEX_NULL && !is_char_node(breakpoint) && (*mem.offset(breakpoint as isize)).b16.s1 as i32==DISC_NODE {
        capacity+=pitex_list_expansion_capacity((*mem.offset((breakpoint+1) as isize)).b32.s0,TEX_NULL,stretching);
    }
    capacity.clamp(0,i32::MAX as i64) as i32
}

unsafe extern "C" fn post_line_break(mut d: bool) {
    let mut q: int32_t = 0;
    let mut r: int32_t = 0;
    let mut s: int32_t = 0;
    let mut p: int32_t = 0;
    let mut k: int32_t = 0;
    let mut w: scaled_t = 0;
    let mut glue_break: bool = false;
    let mut ptmp: int32_t = 0;
    let mut disc_break: bool = false;
    let mut post_disc_break: bool = false;
    let mut cur_width: scaled_t = 0;
    let mut cur_indent: scaled_t = 0;
    let mut t: uint16_t = 0;
    let mut pen: int32_t = 0;
    let mut cur_line: int32_t = 0;
    let mut LR_ptr: int32_t = 0;
    LR_ptr = cur_list.eTeX_aux;
    q = (*mem.offset((best_bet + 1 as int32_t) as isize)).b32.s1;
    cur_p = TEX_NULL as int32_t;
    loop {
        r = q;
        q = (*mem.offset((q + 1 as int32_t) as isize)).b32.s0;
        (*mem.offset((r + 1 as int32_t) as isize)).b32.s0 = cur_p;
        cur_p = r;
        if !(q != TEX_NULL as int32_t) {
            break;
        }
    }
    cur_line = cur_list.prev_graf + 1 as int32_t;
    loop {
        if (*eqtb.offset((INT_BASE + INT_PAR__texxet) as isize)).b32.s1 > 0 as int32_t {
            q = (*mem.offset(TEMP_HEAD as isize)).b32.s1;
            if LR_ptr != TEX_NULL as int32_t {
                temp_ptr = LR_ptr;
                r = q;
                loop {
                    s = new_math(
                        0 as scaled_t,
                        ((*mem.offset(temp_ptr as isize)).b32.s0 - 1 as int32_t) as small_number,
                    );
                    (*mem.offset(s as isize)).b32.s1 = r;
                    r = s;
                    temp_ptr = (*mem.offset(temp_ptr as isize)).b32.s1;
                    if !(temp_ptr != TEX_NULL as int32_t) {
                        break;
                    }
                }
                (*mem.offset(TEMP_HEAD as isize)).b32.s1 = r;
            }
            while q != (*mem.offset((cur_p + 1 as int32_t) as isize)).b32.s1 {
                if q < hi_mem_min
                    && (*mem.offset(q as isize)).b16.s1 as ::core::ffi::c_int == MATH_NODE
                {
                    if (*mem.offset(q as isize)).b16.s0 as ::core::ffi::c_int
                        & 1 as ::core::ffi::c_int
                        != 0
                    {
                        if LR_ptr != TEX_NULL as int32_t
                            && (*mem.offset(LR_ptr as isize)).b32.s0
                                == L_CODE as int32_t
                                    * ((*mem.offset(q as isize)).b16.s0 as int32_t
                                        / L_CODE as int32_t)
                                    + 3 as int32_t
                        {
                            temp_ptr = LR_ptr;
                            LR_ptr = (*mem.offset(temp_ptr as isize)).b32.s1;
                            (*mem.offset(temp_ptr as isize)).b32.s1 = avail;
                            avail = temp_ptr;
                        }
                    } else {
                        temp_ptr = get_avail();
                        (*mem.offset(temp_ptr as isize)).b32.s0 = (L_CODE
                            * ((*mem.offset(q as isize)).b16.s0 as ::core::ffi::c_int / L_CODE)
                            + 3 as ::core::ffi::c_int)
                            as int32_t;
                        (*mem.offset(temp_ptr as isize)).b32.s1 = LR_ptr;
                        LR_ptr = temp_ptr;
                    }
                }
                q = (*mem.offset(q as isize)).b32.s1;
            }
        }
        q = (*mem.offset((cur_p + 1 as int32_t) as isize)).b32.s1;
        disc_break = false_0 != 0;
        post_disc_break = false_0 != 0;
        glue_break = false_0 != 0;
        if q == TEX_NULL as int32_t {
            q = TEMP_HEAD as int32_t;
            while (*mem.offset(q as isize)).b32.s1 != TEX_NULL as int32_t {
                q = (*mem.offset(q as isize)).b32.s1;
            }
        } else if (*mem.offset(q as isize)).b16.s1 as ::core::ffi::c_int == GLUE_NODE {
            delete_glue_ref((*mem.offset((q + 1 as int32_t) as isize)).b32.s0);
            (*mem.offset((q + 1 as int32_t) as isize)).b32.s0 = (*eqtb
                .offset((GLUE_BASE + GLUE_PAR__right_skip) as isize))
            .b32
            .s1;
            (*mem.offset(q as isize)).b16.s0 =
                (GLUE_PAR__right_skip + 1 as ::core::ffi::c_int) as uint16_t;
            let ref mut fresh4 = (*mem.offset(
                (*eqtb.offset((2254340 as ::core::ffi::c_int + 8 as ::core::ffi::c_int) as isize))
                    .b32
                    .s1 as isize,
            ))
            .b32
            .s1;
            *fresh4 += 1;
            glue_break = true_0 != 0;
        } else if (*mem.offset(q as isize)).b16.s1 as ::core::ffi::c_int == DISC_NODE {
            t = (*mem.offset(q as isize)).b16.s0;
            if t as ::core::ffi::c_int == 0 as ::core::ffi::c_int {
                r = (*mem.offset(q as isize)).b32.s1;
            } else {
                r = q;
                while t as ::core::ffi::c_int > 1 as ::core::ffi::c_int {
                    r = (*mem.offset(r as isize)).b32.s1;
                    t = t.wrapping_sub(1);
                }
                s = (*mem.offset(r as isize)).b32.s1;
                r = (*mem.offset(s as isize)).b32.s1;
                (*mem.offset(s as isize)).b32.s1 = TEX_NULL as int32_t;
                flush_node_list((*mem.offset(q as isize)).b32.s1);
                (*mem.offset(q as isize)).b16.s0 = 0 as uint16_t;
            }
            if (*mem.offset((q + 1 as int32_t) as isize)).b32.s1 != TEX_NULL as int32_t {
                s = (*mem.offset((q + 1 as int32_t) as isize)).b32.s1;
                while (*mem.offset(s as isize)).b32.s1 != TEX_NULL as int32_t {
                    s = (*mem.offset(s as isize)).b32.s1;
                }
                (*mem.offset(s as isize)).b32.s1 = r;
                r = (*mem.offset((q + 1 as int32_t) as isize)).b32.s1;
                (*mem.offset((q + 1 as int32_t) as isize)).b32.s1 = TEX_NULL as int32_t;
                post_disc_break = true_0 != 0;
            }
            if (*mem.offset((q + 1 as int32_t) as isize)).b32.s0 != TEX_NULL as int32_t {
                s = (*mem.offset((q + 1 as int32_t) as isize)).b32.s0;
                (*mem.offset(q as isize)).b32.s1 = s;
                while (*mem.offset(s as isize)).b32.s1 != TEX_NULL as int32_t {
                    s = (*mem.offset(s as isize)).b32.s1;
                }
                (*mem.offset((q + 1 as int32_t) as isize)).b32.s0 = TEX_NULL as int32_t;
                q = s;
            }
            (*mem.offset(q as isize)).b32.s1 = r;
            disc_break = true_0 != 0;
        } else if (*mem.offset(q as isize)).b16.s1 as ::core::ffi::c_int == KERN_NODE {
            (*mem.offset((q + 1 as int32_t) as isize)).b32.s1 = 0 as ::core::ffi::c_int as int32_t;
        } else if (*mem.offset(q as isize)).b16.s1 as ::core::ffi::c_int == MATH_NODE {
            (*mem.offset((q + 1 as int32_t) as isize)).b32.s1 = 0 as ::core::ffi::c_int as int32_t;
            if (*eqtb.offset((INT_BASE + INT_PAR__texxet) as isize)).b32.s1 > 0 as int32_t {
                if (*mem.offset(q as isize)).b16.s0 as ::core::ffi::c_int & 1 as ::core::ffi::c_int
                    != 0
                {
                    if LR_ptr != TEX_NULL as int32_t
                        && (*mem.offset(LR_ptr as isize)).b32.s0
                            == L_CODE as int32_t
                                * ((*mem.offset(q as isize)).b16.s0 as int32_t / L_CODE as int32_t)
                                + 3 as int32_t
                    {
                        temp_ptr = LR_ptr;
                        LR_ptr = (*mem.offset(temp_ptr as isize)).b32.s1;
                        (*mem.offset(temp_ptr as isize)).b32.s1 = avail;
                        avail = temp_ptr;
                    }
                } else {
                    temp_ptr = get_avail();
                    (*mem.offset(temp_ptr as isize)).b32.s0 =
                        (L_CODE * ((*mem.offset(q as isize)).b16.s0 as ::core::ffi::c_int / L_CODE)
                            + 3 as ::core::ffi::c_int) as int32_t;
                    (*mem.offset(temp_ptr as isize)).b32.s1 = LR_ptr;
                    LR_ptr = temp_ptr;
                }
            }
        }
        if (*eqtb.offset((INT_BASE + INT_PAR__xetex_protrude_chars) as isize))
            .b32
            .s1
            > 0 as int32_t
        {
            if disc_break as ::core::ffi::c_int != 0
                && (is_char_node(q) as ::core::ffi::c_int != 0
                    || (*mem.offset(q as isize)).b16.s1 as ::core::ffi::c_int != DISC_NODE)
            {
                p = q;
                ptmp = p;
            } else {
                p = prev_rightmost((*mem.offset(TEMP_HEAD as isize)).b32.s1, q);
                ptmp = p;
                p = find_protchar_right((*mem.offset(TEMP_HEAD as isize)).b32.s1, p);
            }
            w = char_pw(p, 1 as small_number);
            if w != 0 as scaled_t {
                k = new_margin_kern(-w, last_rightmost_char, 1 as small_number);
                (*mem.offset(k as isize)).b32.s1 = (*mem.offset(ptmp as isize)).b32.s1;
                (*mem.offset(ptmp as isize)).b32.s1 = k;
                if ptmp == q {
                    q = (*mem.offset(q as isize)).b32.s1;
                }
            }
        }
        if !glue_break {
            r = new_param_glue(GLUE_PAR__right_skip as small_number);
            (*mem.offset(r as isize)).b32.s1 = (*mem.offset(q as isize)).b32.s1;
            (*mem.offset(q as isize)).b32.s1 = r;
            q = r;
        }
        if (*eqtb.offset((INT_BASE + INT_PAR__texxet) as isize)).b32.s1 > 0 as int32_t {
            if LR_ptr != TEX_NULL as int32_t {
                s = TEMP_HEAD as int32_t;
                r = (*mem.offset(s as isize)).b32.s1;
                while r != q {
                    s = r;
                    r = (*mem.offset(s as isize)).b32.s1;
                }
                r = LR_ptr;
                while r != TEX_NULL as int32_t {
                    temp_ptr = new_math(
                        0 as scaled_t,
                        (*mem.offset(r as isize)).b32.s0 as small_number,
                    );
                    (*mem.offset(s as isize)).b32.s1 = temp_ptr;
                    s = temp_ptr;
                    r = (*mem.offset(r as isize)).b32.s1;
                }
                (*mem.offset(s as isize)).b32.s1 = q;
            }
        }
        r = (*mem.offset(q as isize)).b32.s1;
        (*mem.offset(q as isize)).b32.s1 = TEX_NULL as int32_t;
        q = (*mem.offset((4999999 as ::core::ffi::c_int - 3 as ::core::ffi::c_int) as isize))
            .b32
            .s1;
        (*mem.offset((4999999 as ::core::ffi::c_int - 3 as ::core::ffi::c_int) as isize))
            .b32
            .s1 = r;
        if (*eqtb.offset((INT_BASE + INT_PAR__xetex_protrude_chars) as isize))
            .b32
            .s1
            > 0 as int32_t
        {
            p = q;
            p = find_protchar_left(p, false_0 != 0);
            w = char_pw(p, 0 as small_number);
            if w != 0 as scaled_t {
                k = new_margin_kern(-w, last_leftmost_char, 0 as small_number);
                (*mem.offset(k as isize)).b32.s1 = q;
                q = k;
            }
        }
        if (*eqtb.offset((GLUE_BASE + GLUE_PAR__left_skip) as isize))
            .b32
            .s1
            != 0 as int32_t
        {
            r = new_param_glue(GLUE_PAR__left_skip as small_number);
            (*mem.offset(r as isize)).b32.s1 = q;
            q = r;
        }
        if cur_line > last_special_line {
            cur_width = second_width;
            cur_indent = second_indent;
        } else if (*eqtb.offset((LOCAL_BASE + LOCAL__par_shape) as isize))
            .b32
            .s1
            == TEX_NULL as int32_t
        {
            cur_width = first_width;
            cur_indent = first_indent;
        } else {
            cur_width = (*mem.offset(
                ((*eqtb.offset((LOCAL_BASE + LOCAL__par_shape) as isize))
                    .b32
                    .s1
                    + 2 as int32_t * cur_line) as isize,
            ))
            .b32
            .s1 as scaled_t;
            cur_indent = (*mem.offset(
                ((*eqtb.offset((LOCAL_BASE + LOCAL__par_shape) as isize))
                    .b32
                    .s1
                    + 2 as int32_t * cur_line
                    - 1 as int32_t) as isize,
            ))
            .b32
            .s1 as scaled_t;
        }
        if !semantic_pagination_enabled {
            q = pitex_expand_line(q, cur_width);
        }
        crate::engine_fonts::PARAGRAPH_PACKING = true;
        adjust_tail = ADJUST_HEAD as int32_t;
        pre_adjust_tail = PRE_ADJUST_HEAD as int32_t;
        if semantic_pagination_enabled {
            just_box = hpack(q, 0 as scaled_t, ADDITIONAL as small_number);
        } else {
            just_box = hpack(q, cur_width, EXACTLY as small_number);
        }
        crate::engine_fonts::PARAGRAPH_PACKING = false;
        let ignored = crate::engine_fonts::parameter(b"pdfignoreddimen\0");
        let first = crate::engine_fonts::parameter(b"pdffirstlineheight\0");
        let last = crate::engine_fonts::parameter(b"pdflastlinedepth\0");
        let height = if cur_line == cur_list.prev_graf + 1 && first != ignored { first } else { crate::engine_fonts::parameter(b"pdfeachlineheight\0") };
        let depth = if cur_line + 1 == best_line && last != ignored { last } else { crate::engine_fonts::parameter(b"pdfeachlinedepth\0") };
        if height != ignored { (*mem.offset((just_box+3) as isize)).b32.s1 = height; }
        if depth != ignored { (*mem.offset((just_box+2) as isize)).b32.s1 = depth; }
        if cur_list.aux.b32.s1 == ignored { cur_list.aux.b32.s1 = crate::xetex_engine_xetex_ini::IGNORE_DEPTH; }
        (*mem.offset((just_box + 4 as int32_t) as isize)).b32.s1 = cur_indent as int32_t;
        if PRE_ADJUST_HEAD as int32_t != pre_adjust_tail {
            (*mem.offset(cur_list.tail as isize)).b32.s1 = (*mem
                .offset((4999999 as ::core::ffi::c_int - 14 as ::core::ffi::c_int) as isize))
            .b32
            .s1;
            cur_list.tail = pre_adjust_tail;
        }
        pre_adjust_tail = TEX_NULL as int32_t;
        append_to_vlist(just_box);
        if ADJUST_HEAD as int32_t != adjust_tail {
            (*mem.offset(cur_list.tail as isize)).b32.s1 = (*mem
                .offset((4999999 as ::core::ffi::c_int - 5 as ::core::ffi::c_int) as isize))
            .b32
            .s1;
            cur_list.tail = adjust_tail;
        }
        adjust_tail = TEX_NULL as int32_t;
        if cur_line + 1 as int32_t != best_line {
            q = (*eqtb.offset((ETEX_PEN_BASE + ETEX_PENALTIES_PAR__inter_line_penalties) as isize))
                .b32
                .s1;
            if q != TEX_NULL as int32_t {
                r = cur_line;
                if r > (*mem.offset((q + 1 as int32_t) as isize)).b32.s1 {
                    r = (*mem.offset((q + 1 as int32_t) as isize)).b32.s1;
                }
                pen = (*mem.offset((q + r + 1 as int32_t) as isize)).b32.s1;
            } else {
                pen = (*eqtb.offset((INT_BASE + INT_PAR__inter_line_penalty) as isize))
                    .b32
                    .s1;
            }
            q = (*eqtb.offset((ETEX_PEN_BASE + ETEX_PENALTIES_PAR__club_penalties) as isize))
                .b32
                .s1;
            if q != TEX_NULL as int32_t {
                r = cur_line - cur_list.prev_graf;
                if r > (*mem.offset((q + 1 as int32_t) as isize)).b32.s1 {
                    r = (*mem.offset((q + 1 as int32_t) as isize)).b32.s1;
                }
                pen += (*mem.offset((q + r + 1 as int32_t) as isize)).b32.s1;
            } else if cur_line == cur_list.prev_graf + 1 as int32_t {
                pen += (*eqtb.offset((INT_BASE + INT_PAR__club_penalty) as isize))
                    .b32
                    .s1;
            }
            if d {
                q = (*eqtb.offset(
                    (ETEX_PEN_BASE + ETEX_PENALTIES_PAR__display_widow_penalties) as isize,
                ))
                .b32
                .s1;
            } else {
                q = (*eqtb.offset((ETEX_PEN_BASE + ETEX_PENALTIES_PAR__widow_penalties) as isize))
                    .b32
                    .s1;
            }
            if q != TEX_NULL as int32_t {
                r = best_line - cur_line - 1 as int32_t;
                if r > (*mem.offset((q + 1 as int32_t) as isize)).b32.s1 {
                    r = (*mem.offset((q + 1 as int32_t) as isize)).b32.s1;
                }
                pen += (*mem.offset((q + r + 1 as int32_t) as isize)).b32.s1;
            } else if cur_line + 2 as int32_t == best_line {
                if d {
                    pen += (*eqtb.offset((INT_BASE + INT_PAR__display_widow_penalty) as isize))
                        .b32
                        .s1;
                } else {
                    pen += (*eqtb.offset((INT_BASE + INT_PAR__widow_penalty) as isize))
                        .b32
                        .s1;
                }
            }
            if disc_break {
                pen += (*eqtb.offset((INT_BASE + INT_PAR__broken_penalty) as isize))
                    .b32
                    .s1;
            }
            if pen != 0 as int32_t {
                r = new_penalty(pen);
                (*mem.offset(cur_list.tail as isize)).b32.s1 = r;
                cur_list.tail = r;
            }
        }
        cur_line += 1;
        cur_p = (*mem.offset((cur_p + 1 as int32_t) as isize)).b32.s0;
        if cur_p != TEX_NULL as int32_t {
            if !post_disc_break {
                r = TEMP_HEAD as int32_t;
                loop {
                    q = (*mem.offset(r as isize)).b32.s1;
                    if q == (*mem.offset((cur_p + 1 as int32_t) as isize)).b32.s1 {
                        break;
                    }
                    if is_char_node(q) {
                        break;
                    }
                    if is_non_discardable_node(q) {
                        break;
                    }
                    if (*mem.offset(q as isize)).b16.s1 as ::core::ffi::c_int == KERN_NODE
                        && (*mem.offset(q as isize)).b16.s0 as ::core::ffi::c_int != EXPLICIT
                        && (*mem.offset(q as isize)).b16.s0 as ::core::ffi::c_int
                            != SPACE_ADJUSTMENT
                    {
                        break;
                    }
                    r = q;
                    if (*mem.offset(q as isize)).b16.s1 as ::core::ffi::c_int == MATH_NODE
                        && (*eqtb.offset((INT_BASE + INT_PAR__texxet) as isize)).b32.s1
                            > 0 as int32_t
                    {
                        if (*mem.offset(q as isize)).b16.s0 as ::core::ffi::c_int
                            & 1 as ::core::ffi::c_int
                            != 0
                        {
                            if LR_ptr != TEX_NULL as int32_t
                                && (*mem.offset(LR_ptr as isize)).b32.s0
                                    == L_CODE as int32_t
                                        * ((*mem.offset(q as isize)).b16.s0 as int32_t
                                            / L_CODE as int32_t)
                                        + 3 as int32_t
                            {
                                temp_ptr = LR_ptr;
                                LR_ptr = (*mem.offset(temp_ptr as isize)).b32.s1;
                                (*mem.offset(temp_ptr as isize)).b32.s1 = avail;
                                avail = temp_ptr;
                            }
                        } else {
                            temp_ptr = get_avail();
                            (*mem.offset(temp_ptr as isize)).b32.s0 = (L_CODE
                                * ((*mem.offset(q as isize)).b16.s0 as ::core::ffi::c_int / L_CODE)
                                + 3 as ::core::ffi::c_int)
                                as int32_t;
                            (*mem.offset(temp_ptr as isize)).b32.s1 = LR_ptr;
                            LR_ptr = temp_ptr;
                        }
                    }
                }
                if r != TEMP_HEAD as int32_t {
                    (*mem.offset(r as isize)).b32.s1 = TEX_NULL as int32_t;
                    flush_node_list(
                        (*mem.offset(
                            (4999999 as ::core::ffi::c_int - 3 as ::core::ffi::c_int) as isize,
                        ))
                        .b32
                        .s1,
                    );
                    (*mem.offset(
                        (4999999 as ::core::ffi::c_int - 3 as ::core::ffi::c_int) as isize,
                    ))
                    .b32
                    .s1 = q;
                }
            }
        }
        if !(cur_p != TEX_NULL as int32_t) {
            break;
        }
    }
    if cur_line != best_line
        || (*mem.offset((4999999 as ::core::ffi::c_int - 3 as ::core::ffi::c_int) as isize))
            .b32
            .s1
            != TEX_NULL as int32_t
    {
        confusion(b"line breaking\0" as *const u8 as *const ::core::ffi::c_char);
    }
    cur_list.prev_graf = best_line - 1 as int32_t;
    cur_list.eTeX_aux = LR_ptr;
}
unsafe extern "C" fn try_break(mut pi: int32_t, mut break_type: small_number) {
    let mut current_block: u64;
    let mut r: int32_t = 0;
    let mut prev_r: int32_t = 0;
    let mut old_l: int32_t = 0;
    let mut no_break_yet: bool = false;
    let mut prev_prev_r: int32_t = TEX_NULL as int32_t;
    let mut s: int32_t = 0;
    let mut q: int32_t = 0;
    let mut v: int32_t = 0;
    let mut t: int32_t = 0;
    let mut f: internal_font_number = 0;
    let mut l: int32_t = 0;
    let mut node_r_stays_active: bool = false;
    let mut line_width: scaled_t = 0 as scaled_t;
    let mut fit_class: ::core::ffi::c_uchar = 0;
    let mut b: int32_t = 0;
    let mut d: int32_t = 0;
    let mut artificial_demerits: bool = false;
    let mut shortfall: scaled_t = 0;
    let mut g: scaled_t = 0 as scaled_t;
    if semantic_pagination_enabled as ::core::ffi::c_int != 0 && cur_p != TEX_NULL as int32_t {
        return;
    }
    if abs(pi as ::core::ffi::c_int) >= INF_PENALTY {
        if pi > 0 as int32_t {
            return;
        }
        pi = EJECT_PENALTY as int32_t;
    }
    no_break_yet = true_0 != 0;
    prev_r = ACTIVE_LIST as int32_t;
    old_l = 0 as ::core::ffi::c_int as int32_t;
    cur_active_width[1 as ::core::ffi::c_int as usize] =
        active_width[1 as ::core::ffi::c_int as usize];
    cur_active_width[2 as ::core::ffi::c_int as usize] =
        active_width[2 as ::core::ffi::c_int as usize];
    cur_active_width[3 as ::core::ffi::c_int as usize] =
        active_width[3 as ::core::ffi::c_int as usize];
    cur_active_width[4 as ::core::ffi::c_int as usize] =
        active_width[4 as ::core::ffi::c_int as usize];
    cur_active_width[5 as ::core::ffi::c_int as usize] =
        active_width[5 as ::core::ffi::c_int as usize];
    cur_active_width[6 as ::core::ffi::c_int as usize] =
        active_width[6 as ::core::ffi::c_int as usize];
    loop {
        r = (*mem.offset(prev_r as isize)).b32.s1;
        if (*mem.offset(r as isize)).b16.s1 as ::core::ffi::c_int == DELTA_NODE {
            cur_active_width[1 as ::core::ffi::c_int as usize] +=
                (*mem.offset((r + 1 as int32_t) as isize)).b32.s1;
            cur_active_width[2 as ::core::ffi::c_int as usize] +=
                (*mem.offset((r + 2 as int32_t) as isize)).b32.s1;
            cur_active_width[3 as ::core::ffi::c_int as usize] +=
                (*mem.offset((r + 3 as int32_t) as isize)).b32.s1;
            cur_active_width[4 as ::core::ffi::c_int as usize] +=
                (*mem.offset((r + 4 as int32_t) as isize)).b32.s1;
            cur_active_width[5 as ::core::ffi::c_int as usize] +=
                (*mem.offset((r + 5 as int32_t) as isize)).b32.s1;
            cur_active_width[6 as ::core::ffi::c_int as usize] +=
                (*mem.offset((r + 6 as int32_t) as isize)).b32.s1;
            prev_prev_r = prev_r;
            prev_r = r;
        } else {
            l = (*mem.offset((r + 1 as int32_t) as isize)).b32.s0;
            if l > old_l {
                if minimum_demerits < AWFUL_BAD as int32_t
                    && (old_l != easy_line || r == LAST_ACTIVE as int32_t)
                {
                    if no_break_yet {
                        no_break_yet = false_0 != 0;
                        break_width[1 as ::core::ffi::c_int as usize] =
                            background[1 as ::core::ffi::c_int as usize];
                        break_width[2 as ::core::ffi::c_int as usize] =
                            background[2 as ::core::ffi::c_int as usize];
                        break_width[3 as ::core::ffi::c_int as usize] =
                            background[3 as ::core::ffi::c_int as usize];
                        break_width[4 as ::core::ffi::c_int as usize] =
                            background[4 as ::core::ffi::c_int as usize];
                        break_width[5 as ::core::ffi::c_int as usize] =
                            background[5 as ::core::ffi::c_int as usize];
                        break_width[6 as ::core::ffi::c_int as usize] =
                            background[6 as ::core::ffi::c_int as usize];
                        s = cur_p;
                        if break_type as ::core::ffi::c_int > UNHYPHENATED {
                            if cur_p != TEX_NULL as int32_t {
                                t = (*mem.offset(cur_p as isize)).b16.s0 as int32_t;
                                v = cur_p;
                                s = (*mem.offset((cur_p + 1 as int32_t) as isize)).b32.s1;
                                while t > 0 as int32_t {
                                    t -= 1;
                                    v = (*mem.offset(v as isize)).b32.s1;
                                    if is_char_node(v) {
                                        let mut eff_char: int32_t = 0;
                                        f = (*mem.offset(v as isize)).b16.s1
                                            as internal_font_number;
                                        eff_char = effective_char(
                                            true_0 != 0,
                                            f,
                                            (*mem.offset(v as isize)).b16.s0,
                                        );
                                        break_width[1 as ::core::ffi::c_int as usize] -=
                                            (*font_info.offset(
                                                (*width_base.offset(f as isize)
                                                    + (*font_info.offset(
                                                        (*char_base.offset(f as isize) + eff_char)
                                                            as isize,
                                                    ))
                                                    .b16
                                                    .s3
                                                        as int32_t)
                                                    as isize,
                                            ))
                                            .b32
                                            .s1;
                                    } else {
                                        match (*mem.offset(v as isize)).b16.s1 as ::core::ffi::c_int
                                        {
                                            LIGATURE_NODE => {
                                                let mut eff_char_0: int32_t = 0;
                                                f = (*mem.offset((v + 1 as int32_t) as isize))
                                                    .b16
                                                    .s1
                                                    as internal_font_number;
                                                xtx_ligature_present = true_0 != 0;
                                                eff_char_0 = effective_char(
                                                    true_0 != 0,
                                                    f,
                                                    (*mem.offset((v + 1 as int32_t) as isize))
                                                        .b16
                                                        .s0,
                                                );
                                                break_width[1 as ::core::ffi::c_int as usize] -=
                                                    (*font_info.offset(
                                                        (*width_base.offset(f as isize)
                                                            + (*font_info.offset(
                                                                (*char_base.offset(f as isize)
                                                                    + eff_char_0)
                                                                    as isize,
                                                            ))
                                                            .b16
                                                            .s3
                                                                as int32_t)
                                                            as isize,
                                                    ))
                                                    .b32
                                                    .s1;
                                            }
                                            HLIST_NODE | VLIST_NODE | RULE_NODE | KERN_NODE => {
                                                break_width[1 as ::core::ffi::c_int as usize] -=
                                                    (*mem.offset((v + 1 as int32_t) as isize))
                                                        .b32
                                                        .s1;
                                            }
                                            WHATSIT_NODE => {
                                                if (*mem.offset(v as isize)).b16.s0
                                                    as ::core::ffi::c_int
                                                    == NATIVE_WORD_NODE
                                                    || (*mem.offset(v as isize)).b16.s0
                                                        as ::core::ffi::c_int
                                                        == NATIVE_WORD_NODE_AT
                                                    || (*mem.offset(v as isize)).b16.s0
                                                        as ::core::ffi::c_int
                                                        == GLYPH_NODE
                                                    || (*mem.offset(v as isize)).b16.s0
                                                        as ::core::ffi::c_int
                                                        == PIC_NODE
                                                    || (*mem.offset(v as isize)).b16.s0
                                                        as ::core::ffi::c_int
                                                        == PDF_NODE
                                                {
                                                    break_width
                                                        [1 as ::core::ffi::c_int as usize] -=
                                                        (*mem.offset((v + 1 as int32_t) as isize))
                                                            .b32
                                                            .s1;
                                                } else {
                                                    confusion(
                                                        b"disc1a\0" as *const u8
                                                            as *const ::core::ffi::c_char,
                                                    );
                                                }
                                            }
                                            _ => {
                                                confusion(
                                                    b"disc1\0" as *const u8
                                                        as *const ::core::ffi::c_char,
                                                );
                                            }
                                        }
                                    }
                                }
                                while s != TEX_NULL as int32_t {
                                    if is_char_node(s) {
                                        let mut eff_char_1: int32_t = 0;
                                        f = (*mem.offset(s as isize)).b16.s1
                                            as internal_font_number;
                                        eff_char_1 = effective_char(
                                            true_0 != 0,
                                            f,
                                            (*mem.offset(s as isize)).b16.s0,
                                        );
                                        break_width[1 as ::core::ffi::c_int as usize] +=
                                            (*font_info.offset(
                                                (*width_base.offset(f as isize)
                                                    + (*font_info.offset(
                                                        (*char_base.offset(f as isize) + eff_char_1)
                                                            as isize,
                                                    ))
                                                    .b16
                                                    .s3
                                                        as int32_t)
                                                    as isize,
                                            ))
                                            .b32
                                            .s1;
                                    } else {
                                        match (*mem.offset(s as isize)).b16.s1 as ::core::ffi::c_int
                                        {
                                            LIGATURE_NODE => {
                                                let mut eff_char_2: int32_t = 0;
                                                f = (*mem.offset((s + 1 as int32_t) as isize))
                                                    .b16
                                                    .s1
                                                    as internal_font_number;
                                                xtx_ligature_present = true_0 != 0;
                                                eff_char_2 = effective_char(
                                                    true_0 != 0,
                                                    f,
                                                    (*mem.offset((s + 1 as int32_t) as isize))
                                                        .b16
                                                        .s0,
                                                );
                                                break_width[1 as ::core::ffi::c_int as usize] +=
                                                    (*font_info.offset(
                                                        (*width_base.offset(f as isize)
                                                            + (*font_info.offset(
                                                                (*char_base.offset(f as isize)
                                                                    + eff_char_2)
                                                                    as isize,
                                                            ))
                                                            .b16
                                                            .s3
                                                                as int32_t)
                                                            as isize,
                                                    ))
                                                    .b32
                                                    .s1;
                                            }
                                            HLIST_NODE | VLIST_NODE | RULE_NODE | KERN_NODE => {
                                                break_width[1 as ::core::ffi::c_int as usize] +=
                                                    (*mem.offset((s + 1 as int32_t) as isize))
                                                        .b32
                                                        .s1;
                                            }
                                            WHATSIT_NODE => {
                                                if (*mem.offset(s as isize)).b16.s0
                                                    as ::core::ffi::c_int
                                                    == NATIVE_WORD_NODE
                                                    || (*mem.offset(s as isize)).b16.s0
                                                        as ::core::ffi::c_int
                                                        == NATIVE_WORD_NODE_AT
                                                    || (*mem.offset(s as isize)).b16.s0
                                                        as ::core::ffi::c_int
                                                        == GLYPH_NODE
                                                    || (*mem.offset(s as isize)).b16.s0
                                                        as ::core::ffi::c_int
                                                        == PIC_NODE
                                                    || (*mem.offset(s as isize)).b16.s0
                                                        as ::core::ffi::c_int
                                                        == PDF_NODE
                                                {
                                                    break_width
                                                        [1 as ::core::ffi::c_int as usize] +=
                                                        (*mem.offset((s + 1 as int32_t) as isize))
                                                            .b32
                                                            .s1;
                                                } else {
                                                    confusion(
                                                        b"disc2a\0" as *const u8
                                                            as *const ::core::ffi::c_char,
                                                    );
                                                }
                                            }
                                            _ => {
                                                confusion(
                                                    b"disc2\0" as *const u8
                                                        as *const ::core::ffi::c_char,
                                                );
                                            }
                                        }
                                    }
                                    s = (*mem.offset(s as isize)).b32.s1;
                                }
                                break_width[1 as ::core::ffi::c_int as usize] += disc_width;
                                if (*mem.offset((cur_p + 1 as int32_t) as isize)).b32.s1
                                    == TEX_NULL as int32_t
                                {
                                    s = (*mem.offset(v as isize)).b32.s1;
                                }
                            }
                        }
                        while s != TEX_NULL as int32_t {
                            if is_char_node(s) {
                                break;
                            }
                            match (*mem.offset(s as isize)).b16.s1 as ::core::ffi::c_int {
                                GLUE_NODE => {
                                    v = (*mem.offset((s + 1 as int32_t) as isize)).b32.s0;
                                    break_width[1 as ::core::ffi::c_int as usize] -=
                                        (*mem.offset((v + 1 as int32_t) as isize)).b32.s1;
                                    break_width[(2 as ::core::ffi::c_int
                                        + (*mem.offset(v as isize)).b16.s1 as ::core::ffi::c_int)
                                        as usize] -=
                                        (*mem.offset((v + 2 as int32_t) as isize)).b32.s1;
                                    break_width[6 as ::core::ffi::c_int as usize] -=
                                        (*mem.offset((v + 3 as int32_t) as isize)).b32.s1;
                                }
                                PENALTY_NODE => {}
                                MATH_NODE => {
                                    break_width[1 as ::core::ffi::c_int as usize] -=
                                        (*mem.offset((s + 1 as int32_t) as isize)).b32.s1;
                                }
                                KERN_NODE => {
                                    if (*mem.offset(s as isize)).b16.s0 as ::core::ffi::c_int
                                        != EXPLICIT
                                    {
                                        break;
                                    }
                                    break_width[1 as ::core::ffi::c_int as usize] -=
                                        (*mem.offset((s + 1 as int32_t) as isize)).b32.s1;
                                }
                                _ => {
                                    break;
                                }
                            }
                            s = (*mem.offset(s as isize)).b32.s1;
                        }
                    }
                    if (*mem.offset(prev_r as isize)).b16.s1 as ::core::ffi::c_int == DELTA_NODE {
                        (*mem.offset((prev_r + 1 as int32_t) as isize)).b32.s1 += (-cur_active_width
                            [1 as ::core::ffi::c_int as usize]
                            + break_width[1 as ::core::ffi::c_int as usize])
                            as int32_t;
                        (*mem.offset((prev_r + 2 as int32_t) as isize)).b32.s1 += (-cur_active_width
                            [2 as ::core::ffi::c_int as usize]
                            + break_width[2 as ::core::ffi::c_int as usize])
                            as int32_t;
                        (*mem.offset((prev_r + 3 as int32_t) as isize)).b32.s1 += (-cur_active_width
                            [3 as ::core::ffi::c_int as usize]
                            + break_width[3 as ::core::ffi::c_int as usize])
                            as int32_t;
                        (*mem.offset((prev_r + 4 as int32_t) as isize)).b32.s1 += (-cur_active_width
                            [4 as ::core::ffi::c_int as usize]
                            + break_width[4 as ::core::ffi::c_int as usize])
                            as int32_t;
                        (*mem.offset((prev_r + 5 as int32_t) as isize)).b32.s1 += (-cur_active_width
                            [5 as ::core::ffi::c_int as usize]
                            + break_width[5 as ::core::ffi::c_int as usize])
                            as int32_t;
                        (*mem.offset((prev_r + 6 as int32_t) as isize)).b32.s1 += (-cur_active_width
                            [6 as ::core::ffi::c_int as usize]
                            + break_width[6 as ::core::ffi::c_int as usize])
                            as int32_t;
                    } else if prev_r == ACTIVE_LIST as int32_t {
                        active_width[1 as ::core::ffi::c_int as usize] =
                            break_width[1 as ::core::ffi::c_int as usize];
                        active_width[2 as ::core::ffi::c_int as usize] =
                            break_width[2 as ::core::ffi::c_int as usize];
                        active_width[3 as ::core::ffi::c_int as usize] =
                            break_width[3 as ::core::ffi::c_int as usize];
                        active_width[4 as ::core::ffi::c_int as usize] =
                            break_width[4 as ::core::ffi::c_int as usize];
                        active_width[5 as ::core::ffi::c_int as usize] =
                            break_width[5 as ::core::ffi::c_int as usize];
                        active_width[6 as ::core::ffi::c_int as usize] =
                            break_width[6 as ::core::ffi::c_int as usize];
                    } else {
                        q = get_node(DELTA_NODE_SIZE as int32_t);
                        (*mem.offset(q as isize)).b32.s1 = r;
                        (*mem.offset(q as isize)).b16.s1 = DELTA_NODE as uint16_t;
                        (*mem.offset(q as isize)).b16.s0 = 0 as uint16_t;
                        (*mem.offset((q + 1 as int32_t) as isize)).b32.s1 = (break_width
                            [1 as ::core::ffi::c_int as usize]
                            - cur_active_width[1 as ::core::ffi::c_int as usize])
                            as int32_t;
                        (*mem.offset((q + 2 as int32_t) as isize)).b32.s1 = (break_width
                            [2 as ::core::ffi::c_int as usize]
                            - cur_active_width[2 as ::core::ffi::c_int as usize])
                            as int32_t;
                        (*mem.offset((q + 3 as int32_t) as isize)).b32.s1 = (break_width
                            [3 as ::core::ffi::c_int as usize]
                            - cur_active_width[3 as ::core::ffi::c_int as usize])
                            as int32_t;
                        (*mem.offset((q + 4 as int32_t) as isize)).b32.s1 = (break_width
                            [4 as ::core::ffi::c_int as usize]
                            - cur_active_width[4 as ::core::ffi::c_int as usize])
                            as int32_t;
                        (*mem.offset((q + 5 as int32_t) as isize)).b32.s1 = (break_width
                            [5 as ::core::ffi::c_int as usize]
                            - cur_active_width[5 as ::core::ffi::c_int as usize])
                            as int32_t;
                        (*mem.offset((q + 6 as int32_t) as isize)).b32.s1 = (break_width
                            [6 as ::core::ffi::c_int as usize]
                            - cur_active_width[6 as ::core::ffi::c_int as usize])
                            as int32_t;
                        (*mem.offset(prev_r as isize)).b32.s1 = q;
                        prev_prev_r = prev_r;
                        prev_r = q;
                    }
                    if abs((*eqtb.offset((INT_BASE + INT_PAR__adj_demerits) as isize))
                        .b32
                        .s1 as ::core::ffi::c_int) as int32_t
                        >= MAX_HALFWORD as int32_t - minimum_demerits
                    {
                        minimum_demerits = (AWFUL_BAD - 1 as ::core::ffi::c_int) as int32_t;
                    } else {
                        minimum_demerits = minimum_demerits
                            + abs((*eqtb.offset((INT_BASE + INT_PAR__adj_demerits) as isize))
                                .b32
                                .s1 as ::core::ffi::c_int) as int32_t;
                    }
                    fit_class = VERY_LOOSE_FIT as ::core::ffi::c_uchar;
                    while fit_class as ::core::ffi::c_int <= TIGHT_FIT {
                        if minimal_demerits[fit_class as usize] <= minimum_demerits {
                            q = get_node(PASSIVE_NODE_SIZE as int32_t);
                            (*mem.offset(q as isize)).b32.s1 = passive;
                            passive = q;
                            (*mem.offset((q + 1 as int32_t) as isize)).b32.s1 = cur_p;
                            (*mem.offset((q + 1 as int32_t) as isize)).b32.s0 =
                                best_place[fit_class as usize];
                            q = get_node(active_node_size as int32_t);
                            (*mem.offset((q + 1 as int32_t) as isize)).b32.s1 = passive;
                            (*mem.offset((q + 1 as int32_t) as isize)).b32.s0 =
                                best_pl_line[fit_class as usize] + 1 as int32_t;
                            (*mem.offset(q as isize)).b16.s0 = fit_class as uint16_t;
                            (*mem.offset(q as isize)).b16.s1 = break_type as uint16_t;
                            (*mem.offset((q + 2 as int32_t) as isize)).b32.s1 =
                                minimal_demerits[fit_class as usize];
                            if do_last_line_fit {
                                (*mem.offset((q + 3 as int32_t) as isize)).b32.s1 =
                                    best_pl_short[fit_class as usize] as int32_t;
                                (*mem.offset((q + 4 as int32_t) as isize)).b32.s1 =
                                    best_pl_glue[fit_class as usize] as int32_t;
                            }
                            (*mem.offset(q as isize)).b32.s1 = r;
                            (*mem.offset(prev_r as isize)).b32.s1 = q;
                            prev_r = q;
                        }
                        minimal_demerits[fit_class as usize] = MAX_HALFWORD as int32_t;
                        fit_class = fit_class.wrapping_add(1);
                    }
                    minimum_demerits = MAX_HALFWORD as int32_t;
                    if r != LAST_ACTIVE as int32_t {
                        q = get_node(DELTA_NODE_SIZE as int32_t);
                        (*mem.offset(q as isize)).b32.s1 = r;
                        (*mem.offset(q as isize)).b16.s1 = DELTA_NODE as uint16_t;
                        (*mem.offset(q as isize)).b16.s0 = 0 as uint16_t;
                        (*mem.offset((q + 1 as int32_t) as isize)).b32.s1 = (cur_active_width
                            [1 as ::core::ffi::c_int as usize]
                            - break_width[1 as ::core::ffi::c_int as usize])
                            as int32_t;
                        (*mem.offset((q + 2 as int32_t) as isize)).b32.s1 = (cur_active_width
                            [2 as ::core::ffi::c_int as usize]
                            - break_width[2 as ::core::ffi::c_int as usize])
                            as int32_t;
                        (*mem.offset((q + 3 as int32_t) as isize)).b32.s1 = (cur_active_width
                            [3 as ::core::ffi::c_int as usize]
                            - break_width[3 as ::core::ffi::c_int as usize])
                            as int32_t;
                        (*mem.offset((q + 4 as int32_t) as isize)).b32.s1 = (cur_active_width
                            [4 as ::core::ffi::c_int as usize]
                            - break_width[4 as ::core::ffi::c_int as usize])
                            as int32_t;
                        (*mem.offset((q + 5 as int32_t) as isize)).b32.s1 = (cur_active_width
                            [5 as ::core::ffi::c_int as usize]
                            - break_width[5 as ::core::ffi::c_int as usize])
                            as int32_t;
                        (*mem.offset((q + 6 as int32_t) as isize)).b32.s1 = (cur_active_width
                            [6 as ::core::ffi::c_int as usize]
                            - break_width[6 as ::core::ffi::c_int as usize])
                            as int32_t;
                        (*mem.offset(prev_r as isize)).b32.s1 = q;
                        prev_prev_r = prev_r;
                        prev_r = q;
                    }
                }
                if r == LAST_ACTIVE as int32_t {
                    return;
                }
                if l > easy_line {
                    line_width = second_width;
                    old_l = (MAX_HALFWORD - 1 as ::core::ffi::c_int) as int32_t;
                } else {
                    old_l = l;
                    if l > last_special_line {
                        line_width = second_width;
                    } else if (*eqtb.offset((LOCAL_BASE + LOCAL__par_shape) as isize))
                        .b32
                        .s1
                        == TEX_NULL as int32_t
                    {
                        line_width = first_width;
                    } else {
                        line_width = (*mem.offset(
                            ((*eqtb.offset((LOCAL_BASE + LOCAL__par_shape) as isize))
                                .b32
                                .s1
                                + 2 as int32_t * l) as isize,
                        ))
                        .b32
                        .s1 as scaled_t;
                    }
                }
            }
            if semantic_pagination_enabled {
                line_width = cur_active_width[1 as ::core::ffi::c_int as usize];
                artificial_demerits = true_0 != 0;
                shortfall = 0 as ::core::ffi::c_int as scaled_t;
            } else {
                artificial_demerits = false_0 != 0;
                shortfall = line_width - cur_active_width[1 as ::core::ffi::c_int as usize];
                if (*eqtb.offset((INT_BASE + INT_PAR__xetex_protrude_chars) as isize))
                    .b32
                    .s1
                    > 1 as int32_t
                {
                    shortfall = shortfall + total_pw(r, cur_p);
                }
            }
            let pitex_candidate_stretch = pitex_candidate_expansion_capacity(r,cur_p,true);
            let pitex_candidate_shrink = pitex_candidate_expansion_capacity(r,cur_p,false);
            if shortfall > 0 as scaled_t {
                if cur_active_width[3 as ::core::ffi::c_int as usize] != 0 as scaled_t
                    || cur_active_width[4 as ::core::ffi::c_int as usize] != 0 as scaled_t
                    || cur_active_width[5 as ::core::ffi::c_int as usize] != 0 as scaled_t
                {
                    if do_last_line_fit {
                        if cur_p == TEX_NULL as int32_t {
                            if (*mem.offset((r + 3 as int32_t) as isize)).b32.s1 == 0 as int32_t
                                || (*mem.offset((r + 4 as int32_t) as isize)).b32.s1 <= 0 as int32_t
                            {
                                current_block = 12153365054289215322;
                            } else if cur_active_width[3 as ::core::ffi::c_int as usize]
                                != fill_width[0 as ::core::ffi::c_int as usize]
                                || cur_active_width[4 as ::core::ffi::c_int as usize]
                                    != fill_width[1 as ::core::ffi::c_int as usize]
                                || cur_active_width[5 as ::core::ffi::c_int as usize]
                                    != fill_width[2 as ::core::ffi::c_int as usize]
                            {
                                current_block = 12153365054289215322;
                            } else {
                                if (*mem.offset((r + 3 as int32_t) as isize)).b32.s1 > 0 as int32_t
                                {
                                    g = (cur_active_width[2 as ::core::ffi::c_int as usize] + pitex_candidate_stretch);
                                } else {
                                    g = (cur_active_width[6 as ::core::ffi::c_int as usize] + pitex_candidate_shrink);
                                }
                                if g <= 0 as scaled_t {
                                    current_block = 12153365054289215322;
                                } else {
                                    arith_error = false_0 != 0;
                                    g = fract(
                                        g as int32_t,
                                        (*mem.offset((r + 3 as int32_t) as isize)).b32.s1,
                                        (*mem.offset((r + 4 as int32_t) as isize)).b32.s1,
                                        MAX_HALFWORD as int32_t,
                                    ) as scaled_t;
                                    if (*eqtb.offset((INT_BASE + INT_PAR__last_line_fit) as isize))
                                        .b32
                                        .s1
                                        < 1000 as int32_t
                                    {
                                        g = fract(
                                            g as int32_t,
                                            (*eqtb.offset(
                                                (INT_BASE + INT_PAR__last_line_fit) as isize,
                                            ))
                                            .b32
                                            .s1,
                                            1000 as int32_t,
                                            MAX_HALFWORD as int32_t,
                                        ) as scaled_t;
                                    }
                                    if arith_error {
                                        if (*mem.offset((r + 3 as int32_t) as isize)).b32.s1
                                            > 0 as int32_t
                                        {
                                            g = MAX_HALFWORD as scaled_t;
                                        } else {
                                            g = -MAX_HALFWORD as scaled_t;
                                        }
                                    }
                                    if g > 0 as scaled_t {
                                        if g > shortfall {
                                            g = shortfall;
                                        }
                                        if g as ::core::ffi::c_long > 7230584 as ::core::ffi::c_long
                                        {
                                            if ((cur_active_width[2 as ::core::ffi::c_int as usize] + pitex_candidate_stretch)
                                                as ::core::ffi::c_long)
                                                < 1663497 as ::core::ffi::c_long
                                            {
                                                b = INF_BAD as int32_t;
                                                fit_class = VERY_LOOSE_FIT as ::core::ffi::c_uchar;
                                                current_block = 7213206328819867990;
                                            } else {
                                                current_block = 1176253869785344635;
                                            }
                                        } else {
                                            current_block = 1176253869785344635;
                                        }
                                        match current_block {
                                            7213206328819867990 => {}
                                            _ => {
                                                b = badness(
                                                    g,
                                                    cur_active_width
                                                        [2 as ::core::ffi::c_int as usize],
                                                );
                                                if b > 12 as int32_t {
                                                    if b > 99 as int32_t {
                                                        fit_class =
                                                            VERY_LOOSE_FIT as ::core::ffi::c_uchar;
                                                    } else {
                                                        fit_class =
                                                            LOOSE_FIT as ::core::ffi::c_uchar;
                                                    }
                                                } else {
                                                    fit_class = DECENT_FIT as ::core::ffi::c_uchar;
                                                }
                                                current_block = 7213206328819867990;
                                            }
                                        }
                                    } else if g < 0 as scaled_t {
                                        if -g > (cur_active_width[6 as ::core::ffi::c_int as usize] + pitex_candidate_shrink) {
                                            g = -(cur_active_width[6 as ::core::ffi::c_int as usize] + pitex_candidate_shrink);
                                        }
                                        b = badness(
                                            -g,
                                            (cur_active_width[6 as ::core::ffi::c_int as usize] + pitex_candidate_shrink),
                                        );
                                        if b > 12 as int32_t {
                                            fit_class = TIGHT_FIT as ::core::ffi::c_uchar;
                                        } else {
                                            fit_class = DECENT_FIT as ::core::ffi::c_uchar;
                                        }
                                        current_block = 7213206328819867990;
                                    } else {
                                        current_block = 12153365054289215322;
                                    }
                                }
                            }
                        } else {
                            current_block = 12153365054289215322;
                        }
                        match current_block {
                            7213206328819867990 => {}
                            _ => {
                                shortfall = 0 as ::core::ffi::c_int as scaled_t;
                                current_block = 15138287738512116048;
                            }
                        }
                    } else {
                        current_block = 15138287738512116048;
                    }
                    match current_block {
                        7213206328819867990 => {}
                        _ => {
                            b = 0 as ::core::ffi::c_int as int32_t;
                            fit_class = DECENT_FIT as ::core::ffi::c_uchar;
                            current_block = 1995330570110937187;
                        }
                    }
                } else {
                    let mut current_block_230: u64;
                    if shortfall as ::core::ffi::c_long > 7230584 as ::core::ffi::c_long {
                        if ((cur_active_width[2 as ::core::ffi::c_int as usize] + pitex_candidate_stretch)
                            as ::core::ffi::c_long)
                            < 1663497 as ::core::ffi::c_long
                        {
                            b = INF_BAD as int32_t;
                            fit_class = VERY_LOOSE_FIT as ::core::ffi::c_uchar;
                            current_block_230 = 7192749559210104566;
                        } else {
                            current_block_230 = 382537808568233274;
                        }
                    } else {
                        current_block_230 = 382537808568233274;
                    }
                    match current_block_230 {
                        382537808568233274 => {
                            b = badness(
                                shortfall,
                                (cur_active_width[2 as ::core::ffi::c_int as usize] + pitex_candidate_stretch),
                            );
                            if b > 12 as int32_t {
                                if b > 99 as int32_t {
                                    fit_class = VERY_LOOSE_FIT as ::core::ffi::c_uchar;
                                } else {
                                    fit_class = LOOSE_FIT as ::core::ffi::c_uchar;
                                }
                            } else {
                                fit_class = DECENT_FIT as ::core::ffi::c_uchar;
                            }
                        }
                        _ => {}
                    }
                    current_block = 1995330570110937187;
                }
            } else {
                if -shortfall > (cur_active_width[6 as ::core::ffi::c_int as usize] + pitex_candidate_shrink) {
                    b = (INF_BAD + 1 as ::core::ffi::c_int) as int32_t;
                } else {
                    b = badness(
                        -shortfall,
                        (cur_active_width[6 as ::core::ffi::c_int as usize] + pitex_candidate_shrink),
                    );
                }
                if b > 12 as int32_t {
                    fit_class = TIGHT_FIT as ::core::ffi::c_uchar;
                } else {
                    fit_class = DECENT_FIT as ::core::ffi::c_uchar;
                }
                current_block = 1995330570110937187;
            }
            match current_block {
                1995330570110937187 => {
                    if do_last_line_fit {
                        if cur_p == TEX_NULL as int32_t {
                            shortfall = 0 as ::core::ffi::c_int as scaled_t;
                        }
                        if shortfall > 0 as scaled_t {
                            g = cur_active_width[2 as ::core::ffi::c_int as usize];
                        } else if shortfall < 0 as scaled_t {
                            g = cur_active_width[6 as ::core::ffi::c_int as usize];
                        } else {
                            g = 0 as ::core::ffi::c_int as scaled_t;
                        }
                    }
                }
                _ => {}
            }
            if b > INF_BAD as int32_t || pi == EJECT_PENALTY as int32_t {
                if final_pass as ::core::ffi::c_int != 0
                    && minimum_demerits == AWFUL_BAD as int32_t
                    && (*mem.offset(r as isize)).b32.s1 == LAST_ACTIVE as int32_t
                    && prev_r == ACTIVE_LIST as int32_t
                {
                    artificial_demerits = true_0 != 0;
                    current_block = 1254794021369287194;
                } else if b > threshold {
                    current_block = 13762550348153164112;
                } else {
                    current_block = 1254794021369287194;
                }
                match current_block {
                    13762550348153164112 => {}
                    _ => {
                        node_r_stays_active = false_0 != 0;
                        current_block = 3304481414499905106;
                    }
                }
            } else {
                prev_r = r;
                if b > threshold {
                    continue;
                }
                node_r_stays_active = true_0 != 0;
                current_block = 3304481414499905106;
            }
            match current_block {
                3304481414499905106 => {
                    if artificial_demerits {
                        d = 0 as ::core::ffi::c_int as int32_t;
                    } else {
                        d = (*eqtb.offset((INT_BASE + INT_PAR__line_penalty) as isize))
                            .b32
                            .s1
                            + b;
                        if abs(d as ::core::ffi::c_int) >= 10000 as ::core::ffi::c_int {
                            d = 100000000 as int32_t;
                        } else {
                            d = d * d;
                        }
                        if pi != 0 as int32_t {
                            if pi > 0 as int32_t {
                                d = d + pi * pi;
                            } else if pi > EJECT_PENALTY as int32_t {
                                d = d - pi * pi;
                            }
                        }
                        if break_type as ::core::ffi::c_int == HYPHENATED
                            && (*mem.offset(r as isize)).b16.s1 as ::core::ffi::c_int == HYPHENATED
                        {
                            if cur_p != TEX_NULL as int32_t {
                                d = d
                                    + (*eqtb.offset(
                                        (INT_BASE + INT_PAR__double_hyphen_demerits) as isize,
                                    ))
                                    .b32
                                    .s1;
                            } else {
                                d = d
                                    + (*eqtb.offset(
                                        (INT_BASE + INT_PAR__final_hyphen_demerits) as isize,
                                    ))
                                    .b32
                                    .s1;
                            }
                        }
                        if abs(fit_class as ::core::ffi::c_int
                            - (*mem.offset(r as isize)).b16.s0 as ::core::ffi::c_int)
                            > 1 as ::core::ffi::c_int
                        {
                            d = d
                                + (*eqtb.offset((INT_BASE + INT_PAR__adj_demerits) as isize))
                                    .b32
                                    .s1;
                        }
                    }
                    d = d + (*mem.offset((r + 2 as int32_t) as isize)).b32.s1;
                    if d <= minimal_demerits[fit_class as usize] {
                        minimal_demerits[fit_class as usize] = d;
                        best_place[fit_class as usize] =
                            (*mem.offset((r + 1 as int32_t) as isize)).b32.s1;
                        best_pl_line[fit_class as usize] = l;
                        if do_last_line_fit {
                            best_pl_short[fit_class as usize] = shortfall;
                            best_pl_glue[fit_class as usize] = g;
                        }
                        if d < minimum_demerits {
                            minimum_demerits = d;
                        }
                    }
                    if node_r_stays_active {
                        continue;
                    }
                }
                _ => {}
            }
            (*mem.offset(prev_r as isize)).b32.s1 = (*mem.offset(r as isize)).b32.s1;
            free_node(r, active_node_size as int32_t);
            if prev_r == ACTIVE_LIST as int32_t {
                r = (*mem
                    .offset((4999999 as ::core::ffi::c_int - 7 as ::core::ffi::c_int) as isize))
                .b32
                .s1;
                if (*mem.offset(r as isize)).b16.s1 as ::core::ffi::c_int == DELTA_NODE {
                    active_width[1 as ::core::ffi::c_int as usize] +=
                        (*mem.offset((r + 1 as int32_t) as isize)).b32.s1;
                    active_width[2 as ::core::ffi::c_int as usize] +=
                        (*mem.offset((r + 2 as int32_t) as isize)).b32.s1;
                    active_width[3 as ::core::ffi::c_int as usize] +=
                        (*mem.offset((r + 3 as int32_t) as isize)).b32.s1;
                    active_width[4 as ::core::ffi::c_int as usize] +=
                        (*mem.offset((r + 4 as int32_t) as isize)).b32.s1;
                    active_width[5 as ::core::ffi::c_int as usize] +=
                        (*mem.offset((r + 5 as int32_t) as isize)).b32.s1;
                    active_width[6 as ::core::ffi::c_int as usize] +=
                        (*mem.offset((r + 6 as int32_t) as isize)).b32.s1;
                    cur_active_width[1 as ::core::ffi::c_int as usize] =
                        active_width[1 as ::core::ffi::c_int as usize];
                    cur_active_width[2 as ::core::ffi::c_int as usize] =
                        active_width[2 as ::core::ffi::c_int as usize];
                    cur_active_width[3 as ::core::ffi::c_int as usize] =
                        active_width[3 as ::core::ffi::c_int as usize];
                    cur_active_width[4 as ::core::ffi::c_int as usize] =
                        active_width[4 as ::core::ffi::c_int as usize];
                    cur_active_width[5 as ::core::ffi::c_int as usize] =
                        active_width[5 as ::core::ffi::c_int as usize];
                    cur_active_width[6 as ::core::ffi::c_int as usize] =
                        active_width[6 as ::core::ffi::c_int as usize];
                    (*mem.offset(
                        (4999999 as ::core::ffi::c_int - 7 as ::core::ffi::c_int) as isize,
                    ))
                    .b32
                    .s1 = (*mem.offset(r as isize)).b32.s1;
                    free_node(r, DELTA_NODE_SIZE as int32_t);
                }
            } else if (*mem.offset(prev_r as isize)).b16.s1 as ::core::ffi::c_int == DELTA_NODE {
                r = (*mem.offset(prev_r as isize)).b32.s1;
                if r == LAST_ACTIVE as int32_t {
                    cur_active_width[1 as ::core::ffi::c_int as usize] -=
                        (*mem.offset((prev_r + 1 as int32_t) as isize)).b32.s1;
                    cur_active_width[2 as ::core::ffi::c_int as usize] -=
                        (*mem.offset((prev_r + 2 as int32_t) as isize)).b32.s1;
                    cur_active_width[3 as ::core::ffi::c_int as usize] -=
                        (*mem.offset((prev_r + 3 as int32_t) as isize)).b32.s1;
                    cur_active_width[4 as ::core::ffi::c_int as usize] -=
                        (*mem.offset((prev_r + 4 as int32_t) as isize)).b32.s1;
                    cur_active_width[5 as ::core::ffi::c_int as usize] -=
                        (*mem.offset((prev_r + 5 as int32_t) as isize)).b32.s1;
                    cur_active_width[6 as ::core::ffi::c_int as usize] -=
                        (*mem.offset((prev_r + 6 as int32_t) as isize)).b32.s1;
                    (*mem.offset(prev_prev_r as isize)).b32.s1 = LAST_ACTIVE as int32_t;
                    free_node(prev_r, DELTA_NODE_SIZE as int32_t);
                    prev_r = prev_prev_r;
                } else if (*mem.offset(r as isize)).b16.s1 as ::core::ffi::c_int == DELTA_NODE {
                    cur_active_width[1 as ::core::ffi::c_int as usize] +=
                        (*mem.offset((r + 1 as int32_t) as isize)).b32.s1;
                    cur_active_width[2 as ::core::ffi::c_int as usize] +=
                        (*mem.offset((r + 2 as int32_t) as isize)).b32.s1;
                    cur_active_width[3 as ::core::ffi::c_int as usize] +=
                        (*mem.offset((r + 3 as int32_t) as isize)).b32.s1;
                    cur_active_width[4 as ::core::ffi::c_int as usize] +=
                        (*mem.offset((r + 4 as int32_t) as isize)).b32.s1;
                    cur_active_width[5 as ::core::ffi::c_int as usize] +=
                        (*mem.offset((r + 5 as int32_t) as isize)).b32.s1;
                    cur_active_width[6 as ::core::ffi::c_int as usize] +=
                        (*mem.offset((r + 6 as int32_t) as isize)).b32.s1;
                    (*mem.offset((prev_r + 1 as int32_t) as isize)).b32.s1 +=
                        (*mem.offset((r + 1 as int32_t) as isize)).b32.s1;
                    (*mem.offset((prev_r + 2 as int32_t) as isize)).b32.s1 +=
                        (*mem.offset((r + 2 as int32_t) as isize)).b32.s1;
                    (*mem.offset((prev_r + 3 as int32_t) as isize)).b32.s1 +=
                        (*mem.offset((r + 3 as int32_t) as isize)).b32.s1;
                    (*mem.offset((prev_r + 4 as int32_t) as isize)).b32.s1 +=
                        (*mem.offset((r + 4 as int32_t) as isize)).b32.s1;
                    (*mem.offset((prev_r + 5 as int32_t) as isize)).b32.s1 +=
                        (*mem.offset((r + 5 as int32_t) as isize)).b32.s1;
                    (*mem.offset((prev_r + 6 as int32_t) as isize)).b32.s1 +=
                        (*mem.offset((r + 6 as int32_t) as isize)).b32.s1;
                    (*mem.offset(prev_r as isize)).b32.s1 = (*mem.offset(r as isize)).b32.s1;
                    free_node(r, DELTA_NODE_SIZE as int32_t);
                }
            }
        }
    }
}
unsafe extern "C" fn hyphenate() {
    let mut current_block: u64;
    let mut i: ::core::ffi::c_short = 0;
    let mut j: ::core::ffi::c_short = 0;
    let mut l: ::core::ffi::c_short = 0;
    let mut q: int32_t = 0;
    let mut r: int32_t = 0;
    let mut s: int32_t = 0;
    let mut bchar: int32_t = 0;
    let mut major_tail: int32_t = 0;
    let mut minor_tail: int32_t = 0;
    let mut c: UnicodeScalar = 0 as UnicodeScalar;
    let mut c_loc: ::core::ffi::c_short = 0;
    let mut r_count: int32_t = 0;
    let mut hyf_node: int32_t = 0;
    let mut z: trie_pointer = 0;
    let mut v: int32_t = 0;
    let mut h: hyph_pointer = 0;
    let mut k: str_number = 0;
    let mut u: pool_pointer = 0;
    let mut for_end: int32_t = 0;
    j = 0 as ::core::ffi::c_short;
    for_end = hn as int32_t;
    if j as int32_t <= for_end {
        loop {
            hyf[j as usize] = 0 as ::core::ffi::c_uchar;
            let fresh5 = j;
            j = j + 1;
            if !((fresh5 as int32_t) < for_end) {
                break;
            }
        }
    }
    h = hc[1 as ::core::ffi::c_int as usize] as hyph_pointer;
    hn += 1;
    hc[hn as usize] = cur_lang as int32_t;
    let mut for_end_0: int32_t = 0;
    j = 2 as ::core::ffi::c_short;
    for_end_0 = hn as int32_t;
    if j as int32_t <= for_end_0 {
        loop {
            h = ((h as int32_t + h as int32_t + hc[j as usize]) % HYPH_PRIME as int32_t)
                as hyph_pointer;
            let fresh6 = j;
            j = j + 1;
            if !((fresh6 as int32_t) < for_end_0) {
                break;
            }
        }
    }
    loop {
        k = *hyph_word.offset(h as isize);
        if k == 0 as str_number {
            current_block = 656890028679738177;
            break;
        }
        if length(k) == hn as int32_t {
            j = 1 as ::core::ffi::c_short;
            u = *str_start
                .offset((k as ::core::ffi::c_long - 65536 as ::core::ffi::c_long) as isize);
            loop {
                if *str_pool.offset(u as isize) as int32_t != hc[j as usize] {
                    current_block = 2308514170601998468;
                    break;
                }
                j += 1;
                u += 1;
                if j as ::core::ffi::c_int > hn as ::core::ffi::c_int {
                    current_block = 9828876828309294594;
                    break;
                }
            }
            match current_block {
                2308514170601998468 => {}
                _ => {
                    s = *hyph_list.offset(h as isize);
                    while s != TEX_NULL as int32_t {
                        hyf[(*mem.offset(s as isize)).b32.s0 as usize] = 1 as ::core::ffi::c_uchar;
                        s = (*mem.offset(s as isize)).b32.s1;
                    }
                    hn -= 1;
                    current_block = 12611501219888165127;
                    break;
                }
            }
        }
        h = *hyph_link.offset(h as isize);
        if h as ::core::ffi::c_int == 0 as ::core::ffi::c_int {
            current_block = 656890028679738177;
            break;
        }
        h = h.wrapping_sub(1);
    }
    match current_block {
        656890028679738177 => {
            hn -= 1;
            if *trie_trc.offset((cur_lang as ::core::ffi::c_int + 1 as ::core::ffi::c_int) as isize)
                as ::core::ffi::c_int
                != cur_lang as ::core::ffi::c_int
            {
                return;
            }
            hc[0 as ::core::ffi::c_int as usize] = 0 as ::core::ffi::c_int as int32_t;
            hc[(hn as ::core::ffi::c_int + 1 as ::core::ffi::c_int) as usize] =
                0 as ::core::ffi::c_int as int32_t;
            hc[(hn as ::core::ffi::c_int + 2 as ::core::ffi::c_int) as usize] = max_hyph_char;
            let mut for_end_1: int32_t = 0;
            j = 0 as ::core::ffi::c_short;
            for_end_1 = hn as int32_t - r_hyf + 1 as int32_t;
            if j as int32_t <= for_end_1 {
                loop {
                    z = *trie_trl.offset(
                        (cur_lang as ::core::ffi::c_int + 1 as ::core::ffi::c_int) as isize,
                    ) + hc[j as usize];
                    l = j;
                    while hc[l as usize] == *trie_trc.offset(z as isize) as int32_t {
                        if *trie_tro.offset(z as isize) != MIN_TRIE_OP as trie_pointer {
                            v = *trie_tro.offset(z as isize) as int32_t;
                            loop {
                                v = v + op_start[cur_lang as usize];
                                i = (l as ::core::ffi::c_int
                                    - hyf_distance[v as usize] as ::core::ffi::c_int)
                                    as ::core::ffi::c_short;
                                if hyf_num[v as usize] as ::core::ffi::c_int
                                    > hyf[i as usize] as ::core::ffi::c_int
                                {
                                    hyf[i as usize] = hyf_num[v as usize] as ::core::ffi::c_uchar;
                                }
                                v = hyf_next[v as usize] as int32_t;
                                if v == MIN_TRIE_OP as int32_t {
                                    break;
                                }
                            }
                        }
                        l += 1;
                        z = *trie_trl.offset(z as isize) + hc[l as usize];
                    }
                    let fresh7 = j;
                    j = j + 1;
                    if !((fresh7 as int32_t) < for_end_1) {
                        break;
                    }
                }
            }
        }
        _ => {}
    }
    let mut for_end_2: int32_t = 0;
    j = 0 as ::core::ffi::c_short;
    for_end_2 = l_hyf - 1 as int32_t;
    if j as int32_t <= for_end_2 {
        loop {
            hyf[j as usize] = 0 as ::core::ffi::c_uchar;
            let fresh8 = j;
            j = j + 1;
            if !((fresh8 as int32_t) < for_end_2) {
                break;
            }
        }
    }
    let mut for_end_3: int32_t = 0;
    j = 0 as ::core::ffi::c_short;
    for_end_3 = r_hyf - 1 as int32_t;
    if j as int32_t <= for_end_3 {
        loop {
            hyf[(hn as ::core::ffi::c_int - j as ::core::ffi::c_int) as usize] =
                0 as ::core::ffi::c_uchar;
            let fresh9 = j;
            j = j + 1;
            if !((fresh9 as int32_t) < for_end_3) {
                break;
            }
        }
    }
    let mut for_end_4: int32_t = 0;
    j = l_hyf as ::core::ffi::c_short;
    for_end_4 = hn as int32_t - r_hyf;
    if j as int32_t <= for_end_4 {
        current_block = 4216521074440650966;
    } else {
        current_block = 5684854171168229155;
    }
    loop {
        match current_block {
            5684854171168229155 => return,
            _ => {
                if hyf[j as usize] as ::core::ffi::c_int & 1 as ::core::ffi::c_int != 0 {
                    break;
                }
                let fresh10 = j;
                j = j + 1;
                if (fresh10 as int32_t) < for_end_4 {
                    current_block = 4216521074440650966;
                } else {
                    current_block = 5684854171168229155;
                }
            }
        }
    }
    if ha != TEX_NULL as int32_t
        && !is_char_node(ha)
        && (*mem.offset(ha as isize)).b16.s1 as ::core::ffi::c_int == WHATSIT_NODE
        && ((*mem.offset(ha as isize)).b16.s0 as ::core::ffi::c_int == NATIVE_WORD_NODE
            || (*mem.offset(ha as isize)).b16.s0 as ::core::ffi::c_int == NATIVE_WORD_NODE_AT)
    {
        s = cur_p;
        while (*mem.offset(s as isize)).b32.s1 != ha {
            s = (*mem.offset(s as isize)).b32.s1;
        }
        hyphen_passed = 0 as small_number;
        let mut for_end_5: int32_t = 0;
        j = l_hyf as ::core::ffi::c_short;
        for_end_5 = hn as int32_t - r_hyf;
        if j as int32_t <= for_end_5 {
            loop {
                if hyf[j as usize] as ::core::ffi::c_int & 1 as ::core::ffi::c_int != 0 {
                    q = new_native_word_node(hf, j as int32_t - hyphen_passed as int32_t);
                    (*mem.offset(q as isize)).b16.s0 = (*mem.offset(ha as isize)).b16.s0;
                    let mut for_end_6: int32_t = 0;
                    i = 0 as ::core::ffi::c_short;
                    for_end_6 = (j as ::core::ffi::c_int
                        - hyphen_passed as ::core::ffi::c_int
                        - 1 as ::core::ffi::c_int) as int32_t;
                    if i as int32_t <= for_end_6 {
                        loop {
                            *(mem.offset((q + NATIVE_NODE_SIZE as int32_t) as isize)
                                as *mut memory_word
                                as *mut ::core::ffi::c_ushort)
                                .offset(i as isize) = *(mem
                                .offset((ha + NATIVE_NODE_SIZE as int32_t) as isize)
                                as *mut memory_word
                                as *mut ::core::ffi::c_ushort)
                                .offset(
                                    (i as ::core::ffi::c_int + hyphen_passed as ::core::ffi::c_int)
                                        as isize,
                                );
                            let fresh11 = i;
                            i = i + 1;
                            if !((fresh11 as int32_t) < for_end_6) {
                                break;
                            }
                        }
                    }
                    measure_native_node(
                        mem.offset(q as isize) as *mut memory_word as *mut ::core::ffi::c_void,
                        ((*eqtb.offset(
                            (7826729 as ::core::ffi::c_int + 72 as ::core::ffi::c_int) as isize,
                        ))
                        .b32
                        .s1 > 0 as int32_t) as ::core::ffi::c_int,
                    );
                    (*mem.offset(s as isize)).b32.s1 = q;
                    s = q;
                    q = new_disc();
                    (*mem.offset((q + 1 as int32_t) as isize)).b32.s0 =
                        new_native_character(hf, hyf_char as UnicodeScalar);
                    (*mem.offset(s as isize)).b32.s1 = q;
                    s = q;
                    hyphen_passed = j as small_number;
                }
                let fresh12 = j;
                j = j + 1;
                if !((fresh12 as int32_t) < for_end_5) {
                    break;
                }
            }
        }
        hn = (*mem.offset((ha + 4 as int32_t) as isize)).b16.s1 as small_number;
        q = new_native_word_node(hf, hn as int32_t - hyphen_passed as int32_t);
        (*mem.offset(q as isize)).b16.s0 = (*mem.offset(ha as isize)).b16.s0;
        let mut for_end_7: int32_t = 0;
        i = 0 as ::core::ffi::c_short;
        for_end_7 = (hn as ::core::ffi::c_int
            - hyphen_passed as ::core::ffi::c_int
            - 1 as ::core::ffi::c_int) as int32_t;
        if i as int32_t <= for_end_7 {
            loop {
                *(mem.offset((q + NATIVE_NODE_SIZE as int32_t) as isize) as *mut memory_word
                    as *mut ::core::ffi::c_ushort)
                    .offset(i as isize) = *(mem.offset((ha + NATIVE_NODE_SIZE as int32_t) as isize)
                    as *mut memory_word
                    as *mut ::core::ffi::c_ushort)
                    .offset(
                        (i as ::core::ffi::c_int + hyphen_passed as ::core::ffi::c_int) as isize,
                    );
                let fresh13 = i;
                i = i + 1;
                if !((fresh13 as int32_t) < for_end_7) {
                    break;
                }
            }
        }
        measure_native_node(
            mem.offset(q as isize) as *mut memory_word as *mut ::core::ffi::c_void,
            ((*eqtb.offset((7826729 as ::core::ffi::c_int + 72 as ::core::ffi::c_int) as isize))
                .b32
                .s1
                > 0 as int32_t) as ::core::ffi::c_int,
        );
        (*mem.offset(s as isize)).b32.s1 = q;
        s = q;
        q = (*mem.offset(ha as isize)).b32.s1;
        (*mem.offset(s as isize)).b32.s1 = q;
        (*mem.offset(ha as isize)).b32.s1 = TEX_NULL as int32_t;
        flush_node_list(ha);
    } else {
        q = (*mem.offset(hb as isize)).b32.s1;
        (*mem.offset(hb as isize)).b32.s1 = TEX_NULL as int32_t;
        r = (*mem.offset(ha as isize)).b32.s1;
        (*mem.offset(ha as isize)).b32.s1 = TEX_NULL as int32_t;
        bchar = hyf_bchar;
        if is_char_node(ha) {
            if (*mem.offset(ha as isize)).b16.s1 as internal_font_number != hf {
                current_block = 14575669455108526638;
            } else {
                init_list = ha;
                init_lig = false_0 != 0;
                hu[0 as ::core::ffi::c_int as usize] = (*mem.offset(ha as isize)).b16.s0 as int32_t;
                current_block = 3921975509081277429;
            }
        } else if (*mem.offset(ha as isize)).b16.s1 as ::core::ffi::c_int == LIGATURE_NODE {
            if (*mem.offset((ha + 1 as int32_t) as isize)).b16.s1 as internal_font_number != hf {
                current_block = 14575669455108526638;
            } else {
                init_list = (*mem.offset((ha + 1 as int32_t) as isize)).b32.s1;
                init_lig = true_0 != 0;
                init_lft = (*mem.offset(ha as isize)).b16.s0 as ::core::ffi::c_int
                    > 1 as ::core::ffi::c_int;
                hu[0 as ::core::ffi::c_int as usize] =
                    (*mem.offset((ha + 1 as int32_t) as isize)).b16.s0 as int32_t;
                if init_list == TEX_NULL as int32_t {
                    if init_lft {
                        hu[0 as ::core::ffi::c_int as usize] = max_hyph_char;
                        init_lig = false_0 != 0;
                    }
                }
                free_node(ha, SMALL_NODE_SIZE as int32_t);
                current_block = 3921975509081277429;
            }
        } else {
            if !is_char_node(r) {
                if (*mem.offset(r as isize)).b16.s1 as ::core::ffi::c_int == LIGATURE_NODE {
                    if (*mem.offset(r as isize)).b16.s0 as ::core::ffi::c_int
                        > 1 as ::core::ffi::c_int
                    {
                        current_block = 14575669455108526638;
                    } else {
                        current_block = 17212496701767205014;
                    }
                } else {
                    current_block = 17212496701767205014;
                }
            } else {
                current_block = 17212496701767205014;
            }
            match current_block {
                14575669455108526638 => {}
                _ => {
                    j = 1 as ::core::ffi::c_short;
                    s = ha;
                    init_list = TEX_NULL as int32_t;
                    current_block = 2094217840710667113;
                }
            }
        }
        match current_block {
            3921975509081277429 => {
                s = cur_p;
                while (*mem.offset(s as isize)).b32.s1 != ha {
                    s = (*mem.offset(s as isize)).b32.s1;
                }
                j = 0 as ::core::ffi::c_short;
            }
            14575669455108526638 => {
                s = ha;
                j = 0 as ::core::ffi::c_short;
                hu[0 as ::core::ffi::c_int as usize] = max_hyph_char;
                init_lig = false_0 != 0;
                init_list = TEX_NULL as int32_t;
            }
            _ => {}
        }
        flush_node_list(r);
        loop {
            l = j;
            j = (reconstitute(j as small_number, hn, bchar, hyf_char) as ::core::ffi::c_int
                + 1 as ::core::ffi::c_int) as ::core::ffi::c_short;
            if hyphen_passed as ::core::ffi::c_int == 0 as ::core::ffi::c_int {
                (*mem.offset(s as isize)).b32.s1 = (*mem.offset(HOLD_HEAD as isize)).b32.s1;
                while (*mem.offset(s as isize)).b32.s1 > TEX_NULL as int32_t {
                    s = (*mem.offset(s as isize)).b32.s1;
                }
                if hyf[(j as ::core::ffi::c_int - 1 as ::core::ffi::c_int) as usize]
                    as ::core::ffi::c_int
                    & 1 as ::core::ffi::c_int
                    != 0
                {
                    l = j;
                    hyphen_passed =
                        (j as ::core::ffi::c_int - 1 as ::core::ffi::c_int) as small_number;
                    (*mem.offset(HOLD_HEAD as isize)).b32.s1 = TEX_NULL as int32_t;
                }
            }
            if hyphen_passed as ::core::ffi::c_int > 0 as ::core::ffi::c_int {
                loop {
                    r = get_node(SMALL_NODE_SIZE as int32_t);
                    (*mem.offset(r as isize)).b32.s1 = (*mem.offset(HOLD_HEAD as isize)).b32.s1;
                    (*mem.offset(r as isize)).b16.s1 = DISC_NODE as uint16_t;
                    major_tail = r;
                    r_count = 0 as ::core::ffi::c_int as int32_t;
                    while (*mem.offset(major_tail as isize)).b32.s1 > TEX_NULL as int32_t {
                        major_tail = (*mem.offset(major_tail as isize)).b32.s1;
                        r_count += 1;
                    }
                    i = hyphen_passed as ::core::ffi::c_short;
                    hyf[i as usize] = 0 as ::core::ffi::c_uchar;
                    minor_tail = TEX_NULL as int32_t;
                    (*mem.offset((r + 1 as int32_t) as isize)).b32.s0 = TEX_NULL as int32_t;
                    hyf_node = new_character(hf, hyf_char as UTF16_code);
                    if hyf_node != TEX_NULL as int32_t {
                        i += 1;
                        c = hu[i as usize] as UnicodeScalar;
                        hu[i as usize] = hyf_char;
                        (*mem.offset(hyf_node as isize)).b32.s1 = avail;
                        avail = hyf_node;
                    }
                    while l as ::core::ffi::c_int <= i as ::core::ffi::c_int {
                        l = (reconstitute(
                            l as small_number,
                            i as small_number,
                            *font_bchar.offset(hf as isize) as int32_t,
                            TOO_BIG_CHAR as int32_t,
                        ) as ::core::ffi::c_int
                            + 1 as ::core::ffi::c_int)
                            as ::core::ffi::c_short;
                        if (*mem.offset(HOLD_HEAD as isize)).b32.s1 > TEX_NULL as int32_t {
                            if minor_tail == TEX_NULL as int32_t {
                                (*mem.offset((r + 1 as int32_t) as isize)).b32.s0 =
                                    (*mem.offset(HOLD_HEAD as isize)).b32.s1;
                            } else {
                                (*mem.offset(minor_tail as isize)).b32.s1 =
                                    (*mem.offset(HOLD_HEAD as isize)).b32.s1;
                            }
                            minor_tail = (*mem.offset(HOLD_HEAD as isize)).b32.s1;
                            while (*mem.offset(minor_tail as isize)).b32.s1 > TEX_NULL as int32_t {
                                minor_tail = (*mem.offset(minor_tail as isize)).b32.s1;
                            }
                        }
                    }
                    if hyf_node != TEX_NULL as int32_t {
                        hu[i as usize] = c as int32_t;
                        l = i;
                        i -= 1;
                    }
                    minor_tail = TEX_NULL as int32_t;
                    (*mem.offset((r + 1 as int32_t) as isize)).b32.s1 = TEX_NULL as int32_t;
                    c_loc = 0 as ::core::ffi::c_short;
                    if *bchar_label.offset(hf as isize) != NON_ADDRESS as font_index {
                        l -= 1;
                        c = hu[l as usize] as UnicodeScalar;
                        c_loc = l;
                        hu[l as usize] = max_hyph_char;
                    }
                    while (l as ::core::ffi::c_int) < j as ::core::ffi::c_int {
                        loop {
                            l = (reconstitute(l as small_number, hn, bchar, TOO_BIG_CHAR as int32_t)
                                as ::core::ffi::c_int
                                + 1 as ::core::ffi::c_int)
                                as ::core::ffi::c_short;
                            if c_loc as ::core::ffi::c_int > 0 as ::core::ffi::c_int {
                                hu[c_loc as usize] = c as int32_t;
                                c_loc = 0 as ::core::ffi::c_short;
                            }
                            if (*mem.offset(HOLD_HEAD as isize)).b32.s1 > TEX_NULL as int32_t {
                                if minor_tail == TEX_NULL as int32_t {
                                    (*mem.offset((r + 1 as int32_t) as isize)).b32.s1 =
                                        (*mem.offset(HOLD_HEAD as isize)).b32.s1;
                                } else {
                                    (*mem.offset(minor_tail as isize)).b32.s1 =
                                        (*mem.offset(HOLD_HEAD as isize)).b32.s1;
                                }
                                minor_tail = (*mem.offset(HOLD_HEAD as isize)).b32.s1;
                                while (*mem.offset(minor_tail as isize)).b32.s1
                                    > TEX_NULL as int32_t
                                {
                                    minor_tail = (*mem.offset(minor_tail as isize)).b32.s1;
                                }
                            }
                            if l as ::core::ffi::c_int >= j as ::core::ffi::c_int {
                                break;
                            }
                        }
                        while l as ::core::ffi::c_int > j as ::core::ffi::c_int {
                            j = (reconstitute(j as small_number, hn, bchar, TOO_BIG_CHAR as int32_t)
                                as ::core::ffi::c_int
                                + 1 as ::core::ffi::c_int)
                                as ::core::ffi::c_short;
                            (*mem.offset(major_tail as isize)).b32.s1 =
                                (*mem.offset(HOLD_HEAD as isize)).b32.s1;
                            while (*mem.offset(major_tail as isize)).b32.s1 > TEX_NULL as int32_t {
                                major_tail = (*mem.offset(major_tail as isize)).b32.s1;
                                r_count += 1;
                            }
                        }
                    }
                    if r_count > 127 as int32_t {
                        (*mem.offset(s as isize)).b32.s1 = (*mem.offset(r as isize)).b32.s1;
                        (*mem.offset(r as isize)).b32.s1 = TEX_NULL as int32_t;
                        flush_node_list(r);
                    } else {
                        (*mem.offset(s as isize)).b32.s1 = r;
                        (*mem.offset(r as isize)).b16.s0 = r_count as uint16_t;
                    }
                    s = major_tail;
                    hyphen_passed =
                        (j as ::core::ffi::c_int - 1 as ::core::ffi::c_int) as small_number;
                    (*mem.offset(HOLD_HEAD as isize)).b32.s1 = TEX_NULL as int32_t;
                    if !(hyf[(j as ::core::ffi::c_int - 1 as ::core::ffi::c_int) as usize]
                        as ::core::ffi::c_int
                        & 1 as ::core::ffi::c_int
                        != 0)
                    {
                        break;
                    }
                }
            }
            if j as ::core::ffi::c_int > hn as ::core::ffi::c_int {
                break;
            }
        }
        (*mem.offset(s as isize)).b32.s1 = q;
        flush_list(init_list);
    };
}
unsafe extern "C" fn finite_shrink(mut p: int32_t) -> int32_t {
    let mut q: int32_t = 0;
    if no_shrink_error_yet {
        no_shrink_error_yet = false_0 != 0;
        error_here_with_diagnostic(
            b"Infinite glue shrinkage found in a paragraph\0" as *const u8
                as *const ::core::ffi::c_char,
        );
        capture_to_diagnostic(::core::ptr::null_mut::<ttbc_diagnostic_t>());
        help_ptr = 5 as ::core::ffi::c_uchar;
        help_line[4 as ::core::ffi::c_int as usize] =
            b"The paragraph just ended includes some glue that has\0" as *const u8
                as *const ::core::ffi::c_char;
        help_line[3 as ::core::ffi::c_int as usize] =
            b"infinite shrinkability, e.g., `\\hskip 0pt minus 1fil'.\0" as *const u8
                as *const ::core::ffi::c_char;
        help_line[2 as ::core::ffi::c_int as usize] =
            b"Such glue doesn't belong there---it allows a paragraph\0" as *const u8
                as *const ::core::ffi::c_char;
        help_line[1 as ::core::ffi::c_int as usize] =
            b"of any length to fit on one line. But it's safe to proceed,\0" as *const u8
                as *const ::core::ffi::c_char;
        help_line[0 as ::core::ffi::c_int as usize] =
            b"since the offensive shrinkability has been made finite.\0" as *const u8
                as *const ::core::ffi::c_char;
        error();
    }
    q = new_spec(p);
    (*mem.offset(q as isize)).b16.s0 = NORMAL as uint16_t;
    delete_glue_ref(p);
    return q;
}
unsafe extern "C" fn reconstitute(
    mut j: small_number,
    mut n: small_number,
    mut bchar: int32_t,
    mut hchar: int32_t,
) -> small_number {
    let mut current_block: u64;
    let mut p: int32_t = 0;
    let mut t: int32_t = 0;
    let mut q: b16x4 = b16x4_le_t {
        s0: 0,
        s1: 0,
        s2: 0,
        s3: 0,
    };
    let mut cur_rh: int32_t = 0;
    let mut test_char: int32_t = 0;
    let mut w: scaled_t = 0;
    let mut k: font_index = 0;
    hyphen_passed = 0 as small_number;
    t = HOLD_HEAD as int32_t;
    w = 0 as ::core::ffi::c_int as scaled_t;
    (*mem.offset(HOLD_HEAD as isize)).b32.s1 = TEX_NULL as int32_t;
    cur_l = hu[j as usize];
    cur_q = t;
    if j as ::core::ffi::c_int == 0 as ::core::ffi::c_int {
        ligature_present = init_lig;
        p = init_list;
        if ligature_present {
            lft_hit = init_lft;
        }
        while p > TEX_NULL as int32_t {
            (*mem.offset(t as isize)).b32.s1 = get_avail();
            t = (*mem.offset(t as isize)).b32.s1;
            (*mem.offset(t as isize)).b16.s1 = hf as uint16_t;
            (*mem.offset(t as isize)).b16.s0 = (*mem.offset(p as isize)).b16.s0;
            p = (*mem.offset(p as isize)).b32.s1;
        }
    } else if cur_l < TOO_BIG_CHAR as int32_t {
        (*mem.offset(t as isize)).b32.s1 = get_avail();
        t = (*mem.offset(t as isize)).b32.s1;
        (*mem.offset(t as isize)).b16.s1 = hf as uint16_t;
        (*mem.offset(t as isize)).b16.s0 = cur_l as uint16_t;
    }
    lig_stack = TEX_NULL as int32_t;
    if (j as ::core::ffi::c_int) < n as ::core::ffi::c_int {
        cur_r = hu[(j as ::core::ffi::c_int + 1 as ::core::ffi::c_int) as usize];
    } else {
        cur_r = bchar;
    }
    if hyf[j as usize] as ::core::ffi::c_int & 1 as ::core::ffi::c_int != 0 {
        cur_rh = hchar;
    } else {
        cur_rh = TOO_BIG_CHAR as int32_t;
    }
    '_continue_: loop {
        if cur_l == TOO_BIG_CHAR as int32_t {
            k = *bchar_label.offset(hf as isize);
            if k == NON_ADDRESS as font_index {
                current_block = 1742620578862137412;
            } else {
                q = (*font_info.offset(k as isize)).b16;
                current_block = 4090602189656566074;
            }
        } else {
            q = (*font_info.offset(
                (*char_base.offset(hf as isize)
                    + effective_char(1 as ::core::ffi::c_int != 0, hf, cur_l as uint16_t))
                    as isize,
            ))
            .b16;
            if q.s1 as ::core::ffi::c_int % 4 as ::core::ffi::c_int != LIG_TAG {
                current_block = 1742620578862137412;
            } else {
                k = (*lig_kern_base.offset(hf as isize) + q.s0 as int32_t) as font_index;
                q = (*font_info.offset(k as isize)).b16;
                if q.s3 as ::core::ffi::c_int > 128 as ::core::ffi::c_int {
                    k = ((*lig_kern_base.offset(hf as isize)
                        + 256 as int32_t * q.s1 as int32_t
                        + q.s0 as int32_t) as ::core::ffi::c_long
                        + 32768 as ::core::ffi::c_long
                        - (256 as ::core::ffi::c_int * 128 as ::core::ffi::c_int)
                            as ::core::ffi::c_long) as font_index;
                    q = (*font_info.offset(k as isize)).b16;
                }
                current_block = 4090602189656566074;
            }
        }
        match current_block {
            4090602189656566074 => {
                if cur_rh < TOO_BIG_CHAR as int32_t {
                    test_char = cur_rh;
                } else {
                    test_char = cur_r;
                }
                loop {
                    if q.s2 as int32_t == test_char {
                        if q.s3 as ::core::ffi::c_int <= 128 as ::core::ffi::c_int {
                            if cur_rh < TOO_BIG_CHAR as int32_t {
                                hyphen_passed = j;
                                hchar = TOO_BIG_CHAR as int32_t;
                                cur_rh = TOO_BIG_CHAR as int32_t;
                                continue '_continue_;
                            } else {
                                if hchar < TOO_BIG_CHAR as int32_t {
                                    if hyf[j as usize] as ::core::ffi::c_int
                                        & 1 as ::core::ffi::c_int
                                        != 0
                                    {
                                        hyphen_passed = j;
                                        hchar = TOO_BIG_CHAR as int32_t;
                                    }
                                }
                                if (q.s1 as ::core::ffi::c_int) < 128 as ::core::ffi::c_int {
                                    if cur_l == TOO_BIG_CHAR as int32_t {
                                        lft_hit = true_0 != 0;
                                    }
                                    if j as ::core::ffi::c_int == n as ::core::ffi::c_int {
                                        if lig_stack == TEX_NULL as int32_t {
                                            rt_hit = true_0 != 0;
                                        }
                                    }
                                    match q.s1 as ::core::ffi::c_int {
                                        1 | 5 => {
                                            cur_l = q.s0 as int32_t;
                                            ligature_present = true_0 != 0;
                                        }
                                        2 | 6 => {
                                            cur_r = q.s0 as int32_t;
                                            if lig_stack > TEX_NULL as int32_t {
                                                (*mem.offset(lig_stack as isize)).b16.s0 =
                                                    cur_r as uint16_t;
                                            } else {
                                                lig_stack = new_lig_item(cur_r as uint16_t);
                                                if j as ::core::ffi::c_int
                                                    == n as ::core::ffi::c_int
                                                {
                                                    bchar = TOO_BIG_CHAR as int32_t;
                                                } else {
                                                    p = get_avail();
                                                    (*mem.offset(
                                                        (lig_stack + 1 as int32_t) as isize,
                                                    ))
                                                    .b32
                                                    .s1 = p;
                                                    (*mem.offset(p as isize)).b16.s0 = hu[(j
                                                        as ::core::ffi::c_int
                                                        + 1 as ::core::ffi::c_int)
                                                        as usize]
                                                        as uint16_t;
                                                    (*mem.offset(p as isize)).b16.s1 =
                                                        hf as uint16_t;
                                                }
                                            }
                                        }
                                        3 => {
                                            cur_r = q.s0 as int32_t;
                                            p = lig_stack;
                                            lig_stack = new_lig_item(cur_r as uint16_t);
                                            (*mem.offset(lig_stack as isize)).b32.s1 = p;
                                        }
                                        7 | 11 => {
                                            if ligature_present {
                                                p = new_ligature(
                                                    hf,
                                                    cur_l as uint16_t,
                                                    (*mem.offset(cur_q as isize)).b32.s1,
                                                );
                                                if lft_hit {
                                                    (*mem.offset(p as isize)).b16.s0 =
                                                        2 as uint16_t;
                                                    lft_hit = false_0 != 0;
                                                }
                                                (*mem.offset(cur_q as isize)).b32.s1 = p;
                                                t = p;
                                                ligature_present = false_0 != 0;
                                            }
                                            cur_q = t;
                                            cur_l = q.s0 as int32_t;
                                            ligature_present = true_0 != 0;
                                        }
                                        _ => {
                                            cur_l = q.s0 as int32_t;
                                            ligature_present = true_0 != 0;
                                            if lig_stack > TEX_NULL as int32_t {
                                                if (*mem
                                                    .offset((lig_stack + 1 as int32_t) as isize))
                                                .b32
                                                .s1 > TEX_NULL as int32_t
                                                {
                                                    (*mem.offset(t as isize)).b32.s1 = (*mem
                                                        .offset(
                                                            (lig_stack + 1 as int32_t) as isize,
                                                        ))
                                                    .b32
                                                    .s1;
                                                    t = (*mem.offset(t as isize)).b32.s1;
                                                    j += 1;
                                                }
                                                p = lig_stack;
                                                lig_stack = (*mem.offset(p as isize)).b32.s1;
                                                free_node(p, SMALL_NODE_SIZE as int32_t);
                                                if lig_stack == TEX_NULL as int32_t {
                                                    if (j as ::core::ffi::c_int)
                                                        < n as ::core::ffi::c_int
                                                    {
                                                        cur_r = hu[(j as ::core::ffi::c_int
                                                            + 1 as ::core::ffi::c_int)
                                                            as usize];
                                                    } else {
                                                        cur_r = bchar;
                                                    }
                                                    if hyf[j as usize] as ::core::ffi::c_int
                                                        & 1 as ::core::ffi::c_int
                                                        != 0
                                                    {
                                                        cur_rh = hchar;
                                                    } else {
                                                        cur_rh = TOO_BIG_CHAR as int32_t;
                                                    }
                                                } else {
                                                    cur_r = (*mem.offset(lig_stack as isize)).b16.s0
                                                        as int32_t;
                                                }
                                            } else {
                                                if j as ::core::ffi::c_int
                                                    == n as ::core::ffi::c_int
                                                {
                                                    break;
                                                }
                                                (*mem.offset(t as isize)).b32.s1 = get_avail();
                                                t = (*mem.offset(t as isize)).b32.s1;
                                                (*mem.offset(t as isize)).b16.s1 = hf as uint16_t;
                                                (*mem.offset(t as isize)).b16.s0 =
                                                    cur_r as uint16_t;
                                                j += 1;
                                                if (j as ::core::ffi::c_int)
                                                    < n as ::core::ffi::c_int
                                                {
                                                    cur_r = hu[(j as ::core::ffi::c_int
                                                        + 1 as ::core::ffi::c_int)
                                                        as usize];
                                                } else {
                                                    cur_r = bchar;
                                                }
                                                if hyf[j as usize] as ::core::ffi::c_int
                                                    & 1 as ::core::ffi::c_int
                                                    != 0
                                                {
                                                    cur_rh = hchar;
                                                } else {
                                                    cur_rh = TOO_BIG_CHAR as int32_t;
                                                }
                                            }
                                        }
                                    }
                                    if !(q.s1 as ::core::ffi::c_int > 4 as ::core::ffi::c_int) {
                                        continue '_continue_;
                                    }
                                    if q.s1 as ::core::ffi::c_int != 7 as ::core::ffi::c_int {
                                        break;
                                    } else {
                                        continue '_continue_;
                                    }
                                } else {
                                    w = (*font_info.offset(
                                        (*kern_base.offset(hf as isize)
                                            + 256 as int32_t * q.s1 as int32_t
                                            + q.s0 as int32_t)
                                            as isize,
                                    ))
                                    .b32
                                    .s1 as scaled_t;
                                    break;
                                }
                            }
                        }
                    }
                    if q.s3 as ::core::ffi::c_int >= 128 as ::core::ffi::c_int {
                        if cur_rh == TOO_BIG_CHAR as int32_t {
                            break;
                        }
                        cur_rh = TOO_BIG_CHAR as int32_t;
                        continue '_continue_;
                    } else {
                        k = k + q.s3 as font_index + 1 as font_index;
                        q = (*font_info.offset(k as isize)).b16;
                    }
                }
            }
            _ => {}
        }
        if ligature_present {
            p = new_ligature(hf, cur_l as uint16_t, (*mem.offset(cur_q as isize)).b32.s1);
            if lft_hit {
                (*mem.offset(p as isize)).b16.s0 = 2 as uint16_t;
                lft_hit = false_0 != 0;
            }
            if rt_hit {
                if lig_stack == TEX_NULL as int32_t {
                    let ref mut fresh15 = (*mem.offset(p as isize)).b16.s0;
                    *fresh15 = (*fresh15).wrapping_add(1);
                    rt_hit = false_0 != 0;
                }
            }
            (*mem.offset(cur_q as isize)).b32.s1 = p;
            t = p;
            ligature_present = false_0 != 0;
        }
        if w != 0 as scaled_t {
            (*mem.offset(t as isize)).b32.s1 = new_kern(w);
            t = (*mem.offset(t as isize)).b32.s1;
            w = 0 as ::core::ffi::c_int as scaled_t;
            (*mem.offset((t + 2 as int32_t) as isize)).b32.s0 = 0 as ::core::ffi::c_int as int32_t;
        }
        if !(lig_stack > TEX_NULL as int32_t) {
            break;
        }
        cur_q = t;
        cur_l = (*mem.offset(lig_stack as isize)).b16.s0 as int32_t;
        ligature_present = true_0 != 0;
        if (*mem.offset((lig_stack + 1 as int32_t) as isize)).b32.s1 > TEX_NULL as int32_t {
            (*mem.offset(t as isize)).b32.s1 =
                (*mem.offset((lig_stack + 1 as int32_t) as isize)).b32.s1;
            t = (*mem.offset(t as isize)).b32.s1;
            j += 1;
        }
        p = lig_stack;
        lig_stack = (*mem.offset(p as isize)).b32.s1;
        free_node(p, SMALL_NODE_SIZE as int32_t);
        if lig_stack == TEX_NULL as int32_t {
            if (j as ::core::ffi::c_int) < n as ::core::ffi::c_int {
                cur_r = hu[(j as ::core::ffi::c_int + 1 as ::core::ffi::c_int) as usize];
            } else {
                cur_r = bchar;
            }
            if hyf[j as usize] as ::core::ffi::c_int & 1 as ::core::ffi::c_int != 0 {
                cur_rh = hchar;
            } else {
                cur_rh = TOO_BIG_CHAR as int32_t;
            }
        } else {
            cur_r = (*mem.offset(lig_stack as isize)).b16.s0 as int32_t;
        }
    }
    return j;
}
unsafe extern "C" fn total_pw(mut q: int32_t, mut p: int32_t) -> scaled_t {
    let mut current_block: u64;
    let mut l: int32_t = 0;
    let mut r: int32_t = 0;
    let mut n: int32_t = 0;
    if (*mem.offset((q + 1 as int32_t) as isize)).b32.s1 == TEX_NULL as int32_t {
        l = first_p;
    } else {
        l = (*mem
            .offset(((*mem.offset((q + 1 as int32_t) as isize)).b32.s1 + 1 as int32_t) as isize))
        .b32
        .s1;
    }
    r = prev_rightmost(global_prev_p, p);
    if p != TEX_NULL as int32_t
        && (*mem.offset(p as isize)).b16.s1 as ::core::ffi::c_int == DISC_NODE
        && (*mem.offset((p + 1 as int32_t) as isize)).b32.s0 != TEX_NULL as int32_t
    {
        r = (*mem.offset((p + 1 as int32_t) as isize)).b32.s0;
        while (*mem.offset(r as isize)).b32.s1 != TEX_NULL as int32_t {
            r = (*mem.offset(r as isize)).b32.s1;
        }
    } else {
        r = find_protchar_right(l, r);
    }
    if l != TEX_NULL as int32_t
        && (*mem.offset(l as isize)).b16.s1 as ::core::ffi::c_int == DISC_NODE
    {
        if (*mem.offset((l + 1 as int32_t) as isize)).b32.s1 != TEX_NULL as int32_t {
            l = (*mem.offset((l + 1 as int32_t) as isize)).b32.s1;
            current_block = 14581806409783307637;
        } else {
            n = (*mem.offset(l as isize)).b16.s0 as int32_t;
            l = (*mem.offset(l as isize)).b32.s1;
            while n > 0 as int32_t {
                if (*mem.offset(l as isize)).b32.s1 != TEX_NULL as int32_t {
                    l = (*mem.offset(l as isize)).b32.s1;
                }
                n -= 1;
            }
            current_block = 5948590327928692120;
        }
    } else {
        current_block = 5948590327928692120;
    }
    match current_block {
        5948590327928692120 => {
            l = find_protchar_left(l, true_0 != 0);
        }
        _ => {}
    }
    return char_pw(l, 0 as small_number) + char_pw(r, 1 as small_number);
}
unsafe extern "C" fn find_protchar_left(mut l: int32_t, mut d: bool) -> int32_t {
    let mut t: int32_t = 0;
    let mut run: bool = false;
    if (*mem.offset(l as isize)).b32.s1 != TEX_NULL as int32_t
        && (*mem.offset(l as isize)).b16.s1 as ::core::ffi::c_int == HLIST_NODE
        && (*mem.offset((l + 1 as int32_t) as isize)).b32.s1 == 0 as int32_t
        && (*mem.offset((l + 3 as int32_t) as isize)).b32.s1 == 0 as int32_t
        && (*mem.offset((l + 2 as int32_t) as isize)).b32.s1 == 0 as int32_t
        && (*mem.offset((l + 5 as int32_t) as isize)).b32.s1 == TEX_NULL as int32_t
    {
        l = (*mem.offset(l as isize)).b32.s1;
    } else if d {
        while (*mem.offset(l as isize)).b32.s1 != TEX_NULL as int32_t
            && !(is_char_node(l) as ::core::ffi::c_int != 0
                || is_non_discardable_node(l) as ::core::ffi::c_int != 0)
        {
            l = (*mem.offset(l as isize)).b32.s1;
        }
    }
    hlist_stack_level = 0 as ::core::ffi::c_short;
    run = true_0 != 0;
    loop {
        t = l;
        while run as ::core::ffi::c_int != 0
            && (*mem.offset(l as isize)).b16.s1 as ::core::ffi::c_int == HLIST_NODE
            && (*mem.offset((l + 5 as int32_t) as isize)).b32.s1 != TEX_NULL as int32_t
        {
            push_node(l);
            l = (*mem.offset((l + 5 as int32_t) as isize)).b32.s1;
        }
        while run as ::core::ffi::c_int != 0
            && (!is_char_node(l)
                && ((*mem.offset(l as isize)).b16.s1 as ::core::ffi::c_int == INS_NODE
                    || (*mem.offset(l as isize)).b16.s1 as ::core::ffi::c_int == MARK_NODE
                    || (*mem.offset(l as isize)).b16.s1 as ::core::ffi::c_int == ADJUST_NODE
                    || (*mem.offset(l as isize)).b16.s1 as ::core::ffi::c_int == PENALTY_NODE
                    || (*mem.offset(l as isize)).b16.s1 as ::core::ffi::c_int == DISC_NODE
                        && (*mem.offset((l + 1 as int32_t) as isize)).b32.s0
                            == TEX_NULL as int32_t
                        && (*mem.offset((l + 1 as int32_t) as isize)).b32.s1
                            == TEX_NULL as int32_t
                        && (*mem.offset(l as isize)).b16.s0 as ::core::ffi::c_int
                            == 0 as ::core::ffi::c_int
                    || (*mem.offset(l as isize)).b16.s1 as ::core::ffi::c_int == MATH_NODE
                        && (*mem.offset((l + 1 as int32_t) as isize)).b32.s1 == 0 as int32_t
                    || (*mem.offset(l as isize)).b16.s1 as ::core::ffi::c_int == KERN_NODE
                        && ((*mem.offset((l + 1 as int32_t) as isize)).b32.s1 == 0 as int32_t
                            || (*mem.offset(l as isize)).b16.s0 as ::core::ffi::c_int == NORMAL)
                    || (*mem.offset(l as isize)).b16.s1 as ::core::ffi::c_int == GLUE_NODE
                        && (*mem.offset((l + 1 as int32_t) as isize)).b32.s0 == 0 as int32_t
                    || (*mem.offset(l as isize)).b16.s1 as ::core::ffi::c_int == HLIST_NODE
                        && (*mem.offset((l + 1 as int32_t) as isize)).b32.s1 == 0 as int32_t
                        && (*mem.offset((l + 3 as int32_t) as isize)).b32.s1 == 0 as int32_t
                        && (*mem.offset((l + 2 as int32_t) as isize)).b32.s1 == 0 as int32_t
                        && (*mem.offset((l + 5 as int32_t) as isize)).b32.s1
                            == TEX_NULL as int32_t))
        {
            while (*mem.offset(l as isize)).b32.s1 == TEX_NULL as int32_t
                && hlist_stack_level as ::core::ffi::c_int > 0 as ::core::ffi::c_int
            {
                l = pop_node();
            }
            if (*mem.offset(l as isize)).b32.s1 != TEX_NULL as int32_t {
                l = (*mem.offset(l as isize)).b32.s1;
            } else if hlist_stack_level as ::core::ffi::c_int == 0 as ::core::ffi::c_int {
                run = false_0 != 0;
            }
        }
        if t == l {
            break;
        }
    }
    return l;
}
unsafe extern "C" fn find_protchar_right(mut l: int32_t, mut r: int32_t) -> int32_t {
    let mut t: int32_t = 0;
    let mut run: bool = false;
    if r == TEX_NULL as int32_t {
        return TEX_NULL as int32_t;
    }
    hlist_stack_level = 0 as ::core::ffi::c_short;
    run = true_0 != 0;
    loop {
        t = r;
        while run as ::core::ffi::c_int != 0
            && (*mem.offset(r as isize)).b16.s1 as ::core::ffi::c_int == HLIST_NODE
            && (*mem.offset((r + 5 as int32_t) as isize)).b32.s1 != TEX_NULL as int32_t
        {
            push_node(l);
            push_node(r);
            l = (*mem.offset((r + 5 as int32_t) as isize)).b32.s1;
            r = l;
            while (*mem.offset(r as isize)).b32.s1 != TEX_NULL as int32_t {
                r = (*mem.offset(r as isize)).b32.s1;
            }
        }
        while run as ::core::ffi::c_int != 0
            && (!is_char_node(r)
                && ((*mem.offset(r as isize)).b16.s1 as ::core::ffi::c_int == INS_NODE
                    || (*mem.offset(r as isize)).b16.s1 as ::core::ffi::c_int == MARK_NODE
                    || (*mem.offset(r as isize)).b16.s1 as ::core::ffi::c_int == ADJUST_NODE
                    || (*mem.offset(r as isize)).b16.s1 as ::core::ffi::c_int == PENALTY_NODE
                    || (*mem.offset(r as isize)).b16.s1 as ::core::ffi::c_int == DISC_NODE
                        && (*mem.offset((r + 1 as int32_t) as isize)).b32.s0
                            == TEX_NULL as int32_t
                        && (*mem.offset((r + 1 as int32_t) as isize)).b32.s1
                            == TEX_NULL as int32_t
                        && (*mem.offset(r as isize)).b16.s0 as ::core::ffi::c_int
                            == 0 as ::core::ffi::c_int
                    || (*mem.offset(r as isize)).b16.s1 as ::core::ffi::c_int == MATH_NODE
                        && (*mem.offset((r + 1 as int32_t) as isize)).b32.s1 == 0 as int32_t
                    || (*mem.offset(r as isize)).b16.s1 as ::core::ffi::c_int == KERN_NODE
                        && ((*mem.offset((r + 1 as int32_t) as isize)).b32.s1 == 0 as int32_t
                            || (*mem.offset(r as isize)).b16.s0 as ::core::ffi::c_int == NORMAL)
                    || (*mem.offset(r as isize)).b16.s1 as ::core::ffi::c_int == GLUE_NODE
                        && (*mem.offset((r + 1 as int32_t) as isize)).b32.s0 == 0 as int32_t
                    || (*mem.offset(r as isize)).b16.s1 as ::core::ffi::c_int == HLIST_NODE
                        && (*mem.offset((r + 1 as int32_t) as isize)).b32.s1 == 0 as int32_t
                        && (*mem.offset((r + 3 as int32_t) as isize)).b32.s1 == 0 as int32_t
                        && (*mem.offset((r + 2 as int32_t) as isize)).b32.s1 == 0 as int32_t
                        && (*mem.offset((r + 5 as int32_t) as isize)).b32.s1
                            == TEX_NULL as int32_t))
        {
            while r == l && hlist_stack_level as ::core::ffi::c_int > 0 as ::core::ffi::c_int {
                r = pop_node();
                l = pop_node();
            }
            if r != l && r != TEX_NULL as int32_t {
                r = prev_rightmost(l, r);
            } else if r == l && hlist_stack_level as ::core::ffi::c_int == 0 as ::core::ffi::c_int {
                run = false_0 != 0;
            }
        }
        if t == r {
            break;
        }
    }
    return r;
}
unsafe extern "C" fn push_node(mut p: int32_t) {
    if hlist_stack_level as ::core::ffi::c_int > MAX_HLIST_STACK {
        pdf_error(
            b"push_node\0" as *const u8 as *const ::core::ffi::c_char,
            b"stack overflow\0" as *const u8 as *const ::core::ffi::c_char,
        );
    }
    hlist_stack[hlist_stack_level as usize] = p;
    hlist_stack_level =
        (hlist_stack_level as ::core::ffi::c_int + 1 as ::core::ffi::c_int) as ::core::ffi::c_short;
}
unsafe extern "C" fn pop_node() -> int32_t {
    hlist_stack_level =
        (hlist_stack_level as ::core::ffi::c_int - 1 as ::core::ffi::c_int) as ::core::ffi::c_short;
    if (hlist_stack_level as ::core::ffi::c_int) < 0 as ::core::ffi::c_int {
        pdf_error(
            b"pop_node\0" as *const u8 as *const ::core::ffi::c_char,
            b"stack underflow (internal error)\0" as *const u8 as *const ::core::ffi::c_char,
        );
    }
    return hlist_stack[hlist_stack_level as usize];
}
pub const true_0: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const false_0: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
