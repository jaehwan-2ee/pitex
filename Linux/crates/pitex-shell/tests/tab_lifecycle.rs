//! Tab lifecycle — what a discarded tab leaves behind in the session
//! registry, and which activation wins when results arrive late.

use document_session_core::{DocumentMutation, DocumentSaveState};
use pitex_shell::model::{WorkspaceMessage, WorkspaceModel, WorkspacePhase};
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver};
use std::time::Duration;

struct TempDir(PathBuf);

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// An open local project with `a.tex`, `b.tex` and `c.tex` listed.
fn project(tag: &str) -> (WorkspaceModel, TempDir, [PathBuf; 3]) {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("pitex-tabs-{tag}-{}-{nanos}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let files = ["a", "b", "c"].map(|name| {
        let url = dir.join(format!("{name}.tex"));
        std::fs::write(&url, format!("{name} on disk\n")).unwrap();
        url
    });
    let mut model = WorkspaceModel::new();
    model.project_url = Some(dir.clone());
    model.project_files = files.to_vec();
    (model, TempDir(dir), files)
}

fn request(model: &mut WorkspaceModel, url: &PathBuf) -> Receiver<WorkspaceMessage> {
    let (tx, rx) = channel();
    model.activate_document(url.clone(), tx);
    rx
}

fn deliver(model: &mut WorkspaceModel, rx: Receiver<WorkspaceMessage>) -> bool {
    match rx.recv_timeout(Duration::from_secs(10)).expect("activation reported back") {
        WorkspaceMessage::ActivateFinished { generation, root, result } => {
            model.finish_activate(generation, root, result).is_some()
        }
        other => panic!("expected ActivateFinished, got {other:?}"),
    }
}

fn activate(model: &mut WorkspaceModel, url: &PathBuf) {
    let rx = request(model, url);
    assert!(deliver(model, rx), "the latest activation applies");
    assert_eq!(model.active_document_url.as_ref(), Some(url));
}

/// R11: Don't Save on an inactive tab must drop its draft, so reopening
/// shows the disk text, clean.
#[test]
fn discarding_an_inactive_tab_drops_its_draft() {
    let (mut model, _dir, [a, b, _]) = project("discard-inactive");
    activate(&mut model, &a);
    let session = model.registered_sessions[0].clone();
    session
        .apply(DocumentMutation::ReplaceText("unsaved draft\n".into()), 0)
        .unwrap();
    activate(&mut model, &b);
    // The unsaved dialog's Don't Save ends in `close_document`.
    model.close_document(&a);
    activate(&mut model, &a);
    let snapshot = model.document_snapshot.clone().unwrap();
    assert_eq!(snapshot.text, "a on disk\n", "the discarded draft came back");
    assert_eq!(snapshot.save_state, DocumentSaveState::Clean);
    assert_eq!(std::fs::read_to_string(&a).unwrap(), "a on disk\n");
}

/// R10: selecting B then C keeps C even when B's read finishes last, and
/// B's unused session is released.
#[test]
fn a_superseded_activation_does_not_take_focus() {
    let (mut model, _dir, [_, b, c]) = project("superseded");
    let from_b = request(&mut model, &b);
    let from_c = request(&mut model, &c);
    assert!(deliver(&mut model, from_c));
    assert!(!deliver(&mut model, from_b), "B's late result was applied");
    assert_eq!(model.active_document_url.as_ref(), Some(&c));
    assert_eq!(model.open_documents, vec![c.clone()]);
    assert_eq!(model.registry.count(), 1, "B's stale session stayed registered");
}

/// R10: a result that lands after the project closed must not reopen it.
#[test]
fn an_activation_after_close_keeps_the_workspace_closed() {
    let (mut model, _dir, [_, b, _]) = project("after-close");
    let from_b = request(&mut model, &b);
    model.close();
    assert!(!deliver(&mut model, from_b));
    assert!(model.project_url.is_none() && model.active_document_url.is_none());
    assert!(!matches!(model.phase, WorkspacePhase::Ready));
    assert_eq!(model.registry.count(), 0, "the stale session stayed registered");
}

/// R10: re-selecting a superseded file keeps the session the newer request
/// shares with it registered.
#[test]
fn reselecting_a_superseded_file_keeps_its_session() {
    let (mut model, _dir, [_, b, c]) = project("reselect");
    let first_b = request(&mut model, &b);
    let from_c = request(&mut model, &c);
    let second_b = request(&mut model, &b);
    assert!(!deliver(&mut model, first_b));
    assert!(!deliver(&mut model, from_c));
    assert!(deliver(&mut model, second_b));
    let session = model.registered_sessions.iter().find(|s| s.path().raw_value() == "b.tex").unwrap().clone();
    let root = model.project_url.clone().unwrap();
    let held = model.registry.session(&root, session.path()).unwrap().unwrap();
    assert!(held.same_session(&session), "the registry lost the active tab's session");
    assert_eq!(model.registry.count(), 1);
}
