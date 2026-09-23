//! Run with: xvfb-run -a cargo test -p pitex-shell --test native -- --ignored
//! One `#[test]` per binary: GTK initializes on a single thread per process,
//! and libtest runs each test on its own thread.
use app_ports::{DocumentMutation, DocumentMutationResult, DocumentSnapshot};
use editor_feature::{decoration_tag_name, EditorDecoration, EditorDecorationSnapshot};
use gtk4::prelude::*;
use gtk_editor_adapter::{FnSessionClient, GtkEditorAdapter};
use language_core::{DeterministicTeXLexer, TeXDialect};
use pitex_shell::{model::WorkspaceMessage, pdf::PdfRenderer};
use std::{cell::RefCell, rc::Rc, sync::{mpsc, Arc}, time::Duration};

#[test]
#[ignore = "requires a GTK display (use xvfb-run)"]
fn native_edits_highlighting_and_pdf_worker() {
    gtk4::init().unwrap();
    git_diff_surface_folds_and_expands();
    #[cfg(unix)]
    for stop in [false, true] {
        let process = pitex_shell::agent::PiAgentProcess::start(
            std::path::Path::new("/bin/sh"), std::path::Path::new("/tmp"),
            &["-c".into(), if stop { "exec sleep 60" } else { "exit 0" }.into()],
            &std::env::vars().collect(),
        ).unwrap();
        if stop { process.terminate(); }
        process.exited.lock().unwrap().recv_timeout(Duration::from_secs(5)).unwrap();
        assert!(!process.is_running());
    }
    let original = "한👩 \\section{Intro}\nText $x$\n% tail\n";
    let snapshot = Rc::new(RefCell::new(DocumentSnapshot { revision: 0, text: original.into() }));
    let mutations = Rc::new(RefCell::new(Vec::<DocumentMutation>::new()));
    let (read, write, recorded) = (snapshot.clone(), snapshot.clone(), mutations.clone());
    let adapter = GtkEditorAdapter::make(Rc::new(FnSessionClient::new(
        move || read.borrow().clone(),
        move |mutation| {
            let mut state = write.borrow_mut();
            assert_eq!(mutation.base_revision, state.revision);
            let mut units: Vec<_> = state.text.encode_utf16().collect();
            let range = mutation.range.location..mutation.range.location + mutation.range.length;
            units.splice(range, mutation.replacement.encode_utf16());
            state.text = String::from_utf16(&units).unwrap();
            state.revision += 1;
            recorded.borrow_mut().push(mutation.clone());
            DocumentMutationResult::Applied(state.clone())
        },
    )));
    let paint_and_check = || {
        let state = snapshot.borrow();
        let tokens = DeterministicTeXLexer::tokenize(&state.text, TeXDialect::Latex);
        let decorations = tokens.iter().map(|t| EditorDecoration { range: t.range, token_kind: t.kind.clone() }).collect();
        adapter.apply_decorations(&EditorDecorationSnapshot::new(state.revision, decorations));
        for token in tokens {
            let start = token.range.utf8_offset as usize;
            let end = start + token.range.utf8_length as usize;
            for (relative, _) in state.text[start..end].char_indices() {
                let offset = state.text[..start + relative].chars().count() as i32;
                let tags: Vec<_> = adapter.buffer().iter_at_offset(offset).tags().iter()
                    .filter_map(|tag| tag.name()).filter(|name| name.starts_with("pitex.decoration.")).collect();
                assert_eq!(tags.len(), 1, "overlapping or missing tags at {offset}");
                assert_eq!(tags[0].as_str(), decoration_tag_name(&token.kind));
            }
        }
    };
    paint_and_check();
    let buffer = adapter.buffer();
    buffer.begin_user_action();
    buffer.insert(&mut buffer.iter_at_offset(2), "글");
    buffer.end_user_action();
    assert_eq!(mutations.borrow()[0].range.location, 3);
    assert_eq!(mutations.borrow()[0].range.length, 0);
    assert_eq!(mutations.borrow()[0].replacement, "글");
    paint_and_check();
    adapter.undo();
    assert_eq!(adapter.text(), original);
    paint_and_check();
    adapter.redo();
    assert!(adapter.text().starts_with("한👩글"));
    paint_and_check();
    // Removing a math delimiter changes the lexical state of the entire tail.
    let at = adapter.text().chars().position(|c| c == '$').unwrap() as i32;
    buffer.delete(&mut buffer.iter_at_offset(at), &mut buffer.iter_at_offset(at + 1));
    paint_and_check();
    assert_eq!(adapter.text(), snapshot.borrow().text);

    let (sender, receiver) = mpsc::channel();
    let renderer = PdfRenderer::new(sender);
    let pdf: Arc<[u8]> = sample_pdf().into();
    renderer.load(1, pdf.clone());
    match receiver.recv_timeout(Duration::from_secs(10)).unwrap() {
        WorkspaceMessage::PdfLoaded { hash: 1, info: Some(info) } => {
            assert_eq!(info.page_count(), 1);
            assert_eq!(info.page_size(0), Some((120.0, 80.0)));
        }
        other => panic!("{other:?}"),
    }
    renderer.render(1, 10, 0, 1.0);
    let pixels = match receiver.recv_timeout(Duration::from_secs(10)).unwrap() {
        WorkspaceMessage::PdfRendered { key: 10, raster } => {
            assert_eq!((raster.width, raster.height), (120, 80));
            assert_eq!(raster.pixels.len(), raster.stride as usize * 80);
            assert!(raster.pixels.chunks_exact(4).any(|p| p[0] > p[2]));
            raster.pixels
        }
        other => panic!("{other:?}"),
    };
    renderer.render(1, 10, 0, 1.0);
    match receiver.recv_timeout(Duration::from_secs(10)).unwrap() {
        WorkspaceMessage::PdfRendered { raster, .. } => assert!(Arc::ptr_eq(&pixels, &raster.pixels)),
        other => panic!("{other:?}"),
    }
    renderer.load(2, pdf);
    assert!(matches!(receiver.recv_timeout(Duration::from_secs(10)).unwrap(), WorkspaceMessage::PdfLoaded { hash: 2, .. }));
    renderer.render(1, 11, 0, 2.0); // superseded document must never render
    renderer.render(2, 12, 0, 2.0);
    match receiver.recv_timeout(Duration::from_secs(10)).unwrap() {
        WorkspaceMessage::PdfRendered { key: 12, raster } => assert_eq!((raster.width, raster.height), (240, 160)),
        other => panic!("{other:?}"),
    }
}

fn sample_pdf() -> Vec<u8> {
    let stream = "0 0 1 rg 10 10 50 30 re f\n";
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".into(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".into(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 120 80] /Contents 4 0 R >>".into(),
        format!("<< /Length {} >>\nstream\n{stream}endstream", stream.len()),
    ];
    let mut output = String::from("%PDF-1.4\n");
    let mut offsets = vec![0];
    for (i, object) in objects.iter().enumerate() {
        offsets.push(output.len());
        output.push_str(&format!("{} 0 obj\n{object}\nendobj\n", i + 1));
    }
    let xref = output.len();
    output.push_str("xref\n0 5\n0000000000 65535 f \n");
    for offset in offsets.into_iter().skip(1) { output.push_str(&format!("{offset:010} 00000 n \n")); }
    output.push_str(&format!("trailer\n<< /Size 5 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n"));
    output.into_bytes()
}

/// The `CommitDiffView` surface: a parsed patch becomes lazily bound
/// rows, a long unchanged run renders as a fold bar, and expanding it
/// reveals the hidden context pairs. Called from the test above, on the
/// thread that initialized GTK.
fn git_diff_surface_folds_and_expands() {
    use pitex_shell::git_diff::{DiffList, DiffRow};
    let mut patch = String::from(
        "diff --git a/main.tex b/main.tex\nindex 1111111..2222222 100644\n--- a/main.tex\n+++ b/main.tex\n@@ -1,21 +1,21 @@\n",
    );
    for n in 1..=20 {
        patch.push_str(&format!(" line {n}\n"));
    }
    patch.push_str("-old tail\n+new tail\n");
    let section = git_core::GitDiffFileSection {
        file: git_core::GitCommitFile {
            path: "main.tex".into(),
            kind: git_core::GitChangeKind::Modified,
        },
        binary: false,
        rows: git_core::parse_file_diff(&patch),
    };
    let list = DiffList::new();
    list.set_sections(1, true, vec![section]);
    // 3 context + fold(14) + 3 context + the change pair.
    assert_eq!(list.n_items(), 8);
    let Some(DiffRow::Fold { section, id, count }) = list.row(3) else {
        panic!("middle row should be the fold")
    };
    assert_eq!(count, 14);
    list.expand_fold(section, id);
    assert_eq!(list.n_items(), 8 - 1 + 14);
    assert!(matches!(list.row(3), Some(DiffRow::Pair { .. })));
    // A re-set of the same session keeps the expansion instead of
    // re-folding on every refresh.
    list.set_sections(1, true, vec![git_core::GitDiffFileSection {
        file: git_core::GitCommitFile {
            path: "main.tex".into(),
            kind: git_core::GitChangeKind::Modified,
        },
        binary: false,
        rows: git_core::parse_file_diff(&patch),
    }]);
    assert_eq!(list.n_items(), 8 - 1 + 14);
}
