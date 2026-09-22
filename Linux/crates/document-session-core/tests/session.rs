//! Port of `Packages/TexCore/Tests/TexCoreTests/DocumentSessionPersistenceTests.swift`.

use document_session_core::*;
use project_core::ProjectFile;
use std::path::PathBuf;
use tex_domain::{NormalizedRelativePath, StableDocumentID};

fn temp_root() -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "texspark-tests-{}-{nanos}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn main_file() -> ProjectFile {
    ProjectFile {
        document_id: StableDocumentID::new("main").unwrap(),
        path: NormalizedRelativePath::new("main.tex").unwrap(),
    }
}

fn make_session(initial_text: &str) -> DocumentSession {
    DocumentSession::new(&main_file(), initial_text.to_string(), None)
}

#[test]
fn canonical_project_and_relative_path_aliases_coalesce() {
    let root = temp_root();
    let nested = root.join("sources");
    std::fs::create_dir_all(&nested).unwrap();

    let registry = DocumentSessionRegistry::new();
    let canonical_file = ProjectFile {
        document_id: StableDocumentID::new("main").unwrap(),
        path: NormalizedRelativePath::new("sources/main.tex").unwrap(),
    };
    let aliased_file = ProjectFile {
        document_id: canonical_file.document_id.clone(),
        path: NormalizedRelativePath::new("sources/./draft/../main.tex").unwrap(),
    };

    let first = registry
        .open(&root, &canonical_file, "first".to_string(), None)
        .unwrap();
    let second = registry
        .open(
            &nested.join(".."),
            &aliased_file,
            "ignored duplicate".to_string(),
            None,
        )
        .unwrap();
    assert!(first.same_session(&second));
    assert_eq!(registry.count(), 1);
    let did_close = registry.close(&root, &first).unwrap();
    assert!(did_close);

    let reopened = registry
        .open(&root, &canonical_file, "reopened".to_string(), None)
        .unwrap();
    assert!(!first.same_session(&reopened));
    let reopened_snapshot = reopened.snapshot();
    assert_eq!(reopened_snapshot.text, "reopened");
    let stale_close = registry.close(&root, &first).unwrap();
    assert!(!stale_close);
    let registered = registry
        .session(&root, &canonical_file.path)
        .unwrap()
        .unwrap();
    assert!(registered.same_session(&reopened));

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn non_native_mutation_requires_and_checks_expected_revision() {
    let session = make_session("zero");

    let err = session
        .apply_originated(OriginatedDocumentMutation {
            mutation: DocumentMutation::ReplaceText("unversioned".into()),
            origin: DocumentMutationOrigin::AiProposal,
            expected_revision: None,
        })
        .unwrap_err();
    match err {
        ApplyOriginatedError::Revision(
            OriginatedDocumentMutationError::ExpectedRevisionRequired {
                origin: DocumentMutationOrigin::AiProposal,
            },
        ) => {}
        other => panic!("expected revision-required error, got {other:?}"),
    }

    session
        .apply_originated(OriginatedDocumentMutation {
            mutation: DocumentMutation::ReplaceText("one".into()),
            origin: DocumentMutationOrigin::TextKit,
            expected_revision: None,
        })
        .unwrap();

    let err = session
        .apply_originated(OriginatedDocumentMutation {
            mutation: DocumentMutation::ReplaceText("stale".into()),
            origin: DocumentMutationOrigin::Recovery,
            expected_revision: Some(0),
        })
        .unwrap_err();
    match err {
        ApplyOriginatedError::Session(DocumentSessionError::StaleRevision {
            expected: 0,
            actual: 1,
        }) => {}
        other => panic!("expected stale revision error, got {other:?}"),
    }

    let snapshot = session.snapshot();
    assert_eq!(snapshot.text, "one");
    assert_eq!(snapshot.content_hash, DiskContentHash::hashing("one"));
}

#[test]
fn external_collision_preserves_local_and_observed_disk_content() {
    let root = temp_root();
    let file = root.join("main.tex");
    std::fs::write(&file, "baseline").unwrap();
    let baseline_hash = DiskContentHash::hashing("baseline");
    std::fs::write(&file, "external edit").unwrap();

    let outcome = AtomicDocumentStore::new().save("local edit", &file, Some(baseline_hash));
    let DocumentSaveOutcome::StaleBaseline(conflict) = outcome else {
        panic!("expected stale baseline, got {outcome:?}")
    };
    assert_eq!(conflict.local.text, "local edit");
    assert_eq!(conflict.local.hash, DiskContentHash::hashing("local edit"));
    let observed = conflict.observed_disk.unwrap();
    assert_eq!(observed.text, "external edit");
    assert_eq!(observed.hash, DiskContentHash::hashing("external edit"));
    assert_eq!(std::fs::read_to_string(&file).unwrap(), "external edit");

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn atomic_successful_save_reloads_exact_content_and_hash() {
    let root = temp_root();
    let file = root.join("main.tex");
    std::fs::write(&file, "baseline").unwrap();
    let store = AtomicDocumentStore::new();

    let save = store.save(
        "saved π content",
        &file,
        Some(DiskContentHash::hashing("baseline")),
    );
    assert_eq!(
        save,
        DocumentSaveOutcome::Saved(PersistedDocument::new(
            "saved π content".to_string(),
            None
        ))
    );
    assert_eq!(
        store.load(&file, Some(DiskContentHash::hashing("saved π content"))),
        DocumentLoadOutcome::Loaded(PersistedDocument::new(
            "saved π content".to_string(),
            None
        ))
    );
    assert_eq!(std::fs::read(&file).unwrap(), "saved π content".as_bytes());

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn interrupted_write_removes_only_its_temporary_file() {
    let root = temp_root();
    let file = root.join("main.tex");
    let unrelated = root.join("keep.tmp");
    std::fs::write(&file, [0xFF]).unwrap();
    std::fs::write(&unrelated, "unrelated").unwrap();

    let outcome = AtomicDocumentStore::new().save(
        "local",
        &file,
        Some(DiskContentHash::new(1)),
    );
    assert!(
        matches!(outcome, DocumentSaveOutcome::InterruptedWrite { .. }),
        "expected interrupted write, got {outcome:?}"
    );

    assert_eq!(std::fs::read(&file).unwrap(), vec![0xFF]);
    assert_eq!(std::fs::read_to_string(&unrelated).unwrap(), "unrelated");
    let names: std::collections::HashSet<String> = std::fs::read_dir(&root)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        names,
        ["main.tex".to_string(), "keep.tmp".to_string()]
            .into_iter()
            .collect()
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn wrong_baseline_never_overwrites_or_loses_either_version() {
    let root = temp_root();
    let file = root.join("chapter.tex");
    std::fs::write(&file, "disk truth").unwrap();

    let outcome = AtomicDocumentStore::new().save(
        "valuable local work",
        &file,
        Some(DiskContentHash::hashing("old baseline")),
    );
    let DocumentSaveOutcome::StaleBaseline(conflict) = outcome else {
        panic!("expected stale baseline, got {outcome:?}")
    };
    assert_eq!(conflict.local.text, "valuable local work");
    assert_eq!(
        conflict.local.hash,
        DiskContentHash::hashing("valuable local work")
    );
    let observed = conflict.observed_disk.unwrap();
    assert_eq!(observed.text, "disk truth");
    assert_eq!(observed.hash, DiskContentHash::hashing("disk truth"));
    assert_eq!(std::fs::read_to_string(&file).unwrap(), "disk truth");

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn partial_edits_preserve_unicode_revisions_and_conflicts() {
    let session = make_session("한👩🏽‍💻 end");
    let first = session.apply(DocumentMutation::ReplaceRange {
        utf16_offset: 1, utf16_length: 7, text: "새".into(),
    }, 0).unwrap();
    assert_eq!(first.text, "한새 end");
    assert_eq!(first.content_hash, DiskContentHash::hashing(&first.text));
    assert_eq!(first.save_state, DocumentSaveState::Dirty);
    assert!(matches!(session.apply(DocumentMutation::ReplaceRange {
        utf16_offset: 0, utf16_length: 0, text: "stale".into(),
    }, 0), Err(DocumentSessionError::StaleRevision { .. })));
    let emoji = make_session("👩x");
    assert!(matches!(emoji.apply(DocumentMutation::ReplaceRange {
        utf16_offset: 1, utf16_length: 0, text: "bad".into(),
    }, 0), Err(DocumentSessionError::InvalidRange)));
    assert_eq!(emoji.snapshot().revision, 0);
    assert_eq!(emoji.snapshot().text, "👩x");
    let appended = emoji.apply(DocumentMutation::ReplaceRange {
        utf16_offset: 3, utf16_length: 0, text: "!".into(),
    }, 0).unwrap();
    assert_eq!(appended.text, "👩x!");
}
