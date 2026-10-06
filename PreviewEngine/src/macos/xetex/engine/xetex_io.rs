/* tectonic/xetex-io.c: low-level input/output functions tied to the XeTeX engine
   Copyright 2016-2019 The Tectonic Project
   Licensed under the MIT License.
*/
// Translated from xetex/engine/xetex-io.c with C2Rust 0.22.1.
extern "C" {
    pub type ttbc_input_handle_t;
    pub type ttbc_diagnostic_t;
    pub type Opaque_TECkit_Converter;
    pub type UConverter;
    fn __error() -> *mut ::core::ffi::c_int;
    fn malloc(__size: size_t) -> *mut ::core::ffi::c_void;
    fn calloc(__count: size_t, __size: size_t) -> *mut ::core::ffi::c_void;
    fn free(_: *mut ::core::ffi::c_void);
    fn strlen(__s: *const ::core::ffi::c_char) -> size_t;
    fn strdup(__s1: *const ::core::ffi::c_char) -> *mut ::core::ffi::c_char;
    fn _tt_abort(format: *const ::core::ffi::c_char, ...) -> !;
    fn ttstub_input_open(
        path: *const ::core::ffi::c_char,
        format: ttbc_file_format,
        is_gz: ::core::ffi::c_int,
    ) -> rust_input_handle_t;
    fn ttstub_input_open_primary() -> rust_input_handle_t;
    fn ttstub_get_last_input_abspath(buffer_0: *mut ::core::ffi::c_char, len: size_t) -> ssize_t;
    fn ttstub_input_seek(
        handle: rust_input_handle_t,
        offset: ssize_t,
        whence: ::core::ffi::c_int,
    ) -> size_t;
    fn ttstub_input_getc(handle: rust_input_handle_t) -> ::core::ffi::c_int;
    fn ttstub_input_ungetc(
        handle: rust_input_handle_t,
        ch: ::core::ffi::c_int,
    ) -> ::core::ffi::c_int;
    fn ttstub_input_close(handle: rust_input_handle_t) -> ::core::ffi::c_int;
    fn TECkit_CreateConverter(
        mapping: *mut Byte,
        mappingSize: UInt32,
        mapForward: Byte,
        sourceForm: UInt16,
        targetForm: UInt16,
        converter: *mut TECkit_Converter,
    ) -> TECkit_Status;
    fn TECkit_ResetConverter(converter: TECkit_Converter) -> TECkit_Status;
    fn TECkit_ConvertBuffer(
        converter: TECkit_Converter,
        inBuffer: *const Byte,
        inLength: UInt32,
        inUsed: *mut UInt32,
        outBuffer: *mut Byte,
        outLength: UInt32,
        outUsed: *mut UInt32,
        inputIsComplete: Byte,
    ) -> TECkit_Status;
    fn gettexstring(_: str_number) -> *mut ::core::ffi::c_char;
    static mut eqtb: *mut memory_word;
    static mut name_of_file: *mut ::core::ffi::c_char;
    static mut name_of_file16: *mut UTF16_code;
    static mut name_length: int32_t;
    static mut name_length16: int32_t;
    static mut buffer: *mut UnicodeScalar;
    static mut first: int32_t;
    static mut last: int32_t;
    static mut max_buf_stack: int32_t;
    static mut buf_size: int32_t;
    static mut cur_chr: int32_t;
    static mut cur_val: int32_t;
    static mut read_file: [*mut UFILE; 16];
    static mut read_open: [::core::ffi::c_uchar; 17];
    static mut cur_name: str_number;
    static mut cur_area: str_number;
    static mut cur_ext: str_number;
    static mut name_in_progress: bool;
    static mut stop_at_space: bool;
    fn begin_diagnostic();
    fn end_diagnostic(blank_line: bool);
    fn scan_optional_equals();
    fn scan_four_bit_int();
    fn begin_name();
    fn more_name(c: UTF16_code) -> bool;
    fn end_name();
    fn pack_file_name(n: str_number, a: str_number, e: str_number);
    fn scan_file_name();
    fn bad_utf8_warning();
    fn get_input_normalization_state() -> int32_t;
    fn diagnostic_begin_capture_warning_here() -> *mut ttbc_diagnostic_t;
    fn capture_to_diagnostic(diagnostic: *mut ttbc_diagnostic_t);
    fn print_char(s: int32_t);
    fn print_cstr(s: *const ::core::ffi::c_char);
    fn print_nl(s: str_number);
    fn print_nl_cstr(s: *const ::core::ffi::c_char);
    fn print_int(n: int32_t);
    #[link_name = concat!("ucnv_open_", env!("PITEX_PREVIEW_ICU_MAJOR"))]
    fn ucnv_open_78(
        converterName: *const ::core::ffi::c_char,
        err: *mut UErrorCode,
    ) -> *mut UConverter;
    #[link_name = concat!("ucnv_close_", env!("PITEX_PREVIEW_ICU_MAJOR"))]
    fn ucnv_close_78(converter: *mut UConverter);
    #[link_name = concat!("ucnv_toAlgorithmic_", env!("PITEX_PREVIEW_ICU_MAJOR"))]
    fn ucnv_toAlgorithmic_78(
        algorithmicType: UConverterType,
        cnv: *mut UConverter,
        target: *mut ::core::ffi::c_char,
        targetCapacity: int32_t,
        source: *const ::core::ffi::c_char,
        sourceLength: int32_t,
        pErrorCode: *mut UErrorCode,
    ) -> int32_t;
}
pub type __darwin_size_t = usize;
pub type __darwin_ssize_t = isize;
pub type size_t = __darwin_size_t;
pub type int32_t = i32;
pub type uint8_t = u8;
pub type uint16_t = u16;
pub type uint32_t = u32;
pub type ssize_t = __darwin_ssize_t;
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
pub type UErrorCode = ::core::ffi::c_int;
pub const U_ERROR_LIMIT: UErrorCode = 66818;
pub const U_PLUGIN_ERROR_LIMIT: UErrorCode = 66818;
pub const U_PLUGIN_DIDNT_SET_LEVEL: UErrorCode = 66817;
pub const U_PLUGIN_TOO_HIGH: UErrorCode = 66816;
pub const U_PLUGIN_ERROR_START: UErrorCode = 66816;
pub const U_STRINGPREP_CHECK_BIDI_ERROR: UErrorCode = 66562;
pub const U_STRINGPREP_UNASSIGNED_ERROR: UErrorCode = 66561;
pub const U_STRINGPREP_PROHIBITED_ERROR: UErrorCode = 66560;
pub const U_IDNA_ERROR_LIMIT: UErrorCode = 66569;
pub const U_IDNA_DOMAIN_NAME_TOO_LONG_ERROR: UErrorCode = 66568;
pub const U_IDNA_ZERO_LENGTH_LABEL_ERROR: UErrorCode = 66567;
pub const U_IDNA_LABEL_TOO_LONG_ERROR: UErrorCode = 66566;
pub const U_IDNA_VERIFICATION_ERROR: UErrorCode = 66565;
pub const U_IDNA_ACE_PREFIX_ERROR: UErrorCode = 66564;
pub const U_IDNA_STD3_ASCII_RULES_ERROR: UErrorCode = 66563;
pub const U_IDNA_CHECK_BIDI_ERROR: UErrorCode = 66562;
pub const U_IDNA_UNASSIGNED_ERROR: UErrorCode = 66561;
pub const U_IDNA_ERROR_START: UErrorCode = 66560;
pub const U_IDNA_PROHIBITED_ERROR: UErrorCode = 66560;
pub const U_REGEX_ERROR_LIMIT: UErrorCode = 66326;
pub const U_REGEX_INVALID_CAPTURE_GROUP_NAME: UErrorCode = 66325;
pub const U_REGEX_PATTERN_TOO_BIG: UErrorCode = 66324;
pub const U_REGEX_STOPPED_BY_CALLER: UErrorCode = 66323;
pub const U_REGEX_TIME_OUT: UErrorCode = 66322;
pub const U_REGEX_STACK_OVERFLOW: UErrorCode = 66321;
pub const U_REGEX_INVALID_RANGE: UErrorCode = 66320;
pub const U_REGEX_MISSING_CLOSE_BRACKET: UErrorCode = 66319;
pub const U_REGEX_OCTAL_TOO_BIG: UErrorCode = 66318;
pub const U_REGEX_SET_CONTAINS_STRING: UErrorCode = 66317;
pub const U_REGEX_LOOK_BEHIND_LIMIT: UErrorCode = 66316;
pub const U_REGEX_INVALID_FLAG: UErrorCode = 66315;
pub const U_REGEX_INVALID_BACK_REF: UErrorCode = 66314;
pub const U_REGEX_MAX_LT_MIN: UErrorCode = 66313;
pub const U_REGEX_BAD_INTERVAL: UErrorCode = 66312;
pub const U_REGEX_NUMBER_TOO_BIG: UErrorCode = 66311;
pub const U_REGEX_MISMATCHED_PAREN: UErrorCode = 66310;
pub const U_REGEX_UNIMPLEMENTED: UErrorCode = 66309;
pub const U_REGEX_PROPERTY_SYNTAX: UErrorCode = 66308;
pub const U_REGEX_BAD_ESCAPE_SEQUENCE: UErrorCode = 66307;
pub const U_REGEX_INVALID_STATE: UErrorCode = 66306;
pub const U_REGEX_RULE_SYNTAX: UErrorCode = 66305;
pub const U_REGEX_ERROR_START: UErrorCode = 66304;
pub const U_REGEX_INTERNAL_ERROR: UErrorCode = 66304;
pub const U_BRK_ERROR_LIMIT: UErrorCode = 66062;
pub const U_BRK_MALFORMED_RULE_TAG: UErrorCode = 66061;
pub const U_BRK_UNRECOGNIZED_OPTION: UErrorCode = 66060;
pub const U_BRK_RULE_EMPTY_SET: UErrorCode = 66059;
pub const U_BRK_INIT_ERROR: UErrorCode = 66058;
pub const U_BRK_UNDEFINED_VARIABLE: UErrorCode = 66057;
pub const U_BRK_NEW_LINE_IN_QUOTED_STRING: UErrorCode = 66056;
pub const U_BRK_MISMATCHED_PAREN: UErrorCode = 66055;
pub const U_BRK_VARIABLE_REDFINITION: UErrorCode = 66054;
pub const U_BRK_ASSIGN_ERROR: UErrorCode = 66053;
pub const U_BRK_UNCLOSED_SET: UErrorCode = 66052;
pub const U_BRK_RULE_SYNTAX: UErrorCode = 66051;
pub const U_BRK_SEMICOLON_EXPECTED: UErrorCode = 66050;
pub const U_BRK_HEX_DIGITS_EXPECTED: UErrorCode = 66049;
pub const U_BRK_ERROR_START: UErrorCode = 66048;
pub const U_BRK_INTERNAL_ERROR: UErrorCode = 66048;
pub const U_FMT_PARSE_ERROR_LIMIT: UErrorCode = 65825;
pub const U_MF_BAD_OPTION: UErrorCode = 65824;
pub const U_MF_DUPLICATE_VARIANT_ERROR: UErrorCode = 65823;
pub const U_MF_OPERAND_MISMATCH_ERROR: UErrorCode = 65822;
pub const U_MF_DUPLICATE_DECLARATION_ERROR: UErrorCode = 65821;
pub const U_MF_MISSING_SELECTOR_ANNOTATION_ERROR: UErrorCode = 65820;
pub const U_MF_SELECTOR_ERROR: UErrorCode = 65819;
pub const U_MF_DUPLICATE_OPTION_NAME_ERROR: UErrorCode = 65818;
pub const U_MF_NONEXHAUSTIVE_PATTERN_ERROR: UErrorCode = 65817;
pub const U_MF_FORMATTING_ERROR: UErrorCode = 65816;
pub const U_MF_VARIANT_KEY_MISMATCH_ERROR: UErrorCode = 65815;
pub const U_MF_UNKNOWN_FUNCTION_ERROR: UErrorCode = 65814;
pub const U_MF_SYNTAX_ERROR: UErrorCode = 65813;
pub const U_MF_UNRESOLVED_VARIABLE_ERROR: UErrorCode = 65812;
pub const U_NUMBER_SKELETON_SYNTAX_ERROR: UErrorCode = 65811;
pub const U_NUMBER_ARG_OUTOFBOUNDS_ERROR: UErrorCode = 65810;
pub const U_FORMAT_INEXACT_ERROR: UErrorCode = 65809;
pub const U_DECIMAL_NUMBER_SYNTAX_ERROR: UErrorCode = 65808;
pub const U_DEFAULT_KEYWORD_MISSING: UErrorCode = 65807;
pub const U_UNDEFINED_KEYWORD: UErrorCode = 65806;
pub const U_DUPLICATE_KEYWORD: UErrorCode = 65805;
pub const U_ARGUMENT_TYPE_MISMATCH: UErrorCode = 65804;
pub const U_UNSUPPORTED_ATTRIBUTE: UErrorCode = 65803;
pub const U_UNSUPPORTED_PROPERTY: UErrorCode = 65802;
pub const U_UNMATCHED_BRACES: UErrorCode = 65801;
pub const U_ILLEGAL_PAD_POSITION: UErrorCode = 65800;
pub const U_PATTERN_SYNTAX_ERROR: UErrorCode = 65799;
pub const U_MULTIPLE_PAD_SPECIFIERS: UErrorCode = 65798;
pub const U_MULTIPLE_PERMILL_SYMBOLS: UErrorCode = 65797;
pub const U_MULTIPLE_PERCENT_SYMBOLS: UErrorCode = 65796;
pub const U_MALFORMED_EXPONENTIAL_PATTERN: UErrorCode = 65795;
pub const U_MULTIPLE_EXPONENTIAL_SYMBOLS: UErrorCode = 65794;
pub const U_MULTIPLE_DECIMAL_SEPERATORS: UErrorCode = 65793;
pub const U_MULTIPLE_DECIMAL_SEPARATORS: UErrorCode = 65793;
pub const U_FMT_PARSE_ERROR_START: UErrorCode = 65792;
pub const U_UNEXPECTED_TOKEN: UErrorCode = 65792;
pub const U_PARSE_ERROR_LIMIT: UErrorCode = 65571;
pub const U_INVALID_FUNCTION: UErrorCode = 65570;
pub const U_INVALID_ID: UErrorCode = 65569;
pub const U_INTERNAL_TRANSLITERATOR_ERROR: UErrorCode = 65568;
pub const U_ILLEGAL_CHARACTER: UErrorCode = 65567;
pub const U_VARIABLE_RANGE_OVERLAP: UErrorCode = 65566;
pub const U_VARIABLE_RANGE_EXHAUSTED: UErrorCode = 65565;
pub const U_ILLEGAL_CHAR_IN_SEGMENT: UErrorCode = 65564;
pub const U_UNCLOSED_SEGMENT: UErrorCode = 65563;
pub const U_MALFORMED_PRAGMA: UErrorCode = 65562;
pub const U_INVALID_PROPERTY_PATTERN: UErrorCode = 65561;
pub const U_INVALID_RBT_SYNTAX: UErrorCode = 65560;
pub const U_MULTIPLE_COMPOUND_FILTERS: UErrorCode = 65559;
pub const U_MISPLACED_COMPOUND_FILTER: UErrorCode = 65558;
pub const U_RULE_MASK_ERROR: UErrorCode = 65557;
pub const U_UNTERMINATED_QUOTE: UErrorCode = 65556;
pub const U_UNQUOTED_SPECIAL: UErrorCode = 65555;
pub const U_UNDEFINED_VARIABLE: UErrorCode = 65554;
pub const U_UNDEFINED_SEGMENT_REFERENCE: UErrorCode = 65553;
pub const U_TRAILING_BACKSLASH: UErrorCode = 65552;
pub const U_MULTIPLE_POST_CONTEXTS: UErrorCode = 65551;
pub const U_MULTIPLE_CURSORS: UErrorCode = 65550;
pub const U_MULTIPLE_ANTE_CONTEXTS: UErrorCode = 65549;
pub const U_MISSING_SEGMENT_CLOSE: UErrorCode = 65548;
pub const U_MISSING_OPERATOR: UErrorCode = 65547;
pub const U_MISPLACED_QUANTIFIER: UErrorCode = 65546;
pub const U_MISPLACED_CURSOR_OFFSET: UErrorCode = 65545;
pub const U_MISPLACED_ANCHOR_START: UErrorCode = 65544;
pub const U_MISMATCHED_SEGMENT_DELIMITERS: UErrorCode = 65543;
pub const U_MALFORMED_VARIABLE_REFERENCE: UErrorCode = 65542;
pub const U_MALFORMED_VARIABLE_DEFINITION: UErrorCode = 65541;
pub const U_MALFORMED_UNICODE_ESCAPE: UErrorCode = 65540;
pub const U_MALFORMED_SYMBOL_REFERENCE: UErrorCode = 65539;
pub const U_MALFORMED_SET: UErrorCode = 65538;
pub const U_MALFORMED_RULE: UErrorCode = 65537;
pub const U_PARSE_ERROR_START: UErrorCode = 65536;
pub const U_BAD_VARIABLE_DEFINITION: UErrorCode = 65536;
pub const U_STANDARD_ERROR_LIMIT: UErrorCode = 32;
pub const U_INPUT_TOO_LONG_ERROR: UErrorCode = 31;
pub const U_NO_WRITE_PERMISSION: UErrorCode = 30;
pub const U_USELESS_COLLATOR_ERROR: UErrorCode = 29;
pub const U_COLLATOR_VERSION_MISMATCH: UErrorCode = 28;
pub const U_INVALID_STATE_ERROR: UErrorCode = 27;
pub const U_INVARIANT_CONVERSION_ERROR: UErrorCode = 26;
pub const U_ENUM_OUT_OF_SYNC_ERROR: UErrorCode = 25;
pub const U_TOO_MANY_ALIASES_ERROR: UErrorCode = 24;
pub const U_STATE_TOO_OLD_ERROR: UErrorCode = 23;
pub const U_PRIMARY_TOO_LONG_ERROR: UErrorCode = 22;
pub const U_CE_NOT_FOUND_ERROR: UErrorCode = 21;
pub const U_NO_SPACE_AVAILABLE: UErrorCode = 20;
pub const U_UNSUPPORTED_ESCAPE_SEQUENCE: UErrorCode = 19;
pub const U_ILLEGAL_ESCAPE_SEQUENCE: UErrorCode = 18;
pub const U_RESOURCE_TYPE_MISMATCH: UErrorCode = 17;
pub const U_UNSUPPORTED_ERROR: UErrorCode = 16;
pub const U_BUFFER_OVERFLOW_ERROR: UErrorCode = 15;
pub const U_INVALID_TABLE_FILE: UErrorCode = 14;
pub const U_INVALID_TABLE_FORMAT: UErrorCode = 13;
pub const U_ILLEGAL_CHAR_FOUND: UErrorCode = 12;
pub const U_TRUNCATED_CHAR_FOUND: UErrorCode = 11;
pub const U_INVALID_CHAR_FOUND: UErrorCode = 10;
pub const U_PARSE_ERROR: UErrorCode = 9;
pub const U_INDEX_OUTOFBOUNDS_ERROR: UErrorCode = 8;
pub const U_MEMORY_ALLOCATION_ERROR: UErrorCode = 7;
pub const U_MESSAGE_PARSE_ERROR: UErrorCode = 6;
pub const U_INTERNAL_PROGRAM_ERROR: UErrorCode = 5;
pub const U_FILE_ACCESS_ERROR: UErrorCode = 4;
pub const U_INVALID_FORMAT_ERROR: UErrorCode = 3;
pub const U_MISSING_RESOURCE_ERROR: UErrorCode = 2;
pub const U_ILLEGAL_ARGUMENT_ERROR: UErrorCode = 1;
pub const U_ZERO_ERROR: UErrorCode = 0;
pub const U_ERROR_WARNING_LIMIT: UErrorCode = -119;
pub const U_PLUGIN_CHANGED_LEVEL_WARNING: UErrorCode = -120;
pub const U_DIFFERENT_UCA_VERSION: UErrorCode = -121;
pub const U_AMBIGUOUS_ALIAS_WARNING: UErrorCode = -122;
pub const U_SORT_KEY_TOO_SHORT_WARNING: UErrorCode = -123;
pub const U_STRING_NOT_TERMINATED_WARNING: UErrorCode = -124;
pub const U_STATE_OLD_WARNING: UErrorCode = -125;
pub const U_SAFECLONE_ALLOCATED_WARNING: UErrorCode = -126;
pub const U_USING_DEFAULT_WARNING: UErrorCode = -127;
pub const U_ERROR_WARNING_START: UErrorCode = -128;
pub const U_USING_FALLBACK_WARNING: UErrorCode = -128;
pub type UInt8 = ::core::ffi::c_uchar;
pub type UInt16 = ::core::ffi::c_ushort;
pub type UInt32 = ::core::ffi::c_uint;
pub type Byte = UInt8;
pub type TECkit_Status = ::core::ffi::c_long;
pub type TECkit_Converter = *mut Opaque_TECkit_Converter;
pub type UTF16_code = ::core::ffi::c_ushort;
pub type UnicodeScalar = int32_t;
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
pub struct UFILE {
    pub handle: rust_input_handle_t,
    pub savedChar: ::core::ffi::c_long,
    pub skipNextLF: ::core::ffi::c_short,
    pub encodingMode: ::core::ffi::c_short,
    pub conversionData: *mut ::core::ffi::c_void,
}
pub type UConverterType = ::core::ffi::c_int;
pub const UCNV_NUMBER_OF_SUPPORTED_CONVERTER_TYPES: UConverterType = 34;
pub const UCNV_COMPOUND_TEXT: UConverterType = 33;
pub const UCNV_IMAP_MAILBOX: UConverterType = 32;
pub const UCNV_CESU8: UConverterType = 31;
pub const UCNV_UTF32: UConverterType = 30;
pub const UCNV_UTF16: UConverterType = 29;
pub const UCNV_BOCU1: UConverterType = 28;
pub const UCNV_UTF7: UConverterType = 27;
pub const UCNV_US_ASCII: UConverterType = 26;
pub const UCNV_ISCII: UConverterType = 25;
pub const UCNV_SCSU: UConverterType = 24;
pub const UCNV_HZ: UConverterType = 23;
pub const UCNV_LMBCS_LAST: UConverterType = 22;
pub const UCNV_LMBCS_19: UConverterType = 22;
pub const UCNV_LMBCS_18: UConverterType = 21;
pub const UCNV_LMBCS_17: UConverterType = 20;
pub const UCNV_LMBCS_16: UConverterType = 19;
pub const UCNV_LMBCS_11: UConverterType = 18;
pub const UCNV_LMBCS_8: UConverterType = 17;
pub const UCNV_LMBCS_6: UConverterType = 16;
pub const UCNV_LMBCS_5: UConverterType = 15;
pub const UCNV_LMBCS_4: UConverterType = 14;
pub const UCNV_LMBCS_3: UConverterType = 13;
pub const UCNV_LMBCS_2: UConverterType = 12;
pub const UCNV_LMBCS_1: UConverterType = 11;
pub const UCNV_ISO_2022: UConverterType = 10;
pub const UCNV_EBCDIC_STATEFUL: UConverterType = 9;
pub const UCNV_UTF32_LittleEndian: UConverterType = 8;
pub const UCNV_UTF32_BigEndian: UConverterType = 7;
pub const UCNV_UTF16_LittleEndian: UConverterType = 6;
pub const UCNV_UTF16_BigEndian: UConverterType = 5;
pub const UCNV_UTF8: UConverterType = 4;
pub const UCNV_LATIN_1: UConverterType = 3;
pub const UCNV_MBCS: UConverterType = 2;
pub const UCNV_DBCS: UConverterType = 1;
pub const UCNV_SBCS: UConverterType = 0;
pub const UCNV_UNSUPPORTED_CONVERTER: UConverterType = -1;
pub const __DARWIN_NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
pub const EINTR: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
pub const NULL: *mut ::core::ffi::c_void = __DARWIN_NULL;
pub const JUST_OPEN: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const CLOSED: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const NATIVE_UTF32: ::core::ffi::c_int = kForm_UTF32LE;
pub const kStatus_NoError: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const kForm_UTF32LE: ::core::ffi::c_int = 6 as ::core::ffi::c_int;
pub const kForm_NFC: ::core::ffi::c_int = 0x100 as ::core::ffi::c_int;
pub const kForm_NFD: ::core::ffi::c_int = 0x200 as ::core::ffi::c_int;
pub const INT_PAR__xetex_default_input_mode: ::core::ffi::c_int = 75 as ::core::ffi::c_int;
pub const INT_PAR__xetex_default_input_encoding: ::core::ffi::c_int = 76 as ::core::ffi::c_int;
pub const INT_BASE: ::core::ffi::c_int = 7826729 as ::core::ffi::c_int;
pub const SEEK_SET: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
#[inline]
unsafe extern "C" fn print_c_string(mut str: *const ::core::ffi::c_char) {
    while *str != 0 {
        let fresh0 = str;
        str = str.offset(1);
        print_char(*fresh0 as int32_t);
    }
}
pub const AUTO: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const UTF8: ::core::ffi::c_int = 1;
pub const UTF16BE: ::core::ffi::c_int = 2;
pub const UTF16LE: ::core::ffi::c_int = 3;
pub const RAW: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
pub const ICUMAPPING: ::core::ffi::c_int = 5 as ::core::ffi::c_int;
pub const EOF: ::core::ffi::c_int = -(1 as ::core::ffi::c_int);
#[no_mangle]
pub static mut name_of_input_file: *mut ::core::ffi::c_char =
    ::core::ptr::null::<::core::ffi::c_char>() as *mut ::core::ffi::c_char;
#[no_mangle]
pub static mut abspath_of_input_file: [::core::ffi::c_char; 1024] = unsafe {
    ::core::mem::transmute::<
        [u8; 1024],
        [::core::ffi::c_char; 1024],
    >(
        *b"\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0",
    )
};
#[no_mangle]
pub unsafe extern "C" fn tt_xetex_open_input(
    mut filefmt: ::core::ffi::c_int,
) -> rust_input_handle_t {
    let mut handle: rust_input_handle_t = ::core::ptr::null_mut::<ttbc_input_handle_t>();
    if filefmt == TTBC_FILE_FORMAT_TECTONIC_PRIMARY as ::core::ffi::c_int {
        handle = ttstub_input_open_primary();
    } else if *name_of_file.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
        == '|' as i32
    {
        print_nl_cstr(b"Warning: \0" as *const u8 as *const ::core::ffi::c_char);
        diagnostic_begin_capture_warning_here();
        print_cstr(
            b"piped inputs from external commands are not implemented in Tectonic\0" as *const u8
                as *const ::core::ffi::c_char,
        );
        capture_to_diagnostic(::core::ptr::null_mut::<ttbc_diagnostic_t>());
        return ::core::ptr::null_mut::<ttbc_input_handle_t>();
    } else {
        handle = ttstub_input_open(
            name_of_file,
            filefmt as ttbc_file_format,
            0 as ::core::ffi::c_int,
        );
    }
    if handle.is_null() {
        return ::core::ptr::null_mut::<ttbc_input_handle_t>();
    }
    if ttstub_get_last_input_abspath(
        &raw mut abspath_of_input_file as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 1024]>() as size_t,
    ) < 1 as ssize_t
    {
        abspath_of_input_file[0 as ::core::ffi::c_int as usize] =
            '\0' as i32 as ::core::ffi::c_char;
    }
    name_length = strlen(name_of_file) as int32_t;
    free(name_of_input_file as *mut ::core::ffi::c_void);
    name_of_input_file = strdup(name_of_file);
    return handle;
}
#[no_mangle]
pub static mut offsetsFromUTF8: [uint32_t; 6] = [
    0 as ::core::ffi::c_ulong as uint32_t,
    0x3080 as ::core::ffi::c_ulong as uint32_t,
    0xe2080 as ::core::ffi::c_ulong as uint32_t,
    0x3c82080 as ::core::ffi::c_ulong as uint32_t,
    0xfa082080 as ::core::ffi::c_ulong as uint32_t,
    0x82082080 as ::core::ffi::c_ulong as uint32_t,
];
#[no_mangle]
pub static mut bytesFromUTF8: [uint8_t; 256] = [
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    2 as ::core::ffi::c_int as uint8_t,
    2 as ::core::ffi::c_int as uint8_t,
    2 as ::core::ffi::c_int as uint8_t,
    2 as ::core::ffi::c_int as uint8_t,
    2 as ::core::ffi::c_int as uint8_t,
    2 as ::core::ffi::c_int as uint8_t,
    2 as ::core::ffi::c_int as uint8_t,
    2 as ::core::ffi::c_int as uint8_t,
    2 as ::core::ffi::c_int as uint8_t,
    2 as ::core::ffi::c_int as uint8_t,
    2 as ::core::ffi::c_int as uint8_t,
    2 as ::core::ffi::c_int as uint8_t,
    2 as ::core::ffi::c_int as uint8_t,
    2 as ::core::ffi::c_int as uint8_t,
    2 as ::core::ffi::c_int as uint8_t,
    2 as ::core::ffi::c_int as uint8_t,
    3 as ::core::ffi::c_int as uint8_t,
    3 as ::core::ffi::c_int as uint8_t,
    3 as ::core::ffi::c_int as uint8_t,
    3 as ::core::ffi::c_int as uint8_t,
    3 as ::core::ffi::c_int as uint8_t,
    3 as ::core::ffi::c_int as uint8_t,
    3 as ::core::ffi::c_int as uint8_t,
    3 as ::core::ffi::c_int as uint8_t,
    4 as ::core::ffi::c_int as uint8_t,
    4 as ::core::ffi::c_int as uint8_t,
    4 as ::core::ffi::c_int as uint8_t,
    4 as ::core::ffi::c_int as uint8_t,
    5 as ::core::ffi::c_int as uint8_t,
    5 as ::core::ffi::c_int as uint8_t,
    5 as ::core::ffi::c_int as uint8_t,
    5 as ::core::ffi::c_int as uint8_t,
];
#[no_mangle]
pub static mut firstByteMark: [uint8_t; 7] = [
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0xc0 as ::core::ffi::c_int as uint8_t,
    0xe0 as ::core::ffi::c_int as uint8_t,
    0xf0 as ::core::ffi::c_int as uint8_t,
    0xf8 as ::core::ffi::c_int as uint8_t,
    0xfc as ::core::ffi::c_int as uint8_t,
];
#[no_mangle]
pub unsafe extern "C" fn set_input_file_encoding(
    mut f: *mut UFILE,
    mut mode: int32_t,
    mut encodingData: int32_t,
) {
    if (*f).encodingMode as ::core::ffi::c_int == ICUMAPPING && !(*f).conversionData.is_null() {
        ucnv_close_78((*f).conversionData as *mut UConverter);
    }
    (*f).conversionData = ::core::ptr::null_mut::<::core::ffi::c_void>();
    match mode {
        UTF8 | UTF16BE | UTF16LE | RAW => {
            (*f).encodingMode = mode as ::core::ffi::c_short;
        }
        ICUMAPPING => {
            let mut name: *mut ::core::ffi::c_char = gettexstring(encodingData as str_number);
            let mut err: UErrorCode = U_ZERO_ERROR;
            let mut cnv: *mut UConverter = ucnv_open_78(name, &raw mut err);
            if cnv.is_null() {
                begin_diagnostic();
                print_nl('E' as i32);
                print_c_string(b"rror \0" as *const u8 as *const ::core::ffi::c_char);
                print_int(err as int32_t);
                print_c_string(
                    b" creating Unicode converter for `\0" as *const u8
                        as *const ::core::ffi::c_char,
                );
                print_c_string(name);
                print_c_string(
                    b"'; reading as raw bytes\0" as *const u8 as *const ::core::ffi::c_char,
                );
                end_diagnostic(1 as ::core::ffi::c_int != 0);
                (*f).encodingMode = RAW as ::core::ffi::c_short;
            } else {
                (*f).encodingMode = ICUMAPPING as ::core::ffi::c_short;
                (*f).conversionData = cnv as *mut ::core::ffi::c_void;
            }
            free(name as *mut ::core::ffi::c_void);
        }
        _ => {}
    };
}
#[no_mangle]
pub unsafe extern "C" fn u_open_in(
    mut f: *mut *mut UFILE,
    mut filefmt: int32_t,
    mut fopen_mode: *const ::core::ffi::c_char,
    mut mode: int32_t,
    mut encodingData: int32_t,
) -> ::core::ffi::c_int {
    let mut handle: rust_input_handle_t = ::core::ptr::null_mut::<ttbc_input_handle_t>();
    let mut B1: ::core::ffi::c_int = 0;
    let mut B2: ::core::ffi::c_int = 0;
    handle = tt_xetex_open_input(filefmt as ::core::ffi::c_int);
    if handle.is_null() {
        return 0 as ::core::ffi::c_int;
    }
    *f = malloc(::core::mem::size_of::<UFILE>() as size_t) as *mut UFILE;
    (**f).encodingMode = 0 as ::core::ffi::c_short;
    (**f).conversionData = ::core::ptr::null_mut::<::core::ffi::c_void>();
    (**f).savedChar = -(1 as ::core::ffi::c_int) as ::core::ffi::c_long;
    (**f).skipNextLF = 0 as ::core::ffi::c_short;
    (**f).handle = handle;
    if mode == AUTO as int32_t {
        B1 = ttstub_input_getc((**f).handle);
        B2 = ttstub_input_getc((**f).handle);
        if B1 == 0xfe as ::core::ffi::c_int && B2 == 0xff as ::core::ffi::c_int {
            mode = UTF16BE as int32_t;
        } else if B2 == 0xfe as ::core::ffi::c_int && B1 == 0xff as ::core::ffi::c_int {
            mode = UTF16LE as int32_t;
        } else if B1 == 0 as ::core::ffi::c_int && B2 != 0 as ::core::ffi::c_int {
            mode = UTF16BE as int32_t;
            ttstub_input_seek((**f).handle, 0 as ssize_t, SEEK_SET);
        } else if B2 == 0 as ::core::ffi::c_int && B1 != 0 as ::core::ffi::c_int {
            mode = UTF16LE as int32_t;
            ttstub_input_seek((**f).handle, 0 as ssize_t, SEEK_SET);
        } else if B1 == 0xef as ::core::ffi::c_int && B2 == 0xbb as ::core::ffi::c_int {
            let mut B3: ::core::ffi::c_int = ttstub_input_getc((**f).handle);
            if B3 == 0xbf as ::core::ffi::c_int {
                mode = UTF8 as int32_t;
            }
        }
        if mode == AUTO as int32_t {
            ttstub_input_seek((**f).handle, 0 as ssize_t, SEEK_SET);
            mode = UTF8 as int32_t;
        }
    }
    set_input_file_encoding(*f, mode, encodingData);
    return 1 as ::core::ffi::c_int;
}
unsafe extern "C" fn buffer_overflow() {
    _tt_abort(
        b"unable to read an entire line (buf_size=%u)\0" as *const u8 as *const ::core::ffi::c_char,
        buf_size as ::core::ffi::c_uint,
    );
}
unsafe extern "C" fn conversion_error(mut errcode: ::core::ffi::c_int) {
    begin_diagnostic();
    print_nl('U' as i32);
    print_c_string(
        b"nicode conversion failed (ICU error code = \0" as *const u8 as *const ::core::ffi::c_char,
    );
    print_int(errcode as int32_t);
    print_c_string(b") discarding any remaining text\0" as *const u8 as *const ::core::ffi::c_char);
    end_diagnostic(1 as ::core::ffi::c_int != 0);
}
unsafe extern "C" fn apply_normalization(
    mut buf: *mut uint32_t,
    mut len: ::core::ffi::c_int,
    mut norm: ::core::ffi::c_int,
) {
    static mut normalizers: [TECkit_Converter; 2] = [
        ::core::ptr::null::<Opaque_TECkit_Converter>() as *mut Opaque_TECkit_Converter,
        ::core::ptr::null::<Opaque_TECkit_Converter>() as *mut Opaque_TECkit_Converter,
    ];
    let mut status: TECkit_Status = 0;
    let mut inUsed: UInt32 = 0;
    let mut outUsed: UInt32 = 0;
    let mut normPtr: *mut TECkit_Converter = (&raw mut normalizers as *mut TECkit_Converter)
        .offset((norm - 1 as ::core::ffi::c_int) as isize)
        as *mut TECkit_Converter;
    if (*normPtr).is_null() {
        status = TECkit_CreateConverter(
            ::core::ptr::null_mut::<Byte>(),
            0 as UInt32,
            1 as Byte,
            NATIVE_UTF32 as UInt16,
            (NATIVE_UTF32
                | (if norm == 1 as ::core::ffi::c_int {
                    kForm_NFC
                } else {
                    kForm_NFD
                })) as UInt16,
            normPtr,
        );
        if status != kStatus_NoError as TECkit_Status {
            _tt_abort(
                b"failed to create normalizer: error code = %d\0" as *const u8
                    as *const ::core::ffi::c_char,
                status as ::core::ffi::c_int,
            );
        }
    }
    status = TECkit_ConvertBuffer(
        *normPtr,
        buf as *mut Byte,
        (len as usize).wrapping_mul(::core::mem::size_of::<UInt32>() as usize) as UInt32,
        &raw mut inUsed,
        buffer.offset(first as isize) as *mut UnicodeScalar as *mut Byte,
        (::core::mem::size_of::<UnicodeScalar>() as usize).wrapping_mul((buf_size - first) as usize)
            as UInt32,
        &raw mut outUsed,
        1 as Byte,
    );
    TECkit_ResetConverter(*normPtr);
    if status != kStatus_NoError as TECkit_Status {
        buffer_overflow();
    }
    last = (first as usize).wrapping_add(
        (outUsed as usize).wrapping_div(::core::mem::size_of::<UnicodeScalar>() as usize),
    ) as int32_t;
}
#[no_mangle]
pub unsafe extern "C" fn input_line(mut f: *mut UFILE) -> ::core::ffi::c_int {
    static mut byteBuffer: *mut ::core::ffi::c_char =
        ::core::ptr::null::<::core::ffi::c_char>() as *mut ::core::ffi::c_char;
    static mut utf32Buf: *mut uint32_t = ::core::ptr::null::<uint32_t>() as *mut uint32_t;
    let mut i: ::core::ffi::c_int = 0;
    let mut tmpLen: ::core::ffi::c_int = 0;
    let mut norm: ::core::ffi::c_int = get_input_normalization_state() as ::core::ffi::c_int;
    if (*f).handle.is_null() {
        _tt_abort(
            b"reads from synthetic \"terminal\" file #0 should never happen\0" as *const u8
                as *const ::core::ffi::c_char,
        );
    }
    last = first;
    if (*f).encodingMode as ::core::ffi::c_int == ICUMAPPING {
        let mut bytesRead: uint32_t = 0 as uint32_t;
        let mut cnv: *mut UConverter = ::core::ptr::null_mut::<UConverter>();
        let mut outLen: ::core::ffi::c_int = 0;
        let mut errorCode: UErrorCode = U_ZERO_ERROR;
        if byteBuffer.is_null() {
            byteBuffer = malloc((buf_size + 1 as int32_t) as size_t) as *mut ::core::ffi::c_char;
        }
        i = ttstub_input_getc((*f).handle);
        if (*f).skipNextLF != 0 {
            (*f).skipNextLF = 0 as ::core::ffi::c_short;
            if i == '\n' as i32 {
                i = ttstub_input_getc((*f).handle);
            }
        }
        if i != EOF && i != '\n' as i32 && i != '\r' as i32 {
            let fresh1 = bytesRead;
            bytesRead = bytesRead.wrapping_add(1);
            *byteBuffer.offset(fresh1 as isize) = i as ::core::ffi::c_char;
        }
        if i != EOF && i != '\n' as i32 && i != '\r' as i32 {
            while bytesRead < buf_size as uint32_t
                && {
                    i = ttstub_input_getc((*f).handle);
                    i != EOF
                }
                && i != '\n' as i32
                && i != '\r' as i32
            {
                let fresh2 = bytesRead;
                bytesRead = bytesRead.wrapping_add(1);
                *byteBuffer.offset(fresh2 as isize) = i as ::core::ffi::c_char;
            }
        }
        if i == EOF && *__error() != EINTR && bytesRead == 0 as uint32_t {
            return false_0;
        }
        if i != EOF && i != '\n' as i32 && i != '\r' as i32 {
            buffer_overflow();
        }
        cnv = (*f).conversionData as *mut UConverter;
        match norm {
            1 | 2 => {
                if utf32Buf.is_null() {
                    utf32Buf = calloc(
                        buf_size as size_t,
                        ::core::mem::size_of::<uint32_t>() as size_t,
                    ) as *mut uint32_t;
                }
                tmpLen = ucnv_toAlgorithmic_78(
                    UCNV_UTF32_LittleEndian,
                    cnv,
                    utf32Buf as *mut ::core::ffi::c_char,
                    (buf_size as usize).wrapping_mul(::core::mem::size_of::<uint32_t>() as usize)
                        as int32_t,
                    byteBuffer,
                    bytesRead as int32_t,
                    &raw mut errorCode,
                ) as ::core::ffi::c_int;
                if errorCode as ::core::ffi::c_int != 0 as ::core::ffi::c_int {
                    conversion_error(errorCode as ::core::ffi::c_int);
                    return false_0;
                }
                apply_normalization(
                    utf32Buf,
                    (tmpLen as usize).wrapping_div(::core::mem::size_of::<uint32_t>() as usize)
                        as ::core::ffi::c_int,
                    norm,
                );
            }
            _ => {
                outLen = ucnv_toAlgorithmic_78(
                    UCNV_UTF32_LittleEndian,
                    cnv,
                    buffer.offset(first as isize) as *mut UnicodeScalar as *mut ::core::ffi::c_char,
                    (::core::mem::size_of::<UnicodeScalar>() as usize)
                        .wrapping_mul((buf_size - first) as usize) as int32_t,
                    byteBuffer,
                    bytesRead as int32_t,
                    &raw mut errorCode,
                ) as ::core::ffi::c_int;
                if errorCode as ::core::ffi::c_int != 0 as ::core::ffi::c_int {
                    conversion_error(errorCode as ::core::ffi::c_int);
                    return false_0;
                }
                outLen = (outLen as ::core::ffi::c_ulong)
                    .wrapping_div(
                        ::core::mem::size_of::<UnicodeScalar>() as usize as ::core::ffi::c_ulong
                    ) as ::core::ffi::c_int as ::core::ffi::c_int;
                last = first + outLen as int32_t;
            }
        }
    } else {
        i = get_uni_c(f);
        if (*f).skipNextLF != 0 {
            (*f).skipNextLF = 0 as ::core::ffi::c_short;
            if i == '\n' as i32 {
                i = get_uni_c(f);
            }
        }
        match norm {
            1 | 2 => {
                if utf32Buf.is_null() {
                    utf32Buf = calloc(
                        buf_size as size_t,
                        ::core::mem::size_of::<uint32_t>() as size_t,
                    ) as *mut uint32_t;
                }
                tmpLen = 0 as ::core::ffi::c_int;
                if i != EOF && i != '\n' as i32 && i != '\r' as i32 {
                    let fresh3 = tmpLen;
                    tmpLen = tmpLen + 1;
                    *utf32Buf.offset(fresh3 as isize) = i as uint32_t;
                }
                if i != EOF && i != '\n' as i32 && i != '\r' as i32 {
                    while (tmpLen as int32_t) < buf_size
                        && {
                            i = get_uni_c(f);
                            i != EOF
                        }
                        && i != '\n' as i32
                        && i != '\r' as i32
                    {
                        let fresh4 = tmpLen;
                        tmpLen = tmpLen + 1;
                        *utf32Buf.offset(fresh4 as isize) = i as uint32_t;
                    }
                }
                if i == EOF && *__error() != EINTR && tmpLen == 0 as ::core::ffi::c_int {
                    return false_0;
                }
                if i != EOF && i != '\n' as i32 && i != '\r' as i32 {
                    buffer_overflow();
                }
                apply_normalization(utf32Buf, tmpLen, norm);
            }
            _ => {
                if last < buf_size && i != EOF && i != '\n' as i32 && i != '\r' as i32 {
                    let fresh5 = last;
                    last = last + 1;
                    *buffer.offset(fresh5 as isize) = i as UnicodeScalar;
                }
                if i != EOF && i != '\n' as i32 && i != '\r' as i32 {
                    while last < buf_size
                        && {
                            i = get_uni_c(f);
                            i != EOF
                        }
                        && i != '\n' as i32
                        && i != '\r' as i32
                    {
                        let fresh6 = last;
                        last = last + 1;
                        *buffer.offset(fresh6 as isize) = i as UnicodeScalar;
                    }
                }
                if i == EOF && *__error() != EINTR && last == first {
                    return false_0;
                }
                if i != EOF && i != '\n' as i32 && i != '\r' as i32 {
                    buffer_overflow();
                }
            }
        }
    }
    if i == '\r' as i32 {
        (*f).skipNextLF = 1 as ::core::ffi::c_short;
    }
    *buffer.offset(last as isize) = ' ' as i32 as UnicodeScalar;
    if last >= max_buf_stack {
        max_buf_stack = last;
    }
    while last > first
        && (*buffer.offset((last - 1 as int32_t) as isize) == ' ' as i32
            || *buffer.offset((last - 1 as int32_t) as isize) == '\r' as i32
            || *buffer.offset((last - 1 as int32_t) as isize) == '\n' as i32)
    {
        last -= 1;
    }
    return true_0;
}
#[no_mangle]
pub unsafe extern "C" fn u_close(mut f: *mut UFILE) {
    if f.is_null() || (*f).handle.is_null() {
        return;
    }
    ttstub_input_close((*f).handle);
    if (*f).encodingMode as ::core::ffi::c_int == ICUMAPPING && !(*f).conversionData.is_null() {
        ucnv_close_78((*f).conversionData as *mut UConverter);
    }
    free(f as *mut ::core::ffi::c_void);
}
#[no_mangle]
pub unsafe extern "C" fn get_uni_c(mut f: *mut UFILE) -> ::core::ffi::c_int {
    let mut current_block: u64;
    let mut rval: ::core::ffi::c_int = 0;
    let mut c: ::core::ffi::c_int = 0;
    if (*f).savedChar != -(1 as ::core::ffi::c_int) as ::core::ffi::c_long {
        rval = (*f).savedChar as ::core::ffi::c_int;
        (*f).savedChar = -(1 as ::core::ffi::c_int) as ::core::ffi::c_long;
        return rval;
    }
    match (*f).encodingMode as ::core::ffi::c_int {
        UTF8 => {
            rval = ttstub_input_getc((*f).handle);
            c = rval;
            if rval != EOF {
                let mut extraBytes: uint16_t = bytesFromUTF8[rval as usize] as uint16_t;
                match extraBytes as ::core::ffi::c_int {
                    3 => {
                        c = ttstub_input_getc((*f).handle);
                        if c < 0x80 as ::core::ffi::c_int || c >= 0xc0 as ::core::ffi::c_int {
                            current_block = 13308703447601877371;
                        } else {
                            rval <<= 6 as ::core::ffi::c_int;
                            rval += c;
                            current_block = 7475748953310216569;
                        }
                    }
                    2 => {
                        current_block = 7475748953310216569;
                    }
                    1 => {
                        current_block = 9140737750339314049;
                    }
                    5 | 4 => {
                        current_block = 6061421480933744580;
                    }
                    0 | _ => {
                        current_block = 11042950489265723346;
                    }
                }
                match current_block {
                    7475748953310216569 => {
                        c = ttstub_input_getc((*f).handle);
                        if c < 0x80 as ::core::ffi::c_int || c >= 0xc0 as ::core::ffi::c_int {
                            current_block = 13308703447601877371;
                        } else {
                            rval <<= 6 as ::core::ffi::c_int;
                            rval += c;
                            current_block = 9140737750339314049;
                        }
                    }
                    _ => {}
                }
                match current_block {
                    9140737750339314049 => {
                        c = ttstub_input_getc((*f).handle);
                        if c < 0x80 as ::core::ffi::c_int || c >= 0xc0 as ::core::ffi::c_int {
                            current_block = 13308703447601877371;
                        } else {
                            rval <<= 6 as ::core::ffi::c_int;
                            rval += c;
                            current_block = 11042950489265723346;
                        }
                    }
                    _ => {}
                }
                match current_block {
                    11042950489265723346 => {
                        rval = (rval as uint32_t).wrapping_sub(offsetsFromUTF8[extraBytes as usize])
                            as ::core::ffi::c_int
                            as ::core::ffi::c_int;
                        if rval < 0 as ::core::ffi::c_int || rval > 0x10ffff as ::core::ffi::c_int {
                            bad_utf8_warning();
                            return 0xfffd as ::core::ffi::c_int;
                        }
                        current_block = 10930818133215224067;
                    }
                    13308703447601877371 => {
                        if c != EOF {
                            ttstub_input_ungetc((*f).handle, c);
                        }
                        current_block = 6061421480933744580;
                    }
                    _ => {}
                }
                match current_block {
                    10930818133215224067 => {}
                    _ => {
                        bad_utf8_warning();
                        return 0xfffd as ::core::ffi::c_int;
                    }
                }
            }
        }
        UTF16BE => {
            rval = ttstub_input_getc((*f).handle);
            if rval != EOF {
                rval <<= 8 as ::core::ffi::c_int;
                rval += ttstub_input_getc((*f).handle);
                if rval >= 0xd800 as ::core::ffi::c_int && rval <= 0xdbff as ::core::ffi::c_int {
                    let mut lo: ::core::ffi::c_int = ttstub_input_getc((*f).handle);
                    lo <<= 8 as ::core::ffi::c_int;
                    lo += ttstub_input_getc((*f).handle);
                    if lo >= 0xdc00 as ::core::ffi::c_int && lo <= 0xdfff as ::core::ffi::c_int {
                        rval = 0x10000 as ::core::ffi::c_int
                            + (rval - 0xd800 as ::core::ffi::c_int) * 0x400 as ::core::ffi::c_int
                            + (lo - 0xdc00 as ::core::ffi::c_int);
                    } else {
                        rval = 0xfffd as ::core::ffi::c_int;
                        (*f).savedChar = lo as ::core::ffi::c_long;
                    }
                } else if rval >= 0xdc00 as ::core::ffi::c_int
                    && rval <= 0xdfff as ::core::ffi::c_int
                {
                    rval = 0xfffd as ::core::ffi::c_int;
                }
            }
        }
        UTF16LE => {
            rval = ttstub_input_getc((*f).handle);
            if rval != EOF {
                rval += ttstub_input_getc((*f).handle) << 8 as ::core::ffi::c_int;
                if rval >= 0xd800 as ::core::ffi::c_int && rval <= 0xdbff as ::core::ffi::c_int {
                    let mut lo_0: ::core::ffi::c_int = ttstub_input_getc((*f).handle);
                    lo_0 += ttstub_input_getc((*f).handle) << 8 as ::core::ffi::c_int;
                    if lo_0 >= 0xdc00 as ::core::ffi::c_int && lo_0 <= 0xdfff as ::core::ffi::c_int
                    {
                        rval = 0x10000 as ::core::ffi::c_int
                            + (rval - 0xd800 as ::core::ffi::c_int) * 0x400 as ::core::ffi::c_int
                            + (lo_0 - 0xdc00 as ::core::ffi::c_int);
                    } else {
                        rval = 0xfffd as ::core::ffi::c_int;
                        (*f).savedChar = lo_0 as ::core::ffi::c_long;
                    }
                } else if rval >= 0xdc00 as ::core::ffi::c_int
                    && rval <= 0xdfff as ::core::ffi::c_int
                {
                    rval = 0xfffd as ::core::ffi::c_int;
                }
            }
        }
        RAW => {
            rval = ttstub_input_getc((*f).handle);
        }
        _ => {
            _tt_abort(
                b"internal error; file input mode=%d\0" as *const u8 as *const ::core::ffi::c_char,
                (*f).encodingMode as ::core::ffi::c_int,
            );
        }
    }
    return rval;
}
#[no_mangle]
pub unsafe extern "C" fn make_utf16_name() {
    let mut s: *mut ::core::ffi::c_uchar = name_of_file as *mut ::core::ffi::c_uchar;
    let mut rval: uint32_t = 0;
    let mut t: *mut uint16_t = ::core::ptr::null_mut::<uint16_t>();
    static mut name16len: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    if name16len as int32_t <= name_length {
        free(name_of_file16 as *mut ::core::ffi::c_void);
        name16len = (name_length + 10 as int32_t) as ::core::ffi::c_int;
        name_of_file16 = calloc(
            name16len as size_t,
            ::core::mem::size_of::<uint16_t>() as size_t,
        ) as *mut UTF16_code;
    }
    t = name_of_file16 as *mut uint16_t;
    while s < (name_of_file as *mut ::core::ffi::c_uchar).offset(name_length as isize) {
        let mut extraBytes: uint16_t = 0;
        let fresh7 = s;
        s = s.offset(1);
        rval = *fresh7 as uint32_t;
        extraBytes = bytesFromUTF8[rval as usize] as uint16_t;
        let mut current_block_23: u64;
        match extraBytes as ::core::ffi::c_int {
            5 => {
                rval <<= 6 as ::core::ffi::c_int;
                if *s != 0 {
                    let fresh8 = s;
                    s = s.offset(1);
                    rval = rval.wrapping_add(*fresh8 as uint32_t);
                }
                current_block_23 = 16684841253143231238;
            }
            4 => {
                current_block_23 = 16684841253143231238;
            }
            3 => {
                current_block_23 = 1015465775986710323;
            }
            2 => {
                current_block_23 = 14494481496807393559;
            }
            1 => {
                current_block_23 = 15443804656981266449;
            }
            0 | _ => {
                current_block_23 = 5634871135123216486;
            }
        }
        match current_block_23 {
            16684841253143231238 => {
                rval <<= 6 as ::core::ffi::c_int;
                if *s != 0 {
                    let fresh9 = s;
                    s = s.offset(1);
                    rval = rval.wrapping_add(*fresh9 as uint32_t);
                }
                current_block_23 = 1015465775986710323;
            }
            _ => {}
        }
        match current_block_23 {
            1015465775986710323 => {
                rval <<= 6 as ::core::ffi::c_int;
                if *s != 0 {
                    let fresh10 = s;
                    s = s.offset(1);
                    rval = rval.wrapping_add(*fresh10 as uint32_t);
                }
                current_block_23 = 14494481496807393559;
            }
            _ => {}
        }
        match current_block_23 {
            14494481496807393559 => {
                rval <<= 6 as ::core::ffi::c_int;
                if *s != 0 {
                    let fresh11 = s;
                    s = s.offset(1);
                    rval = rval.wrapping_add(*fresh11 as uint32_t);
                }
                current_block_23 = 15443804656981266449;
            }
            _ => {}
        }
        match current_block_23 {
            15443804656981266449 => {
                rval <<= 6 as ::core::ffi::c_int;
                if *s != 0 {
                    let fresh12 = s;
                    s = s.offset(1);
                    rval = rval.wrapping_add(*fresh12 as uint32_t);
                }
            }
            _ => {}
        }
        rval = rval.wrapping_sub(offsetsFromUTF8[extraBytes as usize]);
        if rval > 0xffff as uint32_t {
            rval = rval.wrapping_sub(0x10000 as uint32_t);
            let fresh13 = t;
            t = t.offset(1);
            *fresh13 =
                (0xd800 as uint32_t).wrapping_add(rval.wrapping_div(0x400 as uint32_t)) as uint16_t;
            let fresh14 = t;
            t = t.offset(1);
            *fresh14 =
                (0xdc00 as uint32_t).wrapping_add(rval.wrapping_rem(0x400 as uint32_t)) as uint16_t;
        } else {
            let fresh15 = t;
            t = t.offset(1);
            *fresh15 = rval as uint16_t;
        }
    }
    name_length16 = t.offset_from(name_of_file16) as ::core::ffi::c_long as int32_t;
}
#[no_mangle]
pub unsafe extern "C" fn open_or_close_in() {
    let mut c: ::core::ffi::c_uchar = 0;
    let mut n: ::core::ffi::c_uchar = 0;
    let mut k: int32_t = 0;
    c = cur_chr as ::core::ffi::c_uchar;
    scan_four_bit_int();
    n = cur_val as ::core::ffi::c_uchar;
    if read_open[n as usize] as ::core::ffi::c_int != CLOSED {
        u_close(read_file[n as usize]);
        read_open[n as usize] = CLOSED as ::core::ffi::c_uchar;
    }
    if c as ::core::ffi::c_int != 0 as ::core::ffi::c_int {
        scan_optional_equals();
        scan_file_name();
        pack_file_name(cur_name, cur_area, cur_ext);
        if u_open_in(
            (&raw mut read_file as *mut *mut UFILE).offset(n as isize) as *mut *mut UFILE,
            TTBC_FILE_FORMAT_TEX as ::core::ffi::c_int as int32_t,
            b"rb\0" as *const u8 as *const ::core::ffi::c_char,
            (*eqtb.offset((INT_BASE + INT_PAR__xetex_default_input_mode) as isize))
                .b32
                .s1,
            (*eqtb.offset((INT_BASE + INT_PAR__xetex_default_input_encoding) as isize))
                .b32
                .s1,
        ) != 0
        {
            make_utf16_name();
            name_in_progress = true_0 != 0;
            begin_name();
            stop_at_space = false_0 != 0;
            k = 0 as ::core::ffi::c_int as int32_t;
            while k < name_length16
                && more_name(*name_of_file16.offset(k as isize)) as ::core::ffi::c_int != 0
            {
                k += 1;
            }
            stop_at_space = true_0 != 0;
            end_name();
            name_in_progress = false_0 != 0;
            read_open[n as usize] = JUST_OPEN as ::core::ffi::c_uchar;
        }
    }
}
pub const true_0: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const false_0: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
