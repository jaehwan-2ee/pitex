//! Port of `EquationPreviewSyntaxTests.swift` — runs the shared
//! `Fixtures/equation-preview/syntax.json` cases so the Rust and Swift
//! scanners agree.

use language_core::equation_preview::{
    MathIncludeKey, MathIncludePaths, MathProjectContext, MathSourceScan, MathSourceScanner,
};
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;

const CARET: &str = "\u{2038}";

fn repository_root() -> PathBuf {
    // crates/language-core -> crates -> Linux -> repo root
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .unwrap()
}

fn fixture() -> Value {
    let path = repository_root().join("Fixtures/equation-preview/syntax.json");
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

/// Source without the marker plus the marker's UTF-8 offset.
fn split(marked: &str) -> (String, usize) {
    let byte = marked.find(CARET).unwrap();
    (marked.replacen(CARET, "", 1), byte)
}

fn strings(value: &Value) -> Vec<String> {
    value
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect()
}

#[test]
fn region_fixtures() {
    let cases = fixture()["regions"].as_array().unwrap().clone();
    assert!(!cases.is_empty());
    for entry in &cases {
        let name = entry["name"].as_str().unwrap();
        let (source, offset) = split(entry["source"].as_str().unwrap());
        let scan = MathSourceScanner::scan(&source);
        let region = if entry["mode"].as_str() == Some("character") {
            scan.region_at_character(offset)
        } else {
            scan.region_at_caret(offset)
        };
        let Some(expected) = entry.get("expected").filter(|e| e.is_object()) else {
            assert!(region.is_none(), "{name}");
            continue;
        };
        let Some(region) = region else { panic!("{name}: no region") };
        assert_eq!(region.source_text(&source), expected["text"].as_str().unwrap(), "{name}");
        assert_eq!(region.is_complete, expected["complete"].as_bool().unwrap(), "{name}");
        if !region.is_complete {
            continue;
        }
        assert_eq!(region.render_source(&source), expected["render"].as_str().unwrap(), "{name}");
        assert_eq!(region.display_mode, expected["display"].as_bool().unwrap(), "{name}");
        assert_eq!(
            region.environment_name.as_deref(),
            expected["environment"].as_str(),
            "{name}"
        );
        assert_eq!(region.is_well_formed, expected["wellFormed"].as_bool().unwrap(), "{name}");
    }
}

#[test]
fn definition_and_include_fixtures() {
    let fixture = fixture();
    for entry in fixture["definitions"].as_array().unwrap() {
        let name = entry["name"].as_str().unwrap();
        let scan = MathSourceScanner::scan(entry["source"].as_str().unwrap());
        let expected = entry["expected"].as_array().unwrap();
        assert_eq!(
            scan.definitions.iter().map(|d| d.kind.raw_value()).collect::<Vec<_>>(),
            expected.iter().map(|e| e["kind"].as_str().unwrap()).collect::<Vec<_>>(),
            "{name}"
        );
        assert_eq!(
            scan.definitions.iter().map(|d| d.name.as_str()).collect::<Vec<_>>(),
            expected.iter().map(|e| e["name"].as_str().unwrap()).collect::<Vec<_>>(),
            "{name}"
        );
        assert_eq!(
            scan.definitions.iter().map(|d| d.math_jax_source.as_str()).collect::<Vec<_>>(),
            expected.iter().map(|e| e["mathJax"].as_str().unwrap()).collect::<Vec<_>>(),
            "{name}"
        );
    }
    for entry in fixture["includes"].as_array().unwrap() {
        let scan = MathSourceScanner::scan(entry["source"].as_str().unwrap());
        assert_eq!(
            scan.includes.iter().map(|i| i.target.as_str()).collect::<Vec<_>>(),
            entry["expected"].as_array().unwrap().iter().map(|v| v.as_str().unwrap()).collect::<Vec<_>>(),
            "{}", entry["name"].as_str().unwrap()
        );
    }
}

#[test]
fn project_context_fixtures() {
    for entry in fixture()["contexts"].as_array().unwrap().clone() {
        let name = entry["name"].as_str().unwrap();
        let files = entry["files"].as_object().unwrap();
        let root = entry["root"].as_str().unwrap();
        let active = entry["active"].as_str().unwrap();
        let mut scans: HashMap<String, MathSourceScan> = HashMap::new();
        let mut caret = 0usize;
        for (path, text) in files {
            let text = text.as_str().unwrap();
            if path == active && text.contains(CARET) {
                let (source, offset) = split(text);
                scans.insert(path.clone(), MathSourceScanner::scan(&source));
                caret = offset;
            } else {
                scans.insert(path.clone(), MathSourceScanner::scan(text));
            }
        }
        // The host's role: resolve each requested include against the files
        // that exist, until nothing is left unresolved.
        let mut resolutions: HashMap<MathIncludeKey, Option<String>> = HashMap::new();
        let mut context =
            MathProjectContext::new(root, Some(active), None, &scans, &resolutions);
        while !context.unresolved.is_empty() {
            for request in &context.unresolved {
                let resolved = request
                    .candidates
                    .iter()
                    .find(|c| files.contains_key(c.as_str()))
                    .cloned();
                resolutions.insert(request.key.clone(), resolved);
            }
            context = MathProjectContext::new(root, Some(active), None, &scans, &resolutions);
        }
        let region = scans[active].region_at_caret(caret).unwrap();
        let math = context.context(active, region.range.utf8_offset as usize);
        assert_eq!(math.definitions, strings(&entry["expected"]), "{name}");
        assert!(!math.truncated, "{name}");
        let body = context.body_definitions(
            active,
            region.range.utf8_offset as usize,
            scans[root].document_begin_offset,
        );
        assert_eq!(body, strings(&entry["body"]), "{name}");
    }
}

/// Macro redefinition must change the context identity the preview cache is
/// keyed by; an unrelated edit must not.
#[test]
fn context_key_tracks_definitions_only() {
    fn key(source: &str) -> String {
        let scan = MathSourceScanner::scan(source);
        let scans = HashMap::from([("/a.tex".to_string(), scan.clone())]);
        let context = MathProjectContext::new("/a.tex", Some("/a.tex"), None, &scans, &HashMap::new());
        context.context("/a.tex", scan.regions[0].range.utf8_offset as usize).key
    }
    let original = key("\\newcommand{\\R}{\\mathbb{R}}\n$x \\in \\R$");
    assert_eq!(original, key("\\newcommand{\\R}{\\mathbb{R}}\nSome words.\n$x \\in \\R$"));
    assert_ne!(original, key("\\newcommand{\\R}{\\mathbf{R}}\n$x \\in \\R$"));
    assert_ne!(original, key("$x \\in \\R$"));
}

#[test]
fn definition_bounds_truncate() {
    let source = "\\newcommand{\\x}{y}\n".repeat(MathProjectContext::MAXIMUM_DEFINITIONS + 10) + "$x$";
    let scan = MathSourceScanner::scan(&source);
    let scans = HashMap::from([("/a.tex".to_string(), scan.clone())]);
    let context = MathProjectContext::new("/a.tex", Some("/a.tex"), None, &scans, &HashMap::new());
    let math = context.context("/a.tex", scan.regions[0].range.utf8_offset as usize);
    assert!(math.truncated);
    assert_eq!(math.definitions.len(), MathProjectContext::MAXIMUM_DEFINITIONS);
}

#[test]
fn include_candidates_prefer_main_directory() {
    assert_eq!(
        MathIncludePaths::candidates("defs", "/p/chapters/one.tex", "/p/main.tex"),
        vec!["/p/defs.tex", "/p/chapters/defs.tex"]
    );
    assert_eq!(
        MathIncludePaths::candidates("../shared/m.sty", "/p/main.tex", "/p/main.tex"),
        vec!["/shared/m.sty"]
    );
}

/// Coarse ceiling only; the benchmark harness records real timings.
#[test]
fn large_fixture_scan_is_linear_and_stable() {
    let path = repository_root().join("Fixtures/projects/large/main.tex");
    let source = std::fs::read_to_string(path).unwrap();
    let first = MathSourceScanner::scan(&source);
    assert_eq!(first, MathSourceScanner::scan(&source));
    assert!(first
        .regions
        .iter()
        .zip(first.regions.iter().skip(1))
        .all(|(a, b)| a.range.end_utf8_offset() <= b.range.utf8_offset));
    assert!(first
        .regions
        .iter()
        .all(|r| r.range.end_utf8_offset() <= source.len() as i64));
}
