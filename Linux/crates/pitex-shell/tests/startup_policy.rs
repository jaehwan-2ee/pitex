//! Saved workspace data remains available without reopening on launch.

use pitex_shell::model::{WorkspaceMessage, WorkspaceModel, WorkspacePhase};
use pitex_shell::settings::{Preferences, SettingsStore};
use settings_feature::PersistedSettings;
use std::path::{Path, PathBuf};
use std::time::Duration;

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "pitex-startup-policy-{}-{stamp}",
            std::process::id()
        ));
        std::fs::create_dir_all(root.join("saved-project")).unwrap();
        std::fs::create_dir_all(root.join("explicit-project")).unwrap();
        for folder in ["saved-project", "explicit-project"] {
            std::fs::write(
                root.join(folder).join("main.tex"),
                "\\documentclass{article}\n\\begin{document}\nHello.\n\\end{document}\n",
            )
            .unwrap();
        }
        Self(root.canonicalize().unwrap())
    }

    fn store(&self) -> SettingsStore {
        let mut prefs = Preferences::default();
        let mut settings = PersistedSettings::safe_defaults();
        settings.project.restores_last_project = true;
        prefs.set(
            "dev.pitex.settings",
            serde_json::to_value(settings).unwrap(),
        );
        prefs.set("pitex.pref.editor.restoreSession", true);
        prefs.set(
            "pitex.pref.workspace.recentDocuments",
            serde_json::json!([self.0.join("saved-project/main.tex").to_string_lossy()]),
        );
        prefs.set(
            &format!("commands.{}", self.0.join("saved-project").display()),
            serde_json::json!({ "build": "saved-command", "custom": "saved-custom" }),
        );
        SettingsStore::new(prefs)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn launch(store: &SettingsStore) -> WorkspaceModel {
    let mut workspace = WorkspaceModel::new();
    workspace.load_recents(store);
    workspace
}

fn assert_open_screen(workspace: &WorkspaceModel) {
    assert!(matches!(workspace.phase, WorkspacePhase::NoProject));
    assert!(!workspace.has_project());
    assert!(workspace.active_document_url.is_none());
    assert!(workspace.document_snapshot.is_none());
    assert!(workspace.open_documents.is_empty());
    assert!(workspace.registered_sessions.is_empty());
    assert!(workspace.remote.is_none());
}

fn open(workspace: &mut WorkspaceModel, store: &mut SettingsStore, path: PathBuf) {
    let (tx, rx) = std::sync::mpsc::channel();
    workspace.open(path, tx);
    match rx.recv_timeout(Duration::from_secs(10)).unwrap() {
        WorkspaceMessage::OpenFinished(Ok(opened)) => workspace.apply_open(store, opened),
        message => panic!("unexpected open result: {message:?}"),
    }
}

fn assert_same_file(actual: &Path, expected: &Path) {
    // Windows may expose the same file with or without the verbatim-path
    // prefix. Compare resolved files while retaining the fixture's stored
    // path spelling for recent entries and project command keys.
    assert_eq!(
        actual.canonicalize().unwrap(),
        expected.canonicalize().unwrap(),
        "opened a different file: {} instead of {}",
        actual.display(),
        expected.display()
    );
}

#[test]
fn saved_restore_preferences_and_recents_do_not_open_a_workspace() {
    let fixture = Fixture::new();
    let store = fixture.store();
    assert!(store.settings.project.restores_last_project);
    assert!(store.restore_session());
    let workspace = launch(&store);
    assert_open_screen(&workspace);
    assert_eq!(
        workspace.recent_documents,
        [fixture.0.join("saved-project/main.tex")]
    );
    let key = format!("commands.{}", fixture.0.join("saved-project").display());
    assert_eq!(
        store.prefs().dictionary(&key).unwrap()["build"],
        "saved-command"
    );
}

#[test]
fn explicit_file_and_recent_opens_work_and_next_launch_stays_empty() {
    let fixture = Fixture::new();
    let mut store = fixture.store();
    let mut workspace = launch(&store);
    let selected = fixture.0.join("explicit-project/main.tex");
    open(&mut workspace, &mut store, selected.clone());
    assert!(matches!(workspace.phase, WorkspacePhase::Ready));
    assert_same_file(workspace.active_document_url.as_ref().unwrap(), &selected);

    // Open Recent uses the same explicit open path. Relaunching does not
    // consume or erase saved workspace preferences or project commands.
    let recent = fixture.0.join("saved-project/main.tex");
    open(&mut workspace, &mut store, recent.clone());
    assert_same_file(workspace.active_document_url.as_ref().unwrap(), &recent);
    let key = format!("commands.{}", fixture.0.join("saved-project").display());
    assert_eq!(
        store.prefs().dictionary(&key).unwrap()["build"],
        "saved-command"
    );
    workspace.close();
    drop(workspace);
    let reopened = launch(&store);
    assert_open_screen(&reopened);
    assert_same_file(reopened.recent_documents.first().unwrap(), &recent);
    assert!(store.settings.project.restores_last_project);
    assert!(store.restore_session());
}
