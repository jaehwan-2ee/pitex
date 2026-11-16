//! Two build-time jobs, each gated on its feature/target:
//!
//! - `equation-preview`: embed the equation-preview page tree
//!   (`Assets/equation-preview`, html/js/css only) as an `include_bytes!`
//!   table for the `pitex-equation://` scheme handler — the binary carries
//!   its own renderer, no install-path lookup.
//! - Windows + either web preview feature: resolve `WebView2Loader.dll`
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
    if std::env::var("CARGO_FEATURE_EQUATION_PREVIEW").is_err() {
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
        || (std::env::var("CARGO_FEATURE_MARKDOWN_PREVIEW").is_err()
            && std::env::var("CARGO_FEATURE_EQUATION_PREVIEW").is_err())
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
    let out_dir = PathBuf::from(out_dir);
    // Cargo 1.100 nests the unit hash (`build/<pkg>/<hash>/out`). Older
    // toolchains use `build/<pkg>-<hash>/out`.
    let Some(loader) = find_webview2_loader(&out_dir, arch) else {
        println!(
            "cargo:warning=WebView2Loader.dll not found under webview2-com-sys OUT_DIR; the preview will show the runtime-unavailable page"
        );
        return;
    };
    let mut dirs = vec![out_dir.clone()];
    if let Some(profile_dir) = profile_dir(&out_dir) {
        dirs.push(profile_dir.clone());
        dirs.push(profile_dir.join("deps"));
    }
    for dir in dirs {
        if dir.is_dir() {
            let _ = std::fs::copy(&loader, dir.join("WebView2Loader.dll"));
        }
    }
}

fn profile_dir(out_dir: &Path) -> Option<PathBuf> {
    let mut cursor = out_dir.parent()?;
    for _ in 0..4 {
        if cursor.file_name()?.to_str()? == "build" {
            return cursor.parent().map(|path| path.to_path_buf());
        }
        cursor = cursor.parent()?;
    }
    None
}

fn find_webview2_loader(out_dir: &Path, arch: &str) -> Option<PathBuf> {
    let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
    let mut consider = |path: PathBuf| {
        if !path.is_file() {
            return;
        }
        let modified = std::fs::metadata(&path)
            .and_then(|meta| meta.modified())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
        if best.as_ref().map(|(time, _)| modified > *time).unwrap_or(true) {
            best = Some((modified, path));
        }
    };
    for ancestor in out_dir.ancestors().take(6) {
        let Ok(entries) = std::fs::read_dir(ancestor.join("build")) else {
            continue;
        };
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if !name.starts_with("webview2-com-sys") {
                continue;
            }
            let path = entry.path();
            consider(path.join("out").join(arch).join("WebView2Loader.dll"));
            let Ok(children) = std::fs::read_dir(&path) else {
                continue;
            };
            for child in children.flatten() {
                consider(child.path().join("out").join(arch).join("WebView2Loader.dll"));
            }
        }
    }
    best.map(|(_, path)| path)
}
