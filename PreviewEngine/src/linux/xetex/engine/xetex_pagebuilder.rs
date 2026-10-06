/* tectonic/xetex-pagebuilder.c: the page builder
   Copyright 2017-2018 The Tectonic Project
   Licensed under the MIT License.
*/
// Translated from xetex/engine/xetex-pagebuilder.c with C2Rust 0.22.1.
extern "C" {
    pub type ttbc_diagnostic_t;
    static mut eqtb: *mut memory_word;
    static mut help_line: [*const ::core::ffi::c_char; 6];
    static mut help_ptr: ::core::ffi::c_uchar;
    static mut temp_ptr: int32_t;
    static mut mem: *mut memory_word;
    static mut nest: *mut list_state_record;
    static mut nest_ptr: int32_t;
    static mut cur_list: list_state_record;
    static mut line: int32_t;
    static mut cur_mark: [int32_t; 5];
    static mut dead_cycles: int32_t;
    static mut best_height_plus_depth: scaled_t;
    static mut page_tail: int32_t;
    static mut page_contents: ::core::ffi::c_uchar;
    static mut page_so_far: [scaled_t; 8];
    static mut last_glue: int32_t;
    static mut last_penalty: int32_t;
    static mut last_kern: scaled_t;
    static mut last_node_type: int32_t;
    static mut insert_penalties: int32_t;
    static mut output_active: bool;
    static mut sa_root: [int32_t; 8];
    static mut cur_ptr: int32_t;
    static mut disc_ptr: [int32_t; 4];
    static mut semantic_pagination_enabled: bool;
    fn badness(t: scaled_t, s: scaled_t) -> int32_t;
    fn get_node(s: int32_t) -> int32_t;
    fn free_node(p: int32_t, s: int32_t);
    fn new_null_box() -> int32_t;
    fn new_spec(p: int32_t) -> int32_t;
    fn new_skip_param(n: small_number) -> int32_t;
    fn delete_token_ref(p: int32_t);
    fn delete_glue_ref(p: int32_t);
    fn flush_node_list(p: int32_t);
    fn push_nest();
    fn new_save_level(c: group_code);
    fn geq_word_define(p: int32_t, w: int32_t);
    fn begin_token_list(p: int32_t, t: uint16_t);
    fn find_sa_element(t: small_number, n: int32_t, w: bool);
    fn scan_left_brace();
    fn vpackage(p: int32_t, h: scaled_t, m: small_number, l: scaled_t) -> int32_t;
    fn prune_page_top(p: int32_t, s: bool) -> int32_t;
    fn vert_break(p: int32_t, h: scaled_t, d: scaled_t) -> int32_t;
    fn do_marks(a: small_number, l: small_number, q: int32_t) -> bool;
    fn box_error(n: eight_bits);
    fn normal_paragraph();
    fn error();
    fn confusion(s: *const ::core::ffi::c_char) -> !;
    fn capture_to_diagnostic(diagnostic: *mut ttbc_diagnostic_t);
    fn error_here_with_diagnostic(
        message: *const ::core::ffi::c_char,
    ) -> *mut ttbc_diagnostic_t;
    fn print_cstr(s: *const ::core::ffi::c_char);
    fn print_esc_cstr(s: *const ::core::ffi::c_char);
    fn print_int(n: int32_t);
    fn x_over_n(x: scaled_t, n: int32_t) -> scaled_t;
    fn ship_out(p: int32_t);
}
pub type __uint16_t = u16;
pub type __int32_t = i32;
pub type int32_t = __int32_t;
pub type uint16_t = __uint16_t;
pub type scaled_t = int32_t;
pub type eight_bits = ::core::ffi::c_uchar;
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
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<
    ::core::ffi::c_void,
>();
#[inline]
unsafe extern "C" fn is_non_discardable_node(p: int32_t) -> bool {
    return ((*mem.offset(p as isize)).b16.s1 as ::core::ffi::c_int) < MATH_NODE;
}
pub const MEM_TOP: ::core::ffi::c_int = 4999999 as ::core::ffi::c_int;
pub const INT_PAR__vbadness: ::core::ffi::c_int = 27 as ::core::ffi::c_int;
pub const INT_PAR__output_penalty: ::core::ffi::c_int = 39 as ::core::ffi::c_int;
pub const INT_PAR__max_dead_cycles: ::core::ffi::c_int = 40 as ::core::ffi::c_int;
pub const INT_PAR__holding_inserts: ::core::ffi::c_int = 53 as ::core::ffi::c_int;
pub const INT_PAR__saving_vdiscards: ::core::ffi::c_int = 63 as ::core::ffi::c_int;
pub const INT_PARS: ::core::ffi::c_int = 89 as ::core::ffi::c_int;
pub const DIMEN_PAR__vsize: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
pub const DIMEN_PAR__max_depth: ::core::ffi::c_int = 5 as ::core::ffi::c_int;
pub const DIMEN_PAR__vfuzz: ::core::ffi::c_int = 9 as ::core::ffi::c_int;
pub const DIMEN_PARS: ::core::ffi::c_int = 23 as ::core::ffi::c_int;
pub const GLUE_PAR__top_skip: ::core::ffi::c_int = 9 as ::core::ffi::c_int;
pub const GLUE_PAR__split_top_skip: ::core::ffi::c_int = 10 as ::core::ffi::c_int;
pub const LOCAL__output_routine: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const GLUE_BASE: ::core::ffi::c_int = 2254340 as ::core::ffi::c_int;
pub const SKIP_BASE: ::core::ffi::c_int = 2254359 as ::core::ffi::c_int;
pub const LOCAL_BASE: ::core::ffi::c_int = 2254871 as ::core::ffi::c_int;
pub const BOX_BASE: ::core::ffi::c_int = 2255144 as ::core::ffi::c_int;
pub const INT_BASE: ::core::ffi::c_int = 7826729 as ::core::ffi::c_int;
pub const COUNT_BASE: ::core::ffi::c_int = INT_BASE + INT_PARS;
pub const DEL_CODE_BASE: ::core::ffi::c_int = COUNT_BASE + 256 as ::core::ffi::c_int;
pub const DIMEN_BASE: ::core::ffi::c_int = DEL_CODE_BASE + NUMBER_USVS;
pub const SCALED_BASE: ::core::ffi::c_int = DIMEN_BASE + DIMEN_PARS;
pub const VMODE: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const HLIST_NODE: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const VLIST_NODE: ::core::ffi::c_int = 1;
pub const RULE_NODE: ::core::ffi::c_int = 2;
pub const INS_NODE: ::core::ffi::c_int = 3 as ::core::ffi::c_int;
pub const MARK_NODE: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
pub const WHATSIT_NODE: ::core::ffi::c_int = 8;
pub const MATH_NODE: ::core::ffi::c_int = 9 as ::core::ffi::c_int;
pub const GLUE_NODE: ::core::ffi::c_int = 10 as ::core::ffi::c_int;
pub const KERN_NODE: ::core::ffi::c_int = 11 as ::core::ffi::c_int;
pub const PENALTY_NODE: ::core::ffi::c_int = 12 as ::core::ffi::c_int;
pub const PIC_NODE: ::core::ffi::c_int = 43 as ::core::ffi::c_int;
pub const PDF_NODE: ::core::ffi::c_int = 44 as ::core::ffi::c_int;
pub const COPY_CODE: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const LAST_BOX_CODE: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const TOP_MARK_CODE: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const FIRST_MARK_CODE: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const BOT_MARK_CODE: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
static mut best_page_break: int32_t = 0;
static mut best_size: scaled_t = 0;
static mut least_page_cost: int32_t = 0;
static mut page_max_depth: scaled_t = 0;
#[no_mangle]
pub unsafe extern "C" fn initialize_pagebuilder_variables() {
    page_max_depth = 0 as ::core::ffi::c_int as scaled_t;
}
unsafe extern "C" fn freeze_page_specs(mut s: small_number) {
    page_contents = s as ::core::ffi::c_uchar;
    page_so_far[0 as ::core::ffi::c_int as usize] = (*eqtb
        .offset((DIMEN_BASE + DIMEN_PAR__vsize) as isize))
        .b32
        .s1 as scaled_t;
    page_max_depth = (*eqtb.offset((DIMEN_BASE + DIMEN_PAR__max_depth) as isize)).b32.s1
        as scaled_t;
    page_so_far[7 as ::core::ffi::c_int as usize] = 0 as ::core::ffi::c_int as scaled_t;
    page_so_far[1 as ::core::ffi::c_int as usize] = 0 as ::core::ffi::c_int as scaled_t;
    page_so_far[2 as ::core::ffi::c_int as usize] = 0 as ::core::ffi::c_int as scaled_t;
    page_so_far[3 as ::core::ffi::c_int as usize] = 0 as ::core::ffi::c_int as scaled_t;
    page_so_far[4 as ::core::ffi::c_int as usize] = 0 as ::core::ffi::c_int as scaled_t;
    page_so_far[5 as ::core::ffi::c_int as usize] = 0 as ::core::ffi::c_int as scaled_t;
    page_so_far[6 as ::core::ffi::c_int as usize] = 0 as ::core::ffi::c_int as scaled_t;
    least_page_cost = MAX_HALFWORD as int32_t;
}
unsafe extern "C" fn ensure_vbox(mut n: eight_bits) {
    let mut p: int32_t = (*eqtb.offset((BOX_BASE + n as ::core::ffi::c_int) as isize))
        .b32
        .s1;
    if p == TEX_NULL as int32_t {
        return;
    }
    if (*mem.offset(p as isize)).b16.s1 as ::core::ffi::c_int != HLIST_NODE {
        return;
    }
    error_here_with_diagnostic(
        b"Insertions can only be added to a vbox\0" as *const u8
            as *const ::core::ffi::c_char,
    );
    capture_to_diagnostic(::core::ptr::null_mut::<ttbc_diagnostic_t>());
    help_ptr = 3 as ::core::ffi::c_uchar;
    help_line[2 as ::core::ffi::c_int as usize] = b"Tut tut: You're trying to \\insert into a\0"
        as *const u8 as *const ::core::ffi::c_char;
    help_line[1 as ::core::ffi::c_int as usize] = b"\\box register that now contains an \\hbox.\0"
        as *const u8 as *const ::core::ffi::c_char;
    help_line[0 as ::core::ffi::c_int as usize] = b"Proceed, and I'll discard its present contents.\0"
        as *const u8 as *const ::core::ffi::c_char;
    box_error(n);
}
unsafe extern "C" fn fire_up(mut c: int32_t) {
    let mut p: int32_t = 0;
    let mut q: int32_t = 0;
    let mut r: int32_t = 0;
    let mut s: int32_t = 0;
    let mut prev_p: int32_t = 0;
    let mut n: ::core::ffi::c_uchar = 0;
    let mut wait: bool = false;
    let mut save_vbadness: int32_t = 0;
    let mut save_vfuzz: scaled_t = 0;
    let mut save_split_top_skip: int32_t = 0;
    let mut process_inserts: bool = false;
    if (*mem.offset(best_page_break as isize)).b16.s1 as ::core::ffi::c_int
        == PENALTY_NODE
    {
        geq_word_define(
            INT_BASE as int32_t + INT_PAR__output_penalty as int32_t,
            (*mem.offset((best_page_break + 1 as int32_t) as isize)).b32.s1,
        );
        (*mem.offset((best_page_break + 1 as int32_t) as isize)).b32.s1 = INF_PENALTY
            as int32_t;
    } else {
        geq_word_define(
            INT_BASE as int32_t + INT_PAR__output_penalty as int32_t,
            INF_PENALTY as int32_t,
        );
    }
    if sa_root[MARK_VAL as usize] != TEX_NULL as int32_t {
        if do_marks(
            FIRE_UP_INIT as small_number,
            0 as small_number,
            sa_root[MARK_VAL as usize],
        ) {
            sa_root[MARK_VAL as usize] = TEX_NULL as int32_t;
        }
    }
    if cur_mark[BOT_MARK_CODE as usize] != TEX_NULL as int32_t {
        if cur_mark[TOP_MARK_CODE as usize] != TEX_NULL as int32_t {
            delete_token_ref(cur_mark[TOP_MARK_CODE as usize]);
        }
        cur_mark[TOP_MARK_CODE as usize] = cur_mark[BOT_MARK_CODE as usize];
        let ref mut fresh1 = (*mem
            .offset(cur_mark[0 as ::core::ffi::c_int as usize] as isize))
            .b32
            .s0;
        *fresh1 += 1;
        delete_token_ref(cur_mark[FIRST_MARK_CODE as usize]);
        cur_mark[FIRST_MARK_CODE as usize] = TEX_NULL as int32_t;
    }
    if c == best_page_break {
        best_page_break = TEX_NULL as int32_t;
    }
    if (*eqtb.offset((BOX_BASE + 255 as ::core::ffi::c_int) as isize)).b32.s1
        != TEX_NULL as int32_t
    {
        error_here_with_diagnostic(b"\0" as *const u8 as *const ::core::ffi::c_char);
        print_esc_cstr(b"box\0" as *const u8 as *const ::core::ffi::c_char);
        print_cstr(b"255 is not void\0" as *const u8 as *const ::core::ffi::c_char);
        capture_to_diagnostic(::core::ptr::null_mut::<ttbc_diagnostic_t>());
        help_ptr = 2 as ::core::ffi::c_uchar;
        help_line[1 as ::core::ffi::c_int as usize] = b"You shouldn't use \\box255 except in \\output routines.\0"
            as *const u8 as *const ::core::ffi::c_char;
        help_line[0 as ::core::ffi::c_int as usize] = b"Proceed, and I'll discard its present contents.\0"
            as *const u8 as *const ::core::ffi::c_char;
        box_error(255 as eight_bits);
    }
    insert_penalties = 0 as ::core::ffi::c_int as int32_t;
    save_split_top_skip = (*eqtb.offset((GLUE_BASE + GLUE_PAR__split_top_skip) as isize))
        .b32
        .s1;
    process_inserts = (*eqtb.offset((INT_BASE + INT_PAR__holding_inserts) as isize))
        .b32
        .s1 <= 0 as int32_t && !semantic_pagination_enabled;
    if process_inserts {
        r = (*mem.offset(4999999 as ::core::ffi::c_int as isize)).b32.s1;
        while r != PAGE_INS_HEAD as int32_t {
            if (*mem.offset((r + 2 as int32_t) as isize)).b32.s0 != TEX_NULL as int32_t {
                n = (*mem.offset(r as isize)).b16.s0 as ::core::ffi::c_uchar;
                ensure_vbox(n as eight_bits);
                if (*eqtb.offset((BOX_BASE + n as ::core::ffi::c_int) as isize)).b32.s1
                    == TEX_NULL as int32_t
                {
                    (*eqtb.offset((BOX_BASE + n as ::core::ffi::c_int) as isize))
                        .b32
                        .s1 = new_null_box();
                }
                p = (*eqtb.offset((BOX_BASE + n as ::core::ffi::c_int) as isize)).b32.s1
                    + 5 as int32_t;
                while (*mem.offset(p as isize)).b32.s1 != TEX_NULL as int32_t {
                    p = (*mem.offset(p as isize)).b32.s1;
                }
                (*mem.offset((r + 2 as int32_t) as isize)).b32.s1 = p;
            }
            r = (*mem.offset(r as isize)).b32.s1;
        }
    }
    q = HOLD_HEAD as int32_t;
    (*mem.offset(q as isize)).b32.s1 = TEX_NULL as int32_t;
    prev_p = PAGE_HEAD as int32_t;
    p = (*mem.offset(prev_p as isize)).b32.s1;
    while p != best_page_break {
        if (*mem.offset(p as isize)).b16.s1 as ::core::ffi::c_int == INS_NODE {
            if process_inserts {
                r = (*mem.offset(4999999 as ::core::ffi::c_int as isize)).b32.s1;
                while (*mem.offset(r as isize)).b16.s0 as ::core::ffi::c_int
                    != (*mem.offset(p as isize)).b16.s0 as ::core::ffi::c_int
                {
                    r = (*mem.offset(r as isize)).b32.s1;
                }
                if (*mem.offset((r + 2 as int32_t) as isize)).b32.s0
                    == TEX_NULL as int32_t
                {
                    wait = true_0 != 0;
                } else {
                    wait = false_0 != 0;
                    s = (*mem.offset((r + 2 as int32_t) as isize)).b32.s1;
                    (*mem.offset(s as isize)).b32.s1 = (*mem
                        .offset((p + 4 as int32_t) as isize))
                        .b32
                        .s0;
                    if (*mem.offset((r + 2 as int32_t) as isize)).b32.s0 == p {
                        if (*mem.offset(r as isize)).b16.s1 as ::core::ffi::c_int
                            == SPLIT_UP
                        {
                            if (*mem.offset((r + 1 as int32_t) as isize)).b32.s0 == p
                                && (*mem.offset((r + 1 as int32_t) as isize)).b32.s1
                                    != TEX_NULL as int32_t
                            {
                                while (*mem.offset(s as isize)).b32.s1
                                    != (*mem.offset((r + 1 as int32_t) as isize)).b32.s1
                                {
                                    s = (*mem.offset(s as isize)).b32.s1;
                                }
                                (*mem.offset(s as isize)).b32.s1 = TEX_NULL as int32_t;
                                (*eqtb
                                    .offset((GLUE_BASE + GLUE_PAR__split_top_skip) as isize))
                                    .b32
                                    .s1 = (*mem.offset((p + 4 as int32_t) as isize)).b32.s1;
                                (*mem.offset((p + 4 as int32_t) as isize)).b32.s0 = prune_page_top(
                                    (*mem.offset((r + 1 as int32_t) as isize)).b32.s1,
                                    false_0 != 0,
                                );
                                if (*mem.offset((p + 4 as int32_t) as isize)).b32.s0
                                    != TEX_NULL as int32_t
                                {
                                    temp_ptr = vpackage(
                                        (*mem.offset((p + 4 as int32_t) as isize)).b32.s0,
                                        0 as scaled_t,
                                        ADDITIONAL as small_number,
                                        MAX_HALFWORD as scaled_t,
                                    );
                                    (*mem.offset((p + 3 as int32_t) as isize)).b32.s1 = (*mem
                                        .offset((temp_ptr + 3 as int32_t) as isize))
                                        .b32
                                        .s1
                                        + (*mem.offset((temp_ptr + 2 as int32_t) as isize)).b32.s1;
                                    free_node(temp_ptr, BOX_NODE_SIZE as int32_t);
                                    wait = true_0 != 0;
                                }
                            }
                        }
                        (*mem.offset((r + 2 as int32_t) as isize)).b32.s0 = TEX_NULL
                            as int32_t;
                        n = (*mem.offset(r as isize)).b16.s0 as ::core::ffi::c_uchar;
                        temp_ptr = (*mem
                            .offset(
                                ((*eqtb
                                    .offset(
                                        (2255144 as ::core::ffi::c_int + n as ::core::ffi::c_int)
                                            as isize,
                                    ))
                                    .b32
                                    .s1 + 5 as int32_t) as isize,
                            ))
                            .b32
                            .s1;
                        free_node(
                            (*eqtb.offset((BOX_BASE + n as ::core::ffi::c_int) as isize))
                                .b32
                                .s1,
                            BOX_NODE_SIZE as int32_t,
                        );
                        (*eqtb.offset((BOX_BASE + n as ::core::ffi::c_int) as isize))
                            .b32
                            .s1 = vpackage(
                            temp_ptr,
                            0 as scaled_t,
                            ADDITIONAL as small_number,
                            MAX_HALFWORD as scaled_t,
                        );
                    } else {
                        while (*mem.offset(s as isize)).b32.s1 != TEX_NULL as int32_t {
                            s = (*mem.offset(s as isize)).b32.s1;
                        }
                        (*mem.offset((r + 2 as int32_t) as isize)).b32.s1 = s;
                    }
                }
                (*mem.offset(prev_p as isize)).b32.s1 = (*mem.offset(p as isize)).b32.s1;
                (*mem.offset(p as isize)).b32.s1 = TEX_NULL as int32_t;
                if wait {
                    (*mem.offset(q as isize)).b32.s1 = p;
                    q = p;
                    insert_penalties += 1;
                } else {
                    delete_glue_ref((*mem.offset((p + 4 as int32_t) as isize)).b32.s1);
                    free_node(p, INS_NODE_SIZE as int32_t);
                }
                p = prev_p;
            }
        } else if (*mem.offset(p as isize)).b16.s1 as ::core::ffi::c_int == MARK_NODE {
            if (*mem.offset((p + 1 as int32_t) as isize)).b32.s0 != 0 as int32_t {
                find_sa_element(
                    MARK_VAL as small_number,
                    (*mem.offset((p + 1 as int32_t) as isize)).b32.s0,
                    true_0 != 0,
                );
                if (*mem.offset((cur_ptr + 1 as int32_t) as isize)).b32.s1
                    == TEX_NULL as int32_t
                {
                    (*mem.offset((cur_ptr + 1 as int32_t) as isize)).b32.s1 = (*mem
                        .offset((p + 1 as int32_t) as isize))
                        .b32
                        .s1;
                    let ref mut fresh2 = (*mem
                        .offset(
                            (*mem.offset((p + 1 as int32_t) as isize)).b32.s1 as isize,
                        ))
                        .b32
                        .s0;
                    *fresh2 += 1;
                }
                if (*mem.offset((cur_ptr + 2 as int32_t) as isize)).b32.s0
                    != TEX_NULL as int32_t
                {
                    delete_token_ref(
                        (*mem.offset((cur_ptr + 2 as int32_t) as isize)).b32.s0,
                    );
                }
                (*mem.offset((cur_ptr + 2 as int32_t) as isize)).b32.s0 = (*mem
                    .offset((p + 1 as int32_t) as isize))
                    .b32
                    .s1;
                let ref mut fresh3 = (*mem
                    .offset((*mem.offset((p + 1 as int32_t) as isize)).b32.s1 as isize))
                    .b32
                    .s0;
                *fresh3 += 1;
            } else {
                if cur_mark[FIRST_MARK_CODE as usize] == TEX_NULL as int32_t {
                    cur_mark[FIRST_MARK_CODE as usize] = (*mem
                        .offset((p + 1 as int32_t) as isize))
                        .b32
                        .s1;
                    let ref mut fresh4 = (*mem
                        .offset(cur_mark[1 as ::core::ffi::c_int as usize] as isize))
                        .b32
                        .s0;
                    *fresh4 += 1;
                }
                if cur_mark[BOT_MARK_CODE as usize] != TEX_NULL as int32_t {
                    delete_token_ref(cur_mark[BOT_MARK_CODE as usize]);
                }
                cur_mark[BOT_MARK_CODE as usize] = (*mem
                    .offset((p + 1 as int32_t) as isize))
                    .b32
                    .s1;
                let ref mut fresh5 = (*mem
                    .offset(cur_mark[2 as ::core::ffi::c_int as usize] as isize))
                    .b32
                    .s0;
                *fresh5 += 1;
            }
        }
        prev_p = p;
        p = (*mem.offset(prev_p as isize)).b32.s1;
    }
    (*eqtb.offset((GLUE_BASE + GLUE_PAR__split_top_skip) as isize)).b32.s1 = save_split_top_skip;
    if p != TEX_NULL as int32_t {
        if (*mem
            .offset((4999999 as ::core::ffi::c_int - 1 as ::core::ffi::c_int) as isize))
            .b32
            .s1 == TEX_NULL as int32_t
        {
            if nest_ptr == 0 as int32_t {
                cur_list.tail = page_tail;
            } else {
                (*nest.offset(0 as ::core::ffi::c_int as isize)).tail = page_tail;
            }
        }
        (*mem.offset(page_tail as isize)).b32.s1 = (*mem
            .offset((4999999 as ::core::ffi::c_int - 1 as ::core::ffi::c_int) as isize))
            .b32
            .s1;
        (*mem.offset((4999999 as ::core::ffi::c_int - 1 as ::core::ffi::c_int) as isize))
            .b32
            .s1 = p;
        (*mem.offset(prev_p as isize)).b32.s1 = TEX_NULL as int32_t;
    }
    save_vbadness = (*eqtb.offset((INT_BASE + INT_PAR__vbadness) as isize)).b32.s1;
    (*eqtb.offset((INT_BASE + INT_PAR__vbadness) as isize)).b32.s1 = INF_BAD as int32_t;
    save_vfuzz = (*eqtb.offset((DIMEN_BASE + DIMEN_PAR__vfuzz) as isize)).b32.s1
        as scaled_t;
    (*eqtb.offset((DIMEN_BASE + DIMEN_PAR__vfuzz) as isize)).b32.s1 = MAX_HALFWORD
        as int32_t;
    (*eqtb.offset((BOX_BASE + 255 as ::core::ffi::c_int) as isize)).b32.s1 = vpackage(
        (*mem.offset((4999999 as ::core::ffi::c_int - 2 as ::core::ffi::c_int) as isize))
            .b32
            .s1,
        best_size,
        EXACTLY as small_number,
        page_max_depth,
    );
    (*eqtb.offset((INT_BASE + INT_PAR__vbadness) as isize)).b32.s1 = save_vbadness;
    (*eqtb.offset((DIMEN_BASE + DIMEN_PAR__vfuzz) as isize)).b32.s1 = save_vfuzz
        as int32_t;
    if last_glue != MAX_HALFWORD as int32_t {
        delete_glue_ref(last_glue);
    }
    page_contents = EMPTY as ::core::ffi::c_uchar;
    page_tail = PAGE_HEAD as int32_t;
    (*mem.offset((4999999 as ::core::ffi::c_int - 2 as ::core::ffi::c_int) as isize))
        .b32
        .s1 = TEX_NULL as int32_t;
    last_glue = MAX_HALFWORD as int32_t;
    last_penalty = 0 as ::core::ffi::c_int as int32_t;
    last_kern = 0 as ::core::ffi::c_int as scaled_t;
    last_node_type = -(1 as ::core::ffi::c_int) as int32_t;
    page_so_far[7 as ::core::ffi::c_int as usize] = 0 as ::core::ffi::c_int as scaled_t;
    page_max_depth = 0 as ::core::ffi::c_int as scaled_t;
    if q != HOLD_HEAD as int32_t {
        (*mem.offset((4999999 as ::core::ffi::c_int - 2 as ::core::ffi::c_int) as isize))
            .b32
            .s1 = (*mem
            .offset((4999999 as ::core::ffi::c_int - 4 as ::core::ffi::c_int) as isize))
            .b32
            .s1;
        page_tail = q;
    }
    r = (*mem.offset(4999999 as ::core::ffi::c_int as isize)).b32.s1;
    while r != PAGE_INS_HEAD as int32_t {
        q = (*mem.offset(r as isize)).b32.s1;
        free_node(r, PAGE_INS_NODE_SIZE as int32_t);
        r = q;
    }
    (*mem.offset(4999999 as ::core::ffi::c_int as isize)).b32.s1 = PAGE_INS_HEAD
        as int32_t;
    if sa_root[MARK_VAL as usize] != TEX_NULL as int32_t {
        if do_marks(
            FIRE_UP_DONE as small_number,
            0 as small_number,
            sa_root[MARK_VAL as usize],
        ) {
            sa_root[MARK_VAL as usize] = TEX_NULL as int32_t;
        }
    }
    if cur_mark[TOP_MARK_CODE as usize] != TEX_NULL as int32_t
        && cur_mark[FIRST_MARK_CODE as usize] == TEX_NULL as int32_t
    {
        cur_mark[FIRST_MARK_CODE as usize] = cur_mark[TOP_MARK_CODE as usize];
        let ref mut fresh6 = (*mem
            .offset(cur_mark[0 as ::core::ffi::c_int as usize] as isize))
            .b32
            .s0;
        *fresh6 += 1;
    }
    if (*eqtb.offset((LOCAL_BASE + LOCAL__output_routine) as isize)).b32.s1
        != TEX_NULL as int32_t
    {
        if dead_cycles
            >= (*eqtb.offset((INT_BASE + INT_PAR__max_dead_cycles) as isize)).b32.s1
        {
            error_here_with_diagnostic(
                b"Output loop---\0" as *const u8 as *const ::core::ffi::c_char,
            );
            print_int(dead_cycles);
            print_cstr(
                b" consecutive dead cycles\0" as *const u8 as *const ::core::ffi::c_char,
            );
            capture_to_diagnostic(::core::ptr::null_mut::<ttbc_diagnostic_t>());
            help_ptr = 3 as ::core::ffi::c_uchar;
            help_line[2 as ::core::ffi::c_int as usize] = b"I've concluded that your \\output is awry; it never does a\0"
                as *const u8 as *const ::core::ffi::c_char;
            help_line[1 as ::core::ffi::c_int as usize] = b"\\shipout, so I'm shipping \\box255 out myself. Next time\0"
                as *const u8 as *const ::core::ffi::c_char;
            help_line[0 as ::core::ffi::c_int as usize] = b"increase \\maxdeadcycles if you want me to be more patient!\0"
                as *const u8 as *const ::core::ffi::c_char;
            error();
        } else {
            output_active = true_0 != 0;
            dead_cycles += 1;
            push_nest();
            cur_list.mode = -VMODE as ::core::ffi::c_short;
            cur_list.aux.b32.s1 = IGNORE_DEPTH as int32_t;
            cur_list.mode_line = -line;
            begin_token_list(
                (*eqtb.offset((LOCAL_BASE + LOCAL__output_routine) as isize)).b32.s1,
                OUTPUT_TEXT as uint16_t,
            );
            new_save_level(OUTPUT_GROUP as group_code);
            normal_paragraph();
            scan_left_brace();
            return;
        }
    }
    if (*mem.offset((4999999 as ::core::ffi::c_int - 2 as ::core::ffi::c_int) as isize))
        .b32
        .s1 != TEX_NULL as int32_t
    {
        if (*mem
            .offset((4999999 as ::core::ffi::c_int - 1 as ::core::ffi::c_int) as isize))
            .b32
            .s1 == TEX_NULL as int32_t
        {
            if nest_ptr == 0 as int32_t {
                cur_list.tail = page_tail;
            } else {
                (*nest.offset(0 as ::core::ffi::c_int as isize)).tail = page_tail;
            }
        } else {
            (*mem.offset(page_tail as isize)).b32.s1 = (*mem
                .offset(
                    (4999999 as ::core::ffi::c_int - 1 as ::core::ffi::c_int) as isize,
                ))
                .b32
                .s1;
        }
        (*mem.offset((4999999 as ::core::ffi::c_int - 1 as ::core::ffi::c_int) as isize))
            .b32
            .s1 = (*mem
            .offset((4999999 as ::core::ffi::c_int - 2 as ::core::ffi::c_int) as isize))
            .b32
            .s1;
        (*mem.offset((4999999 as ::core::ffi::c_int - 2 as ::core::ffi::c_int) as isize))
            .b32
            .s1 = TEX_NULL as int32_t;
        page_tail = PAGE_HEAD as int32_t;
    }
    flush_node_list(disc_ptr[LAST_BOX_CODE as usize]);
    disc_ptr[LAST_BOX_CODE as usize] = TEX_NULL as int32_t;
    ship_out((*eqtb.offset((BOX_BASE + 255 as ::core::ffi::c_int) as isize)).b32.s1);
    (*eqtb.offset((BOX_BASE + 255 as ::core::ffi::c_int) as isize)).b32.s1 = TEX_NULL
        as int32_t;
}
pub const AWFUL_BAD: ::core::ffi::c_int = MAX_HALFWORD;
#[no_mangle]
pub unsafe extern "C" fn build_page() {
    let mut current_block: u64;
    let mut p: int32_t = 0;
    let mut q: int32_t = 0;
    let mut r: int32_t = 0;
    let mut b: int32_t = 0;
    let mut c: int32_t = 0;
    let mut pi: int32_t = 0;
    let mut n: ::core::ffi::c_uchar = 0;
    let mut delta: scaled_t = 0;
    let mut h: scaled_t = 0;
    let mut w: scaled_t = 0;
    if (*mem.offset((4999999 as ::core::ffi::c_int - 1 as ::core::ffi::c_int) as isize))
        .b32
        .s1 == TEX_NULL as int32_t || output_active as ::core::ffi::c_int != 0
    {
        return;
    }
    loop {
        p = (*mem
            .offset((4999999 as ::core::ffi::c_int - 1 as ::core::ffi::c_int) as isize))
            .b32
            .s1;
        if last_glue != MAX_HALFWORD as int32_t {
            delete_glue_ref(last_glue);
        }
        last_penalty = 0 as ::core::ffi::c_int as int32_t;
        last_kern = 0 as ::core::ffi::c_int as scaled_t;
        last_node_type = ((*mem.offset(p as isize)).b16.s1 as ::core::ffi::c_int
            + 1 as ::core::ffi::c_int) as int32_t;
        if (*mem.offset(p as isize)).b16.s1 as ::core::ffi::c_int == GLUE_NODE {
            last_glue = (*mem.offset((p + 1 as int32_t) as isize)).b32.s0;
            let ref mut fresh0 = (*mem.offset(last_glue as isize)).b32.s1;
            *fresh0 += 1;
        } else {
            last_glue = MAX_HALFWORD as int32_t;
            if (*mem.offset(p as isize)).b16.s1 as ::core::ffi::c_int == PENALTY_NODE {
                last_penalty = (*mem.offset((p + 1 as int32_t) as isize)).b32.s1;
            } else if (*mem.offset(p as isize)).b16.s1 as ::core::ffi::c_int == KERN_NODE
            {
                last_kern = (*mem.offset((p + 1 as int32_t) as isize)).b32.s1
                    as scaled_t;
            }
        }
        match (*mem.offset(p as isize)).b16.s1 as ::core::ffi::c_int {
            HLIST_NODE | VLIST_NODE | RULE_NODE => {
                if (page_contents as ::core::ffi::c_int) < BOX_THERE {
                    if page_contents as ::core::ffi::c_int == EMPTY {
                        freeze_page_specs(BOX_THERE as small_number);
                    } else {
                        page_contents = BOX_THERE as ::core::ffi::c_uchar;
                    }
                    q = new_skip_param(GLUE_PAR__top_skip as small_number);
                    if (*mem.offset((temp_ptr + 1 as int32_t) as isize)).b32.s1
                        > (*mem.offset((p + 3 as int32_t) as isize)).b32.s1
                    {
                        (*mem.offset((temp_ptr + 1 as int32_t) as isize)).b32.s1
                            -= (*mem.offset((p + 3 as int32_t) as isize)).b32.s1;
                    } else {
                        (*mem.offset((temp_ptr + 1 as int32_t) as isize)).b32.s1 = 0
                            as ::core::ffi::c_int as int32_t;
                    }
                    (*mem.offset(q as isize)).b32.s1 = p;
                    (*mem
                        .offset(
                            (4999999 as ::core::ffi::c_int - 1 as ::core::ffi::c_int)
                                as isize,
                        ))
                        .b32
                        .s1 = q;
                    current_block = 17778012151635330486;
                } else {
                    page_so_far[1 as ::core::ffi::c_int as usize]
                        += (page_so_far[7 as ::core::ffi::c_int as usize]
                            + (*mem.offset((p + 3 as int32_t) as isize)).b32.s1
                                as scaled_t) as int32_t;
                    page_so_far[7 as ::core::ffi::c_int as usize] = (*mem
                        .offset((p + 2 as int32_t) as isize))
                        .b32
                        .s1 as scaled_t;
                    current_block = 371458106509226653;
                }
            }
            WHATSIT_NODE => {
                if (*mem.offset(p as isize)).b16.s0 as ::core::ffi::c_int == PIC_NODE
                    || (*mem.offset(p as isize)).b16.s0 as ::core::ffi::c_int == PDF_NODE
                {
                    page_so_far[1 as ::core::ffi::c_int as usize]
                        += (page_so_far[7 as ::core::ffi::c_int as usize]
                            + (*mem.offset((p + 3 as int32_t) as isize)).b32.s1
                                as scaled_t) as int32_t;
                    page_so_far[7 as ::core::ffi::c_int as usize] = (*mem
                        .offset((p + 2 as int32_t) as isize))
                        .b32
                        .s1 as scaled_t;
                }
                current_block = 371458106509226653;
            }
            GLUE_NODE => {
                if (page_contents as ::core::ffi::c_int) < BOX_THERE {
                    current_block = 14783794473175570661;
                } else if is_non_discardable_node(page_tail) {
                    pi = 0 as ::core::ffi::c_int as int32_t;
                    current_block = 1417769144978639029;
                } else {
                    current_block = 1469412417798502801;
                }
            }
            KERN_NODE => {
                if (page_contents as ::core::ffi::c_int) < BOX_THERE {
                    current_block = 14783794473175570661;
                } else if (*mem.offset(p as isize)).b32.s1 == TEX_NULL as int32_t {
                    return
                } else if (*mem.offset((*mem.offset(p as isize)).b32.s1 as isize)).b16.s1
                    as ::core::ffi::c_int == GLUE_NODE
                {
                    pi = 0 as ::core::ffi::c_int as int32_t;
                    current_block = 1417769144978639029;
                } else {
                    current_block = 1469412417798502801;
                }
            }
            PENALTY_NODE => {
                if (page_contents as ::core::ffi::c_int) < BOX_THERE {
                    current_block = 14783794473175570661;
                } else {
                    pi = (*mem.offset((p + 1 as int32_t) as isize)).b32.s1;
                    current_block = 1417769144978639029;
                }
            }
            MARK_NODE => {
                current_block = 371458106509226653;
            }
            INS_NODE => {
                if page_contents as ::core::ffi::c_int == EMPTY {
                    freeze_page_specs(INSERTS_ONLY as small_number);
                }
                n = (*mem.offset(p as isize)).b16.s0 as ::core::ffi::c_uchar;
                r = PAGE_INS_HEAD as int32_t;
                while n as ::core::ffi::c_int
                    >= (*mem.offset((*mem.offset(r as isize)).b32.s1 as isize)).b16.s0
                        as ::core::ffi::c_int
                {
                    r = (*mem.offset(r as isize)).b32.s1;
                }
                if (*mem.offset(r as isize)).b16.s0 as ::core::ffi::c_int
                    != n as ::core::ffi::c_int
                {
                    q = get_node(PAGE_INS_NODE_SIZE as int32_t);
                    (*mem.offset(q as isize)).b32.s1 = (*mem.offset(r as isize)).b32.s1;
                    (*mem.offset(r as isize)).b32.s1 = q;
                    r = q;
                    (*mem.offset(r as isize)).b16.s0 = n as uint16_t;
                    (*mem.offset(r as isize)).b16.s1 = INSERTING as uint16_t;
                    ensure_vbox(n as eight_bits);
                    if (*eqtb.offset((BOX_BASE + n as ::core::ffi::c_int) as isize))
                        .b32
                        .s1 == TEX_NULL as int32_t
                    {
                        (*mem.offset((r + 3 as int32_t) as isize)).b32.s1 = 0
                            as ::core::ffi::c_int as int32_t;
                    } else {
                        (*mem.offset((r + 3 as int32_t) as isize)).b32.s1 = (*mem
                            .offset(
                                ((*eqtb
                                    .offset(
                                        (2255144 as ::core::ffi::c_int + n as ::core::ffi::c_int)
                                            as isize,
                                    ))
                                    .b32
                                    .s1 + 3 as int32_t) as isize,
                            ))
                            .b32
                            .s1
                            + (*mem
                                .offset(
                                    ((*eqtb
                                        .offset(
                                            (2255144 as ::core::ffi::c_int + n as ::core::ffi::c_int)
                                                as isize,
                                        ))
                                        .b32
                                        .s1 + 2 as int32_t) as isize,
                                ))
                                .b32
                                .s1;
                    }
                    (*mem.offset((r + 2 as int32_t) as isize)).b32.s0 = TEX_NULL
                        as int32_t;
                    q = (*eqtb.offset((SKIP_BASE + n as ::core::ffi::c_int) as isize))
                        .b32
                        .s1;
                    if (*eqtb.offset((COUNT_BASE + n as ::core::ffi::c_int) as isize))
                        .b32
                        .s1 == 1000 as int32_t
                    {
                        h = (*mem.offset((r + 3 as int32_t) as isize)).b32.s1
                            as scaled_t;
                    } else {
                        h = x_over_n(
                            (*mem.offset((r + 3 as int32_t) as isize)).b32.s1
                                as scaled_t,
                            1000 as int32_t,
                        )
                            * (*eqtb
                                .offset((COUNT_BASE + n as ::core::ffi::c_int) as isize))
                                .b32
                                .s1 as scaled_t;
                    }
                    page_so_far[0 as ::core::ffi::c_int as usize]
                        -= (h
                            + (*mem.offset((q + 1 as int32_t) as isize)).b32.s1
                                as scaled_t) as int32_t;
                    page_so_far[(2 as ::core::ffi::c_int
                        + (*mem.offset(q as isize)).b16.s1 as ::core::ffi::c_int)
                        as usize] += (*mem.offset((q + 2 as int32_t) as isize)).b32.s1;
                    page_so_far[6 as ::core::ffi::c_int as usize]
                        += (*mem.offset((q + 3 as int32_t) as isize)).b32.s1;
                    if (*mem.offset(q as isize)).b16.s0 as ::core::ffi::c_int != NORMAL
                        && (*mem.offset((q + 3 as int32_t) as isize)).b32.s1
                            != 0 as int32_t
                    {
                        error_here_with_diagnostic(
                            b"Infinite glue shrinkage inserted from \0" as *const u8
                                as *const ::core::ffi::c_char,
                        );
                        print_esc_cstr(
                            b"skip\0" as *const u8 as *const ::core::ffi::c_char,
                        );
                        print_int(n as int32_t);
                        capture_to_diagnostic(
                            ::core::ptr::null_mut::<ttbc_diagnostic_t>(),
                        );
                        help_ptr = 3 as ::core::ffi::c_uchar;
                        help_line[2 as ::core::ffi::c_int as usize] = b"The correction glue for page breaking with insertions\0"
                            as *const u8 as *const ::core::ffi::c_char;
                        help_line[1 as ::core::ffi::c_int as usize] = b"must have finite shrinkability. But you may proceed,\0"
                            as *const u8 as *const ::core::ffi::c_char;
                        help_line[0 as ::core::ffi::c_int as usize] = b"since the offensive shrinkability has been made finite.\0"
                            as *const u8 as *const ::core::ffi::c_char;
                        error();
                    }
                }
                if (*mem.offset(r as isize)).b16.s1 as ::core::ffi::c_int == SPLIT_UP {
                    insert_penalties
                        += (*mem.offset((p + 1 as int32_t) as isize)).b32.s1;
                } else {
                    (*mem.offset((r + 2 as int32_t) as isize)).b32.s1 = p;
                    delta = page_so_far[0 as ::core::ffi::c_int as usize]
                        - page_so_far[1 as ::core::ffi::c_int as usize]
                        - page_so_far[7 as ::core::ffi::c_int as usize]
                        + page_so_far[6 as ::core::ffi::c_int as usize];
                    if (*eqtb.offset((COUNT_BASE + n as ::core::ffi::c_int) as isize))
                        .b32
                        .s1 == 1000 as int32_t
                    {
                        h = (*mem.offset((p + 3 as int32_t) as isize)).b32.s1
                            as scaled_t;
                    } else {
                        h = x_over_n(
                            (*mem.offset((p + 3 as int32_t) as isize)).b32.s1
                                as scaled_t,
                            1000 as int32_t,
                        )
                            * (*eqtb
                                .offset((COUNT_BASE + n as ::core::ffi::c_int) as isize))
                                .b32
                                .s1 as scaled_t;
                    }
                    if (h <= 0 as scaled_t || h <= delta)
                        && (*mem.offset((p + 3 as int32_t) as isize)).b32.s1
                            + (*mem.offset((r + 3 as int32_t) as isize)).b32.s1
                            <= (*eqtb
                                .offset((SCALED_BASE + n as ::core::ffi::c_int) as isize))
                                .b32
                                .s1
                    {
                        page_so_far[0 as ::core::ffi::c_int as usize] -= h;
                        (*mem.offset((r + 3 as int32_t) as isize)).b32.s1
                            += (*mem.offset((p + 3 as int32_t) as isize)).b32.s1;
                    } else {
                        if (*eqtb
                            .offset((COUNT_BASE + n as ::core::ffi::c_int) as isize))
                            .b32
                            .s1 <= 0 as int32_t
                        {
                            w = MAX_HALFWORD as scaled_t;
                        } else {
                            w = page_so_far[0 as ::core::ffi::c_int as usize]
                                - page_so_far[1 as ::core::ffi::c_int as usize]
                                - page_so_far[7 as ::core::ffi::c_int as usize];
                            if (*eqtb
                                .offset((COUNT_BASE + n as ::core::ffi::c_int) as isize))
                                .b32
                                .s1 != 1000 as int32_t
                            {
                                w = x_over_n(
                                    w,
                                    (*eqtb
                                        .offset((COUNT_BASE + n as ::core::ffi::c_int) as isize))
                                        .b32
                                        .s1,
                                ) * 1000 as scaled_t;
                            }
                        }
                        if w
                            > (*eqtb
                                .offset((SCALED_BASE + n as ::core::ffi::c_int) as isize))
                                .b32
                                .s1 - (*mem.offset((r + 3 as int32_t) as isize)).b32.s1
                        {
                            w = ((*eqtb
                                .offset((SCALED_BASE + n as ::core::ffi::c_int) as isize))
                                .b32
                                .s1 - (*mem.offset((r + 3 as int32_t) as isize)).b32.s1)
                                as scaled_t;
                        }
                        q = vert_break(
                            (*mem.offset((p + 4 as int32_t) as isize)).b32.s0,
                            w,
                            (*mem.offset((p + 2 as int32_t) as isize)).b32.s1 as scaled_t,
                        );
                        (*mem.offset((r + 3 as int32_t) as isize)).b32.s1
                            += best_height_plus_depth as int32_t;
                        if (*eqtb
                            .offset((COUNT_BASE + n as ::core::ffi::c_int) as isize))
                            .b32
                            .s1 != 1000 as int32_t
                        {
                            best_height_plus_depth = x_over_n(
                                best_height_plus_depth,
                                1000 as int32_t,
                            )
                                * (*eqtb
                                    .offset((COUNT_BASE + n as ::core::ffi::c_int) as isize))
                                    .b32
                                    .s1 as scaled_t;
                        }
                        page_so_far[0 as ::core::ffi::c_int as usize]
                            -= best_height_plus_depth;
                        (*mem.offset(r as isize)).b16.s1 = SPLIT_UP as uint16_t;
                        (*mem.offset((r + 1 as int32_t) as isize)).b32.s1 = q;
                        (*mem.offset((r + 1 as int32_t) as isize)).b32.s0 = p;
                        if q == TEX_NULL as int32_t {
                            insert_penalties = (insert_penalties as ::core::ffi::c_int
                                + EJECT_PENALTY) as int32_t;
                        } else if (*mem.offset(q as isize)).b16.s1 as ::core::ffi::c_int
                            == PENALTY_NODE
                        {
                            insert_penalties
                                += (*mem.offset((q + 1 as int32_t) as isize)).b32.s1;
                        }
                    }
                }
                current_block = 371458106509226653;
            }
            _ => {
                confusion(b"page\0" as *const u8 as *const ::core::ffi::c_char);
            }
        }
        match current_block {
            1417769144978639029 => {
                if pi < INF_PENALTY as int32_t {
                    if page_so_far[1 as ::core::ffi::c_int as usize]
                        < page_so_far[0 as ::core::ffi::c_int as usize]
                    {
                        if page_so_far[3 as ::core::ffi::c_int as usize] != 0 as scaled_t
                            || page_so_far[4 as ::core::ffi::c_int as usize]
                                != 0 as scaled_t
                            || page_so_far[5 as ::core::ffi::c_int as usize]
                                != 0 as scaled_t
                        {
                            b = 0 as ::core::ffi::c_int as int32_t;
                        } else {
                            b = badness(
                                page_so_far[0 as ::core::ffi::c_int as usize]
                                    - page_so_far[1 as ::core::ffi::c_int as usize],
                                page_so_far[2 as ::core::ffi::c_int as usize],
                            );
                        }
                    } else if page_so_far[1 as ::core::ffi::c_int as usize]
                        - page_so_far[0 as ::core::ffi::c_int as usize]
                        > page_so_far[6 as ::core::ffi::c_int as usize]
                    {
                        b = AWFUL_BAD as int32_t;
                    } else {
                        b = badness(
                            page_so_far[1 as ::core::ffi::c_int as usize]
                                - page_so_far[0 as ::core::ffi::c_int as usize],
                            page_so_far[6 as ::core::ffi::c_int as usize],
                        );
                    }
                    if b < AWFUL_BAD as int32_t {
                        if pi <= EJECT_PENALTY as int32_t {
                            c = pi;
                        } else if b < INF_BAD as int32_t {
                            c = b + pi + insert_penalties;
                        } else {
                            c = 100000 as int32_t;
                        }
                    } else {
                        c = b;
                    }
                    if insert_penalties >= 10000 as int32_t {
                        c = MAX_HALFWORD as int32_t;
                    }
                    if c <= least_page_cost {
                        best_page_break = p;
                        best_size = page_so_far[0 as ::core::ffi::c_int as usize];
                        least_page_cost = c;
                        r = (*mem.offset(4999999 as ::core::ffi::c_int as isize)).b32.s1;
                        while r != PAGE_INS_HEAD as int32_t {
                            (*mem.offset((r + 2 as int32_t) as isize)).b32.s0 = (*mem
                                .offset((r + 2 as int32_t) as isize))
                                .b32
                                .s1;
                            r = (*mem.offset(r as isize)).b32.s1;
                        }
                    }
                    if c == AWFUL_BAD as int32_t || pi <= EJECT_PENALTY as int32_t {
                        fire_up(p);
                        if output_active {
                            return;
                        }
                        current_block = 17778012151635330486;
                    } else {
                        current_block = 13932507243822716336;
                    }
                } else {
                    current_block = 13932507243822716336;
                }
                match current_block {
                    17778012151635330486 => {}
                    _ => {
                        if ((*mem.offset(p as isize)).b16.s1 as ::core::ffi::c_int)
                            < GLUE_NODE
                            || (*mem.offset(p as isize)).b16.s1 as ::core::ffi::c_int
                                > KERN_NODE
                        {
                            current_block = 371458106509226653;
                        } else {
                            current_block = 1469412417798502801;
                        }
                    }
                }
            }
            14783794473175570661 => {
                (*mem
                    .offset(
                        (4999999 as ::core::ffi::c_int - 1 as ::core::ffi::c_int)
                            as isize,
                    ))
                    .b32
                    .s1 = (*mem.offset(p as isize)).b32.s1;
                (*mem.offset(p as isize)).b32.s1 = TEX_NULL as int32_t;
                if (*eqtb.offset((INT_BASE + INT_PAR__saving_vdiscards) as isize)).b32.s1
                    <= 0 as int32_t
                {
                    flush_node_list(p);
                } else {
                    if disc_ptr[LAST_BOX_CODE as usize] == TEX_NULL as int32_t {
                        disc_ptr[LAST_BOX_CODE as usize] = p;
                    } else {
                        (*mem
                            .offset(disc_ptr[1 as ::core::ffi::c_int as usize] as isize))
                            .b32
                            .s1 = p;
                    }
                    disc_ptr[COPY_CODE as usize] = p;
                }
                current_block = 17778012151635330486;
            }
            _ => {}
        }
        match current_block {
            1469412417798502801 => {
                if (*mem.offset(p as isize)).b16.s1 as ::core::ffi::c_int == KERN_NODE {
                    q = p;
                } else {
                    q = (*mem.offset((p + 1 as int32_t) as isize)).b32.s0;
                    page_so_far[(2 as ::core::ffi::c_int
                        + (*mem.offset(q as isize)).b16.s1 as ::core::ffi::c_int)
                        as usize] += (*mem.offset((q + 2 as int32_t) as isize)).b32.s1;
                    page_so_far[6 as ::core::ffi::c_int as usize]
                        += (*mem.offset((q + 3 as int32_t) as isize)).b32.s1;
                    if (*mem.offset(q as isize)).b16.s0 as ::core::ffi::c_int != NORMAL
                        && (*mem.offset((q + 3 as int32_t) as isize)).b32.s1
                            != 0 as int32_t
                    {
                        error_here_with_diagnostic(
                            b"Infinite glue shrinkage found on current page\0"
                                as *const u8 as *const ::core::ffi::c_char,
                        );
                        capture_to_diagnostic(
                            ::core::ptr::null_mut::<ttbc_diagnostic_t>(),
                        );
                        help_ptr = 4 as ::core::ffi::c_uchar;
                        help_line[3 as ::core::ffi::c_int as usize] = b"The page about to be output contains some infinitely\0"
                            as *const u8 as *const ::core::ffi::c_char;
                        help_line[2 as ::core::ffi::c_int as usize] = b"shrinkable glue, e.g., `\\vss' or `\\vskip 0pt minus 1fil'.\0"
                            as *const u8 as *const ::core::ffi::c_char;
                        help_line[1 as ::core::ffi::c_int as usize] = b"Such glue doesn't belong there; but you can safely proceed,\0"
                            as *const u8 as *const ::core::ffi::c_char;
                        help_line[0 as ::core::ffi::c_int as usize] = b"since the offensive shrinkability has been made finite.\0"
                            as *const u8 as *const ::core::ffi::c_char;
                        error();
                        r = new_spec(q);
                        (*mem.offset(r as isize)).b16.s0 = NORMAL as uint16_t;
                        delete_glue_ref(q);
                        (*mem.offset((p + 1 as int32_t) as isize)).b32.s0 = r;
                        q = r;
                    }
                }
                page_so_far[1 as ::core::ffi::c_int as usize]
                    += (page_so_far[7 as ::core::ffi::c_int as usize]
                        + (*mem.offset((q + 1 as int32_t) as isize)).b32.s1 as scaled_t)
                        as int32_t;
                page_so_far[7 as ::core::ffi::c_int as usize] = 0 as ::core::ffi::c_int
                    as scaled_t;
                current_block = 371458106509226653;
            }
            _ => {}
        }
        match current_block {
            371458106509226653 => {
                if page_so_far[7 as ::core::ffi::c_int as usize] > page_max_depth {
                    page_so_far[1 as ::core::ffi::c_int as usize]
                        += page_so_far[7 as ::core::ffi::c_int as usize]
                            - page_max_depth;
                    page_so_far[7 as ::core::ffi::c_int as usize] = page_max_depth;
                }
                (*mem.offset(page_tail as isize)).b32.s1 = p;
                page_tail = p;
                (*mem
                    .offset(
                        (4999999 as ::core::ffi::c_int - 1 as ::core::ffi::c_int)
                            as isize,
                    ))
                    .b32
                    .s1 = (*mem.offset(p as isize)).b32.s1;
                (*mem.offset(p as isize)).b32.s1 = TEX_NULL as int32_t;
            }
            _ => {}
        }
        if !((*mem
            .offset((4999999 as ::core::ffi::c_int - 1 as ::core::ffi::c_int) as isize))
            .b32
            .s1 != TEX_NULL as int32_t)
        {
            break;
        }
    }
    if nest_ptr == 0 as int32_t {
        cur_list.tail = CONTRIB_HEAD as int32_t;
    } else {
        (*nest.offset(0 as ::core::ffi::c_int as isize)).tail = CONTRIB_HEAD as int32_t;
    };
}
pub const true_0: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const false_0: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const MIN_HALFWORD: ::core::ffi::c_int = -(0xfffffff as ::core::ffi::c_int);
pub const MAX_HALFWORD: ::core::ffi::c_int = 0x3fffffff as ::core::ffi::c_int;
pub const TEX_NULL: ::core::ffi::c_int = MIN_HALFWORD;
pub const BIGGEST_USV: ::core::ffi::c_int = 0x10ffff as ::core::ffi::c_int;
pub const NUMBER_USVS: ::core::ffi::c_int = BIGGEST_USV + 1 as ::core::ffi::c_int;
pub const PAGE_INS_HEAD: ::core::ffi::c_int = MEM_TOP;
pub const CONTRIB_HEAD: ::core::ffi::c_int = MEM_TOP - 1 as ::core::ffi::c_int;
pub const PAGE_HEAD: ::core::ffi::c_int = MEM_TOP - 2 as ::core::ffi::c_int;
pub const HOLD_HEAD: ::core::ffi::c_int = MEM_TOP - 4 as ::core::ffi::c_int;
pub const PAGE_INS_NODE_SIZE: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
pub const INS_NODE_SIZE: ::core::ffi::c_int = 5 as ::core::ffi::c_int;
pub const BOX_NODE_SIZE: ::core::ffi::c_int = 8 as ::core::ffi::c_int;
pub const OUTPUT_GROUP: ::core::ffi::c_int = 8 as ::core::ffi::c_int;
pub const NORMAL: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const OUTPUT_TEXT: ::core::ffi::c_int = 7 as ::core::ffi::c_int;
pub const EMPTY: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const INSERTS_ONLY: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const BOX_THERE: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const EXACTLY: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const INSERTING: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const ADDITIONAL: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const SPLIT_UP: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const MARK_VAL: ::core::ffi::c_int = 7 as ::core::ffi::c_int;
pub const EJECT_PENALTY: ::core::ffi::c_int = -(10000 as ::core::ffi::c_int);
pub const INF_BAD: ::core::ffi::c_int = 10000 as ::core::ffi::c_int;
pub const INF_PENALTY: ::core::ffi::c_int = 10000 as ::core::ffi::c_int;
pub const FIRE_UP_INIT: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const FIRE_UP_DONE: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const IGNORE_DEPTH: ::core::ffi::c_int = -(65536000 as ::core::ffi::c_int);
