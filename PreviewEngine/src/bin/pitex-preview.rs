// Pitex embedded preview: migrated Rust executable.
#![feature(c_variadic, extern_types)]
#![allow(warnings, dangerous_implicit_autorefs)]
#[macro_use]
extern crate c2rust_bitfields;

#[path = "../shared/postscript.rs"]
mod shared_postscript;
#[path = "../shared/embedded_aux.rs"]
mod embedded_aux;
#[path = "../shared/backend_definitions.rs"]
mod backend_definitions;
#[path = "../shared/pdf_metadata.rs"]
mod pdf_metadata;
#[path = "../shared/preview_diagnostics.rs"]
mod preview_diagnostics;

#[path = "../shared/pdf_definitions.rs"]
mod pdf_definitions;
#[path = "../shared/font_definitions.rs"]
mod font_definitions;
#[cfg_attr(target_os = "macos", path = "../macos/driver/engine_tex.rs")]
#[cfg_attr(target_os = "linux", path = "../linux/driver/engine_tex.rs")]
mod driver_engine_tex;
#[cfg_attr(target_os = "macos", path = "../macos/driver/fonts.rs")]
#[cfg_attr(target_os = "linux", path = "../linux/driver/fonts.rs")]
mod driver_fonts;
#[cfg_attr(target_os = "macos", path = "../macos/driver/fs.rs")]
#[cfg_attr(target_os = "linux", path = "../linux/driver/fs.rs")]
mod driver_fs;
#[cfg_attr(target_os = "macos", path = "../macos/driver/images.rs")]
#[cfg_attr(target_os = "linux", path = "../linux/driver/images.rs")]
mod driver_images;
#[cfg_attr(target_os = "macos", path = "../macos/driver/json.rs")]
#[cfg_attr(target_os = "linux", path = "../linux/driver/json.rs")]
mod driver_json;
#[cfg_attr(target_os = "macos", path = "../macos/driver/main.rs")]
#[cfg_attr(target_os = "linux", path = "../linux/driver/main.rs")]
mod driver_main;
#[cfg_attr(target_os = "macos", path = "../macos/driver/myabort.rs")]
#[cfg_attr(target_os = "linux", path = "../linux/driver/myabort.rs")]
mod driver_myabort;
#[cfg_attr(target_os = "macos", path = "../macos/driver/pdfw.rs")]
#[cfg_attr(target_os = "linux", path = "../linux/driver/pdfw.rs")]
mod driver_pdfw;
#[cfg_attr(target_os = "macos", path = "../macos/driver/sprotocol.rs")]
#[cfg_attr(target_os = "linux", path = "../linux/driver/sprotocol.rs")]
mod driver_sprotocol;
#[cfg_attr(target_os = "macos", path = "../macos/driver/state.rs")]
#[cfg_attr(target_os = "linux", path = "../linux/driver/state.rs")]
mod driver_state;
#[cfg_attr(target_os = "macos", path = "../macos/driver/tbuf.rs")]
#[cfg_attr(target_os = "linux", path = "../linux/driver/tbuf.rs")]
mod driver_tbuf;
#[cfg_attr(target_os = "macos", path = "../macos/driver/xdv.rs")]
#[cfg_attr(target_os = "linux", path = "../linux/driver/xdv.rs")]
mod driver_xdv;
#[cfg_attr(target_os = "macos", path = "../macos/driver/xdv2pdf.rs")]
#[cfg_attr(target_os = "linux", path = "../linux/driver/xdv2pdf.rs")]
mod driver_xdv2pdf;
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

fn main() { driver_main::main(); }
