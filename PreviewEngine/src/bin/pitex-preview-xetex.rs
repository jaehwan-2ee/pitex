// Pitex embedded preview: migrated Rust executable.
#![feature(c_variadic, extern_types)]
#![allow(warnings, dangerous_implicit_autorefs)]
#[macro_use]
extern crate c2rust_bitfields;

#[path = "../shared/engine_strings.rs"]
mod engine_strings;
#[path = "../shared/backend_definitions.rs"]
mod backend_definitions;
#[path = "../shared/pdf_metadata.rs"]
mod pdf_metadata;

#[path = "../shared/pdf_definitions.rs"]
mod pdf_definitions;
#[path = "../shared/string_definitions.rs"]
mod string_definitions;
#[path = "../shared/font_definitions.rs"]
mod font_definitions;
#[cfg(target_os = "macos")]
#[path = "../shared/font_macos.rs"]
mod font_macos;
#[path = "../shared/font_freetype.rs"]
mod font_freetype;
#[path = "../shared/font_layout.rs"]
mod font_layout;
#[cfg(target_os = "linux")]
#[path = "../shared/fontconfig_names.rs"]
mod fontconfig_names;
#[path = "../shared/opentype_math.rs"]
mod opentype_math;
#[path = "../shared/engine_fonts.rs"]
mod engine_fonts;
#[cfg_attr(target_os = "macos", path = "../macos/shared/imginfo.rs")]
#[cfg_attr(target_os = "linux", path = "../linux/shared/imginfo.rs")]
mod shared_imginfo;
#[cfg_attr(target_os = "macos", path = "../macos/shared/pdfread.rs")]
#[cfg_attr(target_os = "linux", path = "../linux/shared/pdfread.rs")]
mod shared_pdfread;
#[cfg_attr(target_os = "macos", path = "../macos/shared/pitex_buf.rs")]
#[cfg_attr(target_os = "linux", path = "../linux/shared/pitex_buf.rs")]
mod shared_pitex_buf;
#[cfg_attr(target_os = "macos", path = "../macos/xetex/common/texlive_provider.rs")]
#[cfg_attr(target_os = "linux", path = "../linux/xetex/common/texlive_provider.rs")]
mod xetex_common_texlive_provider;
#[cfg_attr(target_os = "macos", path = "../macos/xetex/common/utils.rs")]
#[cfg_attr(target_os = "linux", path = "../linux/xetex/common/utils.rs")]
mod xetex_common_utils;
#[cfg_attr(target_os = "macos", path = "../macos/xetex/engine/xetex_engine_interface.rs")]
#[cfg_attr(target_os = "linux", path = "../linux/xetex/engine/xetex_engine_interface.rs")]
mod xetex_engine_xetex_engine_interface;
#[cfg_attr(target_os = "macos", path = "../macos/xetex/engine/xetex_errors.rs")]
#[cfg_attr(target_os = "linux", path = "../linux/xetex/engine/xetex_errors.rs")]
mod xetex_engine_xetex_errors;
#[cfg_attr(target_os = "macos", path = "../macos/xetex/engine/xetex_ext.rs")]
#[cfg_attr(target_os = "linux", path = "../linux/xetex/engine/xetex_ext.rs")]
mod xetex_engine_xetex_ext;
#[cfg_attr(target_os = "macos", path = "../macos/xetex/engine/xetex_ini.rs")]
#[cfg_attr(target_os = "linux", path = "../linux/xetex/engine/xetex_ini.rs")]
mod xetex_engine_xetex_ini;
#[cfg_attr(target_os = "macos", path = "../macos/xetex/engine/xetex_io.rs")]
#[cfg_attr(target_os = "linux", path = "../linux/xetex/engine/xetex_io.rs")]
mod xetex_engine_xetex_io;
#[cfg_attr(target_os = "macos", path = "../macos/xetex/engine/xetex_linebreak.rs")]
#[cfg_attr(target_os = "linux", path = "../linux/xetex/engine/xetex_linebreak.rs")]
mod xetex_engine_xetex_linebreak;
#[cfg_attr(target_os = "macos", path = "../macos/xetex/engine/xetex_macos.rs")]
#[cfg_attr(target_os = "linux", path = "../linux/xetex/engine/xetex_macos.rs")]
mod xetex_engine_xetex_macos;
#[cfg_attr(target_os = "macos", path = "../macos/xetex/engine/xetex_math.rs")]
#[cfg_attr(target_os = "linux", path = "../linux/xetex/engine/xetex_math.rs")]
mod xetex_engine_xetex_math;
#[cfg_attr(target_os = "macos", path = "../macos/xetex/engine/xetex_output.rs")]
#[cfg_attr(target_os = "linux", path = "../linux/xetex/engine/xetex_output.rs")]
mod xetex_engine_xetex_output;
#[cfg_attr(target_os = "macos", path = "../macos/xetex/engine/xetex_pagebuilder.rs")]
#[cfg_attr(target_os = "linux", path = "../linux/xetex/engine/xetex_pagebuilder.rs")]
mod xetex_engine_xetex_pagebuilder;
#[cfg_attr(target_os = "macos", path = "../macos/xetex/engine/xetex_pic.rs")]
#[cfg_attr(target_os = "linux", path = "../linux/xetex/engine/xetex_pic.rs")]
mod xetex_engine_xetex_pic;
#[cfg_attr(target_os = "macos", path = "../macos/xetex/engine/xetex_scaledmath.rs")]
#[cfg_attr(target_os = "linux", path = "../linux/xetex/engine/xetex_scaledmath.rs")]
mod xetex_engine_xetex_scaledmath;
#[cfg_attr(target_os = "macos", path = "../macos/xetex/engine/xetex_shipout.rs")]
#[cfg_attr(target_os = "linux", path = "../linux/xetex/engine/xetex_shipout.rs")]
mod xetex_engine_xetex_shipout;
#[cfg_attr(target_os = "macos", path = "../macos/xetex/engine/xetex_stringpool.rs")]
#[cfg_attr(target_os = "linux", path = "../linux/xetex/engine/xetex_stringpool.rs")]
mod xetex_engine_xetex_stringpool;
#[cfg_attr(target_os = "macos", path = "../macos/xetex/engine/xetex_synctex.rs")]
#[cfg_attr(target_os = "linux", path = "../linux/xetex/engine/xetex_synctex.rs")]
mod xetex_engine_xetex_synctex;
#[cfg_attr(target_os = "macos", path = "../macos/xetex/engine/xetex_texmfmp.rs")]
#[cfg_attr(target_os = "linux", path = "../linux/xetex/engine/xetex_texmfmp.rs")]
mod xetex_engine_xetex_texmfmp;
#[cfg_attr(target_os = "macos", path = "../macos/xetex/engine/xetex_xetex0.rs")]
#[cfg_attr(target_os = "linux", path = "../linux/xetex/engine/xetex_xetex0.rs")]
mod xetex_engine_xetex_xetex0;
#[cfg_attr(target_os = "macos", path = "../macos/xetex/main/fork.rs")]
#[cfg_attr(target_os = "linux", path = "../linux/xetex/main/fork.rs")]
mod xetex_main_fork;
#[cfg_attr(target_os = "macos", path = "../macos/xetex/main/formats.rs")]
#[cfg_attr(target_os = "linux", path = "../linux/xetex/main/formats.rs")]
mod xetex_main_formats;
#[cfg_attr(target_os = "macos", path = "../macos/xetex/main/main.rs")]
#[cfg_attr(target_os = "linux", path = "../linux/xetex/main/main.rs")]
mod xetex_main_main;
#[cfg_attr(target_os = "macos", path = "../macos/xetex/main/texpresso_protocol.rs")]
#[cfg_attr(target_os = "linux", path = "../linux/xetex/main/texpresso_protocol.rs")]
mod xetex_main_texpresso_protocol;
#[cfg_attr(target_os = "macos", path = "../macos/xetex/main/zlib_md5.rs")]
#[cfg_attr(target_os = "linux", path = "../linux/xetex/main/zlib_md5.rs")]
mod xetex_main_zlib_md5;

fn main() { xetex_main_main::main(); }
