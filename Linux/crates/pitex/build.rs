use std::{env, path::PathBuf, process::Command};

fn main() {
    println!("cargo:rerun-if-changed=src/ibus_compat.c");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("linux") { return; }
    let flags = Command::new("pkg-config").args(["--cflags", "gio-2.0"]).output().expect("pkg-config");
    assert!(flags.status.success(), "GIO development headers are required");
    let object = PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("ibus_compat.o");
    let status = Command::new(env::var_os("CC").unwrap_or_else(|| "cc".into()))
        .args(["-O2", "-fPIC", "-c", "src/ibus_compat.c", "-o"])
        .arg(&object)
        .args(String::from_utf8(flags.stdout).unwrap().split_whitespace())
        .status().expect("C compiler");
    assert!(status.success(), "IBus compatibility adapter compilation failed");
    println!("cargo:rustc-link-arg={}", object.display());
    println!("cargo:rustc-link-arg=-Wl,--export-dynamic-symbol=ibus_input_context_process_key_event");
    println!("cargo:rustc-link-lib=dl");
}
