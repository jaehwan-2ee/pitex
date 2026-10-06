use std::{env, fs, path::PathBuf, process::Command};

fn main() {
    println!("cargo:rerun-if-changed=../../pitex.ico");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let icon = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("../../pitex.ico");
    let output = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let resource = output.join("pitex.rc");
    let object = output.join("pitex-icon.o");
    fs::write(&resource, format!("1 ICON \"{}\"\n", icon.to_string_lossy().replace('\\', "/")))
        .expect("write icon resource");
    let status = Command::new("windres")
        .args(["-i", resource.to_str().unwrap(), "-O", "coff", "-o", object.to_str().unwrap()])
        .status()
        .expect("windres is required (MSYS2 UCRT64 binutils)");
    assert!(status.success(), "compile Windows icon resource");
    println!("cargo:rustc-link-arg={}", object.display());
}
