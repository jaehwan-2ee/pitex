//! Run with: xvfb-run -a cargo test -p pitex-shell --test editor_rebase -- --ignored
//!
//! `commitSave`/`recordExternalChange`/`resolveConflict` bump the session
//! revision without the adapter knowing — the next native edit used to be
//! rejected and the buffer wholesale-replaced, losing the keystroke and
//! corrupting IME compositions. These run a real DocumentSession behind the
//! same `FnSessionClient` bridge `app_ui::session_client` builds.
//! One `#[test]` per binary: GTK can only be initialized once per process.
use app_ports::{DocumentMutationResult, DocumentSnapshot};
use document_session_core::{DiskContentHash, DocumentSession};
use gtk4::prelude::*;
use gtk_editor_adapter::{FnSessionClient, GtkEditorAdapter, SessionClient};
use project_core::ProjectFile;
use std::rc::Rc;
use tex_domain::{NormalizedRelativePath, StableDocumentID};

fn real_session(text: &str) -> DocumentSession {
    let file = ProjectFile {
        document_id: StableDocumentID::new("main").unwrap(),
        path: NormalizedRelativePath::new("main.tex").unwrap(),
    };
    DocumentSession::new(&file, text.to_string(), None)
}

/// `app_ui::session_client` verbatim shape: Ok → Applied, any rejection →
/// Rejected carrying the current session snapshot.
fn session_client(session: DocumentSession) -> Rc<dyn SessionClient> {
    let snap = session.clone();
    let submit = session;
    Rc::new(FnSessionClient::new(
        move || {
            let s = snap.snapshot();
            DocumentSnapshot { revision: s.revision, text: s.text }
        },
        move |mutation| {
            match submit.apply(
                document_session_core::DocumentMutation::ReplaceRange {
                    utf16_offset: mutation.range.location,
                    utf16_length: mutation.range.length,
                    text: mutation.replacement.clone(),
                },
                mutation.base_revision,
            ) {
                Ok(s) => DocumentMutationResult::Applied(
                    DocumentSnapshot { revision: s.revision, text: s.text },
                ),
                Err(_) => {
                    let s = submit.snapshot();
                    DocumentMutationResult::Rejected {
                        current: DocumentSnapshot { revision: s.revision, text: s.text },
                    }
                }
            }
        },
    ))
}

/// The pre-build/pre-agent save path: a text-neutral revision bump.
fn commit_save(session: &DocumentSession) {
    let revision = session.snapshot().revision;
    session
        .apply(
            document_session_core::DocumentMutation::CommitSave {
                written_disk_hash: DiskContentHash::hashing(&session.snapshot().text),
            },
            revision,
        )
        .unwrap();
}

fn caret_offset(buffer: &sourceview5::Buffer) -> i32 {
    buffer.iter_at_mark(&buffer.get_insert()).offset()
}

#[test]
#[ignore = "requires a GTK display (use xvfb-run)"]
fn stale_base_rebase_and_caret_regressions() {
    gtk4::init().unwrap();

    // 1. Type "ab", bump the revision text-neutrally (CommitSave), then insert
    //    "%" at offset 1 — the keystroke must survive the rebase.
    let session = real_session("ab");
    let adapter = GtkEditorAdapter::make(session_client(session.clone()));
    commit_save(&session);
    let buffer = adapter.buffer();
    buffer.place_cursor(&buffer.iter_at_offset(1));
    buffer.insert_at_cursor("%");
    assert_eq!(adapter.text(), "a%b");
    assert_eq!(session.snapshot().text, "a%b");
    assert_eq!(caret_offset(&buffer), 2);

    // 2. Save→build→agent persist cycles landing between every keystroke.
    let session = real_session("");
    let adapter = GtkEditorAdapter::make(session_client(session.clone()));
    let buffer = adapter.buffer();
    for ch in ["a", "%", "b"] {
        commit_save(&session);
        buffer.place_cursor(&buffer.end_iter());
        buffer.insert_at_cursor(ch);
    }
    assert_eq!(adapter.text(), "a%b");
    assert_eq!(session.snapshot().text, "a%b");
    assert_eq!(caret_offset(&buffer), 3);

    // 3. An external change BEFORE the caret (ResolveConflict with a prefix)
    //    followed by refresh must shift the caret by the inserted length and
    //    keep it on the same logical character.
    let session = real_session("aXb");
    let adapter = GtkEditorAdapter::make(session_client(session.clone()));
    let buffer = adapter.buffer();
    buffer.place_cursor(&buffer.iter_at_offset(2));
    let revision = session.snapshot().revision;
    session
        .apply(
            document_session_core::DocumentMutation::ResolveConflict {
                text: "prefix_aXb".into(),
                disk_baseline_hash: DiskContentHash::hashing("prefix_aXb"),
            },
            revision,
        )
        .unwrap();
    adapter.refresh_from_session();
    assert_eq!(adapter.text(), "prefix_aXb");
    assert_eq!(caret_offset(&buffer), 2 + "prefix_".len() as i32);
    assert_eq!(buffer.iter_at_mark(&buffer.get_insert()).char(), 'b');

    // 4. Shift+Return in the editor fires the build callback and is consumed;
    //    an active IM preedit or extra modifiers leave it alone.
    let view = sourceview5::View::new();
    let built = Rc::new(std::cell::Cell::new(0u32));
    let marked = Rc::new(std::cell::Cell::new(false));
    let controller = pitex_shell::app_ui::install_build_key(
        &view,
        Rc::new({
            let marked = marked.clone();
            move || marked.get()
        }),
        Rc::new({
            let built = built.clone();
            move || built.set(built.get() + 1)
        }),
    );
    let press = |key: gtk4::gdk::Key, state: gtk4::gdk::ModifierType| -> bool {
        controller.emit_by_name::<bool>("key-pressed", &[&key, &0u32, &state])
    };
    assert!(press(gtk4::gdk::Key::Return, gtk4::gdk::ModifierType::SHIFT_MASK));
    assert_eq!(built.get(), 1);
    assert!(view.buffer().text(&view.buffer().start_iter(), &view.buffer().end_iter(), true).is_empty());
    // KP_Enter too.
    assert!(press(gtk4::gdk::Key::KP_Enter, gtk4::gdk::ModifierType::SHIFT_MASK));
    assert_eq!(built.get(), 2);
    // Extra modifiers pass through.
    assert!(!press(gtk4::gdk::Key::Return, gtk4::gdk::ModifierType::SHIFT_MASK | gtk4::gdk::ModifierType::CONTROL_MASK));
    assert_eq!(built.get(), 2);
    // While an IM is composing the key belongs to the IM.
    marked.set(true);
    assert!(!press(gtk4::gdk::Key::Return, gtk4::gdk::ModifierType::SHIFT_MASK));
    assert_eq!(built.get(), 2);
}
