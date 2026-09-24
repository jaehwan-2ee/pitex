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
use pitex_shell::model::{SaveResult, WorkspaceMessage, WorkspaceModel};
use pitex_shell::settings::{Preferences, SettingsStore};
use remote_core::{RemoteMirror, RemoteProject, RemoteSync, SshClient, SshConnection};
use std::path::{Path, PathBuf};
use std::sync::mpsc::Receiver;
use std::sync::Arc;
use std::time::{Duration, Instant};

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

/// The engine `RemoteWorkspace::new` → `RemoteSync::shared` would build
/// gets no test-only arguments, so the test pre-registers one whose
/// client points host-key verification at the private sshd's file.
fn install_engine(mirror: &RemoteMirror, connection: &SshConnection) {
    let mut client = SshClient::new(connection.clone());
    if let Ok(known) = std::env::var("PITEX_TEST_SSH_KNOWN_HOSTS") {
        client
            .extra_arguments
            .extend(["-o".to_string(), format!("UserKnownHostsFile={known}")]);
    }
    RemoteSync::install(
        mirror.clone(),
        client,
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
            WorkspaceMessage::RemoteBuildPrepFailed(problem) => {
                panic!("remote build preparation failed: {problem}");
            }
            WorkspaceMessage::BuildEvent(event) => model.apply_build_event(event.clone()),
            _ => {}
        }
        if want(&message) {
            return message;
        }
    }
}

#[test]
fn live_workspace_round_trip() {
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
    let WorkspaceMessage::BuildFinished { outcome, output_pdf } = message else {
        unreachable!()
    };
    let outcome = outcome.unwrap_or_else(|e| panic!("remote build failed: {e}"));
    let pdf = model
        .apply_build_finished(Ok(outcome), &output_pdf, false, false)
        .expect("a successful remote build loads its PDF");
    assert_eq!(pdf, mirror.root().join("main.pdf"));
    assert_eq!(
        read(&mirror.root().join("main.pdf")),
        "\\documentclass{article}\n\\begin{document}\nremote edit\n\\end{document}\n"
    );
    assert!(model.build_log_text.contains("Building on test"));
    model.close();
}
