//! Port of `Packages/TexCore/Tests/TexCoreTests/SyncTeXQueryTests.swift`.

use std::path::PathBuf;
use synctex_core::*;

const ROOT: &str = "/workspace/multifile";
const OUTPUT_HASH: &str = "sha256:8d5c2f1a";

fn fixture(name: &str) -> String {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .unwrap()
        .to_path_buf();
    std::fs::read_to_string(root.join("Fixtures/synctex").join(name)).unwrap()
}

fn revision() -> SyncTeXRevision {
    SyncTeXRevision::new("build-42", 7_829_367).unwrap()
}

fn binding(revision: &SyncTeXRevision) -> SyncTeXOutputBinding {
    SyncTeXOutputBinding::new(revision.clone(), OUTPUT_HASH).unwrap()
}

fn parse_fixture(name: &str, revision: &SyncTeXRevision) -> SyncTeXQueryDocument {
    SyncTeXQueryParser::parse(&fixture(name), ROOT, &binding(revision)).unwrap()
}

fn path(v: &str) -> NormalizedSourcePath {
    NormalizedSourcePath::new(v).unwrap()
}

#[test]
fn forward_selects_only_exact_child_source_and_pdf() {
    let revision = revision();
    let document = parse_fixture("forward-output.txt", &revision);
    assert_eq!(document.metadata.version, 1);
    assert_eq!(document.metadata.fingerprint, 7_829_367);
    assert_eq!(document.candidates.len(), 3);

    let source = SourceLocation::new(path("sections/details.tex"), 3, 2).unwrap();
    let query = ForwardSyncQuery {
        revision,
        source,
        expected_pdf: path("build/main.pdf"),
    };
    let selected =
        ExactSyncTeXQuerySelector::forward(&document.candidates, &query, OUTPUT_HASH).unwrap();
    assert_eq!(selected.source.path.value, "sections/details.tex");
    assert_eq!(selected.pdf.page, 2);
    assert_eq!(selected.pdf.point, PDFPoint::new(144.25, 221.75).unwrap());
    assert_eq!(selected.h, 144.25);
    assert_eq!(selected.v, 221.75);
    assert_eq!(selected.width, 275.5);
    assert_eq!(selected.height, 7.25);
}

#[test]
fn inverse_returns_exact_child_file_and_line() {
    let revision = revision();
    let document = parse_fixture("inverse-output.txt", &revision);
    let pdf = PDFLocation::new(
        path("build/main.pdf"),
        2,
        PDFPoint::new(144.25, 221.75).unwrap(),
    )
    .unwrap();
    let selected = ExactSyncTeXQuerySelector::inverse(
        &document.candidates,
        &InverseSyncQuery {
            revision,
            pdf,
        },
        OUTPUT_HASH,
        0.0,
    )
    .unwrap();
    assert_eq!(selected.source.path.value, "sections/details.tex");
    assert_eq!(selected.source.line, 3);
    assert_eq!(selected.source.column, 2);
}

#[test]
fn inverse_epsilon_identifies_only_a_unique_block_and_never_nearest_guesses() {
    let revision = revision();
    let candidates = parse_fixture("inverse-output.txt", &revision).candidates;
    let pdf = PDFLocation::new(
        path("build/main.pdf"),
        2,
        PDFPoint::new(144.25005, 221.75005).unwrap(),
    )
    .unwrap();
    let query = InverseSyncQuery {
        revision: revision.clone(),
        pdf,
    };
    let selected =
        ExactSyncTeXQuerySelector::inverse(&candidates, &query, OUTPUT_HASH, 0.0001).unwrap();
    assert_eq!(selected.source.path.value, "sections/details.tex");
    assert_eq!(
        ExactSyncTeXQuerySelector::inverse(&candidates, &query, OUTPUT_HASH, 0.01).unwrap_err(),
        SyncTeXQueryError::AmbiguousMatch { count: 2 }
    );

    let between = PDFLocation::new(
        path("build/main.pdf"),
        2,
        PDFPoint::new(144.2507, 221.7507).unwrap(),
    )
    .unwrap();
    let between_query = InverseSyncQuery {
        revision,
        pdf: between,
    };
    assert_eq!(
        ExactSyncTeXQuerySelector::inverse(&candidates, &between_query, OUTPUT_HASH, 0.0001)
            .unwrap_err(),
        SyncTeXQueryError::NoMatch
    );
}

#[test]
fn duplicate_exact_candidates_are_ambiguous() {
    let revision = revision();
    let document = parse_fixture("forward-output.txt", &revision);
    let candidate = document.candidates[2].clone();
    let query = ForwardSyncQuery {
        revision,
        source: candidate.source.clone(),
        expected_pdf: candidate.pdf.pdf_path.clone(),
    };
    let mut candidates = document.candidates.clone();
    candidates.push(candidate);
    assert_eq!(
        ExactSyncTeXQuerySelector::forward(&candidates, &query, OUTPUT_HASH).unwrap_err(),
        SyncTeXQueryError::AmbiguousMatch { count: 2 }
    );
}

#[test]
fn revision_and_output_hash_must_both_match() {
    let actual_revision = revision();
    let document = parse_fixture("forward-output.txt", &actual_revision);
    let candidate = &document.candidates[0];
    let stale_revision = SyncTeXRevision::new("build-43", 7_829_367).unwrap();
    let stale_query = ForwardSyncQuery {
        revision: stale_revision,
        source: candidate.source.clone(),
        expected_pdf: candidate.pdf.pdf_path.clone(),
    };
    assert_eq!(
        ExactSyncTeXQuerySelector::forward(&document.candidates, &stale_query, OUTPUT_HASH)
            .unwrap_err(),
        SyncTeXQueryError::StaleResult
    );

    let current_query = ForwardSyncQuery {
        revision: actual_revision.clone(),
        source: candidate.source.clone(),
        expected_pdf: candidate.pdf.pdf_path.clone(),
    };
    assert_eq!(
        ExactSyncTeXQuerySelector::forward(
            &document.candidates,
            &current_query,
            "sha256:different"
        )
        .unwrap_err(),
        SyncTeXQueryError::StaleResult
    );

    let stale_metadata = fixture("forward-output.txt").replace(
        "SyncTeX Fingerprint:7829367",
        "SyncTeX Fingerprint:7829368",
    );
    assert_eq!(
        SyncTeXQueryParser::parse(&stale_metadata, ROOT, &binding(&actual_revision))
            .unwrap_err(),
        SyncTeXQueryError::StaleResult
    );
}

#[test]
fn locale_decimal_and_malformed_numeric_fields_are_rejected() {
    let fixture = fixture("forward-output.txt");
    let binding = binding(&revision());
    for malformed in [
        fixture.replace("x:133.768341", "x:133,768341"),
        fixture.replace("W:343.711060", "W:nan"),
        fixture.replace("Column:0", "Column:-1"),
        fixture.replace(
            "SyncTeX Fingerprint:7829367",
            "SyncTeX Fingerprint:7 829 367",
        ),
    ] {
        assert!(SyncTeXQueryParser::parse(&malformed, ROOT, &binding).is_err());
    }
}

#[test]
fn outside_root_and_lexical_escape_paths_are_rejected() {
    let fixture = fixture("forward-output.txt");
    let binding = binding(&revision());
    let outside = fixture.replace(
        "/workspace/multifile/sections/intro.tex",
        "/workspace/other/intro.tex",
    );
    assert!(matches!(
        SyncTeXQueryParser::parse(&outside, ROOT, &binding).unwrap_err(),
        SyncTeXQueryError::PathOutsideRoot { .. }
    ));
    let lexical_escape = fixture.replace(
        "/workspace/multifile/sections/intro.tex",
        "/workspace/multifile/sections/../intro.tex",
    );
    assert!(matches!(
        SyncTeXQueryParser::parse(&lexical_escape, ROOT, &binding).unwrap_err(),
        SyncTeXQueryError::PathOutsideRoot { .. }
    ));
}

#[test]
fn line_and_page_boundaries_are_strict() {
    let fixture = fixture("forward-output.txt");
    let binding = binding(&revision());
    for malformed in [
        fixture.replace("Line:4", "Line:0"),
        fixture.replace("Page:1", "Page:0"),
        fixture.replace("Line:4", "Line:9223372036854775808"),
    ] {
        assert!(matches!(
            SyncTeXQueryParser::parse(&malformed, ROOT, &binding).unwrap_err(),
            SyncTeXQueryError::Malformed { .. }
        ));
    }
}

#[test]
fn no_match_does_not_fall_back_to_basename_page_or_nearest_point() {
    let revision = revision();
    let document = parse_fixture("forward-output.txt", &revision);
    let source = SourceLocation::new(path("other/details.tex"), 3, 2).unwrap();
    let query = ForwardSyncQuery {
        revision,
        source,
        expected_pdf: path("build/main.pdf"),
    };
    assert_eq!(
        ExactSyncTeXQuerySelector::forward(&document.candidates, &query, OUTPUT_HASH)
            .unwrap_err(),
        SyncTeXQueryError::NoMatch
    );
}

#[test]
fn parsing_is_deterministic() {
    let revision = revision();
    let binding = binding(&revision);
    let first = SyncTeXQueryParser::parse(&fixture("inverse-output.txt"), ROOT, &binding).unwrap();
    let second = SyncTeXQueryParser::parse(&fixture("inverse-output.txt"), ROOT, &binding).unwrap();
    assert_eq!(first, second);
    let paths: Vec<&str> = first
        .candidates
        .iter()
        .map(|c| c.source.path.value.as_str())
        .collect();
    assert_eq!(
        paths,
        ["sections/intro.tex", "sections/details.tex", "main.tex"]
    );
}

/// `synctex view` result blocks carry only Output/Page/geometry; the runner
/// injects Input/Line/Column from the query. `synctex edit` blocks carry
/// Input/Line/Column (Column often -1, normalized to the query value) while
/// Page/x/y/h/v/W/H/Output are injected. Input paths may contain `.`
/// components that must canonicalize under the project root.
#[test]
fn normalized_records_from_real_cli_shape() {
    let revision = revision();
    let binding = binding(&revision);
    let view_normalized = "SyncTeX Version:1\nSyncTeX Fingerprint:7829367\nSyncTeX result begin\nInput:/workspace/multifile/main.tex\nLine:4\nColumn:0\nOutput:/workspace/multifile/build/main.pdf\nPage:1\nx:133.768356\ny:167.710464\nh:133.768356\nv:167.710464\nW:343.711060\nH:9.843078\nSyncTeX result end\n";
    let forward_doc = SyncTeXQueryParser::parse(view_normalized, ROOT, &binding).unwrap();
    let forward = ExactSyncTeXQuerySelector::forward(
        &forward_doc.candidates,
        &ForwardSyncQuery {
            revision: revision.clone(),
            source: SourceLocation::new(path("main.tex"), 4, 0).unwrap(),
            expected_pdf: path("build/main.pdf"),
        },
        OUTPUT_HASH,
    )
    .unwrap();
    assert_eq!(forward.pdf.page, 1);
    assert_eq!(
        forward.pdf.point,
        PDFPoint::new(133.768356, 167.710464).unwrap()
    );

    let edit_normalized = "SyncTeX Version:1\nSyncTeX Fingerprint:7829367\nSyncTeX result begin\nInput:/workspace/multifile/./sections/details.tex\nLine:6\nColumn:0\nOutput:/workspace/multifile/build/main.pdf\nPage:1\nx:150\ny:700\nh:150\nv:700\nW:0\nH:0\nSyncTeX result end\n";
    let inverse_doc = SyncTeXQueryParser::parse(edit_normalized, ROOT, &binding).unwrap();
    let inverse = ExactSyncTeXQuerySelector::inverse(
        &inverse_doc.candidates,
        &InverseSyncQuery {
            revision,
            pdf: PDFLocation::new(path("build/main.pdf"), 1, PDFPoint::new(150.0, 700.0).unwrap())
                .unwrap(),
        },
        OUTPUT_HASH,
        2.0,
    )
    .unwrap();
    assert_eq!(inverse.source.path.value, "sections/details.tex");
    assert_eq!(inverse.source.line, 6);
}
