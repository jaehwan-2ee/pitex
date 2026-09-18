//! Port of `LanguageIndexTests`, `UnicodeCoordinateTests`, and
//! `LargeFixtureTests` from `Packages/TexCore/Tests/TexCoreTests`.

use language_core::*;

// ---------------------------------------------------------------------------
// LanguageIndexTests

#[test]
fn cross_file_definitions_uses_and_deterministic_completions() {
    let main = LanguageFileSnapshot::new(
        "main.tex",
        1,
        "\\section{소개}\\label{sec:intro}\\input{chapter}",
        TeXDialect::Latex,
    )
    .unwrap();
    let chapter = LanguageFileSnapshot::new(
        "chapter.tex",
        1,
        "\\subsection{Details}\\ref{sec:intro}\\cite{kim2026}",
        TeXDialect::Latex,
    )
    .unwrap();
    let bib = LanguageFileSnapshot::new(
        "refs.bib",
        1,
        "@article{kim2026, title={제목}}",
        TeXDialect::Bibtex,
    )
    .unwrap();
    let index = ProjectLanguageIndex::new(vec![chapter, bib, main]).unwrap();

    assert!(index.diagnostics().is_empty());
    let keys: Vec<String> = index
        .references()
        .iter()
        .map(|(_, r)| r.key.clone())
        .collect();
    assert_eq!(keys, ["sec:intro", "kim2026", "sec:intro"]);
    let bib_keys: Vec<String> = index
        .bibliography()
        .iter()
        .map(|(_, e)| e.key.clone())
        .collect();
    assert_eq!(bib_keys, ["kim2026"]);
    let completions: Vec<String> = index
        .completions("sec")
        .iter()
        .map(|c| c.text.clone())
        .collect();
    assert_eq!(completions, ["sec:intro"]);
    let command_completions: Vec<String> = LanguageIndex::command_completions("\\sub")
        .iter()
        .map(|c| c.text.clone())
        .collect();
    assert_eq!(command_completions, ["\\subsection", "\\subsubsection"]);
    assert_eq!(index.completions(""), index.completions(""));
}

#[test]
fn recursive_outline_follows_includes_at_their_source_position() {
    let main = LanguageFileSnapshot::new(
        "main.tex",
        1,
        "\\section{Before}\\input{chapters/a}\\section{After}",
        TeXDialect::Latex,
    )
    .unwrap();
    let a = LanguageFileSnapshot::new(
        "chapters/a.tex",
        1,
        "\\subsection{A}\\include{b}",
        TeXDialect::Latex,
    )
    .unwrap();
    let b = LanguageFileSnapshot::new(
        "chapters/b.tex",
        1,
        "\\subsubsection{B}",
        TeXDialect::Latex,
    )
    .unwrap();
    let index = ProjectLanguageIndex::new(vec![b, main, a]).unwrap();

    let outline = index.recursive_outline("main.tex");
    let sources: Vec<&str> = outline.iter().map(|o| o.source_id.as_str()).collect();
    assert_eq!(
        sources,
        ["main.tex", "chapters/a.tex", "chapters/b.tex", "main.tex"]
    );
    let titles: Vec<&str> = outline.iter().map(|o| o.record.title.as_str()).collect();
    assert_eq!(titles, ["Before", "A", "B", "After"]);
    let depths: Vec<i64> = outline.iter().map(|o| o.record.depth).collect();
    assert_eq!(depths, [0, 1, 2, 0]);
}

#[test]
fn bibtex_index_handles_nested_groups_and_ignores_special_entries() {
    let snapshot = LanguageFileSnapshot::new(
        "library.bib",
        4,
        "% @article{ignored,}\n@string{name = \"Journal\"}\n@article{alpha, title={A {Nested} Title}}\n@book(beta, title=\"B\")",
        TeXDialect::Bibtex,
    )
    .unwrap();

    let types: Vec<&str> = snapshot
        .bibliography
        .iter()
        .map(|e| e.entry_type.as_str())
        .collect();
    assert_eq!(types, ["article", "book"]);
    let keys: Vec<&str> = snapshot.bibliography.iter().map(|e| e.key.as_str()).collect();
    assert_eq!(keys, ["alpha", "beta"]);
    let kinds: Vec<MinimapRangeKind> = snapshot
        .minimap_ranges
        .iter()
        .map(|r| r.kind)
        .collect();
    assert_eq!(
        kinds,
        [MinimapRangeKind::Bibliography, MinimapRangeKind::Bibliography]
    );
    assert!(snapshot.diagnostics.is_empty());
}

#[test]
fn comments_verbatim_and_escaped_commands_are_conservative() {
    let source = "% \\section{commented}\n\\begin{verbatim}\n\\section{verbatim}\n\\label{fake}\n\\end{verbatim}\n\\verb|\\ref{also-fake}|\n\\% \\section{real}";
    let snapshot =
        LanguageFileSnapshot::new("main.tex", 1, source, TeXDialect::Latex).unwrap();

    let titles: Vec<&str> = snapshot.outline.iter().map(|o| o.title.as_str()).collect();
    assert_eq!(titles, ["real"]);
    assert!(snapshot.references.is_empty());
}

#[test]
fn missing_duplicates_cycles_and_malformed_groups_are_explicit_and_stable() {
    let a = LanguageFileSnapshot::new(
        "a.tex",
        1,
        "\\label{same}\\ref{missing}\\cite{absent}\\input{b}\\section{oops",
        TeXDialect::Latex,
    )
    .unwrap();
    let b = LanguageFileSnapshot::new(
        "b.tex",
        1,
        "\\label{same}\\input{a}",
        TeXDialect::Latex,
    )
    .unwrap();
    let index = ProjectLanguageIndex::new(vec![b, a]).unwrap();
    let diagnostics = index.diagnostics();

    assert_eq!(diagnostics, index.diagnostics());
    assert!(diagnostics.iter().any(|d| matches!(
        d,
        LanguageDiagnostic::MalformedGroup { source_id, command, .. }
            if source_id == "a.tex" && command == "section"
    )));
    assert!(diagnostics.iter().any(|d| matches!(
        d,
        LanguageDiagnostic::DuplicateLabel { key, .. } if key == "same"
    )));
    assert!(diagnostics.iter().any(|d| matches!(
        d,
        LanguageDiagnostic::MissingLabel { key, .. } if key == "missing"
    )));
    assert!(diagnostics.iter().any(|d| matches!(
        d,
        LanguageDiagnostic::MissingCitation { key, .. } if key == "absent"
    )));
    assert!(diagnostics.iter().any(|d| matches!(
        d,
        LanguageDiagnostic::IncludeCycle(cycle)
            if cycle == &vec!["a.tex".to_string(), "b.tex".to_string(), "a.tex".to_string()]
    )));
}

#[test]
fn stale_suppression_and_large_edit_revision_replacement() {
    let old =
        LanguageFileSnapshot::new("main.tex", 8, "\\section{Old}", TeXDialect::Latex).unwrap();
    let large_source = "가나다라마바사아자차카타파하\n".repeat(10_000) + "\\section{New}";
    let replacement =
        LanguageFileSnapshot::new("main.tex", 9, &large_source, TeXDialect::Latex).unwrap();
    let stale = LanguageFileSnapshot::new("main.tex", 7, "stale", TeXDialect::Latex).unwrap();

    let replaced = ProjectLanguageIndex::new(vec![old.clone()])
        .unwrap()
        .replacing(replacement.clone())
        .unwrap();
    let snapshot = &replaced.snapshots["main.tex"];
    let titles: Vec<&str> = snapshot.outline.iter().map(|o| o.title.as_str()).collect();
    assert_eq!(titles, ["New"]);
    assert_eq!(snapshot.revision, replacement.revision);
    assert_eq!(
        replaced.replacing(stale.clone()).unwrap_err(),
        LanguageCoreError::StaleResult {
            expected: replacement.revision.clone(),
            actual: stale.revision.clone(),
        }
    );
    assert!(replaced.snapshot("main.tex", &old.revision).is_err());
}

#[test]
fn minimap_ordering_is_by_source_range() {
    let snapshot = LanguageFileSnapshot::new(
        "main.tex",
        1,
        "\\section{A}\n\\label{x}\n\\ref{x}\n\\input{child}",
        TeXDialect::Latex,
    )
    .unwrap();
    let kinds: Vec<MinimapRangeKind> = snapshot
        .minimap_ranges
        .iter()
        .map(|r| r.kind)
        .collect();
    assert_eq!(
        kinds,
        [
            MinimapRangeKind::Outline,
            MinimapRangeKind::Definition,
            MinimapRangeKind::Use,
            MinimapRangeKind::Include
        ]
    );
    let mut offsets: Vec<i64> = snapshot
        .minimap_ranges
        .iter()
        .map(|r| r.range.utf8_offset)
        .collect();
    let sorted = offsets.clone();
    offsets.sort();
    assert_eq!(sorted, offsets);
}

// ---------------------------------------------------------------------------
// UnicodeCoordinateTests

#[test]
fn korean_japanese_emoji_and_utf16_surrogate_coordinates() {
    let map = UnicodeCoordinateMap::new("한😀\n日");

    assert_eq!(map.utf8_count, 11);
    assert_eq!(map.utf16_count, 5);
    assert_eq!(map.utf16_offset_for_utf8(3).unwrap(), 1);
    assert_eq!(map.utf16_offset_for_utf8(7).unwrap(), 3);
    assert_eq!(map.utf8_offset_for_utf16(3).unwrap(), 7);
    assert_eq!(
        map.position_for_utf8(7).unwrap(),
        TextPosition { line: 0, column: 3 }
    );
    assert_eq!(
        map.position_for_utf8(8).unwrap(),
        TextPosition { line: 1, column: 0 }
    );
    assert_eq!(
        map.utf8_offset_for_position(TextPosition { line: 1, column: 1 })
            .unwrap(),
        11
    );
    assert_eq!(
        map.utf16_offset_for_position(TextPosition { line: 1, column: 1 })
            .unwrap(),
        5
    );
}

#[test]
fn mid_scalar_and_surrogate_offsets_are_rejected() {
    let map = UnicodeCoordinateMap::new("😀");

    for invalid_utf8 in 1..=3i64 {
        assert_eq!(
            map.utf16_offset_for_utf8(invalid_utf8),
            Err(CoordinateMapError::InvalidUTF8Offset(invalid_utf8))
        );
    }
    assert_eq!(
        map.utf8_offset_for_utf16(1),
        Err(CoordinateMapError::InvalidUTF16Offset(1))
    );
    assert!(map.position_for_utf16(1).is_err());
}

#[test]
fn combining_sequence_scalar_boundary_is_rejected_as_mid_grapheme() {
    let source = "e\u{301}";
    let map = UnicodeCoordinateMap::new(source);

    assert_eq!(source.chars().count(), 2);
    assert_eq!(map.utf8_count, 3);
    assert_eq!(map.utf16_count, 2);
    assert!(map.utf16_offset_for_utf8(1).is_err());
    assert!(map.utf8_offset_for_utf16(1).is_err());
    assert!(map
        .utf8_offset_for_position(TextPosition { line: 0, column: 1 })
        .is_err());
    assert_eq!(
        map.utf8_offset_for_position(TextPosition { line: 0, column: 2 })
            .unwrap(),
        3
    );
}

#[test]
fn complex_emoji_grapheme_only_exposes_outer_boundaries() {
    let emoji = "👩🏽‍💻";
    let map = UnicodeCoordinateMap::new(&format!("A{emoji}B"));
    let emoji_utf8 = emoji.len() as i64;
    let emoji_utf16: i64 = emoji.chars().map(|c| c.len_utf16() as i64).sum();

    assert_eq!(map.utf16_offset_for_utf8(1).unwrap(), 1);
    assert_eq!(
        map.utf16_offset_for_utf8(1 + emoji_utf8).unwrap(),
        1 + emoji_utf16
    );
    assert!(map.utf16_offset_for_utf8(5).is_err());
    assert!(map.utf8_offset_for_utf16(3).is_err());
}

#[test]
fn all_valid_grapheme_boundaries_round_trip() {
    let source = "한국어 日本語 👨‍👩‍👧‍👦 café\n끝";
    let map = UnicodeCoordinateMap::new(source);
    let mut utf8 = 0i64;

    assert_eq!(map.utf8_offset_for_utf16(0).unwrap(), 0);
    for character in unicode_segmentation::UnicodeSegmentation::graphemes(source, true) {
        utf8 += character.len() as i64;
        let utf16 = map.utf16_offset_for_utf8(utf8).unwrap();
        assert_eq!(map.utf8_offset_for_utf16(utf16).unwrap(), utf8);
        let position = map.position_for_utf8(utf8).unwrap();
        assert_eq!(map.utf8_offset_for_position(position).unwrap(), utf8);
    }
}

#[test]
fn crlf_is_one_line_break_and_invalid_positions_fail_closed() {
    let map = UnicodeCoordinateMap::new("a\r\nb");

    assert_eq!(
        map.position_for_utf8(3).unwrap(),
        TextPosition { line: 1, column: 0 }
    );
    assert_eq!(
        map.utf8_offset_for_position(TextPosition { line: 1, column: 1 })
            .unwrap(),
        4
    );
    assert!(map
        .utf8_offset_for_position(TextPosition { line: -1, column: 0 })
        .is_err());
    assert!(map
        .utf16_offset_for_position(TextPosition { line: 0, column: 2 })
        .is_err());
    assert!(map.position_for_utf8(2).is_err());
}

// ---------------------------------------------------------------------------
// LargeFixtureTests

fn fixture_source() -> String {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .unwrap()
        .to_path_buf();
    let path = root.join("Fixtures/projects/large/main.tex");
    std::fs::read_to_string(&path).expect("large fixture must exist")
}

#[test]
fn large_fixture_has_deterministic_stress_characteristics() {
    let source = fixture_source();

    assert!(source.len() >= 1_048_576);
    assert!(source.contains("한국어"));
    assert!(source.contains("日本語"));
    assert!(source.contains("👩🏽‍💻"));
    assert!(source.contains("e\u{301}"));
    assert!(source.contains("\\begin{verbatim}"));
    assert!(source.contains("% block 0699"));
    assert!(source.matches("\\section{").count() >= 700);
    assert!(source.matches("\\label{").count() >= 700);
    assert!(source.matches("\\ref{").count() >= 700);
    assert!(source.split('\n').map(|l| l.len()).max().unwrap() > 1_000);
}

#[test]
fn lexer_index_revision_replacement_and_coordinates_are_deterministic() {
    let source = fixture_source();

    let first_tokens = DeterministicTeXLexer::tokenize(&source, TeXDialect::Latex);
    let second_tokens = DeterministicTeXLexer::tokenize(&source, TeXDialect::Latex);
    assert_eq!(first_tokens, second_tokens);
    assert!(first_tokens.len() > 5_000);
    assert!(first_tokens.len() < source.len());
    assert_eq!(
        first_tokens.last().unwrap().range.end_utf8_offset(),
        source.len() as i64
    );
    assert!(first_tokens
        .iter()
        .all(|t| t.range.end_utf8_offset() <= source.len() as i64));

    let initial =
        LanguageFileSnapshot::new("large/main.tex", 1, &source, TeXDialect::Latex).unwrap();
    assert!(initial.outline.len() >= 1_400);
    assert!(initial
        .references
        .iter()
        .filter(|r| r.kind == ReferenceKind::Label)
        .count()
        >= 1_400);
    assert!(initial
        .references
        .iter()
        .filter(|r| r.kind == ReferenceKind::Reference)
        .count()
        >= 1_400);
    assert!(initial.tokens.len() + initial.outline.len() + initial.references.len() < source.len());

    let index = ProjectLanguageIndex::new(vec![initial.clone()]).unwrap();
    let replacement_source = source.replace("unique-block-0699", "unique-block-0699-revised");
    let replacement =
        LanguageFileSnapshot::new("large/main.tex", 2, &replacement_source, TeXDialect::Latex)
            .unwrap();
    let replaced = index.replacing(replacement).unwrap();
    assert_eq!(replaced.snapshots.len(), 1);
    assert_eq!(replaced.snapshots["large/main.tex"].revision.revision, 2);
    assert!(replaced.snapshots["large/main.tex"]
        .source
        .contains("0699-revised"));
    assert!(replaced.replacing(initial).is_err());

    let sentinel = source.find("[0699:한:日:👩🏽‍💻:e\u{301}]").unwrap();
    let sentinel_utf8 = sentinel as i64;
    let map = UnicodeCoordinateMap::new(&source);
    let position = map.position_for_utf8(sentinel_utf8).unwrap();
    assert_eq!(map.utf8_offset_for_position(position).unwrap(), sentinel_utf8);
    let expected_utf16: i64 = source[..sentinel]
        .chars()
        .map(|c| c.len_utf16() as i64)
        .sum();
    assert_eq!(
        map.utf16_offset_for_utf8(sentinel_utf8).unwrap(),
        expected_utf16
    );
    assert_eq!(map.utf8_count, source.len() as i64);
    let total_utf16: i64 = source.chars().map(|c| c.len_utf16() as i64).sum();
    assert_eq!(map.utf16_count, total_utf16);
}
