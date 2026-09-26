//! Live `WorkspaceModel` against the private test sshd — the model-level
//! counterpart of remote-core's live tests: opening a mirror pulls, a
//! save uploads, a pull adopts remote bytes into the clean session, and
//! a remote build returns its PDF. Gated by the same PITEX_TEST_SSH_*
//! environment variables; without them the test is a no-op.
//!
//! The "remote" shares this filesystem (localhost sshd), so remote edits
//! are plain file writes like remote-core's live tests.

#![cfg(unix)]

use document_session_core::DocumentMutation;
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
        Self(dir)
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
    let port = std::env::var("PITEX_TEST_SSH_PORT").ok().and_then(|p| p.parse().ok());
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
            WorkspaceMessage::RemotePullFinished { root, result } => {
                if model.apply_remote_pull(root, result.clone()) {
                    model.refresh_after_agent_activity(false);
                }
            }
            WorkspaceMessage::GitRefreshed(result) => {
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
    assert!(opened.remote.is_some(), "the mirror opened as a remote project");
    model.apply_open(&mut store, opened);
    assert!(model.remote.is_some());
    assert_eq!(
        read(&mirror.root().join("main.tex")),
        "\\documentclass{article}\n\\begin{document}\nhello\n\\end{document}\n"
    );

    // Edit + save → pushed on request, the remote file is updated.
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
    model.save();
    pump(&mut model, &rx, |m| {
        matches!(m, WorkspaceMessage::SaveFinished { .. })
    });
    model.push_remote();
    pump(&mut model, &rx, |m| {
        matches!(m, WorkspaceMessage::RemotePushFinished { .. })
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

/// Live compile over SSH: the debounce takes the 1500 ms remote floor
/// (not the configured 700), the run executes on the device, and its
/// isolated output lands under `.pitex-live/` — never beside the source.
#[test]
fn live_workspace_live_compile() {
    let _serial = SERIAL.lock().unwrap();
    let Some(connection) = live_connection() else {
        eprintln!("skipped: PITEX_TEST_SSH_DESTINATION not set");
        return;
    };
    let data = TempDir::new("data");
    std::env::set_var("XDG_DATA_HOME", data.path());
    std::env::set_var("XDG_CONFIG_HOME", data.join("config"));
    let mut store = SettingsStore::new(Preferences::standard());

    let remote = TempDir::new("remote-live");
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
    store.set_live_compile_delay_milliseconds(700);
    store.settings.build.custom_shell_acknowledged = true;
    // remote-core creates the isolated output directory on the device;
    // a bare `cp` exercises that setup end to end.
    model.build_command_text = "cp {file} {outdir}/main.pdf".to_string();
    // Deterministic clock for the scheduler.
    let now = std::rc::Rc::new(std::cell::Cell::new(0u64));
    let moved = now.clone();
    model.set_live_clock(std::rc::Rc::new(move || moved.get()));

    // A real edit notes through the bridge: pending deadline is
    // now+1500 — the remote floor, not the configured 700.
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
                text: "% live remote\n".to_string(),
            },
            snap.revision,
        )
        .unwrap();
    model.document_snapshot = Some(session.snapshot());
    model.note_source_edit(&store);
    assert_eq!(model.live.pending_deadline(), Some(1_500));
    // 700 in — the remote floor has not elapsed yet.
    now.set(700);
    assert!(model.poll_live(false).is_empty());
    // At 1500 the live run starts on the device.
    now.set(1_500);
    let requests = model.poll_live(false);
    model.dispatch_live_requests(requests, &store, "en");
    assert!(model.active_run.as_ref().is_some_and(|r| r.live.is_some()));
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
    let outcome = outcome.unwrap_or_else(|e| panic!("remote live build failed: {e}"));
    assert_eq!(output_pdf, ".pitex-live/main/main.pdf");
    model.apply_build_finished(build, Ok(outcome), &output_pdf, &store, "en");
    // The artifact lives under .pitex-live on the device and was fetched
    // back into the mirror; the sibling output was never written.
    assert!(remote.join(".pitex-live/main/main.pdf").exists());
    assert!(mirror.root().join(".pitex-live/main/main.pdf").exists());
    assert!(!remote.join("main.pdf").exists());
    assert!(!mirror.root().join("main.pdf").exists());
    model.close();
}

/// The Git pane over SSH — the mirror has no `.git`, so every command the
/// pane issues must run on the device: a refresh reports the device
/// repo's status, a stage+commit runs there (the push inside carries the
/// saved edit first), and a discard pulls the reverted bytes back into
/// the mirror.
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
    model.refresh_git();
    pump(&mut model, &rx, |m| {
        matches!(m, WorkspaceMessage::GitRefreshed(_))
    });
    let status = model.git_status.clone().expect("the device repo is found");
    assert_eq!(status.unstaged.len(), 2);
    assert!(
        status
            .unstaged
            .iter()
            .any(|c| c.path == "main.tex" && c.kind == GitChangeKind::Modified)
    );
    assert!(
        status
            .unstaged
            .iter()
            .any(|c| c.path == "notes.md" && c.kind == GitChangeKind::Untracked)
    );
    // `status.root` is the *device* toplevel, not a mirror path.
    assert_eq!(status.root, dir);

    // Edit + save locally; the stage/commit below must see it on the
    // device (push-before-operation).
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
    model.save();
    pump(&mut model, &rx, |m| {
        matches!(m, WorkspaceMessage::SaveFinished { .. })
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
    pump(&mut model, &rx, |m| {
        matches!(m, WorkspaceMessage::GitOpFinished { .. })
    });
    assert_eq!(model.git_error, None);
    assert!(
        git(&["diff", "--cached", "--name-only"])
            .stdout_text()
            .contains("main.tex"),
        "the file is staged on the device"
    );

    // Commit through the model — the push inside means the device
    // commits the saved bytes, not stale device-side content.
    model.git_commit_message = "remote commit".to_string();
    model.git_commit();
    pump(&mut model, &rx, |m| {
        matches!(m, WorkspaceMessage::GitOpFinished { .. })
    });
    assert_eq!(model.git_error, None);
    assert!(
        git(&["log", "-1", "--format=%s"])
            .stdout_text()
            .contains("remote commit")
    );
    assert!(
        git(&["show", "HEAD:main.tex"])
            .stdout_text()
            .contains("% saved locally"),
        "push-before-operation: the commit contains the saved edit"
    );

    // A device-side edit the model discards — pull-after-operation
    // brings the reverted bytes back into the mirror.
    write(&remote.join("main.tex"), "device touched\n");
    model.refresh_git();
    pump(&mut model, &rx, |m| {
        matches!(m, WorkspaceMessage::GitRefreshed(_))
    });
    model.git_discard(&GitChange {
        path: "main.tex".to_string(),
        original_path: None,
        kind: GitChangeKind::Modified,
        staged: false,
    });
    pump(&mut model, &rx, |m| {
        matches!(m, WorkspaceMessage::GitOpFinished { .. })
    });
    assert_eq!(model.git_error, None);
    assert_eq!(read(&remote.join("main.tex")), "v2\n% saved locally\n");
    assert_eq!(
        read(&mirror_file),
        "v2\n% saved locally\n",
        "pull-after-operation: the mirror picked up the device rewrite"
    );
    model.close();
}
