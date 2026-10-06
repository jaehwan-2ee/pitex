// Native bibliography differential tools include the preview's original Rust
// auxiliary implementation, which uses the same installed ICU library.
use std::{env, process::Command};
fn pkg_config(argument: &str, packages: &[&str]) -> Option<String> {
    let output = Command::new(env::var("PKG_CONFIG").unwrap_or_else(|_| "pkg-config".into()))
        .arg(argument)
        .args(packages)
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8(output.stdout).ok())
        .flatten()
}
fn main() {
    println!("cargo:rerun-if-env-changed=PKG_CONFIG");
    println!("cargo:rerun-if-env-changed=PKG_CONFIG_PATH");
    println!("cargo:rustc-check-cfg=cfg(pitex_native_icu)");
    // Windows packaging/provenance tools and other non-bibliography tools must
    // build without ICU or pkg-config. Differential auxiliary tests need ICU.
    if env::var("TARGET").unwrap_or_default().contains("windows")
        || env::var("TARGET").ok() != env::var("HOST").ok()
    {
        return;
    }
    let (Some(version), Some(libraries)) = (
        pkg_config("--modversion", &["icu-uc"]),
        pkg_config("--libs", &["icu-i18n", "icu-uc"]),
    )
    else {
        return;
    };
    let major = version.trim().split('.').next().unwrap();
    assert!(!major.is_empty() && major.chars().all(|c| c.is_ascii_digit()));
    println!("cargo:rustc-env=PITEX_PREVIEW_ICU_MAJOR={major}");
    println!("cargo:rustc-cfg=pitex_native_icu");
    for flag in libraries.split_whitespace() {
        if let Some(name) = flag.strip_prefix("-l") {
            println!("cargo:rustc-link-lib={name}");
        } else if let Some(directory) = flag.strip_prefix("-L") {
            println!("cargo:rustc-link-search=native={directory}");
        } else {
            println!("cargo:rustc-link-arg={flag}");
        }
    }
}
