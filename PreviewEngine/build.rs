use std::{env, fs, path::PathBuf, process::Command};

fn output(command: &mut Command) -> String {
    let result = command.output().expect("run preview-engine build tool");
    assert!(
        result.status.success(),
        "{command:?}\n{}",
        String::from_utf8_lossy(&result.stderr)
    );
    String::from_utf8(result.stdout).expect("UTF-8 build-tool output")
}

fn pkg_config(option: &str, packages: &[&str]) -> Vec<String> {
    output(
        Command::new(env::var("PKG_CONFIG").unwrap_or_else(|_| "pkg-config".into()))
            .arg(option)
            .args(packages),
    )
    .split_whitespace()
    .map(str::to_owned)
    .collect()
}

fn link_args(bin: &str, args: impl IntoIterator<Item = String>) {
    for arg in args {
        println!("cargo:rustc-link-arg-bin={bin}={arg}");
    }
}

fn main() {
    let target = env::var("TARGET").expect("Cargo target triple");
    assert!(
        matches!(target.as_str(), "aarch64-apple-darwin" | "x86_64-unknown-linux-gnu"),
        "embedded preview ABI has only been generated and validated for aarch64-apple-darwin and x86_64-unknown-linux-gnu; unsupported target: {target}"
    );
    let macos = target == "aarch64-apple-darwin";
    let icu_version = pkg_config("--modversion", &["icu-uc"]);
    let icu_major = icu_version
        .first()
        .and_then(|version| version.split('.').next())
        .filter(|major| !major.is_empty() && major.chars().all(|digit| digit.is_ascii_digit()))
        .expect("ICU pkg-config version must start with a decimal major version");
    println!("cargo:rustc-env=PITEX_PREVIEW_ICU_MAJOR={icu_major}");
    println!("cargo:rustc-check-cfg=cfg(pitex_native_icu)");
    println!("cargo:rustc-cfg=pitex_native_icu");
    // Primitive names/defaults can change without moving eqtb words. Formats
    // retain their schema fingerprint so a stale registration table is never used.
    let schema = fs::read("xetex/engine/xetex_format.h").expect("format schema header");
    let mut fingerprint = 0xcbf29ce484222325u64;
    for byte in schema {fingerprint ^= byte as u64;fingerprint = fingerprint.wrapping_mul(0x100000001b3);}
    for name in ["backend_definitions.rs","pdf_definitions.rs","string_definitions.rs","font_definitions.rs"] {
        let path=PathBuf::from("src/shared").join(name);
        println!("cargo:rerun-if-changed={}",path.display());
        if let Ok(bytes)=fs::read(path) {for byte in bytes {fingerprint ^= byte as u64;fingerprint = fingerprint.wrapping_mul(0x100000001b3);}}
    }
    println!("cargo:rustc-env=PITEX_PREVIEW_FORMAT_SCHEMA={fingerprint:016x}");
    for variable in [
        "CXX",
        "AR",
        "PKG_CONFIG",
        "PKG_CONFIG_PATH",
        "TECKIT_CFLAGS",
        "TECKIT_LIBS",
        "MACOSX_DEPLOYMENT_TARGET",
        "PITEX_PREVIEW_NATIVE_OPT",
    ] {
        println!("cargo:rerun-if-env-changed={variable}");
    }
    for directory in [
        "xetex/engine",
        "xetex/layout",
        "xetex/include",
        "xetex/common/include",
        "shared",
    ] {
        println!("cargo:rerun-if-changed={directory}");
    }

    let zlib = pkg_config("--libs", &["zlib"]);
    for bin in ["pitex-preview", "selftest"] {
        link_args(bin, zlib.clone());
        link_args(bin, pkg_config("--libs", &["freetype2"]));
        link_args(bin, pkg_config("--libs", &["icu-i18n", "icu-uc"]));
    }

    let mut packages = vec![
        "freetype2",
        "harfbuzz",
        "graphite2",
        "libpng",
        "zlib",
        "icu-uc",
        "icu-i18n",
    ];
    if !macos {
        packages.push("fontconfig");
    }
    let mut flags = vec![
        "-std=c++17".to_owned(),
        "-fPIC".to_owned(),
        "-fno-common".to_owned(),
        "-Wno-unused-result".to_owned(),
    ];
    flags.extend(
        env::var("PITEX_PREVIEW_NATIVE_OPT")
            .unwrap_or_else(|_| "-O3 -g".into())
            .split_whitespace()
            .map(str::to_owned),
    );
    flags.extend(
        [
            "-Ishared",
            "-Ixetex/common/include",
            "-Ixetex/include",
            "-Ixetex/layout",
            "-Ixetex/engine",
        ]
        .map(str::to_owned),
    );
    flags.extend(pkg_config("--cflags", &packages));
    // Upstream headers use <harfbuzz/hb.h>, while harfbuzz.pc's cflags
    // point inside that directory. Include the package's root as well.
    for directory in pkg_config("--variable=includedir", &["harfbuzz"]) {
        flags.push(format!("-I{directory}"));
    }
    flags.extend(
        env::var("TECKIT_CFLAGS")
            .map(|value| value.split_whitespace().map(str::to_owned).collect())
            .unwrap_or_else(|_| pkg_config("--cflags", &["teckit"])),
    );
    if macos {
        flags.push("-DXETEX_MAC".into());
    }

    // Font shaping and platform font access keep their upstream C++ ABI.
    // No C engine, driver, parser, or PDF writer is compiled here.
    let mut sources = Vec::new();
    sources.extend(
        fs::read_dir("xetex/layout")
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| path.extension().is_some_and(|extension| extension == "cpp")),
    );
    if macos {
        sources.push(PathBuf::from("xetex/layout/xetex-XeTeXFontMgr_Mac.mm"));
    }
    sources.sort();
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let compiler = env::var("CXX").unwrap_or_else(|_| "c++".into());
    let mut objects = Vec::new();
    for (index, source) in sources.iter().enumerate() {
        let object = out.join(format!("font-layout-{index}.o"));
        output(
            Command::new(&compiler)
                .args(&flags)
                .arg("-c")
                .arg(source)
                .arg("-o")
                .arg(&object),
        );
        objects.push(object);
    }
    let archive = out.join("libpitex_font_layout.a");
    if archive.exists() {
        fs::remove_file(&archive).unwrap();
    }
    output(
        Command::new(env::var("AR").unwrap_or_else(|_| "ar".into()))
            .arg("crs")
            .arg(&archive)
            .args(&objects),
    );
    link_args(
        "pitex-preview-xetex",
        [archive.to_string_lossy().into_owned()],
    );
    link_args("pitex-preview-xetex", pkg_config("--libs", &packages));
    link_args(
        "pitex-preview-xetex",
        env::var("TECKIT_LIBS")
            .map(|value| value.split_whitespace().map(str::to_owned).collect())
            .unwrap_or_else(|_| pkg_config("--libs", &["teckit"])),
    );
    link_args(
        "pitex-preview-xetex",
        vec![
            "-lm".into(),
            if macos {
                "-lc++".into()
            } else {
                "-lstdc++".into()
            },
        ],
    );
    if macos {
        for framework in [
            "Foundation",
            "CoreFoundation",
            "CoreGraphics",
            "CoreText",
            "AppKit",
        ] {
            link_args(
                "pitex-preview-xetex",
                vec!["-framework".into(), framework.into()],
            );
        }
        link_args(
            "pitex-preview-xetex",
            vec![
                "-Wl,-rpath,@executable_path/../Frameworks".into(),
                "-Wl,-rpath,@executable_path".into(),
            ],
        );
    }
}
