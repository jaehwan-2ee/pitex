// Translated from xetex/main/formats.c with C2Rust 0.22.1.
extern "C" {
    fn sprintf(
        __s: *mut ::core::ffi::c_char,
        __format: *const ::core::ffi::c_char,
        ...
    ) -> ::core::ffi::c_int;
}
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
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<
    ::core::ffi::c_void,
>();
#[no_mangle]
pub static mut exts_enc: [*const ::core::ffi::c_char; 2] = [
    b".enc\0" as *const u8 as *const ::core::ffi::c_char,
    ::core::ptr::null::<::core::ffi::c_char>(),
];
#[no_mangle]
pub static mut exts_font_map: [*const ::core::ffi::c_char; 2] = [
    b".map\0" as *const u8 as *const ::core::ffi::c_char,
    ::core::ptr::null::<::core::ffi::c_char>(),
];
#[no_mangle]
pub static mut exts_tfm: [*const ::core::ffi::c_char; 2] = [
    b".tfm\0" as *const u8 as *const ::core::ffi::c_char,
    ::core::ptr::null::<::core::ffi::c_char>(),
];
#[no_mangle]
pub static mut exts_vf: [*const ::core::ffi::c_char; 2] = [
    b".vf\0" as *const u8 as *const ::core::ffi::c_char,
    ::core::ptr::null::<::core::ffi::c_char>(),
];
#[no_mangle]
pub static mut exts_true_type: [*const ::core::ffi::c_char; 2] = [
    b".ttf\0" as *const u8 as *const ::core::ffi::c_char,
    ::core::ptr::null::<::core::ffi::c_char>(),
];
#[no_mangle]
pub static mut exts_type1: [*const ::core::ffi::c_char; 2] = [
    b".pfb\0" as *const u8 as *const ::core::ffi::c_char,
    ::core::ptr::null::<::core::ffi::c_char>(),
];
#[no_mangle]
pub static mut exts_open_type: [*const ::core::ffi::c_char; 2] = [
    b".otf\0" as *const u8 as *const ::core::ffi::c_char,
    ::core::ptr::null::<::core::ffi::c_char>(),
];
#[no_mangle]
pub static mut exts_tex: [*const ::core::ffi::c_char; 2] = [
    b".tex\0" as *const u8 as *const ::core::ffi::c_char,
    ::core::ptr::null::<::core::ffi::c_char>(),
];
#[no_mangle]
pub static mut exts_none: [*const ::core::ffi::c_char; 1] = [
    ::core::ptr::null::<::core::ffi::c_char>(),
];
#[no_mangle]
pub unsafe extern "C" fn format_extensions(
    mut format: ttbc_file_format,
) -> *mut *const ::core::ffi::c_char {
    match format as ::core::ffi::c_uint {
        44 => return &raw mut exts_enc as *mut *const ::core::ffi::c_char,
        11 => return &raw mut exts_font_map as *mut *const ::core::ffi::c_char,
        3 => return &raw mut exts_tfm as *mut *const ::core::ffi::c_char,
        33 => return &raw mut exts_vf as *mut *const ::core::ffi::c_char,
        36 => return &raw mut exts_true_type as *mut *const ::core::ffi::c_char,
        32 => return &raw mut exts_type1 as *mut *const ::core::ffi::c_char,
        47 => return &raw mut exts_open_type as *mut *const ::core::ffi::c_char,
        26 => return &raw mut exts_tex as *mut *const ::core::ffi::c_char,
        _ => return &raw mut exts_none as *mut *const ::core::ffi::c_char,
    };
}
#[no_mangle]
pub unsafe extern "C" fn ttbc_file_format_to_string(
    mut format: ttbc_file_format,
) -> *const ::core::ffi::c_char {
    match format as ::core::ffi::c_uint {
        4 => return b"AFM\0" as *const u8 as *const ::core::ffi::c_char,
        6 => return b"BIB\0" as *const u8 as *const ::core::ffi::c_char,
        7 => return b"BST\0" as *const u8 as *const ::core::ffi::c_char,
        45 => return b"CMAP\0" as *const u8 as *const ::core::ffi::c_char,
        8 => return b"CNF\0" as *const u8 as *const ::core::ffi::c_char,
        44 => return b"ENC\0" as *const u8 as *const ::core::ffi::c_char,
        10 => return b"FORMAT\0" as *const u8 as *const ::core::ffi::c_char,
        11 => return b"FONT_MAP\0" as *const u8 as *const ::core::ffi::c_char,
        41 => return b"MISC_FONTS\0" as *const u8 as *const ::core::ffi::c_char,
        20 => return b"OFM\0" as *const u8 as *const ::core::ffi::c_char,
        47 => return b"OPEN_TYPE\0" as *const u8 as *const ::core::ffi::c_char,
        23 => return b"OVF\0" as *const u8 as *const ::core::ffi::c_char,
        25 => return b"PICT\0" as *const u8 as *const ::core::ffi::c_char,
        1 => return b"PK\0" as *const u8 as *const ::core::ffi::c_char,
        39 => return b"PROGRAM_DATA\0" as *const u8 as *const ::core::ffi::c_char,
        46 => return b"SFD\0" as *const u8 as *const ::core::ffi::c_char,
        59 => return b"TECTONIC_PRIMARY\0" as *const u8 as *const ::core::ffi::c_char,
        26 => return b"TEX\0" as *const u8 as *const ::core::ffi::c_char,
        30 => return b"TEX_PS_HEADER\0" as *const u8 as *const ::core::ffi::c_char,
        3 => return b"TFM\0" as *const u8 as *const ::core::ffi::c_char,
        36 => return b"TRUE_TYPE\0" as *const u8 as *const ::core::ffi::c_char,
        32 => return b"TYPE1\0" as *const u8 as *const ::core::ffi::c_char,
        33 => return b"VF\0" as *const u8 as *const ::core::ffi::c_char,
        _ => {}
    }
    static mut buf: [::core::ffi::c_char; 80] = [0; 80];
    sprintf(
        &raw mut buf as *mut ::core::ffi::c_char,
        b"unknown format %d\0" as *const u8 as *const ::core::ffi::c_char,
        format as ::core::ffi::c_uint,
    );
    return &raw mut buf as *mut ::core::ffi::c_char;
}
