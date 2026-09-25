//! Windows + `markdown-preview` only: the app resolves `WebView2Loader.dll`
//! at runtime with `LoadLibraryW`, so a missing loader only disables the
//! preview — but ship it anyway. Copy the vendored copy out of the sys
//! crate's OUT_DIR into `target/<profile>` and `deps/` so `cargo build`/
//! `run`/`test` work without extra packaging — `Windows/build.sh` then
//! picks it up into the dist bundle.

fn main() {
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
