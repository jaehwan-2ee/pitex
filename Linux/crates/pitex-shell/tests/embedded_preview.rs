//! Embedded editing preview — model-level behavior against a stub helper
//! that speaks the real `pitex-preview` stdio protocol (no TeX needed):
//! the final-build floor is an *edit revision*, so updates sent after a
//! manual build started (a queued pre-build edit flushed late, the
//! save-only update caused by the build's pre-save, a rescan) never put a
//! preview over the final PDF, while a real later edit — also one made
//! while the build runs — resumes drafting.

#![cfg(all(unix, feature = "embedded-preview"))]

use document_session_core::{DocumentMutation, DocumentSaveState};
use pitex_shell::model::{WorkspaceMessage, WorkspaceModel};
use pitex_shell::settings::{Preferences, SettingsStore};
use std::path::{Path, PathBuf};
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

struct TempDir(PathBuf);

impl TempDir {
    fn new(tag: &str) -> Self {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "pitex-shell-embedded-{tag}-{}-{nanos}",
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

/// Minimal protocol peer: every update is published immediately as a
/// complete one-page artifact of its generation; releases delete it. With
/// `PITEX_STUB_GATE=<file>`, a publication waits until that file exists.
const STUB: &str = r#"#!/bin/sh
while [ $# -gt 0 ]; do case "$1" in --out) OUT="$2"; shift 2;; *) shift;; esac; done
seq=0
printf '{"event":"ready"}\n'
while IFS= read -r line; do
  case "$line" in
    *'"op":"update"'*)
      gen=$(printf '%s' "$line" | sed -n 's/.*"generation":\([0-9][0-9]*\).*/\1/p')
      while [ -n "$PITEX_STUB_GATE" ] && [ ! -e "$PITEX_STUB_GATE" ]; do sleep 0.02; done
      seq=$((seq+1))
      mkdir -p "$OUT/p$seq"
      printf '%%PDF-1.4\n%% stub generation %s\n' "$gen" > "$OUT/p$seq/main.pdf"
      : > "$OUT/p$seq/main.synctex"
      printf '{"event":"published","seq":%s,"generation":%s,"complete":true,"coherent":true,"pages":1,"current_pages":1,"errors":0,"dir":"%s/p%s","pdf":"%s/p%s/main.pdf","synctex":"%s/p%s/main.synctex"}\n' "$seq" "$gen" "$OUT" "$seq" "$OUT" "$seq" "$OUT" "$seq" ;;
    *'"op":"release"'*)
      s=$(printf '%s' "$line" | sed -n 's/.*"seq":\([0-9][0-9]*\).*/\1/p')
      rm -rf "$OUT/p$s" ;;
    *'"op":"quit"'*) exit 0 ;;
  esac
done
"#;

struct Fixture {
    _dir: TempDir,
    model: WorkspaceModel,
    rx: Receiver<WorkspaceMessage>,
    store: SettingsStore,
}

impl Fixture {
    fn open(tag: &str, command: &str) -> Self {
        let dir = TempDir::new(tag);
        std::fs::write(
            dir.join("main.tex"),
            "\\documentclass{article}\n\\begin{document}\nhello\n\\end{document}\n",
        )
        .unwrap();
        let mut store = SettingsStore::new(Preferences::standard());
        store.set_live_compile_enabled(true);
        store.set_live_preview_backend("embedded");
        store.settings.build.custom_shell_acknowledged = true;
        let mut model = WorkspaceModel::new();
        let (tx, rx) = std::sync::mpsc::channel();
        model.set_event_sink(tx.clone());
        model.open(dir.0.clone(), tx);
        let mut fx = Self { _dir: dir, model, rx, store };
        let WorkspaceMessage::OpenFinished(result) =
            fx.pump(|m| matches!(m, WorkspaceMessage::OpenFinished(_)))
        else {
            unreachable!()
        };
        let opened = result.unwrap_or_else(|_| panic!("open failed"));
        fx.model.apply_open(&mut fx.store, opened);
        fx.model.build_command_text = command.to_string();
        fx
    }

    /// Applies every message in arrival order (like the GTK dispatch)
    /// until one matching `want` has been applied, and returns it.
    fn pump(&mut self, want: impl Fn(&WorkspaceMessage) -> bool) -> WorkspaceMessage {
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            let message = self
                .rx
                .recv_timeout(deadline.saturating_duration_since(Instant::now()))
                .expect("a workspace message arrived before the deadline");
            let matched = want(&message);
            match &message {
                WorkspaceMessage::SaveFinished { path, result } => {
                    self.model.apply_save_finished(path, result.clone())
                }
                WorkspaceMessage::BuildEvent { build, event } => {
                    self.model.apply_build_event(build, event.clone())
                }
                WorkspaceMessage::BuildFinished { build, outcome, output_pdf } => {
                    self.model.apply_build_finished(
                        build.clone(),
                        outcome.clone(),
                        output_pdf,
                        &self.store,
                        "en",
                    );
                }
                WorkspaceMessage::EmbeddedPreview { session, event } => {
                    self.model.apply_embedded_event(*session, event.clone());
                }
                _ => {}
            }
            if matched {
                return message;
            }
        }
    }

    fn published(&mut self) {
        self.pump(|m| {
            matches!(
                m,
                WorkspaceMessage::EmbeddedPreview {
                    event: pitex_shell::embedded_preview::PreviewEvent::Published(_),
                    ..
                }
            )
        });
    }

    fn build_finished(&mut self) {
        self.pump(|m| matches!(m, WorkspaceMessage::BuildFinished { .. }));
    }

    /// A real text edit through the editor mutation path + live bridge.
    fn edit(&mut self, text: &str) {
        let session = self
            .model
            .registered_sessions
            .iter()
            .find(|s| s.path().raw_value() == "main.tex")
            .cloned()
            .expect("main.tex session");
        let snap = session.snapshot();
        session
            .apply(
                DocumentMutation::ReplaceRange {
                    utf16_offset: snap.text.encode_utf16().count(),
                    utf16_length: 0,
                    text: text.to_string(),
                },
                snap.revision,
            )
            .unwrap();
        self.model.document_snapshot = Some(session.snapshot());
        let requests = self.model.note_source_edit(&self.store);
        self.model.dispatch_live_requests(requests, &self.store, "en");
    }

    /// The coalescing timer fires.
    fn flush(&mut self) {
        self.model.flush_embedded_preview(&self.store);
    }

    /// A BOUNDED negative wait: applies every message that arrives for `duration` (like `pump`) and fails if a publication shows up.
    fn quiet(&mut self, duration: Duration) {
        let deadline = Instant::now() + duration;
        while let Ok(message) = self.rx.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
            assert!(
                !matches!(
                    &message,
                    WorkspaceMessage::EmbeddedPreview {
                        event: pitex_shell::embedded_preview::PreviewEvent::Published(_),
                        ..
                    }
                ),
                "no publication was expected"
            );
            match &message {
                WorkspaceMessage::SaveFinished { path, result } => {
                    self.model.apply_save_finished(path, result.clone())
                }
                WorkspaceMessage::EmbeddedPreview { session, event } => {
                    self.model.apply_embedded_event(*session, event.clone());
                }
                _ => {}
            }
        }
    }

    fn showing_final(&self) -> bool {
        !self.model.displaying_editing_preview()
            && self.model.latest_built_pdf_name.as_deref() == Some("main.pdf")
    }
}

fn install_stub(dir: &Path) -> PathBuf {
    let path = dir.join("pitex-preview");
    std::fs::write(&path, STUB).unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    path
}

/// Queued pre-build edit, then the pre-save/save-only and rescan updates
/// arriving after the final build succeeded, then a real edit.
fn late_prebuild_updates_never_cover_final() {
    let mut fx = Fixture::open("late", "cp {file} {outdir}/main.pdf");
    fx.model.sync_live_settings(&fx.store);
    fx.flush();
    fx.published();
    assert!(fx.model.displaying_editing_preview(), "initial preview is shown");

    fx.edit("% typed before Build\n");
    // Build before the coalescing window elapsed.
    fx.model.start_build(&fx.store, "en");
    fx.build_finished();
    assert!(fx.showing_final(), "final PDF replaces the preview");

    // The queued edit's timer now fires; its buffers were saved by the
    // build's pre-save, so this is a save-only update with a newer
    // transport generation.
    fx.flush();
    fx.published();
    assert!(fx.showing_final(), "late pre-build update must not cover the final PDF");
    // A save/disk refresh that changes nothing the helper could see (unforced, equal key) sends nothing: no new generation, no publication (a
    // bounded wait, not the 30 s pump), the final PDF stays and the header stays Current.
    let sent = fx.model.embedded.generation;
    fx.model.request_embedded_refresh(&fx.store);
    fx.flush();
    fx.quiet(Duration::from_millis(400));
    assert_eq!(fx.model.embedded.generation, sent, "an equal-key refresh sends no update");
    assert!(fx.showing_final(), "an equal-key refresh must not cover the final PDF");
    assert_eq!(
        fx.model.embedded.status,
        pitex_shell::embedded_preview::PreviewStatus::Current,
        "a skipped refresh leaves the header Current"
    );
    // A FORCED refresh (agent completion, remote pull, a disk event on a file that is not an open source) sends yet another generation.
    fx.model.request_embedded_refresh_forced(&fx.store);
    fx.flush();
    fx.published();
    assert_eq!(fx.model.embedded.generation, sent + 1, "a forced refresh sends a new generation");
    assert!(fx.showing_final(), "rescan update must not cover the final PDF");
    assert_eq!(
        fx.model.embedded.status,
        pitex_shell::embedded_preview::PreviewStatus::Current,
        "the header must not keep saying 'updating' over the current final PDF"
    );

    // A real edit after the build resumes drafting.
    fx.edit("% typed after Build\n");
    fx.flush();
    fx.published();
    assert!(fx.model.displaying_editing_preview(), "a later edit shows the editing preview again");
}

/// Pre-build preview arriving while the build runs is displaced by the
/// final PDF; an edit typed while the build runs keeps drafting after it.
fn previews_during_running_build() {
    let mut fx = Fixture::open("running", "sleep 1 && cp {file} {outdir}/main.pdf");
    fx.model.sync_live_settings(&fx.store);
    fx.flush();
    fx.published();

    fx.edit("% before Build\n");
    fx.model.start_build(&fx.store, "en");
    fx.flush(); // queued pre-build edit, published while the build runs
    fx.published();
    fx.build_finished();
    assert!(fx.showing_final(), "a preview of pre-build edits yields to the final PDF");

    fx.model.start_build(&fx.store, "en");
    fx.edit("% typed while building\n");
    fx.flush();
    fx.published();
    fx.build_finished();
    assert!(
        fx.model.displaying_editing_preview(),
        "an edit made during the build keeps the editing preview after it succeeds"
    );
    // The final build's name/bytes stay paired for download and the
    // assistant context while the preview is on screen.
    assert_eq!(fx.model.latest_built_pdf_name.as_deref(), Some("main.pdf"));
    assert!(matches!(
        fx.model.build_state,
        pitex_shell::model::WorkspaceBuildState::Succeeded { .. }
    ));
}

/// The user saves after an update was sent but before its publication
/// arrives: the saved (now clean) buffer still holds exactly the compiled
/// text, so the preview binds for SyncTeX.
fn save_before_publication_keeps_synctex(gate: &Path) {
    std::env::set_var("PITEX_STUB_GATE", gate);
    std::fs::write(gate, "").unwrap();
    let mut fx = Fixture::open("save", "true");
    fx.model.sync_live_settings(&fx.store);
    fx.flush();
    fx.published();

    std::fs::remove_file(gate).unwrap();
    fx.edit("% draft\n");
    fx.flush();
    fx.model.save();
    fx.pump(|m| matches!(m, WorkspaceMessage::SaveFinished { .. }));
    std::fs::write(gate, "").unwrap();
    fx.published();

    let main = fx.model.project_url.clone().unwrap().join("main.tex");
    let session = fx.model.registered_sessions.iter().find(|s| s.path().raw_value() == "main.tex").unwrap();
    let snap = session.snapshot();
    assert_eq!(snap.save_state, DocumentSaveState::Clean, "the save landed first");
    let expected = std::collections::HashMap::from([(main, snap.content_hash.raw_value)]);
    assert_eq!(
        fx.model.embedded.bound_snapshot.as_ref(),
        Some(&expected),
        "SyncTeX binds: the clean buffer is the compiled source"
    );
    std::env::remove_var("PITEX_STUB_GATE");
}

/// What the flush key cannot see is forced: a disk event on an UNOPENED `\input` (the watcher reports any project file) reaches the helper although no
/// hash changed, while an event on an OPEN source whose content did not change sends nothing (its hash is in the key).
fn disk_events_force_only_what_the_key_cannot_see() {
    let mut fx = Fixture::open("disk", "true");
    fx.model.sync_live_settings(&fx.store);
    fx.flush();
    fx.published();
    let root = fx.model.project_url.clone().unwrap();
    let sent = fx.model.embedded.generation;

    let tail = root.join("tail.tex");
    std::fs::write(&tail, "\\section{Tail}\n").unwrap();
    fx.model.request_embedded_refresh_for_disk_change(&tail, &fx.store);
    fx.flush();
    fx.published();
    assert_eq!(fx.model.embedded.generation, sent + 1, "an unopened \\input changed on disk is sent");

    fx.model.request_embedded_refresh_for_disk_change(&root.join("main.tex"), &fx.store);
    fx.flush();
    fx.quiet(Duration::from_millis(400));
    assert_eq!(fx.model.embedded.generation, sent + 1, "a disk event on an unchanged open source sends nothing");
}

/// An external change to the open, clean main.tex is adopted by `process_disk_change`; the refresh requested AFTER the adoption (the app does it in that
/// order) sees the changed hash and sends.
fn adopted_disk_change_reaches_the_helper() {
    let mut fx = Fixture::open("adopt", "true");
    fx.model.sync_live_settings(&fx.store);
    fx.flush();
    fx.published();
    let main = fx.model.project_url.clone().unwrap().join("main.tex");
    let sent = fx.model.embedded.generation;

    std::fs::write(
        &main,
        "\\documentclass{article}\n\\begin{document}\nchanged on disk\n\\end{document}\n",
    )
    .unwrap();
    fx.model.process_disk_change(&main, false);
    fx.model.request_embedded_refresh_for_disk_change(&main, &fx.store);
    fx.flush();
    fx.published();
    assert_eq!(fx.model.embedded.generation, sent + 1, "the adopted change reached the helper");
    let session = fx
        .model
        .registered_sessions
        .iter()
        .find(|s| s.path().raw_value() == "main.tex")
        .expect("main.tex session");
    assert!(session.snapshot().text.contains("changed on disk"), "the clean session adopted the disk text");
}

/// The resolver changed the build target (here: from "none" back to main.tex through the public refresh): the helper session is retired. An EDITING
/// preview on screen comes back without an edit; a FINAL build output on screen stays and nothing is published over it.
fn target_change_restarts_only_an_editing_preview() {
    let mut fx = Fixture::open("retarget", "cp {file} {outdir}/main.pdf");
    fx.model.sync_live_settings(&fx.store);
    fx.flush();
    fx.published();
    assert!(fx.model.displaying_editing_preview(), "initial preview is shown");
    let sent = fx.model.embedded.generation;

    fx.model.automatic_build_target = None;
    fx.model.refresh_build_target();
    assert!(!fx.model.embedded.is_running(), "the target change retired the helper session");
    fx.model.request_embedded_restart(&fx.store);
    assert!(fx.model.embedded_poll_delay(false).is_some(), "the restart is pending, no edit needed");
    fx.flush();
    fx.published();
    assert!(fx.model.displaying_editing_preview(), "the editing preview is back");
    assert_eq!(fx.model.embedded.generation, sent + 1, "the restart sent one update");

    fx.model.start_build(&fx.store, "en");
    fx.build_finished();
    assert!(fx.showing_final(), "the final PDF replaces the preview");
    fx.model.automatic_build_target = None;
    fx.model.refresh_build_target();
    fx.model.request_embedded_restart(&fx.store);
    assert!(fx.model.embedded_poll_delay(false).is_none(), "nothing is pending over a final PDF");
    fx.quiet(Duration::from_millis(400));
    assert!(fx.showing_final(), "the final PDF stays after the target change");
}

/// Pin and unpin switch the build context unconditionally: an editing preview restarts both times, a final PDF stays.
fn pin_toggle_restarts_only_an_editing_preview() {
    let mut fx = Fixture::open("pin", "cp {file} {outdir}/main.pdf");
    fx.model.sync_live_settings(&fx.store);
    fx.flush();
    fx.published();
    assert!(fx.model.displaying_editing_preview(), "initial preview is shown");

    for _ in 0..2 {
        let sent = fx.model.embedded.generation;
        fx.model.toggle_pinned_build_target();
        fx.model.request_embedded_restart(&fx.store);
        assert!(fx.model.embedded_poll_delay(false).is_some(), "the pin change leaves a restart pending");
        fx.flush();
        fx.published();
        assert!(fx.model.displaying_editing_preview(), "the editing preview is back");
        assert_eq!(fx.model.embedded.generation, sent + 1, "one update per toggle");
    }

    fx.model.start_build(&fx.store, "en");
    fx.build_finished();
    assert!(fx.showing_final(), "the final PDF replaces the preview");
    fx.model.toggle_pinned_build_target();
    fx.model.request_embedded_restart(&fx.store);
    assert!(fx.model.embedded_poll_delay(false).is_none(), "nothing is pending over a final PDF");
    fx.quiet(Duration::from_millis(400));
    assert!(fx.showing_final(), "the final PDF stays after a pin change");
}

/// The finding's real path, through the resolver (two steps). main → none: with a second main in the project and an active file that neither
/// includes, `resolve` is `Ambiguous` (`owners.is_empty() && mains.len() > 1`), so the target is none, the retained PDF is dropped and the helper
/// stops. none → main: now NO editing preview is on screen and no helper runs, so `embedded_comes_back` is false and ONLY the carried flag can
/// restart the preview. (A single main would not give none: the resolver falls back to it for an orphan file.)
fn target_through_a_nil_interval_restarts_the_preview() {
    let mut fx = Fixture::open("nilspan", "true");
    fx.model.sync_live_settings(&fx.store);
    fx.flush();
    fx.published();
    assert!(fx.model.displaying_editing_preview(), "initial preview is shown");
    let sent = fx.model.embedded.generation;

    let root = fx.model.project_url.clone().unwrap();
    let other = root.join("other.tex");
    let orphan = root.join("orphan.tex");
    std::fs::write(&other, "\\documentclass{article}\n\\begin{document}\nother\n\\end{document}\n").unwrap();
    std::fs::write(&orphan, "\\section{Orphan}\n").unwrap();
    fx.model.project_files.push(other);
    fx.model.project_files.push(orphan.clone());
    let main_active = fx.model.active_document_url.clone();
    let main_snapshot = fx
        .model
        .registered_sessions
        .iter()
        .find(|s| s.path().raw_value() == "main.tex")
        .expect("main.tex session")
        .snapshot();

    fx.model.active_document_url = Some(orphan);
    fx.model.document_snapshot = None;
    fx.model.refresh_build_target();
    assert!(
        fx.model.build_source_url().is_none(),
        "the resolver yields no target for an unowned file among two mains (it gave {:?}; message {:?})",
        fx.model.build_source_url(),
        fx.model.build_target_message
    );
    assert!(fx.model.retained_pdf.is_none(), "the retained PDF belonged to the old target");
    assert!(!fx.model.embedded.is_running(), "the helper session was retired");
    assert!(fx.model.embedded.restart_wanted, "the editing preview that was on screen is remembered");
    fx.model.request_embedded_restart(&fx.store);
    assert!(fx.model.embedded_poll_delay(false).is_none(), "nothing starts while there is no target");
    assert!(fx.model.embedded.restart_wanted, "the flag waits for a target");

    fx.model.active_document_url = main_active;
    fx.model.document_snapshot = Some(main_snapshot);
    fx.model.refresh_build_target();
    assert!(fx.model.build_source_url().is_some(), "main.tex is the target again");
    fx.model.request_embedded_restart(&fx.store);
    assert!(fx.model.embedded_poll_delay(false).is_some(), "the restart is pending, no edit needed");
    fx.flush();
    fx.published();
    assert!(fx.model.displaying_editing_preview(), "the editing preview is back");
    assert_eq!(fx.model.embedded.generation, sent + 1, "the restart sent one update");
}

/// A replacement controller must own a new directory while the old
/// controller's helper and asynchronous reaper still own the previous one.
fn controller_recreation_does_not_share_session_directories() {
    let mut previous = Fixture::open("previous-controller", "true");
    previous.model.sync_live_settings(&previous.store);
    previous.flush();
    previous.published();
    let previous_dir = PathBuf::from(&previous.model.retained_pdf.as_ref().unwrap().artifact)
        .parent().unwrap().parent().unwrap().to_path_buf();

    let mut current = Fixture::open("replacement-controller", "true");
    current.model.sync_live_settings(&current.store);
    current.flush();
    current.published();
    let current_dir = PathBuf::from(&current.model.retained_pdf.as_ref().unwrap().artifact)
        .parent().unwrap().parent().unwrap().to_path_buf();
    assert_ne!(previous_dir, current_dir, "each controller owns a distinct helper directory");

    previous.model.stop_embedded();
    let deadline = Instant::now() + Duration::from_secs(3);
    while previous_dir.exists() {
        assert!(Instant::now() < deadline, "the previous helper directory was reaped");
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(current_dir.exists(), "the previous reaper leaves the replacement helper intact");
    current.edit("% edit after the previous controller was reaped\n");
    current.flush();
    current.published();
    assert!(current.model.displaying_editing_preview(), "the replacement helper still publishes previews");
}

/// One entry point — settings and helper lookup are process-global.
#[test]
fn embedded_preview_final_floor() {
    let data = TempDir::new("data");
    std::env::set_var("XDG_DATA_HOME", data.join("data"));
    std::env::set_var("XDG_CONFIG_HOME", data.join("config"));
    std::env::set_var("XDG_CACHE_HOME", data.join("cache"));
    std::env::set_var("XDG_RUNTIME_DIR", data.join("run"));
    std::fs::create_dir_all(data.join("run")).unwrap();
    std::env::set_var("PITEX_PREVIEW_HELPER", install_stub(&data.0));

    late_prebuild_updates_never_cover_final();
    previews_during_running_build();
    save_before_publication_keeps_synctex(&data.join("gate"));
    disk_events_force_only_what_the_key_cannot_see();
    adopted_disk_change_reaches_the_helper();
    target_change_restarts_only_an_editing_preview();
    pin_toggle_restarts_only_an_editing_preview();
    target_through_a_nil_interval_restarts_the_preview();
    controller_recreation_does_not_share_session_directories();
}
