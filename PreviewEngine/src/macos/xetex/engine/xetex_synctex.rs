/* Summary of original C code; see end of file for full comments:
 * Copyright (c) 2008-2017 jerome DOT laurens AT u-bourgogne DOT fr
 * MIT License.
 */
// Translated from xetex/engine/xetex-synctex.c with C2Rust 0.22.1.
use ::c2rust_bitfields;
extern "C" {
    pub type ttbc_output_handle_t;
    fn malloc(__size: size_t) -> *mut ::core::ffi::c_void;
    fn free(_: *mut ::core::ffi::c_void);
    fn realloc(__ptr: *mut ::core::ffi::c_void, __size: size_t) -> *mut ::core::ffi::c_void;
    fn strcat(
        __s1: *mut ::core::ffi::c_char,
        __s2: *const ::core::ffi::c_char,
    ) -> *mut ::core::ffi::c_char;
    fn strcpy(
        __dst: *mut ::core::ffi::c_char,
        __src: *const ::core::ffi::c_char,
    ) -> *mut ::core::ffi::c_char;
    fn strlen(__s: *const ::core::ffi::c_char) -> size_t;
    fn strdup(__s1: *const ::core::ffi::c_char) -> *mut ::core::ffi::c_char;
    fn ttstub_issue_warning(format: *const ::core::ffi::c_char, ...);
    fn ttstub_issue_error(format: *const ::core::ffi::c_char, ...);
    fn ttstub_output_open(
        path: *const ::core::ffi::c_char,
        is_gz: ::core::ffi::c_int,
    ) -> rust_output_handle_t;
    fn ttstub_fprintf(
        handle: rust_output_handle_t,
        format: *const ::core::ffi::c_char,
        ...
    ) -> ::core::ffi::c_int;
    fn ttstub_output_close(handle: rust_output_handle_t) -> ::core::ffi::c_int;
    fn gettexstring(_: str_number) -> *mut ::core::ffi::c_char;
    static mut abspath_of_input_file: [::core::ffi::c_char; 0];
    static mut eqtb: *mut memory_word;
    static mut mem: *mut memory_word;
    static mut cur_input: input_state_t;
    static mut job_name: str_number;
    static mut total_pages: int32_t;
    static mut rule_dp: scaled_t;
    static mut rule_wd: scaled_t;
    static mut rule_ht: scaled_t;
    static mut cur_h: scaled_t;
    static mut cur_v: scaled_t;
    static mut synctex_enabled: bool;
    static mut synctex_use_gz: bool;
    static mut synctex_texpresso_extension: bool;
}
pub type __darwin_size_t = usize;
pub type size_t = __darwin_size_t;
pub type int32_t = i32;
pub type uint16_t = u16;
pub type rust_output_handle_t = *mut ttbc_output_handle_t;
pub type scaled_t = int32_t;
pub type str_number = int32_t;
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
#[derive(Copy, Clone, BitfieldStruct)]
#[repr(C)]
pub struct _flags {
    #[bitfield(name = "content_ready", ty = "::core::ffi::c_uint", bits = "0..=0")]
    #[bitfield(name = "off", ty = "::core::ffi::c_uint", bits = "1..=1")]
    #[bitfield(name = "not_void", ty = "::core::ffi::c_uint", bits = "2..=2")]
    #[bitfield(name = "warn", ty = "::core::ffi::c_uint", bits = "3..=3")]
    #[bitfield(name = "output_p", ty = "::core::ffi::c_uint", bits = "4..=4")]
    pub content_ready_off_not_void_warn_output_p: [u8; 1],
    #[bitfield(padding)]
    pub c2rust_padding: [u8; 3],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed {
    pub file: rust_output_handle_t,
    pub root_name: *mut ::core::ffi::c_char,
    pub count: int32_t,
    pub node: int32_t,
    pub recorder: synctex_recorder_t,
    pub tag: int32_t,
    pub line: int32_t,
    pub curh: int32_t,
    pub curv: int32_t,
    pub magnification: int32_t,
    pub unit: int32_t,
    pub total_length: int32_t,
    pub lastv: int32_t,
    pub form_depth: int32_t,
    pub synctex_tag_counter: ::core::ffi::c_uint,
    pub flags: _flags,
}
pub type synctex_recorder_t = Option<unsafe extern "C" fn(int32_t) -> ()>;
pub const __DARWIN_NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
pub const NULL: *mut ::core::ffi::c_void = __DARWIN_NULL;
pub const SYNCTEX_FIELD_SIZE: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const MEDIUM_NODE_SIZE: ::core::ffi::c_int = 3 as ::core::ffi::c_int;
pub const RULE_NODE_SIZE: ::core::ffi::c_int = 5 as ::core::ffi::c_int;
pub const BOX_NODE_SIZE: ::core::ffi::c_int = 8 as ::core::ffi::c_int;
#[inline]
unsafe extern "C" fn mfree(mut ptr: *mut ::core::ffi::c_void) -> *mut ::core::ffi::c_void {
    free(ptr);
    return NULL;
}
pub const INT_PAR__synctex: ::core::ffi::c_int = 81 as ::core::ffi::c_int;
pub const INT_BASE: ::core::ffi::c_int = 7826729 as ::core::ffi::c_int;
pub const SYNCTEX_VERSION: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const GLUE_NODE_SIZE: ::core::ffi::c_int = MEDIUM_NODE_SIZE;
pub const KERN_NODE_SIZE: ::core::ffi::c_int = MEDIUM_NODE_SIZE;
pub const MATH_NODE_SIZE: ::core::ffi::c_int = MEDIUM_NODE_SIZE;
pub const width_offset: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const depth_offset: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const height_offset: ::core::ffi::c_int = 3 as ::core::ffi::c_int;
pub const rule_node: ::core::ffi::c_int = 2;
pub const glue_node: ::core::ffi::c_int = 10;
pub const kern_node: ::core::ffi::c_int = 11;
static mut synctex_ctxt: C2RustUnnamed = C2RustUnnamed {
    file: ::core::ptr::null::<ttbc_output_handle_t>() as *mut ttbc_output_handle_t,
    root_name: ::core::ptr::null::<::core::ffi::c_char>() as *mut ::core::ffi::c_char,
    count: 0,
    node: 0,
    recorder: None,
    tag: 0,
    line: 0,
    curh: 0,
    curv: 0,
    magnification: 0,
    unit: 0,
    total_length: 0,
    lastv: 0,
    form_depth: 0,
    synctex_tag_counter: 0,
    flags: _flags {
        content_ready_off_not_void_warn_output_p: [0; 1],
        c2rust_padding: [0; 3],
    },
};
unsafe extern "C" fn get_current_name() -> *mut ::core::ffi::c_char {
    return strdup(&raw mut abspath_of_input_file as *mut ::core::ffi::c_char);
}
#[no_mangle]
pub unsafe extern "C" fn synctex_init_command() {
    synctex_ctxt.file = ::core::ptr::null_mut::<ttbc_output_handle_t>();
    synctex_ctxt.root_name = ::core::ptr::null_mut::<::core::ffi::c_char>();
    synctex_ctxt.count = 0 as ::core::ffi::c_int as int32_t;
    synctex_ctxt.node = 0 as ::core::ffi::c_int as int32_t;
    synctex_ctxt.recorder = None;
    synctex_ctxt.tag = 0 as ::core::ffi::c_int as int32_t;
    synctex_ctxt.line = 0 as ::core::ffi::c_int as int32_t;
    synctex_ctxt.curh = 0 as ::core::ffi::c_int as int32_t;
    synctex_ctxt.curv = 0 as ::core::ffi::c_int as int32_t;
    synctex_ctxt.magnification = 0 as ::core::ffi::c_int as int32_t;
    synctex_ctxt.unit = 0 as ::core::ffi::c_int as int32_t;
    synctex_ctxt.total_length = 0 as ::core::ffi::c_int as int32_t;
    synctex_ctxt.lastv = -(1 as ::core::ffi::c_int) as int32_t;
    synctex_ctxt.form_depth = 0 as ::core::ffi::c_int as int32_t;
    synctex_ctxt.synctex_tag_counter = 0 as ::core::ffi::c_uint;
    synctex_ctxt
        .flags
        .set_content_ready(0 as ::core::ffi::c_uint as ::core::ffi::c_uint);
    synctex_ctxt
        .flags
        .set_off(0 as ::core::ffi::c_uint as ::core::ffi::c_uint);
    synctex_ctxt
        .flags
        .set_not_void(0 as ::core::ffi::c_uint as ::core::ffi::c_uint);
    synctex_ctxt
        .flags
        .set_warn(0 as ::core::ffi::c_uint as ::core::ffi::c_uint);
    synctex_ctxt
        .flags
        .set_output_p(0 as ::core::ffi::c_uint as ::core::ffi::c_uint);
    if synctex_enabled {
        (*eqtb.offset((INT_BASE + INT_PAR__synctex) as isize))
            .b32
            .s1 = 1 as ::core::ffi::c_int as int32_t;
    } else {
        (*eqtb.offset((INT_BASE + INT_PAR__synctex) as isize))
            .b32
            .s1 = 0 as ::core::ffi::c_int as int32_t;
    };
}
unsafe extern "C" fn synctexabort() {
    if !synctex_ctxt.file.is_null() {
        ttstub_output_close(synctex_ctxt.file);
        synctex_ctxt.file = ::core::ptr::null_mut::<ttbc_output_handle_t>();
    }
    synctex_ctxt.root_name =
        mfree(synctex_ctxt.root_name as *mut ::core::ffi::c_void) as *mut ::core::ffi::c_char;
    synctex_ctxt
        .flags
        .set_off(1 as ::core::ffi::c_uint as ::core::ffi::c_uint);
}
static mut synctex_suffix: *const ::core::ffi::c_char =
    b".synctex\0" as *const u8 as *const ::core::ffi::c_char;
static mut synctex_suffix_gz: *const ::core::ffi::c_char =
    b".gz\0" as *const u8 as *const ::core::ffi::c_char;
unsafe extern "C" fn synctex_dot_open() -> rust_output_handle_t {
    let mut tmp: *mut ::core::ffi::c_char = ::core::ptr::null_mut::<::core::ffi::c_char>();
    let mut the_name: *mut ::core::ffi::c_char = ::core::ptr::null_mut::<::core::ffi::c_char>();
    let mut len: size_t = 0;
    if synctex_ctxt.flags.off() as ::core::ffi::c_int != 0
        || (*eqtb.offset((INT_BASE + INT_PAR__synctex) as isize))
            .b32
            .s1
            == 0
    {
        return ::core::ptr::null_mut::<ttbc_output_handle_t>();
    }
    if !synctex_ctxt.file.is_null() {
        return synctex_ctxt.file;
    }
    tmp = gettexstring(job_name);
    len = strlen(tmp);
    if !(len <= 0 as size_t) {
        the_name = malloc(
            len.wrapping_add(strlen(synctex_suffix))
                .wrapping_add(
                    (if synctex_use_gz as ::core::ffi::c_int != 0 {
                        strlen(synctex_suffix_gz)
                    } else {
                        0 as size_t
                    }),
                )
                .wrapping_add(1 as size_t),
        ) as *mut ::core::ffi::c_char;
        strcpy(the_name, tmp);
        strcat(the_name, synctex_suffix);
        if synctex_use_gz {
            strcat(the_name, synctex_suffix_gz);
        }
        tmp = mfree(tmp as *mut ::core::ffi::c_void) as *mut ::core::ffi::c_char;
        synctex_ctxt.file = ttstub_output_open(the_name, synctex_use_gz as ::core::ffi::c_int);
        if !synctex_ctxt.file.is_null() {
            if !(synctex_record_preamble() != 0) {
                if synctex_ctxt.magnification == 0 as int32_t {
                    synctex_ctxt.magnification = 1000 as ::core::ffi::c_int as int32_t;
                }
                synctex_ctxt.unit = 1 as ::core::ffi::c_int as int32_t;
                the_name = mfree(the_name as *mut ::core::ffi::c_void) as *mut ::core::ffi::c_char;
                if !synctex_ctxt.root_name.is_null() {
                    synctex_record_input(1 as int32_t, synctex_ctxt.root_name);
                    synctex_ctxt.root_name =
                        mfree(synctex_ctxt.root_name as *mut ::core::ffi::c_void)
                            as *mut ::core::ffi::c_char;
                }
                synctex_ctxt.count = 0 as ::core::ffi::c_int as int32_t;
                return synctex_ctxt.file;
            }
        }
    }
    free(tmp as *mut ::core::ffi::c_void);
    free(the_name as *mut ::core::ffi::c_void);
    synctexabort();
    return ::core::ptr::null_mut::<ttbc_output_handle_t>();
}
unsafe extern "C" fn synctex_prepare_content() -> *mut ::core::ffi::c_void {
    if synctex_ctxt.flags.content_ready() != 0 {
        return synctex_ctxt.file as *mut ::core::ffi::c_void;
    }
    if !synctex_dot_open().is_null()
        && 0 as ::core::ffi::c_int == synctex_record_settings()
        && 0 as ::core::ffi::c_int == synctex_record_content()
    {
        synctex_ctxt
            .flags
            .set_content_ready(1 as ::core::ffi::c_uint as ::core::ffi::c_uint);
        return synctex_ctxt.file as *mut ::core::ffi::c_void;
    }
    synctexabort();
    return NULL;
}
#[no_mangle]
pub unsafe extern "C" fn synctex_start_input() {
    if synctex_ctxt.flags.off() != 0 {
        return;
    }
    if !synctex_ctxt.synctex_tag_counter > 0 as ::core::ffi::c_uint {
        synctex_ctxt.synctex_tag_counter = synctex_ctxt.synctex_tag_counter.wrapping_add(1);
    } else {
        synctex_ctxt.synctex_tag_counter = 0 as ::core::ffi::c_uint;
        return;
    }
    cur_input.synctex_tag = synctex_ctxt.synctex_tag_counter as ::core::ffi::c_int as int32_t;
    if synctex_ctxt.synctex_tag_counter == 1 as ::core::ffi::c_uint {
        synctex_ctxt.root_name = get_current_name();
        if strlen(synctex_ctxt.root_name) == 0 {
            synctex_ctxt.root_name = realloc(
                synctex_ctxt.root_name as *mut ::core::ffi::c_void,
                strlen(b"texput\0" as *const u8 as *const ::core::ffi::c_char)
                    .wrapping_add(1 as size_t),
            ) as *mut ::core::ffi::c_char;
            strcpy(
                synctex_ctxt.root_name,
                b"texput\0" as *const u8 as *const ::core::ffi::c_char,
            );
        }
        return;
    }
    if !synctex_ctxt.file.is_null() || !synctex_dot_open().is_null() {
        let mut tmp: *mut ::core::ffi::c_char = get_current_name();
        synctex_record_input(cur_input.synctex_tag, tmp);
        free(tmp as *mut ::core::ffi::c_void);
    }
}
#[no_mangle]
pub unsafe extern "C" fn synctex_end_file_reading() {
    if !synctex_texpresso_extension {
        return;
    }
    let mut len: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut tag: ::core::ffi::c_int = cur_input.synctex_tag as ::core::ffi::c_int;
    if !synctex_ctxt.file.is_null() && tag > 0 as ::core::ffi::c_int {
        len = ttstub_fprintf(
            synctex_ctxt.file,
            b"/%i\n\0" as *const u8 as *const ::core::ffi::c_char,
            tag,
        );
    }
    if len > 0 as ::core::ffi::c_int {
        synctex_ctxt.total_length =
            (synctex_ctxt.total_length as ::core::ffi::c_int + len) as int32_t;
    }
}
#[no_mangle]
pub unsafe extern "C" fn synctex_terminate(mut log_opened: bool) {
    if !synctex_ctxt.file.is_null() {
        synctex_record_postamble();
        ttstub_output_close(synctex_ctxt.file);
        synctex_ctxt.file = ::core::ptr::null_mut::<ttbc_output_handle_t>();
    }
    synctexabort();
}
#[no_mangle]
pub unsafe extern "C" fn synctex_sheet(mut mag: int32_t) {
    if synctex_ctxt.flags.off() != 0 {
        if (*eqtb.offset((INT_BASE + INT_PAR__synctex) as isize))
            .b32
            .s1
            != 0
            && synctex_ctxt.flags.warn() == 0
        {
            synctex_ctxt
                .flags
                .set_warn(1 as ::core::ffi::c_uint as ::core::ffi::c_uint);
            ttstub_issue_warning(
                b"SyncTeX was disabled -- changing the value of \\synctex has no effect\0"
                    as *const u8 as *const ::core::ffi::c_char,
            );
        }
        return;
    }
    if total_pages == 0 as int32_t {
        if mag > 0 as int32_t {
            synctex_ctxt.magnification = mag;
        }
    }
    if !synctex_prepare_content().is_null() {
        synctex_record_sheet(total_pages + 1 as int32_t);
    }
}
#[no_mangle]
pub unsafe extern "C" fn synctex_teehs() {
    if synctex_ctxt.flags.off() as ::core::ffi::c_int != 0 || synctex_ctxt.file.is_null() {
        return;
    }
    synctex_record_teehs(total_pages);
}
#[no_mangle]
pub unsafe extern "C" fn synctex_vlist(mut this_box: int32_t) {
    if synctex_ctxt.flags.off() as ::core::ffi::c_int != 0
        || (*eqtb.offset((INT_BASE + INT_PAR__synctex) as isize))
            .b32
            .s1
            == 0
        || synctex_ctxt.file.is_null()
    {
        return;
    }
    synctex_ctxt.node = this_box;
    synctex_ctxt.recorder = None;
    synctex_ctxt.tag = (*mem
        .offset((this_box + BOX_NODE_SIZE as int32_t - SYNCTEX_FIELD_SIZE as int32_t) as isize))
    .b32
    .s0;
    synctex_ctxt.line = (*mem
        .offset((this_box + BOX_NODE_SIZE as int32_t - SYNCTEX_FIELD_SIZE as int32_t) as isize))
    .b32
    .s1;
    synctex_ctxt.curh = (cur_h + 4736287 as scaled_t) as int32_t;
    synctex_ctxt.curv = (cur_v + 4736287 as scaled_t) as int32_t;
    synctex_record_node_vlist(this_box);
}
#[no_mangle]
pub unsafe extern "C" fn synctex_tsilv(mut this_box: int32_t) {
    if synctex_ctxt.flags.off() as ::core::ffi::c_int != 0
        || (*eqtb.offset((INT_BASE + INT_PAR__synctex) as isize))
            .b32
            .s1
            == 0
        || synctex_ctxt.file.is_null()
    {
        return;
    }
    synctex_ctxt.node = this_box;
    synctex_ctxt.tag = (*mem
        .offset((this_box + BOX_NODE_SIZE as int32_t - SYNCTEX_FIELD_SIZE as int32_t) as isize))
    .b32
    .s0;
    synctex_ctxt.line = (*mem
        .offset((this_box + BOX_NODE_SIZE as int32_t - SYNCTEX_FIELD_SIZE as int32_t) as isize))
    .b32
    .s1;
    synctex_ctxt.curh = (cur_h + 4736287 as scaled_t) as int32_t;
    synctex_ctxt.curv = (cur_v + 4736287 as scaled_t) as int32_t;
    synctex_ctxt.recorder = None;
    synctex_record_node_tsilv(this_box);
}
#[no_mangle]
pub unsafe extern "C" fn synctex_void_vlist(mut p: int32_t, mut this_box: int32_t) {
    if synctex_ctxt.flags.off() as ::core::ffi::c_int != 0
        || (*eqtb.offset((INT_BASE + INT_PAR__synctex) as isize))
            .b32
            .s1
            == 0
        || synctex_ctxt.file.is_null()
    {
        return;
    }
    synctex_ctxt.node = p;
    synctex_ctxt.tag = (*mem
        .offset((p + BOX_NODE_SIZE as int32_t - SYNCTEX_FIELD_SIZE as int32_t) as isize))
    .b32
    .s0;
    synctex_ctxt.line = (*mem
        .offset((p + BOX_NODE_SIZE as int32_t - SYNCTEX_FIELD_SIZE as int32_t) as isize))
    .b32
    .s1;
    synctex_ctxt.curh = (cur_h + 4736287 as scaled_t) as int32_t;
    synctex_ctxt.curv = (cur_v + 4736287 as scaled_t) as int32_t;
    synctex_ctxt.recorder = None;
    synctex_record_node_void_vlist(p);
}
#[no_mangle]
pub unsafe extern "C" fn synctex_hlist(mut this_box: int32_t) {
    if synctex_ctxt.flags.off() as ::core::ffi::c_int != 0
        || (*eqtb.offset((INT_BASE + INT_PAR__synctex) as isize))
            .b32
            .s1
            == 0
        || synctex_ctxt.file.is_null()
    {
        return;
    }
    synctex_ctxt.node = this_box;
    synctex_ctxt.tag = (*mem
        .offset((this_box + BOX_NODE_SIZE as int32_t - SYNCTEX_FIELD_SIZE as int32_t) as isize))
    .b32
    .s0;
    synctex_ctxt.line = (*mem
        .offset((this_box + BOX_NODE_SIZE as int32_t - SYNCTEX_FIELD_SIZE as int32_t) as isize))
    .b32
    .s1;
    synctex_ctxt.curh = (cur_h + 4736287 as scaled_t) as int32_t;
    synctex_ctxt.curv = (cur_v + 4736287 as scaled_t) as int32_t;
    synctex_ctxt.recorder = None;
    synctex_record_node_hlist(this_box);
}
#[no_mangle]
pub unsafe extern "C" fn synctex_tsilh(mut this_box: int32_t) {
    if synctex_ctxt.flags.off() as ::core::ffi::c_int != 0
        || (*eqtb.offset((INT_BASE + INT_PAR__synctex) as isize))
            .b32
            .s1
            == 0
        || synctex_ctxt.file.is_null()
    {
        return;
    }
    synctex_ctxt.node = this_box;
    synctex_ctxt.tag = (*mem
        .offset((this_box + BOX_NODE_SIZE as int32_t - SYNCTEX_FIELD_SIZE as int32_t) as isize))
    .b32
    .s0;
    synctex_ctxt.line = (*mem
        .offset((this_box + BOX_NODE_SIZE as int32_t - SYNCTEX_FIELD_SIZE as int32_t) as isize))
    .b32
    .s1;
    synctex_ctxt.curh = (cur_h + 4736287 as scaled_t) as int32_t;
    synctex_ctxt.curv = (cur_v + 4736287 as scaled_t) as int32_t;
    synctex_ctxt.recorder = None;
    synctex_record_node_tsilh(this_box);
}
#[no_mangle]
pub unsafe extern "C" fn synctex_void_hlist(mut p: int32_t, mut this_box: int32_t) {
    if synctex_ctxt.flags.off() as ::core::ffi::c_int != 0
        || (*eqtb.offset((INT_BASE + INT_PAR__synctex) as isize))
            .b32
            .s1
            == 0
        || synctex_ctxt.file.is_null()
    {
        return;
    }
    if synctex_ctxt.recorder.is_some() {
        Some(synctex_ctxt.recorder.expect("non-null function pointer"))
            .expect("non-null function pointer")(synctex_ctxt.node);
    }
    synctex_ctxt.node = p;
    synctex_ctxt.tag = (*mem
        .offset((p + BOX_NODE_SIZE as int32_t - SYNCTEX_FIELD_SIZE as int32_t) as isize))
    .b32
    .s0;
    synctex_ctxt.line = (*mem
        .offset((p + BOX_NODE_SIZE as int32_t - SYNCTEX_FIELD_SIZE as int32_t) as isize))
    .b32
    .s1;
    synctex_ctxt.curh = (cur_h + 4736287 as scaled_t) as int32_t;
    synctex_ctxt.curv = (cur_v + 4736287 as scaled_t) as int32_t;
    synctex_ctxt.recorder = None;
    synctex_record_node_void_hlist(p);
}
#[no_mangle]
pub unsafe extern "C" fn synctex_math(mut p: int32_t, mut this_box: int32_t) {
    if synctex_ctxt.flags.off() as ::core::ffi::c_int != 0
        || (*eqtb.offset((INT_BASE + INT_PAR__synctex) as isize))
            .b32
            .s1
            == 0
        || synctex_ctxt.file.is_null()
    {
        return;
    }
    if synctex_ctxt.recorder.is_some()
        && (0 as int32_t == synctex_ctxt.node
            || (*mem
                .offset((p + MATH_NODE_SIZE as int32_t - SYNCTEX_FIELD_SIZE as int32_t) as isize))
            .b32
            .s0 != synctex_ctxt.tag
            || (*mem
                .offset((p + MATH_NODE_SIZE as int32_t - SYNCTEX_FIELD_SIZE as int32_t) as isize))
            .b32
            .s1 != synctex_ctxt.line)
    {
        Some(synctex_ctxt.recorder.expect("non-null function pointer"))
            .expect("non-null function pointer")(synctex_ctxt.node);
    }
    synctex_ctxt.node = p;
    synctex_ctxt.tag = (*mem
        .offset((p + MATH_NODE_SIZE as int32_t - SYNCTEX_FIELD_SIZE as int32_t) as isize))
    .b32
    .s0;
    synctex_ctxt.line = (*mem
        .offset((p + MATH_NODE_SIZE as int32_t - SYNCTEX_FIELD_SIZE as int32_t) as isize))
    .b32
    .s1;
    synctex_ctxt.curh = (cur_h + 4736287 as scaled_t) as int32_t;
    synctex_ctxt.curv = (cur_v + 4736287 as scaled_t) as int32_t;
    synctex_ctxt.recorder = None;
    synctex_record_node_math(p);
}
#[no_mangle]
pub unsafe extern "C" fn synctex_horizontal_rule_or_glue(mut p: int32_t, mut this_box: int32_t) {
    match (*mem.offset(p as isize)).b16.s1 as ::core::ffi::c_int {
        rule_node => {
            if synctex_ctxt.flags.off() as ::core::ffi::c_int != 0
                || (*eqtb.offset((INT_BASE + INT_PAR__synctex) as isize))
                    .b32
                    .s1
                    == 0
                || 0 as int32_t
                    >= (*mem.offset(
                        (p + RULE_NODE_SIZE as int32_t - SYNCTEX_FIELD_SIZE as int32_t) as isize,
                    ))
                    .b32
                    .s0
                || 0 as int32_t
                    >= (*mem.offset(
                        (p + RULE_NODE_SIZE as int32_t - SYNCTEX_FIELD_SIZE as int32_t) as isize,
                    ))
                    .b32
                    .s1
            {
                return;
            }
        }
        glue_node => {
            if synctex_ctxt.flags.off() as ::core::ffi::c_int != 0
                || (*eqtb.offset((INT_BASE + INT_PAR__synctex) as isize))
                    .b32
                    .s1
                    == 0
                || 0 as int32_t
                    >= (*mem.offset(
                        (p + GLUE_NODE_SIZE as int32_t - SYNCTEX_FIELD_SIZE as int32_t) as isize,
                    ))
                    .b32
                    .s0
                || 0 as int32_t
                    >= (*mem.offset(
                        (p + GLUE_NODE_SIZE as int32_t - SYNCTEX_FIELD_SIZE as int32_t) as isize,
                    ))
                    .b32
                    .s1
            {
                return;
            }
        }
        kern_node => {
            if synctex_ctxt.flags.off() as ::core::ffi::c_int != 0
                || (*eqtb.offset((INT_BASE + INT_PAR__synctex) as isize))
                    .b32
                    .s1
                    == 0
                || 0 as int32_t
                    >= (*mem.offset(
                        (p + KERN_NODE_SIZE as int32_t - SYNCTEX_FIELD_SIZE as int32_t) as isize,
                    ))
                    .b32
                    .s0
                || 0 as int32_t
                    >= (*mem.offset(
                        (p + KERN_NODE_SIZE as int32_t - SYNCTEX_FIELD_SIZE as int32_t) as isize,
                    ))
                    .b32
                    .s1
            {
                return;
            }
        }
        _ => {
            ttstub_issue_error(
                b"unknown node type %d in SyncTeX\0" as *const u8 as *const ::core::ffi::c_char,
                (*mem.offset(p as isize)).b16.s1 as ::core::ffi::c_int,
            );
        }
    }
    synctex_ctxt.node = p;
    synctex_ctxt.curh = (cur_h + 4736287 as scaled_t) as int32_t;
    synctex_ctxt.curv = (cur_v + 4736287 as scaled_t) as int32_t;
    synctex_ctxt.recorder = None;
    match (*mem.offset(p as isize)).b16.s1 as ::core::ffi::c_int {
        rule_node => {
            synctex_ctxt.tag = (*mem
                .offset((p + RULE_NODE_SIZE as int32_t - SYNCTEX_FIELD_SIZE as int32_t) as isize))
            .b32
            .s0;
            synctex_ctxt.line = (*mem
                .offset((p + RULE_NODE_SIZE as int32_t - SYNCTEX_FIELD_SIZE as int32_t) as isize))
            .b32
            .s1;
            synctex_record_node_rule(p);
        }
        glue_node => {
            synctex_ctxt.tag = (*mem
                .offset((p + GLUE_NODE_SIZE as int32_t - SYNCTEX_FIELD_SIZE as int32_t) as isize))
            .b32
            .s0;
            synctex_ctxt.line = (*mem
                .offset((p + GLUE_NODE_SIZE as int32_t - SYNCTEX_FIELD_SIZE as int32_t) as isize))
            .b32
            .s1;
            synctex_record_node_glue(p);
        }
        kern_node => {
            synctex_ctxt.tag = (*mem
                .offset((p + KERN_NODE_SIZE as int32_t - SYNCTEX_FIELD_SIZE as int32_t) as isize))
            .b32
            .s0;
            synctex_ctxt.line = (*mem
                .offset((p + KERN_NODE_SIZE as int32_t - SYNCTEX_FIELD_SIZE as int32_t) as isize))
            .b32
            .s1;
            synctex_record_node_kern(p);
        }
        _ => {
            ttstub_issue_error(
                b"unknown node type %d in SyncTeX\0" as *const u8 as *const ::core::ffi::c_char,
                (*mem.offset(p as isize)).b16.s1 as ::core::ffi::c_int,
            );
        }
    };
}
#[no_mangle]
pub unsafe extern "C" fn synctex_kern(mut p: int32_t, mut this_box: int32_t) {
    if synctex_ctxt.flags.off() as ::core::ffi::c_int != 0
        || (*eqtb.offset((INT_BASE + INT_PAR__synctex) as isize))
            .b32
            .s1
            == 0
        || 0 as int32_t
            >= (*mem
                .offset((p + KERN_NODE_SIZE as int32_t - SYNCTEX_FIELD_SIZE as int32_t) as isize))
            .b32
            .s0
        || 0 as int32_t
            >= (*mem
                .offset((p + KERN_NODE_SIZE as int32_t - SYNCTEX_FIELD_SIZE as int32_t) as isize))
            .b32
            .s1
    {
        return;
    }
    if 0 as int32_t == synctex_ctxt.node
        || (*mem.offset((p + KERN_NODE_SIZE as int32_t - SYNCTEX_FIELD_SIZE as int32_t) as isize))
            .b32
            .s0
            != synctex_ctxt.tag
        || (*mem.offset((p + KERN_NODE_SIZE as int32_t - SYNCTEX_FIELD_SIZE as int32_t) as isize))
            .b32
            .s1
            != synctex_ctxt.line
    {
        if synctex_ctxt.recorder.is_some() {
            Some(synctex_ctxt.recorder.expect("non-null function pointer"))
                .expect("non-null function pointer")(synctex_ctxt.node);
        }
        if synctex_ctxt.node == this_box {
            synctex_ctxt.node = p;
            synctex_ctxt.tag = (*mem
                .offset((p + KERN_NODE_SIZE as int32_t - SYNCTEX_FIELD_SIZE as int32_t) as isize))
            .b32
            .s0;
            synctex_ctxt.line = (*mem
                .offset((p + KERN_NODE_SIZE as int32_t - SYNCTEX_FIELD_SIZE as int32_t) as isize))
            .b32
            .s1;
            synctex_ctxt.recorder =
                Some(synctex_record_node_kern as unsafe extern "C" fn(int32_t) -> ())
                    as synctex_recorder_t;
        } else {
            synctex_ctxt.node = p;
            synctex_ctxt.tag = (*mem
                .offset((p + KERN_NODE_SIZE as int32_t - SYNCTEX_FIELD_SIZE as int32_t) as isize))
            .b32
            .s0;
            synctex_ctxt.line = (*mem
                .offset((p + KERN_NODE_SIZE as int32_t - SYNCTEX_FIELD_SIZE as int32_t) as isize))
            .b32
            .s1;
            synctex_ctxt.recorder = None;
            synctex_record_node_kern(p);
        }
    } else {
        synctex_ctxt.node = p;
        synctex_ctxt.tag = (*mem
            .offset((p + KERN_NODE_SIZE as int32_t - SYNCTEX_FIELD_SIZE as int32_t) as isize))
        .b32
        .s0;
        synctex_ctxt.line = (*mem
            .offset((p + KERN_NODE_SIZE as int32_t - SYNCTEX_FIELD_SIZE as int32_t) as isize))
        .b32
        .s1;
        synctex_ctxt.recorder =
            Some(synctex_record_node_kern as unsafe extern "C" fn(int32_t) -> ())
                as synctex_recorder_t;
    };
}
#[no_mangle]
pub unsafe extern "C" fn synctex_current() {
    let mut len: ::core::ffi::c_int = 0;
    if synctex_ctxt.flags.off() as ::core::ffi::c_int != 0
        || (*eqtb.offset((INT_BASE + INT_PAR__synctex) as isize))
            .b32
            .s1
            == 0
        || synctex_ctxt.file.is_null()
    {
        return;
    }
    len = ttstub_fprintf(
        synctex_ctxt.file,
        b"x%i,%i:%i,%i\n\0" as *const u8 as *const ::core::ffi::c_char,
        synctex_ctxt.tag,
        synctex_ctxt.line,
        (cur_h + 4736287 as scaled_t) / synctex_ctxt.unit as scaled_t,
        (cur_v + 4736287 as scaled_t) / synctex_ctxt.unit as scaled_t,
    );
    synctex_ctxt.lastv = (cur_v + 4736287 as scaled_t) as int32_t;
    if len > 0 as ::core::ffi::c_int {
        synctex_ctxt.total_length =
            (synctex_ctxt.total_length as ::core::ffi::c_int + len) as int32_t;
    } else {
        synctexabort();
    };
}
#[inline]
unsafe extern "C" fn synctex_record_settings() -> ::core::ffi::c_int {
    let mut len: ::core::ffi::c_int = 0;
    if synctex_ctxt.file.is_null() {
        return 0 as ::core::ffi::c_int;
    }
    len = ttstub_fprintf(
        synctex_ctxt.file,
        b"Output:pdf\nMagnification:%i\nUnit:%i\nX Offset:0\nY Offset:0\n\0" as *const u8
            as *const ::core::ffi::c_char,
        synctex_ctxt.magnification,
        synctex_ctxt.unit,
    );
    if len > 0 as ::core::ffi::c_int {
        synctex_ctxt.total_length =
            (synctex_ctxt.total_length as ::core::ffi::c_int + len) as int32_t;
        return 0 as ::core::ffi::c_int;
    }
    synctexabort();
    return -(1 as ::core::ffi::c_int);
}
#[inline]
unsafe extern "C" fn synctex_record_preamble() -> ::core::ffi::c_int {
    let mut len: ::core::ffi::c_int = ttstub_fprintf(
        synctex_ctxt.file,
        b"SyncTeX Version:%i\n\0" as *const u8 as *const ::core::ffi::c_char,
        SYNCTEX_VERSION,
    );
    if len > 0 as ::core::ffi::c_int {
        synctex_ctxt.total_length = len as int32_t;
        return 0 as ::core::ffi::c_int;
    }
    synctexabort();
    return -(1 as ::core::ffi::c_int);
}
#[inline]
unsafe extern "C" fn synctex_record_input(
    mut tag: int32_t,
    mut name: *mut ::core::ffi::c_char,
) -> ::core::ffi::c_int {
    let mut len: ::core::ffi::c_int = ttstub_fprintf(
        synctex_ctxt.file,
        b"Input:%i:%s\n\0" as *const u8 as *const ::core::ffi::c_char,
        tag,
        name,
    );
    if len > 0 as ::core::ffi::c_int {
        synctex_ctxt.total_length =
            (synctex_ctxt.total_length as ::core::ffi::c_int + len) as int32_t;
        return 0 as ::core::ffi::c_int;
    }
    synctexabort();
    return -(1 as ::core::ffi::c_int);
}
#[inline]
unsafe extern "C" fn synctex_record_anchor() -> ::core::ffi::c_int {
    let mut len: ::core::ffi::c_int = ttstub_fprintf(
        synctex_ctxt.file,
        b"!%i\n\0" as *const u8 as *const ::core::ffi::c_char,
        synctex_ctxt.total_length,
    );
    if len > 0 as ::core::ffi::c_int {
        synctex_ctxt.total_length = len as int32_t;
        synctex_ctxt.count += 1;
        return 0 as ::core::ffi::c_int;
    }
    synctexabort();
    return -(1 as ::core::ffi::c_int);
}
#[inline]
unsafe extern "C" fn synctex_record_content() -> ::core::ffi::c_int {
    let mut len: ::core::ffi::c_int = ttstub_fprintf(
        synctex_ctxt.file,
        b"Content:\n\0" as *const u8 as *const ::core::ffi::c_char,
    );
    if len > 0 as ::core::ffi::c_int {
        synctex_ctxt.total_length =
            (synctex_ctxt.total_length as ::core::ffi::c_int + len) as int32_t;
        return 0 as ::core::ffi::c_int;
    }
    synctexabort();
    return -(1 as ::core::ffi::c_int);
}
#[inline]
unsafe extern "C" fn synctex_record_sheet(mut sheet: int32_t) -> ::core::ffi::c_int {
    if 0 as ::core::ffi::c_int == synctex_record_anchor() {
        let mut len: ::core::ffi::c_int = ttstub_fprintf(
            synctex_ctxt.file,
            b"{%i\n\0" as *const u8 as *const ::core::ffi::c_char,
            sheet,
        );
        if len > 0 as ::core::ffi::c_int {
            synctex_ctxt.total_length =
                (synctex_ctxt.total_length as ::core::ffi::c_int + len) as int32_t;
            synctex_ctxt.count += 1;
            return 0 as ::core::ffi::c_int;
        }
    }
    synctexabort();
    return -(1 as ::core::ffi::c_int);
}
#[inline]
unsafe extern "C" fn synctex_record_teehs(mut sheet: int32_t) -> ::core::ffi::c_int {
    if 0 as ::core::ffi::c_int == synctex_record_anchor() {
        let mut len: ::core::ffi::c_int = ttstub_fprintf(
            synctex_ctxt.file,
            b"}%i\n\0" as *const u8 as *const ::core::ffi::c_char,
            sheet,
        );
        if len > 0 as ::core::ffi::c_int {
            synctex_ctxt.total_length =
                (synctex_ctxt.total_length as ::core::ffi::c_int + len) as int32_t;
            synctex_ctxt.count += 1;
            return 0 as ::core::ffi::c_int;
        }
    }
    synctexabort();
    return -(1 as ::core::ffi::c_int);
}
#[no_mangle]
pub unsafe extern "C" fn synctex_pdfxform(mut p: int32_t) {
    if synctex_ctxt.flags.off() != 0 {
        if (*eqtb.offset((INT_BASE + INT_PAR__synctex) as isize))
            .b32
            .s1
            != 0
            && synctex_ctxt.flags.warn() == 0
        {
            synctex_ctxt
                .flags
                .set_warn(1 as ::core::ffi::c_uint as ::core::ffi::c_uint);
            ttstub_issue_warning(
                b"SyncTeX was disabled - changing the value of \\synctex has no effect\0"
                    as *const u8 as *const ::core::ffi::c_char,
            );
        }
        return;
    }
    if !synctex_prepare_content().is_null() {
        synctex_record_pdfxform(p);
    }
}
#[no_mangle]
pub unsafe extern "C" fn synctex_mrofxfdp() {
    if !synctex_ctxt.file.is_null() {
        synctex_record_mrofxfdp();
    }
}
#[no_mangle]
pub unsafe extern "C" fn synctex_pdfrefxform(mut objnum: ::core::ffi::c_int) {
    if !synctex_ctxt.file.is_null() {
        synctex_record_node_pdfrefxform(objnum);
    }
}
#[inline]
unsafe extern "C" fn synctex_record_pdfxform(mut form: int32_t) -> ::core::ffi::c_int {
    if synctex_ctxt.flags.off() as ::core::ffi::c_int != 0
        || (*eqtb.offset((INT_BASE + INT_PAR__synctex) as isize))
            .b32
            .s1
            == 0
        || synctex_ctxt.file.is_null()
    {
        return 0 as ::core::ffi::c_int;
    } else {
        let mut len: ::core::ffi::c_int = 0;
        synctex_ctxt.form_depth += 1;
        len = ttstub_fprintf(
            synctex_ctxt.file,
            b"<%i\n\0" as *const u8 as *const ::core::ffi::c_char,
            synctex_ctxt.form_depth,
        );
        if len > 0 as ::core::ffi::c_int {
            synctex_ctxt.total_length =
                (synctex_ctxt.total_length as ::core::ffi::c_int + len) as int32_t;
            synctex_ctxt.count += 1;
            return 0 as ::core::ffi::c_int;
        }
    }
    synctexabort();
    return -(1 as ::core::ffi::c_int);
}
#[inline]
unsafe extern "C" fn synctex_record_mrofxfdp() -> ::core::ffi::c_int {
    if 0 as ::core::ffi::c_int == synctex_record_anchor() {
        let mut len: ::core::ffi::c_int = 0;
        synctex_ctxt.form_depth -= 1;
        len = ttstub_fprintf(
            synctex_ctxt.file,
            b">\n\0" as *const u8 as *const ::core::ffi::c_char,
        );
        if len > 0 as ::core::ffi::c_int {
            synctex_ctxt.total_length =
                (synctex_ctxt.total_length as ::core::ffi::c_int + len) as int32_t;
            synctex_ctxt.count += 1;
            return 0 as ::core::ffi::c_int;
        }
    }
    synctexabort();
    return -(1 as ::core::ffi::c_int);
}
#[inline]
unsafe extern "C" fn synctex_record_node_pdfrefxform(
    mut objnum: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    synctex_ctxt.curh = (cur_h + 4736287 as scaled_t) as int32_t;
    synctex_ctxt.curv = (cur_v + 4736287 as scaled_t) as int32_t;
    if synctex_ctxt.flags.off() as ::core::ffi::c_int != 0
        || (*eqtb.offset((INT_BASE + INT_PAR__synctex) as isize))
            .b32
            .s1
            == 0
        || synctex_ctxt.file.is_null()
    {
        return 0 as ::core::ffi::c_int;
    } else {
        let mut len: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
        len = ttstub_fprintf(
            synctex_ctxt.file,
            b"f%i:%i,%i\n\0" as *const u8 as *const ::core::ffi::c_char,
            objnum,
            (cur_h + 4736287 as scaled_t) / synctex_ctxt.unit as scaled_t,
            (cur_v + 4736287 as scaled_t) / synctex_ctxt.unit as scaled_t,
        );
        synctex_ctxt.lastv = (cur_v + 4736287 as scaled_t) as int32_t;
        if len > 0 as ::core::ffi::c_int {
            synctex_ctxt.total_length =
                (synctex_ctxt.total_length as ::core::ffi::c_int + len) as int32_t;
            synctex_ctxt.count += 1;
            return 0 as ::core::ffi::c_int;
        }
    }
    synctexabort();
    return -(1 as ::core::ffi::c_int);
}
#[inline]
unsafe extern "C" fn synctex_record_node_void_vlist(mut p: int32_t) {
    let mut len: ::core::ffi::c_int = ttstub_fprintf(
        synctex_ctxt.file,
        b"v%i,%i:%i,%i:%i,%i,%i\n\0" as *const u8 as *const ::core::ffi::c_char,
        (*mem.offset((p + BOX_NODE_SIZE as int32_t - SYNCTEX_FIELD_SIZE as int32_t) as isize))
            .b32
            .s0,
        (*mem.offset((p + BOX_NODE_SIZE as int32_t - SYNCTEX_FIELD_SIZE as int32_t) as isize))
            .b32
            .s1,
        synctex_ctxt.curh / synctex_ctxt.unit,
        synctex_ctxt.curv / synctex_ctxt.unit,
        (*mem.offset((p + width_offset as int32_t) as isize)).b32.s1 / synctex_ctxt.unit,
        (*mem.offset((p + height_offset as int32_t) as isize))
            .b32
            .s1
            / synctex_ctxt.unit,
        (*mem.offset((p + depth_offset as int32_t) as isize)).b32.s1 / synctex_ctxt.unit,
    );
    synctex_ctxt.lastv = (cur_v + 4736287 as scaled_t) as int32_t;
    if len > 0 as ::core::ffi::c_int {
        synctex_ctxt.total_length =
            (synctex_ctxt.total_length as ::core::ffi::c_int + len) as int32_t;
        synctex_ctxt.count += 1;
    } else {
        synctexabort();
    };
}
#[inline]
unsafe extern "C" fn synctex_record_node_vlist(mut p: int32_t) {
    let mut len: ::core::ffi::c_int = 0;
    synctex_ctxt
        .flags
        .set_not_void(1 as ::core::ffi::c_uint as ::core::ffi::c_uint);
    len = ttstub_fprintf(
        synctex_ctxt.file,
        b"[%i,%i:%i,%i:%i,%i,%i\n\0" as *const u8 as *const ::core::ffi::c_char,
        (*mem.offset((p + BOX_NODE_SIZE as int32_t - SYNCTEX_FIELD_SIZE as int32_t) as isize))
            .b32
            .s0,
        (*mem.offset((p + BOX_NODE_SIZE as int32_t - SYNCTEX_FIELD_SIZE as int32_t) as isize))
            .b32
            .s1,
        synctex_ctxt.curh / synctex_ctxt.unit,
        synctex_ctxt.curv / synctex_ctxt.unit,
        (*mem.offset((p + width_offset as int32_t) as isize)).b32.s1 / synctex_ctxt.unit,
        (*mem.offset((p + height_offset as int32_t) as isize))
            .b32
            .s1
            / synctex_ctxt.unit,
        (*mem.offset((p + depth_offset as int32_t) as isize)).b32.s1 / synctex_ctxt.unit,
    );
    synctex_ctxt.lastv = (cur_v + 4736287 as scaled_t) as int32_t;
    if len > 0 as ::core::ffi::c_int {
        synctex_ctxt.total_length =
            (synctex_ctxt.total_length as ::core::ffi::c_int + len) as int32_t;
        synctex_ctxt.count += 1;
    } else {
        synctexabort();
    };
}
#[inline]
unsafe extern "C" fn synctex_record_node_tsilv(mut p: int32_t) {
    let mut len: ::core::ffi::c_int = ttstub_fprintf(
        synctex_ctxt.file,
        b"]\n\0" as *const u8 as *const ::core::ffi::c_char,
    );
    if len > 0 as ::core::ffi::c_int {
        synctex_ctxt.total_length =
            (synctex_ctxt.total_length as ::core::ffi::c_int + len) as int32_t;
    } else {
        synctexabort();
    };
}
#[inline]
unsafe extern "C" fn synctex_record_node_void_hlist(mut p: int32_t) {
    let mut len: ::core::ffi::c_int = ttstub_fprintf(
        synctex_ctxt.file,
        b"h%i,%i:%i,%i:%i,%i,%i\n\0" as *const u8 as *const ::core::ffi::c_char,
        (*mem.offset((p + BOX_NODE_SIZE as int32_t - SYNCTEX_FIELD_SIZE as int32_t) as isize))
            .b32
            .s0,
        (*mem.offset((p + BOX_NODE_SIZE as int32_t - SYNCTEX_FIELD_SIZE as int32_t) as isize))
            .b32
            .s1,
        synctex_ctxt.curh / synctex_ctxt.unit,
        synctex_ctxt.curv / synctex_ctxt.unit,
        (*mem.offset((p + width_offset as int32_t) as isize)).b32.s1 / synctex_ctxt.unit,
        (*mem.offset((p + height_offset as int32_t) as isize))
            .b32
            .s1
            / synctex_ctxt.unit,
        (*mem.offset((p + depth_offset as int32_t) as isize)).b32.s1 / synctex_ctxt.unit,
    );
    synctex_ctxt.lastv = (cur_v + 4736287 as scaled_t) as int32_t;
    if len > 0 as ::core::ffi::c_int {
        synctex_ctxt.total_length =
            (synctex_ctxt.total_length as ::core::ffi::c_int + len) as int32_t;
        synctex_ctxt.count += 1;
    } else {
        synctexabort();
    };
}
#[inline]
unsafe extern "C" fn synctex_record_node_hlist(mut p: int32_t) {
    let mut len: ::core::ffi::c_int = 0;
    synctex_ctxt
        .flags
        .set_not_void(1 as ::core::ffi::c_uint as ::core::ffi::c_uint);
    len = ttstub_fprintf(
        synctex_ctxt.file,
        b"(%i,%i:%i,%i:%i,%i,%i\n\0" as *const u8 as *const ::core::ffi::c_char,
        (*mem.offset((p + BOX_NODE_SIZE as int32_t - SYNCTEX_FIELD_SIZE as int32_t) as isize))
            .b32
            .s0,
        (*mem.offset((p + BOX_NODE_SIZE as int32_t - SYNCTEX_FIELD_SIZE as int32_t) as isize))
            .b32
            .s1,
        synctex_ctxt.curh / synctex_ctxt.unit,
        synctex_ctxt.curv / synctex_ctxt.unit,
        (*mem.offset((p + width_offset as int32_t) as isize)).b32.s1 / synctex_ctxt.unit,
        (*mem.offset((p + height_offset as int32_t) as isize))
            .b32
            .s1
            / synctex_ctxt.unit,
        (*mem.offset((p + depth_offset as int32_t) as isize)).b32.s1 / synctex_ctxt.unit,
    );
    synctex_ctxt.lastv = (cur_v + 4736287 as scaled_t) as int32_t;
    if len > 0 as ::core::ffi::c_int {
        synctex_ctxt.total_length =
            (synctex_ctxt.total_length as ::core::ffi::c_int + len) as int32_t;
        synctex_ctxt.count += 1;
    } else {
        synctexabort();
    };
}
#[inline]
unsafe extern "C" fn synctex_record_node_tsilh(mut p: int32_t) {
    let mut len: ::core::ffi::c_int = ttstub_fprintf(
        synctex_ctxt.file,
        b")\n\0" as *const u8 as *const ::core::ffi::c_char,
    );
    if len > 0 as ::core::ffi::c_int {
        synctex_ctxt.total_length =
            (synctex_ctxt.total_length as ::core::ffi::c_int + len) as int32_t;
        synctex_ctxt.count += 1;
    } else {
        synctexabort();
    };
}
#[inline]
unsafe extern "C" fn synctex_record_count() -> ::core::ffi::c_int {
    let mut len: ::core::ffi::c_int = ttstub_fprintf(
        synctex_ctxt.file,
        b"Count:%i\n\0" as *const u8 as *const ::core::ffi::c_char,
        synctex_ctxt.count,
    );
    if len > 0 as ::core::ffi::c_int {
        synctex_ctxt.total_length =
            (synctex_ctxt.total_length as ::core::ffi::c_int + len) as int32_t;
        return 0 as ::core::ffi::c_int;
    }
    synctexabort();
    return -(1 as ::core::ffi::c_int);
}
#[inline]
unsafe extern "C" fn synctex_record_postamble() -> ::core::ffi::c_int {
    if 0 as ::core::ffi::c_int == synctex_record_anchor() {
        let mut len: ::core::ffi::c_int = ttstub_fprintf(
            synctex_ctxt.file,
            b"Postamble:\n\0" as *const u8 as *const ::core::ffi::c_char,
        );
        if len > 0 as ::core::ffi::c_int {
            synctex_ctxt.total_length =
                (synctex_ctxt.total_length as ::core::ffi::c_int + len) as int32_t;
            if synctex_record_count() == 0 && synctex_record_anchor() == 0 {
                len = ttstub_fprintf(
                    synctex_ctxt.file,
                    b"Post scriptum:\n\0" as *const u8 as *const ::core::ffi::c_char,
                );
                if len > 0 as ::core::ffi::c_int {
                    synctex_ctxt.total_length =
                        (synctex_ctxt.total_length as ::core::ffi::c_int + len) as int32_t;
                    return 0 as ::core::ffi::c_int;
                }
            }
        }
    }
    synctexabort();
    return -(1 as ::core::ffi::c_int);
}
#[inline]
unsafe extern "C" fn synctex_record_node_glue(mut p: int32_t) {
    let mut len: ::core::ffi::c_int = ttstub_fprintf(
        synctex_ctxt.file,
        b"g%i,%i:%i,%i\n\0" as *const u8 as *const ::core::ffi::c_char,
        (*mem.offset((p + GLUE_NODE_SIZE as int32_t - SYNCTEX_FIELD_SIZE as int32_t) as isize))
            .b32
            .s0,
        (*mem.offset((p + GLUE_NODE_SIZE as int32_t - SYNCTEX_FIELD_SIZE as int32_t) as isize))
            .b32
            .s1,
        synctex_ctxt.curh / synctex_ctxt.unit,
        synctex_ctxt.curv / synctex_ctxt.unit,
    );
    synctex_ctxt.lastv = (cur_v + 4736287 as scaled_t) as int32_t;
    if len > 0 as ::core::ffi::c_int {
        synctex_ctxt.total_length =
            (synctex_ctxt.total_length as ::core::ffi::c_int + len) as int32_t;
        synctex_ctxt.count += 1;
    } else {
        synctexabort();
    };
}
#[inline]
unsafe extern "C" fn synctex_record_node_kern(mut p: int32_t) {
    let mut len: ::core::ffi::c_int = ttstub_fprintf(
        synctex_ctxt.file,
        b"k%i,%i:%i,%i:%i\n\0" as *const u8 as *const ::core::ffi::c_char,
        (*mem.offset((p + GLUE_NODE_SIZE as int32_t - SYNCTEX_FIELD_SIZE as int32_t) as isize))
            .b32
            .s0,
        (*mem.offset((p + GLUE_NODE_SIZE as int32_t - SYNCTEX_FIELD_SIZE as int32_t) as isize))
            .b32
            .s1,
        synctex_ctxt.curh / synctex_ctxt.unit,
        synctex_ctxt.curv / synctex_ctxt.unit,
        (*mem.offset((p + width_offset as int32_t) as isize)).b32.s1 / synctex_ctxt.unit,
    );
    synctex_ctxt.lastv = (cur_v + 4736287 as scaled_t) as int32_t;
    if len > 0 as ::core::ffi::c_int {
        synctex_ctxt.total_length =
            (synctex_ctxt.total_length as ::core::ffi::c_int + len) as int32_t;
        synctex_ctxt.count += 1;
    } else {
        synctexabort();
    };
}
#[inline]
unsafe extern "C" fn synctex_record_node_rule(mut p: int32_t) {
    let mut len: ::core::ffi::c_int = ttstub_fprintf(
        synctex_ctxt.file,
        b"r%i,%i:%i,%i:%i,%i,%i\n\0" as *const u8 as *const ::core::ffi::c_char,
        (*mem.offset((p + RULE_NODE_SIZE as int32_t - SYNCTEX_FIELD_SIZE as int32_t) as isize))
            .b32
            .s0,
        (*mem.offset((p + RULE_NODE_SIZE as int32_t - SYNCTEX_FIELD_SIZE as int32_t) as isize))
            .b32
            .s1,
        synctex_ctxt.curh / synctex_ctxt.unit,
        synctex_ctxt.curv / synctex_ctxt.unit,
        rule_wd / synctex_ctxt.unit as scaled_t,
        rule_ht / synctex_ctxt.unit as scaled_t,
        rule_dp / synctex_ctxt.unit as scaled_t,
    );
    synctex_ctxt.lastv = (cur_v + 4736287 as scaled_t) as int32_t;
    if len > 0 as ::core::ffi::c_int {
        synctex_ctxt.total_length =
            (synctex_ctxt.total_length as ::core::ffi::c_int + len) as int32_t;
        synctex_ctxt.count += 1;
    } else {
        synctexabort();
    };
}
unsafe extern "C" fn synctex_record_node_math(mut p: int32_t) {
    let mut len: ::core::ffi::c_int = ttstub_fprintf(
        synctex_ctxt.file,
        b"$%i,%i:%i,%i\n\0" as *const u8 as *const ::core::ffi::c_char,
        (*mem.offset((p + MATH_NODE_SIZE as int32_t - SYNCTEX_FIELD_SIZE as int32_t) as isize))
            .b32
            .s0,
        (*mem.offset((p + MATH_NODE_SIZE as int32_t - SYNCTEX_FIELD_SIZE as int32_t) as isize))
            .b32
            .s1,
        synctex_ctxt.curh / synctex_ctxt.unit,
        synctex_ctxt.curv / synctex_ctxt.unit,
    );
    synctex_ctxt.lastv = (cur_v + 4736287 as scaled_t) as int32_t;
    if len > 0 as ::core::ffi::c_int {
        synctex_ctxt.total_length =
            (synctex_ctxt.total_length as ::core::ffi::c_int + len) as int32_t;
        synctex_ctxt.count += 1;
    } else {
        synctexabort();
    };
}
pub const false_0: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
unsafe extern "C" fn run_static_initializers() {
    synctex_ctxt = C2RustUnnamed {
        file: ::core::ptr::null_mut::<ttbc_output_handle_t>(),
        root_name: ::core::ptr::null_mut::<::core::ffi::c_char>(),
        count: 0 as int32_t,
        node: 0 as int32_t,
        recorder: None,
        tag: 0 as int32_t,
        line: 0 as int32_t,
        curh: 0 as int32_t,
        curv: 0 as int32_t,
        magnification: 0 as int32_t,
        unit: 0 as int32_t,
        total_length: 0 as int32_t,
        lastv: -(1 as int32_t),
        form_depth: 0 as int32_t,
        synctex_tag_counter: 0 as ::core::ffi::c_uint,
        flags: {
            let mut init = _flags {
                content_ready_off_not_void_warn_output_p: [0; 1],
                c2rust_padding: [0; 3],
            };
            init.set_content_ready(0 as ::core::ffi::c_uint);
            init.set_off(0 as ::core::ffi::c_uint);
            init.set_not_void(0 as ::core::ffi::c_uint);
            init.set_warn(0 as ::core::ffi::c_uint);
            init.set_output_p(0 as ::core::ffi::c_uint);
            init
        },
    };
}
#[used]
#[cfg_attr(target_os = "linux", link_section = ".init_array")]
#[cfg_attr(target_os = "windows", link_section = ".CRT$XIB")]
#[cfg_attr(target_os = "macos", link_section = "__DATA,__mod_init_func")]
static INIT_ARRAY: [unsafe extern "C" fn(); 1] = [run_static_initializers];
