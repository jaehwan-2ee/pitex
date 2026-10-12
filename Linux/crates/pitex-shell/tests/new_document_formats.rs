//! File → New Document formats — model-level coverage for
//! `create_document(format:)`: each format writes `untitled.<ext>` under
//! the project root in its own naming sequence with its own initial
//! contents, registers and activates the file, never overwrites, and a
//! fresh .bib/.md never takes over the TeX build target. The "+" menu's
//! New Document submenu keeps all three formats on every build.

use gtk4::prelude::*;
use pitex_shell::app_ui::new_document_submenu;
use pitex_shell::model::{NewDocumentFormat, WorkspaceMessage, WorkspaceModel, WorkspacePhase};
use std::path::PathBuf;
use std::time::Duration;

const TEX_TEMPLATE: &str = "\\documentclass{article}\n\\begin{document}\n\n\\end{document}\n";

struct TempDir(PathBuf);

impl TempDir {
    fn new(tag: &str) -> Self {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "pitex-shell-newdoc-{tag}-{}-{nanos}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }
    fn join(&self, rest: &str) -> PathBuf {
        self.0.join(rest)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// A minimal open project — `main.tex` is the sole main document.
fn project(tag: &str) -> (WorkspaceModel, TempDir, PathBuf) {
    let dir = TempDir::new(tag);
    let main = dir.join("main.tex");
    std::fs::write(
        &main,
        "\\documentclass{article}\n\\begin{document}\nBody.\n\\end{document}\n",
    )
    .unwrap();
    let mut model = WorkspaceModel::new();
    model.project_url = Some(dir.0.clone());
    model.project_files = vec![main.clone()];
    (model, dir, main)
}

/// `create_document` ends by activating the file — the result arrives on
/// the channel; apply it like the GTK dispatch does.
fn create(model: &mut WorkspaceModel, format: NewDocumentFormat) -> PathBuf {
    let (tx, rx) = std::sync::mpsc::channel();
    model.create_document(format, tx);
    match rx
        .recv_timeout(Duration::from_secs(10))
        .expect("activation reported back")
    {
        WorkspaceMessage::ActivateFinished(Ok(activated)) => {
            let url = activated.url.clone();
            model.apply_activate(activated);
            url
        }
        other => panic!("expected ActivateFinished(Ok), got {other:?}"),
    }
}

#[test]
fn creates_each_format_with_expected_contents_and_activation() {
    let (mut model, dir, _main) = project("formats");

    // LaTeX keeps exactly the template `create_document` always wrote.
    let tex = create(&mut model, NewDocumentFormat::Latex);
    assert_eq!(tex, dir.join("untitled.tex"));
    assert_eq!(std::fs::read_to_string(&tex).unwrap(), TEX_TEMPLATE);
    assert_eq!(model.active_document_url.as_ref(), Some(&tex));
    assert!(model.project_files.contains(&tex));
    // The new main resolves itself as the build target.
    assert_eq!(model.build_source_url().as_ref(), Some(&tex));

    // BibTeX and Markdown start empty and activate through the editor.
    let bib = create(&mut model, NewDocumentFormat::Bibtex);
    assert_eq!(bib, dir.join("untitled.bib"));
    assert_eq!(std::fs::read_to_string(&bib).unwrap(), "");
    assert_eq!(model.active_document_url.as_ref(), Some(&bib));
    // Two mains now exist, so resolution is ambiguous — but the .bib
    // itself must never land in the target slot.
    assert_ne!(model.build_source_url().as_ref(), Some(&bib));

    let md = create(&mut model, NewDocumentFormat::Markdown);
    assert_eq!(md, dir.join("untitled.md"));
    assert_eq!(std::fs::read_to_string(&md).unwrap(), "");
    assert_eq!(model.active_document_url.as_ref(), Some(&md));
    assert!(model.active_is_markdown());
    assert_ne!(model.build_source_url().as_ref(), Some(&md));

    // A .bib in a single-main project keeps that main as the target.
    let (mut model, dir, main) = project("formats-bib-target");
    let bib = create(&mut model, NewDocumentFormat::Bibtex);
    assert_eq!(
        model.build_source_url().as_deref(),
        Some(main.as_path()),
        "a new .bib must not become the TeX build target"
    );
    assert!(dir.join("untitled.bib").exists() && bib == dir.join("untitled.bib"));
}

#[test]
fn untitled_sequences_are_per_extension_and_never_overwrite() {
    let (mut model, dir, _main) = project("collide");
    std::fs::write(dir.join("untitled.tex"), "existing tex\n").unwrap();
    std::fs::write(dir.join("untitled.bib"), "existing bib\n").unwrap();

    let tex = create(&mut model, NewDocumentFormat::Latex);
    assert_eq!(tex, dir.join("untitled-2.tex"));
    let bib = create(&mut model, NewDocumentFormat::Bibtex);
    // The .bib counter starts fresh — it does not continue the .tex one.
    assert_eq!(bib, dir.join("untitled-2.bib"));
    let md = create(&mut model, NewDocumentFormat::Markdown);
    assert_eq!(md, dir.join("untitled.md"));

    // Pre-existing files were left untouched.
    assert_eq!(
        std::fs::read_to_string(dir.join("untitled.tex")).unwrap(),
        "existing tex\n"
    );
    assert_eq!(
        std::fs::read_to_string(dir.join("untitled.bib")).unwrap(),
        "existing bib\n"
    );
}

#[test]
fn create_without_a_project_is_a_noop() {
    let mut model = WorkspaceModel::new();
    for format in NewDocumentFormat::ALL {
        let (tx, rx) = std::sync::mpsc::channel();
        model.create_document(format, tx);
        assert!(
            rx.recv_timeout(Duration::from_millis(300)).is_err(),
            "{format:?} without a project must not produce messages"
        );
    }
    assert!(matches!(model.phase, WorkspacePhase::NoProject));
    assert!(model.project_files.is_empty() && model.active_document_url.is_none());
}

/// Both "+" menu builds go through `new_document_submenu` — assert the
/// one shared model keeps the fixed action contract, Ctrl+N's
/// `win.newdoc` first. (GTK's second build path is `refresh_phase`.)
#[test]
fn new_document_menu_keeps_all_formats() {
    let menu = new_document_submenu("en");
    assert_eq!(menu.n_items(), 3);
    let actions: Vec<String> = (0..menu.n_items())
        .map(|i| {
            menu.item_attribute_value(i, "action", None)
                .and_then(|v| v.get::<String>())
                .unwrap_or_default()
        })
        .collect();
    assert_eq!(
        actions,
        ["win.newdoc", "win.newbib", "win.newmd"],
        "every format keeps its own window action — newdoc stays first"
    );
}

/// The same helper's labels resolve against the shared `.strings`
/// tables — every shipped locale gets three distinct translated items,
/// never the raw key `tr` returns on a miss.
#[test]
fn new_document_menu_labels_are_localized() {
    for lang in ["en", "ko", "ja", "vi", "es", "ru", "zh-Hans"] {
        let menu = new_document_submenu(lang);
        let labels: Vec<String> = (0..menu.n_items())
            .map(|i| {
                menu.item_attribute_value(i, "label", None)
                    .and_then(|v| v.get::<String>())
                    .unwrap_or_default()
            })
            .collect();
        assert_eq!(labels.len(), 3, "{lang}: New Document submenu lost items");
        for (label, format) in labels.iter().zip(NewDocumentFormat::ALL.iter()) {
            assert!(
                !label.is_empty() && label != format.label_key(),
                "{lang}: {format:?} produced an unlocalized label {label:?}"
            );
        }
        let distinct: std::collections::BTreeSet<_> = labels.iter().collect();
        assert_eq!(
            distinct.len(),
            labels.len(),
            "{lang}: format labels must differ, got {labels:?}"
        );
    }
}
