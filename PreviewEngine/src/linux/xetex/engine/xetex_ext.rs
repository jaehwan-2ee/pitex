/****************************************************************************\
 Part of the XeTeX typesetting system
 Copyright (c) 1994-2008 by SIL International
 Copyright (c) 2009, 2011 by Jonathan Kew
 Copyright (c) 2012-2015 by Khaled Hosny
 Copyright (c) 2012, 2013 by Jiang Jiang

 SIL Author(s): Jonathan Kew

Permission is hereby granted, free of charge, to any person obtaining
a copy of this software and associated documentation files (the
"Software"), to deal in the Software without restriction, including
without limitation the rights to use, copy, modify, merge, publish,
distribute, sublicense, and/or sell copies of the Software, and to
permit persons to whom the Software is furnished to do so, subject to
the following conditions:

The above copyright notice and this permission notice shall be
included in all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND
NONINFRINGEMENT. IN NO EVENT SHALL THE COPYRIGHT HOLDERS BE LIABLE
FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF
CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION
WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

Except as contained in this notice, the name of the copyright holders
shall not be used in advertising or otherwise to promote the sale,
use or other dealings in this Software without prior written
authorization from the copyright holders.
\****************************************************************************/
// Translated from xetex/engine/xetex-ext.c with C2Rust 0.22.1.
extern "C" {
    pub type ttbc_input_handle_t;
    pub type _FcPattern;
    pub type XeTeXFont_rec;
    pub type XeTeXLayoutEngine_rec;
    pub type UBreakIterator;
    pub type UConverter;
    pub type Opaque_TECkit_Converter;
    pub type UBiDi;
    fn __assert_fail(
        __assertion: *const ::core::ffi::c_char,
        __file: *const ::core::ffi::c_char,
        __line: ::core::ffi::c_uint,
        __function: *const ::core::ffi::c_char,
    ) -> !;
    fn malloc(__size: size_t) -> *mut ::core::ffi::c_void;
    fn calloc(__nmemb: size_t, __size: size_t) -> *mut ::core::ffi::c_void;
    fn realloc(
        __ptr: *mut ::core::ffi::c_void,
        __size: size_t,
    ) -> *mut ::core::ffi::c_void;
    fn free(__ptr: *mut ::core::ffi::c_void);
    fn memcpy(
        __dest: *mut ::core::ffi::c_void,
        __src: *const ::core::ffi::c_void,
        __n: size_t,
    ) -> *mut ::core::ffi::c_void;
    fn strcpy(
        __dest: *mut ::core::ffi::c_char,
        __src: *const ::core::ffi::c_char,
    ) -> *mut ::core::ffi::c_char;
    fn strncpy(
        __dest: *mut ::core::ffi::c_char,
        __src: *const ::core::ffi::c_char,
        __n: size_t,
    ) -> *mut ::core::ffi::c_char;
    fn strcat(
        __dest: *mut ::core::ffi::c_char,
        __src: *const ::core::ffi::c_char,
    ) -> *mut ::core::ffi::c_char;
    fn strcmp(
        __s1: *const ::core::ffi::c_char,
        __s2: *const ::core::ffi::c_char,
    ) -> ::core::ffi::c_int;
    fn strncmp(
        __s1: *const ::core::ffi::c_char,
        __s2: *const ::core::ffi::c_char,
        __n: size_t,
    ) -> ::core::ffi::c_int;
    fn strdup(__s: *const ::core::ffi::c_char) -> *mut ::core::ffi::c_char;
    fn strstr(
        __haystack: *const ::core::ffi::c_char,
        __needle: *const ::core::ffi::c_char,
    ) -> *mut ::core::ffi::c_char;
    fn strlen(__s: *const ::core::ffi::c_char) -> size_t;
    fn strcasecmp(
        __s1: *const ::core::ffi::c_char,
        __s2: *const ::core::ffi::c_char,
    ) -> ::core::ffi::c_int;
    fn hb_tag_from_string(
        str: *const ::core::ffi::c_char,
        len: ::core::ffi::c_int,
    ) -> hb_tag_t;
    static mut loaded_font_design_size: Fixed;
    fn get_cp_code(
        fontNum: ::core::ffi::c_int,
        code: ::core::ffi::c_uint,
        side: ::core::ffi::c_int,
    ) -> ::core::ffi::c_int;
    fn getCachedGlyphBBox(
        fontID: uint16_t,
        glyphID: uint16_t,
        bbox: *mut GlyphBBox,
    ) -> ::core::ffi::c_int;
    fn cacheGlyphBBox(fontID: uint16_t, glyphID: uint16_t, bbox: *const GlyphBBox);
    fn createFont(fontRef: PlatformFontRef, pointSize: Fixed) -> XeTeXFont;
    fn createFontFromFile(
        filename: *const ::core::ffi::c_char,
        index: ::core::ffi::c_int,
        pointSize: Fixed,
    ) -> XeTeXFont;
    fn setFontLayoutDir(font: XeTeXFont, vertical: ::core::ffi::c_int);
    fn findFontByName(
        name: *const ::core::ffi::c_char,
        var: *mut ::core::ffi::c_char,
        size: ::core::ffi::c_double,
    ) -> PlatformFontRef;
    fn getReqEngine() -> ::core::ffi::c_char;
    fn setReqEngine(reqEngine: ::core::ffi::c_char);
    fn getFullName(fontRef: PlatformFontRef) -> *const ::core::ffi::c_char;
    fn getFontFilename(
        engine: XeTeXLayoutEngine,
        index: *mut uint32_t,
    ) -> *mut ::core::ffi::c_char;
    fn getDesignSize(font: XeTeXFont) -> ::core::ffi::c_double;
    fn deleteFont(font: XeTeXFont);
    fn getSlant(font: XeTeXFont) -> Fixed;
    fn countScripts(font: XeTeXFont) -> ::core::ffi::c_uint;
    fn countLanguages(font: XeTeXFont, script: hb_tag_t) -> ::core::ffi::c_uint;
    fn countFeatures(
        font: XeTeXFont,
        script: hb_tag_t,
        language: hb_tag_t,
    ) -> ::core::ffi::c_uint;
    fn countGlyphs(font: XeTeXFont) -> ::core::ffi::c_uint;
    fn getIndScript(font: XeTeXFont, index: ::core::ffi::c_uint) -> hb_tag_t;
    fn getIndLanguage(
        font: XeTeXFont,
        script: hb_tag_t,
        index: ::core::ffi::c_uint,
    ) -> hb_tag_t;
    fn getIndFeature(
        font: XeTeXFont,
        script: hb_tag_t,
        language: hb_tag_t,
        index: ::core::ffi::c_uint,
    ) -> hb_tag_t;
    fn getGlyphWidth(font: XeTeXFont, gid: uint32_t) -> ::core::ffi::c_float;
    fn createLayoutEngine(
        fontRef: PlatformFontRef,
        font: XeTeXFont,
        script: hb_tag_t,
        language: *mut ::core::ffi::c_char,
        features: *mut hb_feature_t,
        nFeatures: ::core::ffi::c_int,
        shapers: *mut *mut ::core::ffi::c_char,
        rgbValue: uint32_t,
        extend: ::core::ffi::c_float,
        slant: ::core::ffi::c_float,
        embolden: ::core::ffi::c_float,
    ) -> XeTeXLayoutEngine;
    fn deleteLayoutEngine(engine: XeTeXLayoutEngine);
    fn getFont(engine: XeTeXLayoutEngine) -> XeTeXFont;
    fn getFontRef(engine: XeTeXLayoutEngine) -> PlatformFontRef;
    fn getExtendFactor(engine: XeTeXLayoutEngine) -> ::core::ffi::c_float;
    fn getSlantFactor(engine: XeTeXLayoutEngine) -> ::core::ffi::c_float;
    fn getEmboldenFactor(engine: XeTeXLayoutEngine) -> ::core::ffi::c_float;
    fn layoutChars(
        engine: XeTeXLayoutEngine,
        chars: *mut uint16_t,
        offset: int32_t,
        count: int32_t,
        max: int32_t,
        rightToLeft: bool,
    ) -> ::core::ffi::c_int;
    fn getGlyphs(engine: XeTeXLayoutEngine, glyphs: *mut uint32_t);
    fn getGlyphAdvances(engine: XeTeXLayoutEngine, advances: *mut ::core::ffi::c_float);
    fn getGlyphPositions(engine: XeTeXLayoutEngine, positions: *mut FloatPoint);
    fn getPointSize(engine: XeTeXLayoutEngine) -> ::core::ffi::c_float;
    fn getAscentAndDescent(
        engine: XeTeXLayoutEngine,
        ascent: *mut ::core::ffi::c_float,
        descent: *mut ::core::ffi::c_float,
    );
    fn getCapAndXHeight(
        engine: XeTeXLayoutEngine,
        capheight: *mut ::core::ffi::c_float,
        xheight: *mut ::core::ffi::c_float,
    );
    fn getDefaultDirection(engine: XeTeXLayoutEngine) -> ::core::ffi::c_int;
    fn getRgbValue(engine: XeTeXLayoutEngine) -> uint32_t;
    fn getGlyphBounds(
        engine: XeTeXLayoutEngine,
        glyphID: uint32_t,
        bbox: *mut GlyphBBox,
    );
    fn getGlyphWidthFromEngine(
        engine: XeTeXLayoutEngine,
        glyphID: uint32_t,
    ) -> ::core::ffi::c_float;
    fn getGlyphHeightDepth(
        engine: XeTeXLayoutEngine,
        glyphID: uint32_t,
        height: *mut ::core::ffi::c_float,
        depth: *mut ::core::ffi::c_float,
    );
    fn getGlyphSidebearings(
        engine: XeTeXLayoutEngine,
        glyphID: uint32_t,
        lsb: *mut ::core::ffi::c_float,
        rsb: *mut ::core::ffi::c_float,
    );
    fn getGlyphItalCorr(
        engine: XeTeXLayoutEngine,
        glyphID: uint32_t,
    ) -> ::core::ffi::c_float;
    fn mapCharToGlyph(engine: XeTeXLayoutEngine, charCode: uint32_t) -> uint32_t;
    fn mapGlyphToIndex(
        engine: XeTeXLayoutEngine,
        glyphName: *const ::core::ffi::c_char,
    ) -> ::core::ffi::c_int;
    fn getGlyphName(
        font: XeTeXFont,
        gid: uint16_t,
        len: *mut ::core::ffi::c_int,
    ) -> *const ::core::ffi::c_char;
    fn getFontCharRange(
        engine: XeTeXLayoutEngine,
        reqFirst: ::core::ffi::c_int,
    ) -> ::core::ffi::c_int;
    fn initGraphiteBreaking(
        engine: XeTeXLayoutEngine,
        txtPtr: *const uint16_t,
        txtLen: ::core::ffi::c_int,
    ) -> bool;
    fn findNextGraphiteBreak() -> ::core::ffi::c_int;
    fn findGraphiteFeature(
        engine: XeTeXLayoutEngine,
        s: *const ::core::ffi::c_char,
        e: *const ::core::ffi::c_char,
        f: *mut hb_tag_t,
        v: *mut ::core::ffi::c_int,
    ) -> bool;
    fn countGraphiteFeatures(engine: XeTeXLayoutEngine) -> uint32_t;
    fn getGraphiteFeatureCode(engine: XeTeXLayoutEngine, index: uint32_t) -> uint32_t;
    fn countGraphiteFeatureSettings(
        engine: XeTeXLayoutEngine,
        feature: uint32_t,
    ) -> uint32_t;
    fn getGraphiteFeatureSettingCode(
        engine: XeTeXLayoutEngine,
        feature: uint32_t,
        index: uint32_t,
    ) -> uint32_t;
    fn getGraphiteFeatureDefaultSetting(
        engine: XeTeXLayoutEngine,
        feature: uint32_t,
    ) -> uint32_t;
    fn getGraphiteFeatureLabel(
        engine: XeTeXLayoutEngine,
        feature: uint32_t,
    ) -> *mut ::core::ffi::c_char;
    fn getGraphiteFeatureSettingLabel(
        engine: XeTeXLayoutEngine,
        feature: uint32_t,
        setting: uint32_t,
    ) -> *mut ::core::ffi::c_char;
    fn findGraphiteFeatureNamed(
        engine: XeTeXLayoutEngine,
        name: *const ::core::ffi::c_char,
        namelength: ::core::ffi::c_int,
    ) -> ::core::ffi::c_long;
    fn findGraphiteFeatureSettingNamed(
        engine: XeTeXLayoutEngine,
        feature: uint32_t,
        name: *const ::core::ffi::c_char,
        namelength: ::core::ffi::c_int,
    ) -> ::core::ffi::c_long;
    fn ttxl_platfont_get_desc(fontRef: PlatformFontRef) -> *const ::core::ffi::c_char;
    fn maketexstring(s: *const ::core::ffi::c_char) -> ::core::ffi::c_int;
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
    fn _tt_abort(format: *const ::core::ffi::c_char, ...) -> !;
    fn ttstub_input_open(
        path: *const ::core::ffi::c_char,
        format: ttbc_file_format,
        is_gz: ::core::ffi::c_int,
    ) -> rust_input_handle_t;
    fn ttstub_input_get_size(handle: rust_input_handle_t) -> size_t;
    fn ttstub_input_read(
        handle: rust_input_handle_t,
        data: *mut ::core::ffi::c_char,
        len: size_t,
    ) -> ssize_t;
    fn ttstub_input_close(handle: rust_input_handle_t) -> ::core::ffi::c_int;
    #[link_name = concat!("ubidi_open_", env!("PITEX_PREVIEW_ICU_MAJOR"))]
    fn ubidi_open_74() -> *mut UBiDi;
    #[link_name = concat!("ubidi_close_", env!("PITEX_PREVIEW_ICU_MAJOR"))]
    fn ubidi_close_74(pBiDi: *mut UBiDi);
    #[link_name = concat!("ubidi_setPara_", env!("PITEX_PREVIEW_ICU_MAJOR"))]
    fn ubidi_setPara_74(
        pBiDi: *mut UBiDi,
        text: *const UChar,
        length: int32_t,
        paraLevel: UBiDiLevel,
        embeddingLevels: *mut UBiDiLevel,
        pErrorCode: *mut UErrorCode,
    );
    #[link_name = concat!("ubidi_getDirection_", env!("PITEX_PREVIEW_ICU_MAJOR"))]
    fn ubidi_getDirection_74(pBiDi: *const UBiDi) -> UBiDiDirection;
    #[link_name = concat!("ubidi_countRuns_", env!("PITEX_PREVIEW_ICU_MAJOR"))]
    fn ubidi_countRuns_74(pBiDi: *mut UBiDi, pErrorCode: *mut UErrorCode) -> int32_t;
    #[link_name = concat!("ubidi_getVisualRun_", env!("PITEX_PREVIEW_ICU_MAJOR"))]
    fn ubidi_getVisualRun_74(
        pBiDi: *mut UBiDi,
        runIndex: int32_t,
        pLogicalStart: *mut int32_t,
        pLength: *mut int32_t,
    ) -> UBiDiDirection;
    #[link_name = concat!("ubrk_open_", env!("PITEX_PREVIEW_ICU_MAJOR"))]
    fn ubrk_open_74(
        type_0: UBreakIteratorType,
        locale: *const ::core::ffi::c_char,
        text: *const UChar,
        textLength: int32_t,
        status: *mut UErrorCode,
    ) -> *mut UBreakIterator;
    #[link_name = concat!("ubrk_close_", env!("PITEX_PREVIEW_ICU_MAJOR"))]
    fn ubrk_close_74(bi: *mut UBreakIterator);
    #[link_name = concat!("ubrk_setText_", env!("PITEX_PREVIEW_ICU_MAJOR"))]
    fn ubrk_setText_74(
        bi: *mut UBreakIterator,
        text: *const UChar,
        textLength: int32_t,
        status: *mut UErrorCode,
    );
    #[link_name = concat!("ubrk_next_", env!("PITEX_PREVIEW_ICU_MAJOR"))]
    fn ubrk_next_74(bi: *mut UBreakIterator) -> int32_t;
    #[link_name = concat!("ucnv_open_", env!("PITEX_PREVIEW_ICU_MAJOR"))]
    fn ucnv_open_74(
        converterName: *const ::core::ffi::c_char,
        err: *mut UErrorCode,
    ) -> *mut UConverter;
    #[link_name = concat!("ucnv_close_", env!("PITEX_PREVIEW_ICU_MAJOR"))]
    fn ucnv_close_74(converter: *mut UConverter);
    fn gr_label_destroy(label: *mut ::core::ffi::c_void);
    fn gettexstring(_: str_number) -> *mut ::core::ffi::c_char;
    static mut name_of_file: *mut ::core::ffi::c_char;
    static mut name_length: int32_t;
    static mut font_info: *mut memory_word;
    static mut font_area: *mut str_number;
    static mut font_layout_engine: *mut *mut ::core::ffi::c_void;
    static mut font_flags: *mut ::core::ffi::c_char;
    static mut font_letter_space: *mut scaled_t;
    static mut loaded_font_mapping: *mut ::core::ffi::c_void;
    static mut loaded_font_flags: ::core::ffi::c_char;
    static mut loaded_font_letter_space: scaled_t;
    static mut mapped_text: *mut UTF16_code;
    static mut xdv_buffer: *mut ::core::ffi::c_char;
    static mut height_base: *mut int32_t;
    static mut depth_base: *mut int32_t;
    static mut param_base: *mut int32_t;
    static mut native_font_type_flag: int32_t;
    fn begin_diagnostic();
    fn end_diagnostic(blank_line: bool);
    fn font_feature_warning(
        featureNameP: *const ::core::ffi::c_void,
        featLen: int32_t,
        settingNameP: *const ::core::ffi::c_void,
        setLen: int32_t,
    );
    fn font_mapping_warning(
        mappingNameP: *const ::core::ffi::c_void,
        mappingNameLen: int32_t,
        warningType: int32_t,
    );
    fn get_tracing_fonts_state() -> int32_t;
    fn print_raw_char(s: UTF16_code, incr_offset: bool);
    fn print_char(s: int32_t);
    fn print_nl(s: str_number);
    fn print_int(n: int32_t);
    fn xn_over_d(x: scaled_t, n: int32_t, d: int32_t) -> scaled_t;
}
pub type __uint8_t = u8;
pub type __uint16_t = u16;
pub type __int32_t = i32;
pub type __uint32_t = u32;
pub type int32_t = __int32_t;
pub type uint8_t = __uint8_t;
pub type uint16_t = __uint16_t;
pub type uint32_t = __uint32_t;
pub type size_t = usize;
pub type ssize_t = isize;
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
pub type UChar = uint16_t;
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
pub const U_FMT_PARSE_ERROR_LIMIT: UErrorCode = 65812;
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
pub type FcPattern = _FcPattern;
pub type hb_tag_t = uint32_t;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct hb_feature_t {
    pub tag: hb_tag_t,
    pub value: uint32_t,
    pub start: ::core::ffi::c_uint,
    pub end: ::core::ffi::c_uint,
}
pub type scaled_t = int32_t;
pub type PlatformFontRef = *mut FcPattern;
pub type Fixed = int32_t;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct FloatPoint {
    pub x: ::core::ffi::c_float,
    pub y: ::core::ffi::c_float,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct GlyphBBox {
    pub xMin: ::core::ffi::c_float,
    pub yMin: ::core::ffi::c_float,
    pub xMax: ::core::ffi::c_float,
    pub yMax: ::core::ffi::c_float,
}
pub type XeTeXFont = *mut XeTeXFont_rec;
pub type XeTeXLayoutEngine = *mut XeTeXLayoutEngine_rec;
pub type CFDictionaryRef = *mut ::core::ffi::c_void;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct FixedPoint {
    pub x: Fixed,
    pub y: Fixed,
}
pub type str_number = int32_t;
pub type UBreakIteratorType = ::core::ffi::c_uint;
pub const UBRK_COUNT: UBreakIteratorType = 5;
pub const UBRK_TITLE: UBreakIteratorType = 4;
pub const UBRK_SENTENCE: UBreakIteratorType = 3;
pub const UBRK_LINE: UBreakIteratorType = 2;
pub const UBRK_WORD: UBreakIteratorType = 1;
pub const UBRK_CHARACTER: UBreakIteratorType = 0;
pub type UTF16_code = ::core::ffi::c_ushort;
pub type TECkit_Converter = *mut Opaque_TECkit_Converter;
pub type Byte = UInt8;
pub type UInt8 = ::core::ffi::c_uchar;
pub type TECkit_Status = ::core::ffi::c_long;
pub type UInt16 = ::core::ffi::c_ushort;
pub type UInt32 = ::core::ffi::c_uint;
pub type b32x2 = b32x2_le_t;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct b32x2_le_t {
    pub s0: int32_t,
    pub s1: int32_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union memory_word {
    pub b32: b32x2,
    pub b16: b16x4,
    pub gr: ::core::ffi::c_double,
    pub ptr: *mut ::core::ffi::c_void,
}
pub type b16x4 = b16x4_le_t;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct b16x4_le_t {
    pub s0: uint16_t,
    pub s1: uint16_t,
    pub s2: uint16_t,
    pub s3: uint16_t,
}
pub type UniChar = UInt16;
pub const UBIDI_RTL: UBiDiDirection = 1;
pub type UBiDiDirection = ::core::ffi::c_uint;
pub const UBIDI_NEUTRAL: UBiDiDirection = 3;
pub const UBIDI_MIXED: UBiDiDirection = 2;
pub const UBIDI_LTR: UBiDiDirection = 0;
pub type UBiDiLevel = uint8_t;
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<
    ::core::ffi::c_void,
>();
pub const NULL_0: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<
    ::core::ffi::c_void,
>();
pub const HB_TAG_NONE: hb_tag_t = (0 as ::core::ffi::c_int as uint32_t
    & 0xff as uint32_t) << 24 as ::core::ffi::c_int
    | (0 as ::core::ffi::c_int as uint32_t & 0xff as uint32_t)
        << 16 as ::core::ffi::c_int
    | (0 as ::core::ffi::c_int as uint32_t & 0xff as uint32_t) << 8 as ::core::ffi::c_int
    | 0 as ::core::ffi::c_int as uint32_t & 0xff as uint32_t;
pub const US_NATIVE_UTF16: ::core::ffi::c_int = UTF16LE;
pub const UTF16_NATIVE: ::core::ffi::c_int = kForm_UTF16LE;
pub const FONT_FLAGS_COLORED: ::core::ffi::c_int = 0x1 as ::core::ffi::c_int;
pub const FONT_FLAGS_VERTICAL: ::core::ffi::c_int = 0x2 as ::core::ffi::c_int;
pub const LEFT_SIDE: ::core::ffi::c_int = 0;
pub const RIGHT_SIDE: ::core::ffi::c_int = 1;
pub const OTGR_FONT_FLAG: ::core::ffi::c_uint = 0xfffe as ::core::ffi::c_uint;
pub const XeTeX_count_glyphs: ::core::ffi::c_int = 1;
pub const XeTeX_count_features: ::core::ffi::c_int = 8;
pub const XeTeX_feature_code: ::core::ffi::c_int = 9;
pub const XeTeX_find_feature_by_name: ::core::ffi::c_int = 10;
pub const XeTeX_is_exclusive_feature: ::core::ffi::c_int = 11;
pub const XeTeX_count_selectors: ::core::ffi::c_int = 12;
pub const XeTeX_selector_code: ::core::ffi::c_int = 13;
pub const XeTeX_find_selector_by_name: ::core::ffi::c_int = 14;
pub const XeTeX_is_default_selector: ::core::ffi::c_int = 15;
pub const XeTeX_OT_count_scripts: ::core::ffi::c_int = 16;
pub const XeTeX_OT_count_languages: ::core::ffi::c_int = 17;
pub const XeTeX_OT_count_features: ::core::ffi::c_int = 18;
pub const XeTeX_OT_script_code: ::core::ffi::c_int = 19;
pub const XeTeX_OT_language_code: ::core::ffi::c_int = 20;
pub const XeTeX_OT_feature_code: ::core::ffi::c_int = 21;
pub const width_offset: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const depth_offset: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const height_offset: ::core::ffi::c_int = 3 as ::core::ffi::c_int;
pub const native_info_offset: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
pub const native_glyph_info_offset: ::core::ffi::c_int = 5 as ::core::ffi::c_int;
pub const native_glyph_info_size: ::core::ffi::c_int = 10 as ::core::ffi::c_int;
static mut brkIter: *mut UBreakIterator = ::core::ptr::null::<UBreakIterator>()
    as *mut UBreakIterator;
static mut brkLocaleStrNum: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
#[no_mangle]
pub unsafe extern "C" fn linebreak_start(
    mut f: ::core::ffi::c_int,
    mut localeStrNum: int32_t,
    mut text: *mut uint16_t,
    mut textLength: int32_t,
) {
    let mut status: UErrorCode = U_ZERO_ERROR;
    let mut locale: *mut ::core::ffi::c_char = gettexstring(localeStrNum as str_number);
    if *font_area.offset(f as isize) as ::core::ffi::c_uint == OTGR_FONT_FLAG
        && streq_ptr(locale, b"G\0" as *const u8 as *const ::core::ffi::c_char)
            as ::core::ffi::c_int != 0
    {
        let mut engine: XeTeXLayoutEngine = *font_layout_engine.offset(f as isize)
            as XeTeXLayoutEngine;
        if initGraphiteBreaking(engine, text, textLength as ::core::ffi::c_int) {
            return;
        }
    }
    if localeStrNum != brkLocaleStrNum as int32_t && !brkIter.is_null() {
        ubrk_close_74(brkIter);
        brkIter = ::core::ptr::null_mut::<UBreakIterator>();
    }
    if brkIter.is_null() {
        brkIter = ubrk_open_74(
            UBRK_LINE,
            locale,
            ::core::ptr::null::<UChar>(),
            0 as int32_t,
            &raw mut status,
        );
        if status as ::core::ffi::c_int > U_ZERO_ERROR as ::core::ffi::c_int {
            begin_diagnostic();
            print_nl('E' as i32);
            print_c_string(b"rror \0" as *const u8 as *const ::core::ffi::c_char);
            print_int(status as int32_t);
            print_c_string(
                b" creating linebreak iterator for locale `\0" as *const u8
                    as *const ::core::ffi::c_char,
            );
            print_c_string(locale);
            print_c_string(
                b"'; trying default locale `en_us'.\0" as *const u8
                    as *const ::core::ffi::c_char,
            );
            end_diagnostic(1 as ::core::ffi::c_int != 0);
            if !brkIter.is_null() {
                ubrk_close_74(brkIter);
            }
            status = U_ZERO_ERROR;
            brkIter = ubrk_open_74(
                UBRK_LINE,
                b"en_us\0" as *const u8 as *const ::core::ffi::c_char,
                ::core::ptr::null::<UChar>(),
                0 as int32_t,
                &raw mut status,
            );
        }
        free(locale as *mut ::core::ffi::c_void);
        brkLocaleStrNum = localeStrNum as ::core::ffi::c_int;
    }
    if brkIter.is_null() {
        _tt_abort(
            b"failed to create linebreak iterator, status=%d\0" as *const u8
                as *const ::core::ffi::c_char,
            status as ::core::ffi::c_int,
        );
    }
    ubrk_setText_74(brkIter, text as *mut UChar, textLength, &raw mut status);
}
#[no_mangle]
pub unsafe extern "C" fn linebreak_next() -> ::core::ffi::c_int {
    if !brkIter.is_null() {
        return ubrk_next_74(brkIter) as ::core::ffi::c_int
    } else {
        return findNextGraphiteBreak()
    };
}
#[no_mangle]
pub unsafe extern "C" fn get_encoding_mode_and_info(
    mut info: *mut int32_t,
) -> ::core::ffi::c_int {
    let mut err: UErrorCode = U_ZERO_ERROR;
    let mut cnv: *mut UConverter = ::core::ptr::null_mut::<UConverter>();
    *info = 0 as ::core::ffi::c_int as int32_t;
    if strcasecmp(name_of_file, b"auto\0" as *const u8 as *const ::core::ffi::c_char)
        == 0 as ::core::ffi::c_int
    {
        return AUTO;
    }
    if strcasecmp(name_of_file, b"utf8\0" as *const u8 as *const ::core::ffi::c_char)
        == 0 as ::core::ffi::c_int
    {
        return UTF8;
    }
    if strcasecmp(name_of_file, b"utf16\0" as *const u8 as *const ::core::ffi::c_char)
        == 0 as ::core::ffi::c_int
    {
        return US_NATIVE_UTF16;
    }
    if strcasecmp(name_of_file, b"utf16be\0" as *const u8 as *const ::core::ffi::c_char)
        == 0 as ::core::ffi::c_int
    {
        return UTF16BE;
    }
    if strcasecmp(name_of_file, b"utf16le\0" as *const u8 as *const ::core::ffi::c_char)
        == 0 as ::core::ffi::c_int
    {
        return UTF16LE;
    }
    if strcasecmp(name_of_file, b"bytes\0" as *const u8 as *const ::core::ffi::c_char)
        == 0 as ::core::ffi::c_int
    {
        return RAW;
    }
    cnv = ucnv_open_74(name_of_file, &raw mut err);
    if cnv.is_null() {
        begin_diagnostic();
        print_nl('U' as i32);
        print_c_string(
            b"nknown encoding `\0" as *const u8 as *const ::core::ffi::c_char,
        );
        print_c_string(name_of_file);
        print_c_string(
            b"'; reading as raw bytes\0" as *const u8 as *const ::core::ffi::c_char,
        );
        end_diagnostic(1 as ::core::ffi::c_int != 0);
        return RAW;
    } else {
        ucnv_close_74(cnv);
        *info = maketexstring(name_of_file) as int32_t;
        return ICUMAPPING;
    };
}
#[no_mangle]
pub unsafe extern "C" fn print_utf8_str(
    mut str: *const ::core::ffi::c_uchar,
    mut len: ::core::ffi::c_int,
) {
    loop {
        let fresh1 = len;
        len = len - 1;
        if !(fresh1 > 0 as ::core::ffi::c_int) {
            break;
        }
        let fresh2 = str;
        str = str.offset(1);
        print_raw_char(*fresh2 as UTF16_code, true_0 != 0);
    };
}
#[no_mangle]
pub unsafe extern "C" fn print_chars(
    mut str: *const ::core::ffi::c_ushort,
    mut len: ::core::ffi::c_int,
) {
    loop {
        let fresh3 = len;
        len = len - 1;
        if !(fresh3 > 0 as ::core::ffi::c_int) {
            break;
        }
        let fresh4 = str;
        str = str.offset(1);
        print_char(*fresh4 as int32_t);
    };
}
unsafe extern "C" fn load_mapping_file(
    mut s: *const ::core::ffi::c_char,
    mut e: *const ::core::ffi::c_char,
    mut byteMapping: ::core::ffi::c_char,
) -> *mut ::core::ffi::c_void {
    let mut cnv: TECkit_Converter = ::core::ptr::null_mut::<Opaque_TECkit_Converter>();
    let mut buffer: *mut ::core::ffi::c_char = malloc(
        (e.offset_from(s) as ::core::ffi::c_long + 5 as ::core::ffi::c_long) as size_t,
    ) as *mut ::core::ffi::c_char;
    let mut map: rust_input_handle_t = ::core::ptr::null_mut::<ttbc_input_handle_t>();
    strncpy(buffer, s, e.offset_from(s) as ::core::ffi::c_long as size_t);
    *buffer.offset(e.offset_from(s) as ::core::ffi::c_long as isize) = 0
        as ::core::ffi::c_char;
    strcat(buffer, b".tec\0" as *const u8 as *const ::core::ffi::c_char);
    map = ttstub_input_open(
        buffer,
        TTBC_FILE_FORMAT_MISC_FONTS,
        0 as ::core::ffi::c_int,
    );
    if !map.is_null() {
        let mut mappingSize: size_t = ttstub_input_get_size(map);
        let mut mapping: *mut Byte = malloc(mappingSize) as *mut Byte;
        let mut r: ssize_t = ttstub_input_read(
            map,
            mapping as *mut ::core::ffi::c_char,
            mappingSize,
        );
        if r < 0 as ssize_t || r as size_t != mappingSize {
            _tt_abort(
                b"could not read mapping file \"%s\"\0" as *const u8
                    as *const ::core::ffi::c_char,
                buffer,
            );
        }
        ttstub_input_close(map);
        if byteMapping as ::core::ffi::c_int != 0 as ::core::ffi::c_int {
            TECkit_CreateConverter(
                mapping,
                mappingSize as UInt32,
                false_0 as Byte,
                UTF16_NATIVE as UInt16,
                kForm_Bytes as UInt16,
                &raw mut cnv,
            );
        } else {
            TECkit_CreateConverter(
                mapping,
                mappingSize as UInt32,
                true_0 as Byte,
                UTF16_NATIVE as UInt16,
                UTF16_NATIVE as UInt16,
                &raw mut cnv,
            );
        }
        if cnv.is_null() {
            font_mapping_warning(
                buffer as *const ::core::ffi::c_void,
                strlen(buffer) as int32_t,
                2 as int32_t,
            );
        } else if get_tracing_fonts_state() > 1 as int32_t {
            font_mapping_warning(
                buffer as *const ::core::ffi::c_void,
                strlen(buffer) as int32_t,
                0 as int32_t,
            );
        }
        free(mapping as *mut ::core::ffi::c_void);
    } else {
        font_mapping_warning(
            buffer as *const ::core::ffi::c_void,
            strlen(buffer) as int32_t,
            1 as int32_t,
        );
    }
    free(buffer as *mut ::core::ffi::c_void);
    return cnv as *mut ::core::ffi::c_void;
}
static mut saved_mapping_name: *mut ::core::ffi::c_char = ::core::ptr::null::<
    ::core::ffi::c_char,
>() as *mut ::core::ffi::c_char;
#[no_mangle]
pub unsafe extern "C" fn check_for_tfm_font_mapping() {
    let mut cp: *mut ::core::ffi::c_char = strstr(
        name_of_file,
        b":mapping=\0" as *const u8 as *const ::core::ffi::c_char,
    );
    saved_mapping_name = mfree(saved_mapping_name as *mut ::core::ffi::c_void)
        as *mut ::core::ffi::c_char;
    if !cp.is_null() {
        *cp = 0 as ::core::ffi::c_char;
        cp = cp.offset(9 as ::core::ffi::c_int as isize);
        while *cp as ::core::ffi::c_int != 0 && *cp as ::core::ffi::c_int <= ' ' as i32 {
            cp = cp.offset(1);
        }
        if *cp != 0 {
            saved_mapping_name = strdup(cp);
        }
    }
}
#[no_mangle]
pub unsafe extern "C" fn load_tfm_font_mapping() -> *mut ::core::ffi::c_void {
    let mut rval: *mut ::core::ffi::c_void = NULL_0;
    if !saved_mapping_name.is_null() {
        rval = load_mapping_file(
            saved_mapping_name,
            saved_mapping_name.offset(strlen(saved_mapping_name) as isize),
            1 as ::core::ffi::c_char,
        );
        saved_mapping_name = mfree(saved_mapping_name as *mut ::core::ffi::c_void)
            as *mut ::core::ffi::c_char;
    }
    return rval;
}
#[no_mangle]
pub unsafe extern "C" fn apply_tfm_font_mapping(
    mut cnv: *mut ::core::ffi::c_void,
    mut c: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut in_0: UniChar = c as UniChar;
    let mut out: [Byte; 2] = [0; 2];
    let mut inUsed: UInt32 = 0;
    let mut outUsed: UInt32 = 0;
    TECkit_ConvertBuffer(
        cnv as TECkit_Converter,
        &raw mut in_0 as *const Byte,
        ::core::mem::size_of::<UniChar>() as UInt32,
        &raw mut inUsed,
        &raw mut out as *mut Byte,
        ::core::mem::size_of::<[Byte; 2]>() as UInt32,
        &raw mut outUsed,
        1 as Byte,
    );
    TECkit_ResetConverter(cnv as TECkit_Converter);
    if outUsed < 1 as UInt32 {
        return 0 as ::core::ffi::c_int
    } else {
        return out[0 as ::core::ffi::c_int as usize] as ::core::ffi::c_int
    };
}
#[no_mangle]
pub unsafe extern "C" fn read_double(
    mut s: *mut *const ::core::ffi::c_char,
) -> ::core::ffi::c_double {
    let mut neg: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut val: ::core::ffi::c_double = 0.0f64;
    let mut cp: *const ::core::ffi::c_char = *s;
    while *cp as ::core::ffi::c_int == ' ' as i32
        || *cp as ::core::ffi::c_int == '\t' as i32
    {
        cp = cp.offset(1);
    }
    if *cp as ::core::ffi::c_int == '-' as i32 {
        neg = 1 as ::core::ffi::c_int;
        cp = cp.offset(1);
    } else if *cp as ::core::ffi::c_int == '+' as i32 {
        cp = cp.offset(1);
    }
    while *cp as ::core::ffi::c_int >= '0' as i32
        && *cp as ::core::ffi::c_int <= '9' as i32
    {
        val = val * 10.0f64 + *cp as ::core::ffi::c_int as ::core::ffi::c_double
            - '0' as i32 as ::core::ffi::c_double;
        cp = cp.offset(1);
    }
    if *cp as ::core::ffi::c_int == '.' as i32 {
        let mut dec: ::core::ffi::c_double = 10.0f64;
        cp = cp.offset(1);
        while *cp as ::core::ffi::c_int >= '0' as i32
            && *cp as ::core::ffi::c_int <= '9' as i32
        {
            val = val
                + (*cp as ::core::ffi::c_int - '0' as i32) as ::core::ffi::c_double
                    / dec;
            cp = cp.offset(1);
            dec = dec * 10.0f64;
        }
    }
    *s = cp;
    return if neg != 0 { -val } else { val };
}
unsafe extern "C" fn read_tag_with_param(
    mut cp: *const ::core::ffi::c_char,
    mut param: *mut ::core::ffi::c_int,
) -> hb_tag_t {
    let mut cp2: *const ::core::ffi::c_char = ::core::ptr::null::<::core::ffi::c_char>();
    let mut tag: hb_tag_t = 0;
    cp2 = cp;
    while *cp2 as ::core::ffi::c_int != 0 && *cp2 as ::core::ffi::c_int != ':' as i32
        && *cp2 as ::core::ffi::c_int != ';' as i32
        && *cp2 as ::core::ffi::c_int != ',' as i32
        && *cp2 as ::core::ffi::c_int != '=' as i32
    {
        cp2 = cp2.offset(1);
    }
    tag = hb_tag_from_string(
        cp,
        cp2.offset_from(cp) as ::core::ffi::c_long as ::core::ffi::c_int,
    );
    cp = cp2;
    if *cp as ::core::ffi::c_int == '=' as i32 {
        let mut neg: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
        cp = cp.offset(1);
        if *cp as ::core::ffi::c_int == '-' as i32 {
            neg += 1;
            cp = cp.offset(1);
        }
        while *cp as ::core::ffi::c_int >= '0' as i32
            && *cp as ::core::ffi::c_int <= '9' as i32
        {
            *param = *param * 10 as ::core::ffi::c_int + *cp as ::core::ffi::c_int
                - '0' as i32;
            cp = cp.offset(1);
        }
        if neg != 0 {
            *param = -*param;
        }
    }
    return tag;
}
#[no_mangle]
pub unsafe extern "C" fn read_rgb_a(
    mut cp: *mut *const ::core::ffi::c_char,
) -> ::core::ffi::c_uint {
    let mut rgbValue: uint32_t = 0 as uint32_t;
    let mut alpha: uint32_t = 0 as uint32_t;
    let mut i: ::core::ffi::c_int = 0;
    i = 0 as ::core::ffi::c_int;
    while i < 6 as ::core::ffi::c_int {
        if **cp as ::core::ffi::c_int >= '0' as i32
            && **cp as ::core::ffi::c_int <= '9' as i32
        {
            rgbValue = (rgbValue << 4 as ::core::ffi::c_int)
                .wrapping_add(**cp as uint32_t)
                .wrapping_sub('0' as i32 as uint32_t);
        } else if **cp as ::core::ffi::c_int >= 'A' as i32
            && **cp as ::core::ffi::c_int <= 'F' as i32
        {
            rgbValue = (rgbValue << 4 as ::core::ffi::c_int)
                .wrapping_add(**cp as uint32_t)
                .wrapping_sub('A' as i32 as uint32_t)
                .wrapping_add(10 as uint32_t);
        } else if **cp as ::core::ffi::c_int >= 'a' as i32
            && **cp as ::core::ffi::c_int <= 'f' as i32
        {
            rgbValue = (rgbValue << 4 as ::core::ffi::c_int)
                .wrapping_add(**cp as uint32_t)
                .wrapping_sub('a' as i32 as uint32_t)
                .wrapping_add(10 as uint32_t);
        } else {
            return 0xff as ::core::ffi::c_uint
        }
        *cp = (*cp).offset(1);
        i += 1;
    }
    rgbValue <<= 8 as ::core::ffi::c_int;
    i = 0 as ::core::ffi::c_int;
    while i < 2 as ::core::ffi::c_int {
        if **cp as ::core::ffi::c_int >= '0' as i32
            && **cp as ::core::ffi::c_int <= '9' as i32
        {
            alpha = (alpha << 4 as ::core::ffi::c_int)
                .wrapping_add(**cp as uint32_t)
                .wrapping_sub('0' as i32 as uint32_t);
        } else if **cp as ::core::ffi::c_int >= 'A' as i32
            && **cp as ::core::ffi::c_int <= 'F' as i32
        {
            alpha = (alpha << 4 as ::core::ffi::c_int)
                .wrapping_add(**cp as uint32_t)
                .wrapping_sub('A' as i32 as uint32_t)
                .wrapping_add(10 as uint32_t);
        } else {
            if !(**cp as ::core::ffi::c_int >= 'a' as i32
                && **cp as ::core::ffi::c_int <= 'f' as i32)
            {
                break;
            }
            alpha = (alpha << 4 as ::core::ffi::c_int)
                .wrapping_add(**cp as uint32_t)
                .wrapping_sub('a' as i32 as uint32_t)
                .wrapping_add(10 as uint32_t);
        }
        *cp = (*cp).offset(1);
        i += 1;
    }
    if i == 2 as ::core::ffi::c_int {
        rgbValue = rgbValue.wrapping_add(alpha);
    } else {
        rgbValue = rgbValue.wrapping_add(0xff as uint32_t);
    }
    return rgbValue as ::core::ffi::c_uint;
}
#[no_mangle]
pub unsafe extern "C" fn readCommonFeatures(
    mut feat: *const ::core::ffi::c_char,
    mut end: *const ::core::ffi::c_char,
    mut extend: *mut ::core::ffi::c_float,
    mut slant: *mut ::core::ffi::c_float,
    mut embolden: *mut ::core::ffi::c_float,
    mut letterspace: *mut ::core::ffi::c_float,
    mut rgbValue: *mut uint32_t,
) -> ::core::ffi::c_int {
    let mut sep: *const ::core::ffi::c_char = ::core::ptr::null::<::core::ffi::c_char>();
    sep = strstartswith(feat, b"mapping\0" as *const u8 as *const ::core::ffi::c_char);
    if !sep.is_null() {
        if *sep as ::core::ffi::c_int != '=' as i32 {
            return -(1 as ::core::ffi::c_int);
        }
        loaded_font_mapping = load_mapping_file(
            sep.offset(1 as ::core::ffi::c_int as isize),
            end,
            0 as ::core::ffi::c_char,
        );
        return 1 as ::core::ffi::c_int;
    }
    sep = strstartswith(feat, b"extend\0" as *const u8 as *const ::core::ffi::c_char);
    if !sep.is_null() {
        if *sep as ::core::ffi::c_int != '=' as i32 {
            return -(1 as ::core::ffi::c_int);
        }
        sep = sep.offset(1);
        *extend = read_double(&raw mut sep) as ::core::ffi::c_float;
        return 1 as ::core::ffi::c_int;
    }
    sep = strstartswith(feat, b"slant\0" as *const u8 as *const ::core::ffi::c_char);
    if !sep.is_null() {
        if *sep as ::core::ffi::c_int != '=' as i32 {
            return -(1 as ::core::ffi::c_int);
        }
        sep = sep.offset(1);
        *slant = read_double(&raw mut sep) as ::core::ffi::c_float;
        return 1 as ::core::ffi::c_int;
    }
    sep = strstartswith(feat, b"embolden\0" as *const u8 as *const ::core::ffi::c_char);
    if !sep.is_null() {
        if *sep as ::core::ffi::c_int != '=' as i32 {
            return -(1 as ::core::ffi::c_int);
        }
        sep = sep.offset(1);
        *embolden = read_double(&raw mut sep) as ::core::ffi::c_float;
        return 1 as ::core::ffi::c_int;
    }
    sep = strstartswith(
        feat,
        b"letterspace\0" as *const u8 as *const ::core::ffi::c_char,
    );
    if !sep.is_null() {
        if *sep as ::core::ffi::c_int != '=' as i32 {
            return -(1 as ::core::ffi::c_int);
        }
        sep = sep.offset(1);
        *letterspace = read_double(&raw mut sep) as ::core::ffi::c_float;
        return 1 as ::core::ffi::c_int;
    }
    sep = strstartswith(feat, b"color\0" as *const u8 as *const ::core::ffi::c_char);
    if !sep.is_null() {
        let mut s: *const ::core::ffi::c_char = ::core::ptr::null::<
            ::core::ffi::c_char,
        >();
        if *sep as ::core::ffi::c_int != '=' as i32 {
            return -(1 as ::core::ffi::c_int);
        }
        sep = sep.offset(1);
        s = sep;
        *rgbValue = read_rgb_a(&raw mut sep) as uint32_t;
        if sep == s.offset(6 as ::core::ffi::c_int as isize)
            || sep == s.offset(8 as ::core::ffi::c_int as isize)
        {
            loaded_font_flags = (loaded_font_flags as ::core::ffi::c_int
                | FONT_FLAGS_COLORED) as ::core::ffi::c_char;
        } else {
            return -(1 as ::core::ffi::c_int)
        }
        return 1 as ::core::ffi::c_int;
    }
    return 0 as ::core::ffi::c_int;
}
unsafe extern "C" fn readFeatureNumber(
    mut s: *const ::core::ffi::c_char,
    mut e: *const ::core::ffi::c_char,
    mut f: *mut hb_tag_t,
    mut v: *mut ::core::ffi::c_int,
) -> bool {
    *f = 0 as hb_tag_t;
    *v = 0 as ::core::ffi::c_int;
    if (*s as ::core::ffi::c_int) < '0' as i32 || *s as ::core::ffi::c_int > '9' as i32 {
        return false_0 != 0;
    }
    while *s as ::core::ffi::c_int >= '0' as i32
        && *s as ::core::ffi::c_int <= '9' as i32
    {
        let fresh9 = s;
        s = s.offset(1);
        *f = (*f)
            .wrapping_mul(10 as hb_tag_t)
            .wrapping_add(*fresh9 as hb_tag_t)
            .wrapping_sub('0' as i32 as hb_tag_t);
    }
    while *s as ::core::ffi::c_int == ' ' as i32
        || *s as ::core::ffi::c_int == '\t' as i32
    {
        s = s.offset(1);
    }
    let fresh10 = s;
    s = s.offset(1);
    if *fresh10 as ::core::ffi::c_int != '=' as i32 {
        return false_0 != 0;
    }
    if (*s as ::core::ffi::c_int) < '0' as i32 || *s as ::core::ffi::c_int > '9' as i32 {
        return false_0 != 0;
    }
    while *s as ::core::ffi::c_int >= '0' as i32
        && *s as ::core::ffi::c_int <= '9' as i32
    {
        let fresh11 = s;
        s = s.offset(1);
        *v = *v * 10 as ::core::ffi::c_int + *fresh11 as ::core::ffi::c_int - '0' as i32;
    }
    while *s as ::core::ffi::c_int == ' ' as i32
        || *s as ::core::ffi::c_int == '\t' as i32
    {
        s = s.offset(1);
    }
    if s != e {
        return false_0 != 0;
    }
    return true_0 != 0;
}
unsafe extern "C" fn loadOTfont(
    mut fontRef: PlatformFontRef,
    mut font: XeTeXFont,
    mut scaled_size: Fixed,
    mut cp1: *mut ::core::ffi::c_char,
) -> *mut ::core::ffi::c_void {
    let mut current_block: u64;
    let mut engine: XeTeXLayoutEngine = ::core::ptr::null_mut::<XeTeXLayoutEngine_rec>();
    let mut script: hb_tag_t = HB_TAG_NONE;
    let mut language: *mut ::core::ffi::c_char = ::core::ptr::null_mut::<
        ::core::ffi::c_char,
    >();
    let mut features: *mut hb_feature_t = ::core::ptr::null_mut::<hb_feature_t>();
    let mut shapers: *mut *mut ::core::ffi::c_char = ::core::ptr::null_mut::<
        *mut ::core::ffi::c_char,
    >();
    let mut nFeatures: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut nShapers: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut cp2: *mut ::core::ffi::c_char = ::core::ptr::null_mut::<
        ::core::ffi::c_char,
    >();
    let mut cp3: *const ::core::ffi::c_char = ::core::ptr::null::<::core::ffi::c_char>();
    let mut tag: hb_tag_t = 0;
    let mut rgbValue: uint32_t = 0xff as uint32_t;
    let mut extend: ::core::ffi::c_float = 1.0f32;
    let mut slant: ::core::ffi::c_float = 0.0f32;
    let mut embolden: ::core::ffi::c_float = 0.0f32;
    let mut letterspace: ::core::ffi::c_float = 0.0f32;
    let mut i: ::core::ffi::c_int = 0;
    let mut reqEngine: ::core::ffi::c_char = getReqEngine();
    if reqEngine as ::core::ffi::c_int == 'O' as i32
        || reqEngine as ::core::ffi::c_int == 'G' as i32
    {
        shapers = realloc(
            shapers as *mut ::core::ffi::c_void,
            ((nShapers + 1 as ::core::ffi::c_int) as size_t)
                .wrapping_mul(
                    ::core::mem::size_of::<*mut ::core::ffi::c_char>() as size_t,
                ),
        ) as *mut *mut ::core::ffi::c_char;
        if reqEngine as ::core::ffi::c_int == 'O' as i32 {
            static mut ot_const: [::core::ffi::c_char; 3] = unsafe {
                ::core::mem::transmute::<[u8; 3], [::core::ffi::c_char; 3]>(*b"ot\0")
            };
            let ref mut fresh5 = *shapers.offset(nShapers as isize);
            *fresh5 = &raw mut ot_const as *mut ::core::ffi::c_char;
        } else if reqEngine as ::core::ffi::c_int == 'G' as i32 {
            static mut graphite2_const: [::core::ffi::c_char; 10] = unsafe {
                ::core::mem::transmute::<
                    [u8; 10],
                    [::core::ffi::c_char; 10],
                >(*b"graphite2\0")
            };
            let ref mut fresh6 = *shapers.offset(nShapers as isize);
            *fresh6 = &raw mut graphite2_const as *mut ::core::ffi::c_char;
        }
        nShapers += 1;
    }
    if reqEngine as ::core::ffi::c_int == 'G' as i32 {
        let mut tmpShapers: [*mut ::core::ffi::c_char; 1] = [
            *shapers.offset(0 as ::core::ffi::c_int as isize),
        ];
        engine = createLayoutEngine(
            fontRef,
            font,
            script,
            language,
            features,
            nFeatures,
            &raw mut tmpShapers as *mut *mut ::core::ffi::c_char,
            rgbValue,
            extend,
            slant,
            embolden,
        );
        if engine.is_null() {
            return NULL_0;
        }
    }
    if !cp1.is_null() {
        while *cp1 != 0 {
            if *cp1 as ::core::ffi::c_int == ':' as i32
                || *cp1 as ::core::ffi::c_int == ';' as i32
                || *cp1 as ::core::ffi::c_int == ',' as i32
            {
                cp1 = cp1.offset(1);
            }
            while *cp1 as ::core::ffi::c_int == ' ' as i32
                || *cp1 as ::core::ffi::c_int == '\t' as i32
            {
                cp1 = cp1.offset(1);
            }
            if *cp1 as ::core::ffi::c_int == 0 as ::core::ffi::c_int {
                break;
            }
            cp2 = cp1;
            while *cp2 as ::core::ffi::c_int != 0
                && *cp2 as ::core::ffi::c_int != ':' as i32
                && *cp2 as ::core::ffi::c_int != ';' as i32
                && *cp2 as ::core::ffi::c_int != ',' as i32
            {
                cp2 = cp2.offset(1);
            }
            cp3 = strstartswith(
                cp1,
                b"script\0" as *const u8 as *const ::core::ffi::c_char,
            );
            if !cp3.is_null() {
                if *cp3 as ::core::ffi::c_int != '=' as i32 {
                    current_block = 15786498606502739833;
                } else {
                    cp3 = cp3.offset(1);
                    script = hb_tag_from_string(
                        cp3,
                        cp2.offset_from(cp3) as ::core::ffi::c_long as ::core::ffi::c_int,
                    );
                    current_block = 10025626958702264870;
                }
            } else {
                cp3 = strstartswith(
                    cp1,
                    b"language\0" as *const u8 as *const ::core::ffi::c_char,
                );
                if !cp3.is_null() {
                    if *cp3 as ::core::ffi::c_int != '=' as i32 {
                        current_block = 15786498606502739833;
                    } else {
                        cp3 = cp3.offset(1);
                        language = malloc(
                            (cp2.offset_from(cp3) as ::core::ffi::c_long
                                + 1 as ::core::ffi::c_long) as size_t,
                        ) as *mut ::core::ffi::c_char;
                        *language
                            .offset(
                                cp2.offset_from(cp3) as ::core::ffi::c_long as isize,
                            ) = '\0' as i32 as ::core::ffi::c_char;
                        memcpy(
                            language as *mut ::core::ffi::c_void,
                            cp3 as *const ::core::ffi::c_void,
                            cp2.offset_from(cp3) as ::core::ffi::c_long as size_t,
                        );
                        current_block = 10025626958702264870;
                    }
                } else {
                    cp3 = strstartswith(
                        cp1,
                        b"shaper\0" as *const u8 as *const ::core::ffi::c_char,
                    );
                    if !cp3.is_null() {
                        if *cp3 as ::core::ffi::c_int != '=' as i32 {
                            current_block = 15786498606502739833;
                        } else {
                            cp3 = cp3.offset(1);
                            shapers = realloc(
                                shapers as *mut ::core::ffi::c_void,
                                ((nShapers + 1 as ::core::ffi::c_int) as size_t)
                                    .wrapping_mul(
                                        ::core::mem::size_of::<*mut ::core::ffi::c_char>() as size_t,
                                    ),
                            ) as *mut *mut ::core::ffi::c_char;
                            let ref mut fresh7 = *shapers.offset(nShapers as isize);
                            *fresh7 = strdup(cp3);
                            *(*shapers.offset(nShapers as isize))
                                .offset(
                                    cp2.offset_from(cp3) as ::core::ffi::c_long as isize,
                                ) = '\0' as i32 as ::core::ffi::c_char;
                            nShapers += 1;
                            current_block = 10025626958702264870;
                        }
                    } else {
                        i = readCommonFeatures(
                            cp1,
                            cp2,
                            &raw mut extend,
                            &raw mut slant,
                            &raw mut embolden,
                            &raw mut letterspace,
                            &raw mut rgbValue,
                        );
                        if i == 1 as ::core::ffi::c_int {
                            current_block = 10025626958702264870;
                        } else if i == -(1 as ::core::ffi::c_int) {
                            current_block = 15786498606502739833;
                        } else {
                            if reqEngine as ::core::ffi::c_int == 'G' as i32 {
                                let mut value: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
                                if readFeatureNumber(cp1, cp2, &raw mut tag, &raw mut value)
                                    as ::core::ffi::c_int != 0
                                    || findGraphiteFeature(
                                        engine,
                                        cp1,
                                        cp2,
                                        &raw mut tag,
                                        &raw mut value,
                                    ) as ::core::ffi::c_int != 0
                                {
                                    features = realloc(
                                        features as *mut ::core::ffi::c_void,
                                        ((nFeatures + 1 as ::core::ffi::c_int) as size_t)
                                            .wrapping_mul(
                                                ::core::mem::size_of::<hb_feature_t>() as size_t,
                                            ),
                                    ) as *mut hb_feature_t;
                                    (*features.offset(nFeatures as isize)).tag = tag;
                                    (*features.offset(nFeatures as isize)).value = value
                                        as uint32_t;
                                    (*features.offset(nFeatures as isize)).start = 0
                                        as ::core::ffi::c_uint;
                                    (*features.offset(nFeatures as isize)).end = -(1
                                        as ::core::ffi::c_int) as ::core::ffi::c_uint;
                                    nFeatures += 1;
                                    current_block = 10025626958702264870;
                                } else {
                                    current_block = 13826291924415791078;
                                }
                            } else {
                                current_block = 13826291924415791078;
                            }
                            match current_block {
                                10025626958702264870 => {}
                                _ => {
                                    if *cp1 as ::core::ffi::c_int == '+' as i32 {
                                        let mut param: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
                                        tag = read_tag_with_param(
                                            cp1.offset(1 as ::core::ffi::c_int as isize),
                                            &raw mut param,
                                        );
                                        features = realloc(
                                            features as *mut ::core::ffi::c_void,
                                            ((nFeatures + 1 as ::core::ffi::c_int) as size_t)
                                                .wrapping_mul(
                                                    ::core::mem::size_of::<hb_feature_t>() as size_t,
                                                ),
                                        ) as *mut hb_feature_t;
                                        (*features.offset(nFeatures as isize)).tag = tag;
                                        (*features.offset(nFeatures as isize)).start = 0
                                            as ::core::ffi::c_uint;
                                        (*features.offset(nFeatures as isize)).end = -(1
                                            as ::core::ffi::c_int) as ::core::ffi::c_uint;
                                        if param >= 0 as ::core::ffi::c_int {
                                            param += 1;
                                        }
                                        (*features.offset(nFeatures as isize)).value = param
                                            as uint32_t;
                                        nFeatures += 1;
                                        current_block = 10025626958702264870;
                                    } else if *cp1 as ::core::ffi::c_int == '-' as i32 {
                                        cp1 = cp1.offset(1);
                                        tag = hb_tag_from_string(
                                            cp1,
                                            cp2.offset_from(cp1) as ::core::ffi::c_long
                                                as ::core::ffi::c_int,
                                        );
                                        features = realloc(
                                            features as *mut ::core::ffi::c_void,
                                            ((nFeatures + 1 as ::core::ffi::c_int) as size_t)
                                                .wrapping_mul(
                                                    ::core::mem::size_of::<hb_feature_t>() as size_t,
                                                ),
                                        ) as *mut hb_feature_t;
                                        (*features.offset(nFeatures as isize)).tag = tag;
                                        (*features.offset(nFeatures as isize)).start = 0
                                            as ::core::ffi::c_uint;
                                        (*features.offset(nFeatures as isize)).end = -(1
                                            as ::core::ffi::c_int) as ::core::ffi::c_uint;
                                        (*features.offset(nFeatures as isize)).value = 0
                                            as uint32_t;
                                        nFeatures += 1;
                                        current_block = 10025626958702264870;
                                    } else if !strstartswith(
                                            cp1,
                                            b"vertical\0" as *const u8 as *const ::core::ffi::c_char,
                                        )
                                        .is_null()
                                    {
                                        cp3 = cp2;
                                        if *cp3 as ::core::ffi::c_int == ';' as i32
                                            || *cp3 as ::core::ffi::c_int == ':' as i32
                                            || *cp3 as ::core::ffi::c_int == ',' as i32
                                        {
                                            cp3 = cp3.offset(-1);
                                        }
                                        while *cp3 as ::core::ffi::c_int == '\0' as i32
                                            || *cp3 as ::core::ffi::c_int == ' ' as i32
                                            || *cp3 as ::core::ffi::c_int == '\t' as i32
                                        {
                                            cp3 = cp3.offset(-1);
                                        }
                                        if *cp3 != 0 {
                                            cp3 = cp3.offset(1);
                                        }
                                        if cp3
                                            == cp1.offset(8 as ::core::ffi::c_int as isize)
                                                as *const ::core::ffi::c_char
                                        {
                                            loaded_font_flags = (loaded_font_flags as ::core::ffi::c_int
                                                | FONT_FLAGS_VERTICAL) as ::core::ffi::c_char;
                                            current_block = 10025626958702264870;
                                        } else {
                                            current_block = 15786498606502739833;
                                        }
                                    } else {
                                        current_block = 15786498606502739833;
                                    }
                                }
                            }
                        }
                    }
                }
            }
            match current_block {
                15786498606502739833 => {
                    font_feature_warning(
                        cp1 as *mut ::core::ffi::c_void,
                        cp2.offset_from(cp1) as ::core::ffi::c_long as int32_t,
                        ::core::ptr::null::<::core::ffi::c_void>(),
                        0 as int32_t,
                    );
                }
                _ => {}
            }
            cp1 = cp2;
        }
    }
    if !shapers.is_null() {
        shapers = realloc(
            shapers as *mut ::core::ffi::c_void,
            ((nShapers + 1 as ::core::ffi::c_int) as size_t)
                .wrapping_mul(
                    ::core::mem::size_of::<*mut ::core::ffi::c_char>() as size_t,
                ),
        ) as *mut *mut ::core::ffi::c_char;
        let ref mut fresh8 = *shapers.offset(nShapers as isize);
        *fresh8 = ::core::ptr::null_mut::<::core::ffi::c_char>();
    }
    if embolden as ::core::ffi::c_double != 0.0f64 {
        embolden = (embolden as ::core::ffi::c_double * Fix2D(scaled_size) / 100.0f64)
            as ::core::ffi::c_float;
    }
    if letterspace as ::core::ffi::c_double != 0.0f64 {
        loaded_font_letter_space = (letterspace as ::core::ffi::c_double / 100.0f64
            * scaled_size as ::core::ffi::c_double) as scaled_t;
    }
    if loaded_font_flags as ::core::ffi::c_int & FONT_FLAGS_COLORED
        == 0 as ::core::ffi::c_int
    {
        rgbValue = 0xff as uint32_t;
    }
    if loaded_font_flags as ::core::ffi::c_int & FONT_FLAGS_VERTICAL
        != 0 as ::core::ffi::c_int
    {
        setFontLayoutDir(font, 1 as ::core::ffi::c_int);
    }
    engine = createLayoutEngine(
        fontRef,
        font,
        script,
        language,
        features,
        nFeatures,
        shapers,
        rgbValue,
        extend,
        slant,
        embolden,
    );
    if engine.is_null() {
        free(features as *mut ::core::ffi::c_void);
        free(shapers as *mut ::core::ffi::c_void);
    } else {
        native_font_type_flag = OTGR_FONT_FLAG as int32_t;
    }
    return engine as *mut ::core::ffi::c_void;
}
unsafe extern "C" fn splitFontName(
    mut name: *mut ::core::ffi::c_char,
    mut var: *mut *mut ::core::ffi::c_char,
    mut feat: *mut *mut ::core::ffi::c_char,
    mut end: *mut *mut ::core::ffi::c_char,
    mut index: *mut ::core::ffi::c_int,
) {
    *var = ::core::ptr::null_mut::<::core::ffi::c_char>();
    *feat = ::core::ptr::null_mut::<::core::ffi::c_char>();
    *index = 0 as ::core::ffi::c_int;
    if *name as ::core::ffi::c_int == '[' as i32 {
        let mut withinFileName: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
        name = name.offset(1);
        while *name != 0 {
            if withinFileName != 0 && *name as ::core::ffi::c_int == ']' as i32 {
                withinFileName = 0 as ::core::ffi::c_int;
                if (*var).is_null() {
                    *var = name;
                }
            } else if *name as ::core::ffi::c_int == ':' as i32 {
                if withinFileName != 0 && (*var).is_null() {
                    *var = name;
                    name = name.offset(1);
                    while *name as ::core::ffi::c_int >= '0' as i32
                        && *name as ::core::ffi::c_int <= '9' as i32
                    {
                        let fresh12 = name;
                        name = name.offset(1);
                        *index = *index * 10 as ::core::ffi::c_int
                            + *fresh12 as ::core::ffi::c_int - '0' as i32;
                    }
                    name = name.offset(-1);
                } else if withinFileName == 0 && (*feat).is_null() {
                    *feat = name;
                }
            }
            name = name.offset(1);
        }
        *end = name;
    } else {
        while *name != 0 {
            if *name as ::core::ffi::c_int == '/' as i32 && (*var).is_null()
                && (*feat).is_null()
            {
                *var = name;
            } else if *name as ::core::ffi::c_int == ':' as i32 && (*feat).is_null() {
                *feat = name;
            }
            name = name.offset(1);
        }
        *end = name;
    }
    if (*feat).is_null() {
        *feat = name;
    }
    if (*var).is_null() {
        *var = *feat;
    }
}
#[no_mangle]
pub unsafe extern "C" fn find_native_font(
    mut uname: *mut ::core::ffi::c_char,
    mut scaled_size: int32_t,
) -> *mut ::core::ffi::c_void {
    let mut rval: *mut ::core::ffi::c_void = NULL_0;
    let mut nameString: *mut ::core::ffi::c_char = ::core::ptr::null_mut::<
        ::core::ffi::c_char,
    >();
    let mut var: *mut ::core::ffi::c_char = ::core::ptr::null_mut::<
        ::core::ffi::c_char,
    >();
    let mut feat: *mut ::core::ffi::c_char = ::core::ptr::null_mut::<
        ::core::ffi::c_char,
    >();
    let mut end: *mut ::core::ffi::c_char = ::core::ptr::null_mut::<
        ::core::ffi::c_char,
    >();
    let mut name: *mut ::core::ffi::c_char = uname;
    let mut varString: *mut ::core::ffi::c_char = ::core::ptr::null_mut::<
        ::core::ffi::c_char,
    >();
    let mut featString: *mut ::core::ffi::c_char = ::core::ptr::null_mut::<
        ::core::ffi::c_char,
    >();
    let mut fontRef: PlatformFontRef = ::core::ptr::null_mut::<FcPattern>();
    let mut font: XeTeXFont = ::core::ptr::null_mut::<XeTeXFont_rec>();
    let mut index: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    loaded_font_mapping = NULL_0;
    loaded_font_flags = 0 as ::core::ffi::c_char;
    loaded_font_letter_space = 0 as ::core::ffi::c_int as scaled_t;
    splitFontName(name, &raw mut var, &raw mut feat, &raw mut end, &raw mut index);
    nameString = malloc(
        (var.offset_from(name) as ::core::ffi::c_long + 1 as ::core::ffi::c_long)
            as size_t,
    ) as *mut ::core::ffi::c_char;
    strncpy(nameString, name, var.offset_from(name) as ::core::ffi::c_long as size_t);
    *nameString.offset(var.offset_from(name) as ::core::ffi::c_long as isize) = 0
        as ::core::ffi::c_char;
    if feat > var {
        varString = malloc(feat.offset_from(var) as ::core::ffi::c_long as size_t)
            as *mut ::core::ffi::c_char;
        strncpy(
            varString,
            var.offset(1 as ::core::ffi::c_int as isize),
            (feat.offset_from(var) as ::core::ffi::c_long - 1 as ::core::ffi::c_long)
                as size_t,
        );
        *varString
            .offset(
                (feat.offset_from(var) as ::core::ffi::c_long - 1 as ::core::ffi::c_long)
                    as isize,
            ) = 0 as ::core::ffi::c_char;
    }
    if end > feat {
        featString = malloc(end.offset_from(feat) as ::core::ffi::c_long as size_t)
            as *mut ::core::ffi::c_char;
        strncpy(
            featString,
            feat.offset(1 as ::core::ffi::c_int as isize),
            (end.offset_from(feat) as ::core::ffi::c_long - 1 as ::core::ffi::c_long)
                as size_t,
        );
        *featString
            .offset(
                (end.offset_from(feat) as ::core::ffi::c_long - 1 as ::core::ffi::c_long)
                    as isize,
            ) = 0 as ::core::ffi::c_char;
    }
    if *nameString.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
        == '[' as i32
    {
        if scaled_size < 0 as int32_t {
            font = createFontFromFile(
                nameString.offset(1 as ::core::ffi::c_int as isize),
                index,
                655360 as Fixed,
            );
            if !font.is_null() {
                let mut dsize: Fixed = D2Fix(getDesignSize(font));
                if scaled_size == -(1000 as int32_t) {
                    scaled_size = dsize as int32_t;
                } else {
                    scaled_size = xn_over_d(
                        dsize as scaled_t,
                        -scaled_size,
                        1000 as int32_t,
                    ) as int32_t;
                }
                deleteFont(font);
            }
        }
        font = createFontFromFile(
            nameString.offset(1 as ::core::ffi::c_int as isize),
            index,
            scaled_size as Fixed,
        );
        if !font.is_null() {
            loaded_font_design_size = D2Fix(getDesignSize(font));
            setReqEngine(0 as ::core::ffi::c_char);
            if !varString.is_null() {
                if !strstartswith(
                        varString,
                        b"/AAT\0" as *const u8 as *const ::core::ffi::c_char,
                    )
                    .is_null()
                {
                    setReqEngine('A' as i32 as ::core::ffi::c_char);
                } else if !strstartswith(
                        varString,
                        b"/OT\0" as *const u8 as *const ::core::ffi::c_char,
                    )
                    .is_null()
                    || !strstartswith(
                            varString,
                            b"/ICU\0" as *const u8 as *const ::core::ffi::c_char,
                        )
                        .is_null()
                {
                    setReqEngine('O' as i32 as ::core::ffi::c_char);
                } else if !strstartswith(
                        varString,
                        b"/GR\0" as *const u8 as *const ::core::ffi::c_char,
                    )
                    .is_null()
                {
                    setReqEngine('G' as i32 as ::core::ffi::c_char);
                }
            }
            rval = loadOTfont(
                ::core::ptr::null_mut::<FcPattern>(),
                font,
                scaled_size as Fixed,
                featString,
            );
            if rval.is_null() {
                deleteFont(font);
            }
            if !rval.is_null() && get_tracing_fonts_state() > 0 as int32_t {
                begin_diagnostic();
                print_nl(' ' as i32);
                print_c_string(b"-> \0" as *const u8 as *const ::core::ffi::c_char);
                print_c_string(nameString.offset(1 as ::core::ffi::c_int as isize));
                end_diagnostic(0 as ::core::ffi::c_int != 0);
            }
        }
    } else {
        extern "C" {
            #[link_name = "pitex_guard_platform_font"]
            fn pitex_guard_platform_font_0();
        }
        pitex_guard_platform_font_0();
        fontRef = findFontByName(nameString, varString, Fix2D(scaled_size as Fixed));
        if get_tracing_fonts_state() > 0 as int32_t {
            begin_diagnostic();
            print_nl(' ' as i32);
            print_c_string(b"-> \0" as *const u8 as *const ::core::ffi::c_char);
            print_c_string(ttxl_platfont_get_desc(fontRef));
            end_diagnostic(0 as ::core::ffi::c_int != 0);
        }
        if !fontRef.is_null() {
            let mut fullName: *const ::core::ffi::c_char = getFullName(fontRef);
            name_length = strlen(fullName) as int32_t;
            if !featString.is_null() {
                name_length = (name_length as size_t)
                    .wrapping_add(strlen(featString).wrapping_add(1 as size_t))
                    as int32_t as int32_t;
            }
            if !varString.is_null() {
                name_length = (name_length as size_t)
                    .wrapping_add(strlen(varString).wrapping_add(1 as size_t)) as int32_t
                    as int32_t;
            }
            free(name_of_file as *mut ::core::ffi::c_void);
            name_of_file = malloc((name_length + 1 as int32_t) as size_t)
                as *mut ::core::ffi::c_char;
            strcpy(name_of_file, fullName);
            if scaled_size < 0 as int32_t {
                font = createFont(fontRef, scaled_size as Fixed);
                if !font.is_null() {
                    let mut dsize_0: Fixed = D2Fix(getDesignSize(font));
                    if scaled_size == -(1000 as int32_t) {
                        scaled_size = dsize_0 as int32_t;
                    } else {
                        scaled_size = xn_over_d(
                            dsize_0 as scaled_t,
                            -scaled_size,
                            1000 as int32_t,
                        ) as int32_t;
                    }
                    deleteFont(font);
                }
            }
            font = createFont(fontRef, scaled_size as Fixed);
            if !font.is_null() {
                rval = loadOTfont(fontRef, font, scaled_size as Fixed, featString);
                if rval.is_null() {
                    deleteFont(font);
                }
            }
            if !varString.is_null()
                && *varString as ::core::ffi::c_int != 0 as ::core::ffi::c_int
            {
                strcat(name_of_file, b"/\0" as *const u8 as *const ::core::ffi::c_char);
                strcat(name_of_file, varString);
            }
            if !featString.is_null()
                && *featString as ::core::ffi::c_int != 0 as ::core::ffi::c_int
            {
                strcat(name_of_file, b":\0" as *const u8 as *const ::core::ffi::c_char);
                strcat(name_of_file, featString);
            }
            name_length = strlen(name_of_file) as int32_t;
        }
    }
    free(varString as *mut ::core::ffi::c_void);
    free(featString as *mut ::core::ffi::c_void);
    free(nameString as *mut ::core::ffi::c_void);
    return rval;
}
#[no_mangle]
pub unsafe extern "C" fn release_font_engine(
    mut engine: *mut ::core::ffi::c_void,
    mut type_flag: ::core::ffi::c_int,
) {
    if type_flag as ::core::ffi::c_uint == OTGR_FONT_FLAG {
        deleteLayoutEngine(engine as XeTeXLayoutEngine);
    }
}
#[no_mangle]
pub unsafe extern "C" fn ot_get_font_metrics(
    mut pEngine: *mut ::core::ffi::c_void,
    mut ascent: *mut scaled_t,
    mut descent: *mut scaled_t,
    mut xheight: *mut scaled_t,
    mut capheight: *mut scaled_t,
    mut slant: *mut scaled_t,
) {
    let mut engine: XeTeXLayoutEngine = pEngine as XeTeXLayoutEngine;
    let mut a: ::core::ffi::c_float = 0.;
    let mut d: ::core::ffi::c_float = 0.;
    getAscentAndDescent(engine, &raw mut a, &raw mut d);
    *ascent = D2Fix(a as ::core::ffi::c_double) as scaled_t;
    *descent = D2Fix(d as ::core::ffi::c_double) as scaled_t;
    *slant = D2Fix(
        Fix2D(getSlant(getFont(engine)))
            * getExtendFactor(engine) as ::core::ffi::c_double
            + getSlantFactor(engine) as ::core::ffi::c_double,
    ) as scaled_t;
    getCapAndXHeight(engine, &raw mut a, &raw mut d);
    *capheight = D2Fix(a as ::core::ffi::c_double) as scaled_t;
    *xheight = D2Fix(d as ::core::ffi::c_double) as scaled_t;
    if *xheight == 0 as scaled_t {
        let mut glyphID: ::core::ffi::c_int = mapCharToGlyph(
            engine,
            'x' as i32 as uint32_t,
        ) as ::core::ffi::c_int;
        if glyphID != 0 as ::core::ffi::c_int {
            getGlyphHeightDepth(engine, glyphID as uint32_t, &raw mut a, &raw mut d);
            *xheight = D2Fix(a as ::core::ffi::c_double) as scaled_t;
        } else {
            *xheight = *ascent / 2 as scaled_t;
        }
    }
    if *capheight == 0 as scaled_t {
        let mut glyphID_0: ::core::ffi::c_int = mapCharToGlyph(
            engine,
            'X' as i32 as uint32_t,
        ) as ::core::ffi::c_int;
        if glyphID_0 != 0 as ::core::ffi::c_int {
            getGlyphHeightDepth(engine, glyphID_0 as uint32_t, &raw mut a, &raw mut d);
            *capheight = D2Fix(a as ::core::ffi::c_double) as scaled_t;
        } else {
            *capheight = *ascent;
        }
    }
}
#[no_mangle]
pub unsafe extern "C" fn ot_font_get(
    mut what: int32_t,
    mut pEngine: *mut ::core::ffi::c_void,
) -> int32_t {
    let mut engine: XeTeXLayoutEngine = pEngine as XeTeXLayoutEngine;
    let mut fontInst: XeTeXFont = getFont(engine);
    match what {
        XeTeX_count_glyphs => return countGlyphs(fontInst) as int32_t,
        XeTeX_count_features => return countGraphiteFeatures(engine) as int32_t,
        XeTeX_OT_count_scripts => return countScripts(fontInst) as int32_t,
        _ => {}
    }
    return 0 as int32_t;
}
#[no_mangle]
pub unsafe extern "C" fn ot_font_get_1(
    mut what: int32_t,
    mut pEngine: *mut ::core::ffi::c_void,
    mut param: int32_t,
) -> int32_t {
    let mut engine: XeTeXLayoutEngine = pEngine as XeTeXLayoutEngine;
    let mut fontInst: XeTeXFont = getFont(engine);
    match what {
        XeTeX_OT_count_languages => {
            return countLanguages(fontInst, param as hb_tag_t) as int32_t;
        }
        XeTeX_OT_script_code => {
            return getIndScript(fontInst, param as ::core::ffi::c_uint) as int32_t;
        }
        XeTeX_feature_code => {
            return getGraphiteFeatureCode(engine, param as uint32_t) as int32_t;
        }
        XeTeX_is_exclusive_feature => return 1 as int32_t,
        XeTeX_count_selectors => {
            return countGraphiteFeatureSettings(engine, param as uint32_t) as int32_t;
        }
        _ => {}
    }
    return 0 as int32_t;
}
#[no_mangle]
pub unsafe extern "C" fn ot_font_get_2(
    mut what: int32_t,
    mut pEngine: *mut ::core::ffi::c_void,
    mut param1: int32_t,
    mut param2: int32_t,
) -> int32_t {
    let mut engine: XeTeXLayoutEngine = pEngine as XeTeXLayoutEngine;
    let mut fontInst: XeTeXFont = getFont(engine);
    match what {
        XeTeX_OT_language_code => {
            return getIndLanguage(
                fontInst,
                param1 as hb_tag_t,
                param2 as ::core::ffi::c_uint,
            ) as int32_t;
        }
        XeTeX_OT_count_features => {
            return countFeatures(fontInst, param1 as hb_tag_t, param2 as hb_tag_t)
                as int32_t;
        }
        XeTeX_selector_code => {
            return getGraphiteFeatureSettingCode(
                engine,
                param1 as uint32_t,
                param2 as uint32_t,
            ) as int32_t;
        }
        XeTeX_is_default_selector => {
            return (getGraphiteFeatureDefaultSetting(engine, param1 as uint32_t)
                == param2 as uint32_t) as ::core::ffi::c_int;
        }
        _ => {}
    }
    return 0 as int32_t;
}
#[no_mangle]
pub unsafe extern "C" fn ot_font_get_3(
    mut what: int32_t,
    mut pEngine: *mut ::core::ffi::c_void,
    mut param1: int32_t,
    mut param2: int32_t,
    mut param3: int32_t,
) -> int32_t {
    let mut engine: XeTeXLayoutEngine = pEngine as XeTeXLayoutEngine;
    let mut fontInst: XeTeXFont = getFont(engine);
    match what {
        XeTeX_OT_feature_code => {
            return getIndFeature(
                fontInst,
                param1 as hb_tag_t,
                param2 as hb_tag_t,
                param3 as ::core::ffi::c_uint,
            ) as int32_t;
        }
        _ => {}
    }
    return 0 as int32_t;
}
#[no_mangle]
pub unsafe extern "C" fn gr_print_font_name(
    mut what: int32_t,
    mut pEngine: *mut ::core::ffi::c_void,
    mut param1: int32_t,
    mut param2: int32_t,
) {
    let mut name: *mut ::core::ffi::c_char = ::core::ptr::null_mut::<
        ::core::ffi::c_char,
    >();
    let mut engine: XeTeXLayoutEngine = pEngine as XeTeXLayoutEngine;
    match what {
        XETEX_FEATURE_NAME_CODE => {
            name = getGraphiteFeatureLabel(engine, param1 as uint32_t);
        }
        XETEX_SELECTOR_NAME_CODE => {
            name = getGraphiteFeatureSettingLabel(
                engine,
                param1 as uint32_t,
                param2 as uint32_t,
            );
        }
        _ => {}
    }
    if !name.is_null() {
        print_c_string(name);
        gr_label_destroy(name as *mut ::core::ffi::c_void);
    }
}
#[no_mangle]
pub unsafe extern "C" fn gr_font_get_named(
    mut what: int32_t,
    mut pEngine: *mut ::core::ffi::c_void,
) -> int32_t {
    let mut rval: ::core::ffi::c_long = -(1 as ::core::ffi::c_int)
        as ::core::ffi::c_long;
    let mut engine: XeTeXLayoutEngine = pEngine as XeTeXLayoutEngine;
    match what {
        XeTeX_find_feature_by_name => {
            rval = findGraphiteFeatureNamed(
                engine,
                name_of_file,
                name_length as ::core::ffi::c_int,
            );
        }
        _ => {}
    }
    return rval as int32_t;
}
#[no_mangle]
pub unsafe extern "C" fn gr_font_get_named_1(
    mut what: int32_t,
    mut pEngine: *mut ::core::ffi::c_void,
    mut param: int32_t,
) -> int32_t {
    let mut rval: ::core::ffi::c_long = -(1 as ::core::ffi::c_int)
        as ::core::ffi::c_long;
    let mut engine: XeTeXLayoutEngine = pEngine as XeTeXLayoutEngine;
    match what {
        XeTeX_find_selector_by_name => {
            rval = findGraphiteFeatureSettingNamed(
                engine,
                param as uint32_t,
                name_of_file,
                name_length as ::core::ffi::c_int,
            );
        }
        _ => {}
    }
    return rval as int32_t;
}
pub const XDV_FLAG_VERTICAL: ::core::ffi::c_int = 0x100 as ::core::ffi::c_int;
pub const XDV_FLAG_COLORED: ::core::ffi::c_int = 0x200 as ::core::ffi::c_int;
pub const XDV_FLAG_EXTEND: ::core::ffi::c_int = 0x1000 as ::core::ffi::c_int;
pub const XDV_FLAG_SLANT: ::core::ffi::c_int = 0x2000 as ::core::ffi::c_int;
pub const XDV_FLAG_EMBOLDEN: ::core::ffi::c_int = 0x4000 as ::core::ffi::c_int;
static mut xdvBufSize: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
#[no_mangle]
pub unsafe extern "C" fn makeXDVGlyphArrayData(
    mut pNode: *mut ::core::ffi::c_void,
) -> ::core::ffi::c_int {
    let mut cp: *mut ::core::ffi::c_uchar = ::core::ptr::null_mut::<
        ::core::ffi::c_uchar,
    >();
    let mut glyphIDs: *mut uint16_t = ::core::ptr::null_mut::<uint16_t>();
    let mut p: *mut memory_word = pNode as *mut memory_word;
    let mut glyph_info: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<
        ::core::ffi::c_void,
    >();
    let mut locations: *mut FixedPoint = ::core::ptr::null_mut::<FixedPoint>();
    let mut width: Fixed = 0;
    let mut glyphCount: uint16_t = (*p.offset(native_info_offset as isize)).b16.s0;
    let mut i: ::core::ffi::c_int = glyphCount as ::core::ffi::c_int
        * native_glyph_info_size + 8 as ::core::ffi::c_int;
    if i > xdvBufSize {
        free(xdv_buffer as *mut ::core::ffi::c_void);
        xdvBufSize = (i / 1024 as ::core::ffi::c_int + 1 as ::core::ffi::c_int)
            * 1024 as ::core::ffi::c_int;
        xdv_buffer = malloc(xdvBufSize as size_t) as *mut ::core::ffi::c_char;
    }
    glyph_info = (*p.offset(native_glyph_info_offset as isize)).ptr;
    locations = glyph_info as *mut FixedPoint;
    glyphIDs = locations.offset(glyphCount as ::core::ffi::c_int as isize)
        as *mut uint16_t;
    cp = xdv_buffer as *mut ::core::ffi::c_uchar;
    width = (*p.offset(width_offset as isize)).b32.s1 as Fixed;
    let fresh13 = cp;
    cp = cp.offset(1);
    *fresh13 = (width >> 24 as ::core::ffi::c_int & 0xff as Fixed)
        as ::core::ffi::c_uchar;
    let fresh14 = cp;
    cp = cp.offset(1);
    *fresh14 = (width >> 16 as ::core::ffi::c_int & 0xff as Fixed)
        as ::core::ffi::c_uchar;
    let fresh15 = cp;
    cp = cp.offset(1);
    *fresh15 = (width >> 8 as ::core::ffi::c_int & 0xff as Fixed)
        as ::core::ffi::c_uchar;
    let fresh16 = cp;
    cp = cp.offset(1);
    *fresh16 = (width & 0xff as Fixed) as ::core::ffi::c_uchar;
    let fresh17 = cp;
    cp = cp.offset(1);
    *fresh17 = (glyphCount as ::core::ffi::c_int >> 8 as ::core::ffi::c_int
        & 0xff as ::core::ffi::c_int) as ::core::ffi::c_uchar;
    let fresh18 = cp;
    cp = cp.offset(1);
    *fresh18 = (glyphCount as ::core::ffi::c_int & 0xff as ::core::ffi::c_int)
        as ::core::ffi::c_uchar;
    i = 0 as ::core::ffi::c_int;
    while i < glyphCount as ::core::ffi::c_int {
        let mut x: Fixed = (*locations.offset(i as isize)).x;
        let mut y: Fixed = (*locations.offset(i as isize)).y;
        let fresh19 = cp;
        cp = cp.offset(1);
        *fresh19 = (x >> 24 as ::core::ffi::c_int & 0xff as Fixed)
            as ::core::ffi::c_uchar;
        let fresh20 = cp;
        cp = cp.offset(1);
        *fresh20 = (x >> 16 as ::core::ffi::c_int & 0xff as Fixed)
            as ::core::ffi::c_uchar;
        let fresh21 = cp;
        cp = cp.offset(1);
        *fresh21 = (x >> 8 as ::core::ffi::c_int & 0xff as Fixed)
            as ::core::ffi::c_uchar;
        let fresh22 = cp;
        cp = cp.offset(1);
        *fresh22 = (x & 0xff as Fixed) as ::core::ffi::c_uchar;
        let fresh23 = cp;
        cp = cp.offset(1);
        *fresh23 = (y >> 24 as ::core::ffi::c_int & 0xff as Fixed)
            as ::core::ffi::c_uchar;
        let fresh24 = cp;
        cp = cp.offset(1);
        *fresh24 = (y >> 16 as ::core::ffi::c_int & 0xff as Fixed)
            as ::core::ffi::c_uchar;
        let fresh25 = cp;
        cp = cp.offset(1);
        *fresh25 = (y >> 8 as ::core::ffi::c_int & 0xff as Fixed)
            as ::core::ffi::c_uchar;
        let fresh26 = cp;
        cp = cp.offset(1);
        *fresh26 = (y & 0xff as Fixed) as ::core::ffi::c_uchar;
        i += 1;
    }
    i = 0 as ::core::ffi::c_int;
    while i < glyphCount as ::core::ffi::c_int {
        let mut g: uint16_t = *glyphIDs.offset(i as isize);
        let fresh27 = cp;
        cp = cp.offset(1);
        *fresh27 = (g as ::core::ffi::c_int >> 8 as ::core::ffi::c_int
            & 0xff as ::core::ffi::c_int) as ::core::ffi::c_uchar;
        let fresh28 = cp;
        cp = cp.offset(1);
        *fresh28 = (g as ::core::ffi::c_int & 0xff as ::core::ffi::c_int)
            as ::core::ffi::c_uchar;
        i += 1;
    }
    return (cp as *mut ::core::ffi::c_char).offset_from(xdv_buffer)
        as ::core::ffi::c_long as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn make_font_def(mut f: int32_t) -> ::core::ffi::c_int {
    let mut flags: uint16_t = 0 as uint16_t;
    let mut rgba: uint32_t = 0;
    let mut size: Fixed = 0;
    let mut filename: *mut ::core::ffi::c_char = ::core::ptr::null_mut::<
        ::core::ffi::c_char,
    >();
    let mut index: uint32_t = 0;
    let mut filenameLen: uint8_t = 0;
    let mut fontDefLength: ::core::ffi::c_int = 0;
    let mut cp: *mut ::core::ffi::c_char = ::core::ptr::null_mut::<
        ::core::ffi::c_char,
    >();
    let mut extend: ::core::ffi::c_float = 1.0f32;
    let mut slant: ::core::ffi::c_float = 0.0f32;
    let mut embolden: ::core::ffi::c_float = 0.0f32;
    if *font_area.offset(f as isize) as ::core::ffi::c_uint == OTGR_FONT_FLAG {
        let mut engine: XeTeXLayoutEngine = ::core::ptr::null_mut::<
            XeTeXLayoutEngine_rec,
        >();
        engine = *font_layout_engine.offset(f as isize) as XeTeXLayoutEngine;
        getFontRef(engine);
        filename = getFontFilename(engine, &raw mut index);
        '_c2rust_label: {
            if !filename.is_null() {} else {
                __assert_fail(
                    b"filename\0" as *const u8 as *const ::core::ffi::c_char,
                    b"xetex/engine/xetex-ext.c\0" as *const u8
                        as *const ::core::ffi::c_char,
                    1209 as ::core::ffi::c_uint,
                    b"int make_font_def(int32_t)\0" as *const u8
                        as *const ::core::ffi::c_char,
                );
            }
        };
        rgba = getRgbValue(engine);
        if *font_flags.offset(f as isize) as ::core::ffi::c_int & FONT_FLAGS_VERTICAL
            != 0 as ::core::ffi::c_int
        {
            flags = (flags as ::core::ffi::c_int | XDV_FLAG_VERTICAL) as uint16_t;
        }
        extend = getExtendFactor(engine);
        slant = getSlantFactor(engine);
        embolden = getEmboldenFactor(engine);
        size = D2Fix(getPointSize(engine) as ::core::ffi::c_double);
    } else {
        _tt_abort(
            b"bad native font flag in `make_font_def`\0" as *const u8
                as *const ::core::ffi::c_char,
        );
    }
    filenameLen = strlen(filename) as uint8_t;
    fontDefLength = 4 as ::core::ffi::c_int + 2 as ::core::ffi::c_int
        + 1 as ::core::ffi::c_int + filenameLen as ::core::ffi::c_int
        + 4 as ::core::ffi::c_int;
    if *font_flags.offset(f as isize) as ::core::ffi::c_int & FONT_FLAGS_COLORED
        != 0 as ::core::ffi::c_int
    {
        fontDefLength += 4 as ::core::ffi::c_int;
        flags = (flags as ::core::ffi::c_int | XDV_FLAG_COLORED) as uint16_t;
    }
    if extend as ::core::ffi::c_double != 1.0f64 {
        fontDefLength += 4 as ::core::ffi::c_int;
        flags = (flags as ::core::ffi::c_int | XDV_FLAG_EXTEND) as uint16_t;
    }
    if slant as ::core::ffi::c_double != 0.0f64 {
        fontDefLength += 4 as ::core::ffi::c_int;
        flags = (flags as ::core::ffi::c_int | XDV_FLAG_SLANT) as uint16_t;
    }
    if embolden as ::core::ffi::c_double != 0.0f64 {
        fontDefLength += 4 as ::core::ffi::c_int;
        flags = (flags as ::core::ffi::c_int | XDV_FLAG_EMBOLDEN) as uint16_t;
    }
    if fontDefLength > xdvBufSize {
        free(xdv_buffer as *mut ::core::ffi::c_void);
        xdvBufSize = (fontDefLength / 1024 as ::core::ffi::c_int
            + 1 as ::core::ffi::c_int) * 1024 as ::core::ffi::c_int;
        xdv_buffer = malloc(xdvBufSize as size_t) as *mut ::core::ffi::c_char;
    }
    cp = xdv_buffer;
    *(cp as *mut Fixed) = SWAP32(size as uint32_t) as Fixed;
    cp = cp.offset(4 as ::core::ffi::c_int as isize);
    *(cp as *mut uint16_t) = SWAP16(flags);
    cp = cp.offset(2 as ::core::ffi::c_int as isize);
    *(cp as *mut uint8_t) = filenameLen;
    cp = cp.offset(1 as ::core::ffi::c_int as isize);
    memcpy(
        cp as *mut ::core::ffi::c_void,
        filename as *const ::core::ffi::c_void,
        filenameLen as size_t,
    );
    cp = cp.offset(filenameLen as ::core::ffi::c_int as isize);
    *(cp as *mut uint32_t) = SWAP32(index);
    cp = cp.offset(4 as ::core::ffi::c_int as isize);
    if *font_flags.offset(f as isize) as ::core::ffi::c_int & FONT_FLAGS_COLORED
        != 0 as ::core::ffi::c_int
    {
        *(cp as *mut uint32_t) = SWAP32(rgba);
        cp = cp.offset(4 as ::core::ffi::c_int as isize);
    }
    if flags as ::core::ffi::c_int & XDV_FLAG_EXTEND != 0 {
        let mut f_0: Fixed = D2Fix(extend as ::core::ffi::c_double);
        *(cp as *mut uint32_t) = SWAP32(f_0 as uint32_t);
        cp = cp.offset(4 as ::core::ffi::c_int as isize);
    }
    if flags as ::core::ffi::c_int & XDV_FLAG_SLANT != 0 {
        let mut f_1: Fixed = D2Fix(slant as ::core::ffi::c_double);
        *(cp as *mut uint32_t) = SWAP32(f_1 as uint32_t);
        cp = cp.offset(4 as ::core::ffi::c_int as isize);
    }
    if flags as ::core::ffi::c_int & XDV_FLAG_EMBOLDEN != 0 {
        let mut f_2: Fixed = D2Fix(embolden as ::core::ffi::c_double);
        *(cp as *mut uint32_t) = SWAP32(f_2 as uint32_t);
        cp = cp.offset(4 as ::core::ffi::c_int as isize);
    }
    free(filename as *mut ::core::ffi::c_void);
    return fontDefLength;
}
#[no_mangle]
pub unsafe extern "C" fn apply_mapping(
    mut pCnv: *mut ::core::ffi::c_void,
    mut txtPtr: *mut uint16_t,
    mut txtLen: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut cnv: TECkit_Converter = pCnv as TECkit_Converter;
    let mut inUsed: UInt32 = 0;
    let mut outUsed: UInt32 = 0;
    let mut status: TECkit_Status = 0;
    static mut outLength: UInt32 = 0 as UInt32;
    if (outLength as usize)
        < (txtLen as usize)
            .wrapping_mul(::core::mem::size_of::<UniChar>() as usize)
            .wrapping_add(32 as usize)
    {
        free(mapped_text as *mut ::core::ffi::c_void);
        outLength = (txtLen as usize)
            .wrapping_mul(::core::mem::size_of::<UniChar>() as usize)
            .wrapping_add(32 as usize) as UInt32;
        mapped_text = malloc(outLength as size_t) as *mut UTF16_code;
    }
    loop {
        status = TECkit_ConvertBuffer(
            cnv,
            txtPtr as *mut Byte,
            (txtLen as usize).wrapping_mul(::core::mem::size_of::<UniChar>() as usize)
                as UInt32,
            &raw mut inUsed,
            mapped_text as *mut Byte,
            outLength,
            &raw mut outUsed,
            true_0 as Byte,
        );
        TECkit_ResetConverter(cnv);
        match status {
            0 => {
                txtPtr = mapped_text as *mut UniChar as *mut uint16_t;
                return (outUsed as usize)
                    .wrapping_div(::core::mem::size_of::<UniChar>() as usize)
                    as ::core::ffi::c_int;
            }
            1 => {
                outLength = (outLength as ::core::ffi::c_ulong)
                    .wrapping_add(
                        (txtLen as usize)
                            .wrapping_mul(::core::mem::size_of::<UniChar>() as usize)
                            .wrapping_add(32 as usize) as ::core::ffi::c_ulong,
                    ) as UInt32 as UInt32;
                free(mapped_text as *mut ::core::ffi::c_void);
                mapped_text = malloc(outLength as size_t) as *mut UTF16_code;
            }
            _ => return 0 as ::core::ffi::c_int,
        }
    };
}
unsafe extern "C" fn snap_zone(
    mut value: *mut scaled_t,
    mut snap_value: scaled_t,
    mut fuzz: scaled_t,
) {
    let mut difference: scaled_t = *value - snap_value;
    if difference <= fuzz && difference >= -fuzz {
        *value = snap_value;
    }
}
#[no_mangle]
pub unsafe extern "C" fn get_native_char_height_depth(
    mut font: int32_t,
    mut ch: int32_t,
    mut height: *mut scaled_t,
    mut depth: *mut scaled_t,
) {
    let mut ht: ::core::ffi::c_float = 0.0f32;
    let mut dp: ::core::ffi::c_float = 0.0f32;
    let mut fuzz: Fixed = 0;
    if *font_area.offset(font as isize) as ::core::ffi::c_uint == OTGR_FONT_FLAG {
        let mut engine: XeTeXLayoutEngine = *font_layout_engine.offset(font as isize)
            as XeTeXLayoutEngine;
        let mut gid: ::core::ffi::c_int = mapCharToGlyph(engine, ch as uint32_t)
            as ::core::ffi::c_int;
        getGlyphHeightDepth(engine, gid as uint32_t, &raw mut ht, &raw mut dp);
    } else {
        _tt_abort(
            b"bad native font flag in `get_native_char_height_depth`\0" as *const u8
                as *const ::core::ffi::c_char,
        );
    }
    *height = D2Fix(ht as ::core::ffi::c_double) as scaled_t;
    *depth = D2Fix(dp as ::core::ffi::c_double) as scaled_t;
    fuzz = ((*font_info
        .offset((6 as int32_t + *param_base.offset(font as isize)) as isize))
        .b32
        .s1 / 25 as int32_t) as Fixed;
    snap_zone(depth, 0 as scaled_t, fuzz as scaled_t);
    snap_zone(height, 0 as scaled_t, fuzz as scaled_t);
    snap_zone(
        height,
        (*font_info.offset((5 as int32_t + *param_base.offset(font as isize)) as isize))
            .b32
            .s1 as scaled_t,
        fuzz as scaled_t,
    );
    snap_zone(
        height,
        (*font_info.offset((8 as int32_t + *param_base.offset(font as isize)) as isize))
            .b32
            .s1 as scaled_t,
        fuzz as scaled_t,
    );
}
#[no_mangle]
pub unsafe extern "C" fn getnativecharht(mut f: int32_t, mut c: int32_t) -> scaled_t {
    let mut h: scaled_t = 0;
    let mut d: scaled_t = 0;
    get_native_char_height_depth(f, c, &raw mut h, &raw mut d);
    return h;
}
#[no_mangle]
pub unsafe extern "C" fn getnativechardp(mut f: int32_t, mut c: int32_t) -> scaled_t {
    let mut h: scaled_t = 0;
    let mut d: scaled_t = 0;
    get_native_char_height_depth(f, c, &raw mut h, &raw mut d);
    return d;
}
#[no_mangle]
pub unsafe extern "C" fn get_native_char_sidebearings(
    mut font: int32_t,
    mut ch: int32_t,
    mut lsb: *mut scaled_t,
    mut rsb: *mut scaled_t,
) {
    let mut l: ::core::ffi::c_float = 0.;
    let mut r: ::core::ffi::c_float = 0.;
    if *font_area.offset(font as isize) as ::core::ffi::c_uint == OTGR_FONT_FLAG {
        let mut engine: XeTeXLayoutEngine = *font_layout_engine.offset(font as isize)
            as XeTeXLayoutEngine;
        let mut gid: ::core::ffi::c_int = mapCharToGlyph(engine, ch as uint32_t)
            as ::core::ffi::c_int;
        getGlyphSidebearings(engine, gid as uint32_t, &raw mut l, &raw mut r);
    } else {
        _tt_abort(
            b"bad native font flag in `get_native_char_side_bearings`\0" as *const u8
                as *const ::core::ffi::c_char,
        );
    }
    *lsb = D2Fix(l as ::core::ffi::c_double) as scaled_t;
    *rsb = D2Fix(r as ::core::ffi::c_double) as scaled_t;
}
#[no_mangle]
pub unsafe extern "C" fn get_glyph_bounds(
    mut font: int32_t,
    mut edge: int32_t,
    mut gid: int32_t,
) -> scaled_t {
    let mut a: ::core::ffi::c_float = 0.;
    let mut b: ::core::ffi::c_float = 0.;
    if *font_area.offset(font as isize) as ::core::ffi::c_uint == OTGR_FONT_FLAG {
        let mut engine: XeTeXLayoutEngine = *font_layout_engine.offset(font as isize)
            as XeTeXLayoutEngine;
        if edge & 1 as int32_t != 0 {
            getGlyphSidebearings(engine, gid as uint32_t, &raw mut a, &raw mut b);
        } else {
            getGlyphHeightDepth(engine, gid as uint32_t, &raw mut a, &raw mut b);
        }
    } else {
        _tt_abort(
            b"bad native font flag in `get_glyph_bounds`\0" as *const u8
                as *const ::core::ffi::c_char,
        );
    }
    return D2Fix((if edge <= 2 as int32_t { a } else { b }) as ::core::ffi::c_double)
        as scaled_t;
}
#[no_mangle]
pub unsafe extern "C" fn getnativecharic(mut f: int32_t, mut c: int32_t) -> scaled_t {
    let mut lsb: scaled_t = 0;
    let mut rsb: scaled_t = 0;
    get_native_char_sidebearings(f, c, &raw mut lsb, &raw mut rsb);
    if rsb < 0 as scaled_t {
        return *font_letter_space.offset(f as isize) - rsb
    } else {
        return *font_letter_space.offset(f as isize)
    };
}
#[no_mangle]
pub unsafe extern "C" fn getnativecharwd(mut f: int32_t, mut c: int32_t) -> scaled_t {
    let mut wd: scaled_t = 0 as scaled_t;
    if *font_area.offset(f as isize) as ::core::ffi::c_uint == OTGR_FONT_FLAG {
        let mut engine: XeTeXLayoutEngine = *font_layout_engine.offset(f as isize)
            as XeTeXLayoutEngine;
        let mut gid: ::core::ffi::c_int = mapCharToGlyph(engine, c as uint32_t)
            as ::core::ffi::c_int;
        wd = D2Fix(
            getGlyphWidthFromEngine(engine, gid as uint32_t) as ::core::ffi::c_double,
        ) as scaled_t;
    } else {
        _tt_abort(
            b"bad native font flag in `get_native_char_wd`\0" as *const u8
                as *const ::core::ffi::c_char,
        );
    }
    return wd;
}
#[no_mangle]
pub unsafe extern "C" fn real_get_native_glyph(
    mut pNode: *mut ::core::ffi::c_void,
    mut index: ::core::ffi::c_uint,
) -> uint16_t {
    let mut node: *mut memory_word = pNode as *mut memory_word;
    let mut locations: *mut FixedPoint = (*node
        .offset(native_glyph_info_offset as isize))
        .ptr as *mut FixedPoint;
    let mut glyphIDs: *mut uint16_t = locations
        .offset(
            (*node.offset(native_info_offset as isize)).b16.s0 as ::core::ffi::c_int
                as isize,
        ) as *mut uint16_t;
    if index >= (*node.offset(native_info_offset as isize)).b16.s0 as ::core::ffi::c_uint
    {
        return 0 as uint16_t
    } else {
        return *glyphIDs.offset(index as isize)
    };
}
#[no_mangle]
pub unsafe extern "C" fn store_justified_native_glyphs(
    mut pNode: *mut ::core::ffi::c_void,
) {
    let mut node: *mut memory_word = pNode as *mut memory_word;
    let mut f: ::core::ffi::c_uint = (*node.offset(native_info_offset as isize)).b16.s2
        as ::core::ffi::c_uint;
    let mut savedWidth: ::core::ffi::c_int = (*node.offset(width_offset as isize)).b32.s1
        as ::core::ffi::c_int;
    measure_native_node(node as *mut ::core::ffi::c_void, 0 as ::core::ffi::c_int);
    if (*node.offset(width_offset as isize)).b32.s1 != savedWidth as int32_t {
        let mut justAmount: ::core::ffi::c_double = Fix2D(
            savedWidth as Fixed - (*node.offset(width_offset as isize)).b32.s1 as Fixed,
        );
        let mut locations: *mut FixedPoint = (*node
            .offset(native_glyph_info_offset as isize))
            .ptr as *mut FixedPoint;
        let mut glyphIDs: *mut uint16_t = locations
            .offset(
                (*node.offset(native_info_offset as isize)).b16.s0 as ::core::ffi::c_int
                    as isize,
            ) as *mut uint16_t;
        let mut glyphCount: ::core::ffi::c_int = (*node
            .offset(native_info_offset as isize))
            .b16
            .s0 as ::core::ffi::c_int;
        let mut spaceCount: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
        let mut i: ::core::ffi::c_int = 0;
        let mut spaceGlyph: ::core::ffi::c_int = map_char_to_glyph(
            f as int32_t,
            ' ' as i32,
        ) as ::core::ffi::c_int;
        i = 0 as ::core::ffi::c_int;
        while i < glyphCount {
            if *glyphIDs.offset(i as isize) as ::core::ffi::c_int == spaceGlyph {
                spaceCount += 1;
            }
            i += 1;
        }
        if spaceCount > 0 as ::core::ffi::c_int {
            let mut adjustment: ::core::ffi::c_double = 0 as ::core::ffi::c_int
                as ::core::ffi::c_double;
            let mut spaceIndex: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
            i = 0 as ::core::ffi::c_int;
            while i < glyphCount {
                (*locations.offset(i as isize)).x = D2Fix(
                    Fix2D((*locations.offset(i as isize)).x) + adjustment,
                );
                if *glyphIDs.offset(i as isize) as ::core::ffi::c_int == spaceGlyph {
                    spaceIndex += 1;
                    adjustment = justAmount * spaceIndex as ::core::ffi::c_double
                        / spaceCount as ::core::ffi::c_double;
                }
                i += 1;
            }
        } else {
            i = 1 as ::core::ffi::c_int;
            while i < glyphCount {
                (*locations.offset(i as isize)).x = D2Fix(
                    Fix2D((*locations.offset(i as isize)).x)
                        + justAmount * i as ::core::ffi::c_double
                            / (glyphCount - 1 as ::core::ffi::c_int)
                                as ::core::ffi::c_double,
                );
                i += 1;
            }
        }
        (*node.offset(width_offset as isize)).b32.s1 = savedWidth as int32_t;
    }
}
#[no_mangle]
pub unsafe extern "C" fn measure_native_node(
    mut pNode: *mut ::core::ffi::c_void,
    mut use_glyph_metrics: ::core::ffi::c_int,
) {
    let mut node: *mut memory_word = pNode as *mut memory_word;
    let mut txtLen: ::core::ffi::c_int = (*node.offset(native_info_offset as isize))
        .b16
        .s1 as ::core::ffi::c_int;
    let mut txtPtr: *mut uint16_t = node.offset(NATIVE_NODE_SIZE as isize)
        as *mut uint16_t;
    let mut f: ::core::ffi::c_uint = (*node.offset(native_info_offset as isize)).b16.s2
        as ::core::ffi::c_uint;
    if *font_area.offset(f as isize) as ::core::ffi::c_uint == OTGR_FONT_FLAG {
        let mut engine: XeTeXLayoutEngine = *font_layout_engine.offset(f as isize)
            as XeTeXLayoutEngine;
        let mut locations: *mut FixedPoint = ::core::ptr::null_mut::<FixedPoint>();
        let mut glyphIDs: *mut uint16_t = ::core::ptr::null_mut::<uint16_t>();
        let mut glyphAdvances: *mut Fixed = ::core::ptr::null_mut::<Fixed>();
        let mut totalGlyphCount: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
        let mut dir: UBiDiDirection = UBIDI_LTR;
        let mut glyph_info: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<
            ::core::ffi::c_void,
        >();
        static mut positions: *mut FloatPoint = ::core::ptr::null::<FloatPoint>()
            as *mut FloatPoint;
        static mut advances: *mut ::core::ffi::c_float = ::core::ptr::null::<
            ::core::ffi::c_float,
        >() as *mut ::core::ffi::c_float;
        static mut glyphs: *mut uint32_t = ::core::ptr::null::<uint32_t>()
            as *mut uint32_t;
        let mut pBiDi: *mut UBiDi = ubidi_open_74();
        let mut errorCode: UErrorCode = U_ZERO_ERROR;
        ubidi_setPara_74(
            pBiDi,
            txtPtr as *const UChar,
            txtLen as int32_t,
            getDefaultDirection(engine) as UBiDiLevel,
            ::core::ptr::null_mut::<UBiDiLevel>(),
            &raw mut errorCode,
        );
        dir = ubidi_getDirection_74(pBiDi);
        if dir as ::core::ffi::c_uint
            == UBIDI_MIXED as ::core::ffi::c_int as ::core::ffi::c_uint
        {
            let mut nRuns: ::core::ffi::c_int = ubidi_countRuns_74(
                pBiDi,
                &raw mut errorCode,
            ) as ::core::ffi::c_int;
            let mut width: ::core::ffi::c_double = 0 as ::core::ffi::c_int
                as ::core::ffi::c_double;
            let mut i: ::core::ffi::c_int = 0;
            let mut runIndex: ::core::ffi::c_int = 0;
            let mut logicalStart: int32_t = 0;
            let mut length: int32_t = 0;
            runIndex = 0 as ::core::ffi::c_int;
            while runIndex < nRuns {
                dir = ubidi_getVisualRun_74(
                    pBiDi,
                    runIndex as int32_t,
                    &raw mut logicalStart,
                    &raw mut length,
                );
                totalGlyphCount
                    += layoutChars(
                        engine,
                        txtPtr,
                        logicalStart,
                        length,
                        txtLen as int32_t,
                        dir as ::core::ffi::c_uint
                            == UBIDI_RTL as ::core::ffi::c_int as ::core::ffi::c_uint,
                    );
                runIndex += 1;
            }
            if totalGlyphCount > 0 as ::core::ffi::c_int {
                let mut x: ::core::ffi::c_double = 0.;
                let mut y: ::core::ffi::c_double = 0.;
                glyph_info = calloc(
                    totalGlyphCount as size_t,
                    native_glyph_info_size as size_t,
                );
                locations = glyph_info as *mut FixedPoint;
                glyphIDs = locations.offset(totalGlyphCount as isize) as *mut uint16_t;
                glyphAdvances = calloc(
                    totalGlyphCount as size_t,
                    ::core::mem::size_of::<Fixed>() as size_t,
                ) as *mut Fixed;
                totalGlyphCount = 0 as ::core::ffi::c_int;
                y = 0.0f64;
                x = y;
                runIndex = 0 as ::core::ffi::c_int;
                while runIndex < nRuns {
                    let mut nGlyphs: ::core::ffi::c_int = 0;
                    dir = ubidi_getVisualRun_74(
                        pBiDi,
                        runIndex as int32_t,
                        &raw mut logicalStart,
                        &raw mut length,
                    );
                    nGlyphs = layoutChars(
                        engine,
                        txtPtr,
                        logicalStart,
                        length,
                        txtLen as int32_t,
                        dir as ::core::ffi::c_uint
                            == UBIDI_RTL as ::core::ffi::c_int as ::core::ffi::c_uint,
                    );
                    glyphs = calloc(
                        nGlyphs as size_t,
                        ::core::mem::size_of::<uint32_t>() as size_t,
                    ) as *mut uint32_t;
                    positions = calloc(
                        (nGlyphs + 1 as ::core::ffi::c_int) as size_t,
                        ::core::mem::size_of::<FloatPoint>() as size_t,
                    ) as *mut FloatPoint;
                    advances = calloc(
                        nGlyphs as size_t,
                        ::core::mem::size_of::<::core::ffi::c_float>() as size_t,
                    ) as *mut ::core::ffi::c_float;
                    getGlyphs(engine, glyphs);
                    getGlyphAdvances(engine, advances);
                    getGlyphPositions(engine, positions);
                    i = 0 as ::core::ffi::c_int;
                    while i < nGlyphs {
                        *glyphIDs.offset(totalGlyphCount as isize) = *glyphs
                            .offset(i as isize) as uint16_t;
                        (*locations.offset(totalGlyphCount as isize)).x = D2Fix(
                            (*positions.offset(i as isize)).x as ::core::ffi::c_double
                                + x,
                        );
                        (*locations.offset(totalGlyphCount as isize)).y = D2Fix(
                            (*positions.offset(i as isize)).y as ::core::ffi::c_double
                                + y,
                        );
                        *glyphAdvances.offset(totalGlyphCount as isize) = D2Fix(
                            *advances.offset(i as isize) as ::core::ffi::c_double,
                        );
                        totalGlyphCount += 1;
                        i += 1;
                    }
                    x
                        += (*positions.offset(nGlyphs as isize)).x
                            as ::core::ffi::c_double;
                    y
                        += (*positions.offset(nGlyphs as isize)).y
                            as ::core::ffi::c_double;
                    free(glyphs as *mut ::core::ffi::c_void);
                    free(positions as *mut ::core::ffi::c_void);
                    free(advances as *mut ::core::ffi::c_void);
                    runIndex += 1;
                }
                width = x;
            }
            (*node.offset(width_offset as isize)).b32.s1 = D2Fix(width) as int32_t;
            (*node.offset(native_info_offset as isize)).b16.s0 = totalGlyphCount
                as uint16_t;
            let ref mut fresh29 = (*node.offset(native_glyph_info_offset as isize)).ptr;
            *fresh29 = glyph_info;
        } else {
            let mut width_0: ::core::ffi::c_double = 0 as ::core::ffi::c_int
                as ::core::ffi::c_double;
            totalGlyphCount = layoutChars(
                engine,
                txtPtr,
                0 as int32_t,
                txtLen as int32_t,
                txtLen as int32_t,
                dir as ::core::ffi::c_uint
                    == UBIDI_RTL as ::core::ffi::c_int as ::core::ffi::c_uint,
            );
            glyphs = calloc(
                totalGlyphCount as size_t,
                ::core::mem::size_of::<uint32_t>() as size_t,
            ) as *mut uint32_t;
            positions = calloc(
                (totalGlyphCount + 1 as ::core::ffi::c_int) as size_t,
                ::core::mem::size_of::<FloatPoint>() as size_t,
            ) as *mut FloatPoint;
            advances = calloc(
                totalGlyphCount as size_t,
                ::core::mem::size_of::<::core::ffi::c_float>() as size_t,
            ) as *mut ::core::ffi::c_float;
            getGlyphs(engine, glyphs);
            getGlyphAdvances(engine, advances);
            getGlyphPositions(engine, positions);
            if totalGlyphCount > 0 as ::core::ffi::c_int {
                let mut i_0: ::core::ffi::c_int = 0;
                glyph_info = calloc(
                    totalGlyphCount as size_t,
                    native_glyph_info_size as size_t,
                );
                locations = glyph_info as *mut FixedPoint;
                glyphIDs = locations.offset(totalGlyphCount as isize) as *mut uint16_t;
                glyphAdvances = calloc(
                    totalGlyphCount as size_t,
                    ::core::mem::size_of::<Fixed>() as size_t,
                ) as *mut Fixed;
                i_0 = 0 as ::core::ffi::c_int;
                while i_0 < totalGlyphCount {
                    *glyphIDs.offset(i_0 as isize) = *glyphs.offset(i_0 as isize)
                        as uint16_t;
                    *glyphAdvances.offset(i_0 as isize) = D2Fix(
                        *advances.offset(i_0 as isize) as ::core::ffi::c_double,
                    );
                    (*locations.offset(i_0 as isize)).x = D2Fix(
                        (*positions.offset(i_0 as isize)).x as ::core::ffi::c_double,
                    );
                    (*locations.offset(i_0 as isize)).y = D2Fix(
                        (*positions.offset(i_0 as isize)).y as ::core::ffi::c_double,
                    );
                    i_0 += 1;
                }
                width_0 = (*positions.offset(totalGlyphCount as isize)).x
                    as ::core::ffi::c_double;
            }
            (*node.offset(width_offset as isize)).b32.s1 = D2Fix(width_0) as int32_t;
            (*node.offset(native_info_offset as isize)).b16.s0 = totalGlyphCount
                as uint16_t;
            let ref mut fresh30 = (*node.offset(native_glyph_info_offset as isize)).ptr;
            *fresh30 = glyph_info;
            free(glyphs as *mut ::core::ffi::c_void);
            free(positions as *mut ::core::ffi::c_void);
            free(advances as *mut ::core::ffi::c_void);
        }
        ubidi_close_74(pBiDi);
        if *font_letter_space.offset(f as isize) != 0 as scaled_t {
            let mut lsDelta: Fixed = 0 as Fixed;
            let mut lsUnit: Fixed = *font_letter_space.offset(f as isize) as Fixed;
            let mut i_1: ::core::ffi::c_int = 0;
            i_1 = 0 as ::core::ffi::c_int;
            while i_1 < totalGlyphCount {
                if *glyphAdvances.offset(i_1 as isize) == 0 as Fixed
                    && lsDelta != 0 as Fixed
                {
                    lsDelta -= lsUnit;
                }
                (*locations.offset(i_1 as isize)).x += lsDelta;
                lsDelta += lsUnit;
                i_1 += 1;
            }
            if lsDelta != 0 as Fixed {
                lsDelta -= lsUnit;
                (*node.offset(width_offset as isize)).b32.s1 += lsDelta as int32_t;
            }
        }
        free(glyphAdvances as *mut ::core::ffi::c_void);
    } else {
        _tt_abort(
            b"bad native font flag in `measure_native_node`\0" as *const u8
                as *const ::core::ffi::c_char,
        );
    }
    if use_glyph_metrics == 0 as ::core::ffi::c_int
        || (*node.offset(native_info_offset as isize)).b16.s0 as ::core::ffi::c_int
            == 0 as ::core::ffi::c_int
    {
        (*node.offset(height_offset as isize)).b32.s1 = *height_base.offset(f as isize);
        (*node.offset(depth_offset as isize)).b32.s1 = *depth_base.offset(f as isize);
    } else {
        let mut locations_0: *mut FixedPoint = (*node
            .offset(native_glyph_info_offset as isize))
            .ptr as *mut FixedPoint;
        let mut glyphIDs_0: *mut uint16_t = locations_0
            .offset(
                (*node.offset(native_info_offset as isize)).b16.s0 as ::core::ffi::c_int
                    as isize,
            ) as *mut uint16_t;
        let mut yMin: ::core::ffi::c_float = 65536.0f32;
        let mut yMax: ::core::ffi::c_float = -65536.0f64 as ::core::ffi::c_float;
        let mut i_2: ::core::ffi::c_int = 0;
        i_2 = 0 as ::core::ffi::c_int;
        while i_2
            < (*node.offset(native_info_offset as isize)).b16.s0 as ::core::ffi::c_int
        {
            let mut ht: ::core::ffi::c_float = 0.;
            let mut dp: ::core::ffi::c_float = 0.;
            let mut y_0: ::core::ffi::c_float = Fix2D(
                -(*locations_0.offset(i_2 as isize)).y,
            ) as ::core::ffi::c_float;
            let mut bbox: GlyphBBox = GlyphBBox {
                xMin: 0.,
                yMin: 0.,
                xMax: 0.,
                yMax: 0.,
            };
            if getCachedGlyphBBox(
                f as uint16_t,
                *glyphIDs_0.offset(i_2 as isize),
                &raw mut bbox,
            ) == 0 as ::core::ffi::c_int
            {
                if *font_area.offset(f as isize) as ::core::ffi::c_uint == OTGR_FONT_FLAG
                {
                    getGlyphBounds(
                        *font_layout_engine.offset(f as isize) as XeTeXLayoutEngine,
                        *glyphIDs_0.offset(i_2 as isize) as uint32_t,
                        &raw mut bbox,
                    );
                }
                cacheGlyphBBox(
                    f as uint16_t,
                    *glyphIDs_0.offset(i_2 as isize),
                    &raw mut bbox,
                );
            }
            ht = bbox.yMax;
            dp = -bbox.yMin;
            if y_0 + ht > yMax {
                yMax = y_0 + ht;
            }
            if y_0 - dp < yMin {
                yMin = y_0 - dp;
            }
            i_2 += 1;
        }
        (*node.offset(height_offset as isize)).b32.s1 = D2Fix(
            yMax as ::core::ffi::c_double,
        ) as int32_t;
        (*node.offset(depth_offset as isize)).b32.s1 = -D2Fix(
            yMin as ::core::ffi::c_double,
        ) as int32_t;
    };
}
#[no_mangle]
pub unsafe extern "C" fn real_get_native_italic_correction(
    mut pNode: *mut ::core::ffi::c_void,
) -> Fixed {
    let mut node: *mut memory_word = pNode as *mut memory_word;
    let mut f: ::core::ffi::c_uint = (*node.offset(native_info_offset as isize)).b16.s2
        as ::core::ffi::c_uint;
    let mut n: ::core::ffi::c_uint = (*node.offset(native_info_offset as isize)).b16.s0
        as ::core::ffi::c_uint;
    if n > 0 as ::core::ffi::c_uint {
        let mut locations: *mut FixedPoint = (*node
            .offset(native_glyph_info_offset as isize))
            .ptr as *mut FixedPoint;
        let mut glyphIDs: *mut uint16_t = locations.offset(n as isize) as *mut uint16_t;
        if *font_area.offset(f as isize) as ::core::ffi::c_uint == OTGR_FONT_FLAG {
            return D2Fix(
                getGlyphItalCorr(
                    *font_layout_engine.offset(f as isize) as XeTeXLayoutEngine,
                    *glyphIDs.offset(n.wrapping_sub(1 as ::core::ffi::c_uint) as isize)
                        as uint32_t,
                ) as ::core::ffi::c_double,
            ) + *font_letter_space.offset(f as isize) as Fixed;
        }
    }
    return 0 as Fixed;
}
#[no_mangle]
pub unsafe extern "C" fn real_get_native_glyph_italic_correction(
    mut pNode: *mut ::core::ffi::c_void,
) -> Fixed {
    let mut node: *mut memory_word = pNode as *mut memory_word;
    let mut gid: uint16_t = (*node.offset(native_info_offset as isize)).b16.s1;
    let mut f: ::core::ffi::c_uint = (*node.offset(native_info_offset as isize)).b16.s2
        as ::core::ffi::c_uint;
    if *font_area.offset(f as isize) as ::core::ffi::c_uint == OTGR_FONT_FLAG {
        return D2Fix(
            getGlyphItalCorr(
                *font_layout_engine.offset(f as isize) as XeTeXLayoutEngine,
                gid as uint32_t,
            ) as ::core::ffi::c_double,
        );
    }
    return 0 as Fixed;
}
#[no_mangle]
pub unsafe extern "C" fn measure_native_glyph(
    mut pNode: *mut ::core::ffi::c_void,
    mut use_glyph_metrics: ::core::ffi::c_int,
) {
    let mut node: *mut memory_word = pNode as *mut memory_word;
    let mut gid: uint16_t = (*node.offset(native_info_offset as isize)).b16.s1;
    let mut f: ::core::ffi::c_uint = (*node.offset(native_info_offset as isize)).b16.s2
        as ::core::ffi::c_uint;
    let mut ht: ::core::ffi::c_float = 0.0f32;
    let mut dp: ::core::ffi::c_float = 0.0f32;
    if *font_area.offset(f as isize) as ::core::ffi::c_uint == OTGR_FONT_FLAG {
        let mut engine: XeTeXLayoutEngine = *font_layout_engine.offset(f as isize)
            as XeTeXLayoutEngine;
        let mut fontInst: XeTeXFont = getFont(engine);
        (*node.offset(width_offset as isize)).b32.s1 = D2Fix(
            getGlyphWidth(fontInst, gid as uint32_t) as ::core::ffi::c_double,
        ) as int32_t;
        if use_glyph_metrics != 0 {
            getGlyphHeightDepth(engine, gid as uint32_t, &raw mut ht, &raw mut dp);
        }
    } else {
        _tt_abort(
            b"bad native font flag in `measure_native_glyph`\0" as *const u8
                as *const ::core::ffi::c_char,
        );
    }
    if use_glyph_metrics != 0 {
        (*node.offset(height_offset as isize)).b32.s1 = D2Fix(
            ht as ::core::ffi::c_double,
        ) as int32_t;
        (*node.offset(depth_offset as isize)).b32.s1 = D2Fix(dp as ::core::ffi::c_double)
            as int32_t;
    } else {
        (*node.offset(height_offset as isize)).b32.s1 = *height_base.offset(f as isize);
        (*node.offset(depth_offset as isize)).b32.s1 = *depth_base.offset(f as isize);
    };
}
#[no_mangle]
pub unsafe extern "C" fn map_char_to_glyph(
    mut font: int32_t,
    mut ch: int32_t,
) -> int32_t {
    if ch > 0x10ffff as int32_t || ch >= 0xd800 as int32_t && ch <= 0xdfff as int32_t {
        return 0 as int32_t;
    }
    if *font_area.offset(font as isize) as ::core::ffi::c_uint == OTGR_FONT_FLAG {
        return mapCharToGlyph(
            *font_layout_engine.offset(font as isize) as XeTeXLayoutEngine,
            ch as uint32_t,
        ) as int32_t
    } else {
        _tt_abort(
            b"bad native font flag in `map_char_to_glyph`\0" as *const u8
                as *const ::core::ffi::c_char,
        );
    };
}
#[no_mangle]
pub unsafe extern "C" fn map_glyph_to_index(mut font: int32_t) -> int32_t {
    if *font_area.offset(font as isize) as ::core::ffi::c_uint == OTGR_FONT_FLAG {
        return mapGlyphToIndex(
            *font_layout_engine.offset(font as isize) as XeTeXLayoutEngine,
            name_of_file,
        ) as int32_t
    } else {
        _tt_abort(
            b"bad native font flag in `map_glyph_to_index`\0" as *const u8
                as *const ::core::ffi::c_char,
        );
    };
}
#[no_mangle]
pub unsafe extern "C" fn get_font_char_range(
    mut font: int32_t,
    mut first: ::core::ffi::c_int,
) -> int32_t {
    if *font_area.offset(font as isize) as ::core::ffi::c_uint == OTGR_FONT_FLAG {
        return getFontCharRange(
            *font_layout_engine.offset(font as isize) as XeTeXLayoutEngine,
            first,
        ) as int32_t
    } else {
        _tt_abort(
            b"bad native font flag in `get_font_char_range'`\0" as *const u8
                as *const ::core::ffi::c_char,
        );
    };
}
#[no_mangle]
pub unsafe extern "C" fn D2Fix(mut d: ::core::ffi::c_double) -> Fixed {
    let mut rval: Fixed = (d * 65536.0f64 + 0.5f64) as Fixed;
    return rval;
}
#[no_mangle]
pub unsafe extern "C" fn Fix2D(mut f: Fixed) -> ::core::ffi::c_double {
    let mut rval: ::core::ffi::c_double = f as ::core::ffi::c_double / 65536.0f64;
    return rval;
}
#[no_mangle]
pub unsafe extern "C" fn aat_get_font_metrics(
    mut attributes: CFDictionaryRef,
    mut ascent: *mut int32_t,
    mut descent: *mut int32_t,
    mut xheight: *mut int32_t,
    mut capheight: *mut int32_t,
    mut slant: *mut int32_t,
) {}
#[no_mangle]
pub unsafe extern "C" fn aat_font_get(
    mut what: ::core::ffi::c_int,
    mut attributes: CFDictionaryRef,
) -> ::core::ffi::c_int {
    let mut rval: ::core::ffi::c_int = -(1 as ::core::ffi::c_int);
    return rval;
}
#[no_mangle]
pub unsafe extern "C" fn aat_font_get_1(
    mut what: ::core::ffi::c_int,
    mut attributes: CFDictionaryRef,
    mut param: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut rval: ::core::ffi::c_int = -(1 as ::core::ffi::c_int);
    return rval;
}
#[no_mangle]
pub unsafe extern "C" fn aat_font_get_2(
    mut what: ::core::ffi::c_int,
    mut attributes: CFDictionaryRef,
    mut param1: ::core::ffi::c_int,
    mut param2: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut rval: ::core::ffi::c_int = -(1 as ::core::ffi::c_int);
    return rval;
}
#[no_mangle]
pub unsafe extern "C" fn aat_font_get_named(
    mut what: ::core::ffi::c_int,
    mut attributes: CFDictionaryRef,
) -> ::core::ffi::c_int {
    let mut rval: ::core::ffi::c_int = -(1 as ::core::ffi::c_int);
    return rval;
}
#[no_mangle]
pub unsafe extern "C" fn aat_font_get_named_1(
    mut what: ::core::ffi::c_int,
    mut attributes: CFDictionaryRef,
    mut param: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut rval: ::core::ffi::c_int = -(1 as ::core::ffi::c_int);
    return rval;
}
#[no_mangle]
pub unsafe extern "C" fn aat_print_font_name(
    mut what: ::core::ffi::c_int,
    mut attributes: CFDictionaryRef,
    mut param1: ::core::ffi::c_int,
    mut param2: ::core::ffi::c_int,
) {}
#[no_mangle]
pub unsafe extern "C" fn print_glyph_name(mut font: int32_t, mut gid: int32_t) {
    let mut s: *const ::core::ffi::c_char = ::core::ptr::null::<::core::ffi::c_char>();
    let mut len: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    if *font_area.offset(font as isize) as ::core::ffi::c_uint == OTGR_FONT_FLAG {
        let mut engine: XeTeXLayoutEngine = *font_layout_engine.offset(font as isize)
            as XeTeXLayoutEngine;
        s = getGlyphName(getFont(engine), gid as uint16_t, &raw mut len);
    } else {
        _tt_abort(
            b"bad native font flag in `print_glyph_name`\0" as *const u8
                as *const ::core::ffi::c_char,
        );
    }
    loop {
        let fresh31 = len;
        len = len - 1;
        if !(fresh31 > 0 as ::core::ffi::c_int) {
            break;
        }
        let fresh32 = s;
        s = s.offset(1);
        print_char(*fresh32 as int32_t);
    };
}
#[no_mangle]
pub unsafe extern "C" fn real_get_native_word_cp(
    mut pNode: *mut ::core::ffi::c_void,
    mut side: ::core::ffi::c_int,
) -> int32_t {
    let mut node: *mut memory_word = pNode as *mut memory_word;
    let mut locations: *mut FixedPoint = (*node
        .offset(native_glyph_info_offset as isize))
        .ptr as *mut FixedPoint;
    let mut glyphIDs: *mut uint16_t = locations
        .offset(
            (*node.offset(native_info_offset as isize)).b16.s0 as ::core::ffi::c_int
                as isize,
        ) as *mut uint16_t;
    let mut glyphCount: uint16_t = (*node.offset(native_info_offset as isize)).b16.s0;
    let mut f: int32_t = (*node.offset(native_info_offset as isize)).b16.s2 as int32_t;
    let mut actual_glyph: uint16_t = 0;
    if glyphCount as ::core::ffi::c_int == 0 as ::core::ffi::c_int {
        return 0 as int32_t;
    }
    match side {
        LEFT_SIDE => {
            actual_glyph = *glyphIDs;
        }
        RIGHT_SIDE => {
            actual_glyph = *glyphIDs
                .offset(
                    (glyphCount as ::core::ffi::c_int - 1 as ::core::ffi::c_int) as isize,
                );
        }
        _ => {
            '_c2rust_label: {
                __assert_fail(
                    b"0\0" as *const u8 as *const ::core::ffi::c_char,
                    b"xetex/engine/xetex-ext.c\0" as *const u8
                        as *const ::core::ffi::c_char,
                    2157 as ::core::ffi::c_uint,
                    b"int32_t real_get_native_word_cp(void *, int)\0" as *const u8
                        as *const ::core::ffi::c_char,
                );
            };
        }
    }
    return get_cp_code(
        f as ::core::ffi::c_int,
        actual_glyph as ::core::ffi::c_uint,
        side,
    ) as int32_t;
}
pub const kStatus_NoError: TECkit_Status = 0 as TECkit_Status;
pub const kStatus_OutputBufferFull: TECkit_Status = 1 as TECkit_Status;
pub const kForm_Bytes: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const kForm_UTF16LE: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
#[inline]
unsafe extern "C" fn SWAP16(p: uint16_t) -> uint16_t {
    return ((p as ::core::ffi::c_int >> 8 as ::core::ffi::c_int)
        + ((p as ::core::ffi::c_int) << 8 as ::core::ffi::c_int)) as uint16_t;
}
#[inline]
unsafe extern "C" fn SWAP32(p: uint32_t) -> uint32_t {
    return (p >> 24 as ::core::ffi::c_int)
        .wrapping_add(p >> 8 as ::core::ffi::c_int & 0xff00 as uint32_t)
        .wrapping_add(p << 8 as ::core::ffi::c_int & 0xff0000 as uint32_t)
        .wrapping_add(p << 24 as ::core::ffi::c_int);
}
#[inline]
unsafe extern "C" fn mfree(
    mut ptr: *mut ::core::ffi::c_void,
) -> *mut ::core::ffi::c_void {
    free(ptr);
    return NULL;
}
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
#[inline]
unsafe extern "C" fn strstartswith(
    mut s: *const ::core::ffi::c_char,
    mut prefix: *const ::core::ffi::c_char,
) -> *const ::core::ffi::c_char {
    let mut length: size_t = 0;
    length = strlen(prefix);
    if strncmp(s, prefix, length) == 0 as ::core::ffi::c_int {
        return s.offset(length as isize);
    }
    return ::core::ptr::null::<::core::ffi::c_char>();
}
#[inline]
unsafe extern "C" fn print_c_string(mut str: *const ::core::ffi::c_char) {
    while *str != 0 {
        let fresh0 = str;
        str = str.offset(1);
        print_char(*fresh0 as int32_t);
    }
}
pub const AUTO: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const UTF8: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const UTF16BE: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const UTF16LE: ::core::ffi::c_int = 3 as ::core::ffi::c_int;
pub const RAW: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
pub const ICUMAPPING: ::core::ffi::c_int = 5 as ::core::ffi::c_int;
pub const XETEX_FEATURE_NAME_CODE: ::core::ffi::c_int = 35;
pub const XETEX_SELECTOR_NAME_CODE: ::core::ffi::c_int = 36;
pub const XETEX_INT: ::core::ffi::c_int = 27 as ::core::ffi::c_int;
pub const XETEX_COUNT_GLYPHS_CODE: ::core::ffi::c_int = 28 as ::core::ffi::c_int;
pub const XETEX_COUNT_FEATURES_CODE: ::core::ffi::c_int = 35 as ::core::ffi::c_int;
pub const XETEX_FEATURE_CODE_CODE: ::core::ffi::c_int = 36 as ::core::ffi::c_int;
pub const XETEX_FIND_FEATURE_BY_NAME_CODE: ::core::ffi::c_int = 37 as ::core::ffi::c_int;
pub const XETEX_IS_EXCLUSIVE_FEATURE_CODE: ::core::ffi::c_int = 38 as ::core::ffi::c_int;
pub const XETEX_COUNT_SELECTORS_CODE: ::core::ffi::c_int = 39 as ::core::ffi::c_int;
pub const XETEX_SELECTOR_CODE_CODE: ::core::ffi::c_int = 40 as ::core::ffi::c_int;
pub const XETEX_FIND_SELECTOR_BY_NAME_CODE: ::core::ffi::c_int = 41
    as ::core::ffi::c_int;
pub const XETEX_IS_DEFAULT_SELECTOR_CODE: ::core::ffi::c_int = 42 as ::core::ffi::c_int;
pub const XETEX_OT_COUNT_SCRIPTS_CODE: ::core::ffi::c_int = 43 as ::core::ffi::c_int;
pub const XETEX_OT_COUNT_LANGUAGES_CODE: ::core::ffi::c_int = 44 as ::core::ffi::c_int;
pub const XETEX_OT_COUNT_FEATURES_CODE: ::core::ffi::c_int = 45 as ::core::ffi::c_int;
pub const XETEX_OT_SCRIPT_CODE: ::core::ffi::c_int = 46 as ::core::ffi::c_int;
pub const XETEX_OT_LANGUAGE_CODE: ::core::ffi::c_int = 47 as ::core::ffi::c_int;
pub const XETEX_OT_FEATURE_CODE: ::core::ffi::c_int = 48 as ::core::ffi::c_int;
pub const true_0: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const false_0: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const NATIVE_NODE_SIZE: ::core::ffi::c_int = 6 as ::core::ffi::c_int;
