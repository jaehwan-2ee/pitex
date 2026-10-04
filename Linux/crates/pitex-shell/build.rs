//! Two build-time jobs, each gated on its feature/target:
//!
//! - Unix + `equation-preview`: embed the equation-preview page tree
//!   (`Assets/equation-preview`, html/js/css only) as an `include_bytes!`
//!   table for the `pitex-equation://` scheme handler — the binary carries
//!   its own renderer, no install-path lookup.
//! - Windows + `markdown-preview`: the app resolves `WebView2Loader.dll`
//!   at runtime with `LoadLibraryW`, so a missing loader only disables the
//!   preview — but ship it anyway. Copy the vendored copy out of the sys
//!   crate's OUT_DIR into `target/<profile>` and `deps/` so `cargo build`/
//!   `run`/`test` work without extra packaging — `Windows/build.sh` then
//!   picks it up into the dist bundle.

use std::path::{Path, PathBuf};

fn main() {
    embed_equation_assets();
    copy_webview2_loader();
}

fn embed_equation_assets() {
    if std::env::var("CARGO_CFG_TARGET_FAMILY").as_deref() != Ok("unix")
        || std::env::var("CARGO_FEATURE_EQUATION_PREVIEW").is_err()
    {
        return;
    }
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let root = manifest
        .join("../../../Assets/equation-preview")
        .canonicalize()
        .expect("Assets/equation-preview is missing");
    println!("cargo:rerun-if-changed={}", root.display());
    let mut files = Vec::new();
    collect(&root, &root, &mut files);
    files.sort();
    assert!(
        files.iter().any(|(name, _)| name == "renderer.html")
            && files.iter().any(|(name, _)| name == "vendor/mathjax/tex-svg-nofont.js"),
        "Assets/equation-preview is incomplete — run node Tools/fetch-equation-preview-assets.mjs"
    );
    let mut table = String::from("pub(crate) static EQUATION_ASSETS: &[(&str, &[u8])] = &[\n");
    for (name, path) in &files {
        println!("cargo:rerun-if-changed={}", path.display());
        table.push_str(&format!("    ({name:?}, include_bytes!({:?})),\n", path.to_string_lossy()));
    }
    table.push_str("];\n");
    let out = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR")).join("equation_assets.rs");
    std::fs::write(out, table).expect("write equation_assets.rs");
}

/// Page files only — licenses/manifests stay out of the served tree.
fn collect(root: &Path, dir: &Path, files: &mut Vec<(String, PathBuf)>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(root, &path, files);
        } else if matches!(path.extension().and_then(|e| e.to_str()), Some("html" | "js" | "css")) {
            let name = path
                .strip_prefix(root)
                .expect("inside root")
                .components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join("/");
            files.push((name, path));
        }
    }
}

fn copy_webview2_loader() {
    if std::env::var("CARGO_CFG_TARGET_FAMILY").as_deref() != Ok("windows")
        || std::env::var("CARGO_FEATURE_MARKDOWN_PREVIEW").is_err()
    {
        return;
    }
    let Ok(out_dir) = std::env::var("OUT_DIR") else {
        return;
    };
    // Mirrors webview2-com-sys's CARGO_CFG_TARGET_ARCH -> folder mapping.
    let arch = match std::env::var("CARGO_CFG_TARGET_ARCH").as_deref() {
        Ok("aarch64") => "arm64",
        Ok("x86") => "x86",
        _ => "x64",
    };
    // OUT_DIR = target/<profile>/build/pitex-shell-<hash>/out — climb to
    // target/<profile>.
    let Some(profile_dir) = std::path::Path::new(&out_dir)
        .ancestors()
        .nth(3)
        .map(|p| p.to_path_buf())
    else {
        return;
    };
    let Ok(builds) = std::fs::read_dir(profile_dir.join("build")) else {
        return;
    };
    // Several webview2-com-sys build dirs can coexist (feature-set
    // variants) — read_dir order is arbitrary, so take the newest.
    let loader = builds
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with("webview2-com-sys-")
        })
        .map(|entry| {
            entry
                .path()
                .join("out")
                .join(arch)
                .join("WebView2Loader.dll")
        })
        .filter(|path| path.is_file())
        .max_by_key(|path| {
            std::fs::metadata(path)
                .and_then(|m| m.modified())
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
        });
    let Some(loader) = loader else {
        println!(
            "cargo:warning=WebView2Loader.dll not found under webview2-com-sys OUT_DIR; the preview will show the runtime-unavailable page"
        );
        return;
    };
    for dir in [profile_dir.clone(), profile_dir.join("deps")] {
        if dir.is_dir() {
            let _ = std::fs::copy(&loader, dir.join("WebView2Loader.dll"));
        }
    }
}
