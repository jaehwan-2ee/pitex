//! Port of `CompletionContextTests` and `SymbolsTests` from
//! `Packages/TexCore/Tests/TexCoreTests` — keep both suites aligned.

use language_core::*;
use std::collections::BTreeSet;

fn detect(source: &str, caret: Option<usize>) -> Option<CompletionContext> {
    CompletionContextDetector::context(
        source,
        caret.unwrap_or_else(|| source.encode_utf16().count()),
    )
}

fn keys(list: &[&str]) -> BTreeSet<String> {
    list.iter().map(|s| s.to_string()).collect()
}

#[test]
fn command_prefix_at_caret_includes_backslash() {
    let context = detect("\\sub", None).unwrap();
    assert_eq!(context.kind, CompletionContextKind::Command);
    assert_eq!(context.prefix, "\\sub");
    assert_eq!(context.prefix_utf16_offset, 0);
}

#[test]
fn bare_backslash_and_plain_text() {
    let backslash = detect("\\", None).unwrap();
    assert_eq!(backslash.kind, CompletionContextKind::Command);
    assert_eq!(backslash.prefix, "\\");
    assert!(detect("hello ", None).is_none());
    assert!(detect("", None).is_none());
    assert!(detect("3 + 4", None).is_none());
}

#[test]
fn escaped_backslash_is_not_a_command() {
    assert!(detect("\\\\ab", None).is_none());
    assert!(detect("x \\\\", None).is_none());
}

#[test]
fn citation_contexts() {
    let context = detect("\\cite{kim", None).unwrap();
    assert_eq!(context.kind, CompletionContextKind::Citation);
    assert_eq!(context.prefix, "kim");
    assert_eq!(context.prefix_utf16_offset, 6);

    let empty = detect("\\cite{", None).unwrap();
    assert_eq!(empty.kind, CompletionContextKind::Citation);
    assert_eq!(empty.prefix, "");
    assert_eq!(empty.prefix_utf16_offset, 6);
}

#[test]
fn comma_separated_and_whitespace_citation_keys() {
    let second = detect("\\cite{foo,ba", None).unwrap();
    assert_eq!(second.kind, CompletionContextKind::Citation);
    assert_eq!(second.prefix, "ba");
    assert_eq!(second.prefix_utf16_offset, 10);

    let spaced = detect("\\cite{foo, ba", None).unwrap();
    assert_eq!(spaced.kind, CompletionContextKind::Citation);
    assert_eq!(spaced.prefix, "ba");

    let first = detect("\\cite{foo,ba", Some(8)).unwrap();
    assert_eq!(first.kind, CompletionContextKind::Citation);
    assert_eq!(first.prefix, "fo");
}

#[test]
fn optional_argument_and_starred_citation_commands() {
    let optional = detect("\\cite[see]{ki", None).unwrap();
    assert_eq!(optional.kind, CompletionContextKind::Citation);
    assert_eq!(optional.prefix, "ki");

    let starred = detect("\\citep*{x", None).unwrap();
    assert_eq!(starred.kind, CompletionContextKind::Citation);
    assert_eq!(starred.prefix, "x");

    let natbib = detect("\\parencite{al", None).unwrap();
    assert_eq!(natbib.kind, CompletionContextKind::Citation);
    assert_eq!(natbib.prefix, "al");
}

#[test]
fn reference_contexts() {
    let reference = detect("\\ref{sec:in", None).unwrap();
    assert_eq!(reference.kind, CompletionContextKind::Reference);
    assert_eq!(reference.prefix, "sec:in");

    assert_eq!(
        detect("\\pageref{a", None).unwrap().kind,
        CompletionContextKind::Reference
    );
    assert_eq!(
        detect("\\eqref{eq:", None).unwrap().kind,
        CompletionContextKind::Reference
    );
    assert_eq!(
        detect("\\cref{x", None).unwrap().kind,
        CompletionContextKind::Reference
    );
    assert_eq!(
        detect("\\autoref{x", None).unwrap().kind,
        CompletionContextKind::Reference
    );
}

#[test]
fn unrecognised_groups_and_escaped_groups_do_not_complete() {
    assert!(detect("\\section{ti", None).is_none());
    assert!(detect("\\label{x", None).is_none());
    assert!(detect("\\foo{bar", None).is_none());
    assert!(detect("\\cite\\{k", None).is_none());
    assert!(detect("\\\\cite{k", None).is_none());
}

#[test]
fn whitespace_between_command_and_brace() {
    let context = detect("\\cite {ki", None).unwrap();
    assert_eq!(context.kind, CompletionContextKind::Citation);
    assert_eq!(context.prefix, "ki");
}

#[test]
fn non_bmp_text_before_caret_keeps_utf16_offsets() {
    // "한😀" is 3 UTF-16 units; `\cite{k` starts its prefix at 3 + 6.
    let context = detect("한😀\\cite{k", None).unwrap();
    assert_eq!(context.kind, CompletionContextKind::Citation);
    assert_eq!(context.prefix, "k");
    assert_eq!(context.prefix_utf16_offset, 9);
}

#[test]
fn contextual_completions_filter_project_key_sets() {
    let citations = LanguageIndex::completions(
        &CompletionContext {
            kind: CompletionContextKind::Citation,
            prefix: "k".to_string(),
            prefix_utf16_offset: 0,
        },
        &keys(&["sec:intro"]),
        &keys(&["kim2026", "knuth1984", "other"]),
    );
    let texts: Vec<&str> = citations.iter().map(|c| c.text.as_str()).collect();
    assert_eq!(texts, ["kim2026", "knuth1984"]);
    assert!(citations.iter().all(|c| c.kind == CompletionKind::Citation));

    let references = LanguageIndex::completions(
        &CompletionContext {
            kind: CompletionContextKind::Reference,
            prefix: "sec".to_string(),
            prefix_utf16_offset: 0,
        },
        &keys(&["sec:intro", "sec:methods", "fig:x"]),
        &keys(&["sec:cite"]),
    );
    let texts: Vec<&str> = references.iter().map(|c| c.text.as_str()).collect();
    assert_eq!(texts, ["sec:intro", "sec:methods"]);
    assert!(references.iter().all(|c| c.kind == CompletionKind::Label));

    let commands = LanguageIndex::completions(
        &CompletionContext {
            kind: CompletionContextKind::Command,
            prefix: "\\sub".to_string(),
            prefix_utf16_offset: 0,
        },
        &keys(&[]),
        &keys(&[]),
    );
    let texts: Vec<&str> = commands.iter().map(|c| c.text.as_str()).collect();
    assert_eq!(texts, ["\\subsection", "\\subsubsection"]);
    assert!(commands.iter().all(|c| c.kind == CompletionKind::Command));
}

// ---------------------------------------------------------------------------
// SymbolsTests

#[test]
fn every_category_has_symbols_in_catalogue_order() {
    for category in SymbolCategory::ALL {
        let rows: Vec<&TexSymbol> = symbols_in(*category).collect();
        assert!(!rows.is_empty(), "{category:?} must not be empty");
        assert!(rows.iter().all(|s| s.category == *category));
        assert!(rows.iter().all(|s| !s.glyph.is_empty()));
        assert!(rows.iter().all(|s| !s.command.is_empty()));
    }
}

#[test]
fn catalogue_contains_required_groups_and_commands() {
    let commands: BTreeSet<&str> = TEX_SYMBOLS.iter().map(|s| s.command).collect();
    for required in [
        "\\alpha",
        "\\Omega",
        "\\leq",
        "\\in",
        "\\times",
        "\\sum",
        "\\int",
        "\\rightarrow",
        "\\Rightarrow",
        "\\langle",
        "\\infty",
        "\\partial",
        "\\S",
        "\\ulcorner",
        "\\digamma",
        "\\boxplus",
        "\\leqslant",
        "\\dashrightarrow",
    ] {
        assert!(commands.contains(required), "missing {required}");
    }
}

#[test]
fn symbol_category_title_keys_are_stable() {
    assert_eq!(SymbolCategory::GreekLetters.title_key(), "editor.symbol_cat_greek");
    assert_eq!(
        SymbolCategory::AmsArrows.title_key(),
        "editor.symbol_cat_ams_arrows"
    );
    assert_eq!(SymbolCategory::ALL.len(), 14);
}
