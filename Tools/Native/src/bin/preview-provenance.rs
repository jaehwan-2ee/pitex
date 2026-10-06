//! Track imported and authored embedded-engine source files and license notices.
use pitex_native_tools::repo_root;
use regex::Regex;
use serde::Serialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use std::process::Command;

type Result<T> = std::result::Result<T, Box<dyn Error>>;
const UPSTREAM_URL: &str = "https://github.com/let-def/texpresso";
const UPSTREAM_SHA: &str = "e8df7709077b2f86f6e16e6c86ceefb86de06f8d";
// Shared original Rust imported by the Windows helper and its native tests.
const SHARED_SOURCE_INPUTS: &[&str] = &[
    "Linux/crates/synctex-core/src/anchors.rs",
    "Linux/crates/synctex-core/src/cli_workspace.rs",
];

fn sha256(path: &Path) -> Result<String> {
    let mut file = File::open(path)?;
    let mut hash = Sha256::new();
    let mut chunk = [0; 1 << 16];
    loop {
        let length = file.read(&mut chunk)?;
        if length == 0 {
            break;
        }
        hash.update(&chunk[..length]);
    }
    Ok(format!("{:x}", hash.finalize()))
}

fn detect_license(rel: &str, path: &Path) -> &'static str {
    if rel.ends_with(".md") {
        return "n/a (documentation)";
    }
    let mut head = Vec::new();
    let Ok(file) = File::open(path) else {
        return "unknown";
    };
    if file.take(4096).read_to_end(&mut head).is_err() {
        return "unknown";
    }
    let head = String::from_utf8_lossy(&head);
    if head.contains("GNU General Public License") && !head.contains("Lesser") {
        "GPL"
    } else if head.contains("Tectonic Project") && head.contains("MIT") {
        "MIT (Tectonic Project)"
    } else if head.contains("Permission is hereby granted") && head.contains("SIL") {
        "MIT/X11 (SIL International)"
    } else if head.contains("MIT License") || head.contains("Permission is hereby granted") {
        "MIT"
    } else if head.to_lowercase().contains("public domain") {
        "public domain"
    } else if head.contains("Pitex-authored") || head.contains("SPDX-License-Identifier: AGPL-3.0-or-later") {
        "AGPL-3.0-or-later (Pitex)"
    } else {
        "none-in-file"
    }
}

fn tracked_files(root: &Path) -> Result<Vec<String>> {
    fn visit(root: &Path, base: &Path, files: &mut Vec<String>) -> Result<()> {
        for entry in fs::read_dir(base)? {
            let entry = entry?;
            let path = entry.path();
            let kind = entry.file_type()?;
            if kind.is_dir() {
                // Standalone Cargo helpers keep their local build output
                // here; provenance records source inputs, not target files.
                if !matches!(
                    entry.file_name().to_str(),
                    Some(".git" | "__pycache__" | "target")
                ) {
                    visit(root, &path, files)?;
                }
            } else if !(kind.is_symlink() && path.is_dir()) {
                let rel = path
                    .strip_prefix(root)?
                    .to_string_lossy()
                    .replace('\\', "/");
                if rel != "provenance.json" {
                    files.push(rel);
                }
            }
        }
        Ok(())
    }
    let mut files = Vec::new();
    visit(root, root, &mut files)?;
    files.sort();
    Ok(files)
}

fn migrated_sources(migration: &Value) -> Result<BTreeMap<String, String>> {
    let mut files = BTreeMap::new();
    for collection in ["files", "manual_migrations"] {
        if let Some(entries) = migration[collection].as_array() {
            for entry in entries {
                files.insert(
                    entry["path"]
                        .as_str()
                        .ok_or("migration file has no path")?
                        .into(),
                    entry["source"]
                        .as_str()
                        .ok_or("migration file has no source")?
                        .into(),
                );
            }
        }
    }
    Ok(files)
}

fn translation_metadata(migration: &Value, path: &str, source: &str) -> Result<Value> {
    if let Some(manual) = migration["manual_migrations"]
        .as_array()
        .and_then(|entries| entries.iter().find(|entry| entry["path"] == path))
    {
        for field in ["source_sha256", "source_commit", "translator", "targets"] {
            if manual[field].is_null() {
                return Err(format!("manual migration {path} has no {field}").into());
            }
        }
        let mut metadata=json!({"path": source, "sha256": manual["source_sha256"],
            "commit": manual["source_commit"], "translator": manual["translator"],
            "targets": manual["targets"]});
        for field in ["source_state", "native_boundary", "license_notice"] {
            if !manual[field].is_null() {metadata[field]=manual[field].clone();}
        }
        return Ok(metadata);
    }
    Ok(
        json!({"path": source, "sha256": migration["source_sha256"][source],
        "commit": migration["source_commit"], "translator": migration["translator"]}),
    )
}

#[cfg(test)]
mod migration_tests {
    use super::*;
    #[test]
    fn manual_migration_keeps_the_historical_translator() {
        let record = json!({"source_commit":"historical", "translator":"C2Rust 0.22.1",
            "source_sha256":{"old.c":"old-hash"},"files":[{"path":"old.rs","source":"old.c"}],
            "manual_migrations":[{"path":"math.rs","source":"math.cpp","source_sha256":"math-hash",
                "source_commit":"later","translator":"Manual C++ to Rust","targets":["aarch64-apple-darwin","x86_64-unknown-linux-gnu"]}]});
        let mappings = migrated_sources(&record).unwrap();
        assert_eq!(mappings["math.rs"], "math.cpp");
        let old = translation_metadata(&record, "old.rs", &mappings["old.rs"]).unwrap();
        let math = translation_metadata(&record, "math.rs", &mappings["math.rs"]).unwrap();
        assert_eq!(old["translator"], "C2Rust 0.22.1");
        assert_eq!(old["commit"], "historical");
        assert_eq!(math["translator"], "Manual C++ to Rust");
        assert_eq!(math["commit"], "later");
        assert_eq!(math["sha256"], "math-hash");
        assert_eq!(math["targets"].as_array().unwrap().len(), 2);
    }
}

fn upstream_path<'a>(
    rel: &'a str,
    migrated: &'a BTreeMap<String, String>,
    data: &Value,
) -> Option<String> {
    // Original boundary headers live beside imported XeTeX sources without
    // inheriting those directory mappings.
    if data["AUTHORED_PATHS"]
        .as_array()
        .is_some_and(|paths| paths.iter().any(|path| path.as_str() == Some(rel)))
    {
        return None;
    }
    let rel = migrated.get(rel).map(String::as_str).unwrap_or(rel);
    if let Some(path) = data["DRIVER_MAP"][rel].as_str() {
        return Some(path.into());
    }
    for mapping in data["IMPORT_MAP"].as_array()? {
        let local = mapping[0].as_str()?;
        let up = mapping[1].as_str()?;
        if rel == local || (local.ends_with('/') && rel.starts_with(local)) {
            return Some(if local.ends_with('/') {
                format!("{up}{}", &rel[local.len()..])
            } else {
                up.into()
            });
        }
    }
    None
}

fn relative_path(path: &Path) -> Result<String> {
    let cwd = std::env::current_dir()?;
    let source: Vec<_> = cwd.components().collect();
    let target: Vec<_> = path.components().collect();
    let common = source
        .iter()
        .zip(&target)
        .take_while(|(a, b)| a == b)
        .count();
    let mut result = PathBuf::new();
    for part in &source[common..] {
        if matches!(part, Component::Normal(_)) {
            result.push("..");
        }
    }
    for part in &target[common..] {
        result.push(part.as_os_str());
    }
    Ok(result.to_string_lossy().into())
}

fn sort_keys(value: &mut Value) {
    match value {
        Value::Object(map) => {
            for nested in map.values_mut() {
                sort_keys(nested);
            }
            map.sort_keys();
        }
        Value::Array(values) => values.iter_mut().for_each(sort_keys),
        _ => {}
    }
}

fn generate(root: &Path, upstream: &Path, data: &Value) -> Result<()> {
    let git = Command::new("git")
        .arg("-C")
        .arg(upstream)
        .args(["rev-parse", "HEAD"])
        .output()?;
    if !git.status.success() {
        return Err(String::from_utf8_lossy(&git.stderr)
            .trim()
            .to_string()
            .into());
    }
    let head = String::from_utf8(git.stdout)?.trim().to_string();
    if head != UPSTREAM_SHA {
        return Err(format!("upstream checkout is at {head}, expected {UPSTREAM_SHA}").into());
    }
    let migration_path = root.join("migration.json");
    let migration: Value = if migration_path.is_file() {
        serde_json::from_reader(File::open(migration_path)?)?
    } else {
        json!({})
    };
    let migrated = migrated_sources(&migration)?;
    let mut entries = Vec::new();
    for rel in tracked_files(root)? {
        let path = root.join(&rel);
        let source = migrated.get(&rel);
        let mut entry = json!({"path": rel, "sha256": sha256(&path)?, "license_in_file": detect_license(&rel, &path)});
        if let Some(source) = source {
            entry["translated_from"] = translation_metadata(&migration, &rel, source)?;
        }
        if let Some(up) = upstream_path(&rel, &migrated, data) {
            let up_file = upstream.join(&up);
            if !up_file.is_file() {
                return Err(format!("{rel}: upstream file {up} missing").into());
            }
            entry["origin"] = "texpresso".into();
            entry["upstream_path"] = up.clone().into();
            entry["upstream_sha256"] = sha256(&up_file)?.into();
            entry["upstream_license"] = detect_license(&up, &up_file).into();
            entry["modified"] = (entry["sha256"] != entry["upstream_sha256"]).into();
        } else {
            entry["origin"] = "pitex".into();
            if let Some(note) = data["DERIVED_NOTES"][source.unwrap_or(&rel)].as_str() {
                entry["adapts"] = note.into();
            }
        }
        entries.push(entry);
    }
    let length = entries.len();
    let repository = root
        .parent()
        .ok_or("PreviewEngine has no repository parent")?;
    let shared = SHARED_SOURCE_INPUTS
        .iter()
        .map(|rel| {
            let path = repository.join(rel);
            Ok(json!({"path": rel, "sha256": sha256(&path)?,
            "origin": "pitex", "license_in_file": detect_license(rel, &path)}))
        })
        .collect::<Result<Vec<_>>>()?;
    let mut manifest = json!({
        "upstream": {"url": UPSTREAM_URL, "commit": UPSTREAM_SHA,
            "root_license": "MIT (Frédéric Bour); per-file notices retained"},
        "excluded_upstream_components": data["EXCLUDED"],
        "runtime_dependencies": data["RUNTIME_DEPS"],
        "rust_dependency_lockfile": "Cargo.lock",
        "rust_build_dependencies": data["RUST_BUILD_DEPS"],
        "files": entries,
        "shared_source_inputs": shared,
    });
    sort_keys(&mut manifest);
    let manifest_path = root.join("provenance.json");
    let mut file = File::create(&manifest_path)?;
    let formatter = serde_json::ser::PrettyFormatter::with_indent(b" ");
    manifest.serialize(&mut serde_json::Serializer::with_formatter(
        &mut file, formatter,
    ))?;
    file.write_all(b"\n")?;
    println!("wrote {} ({length} files)", relative_path(&manifest_path)?);
    Ok(())
}

fn verify(root: &Path) -> Result<bool> {
    let manifest: Value = serde_json::from_reader(File::open(root.join("provenance.json"))?)?;
    let entries = manifest["files"]
        .as_array()
        .ok_or("manifest files is not an array")?;
    let mut listed = BTreeMap::new();
    for entry in entries {
        listed.insert(
            entry["path"].as_str().ok_or("manifest entry has no path")?,
            entry,
        );
    }
    let current = tracked_files(root)?;
    let current_set: BTreeSet<_> = current.iter().map(String::as_str).collect();
    let patterns = [
        (Regex::new(r"^xetex/.*dpx-")?, "GPL dpx source"),
        (
            Regex::new(r"(^|/)teckit-[^/]*\.(c|cpp|h)$")?,
            "vendored TECkit source",
        ),
        (
            Regex::new(r"(^|/)(incdvi|renderer)\.[ch]$")?,
            "MuPDF renderer source",
        ),
        (Regex::new(r"(?i)mupdf")?, "MuPDF reference in file name"),
    ];
    let mut problems = Vec::new();
    for rel in &current {
        for (pattern, what) in &patterns {
            if pattern.is_match(rel) {
                problems.push(format!("{rel}: forbidden component ({what})"));
            }
        }
        let Some(entry) = listed.get(rel.as_str()) else {
            problems.push(format!("{rel}: not in provenance.json (regenerate)"));
            continue;
        };
        let path = root.join(rel);
        if sha256(&path)? != entry["sha256"] {
            problems.push(format!(
                "{rel}: content differs from provenance.json (regenerate)"
            ));
        }
        let license = detect_license(rel, &path);
        if license == "GPL" {
            problems.push(format!("{rel}: GPL notice found"));
        }
        if entry["origin"] == "texpresso" {
            if let Some(upstream_license) = entry["upstream_license"].as_str() {
                if upstream_license != "none-in-file" && license != upstream_license {
                    problems.push(format!("{rel}: upstream notice '{upstream_license}' not retained (found '{license}')"));
                }
            }
        }
    }
    for rel in listed.keys() {
        if !current_set.contains(rel) {
            problems.push(format!("{rel}: listed but missing"));
        }
    }
    let shared = manifest["shared_source_inputs"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    let repository = root
        .parent()
        .ok_or("PreviewEngine has no repository parent")?;
    for rel in SHARED_SOURCE_INPUTS {
        match shared
            .iter()
            .find(|entry| entry["path"].as_str() == Some(rel))
        {
            None => problems.push(format!(
                "{rel}: shared source not in provenance.json (regenerate)"
            )),
            Some(entry) => {
                let path = repository.join(rel);
                if !path.is_file() || entry["sha256"] != sha256(&path)? {
                    problems.push(format!(
                        "{rel}: shared source changed or missing (regenerate)"
                    ));
                }
                if entry["origin"] != "pitex"
                    || (path.is_file() && detect_license(rel, &path) == "GPL")
                {
                    problems.push(format!("{rel}: invalid shared source origin or notice"));
                }
            }
        }
    }
    if shared.len() != SHARED_SOURCE_INPUTS.len() {
        problems.push("Shared source input inventory differs from the helper imports".into());
    }
    if !problems.is_empty() {
        println!("{}", problems.join("\n"));
        return Ok(false);
    }
    let imported = entries
        .iter()
        .filter(|entry| entry["origin"] == "texpresso")
        .count();
    let modified = entries
        .iter()
        .filter(|entry| entry["modified"].as_bool() == Some(true))
        .count();
    let commit = manifest["upstream"]["commit"]
        .as_str()
        .ok_or("manifest upstream has no commit")?;
    println!("provenance OK: {} files, {imported} from TeXpresso {} ({modified} modified), {} Pitex-authored",
             current.len(), commit.chars().take(12).collect::<String>(), current.len() - imported);
    Ok(true)
}

fn run() -> Result<bool> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() >= 2 && args[1] == "verify" {
        return verify(&repo_root()?.join("PreviewEngine"));
    }
    if args.len() == 4 && args[1] == "generate" && args[2] == "--upstream" {
        let data = serde_json::from_str(DATA)?;
        generate(
            &repo_root()?.join("PreviewEngine"),
            Path::new(&args[3]),
            &data,
        )?;
        return Ok(true);
    }
    Err(USAGE.into())
}

fn main() {
    match run() {
        Ok(true) => {}
        Ok(false) => std::process::exit(1),
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
}

const DATA: &str = r###"{
 "AUTHORED_PATHS": ["xetex/layout/pitex_font_freetype.h"],
 "DERIVED_NOTES": {
  "driver/main.c": "update/close handling adapted from src/frontend/main.c interpret_open/interpret_close (MIT); the rest is Pitex-authored"
 },
 "DRIVER_MAP": {
  "driver/engine_tex.c": "src/frontend/engine_tex.c",
  "driver/engine_tex.h": "src/frontend/engine.h",
  "driver/fs.c": "src/frontend/fs.c",
  "driver/myabort.c": "src/frontend/myabort.c",
  "driver/myabort.h": "src/frontend/myabort.h",
  "driver/sprotocol.c": "src/frontend/sprotocol.c",
  "driver/sprotocol.h": "src/frontend/sprotocol.h",
  "driver/state.c": "src/frontend/state.c",
  "driver/state.h": "src/frontend/state.h"
 },
 "EXCLUDED": [
  {
   "license": "GPL-2.0-or-later (xdvipdfmx)",
   "reason": "Not imported. The engine only used it to size PDF/PNG/JPEG/BMP pictures; replaced by Pitex-authored shared/pdfread.c and shared/imginfo.c written from the format specifications. PDF output is produced by the Pitex-authored driver/xdv2pdf.c instead of xdvipdfmx.",
   "upstream": "src/engine/dpx/"
  },
  {
   "license": "LGPL-2.1-or-later OR CPL-0.5-or-later (SIL TECkit)",
   "reason": "Not vendored. The engine dynamically links the distribution's unmodified, replaceable libTECkit shared library (Ubuntu libteckit0, Homebrew teckit).",
   "upstream": "src/engine/engine/teckit-*"
  },
  {
   "license": "MIT source, but links MuPDF (AGPL-3.0/commercial) and SDL",
   "reason": "The MuPDF display-list renderer and SDL window are replaced by the Pitex XDV->PDF writer and the native Pitex PDF views.",
   "upstream": "src/frontend/ (renderer, incdvi, main, editor, SDL/MuPDF UI)"
  },
  {
   "license": "MIT source, built on MuPDF fz_* APIs",
   "reason": "MuPDF-coupled XDV renderer; replaced by driver/xdv.c, driver/xdv2pdf.c, driver/fonts.c, driver/images.c, driver/pdfw.c.",
   "upstream": "src/dvi/"
  },
  {
   "license": "MIT",
   "reason": "External `tectonic` CLI file provider (process spawn / automatic bundle download) was removed; texlive_provider.c is the only provider. Covered by TeXpresso's root MIT license (Frédéric Bour).",
   "upstream": "src/common/tectonic_provider.c"
  },
  {
   "license": "MIT",
   "reason": "Build glue and editor integrations for the upstream application.",
   "upstream": "mupdf-config.sh, Makefile, emacs/, doc/, test/"
  }
 ],
 "IMPORT_MAP": [
  [
   "xetex/engine/",
   "src/engine/engine/"
  ],
  [
   "xetex/layout/",
   "src/engine/layout/"
  ],
  [
   "xetex/main/",
   "src/engine/main/"
  ],
  [
   "xetex/include/",
   "src/engine/include/"
  ],
  [
   "xetex/common/include/",
   "src/include/"
  ],
  [
   "xetex/common/",
   "src/common/"
  ],
  [
   "licenses/TeXpresso-LICENSE.txt",
   "LICENSE"
  ]
 ],
 "RUNTIME_DEPS": [
  {
   "library": "Rust standard library",
   "license": "MIT OR Apache-2.0",
   "license_notes": "Pinned nightly-2026-10-04; full standard-library copyright notices bundled",
   "linkage": "static",
   "used_by": "both helpers"
  },
  {
   "library": "libc Rust crate 0.2.190",
   "license": "MIT OR Apache-2.0",
   "linkage": "static",
   "used_by": "both helpers"
  },
  {
   "library": "c2rust-bitfields 0.22.1",
   "license": "BSD-3-Clause",
   "linkage": "static",
   "used_by": "pitex-preview-xetex"
  },
  {
   "library": "regex 1.13.1",
   "license": "MIT",
   "license_notes": "MIT option selected; full original copyright and grant in Rust-regex-1.13.1-LICENSE-MIT.txt",
   "linkage": "static",
   "source_availability": "Registry package and checksum pinned in PreviewEngine/Cargo.lock",
   "used_by": "pitex-preview"
  },
  {
   "library": "regex-automata 0.4.18",
   "license": "MIT",
   "license_notes": "MIT option selected; full original copyright and grant in Rust-regex-automata-0.4.18-LICENSE-MIT.txt",
   "linkage": "static",
   "source_availability": "Registry package and checksum pinned in PreviewEngine/Cargo.lock",
   "used_by": "pitex-preview"
  },
  {
   "library": "regex-syntax 0.8.11",
   "license": "MIT",
   "license_notes": "MIT option selected; full original copyright and grant in Rust-regex-syntax-0.8.11-LICENSE-MIT.txt",
   "linkage": "static",
   "source_availability": "Registry package and checksum pinned in PreviewEngine/Cargo.lock",
   "used_by": "pitex-preview"
  },
  {
   "library": "aho-corasick 1.1.5",
   "license": "MIT",
   "license_notes": "MIT option selected; full original copyright and grant in Rust-aho-corasick-1.1.5-LICENSE-MIT.txt",
   "linkage": "static",
   "source_availability": "Registry package and checksum pinned in PreviewEngine/Cargo.lock",
   "used_by": "pitex-preview"
  },
  {
   "library": "memchr 2.8.3",
   "license": "MIT",
   "license_notes": "MIT option selected; full original copyright and grant in Rust-memchr-2.8.3-LICENSE-MIT.txt",
   "linkage": "static",
   "source_availability": "Registry package and checksum pinned in PreviewEngine/Cargo.lock",
   "used_by": "pitex-preview"
  },
  {
   "library": "jpeg-decoder 0.3.2",
   "license": "MIT",
   "license_notes": "MIT option selected; full original copyright and grant in Rust-jpeg-decoder-0.3.2-LICENSE-MIT.txt; optional Rayon dependency disabled",
   "linkage": "static",
   "source_availability": "Registry package and checksum pinned in PreviewEngine/Cargo.lock",
   "used_by": "pitex-preview"
  },
  {
   "library": "libTECkit",
   "license": "LGPL-2.1-or-later OR LicenseRef-TECkit-CPL-0.5",
   "license_notes": "SIL dual choice: LGPL-2.1+ or CPL-0.5+; LGPL-2.1 selected",
   "linkage": "shared, unmodified, replaceable",
   "source_availability": "https://github.com/silnrsi/teckit/releases/download/v2.5.13/teckit-2.5.13.tar.xz (sha256 3f55cd3670f1ff1a439d5a40071870b9e4ca2be8877a0eb80e24783cb532b380; configure --with-system-zlib); Debian/Ubuntu: apt source libteckit0",
   "used_by": "pitex-preview-xetex"
  },
  {
   "library": "HarfBuzz",
   "license": "MIT",
   "license_notes": "upstream 'Old MIT' text; SPDX MIT",
   "linkage": "shared",
   "source_availability": "https://github.com/harfbuzz/harfbuzz/releases/download/14.5.0/harfbuzz-14.5.0.tar.xz (sha256 b7132e148358a45185c9feafd049dbaf243649d3c44414b3534d9c95d18592b9); Debian/Ubuntu: apt source libharfbuzz0b",
   "used_by": "pitex-preview-xetex"
  },
  {
   "library": "graphite2",
   "license": "LGPL-2.1-or-later OR MPL-2.0 OR GPL-2.0-or-later",
   "license_notes": "shipped under LGPL-2.1-or-later; dynamically linked",
   "linkage": "shared",
   "source_availability": "Debian/Ubuntu: apt source libgraphite2-3; macOS bundle: formula + pinned source URL/SHA256 in BUNDLED-VERSIONS.txt",
   "used_by": "pitex-preview-xetex"
  },
  {
   "library": "FreeType",
   "license": "FTL OR GPL-2.0-or-later",
   "license_notes": "FTL selected; copyright © The FreeType Project (www.freetype.org). All rights reserved.",
   "linkage": "shared",
   "source_availability": "Debian/Ubuntu: apt source libfreetype6; macOS bundle: formula + pinned source URL/SHA256 in BUNDLED-VERSIONS.txt",
   "used_by": "both helpers"
  },
  {
   "library": "ICU (uc, i18n, data)",
   "license": "Unicode-3.0",
   "license_notes": "Unicode License v3; Debian records also mark some ICU data files Expat; bibliography regex and locale collation use the existing i18n component",
   "linkage": "shared",
   "used_by": "both helpers"
  },
  {
   "library": "fontconfig",
   "license": "HPND-sell-variant",
   "license_notes": "MIT-style license including permission to sell",
   "linkage": "shared (Linux)",
   "used_by": "pitex-preview-xetex"
  },
  {
   "library": "libpng",
   "license": "Libpng",
   "license_notes": "PNG Reference Library License v2",
   "linkage": "shared",
   "used_by": "pitex-preview-xetex"
  },
  {
   "library": "zlib",
   "license": "Zlib",
   "linkage": "shared",
   "used_by": "both helpers"
  },
  {
   "library": "TeX distribution (TeX Live / MacTeX)",
   "license_notes": "aggregate data read at run time; per-file licenses (LPPL, GPL variants, OFL, ...) ship inside the distribution itself — not a linked component, no single expression applies",
   "linkage": "not linked: files read at run time via kpsewhich/ls-R, format generated locally",
   "used_by": "both helpers"
  }
 ],
 "RUST_BUILD_DEPS": [
  {
   "crate": "c2rust-bitfields-derive",
   "license": "BSD-3-Clause",
   "version": "0.22.1"
  },
  {
   "crate": "proc-macro2",
   "license": "MIT OR Apache-2.0",
   "version": "1.0.103"
  },
  {
   "crate": "quote",
   "license": "MIT OR Apache-2.0",
   "version": "1.0.40"
  },
  {
   "crate": "syn",
   "license": "MIT OR Apache-2.0",
   "version": "2.0.106"
  },
  {
   "crate": "unicode-ident",
   "license": "(MIT OR Apache-2.0) AND Unicode-3.0",
   "version": "1.0.22"
  }
 ]
}"###;

const USAGE: &str = r###"Provenance manifest for PreviewEngine/.

  preview-provenance generate --upstream <texpresso checkout at UPSTREAM_SHA>
  preview-provenance verify

`generate` classifies every file under PreviewEngine/ as imported from
TeXpresso (upstream path, upstream and current SHA-256, modified flag,
license notice found in the file) or Pitex-authored, records the upstream
components that were deliberately NOT imported, and writes provenance.json.
`verify` (no network, no upstream checkout) fails when a file changed or
appeared without the manifest being regenerated, when an imported file lost
its upstream license notice, or when an excluded component (GPL dpx code,
vendored TECkit, MuPDF/SDL frontend) is present again.

This documents the technical dependency boundary; it is not legal advice.
"###;
