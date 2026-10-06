//! Port of `TexAppTests/AccessibilityContractTests.swift`: the same fixture
//! (`Fixtures/expected/accessibility-identifiers.json`) drives identical
//! assertions against the GTK shell sources instead of the SwiftUI sources.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

#[derive(serde::Deserialize)]
struct Contract {
    identifiers: Vec<String>,
    locales: Vec<String>,
    #[serde(rename = "localizationKeys")]
    localization_keys: Vec<String>,
    #[serde(rename = "forbiddenUIStrings")]
    forbidden_ui_strings: Vec<String>,
}

fn repository_root() -> PathBuf {
    // tests/ is at Linux/crates/pitex-shell — four components up is the repo.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .unwrap()
        .to_path_buf()
}

fn load_contract() -> Contract {
    let path = repository_root().join("Fixtures/expected/accessibility-identifiers.json");
    let text = std::fs::read_to_string(&path).expect("contract fixture readable");
    serde_json::from_str(&text).expect("contract fixture parses")
}

/// Concatenated Rust shell sources — the analogue of `appSwiftSource()`.
fn shell_source() -> String {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .expect("src dir readable")
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().map(|e| e == "rs").unwrap_or(false))
        .collect();
    files.sort();
    assert!(!files.is_empty());
    files
        .iter()
        .map(|p| std::fs::read_to_string(p).expect("source file readable"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn localization_keys(locale: &str) -> BTreeSet<String> {
    let path = repository_root().join(format!("Mac/Resources/{locale}.lproj/Localizable.strings"));
    let text = std::fs::read_to_string(&path).expect("strings file readable");
    let mut keys = BTreeSet::new();
    for line in text.lines() {
        let line = line.trim();
        if !line.starts_with('"') {
            continue;
        }
        if let Some(eq) = line.find("\" =") {
            keys.insert(line[1..eq].to_string());
        }
    }
    keys
}

#[test]
fn native_sources_contain_every_stable_identifier_exactly_once() {
    let contract = load_contract();
    assert_eq!(
        contract.identifiers.iter().collect::<BTreeSet<_>>().len(),
        contract.identifiers.len(),
        "fixture contains duplicate identifiers"
    );
    assert!(contract.identifiers.iter().all(|i| i.starts_with("pitex.")));

    let source = shell_source();
    for identifier in &contract.identifiers {
        let needle = format!("\"{identifier}\"");
        assert_eq!(
            source.matches(&needle).count(),
            1,
            "expected one source declaration of {identifier}"
        );
    }

    // Every declared `a11y` identifier must be unique.
    let mut declared = Vec::new();
    for caps in source.split("a11y(&").skip(1) {
        if let Some(rest) = caps.split_once(", \"") {
            if let Some((ident, _)) = rest.1.split_once('"') {
                if ident.starts_with("pitex.") {
                    declared.push(ident.to_string());
                }
            }
        }
    }
    assert_eq!(
        declared.len(),
        declared.iter().collect::<BTreeSet<_>>().len(),
        "native source declares a duplicate accessibility identifier"
    );
    let declared_set: BTreeSet<String> = declared.into_iter().collect();
    for identifier in &contract.identifiers {
        assert!(
            declared_set.contains(identifier),
            "contract identifier {identifier} not declared via a11y"
        );
    }
}

#[test]
fn every_required_localization_key_exists_in_every_locale() {
    let contract = load_contract();
    assert_eq!(
        contract.locales.iter().collect::<BTreeSet<_>>().len(),
        contract.locales.len()
    );

    let mut baseline: Option<BTreeSet<String>> = None;
    for locale in &contract.locales {
        let keys = localization_keys(locale);
        for required in &contract.localization_keys {
            assert!(
                keys.contains(required),
                "missing required key {required} in {locale}"
            );
        }
        if let Some(baseline) = &baseline {
            assert_eq!(
                &keys, baseline,
                "locale {locale} does not have identical key coverage"
            );
        } else {
            baseline = Some(keys);
        }
    }

    // Every key used through `tr(lang, "...")` / `a11y(..., "...")` resolves.
    // Mirrors the Swift regex: only strings in the localization-key namespaces
    // count — accessibility identifiers and file names do not.
    const KEY_PREFIXES: &[&str] = &[
        "app", "workspace", "editor", "build", "preview", "assistant", "command",
        "settings", "state", "error", "conflict", "recovery", "warning",
        "accessibility",
        "git", "ssh", "remote",
    ];
    let baseline = baseline.unwrap_or_default();
    let source = shell_source();
    // Only literals passed to `tr`/`tr1`/`a11y` are UI strings — the same
    // strings the Swift test's regex intended to capture. (A plain literal
    // scan would also match file names like the pi agent's `settings.json`.)
    let mut used = BTreeSet::new();
    let mut rest = source.as_str();
    loop {
        let next = ["tr(", "tr1(", "a11y("]
            .iter()
            .filter_map(|n| rest.find(n).map(|i| i + n.len()))
            .min();
        let Some(offset) = next else { break };
        let after = &rest[offset..];
        let end = after.find(')').unwrap_or(after.len());
        for segment in after[..end].split('"').skip(1).step_by(2) {
            let has_prefix = KEY_PREFIXES
                .iter()
                .any(|p| segment.starts_with(&format!("{p}.")));
            if has_prefix
                && segment
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_')
            {
                used.insert(segment.to_string());
            }
        }
        rest = &rest[offset..];
    }
    let unlocalized: Vec<_> = used.difference(&baseline).collect();
    assert!(
        unlocalized.is_empty(),
        "shell source uses unlocalized keys: {unlocalized:?}"
    );
}

#[test]
fn native_ui_contains_no_api_key_or_updater_surface() {
    let contract = load_contract();
    let mut visible_text = shell_source();
    for locale in &contract.locales {
        let path =
            repository_root().join(format!("Mac/Resources/{locale}.lproj/Localizable.strings"));
        visible_text += &std::fs::read_to_string(&path).expect("strings file readable");
    }
    let folded = visible_text.to_lowercase();
    for forbidden in &contract.forbidden_ui_strings {
        assert!(
            !folded.contains(&forbidden.to_lowercase()),
            "forbidden UI string: {forbidden}"
        );
    }
}
