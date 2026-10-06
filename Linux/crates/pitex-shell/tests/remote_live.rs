//! Live `WorkspaceModel` against the private test sshd — the model-level
//! counterpart of remote-core's live tests: opening a mirror pulls, a
//! explicit remote save uploads, local previews never upload, a pull
//! adopts remote bytes into the clean session, and a remote build uses
//! the last explicit save. Gated by the same PITEX_TEST_SSH_*
//! environment variables; without them the test is a no-op.
//!
//! The "remote" shares this filesystem (localhost sshd), so remote edits
//! are plain file writes like remote-core's live tests.

#![cfg(unix)]

use document_session_core::{DocumentMutation, DocumentSaveState};
use git_core::{GitChange, GitChangeKind};
use pitex_shell::model::{ConsoleSection, SaveResult, WorkspaceMessage, WorkspaceModel};
use pitex_shell::settings::{Preferences, SettingsStore};
use remote_core::{RemoteMirror, RemoteProject, RemoteSync, SshClient, SshConnection};
use std::path::{Path, PathBuf};
use std::sync::mpsc::Receiver;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Every test in this binary sets process-global `XDG_*` paths — run
/// them serially so a neighbour can't swap the data store mid-write.
static SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());

struct TempDir(PathBuf);

impl TempDir {
    fn new(tag: &str) -> Self {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "pitex-shell-live-{tag}-{}-{nanos}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        Self(std::fs::canonicalize(dir).unwrap())
    }

    fn path(&self) -> &Path {
        &self.0
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

fn write(path: &Path, text: &str) {
    std::fs::write(path, text).unwrap();
}

fn read(path: &Path) -> String {
    String::from_utf8(std::fs::read(path).unwrap()).unwrap()
}

/// Configure with PITEX_TEST_SSH_DESTINATION (+ _PORT, _KEY, _KNOWN_HOSTS).
/// The key rides in `identityFile`, so the engine's client authenticates
/// like the live remote-core tests.
fn live_connection() -> Option<SshConnection> {
    let destination = std::env::var("PITEX_TEST_SSH_DESTINATION").ok()?;
    let key = std::env::var("PITEX_TEST_SSH_KEY").ok();
    let port = std::env::var("PITEX_TEST_SSH_PORT")
        .ok()
        .and_then(|p| p.parse().ok());
    Some(SshConnection::new("test", destination, None, port, key))
}

/// The client the test uses to drive the device directly — `run_git`
/// for repo setup/verification and the engine `install_engine`
/// pre-registers for the model's sync.
fn live_client(connection: &SshConnection) -> SshClient {
    let mut client = SshClient::new(connection.clone());
    if let Ok(known) = std::env::var("PITEX_TEST_SSH_KNOWN_HOSTS") {
        client
            .extra_arguments
            .extend(["-o".to_string(), format!("UserKnownHostsFile={known}")]);
    }
    client
}

/// The engine `RemoteWorkspace::new` → `RemoteSync::shared` would build
/// gets no test-only arguments, so the test pre-registers one whose
/// client points host-key verification at the private sshd's file.
fn install_engine(mirror: &RemoteMirror, connection: &SshConnection) {
    RemoteSync::install(
        mirror.clone(),
        live_client(connection),
        Some(Arc::new(pitex_shell::remote::SharedGate)),
    );
}

/// Drives the channel the way the GTK dispatch does — results land back
/// on the model — until `want` matches; the message is returned. Anything
/// unexpected for this test (a prep failure, a failed save) panics here
/// rather than timing out opaquely.
fn pump(
    model: &mut WorkspaceModel,
    rx: &Receiver<WorkspaceMessage>,
    want: impl Fn(&WorkspaceMessage) -> bool,
) -> WorkspaceMessage {
    let deadline = Instant::now() + Duration::from_secs(120);
    loop {
        let message = rx
            .recv_timeout(deadline.saturating_duration_since(Instant::now()))
            .expect("a workspace message arrived before the deadline");
        match &message {
            WorkspaceMessage::SaveFinished { path, result } => {
                assert!(
                    matches!(result, SaveResult::Saved | SaveResult::Skipped),
                    "save failed: {result:?}"
                );
                model.apply_save_finished(path, result.clone());
            }
            WorkspaceMessage::RemotePushFinished { root, result } => {
                model.apply_remote_push(root, result.clone());
            }
            WorkspaceMessage::RemoteSaveFinished { root, result } => {
                model.apply_remote_push(root, result.clone());
            }
            WorkspaceMessage::RemotePullFinished { root, result } => {
                if model.apply_remote_pull(root, result.clone()) {
                    model.refresh_after_agent_activity(false);
                }
            }
            WorkspaceMessage::GitRefreshed { result, .. } => {
                model.apply_git_refreshed(result);
            }
            WorkspaceMessage::GitOpFinished { error, .. } => {
                model.git_busy = false;
                model.git_error = error.clone();
            }
            WorkspaceMessage::RemoteBuildPrepFailed { problem, .. } => {
                panic!("remote build preparation failed: {problem}");
            }
            WorkspaceMessage::BuildEvent { build, event } => {
                model.apply_build_event(build, event.clone())
            }
            _ => {}
        }
        if want(&message) {
            return message;
        }
    }
}

/// Worktree-changing operations publish one download before completion;
/// index-only operations complete without downloading the mirror.
fn pump_git_operation(model: &mut WorkspaceModel, rx: &Receiver<WorkspaceMessage>, expected_pulls: usize) {
    let pulls = std::cell::Cell::new(0);
    pump(model, rx, |message| {
        if let WorkspaceMessage::RemotePullFinished { result, .. } = message {
            assert!(result.is_ok(), "post-Git mirror download failed: {result:?}");
            pulls.set(pulls.get() + 1);
        }
        matches!(message, WorkspaceMessage::GitOpFinished { .. })
    });
    assert_eq!(model.git_error, None);
    assert_eq!(pulls.get(), expected_pulls, "unexpected post-Git mirror downloads");
}

fn refresh_git(model: &mut WorkspaceModel, rx: &Receiver<WorkspaceMessage>) {
    model.refresh_git();
    pump(model, rx, |message| matches!(message, WorkspaceMessage::GitRefreshed { .. }));
}

#[test]
fn live_workspace_round_trip() {
    let _serial = SERIAL.lock().unwrap();
    let Some(connection) = live_connection() else {
        eprintln!("skipped: PITEX_TEST_SSH_DESTINATION not set");
        return;
    };
    // Isolate the mirror store (`XDG_DATA_HOME/pitex/Remote`) and prefs.
    let data = TempDir::new("data");
    std::env::set_var("XDG_DATA_HOME", data.path());
    std::env::set_var("XDG_CONFIG_HOME", data.join("config"));
    let mut store = SettingsStore::new(Preferences::standard());

    let remote = TempDir::new("remote");
    write(
        &remote.join("main.tex"),
        "\\documentclass{article}\n\\begin{document}\nhello\n\\end{document}\n",
    );
    let mirror = RemoteMirror::prepare(
        RemoteProject::new(connection.clone(), remote.path().to_str().unwrap()),
        &RemoteMirror::default_store(),
    )
    .unwrap();
    install_engine(&mirror, &connection);

    // Open → the initial pull lands before the project is read.
    let mut model = WorkspaceModel::new();
    let (tx, rx) = std::sync::mpsc::channel();
    model.set_event_sink(tx.clone());
    model.open(mirror.root(), tx);
    let message = pump(&mut model, &rx, |m| {
        matches!(m, WorkspaceMessage::OpenFinished(_))
    });
    let WorkspaceMessage::OpenFinished(result) = message else {
        unreachable!()
    };
    let opened = result.unwrap_or_else(|failure| match failure {
        pitex_shell::model::OpenFailure::Error(error) => panic!("open failed: {error}"),
        pitex_shell::model::OpenFailure::RemoteOpen { device, error } => {
            panic!("remote open failed on {device}: {error}")
        }
    });
    assert!(
        opened.remote.is_some(),
        "the mirror opened as a remote project"
    );
    model.apply_open(&mut store, opened);
    assert!(model.remote.is_some());
    assert_eq!(
        read(&mirror.root().join("main.tex")),
        "\\documentclass{article}\n\\begin{document}\nhello\n\\end{document}\n"
    );

    // Explicit remote save persists the edit and uploads it.
    let session = model
        .registered_sessions
        .iter()
        .find(|s| s.path().raw_value() == "main.tex")
        .expect("main.tex session")
        .clone();
    let snap = session.snapshot();
    session
        .apply(
            DocumentMutation::ReplaceRange {
                utf16_offset: snap.text.encode_utf16().count(),
                utf16_length: 0,
                text: "% local edit\n".to_string(),
            },
            snap.revision,
        )
        .unwrap();
    model.document_snapshot = Some(session.snapshot());
    assert!(model.can_save(), "the edited document is saveable");
    model
        .save_remote_sources()
        .expect("explicit remote save starts");
    pump(&mut model, &rx, |m| {
        matches!(m, WorkspaceMessage::RemoteSaveFinished { .. })
    });
    assert!(model.remote.as_ref().unwrap().conflicts.is_empty());
    assert_eq!(
        read(&remote.join("main.tex")),
        "\\documentclass{article}\n\\begin{document}\nhello\n\\end{document}\n% local edit\n"
    );

    // A remote edit + pull → the clean session reloads the new bytes.
    write(
        &remote.join("main.tex"),
        "\\documentclass{article}\n\\begin{document}\nremote edit\n\\end{document}\n",
    );
    model.pull_remote();
    pump(&mut model, &rx, |m| {
        matches!(m, WorkspaceMessage::RemotePullFinished { .. })
    });
    assert_eq!(
        session.snapshot().text,
        "\\documentclass{article}\n\\begin{document}\nremote edit\n\\end{document}\n"
    );

    // A remote build runs the custom command on the device and the
    // required PDF comes back into the mirror.
    store.settings.build.custom_shell_acknowledged = true;
    model.build_command_text = "cp main.tex main.pdf".to_string();
    model.start_build(&store, "en");
    let message = pump(&mut model, &rx, |m| {
        matches!(m, WorkspaceMessage::BuildFinished { .. })
    });
    let WorkspaceMessage::BuildFinished {
        build,
        outcome,
        output_pdf,
    } = message
    else {
        unreachable!()
    };
    let outcome = outcome.unwrap_or_else(|e| panic!("remote build failed: {e}"));
    let pdf = model
        .apply_build_finished(build, Ok(outcome), &output_pdf, &store, "en")
        .expect("a successful remote build loads its PDF");
    assert_eq!(pdf, mirror.root().join("main.pdf"));
    assert_eq!(
        read(&mirror.root().join("main.pdf")),
        "\\documentclass{article}\n\\begin{document}\nremote edit\n\\end{document}\n"
    );
    assert!(model.build_log_text.contains("Building on test"));
    model.close();
}

/// SSH previews and local disk writes stay in the mirror. Only an
/// explicit remote save publishes source changes, and a remote build
/// cannot accidentally publish edits made after that save.
#[test]
fn live_workspace_explicit_save_boundary() {
    let _serial = SERIAL.lock().unwrap();
    let Some(connection) = live_connection() else {
        eprintln!("skipped: PITEX_TEST_SSH_DESTINATION not set");
        return;
    };
    let data = TempDir::new("data");
    std::env::set_var("XDG_DATA_HOME", data.path());
    std::env::set_var("XDG_CONFIG_HOME", data.join("config"));
    let mut store = SettingsStore::new(Preferences::standard());

    let remote = TempDir::new("remote-explicit-save");
    let initial = "\\documentclass{article}\n\\begin{document}\nhello\n\\end{document}\n";
    write(&remote.join("main.tex"), initial);
    let mirror = RemoteMirror::prepare(
        RemoteProject::new(connection.clone(), remote.path().to_str().unwrap()),
        &RemoteMirror::default_store(),
    )
    .unwrap();
    install_engine(&mirror, &connection);

    let mut model = WorkspaceModel::new();
    let (tx, rx) = std::sync::mpsc::channel();
    model.set_event_sink(tx.clone());
    model.open(mirror.root(), tx);
    let message = pump(&mut model, &rx, |m| {
        matches!(m, WorkspaceMessage::OpenFinished(_))
    });
    let WorkspaceMessage::OpenFinished(result) = message else {
        unreachable!()
    };
    let opened = result.unwrap_or_else(|failure| match failure {
        pitex_shell::model::OpenFailure::Error(error) => panic!("open failed: {error}"),
        pitex_shell::model::OpenFailure::RemoteOpen { device, error } => {
            panic!("remote open failed on {device}: {error}")
        }
    });
    model.apply_open(&mut store, opened);
    assert!(model.remote.is_some());

    store.set_live_compile_enabled(true);
    // SSH uses the local embedded preview even if compiler preview is
    // the user's normal selection. A build never starts on each edit.
    store.set_live_preview_backend("compiler");
    store.set_live_compile_delay_milliseconds(700);
    store.settings.build.custom_shell_acknowledged = true;
    model.build_command_text = "cp main.tex main.pdf".to_string();
    let now = std::rc::Rc::new(std::cell::Cell::new(0u64));
    let moved = now.clone();
    model.set_live_clock(std::rc::Rc::new(move || moved.get()));
    assert_eq!(
        model.embedded_wanted(&store),
        pitex_shell::embedded_preview::SUPPORTED,
        "SSH forces local embedded preview when the helper is supported"
    );

    let session = model
        .registered_sessions
        .iter()
        .find(|s| s.path().raw_value() == "main.tex")
        .expect("main.tex session")
        .clone();
    let snap = session.snapshot();
    session
        .apply(
            DocumentMutation::ReplaceRange {
                utf16_offset: snap.text.encode_utf16().count(),
                utf16_length: 0,
                text: "% local-only edit\n".to_string(),
            },
            snap.revision,
        )
        .unwrap();
    model.document_snapshot = Some(session.snapshot());
    let requests = model.note_source_edit(&store);
    model.dispatch_live_requests(requests, &store, "en");
    now.set(10_000);
    let requests = model.poll_live(false);
    model.dispatch_live_requests(requests, &store, "en");
    assert!(
        model.active_run.is_none(),
        "an edit never starts an SSH build"
    );
    assert_eq!(model.live.pending_deadline(), None);
    assert_eq!(read(&remote.join("main.tex")), initial);

    // A local save is also not an upload. This is the persistence path
    // used by local-only work such as an Assistant preparation.
    model.save();
    pump(&mut model, &rx, |m| {
        matches!(m, WorkspaceMessage::SaveFinished { .. })
    });
    let saved = format!("{initial}% local-only edit\n");
    assert_eq!(read(&mirror.root().join("main.tex")), saved);
    assert_eq!(read(&remote.join("main.tex")), initial);

    // Assistant/filesystem changes remain local until explicit Save.
    write(&mirror.root().join("notes.md"), "local Assistant change\n");
    model.refresh_after_agent_activity(false);
    assert!(!remote.join("notes.md").exists());
    assert_eq!(read(&remote.join("main.tex")), initial);

    model
        .save_remote_sources()
        .expect("explicit remote save starts");
    // The next draft arrives while the Save upload is still pending.
    let snap = session.snapshot();
    session
        .apply(
            DocumentMutation::ReplaceRange {
                utf16_offset: snap.text.encode_utf16().count(),
                utf16_length: 0,
                text: "% unsaved edit\n".to_string(),
            },
            snap.revision,
        )
        .unwrap();
    model.document_snapshot = Some(session.snapshot());
    let requests = model.note_source_edit(&store);
    model.dispatch_live_requests(requests, &store, "en");
    let message = pump(&mut model, &rx, |m| {
        matches!(m, WorkspaceMessage::RemoteSaveFinished { .. })
    });
    let WorkspaceMessage::RemoteSaveFinished { result, .. } = message else {
        unreachable!()
    };
    assert!(result.expect("explicit save uploads").is_empty());
    assert_eq!(read(&remote.join("main.tex")), saved);
    assert_eq!(read(&remote.join("notes.md")), "local Assistant change\n");
    assert_eq!(session.snapshot().save_state, DocumentSaveState::Dirty);

    // A build cannot upload newer mirror bytes or persist the later draft.
    write(&mirror.root().join("notes.md"), "new local change\n");
    model.refresh_after_agent_activity(false);
    model.start_build(&store, "en");
    assert!(
        model.save_remote_sources().is_err(),
        "Save cannot race an active build"
    );
    let message = pump(&mut model, &rx, |m| {
        matches!(m, WorkspaceMessage::BuildFinished { .. })
    });
    let WorkspaceMessage::BuildFinished {
        build,
        outcome,
        output_pdf,
    } = message
    else {
        unreachable!()
    };
    let outcome = outcome.unwrap_or_else(|e| panic!("remote build failed: {e}"));
    model
        .apply_build_finished(build, Ok(outcome), &output_pdf, &store, "en")
        .expect("the saved remote PDF is fetched into the mirror");
    assert_eq!(read(&remote.join("main.tex")), saved);
    assert_eq!(read(&remote.join("main.pdf")), saved);
    assert_eq!(read(&mirror.root().join("main.tex")), saved);
    assert_eq!(read(&remote.join("notes.md")), "local Assistant change\n");
    assert_eq!(read(&mirror.root().join("notes.md")), "new local change\n");
    assert_eq!(session.snapshot().save_state, DocumentSaveState::Dirty);
    assert!(session.snapshot().text.ends_with("% unsaved edit\n"));

    // A conflicting local write prevents explicit Save/close from
    // silently discarding the editor text or tearing down the project.
    let unsaved = session.snapshot().text;
    write(&mirror.root().join("main.tex"), "outside mirror rewrite\n");
    assert!(model.save_remote_sources().is_err());
    assert_eq!(session.snapshot().text, unsaved);
    assert_ne!(session.snapshot().save_state, DocumentSaveState::Clean);
    assert_eq!(model.project_url.as_deref(), Some(mirror.root().as_path()));
    assert!(model.remote.is_some());
    assert_eq!(
        read(&mirror.root().join("main.tex")),
        "outside mirror rewrite\n"
    );
    assert_eq!(read(&remote.join("main.tex")), saved);
}

/// The Git pane over SSH runs commands on the device. An explicit save
/// publishes the edit before stage/commit; discard pulls reverted bytes
/// back into the mirror.
#[test]
fn live_workspace_git() {
    let _serial = SERIAL.lock().unwrap();
    let Some(connection) = live_connection() else {
        eprintln!("skipped: PITEX_TEST_SSH_DESTINATION not set");
        return;
    };
    let client = live_client(&connection);
    let data = TempDir::new("data");
    std::env::set_var("XDG_DATA_HOME", data.path());
    std::env::set_var("XDG_CONFIG_HOME", data.join("config"));
    let mut store = SettingsStore::new(Preferences::standard());

    // A repo on the device: one commit, a modified tracked file, an
    // untracked file.
    let remote = TempDir::new("remote-git");
    let dir = remote.path().to_str().unwrap().to_string();
    let git = |args: &[&str]| {
        let argv: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        let result = client.run_git(&dir, &argv).unwrap();
        assert_eq!(
            result.status,
            0,
            "device git {} failed: {}",
            args.join(" "),
            result.stderr_text()
        );
        result
    };
    git(&["init", "-q"]);
    git(&["config", "user.email", "test@example.com"]);
    git(&["config", "user.name", "Pitex Test"]);
    write(&remote.join("main.tex"), "v1\n");
    git(&["add", "main.tex"]);
    git(&["commit", "-q", "-m", "first"]);
    write(&remote.join("main.tex"), "v2\n");
    write(&remote.join("notes.md"), "new\n");

    let mirror = RemoteMirror::prepare(
        RemoteProject::new(connection.clone(), dir.clone()),
        &RemoteMirror::default_store(),
    )
    .unwrap();
    install_engine(&mirror, &connection);

    let mut model = WorkspaceModel::new();
    let (tx, rx) = std::sync::mpsc::channel();
    model.set_event_sink(tx.clone());
    model.open(mirror.root(), tx);
    let message = pump(&mut model, &rx, |m| {
        matches!(m, WorkspaceMessage::OpenFinished(_))
    });
    let WorkspaceMessage::OpenFinished(result) = message else {
        unreachable!()
    };
    let opened = result.unwrap_or_else(|failure| match failure {
        pitex_shell::model::OpenFailure::Error(error) => panic!("open failed: {error}"),
        pitex_shell::model::OpenFailure::RemoteOpen { device, error } => {
            panic!("remote open failed on {device}: {error}")
        }
    });
    assert!(opened.remote.is_some());
    model.apply_open(&mut store, opened);
    assert!(model.remote.is_some());

    // Refresh — the pane sees the device repo even though the mirror
    // holds no `.git`.
    model.console_section = ConsoleSection::Git;
    model.bottom_panel_visible = true;
    refresh_git(&mut model, &rx);
    let status = model.git_status.clone().expect("the device repo is found");
    assert_eq!(status.unstaged.len(), 2);
    assert!(status
        .unstaged
        .iter()
        .any(|c| c.path == "main.tex" && c.kind == GitChangeKind::Modified));
    assert!(status
        .unstaged
        .iter()
        .any(|c| c.path == "notes.md" && c.kind == GitChangeKind::Untracked));
    // `status.root` is the *device* toplevel, not a mirror path.
    assert_eq!(status.root, dir);

    // Explicit remote save publishes the edit before Git sees it.
    let session = model
        .registered_sessions
        .iter()
        .find(|s| s.path().raw_value() == "main.tex")
        .expect("main.tex session")
        .clone();
    let snap = session.snapshot();
    session
        .apply(
            DocumentMutation::ReplaceRange {
                utf16_offset: snap.text.encode_utf16().count(),
                utf16_length: 0,
                text: "% saved locally\n".to_string(),
            },
            snap.revision,
        )
        .unwrap();
    model.document_snapshot = Some(session.snapshot());
    model
        .save_remote_sources()
        .expect("explicit remote save starts");
    pump(&mut model, &rx, |m| {
        matches!(m, WorkspaceMessage::RemoteSaveFinished { .. })
    });
    let mirror_file = mirror.root().join("main.tex");
    assert!(read(&mirror_file).contains("% saved locally"));

    // Stage through the model — runs `git add` on the device.
    model.git_stage(&GitChange {
        path: "main.tex".to_string(),
        original_path: None,
        kind: GitChangeKind::Modified,
        staged: false,
    });
    pump_git_operation(&mut model, &rx, 0);
    assert!(
        git(&["diff", "--cached", "--name-only"])
            .stdout_text()
            .contains("main.tex"),
        "the file is staged on the device"
    );
    refresh_git(&mut model, &rx);

    // Unstage changes only the device index and also skips a download.
    let staged = model.git_status.as_ref().unwrap().staged.iter()
        .find(|change| change.path == "main.tex").unwrap().clone();
    model.git_unstage(&staged);
    pump_git_operation(&mut model, &rx, 0);
    assert!(git(&["diff", "--cached", "--name-only"]).stdout_text().trim().is_empty());
    refresh_git(&mut model, &rx);
    let unstaged = model.git_status.as_ref().unwrap().unstaged.iter()
        .find(|change| change.path == "main.tex").unwrap().clone();
    model.git_stage(&unstaged);
    pump_git_operation(&mut model, &rx, 0);
    refresh_git(&mut model, &rx);

    // Commit through the model uses the explicitly saved device bytes.
    model.git_commit_message = "remote commit".to_string();
    model.git_commit();
    // Commit hooks may rewrite source files, so commits retain a download.
    pump_git_operation(&mut model, &rx, 1);
    assert!(git(&["log", "-1", "--format=%s"])
        .stdout_text()
        .contains("remote commit"));
    assert!(
        git(&["show", "HEAD:main.tex"])
            .stdout_text()
            .contains("% saved locally"),
        "the commit contains the explicitly saved edit"
    );

    // A device-side edit the model discards — pull-after-operation
    // brings the reverted bytes back into the mirror.
    write(&remote.join("main.tex"), "device touched\n");
    refresh_git(&mut model, &rx);
    model.git_discard(&GitChange {
        path: "main.tex".to_string(),
        original_path: None,
        kind: GitChangeKind::Modified,
        staged: false,
    });
    pump_git_operation(&mut model, &rx, 1);
    assert_eq!(read(&remote.join("main.tex")), "v2\n% saved locally\n");
    assert_eq!(
        read(&mirror_file),
        "v2\n% saved locally\n",
        "pull-after-operation: the mirror picked up the device rewrite"
    );
    model.close();
}
