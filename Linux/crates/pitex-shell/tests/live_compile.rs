//! Live-compile scheduler bridge — model-level integration tests:
//! real edit → coalesced debounce → auto-save → isolated `.pitex-live`
//! build, own-save non-retriggering, stale log/completion suppression,
//! orphan-completion busy release, and the `{outdir}` requirement for
//! arbitrary custom commands. Build commands are hermetic `/bin/sh`
//! one-liners — no TeX install needed.

#![cfg(unix)]

use build_core::{BuildEvent, BuildID, BuildLifecycle, BuildLogChannel, BuildLogEntry, BuildOutcome};
use document_session_core::DocumentMutation;
use pitex_shell::model::{
    ConsoleSection, WorkspaceBuildState, WorkspaceMessage, WorkspaceModel,
};
use pitex_shell::settings::{Preferences, SettingsStore};
use std::cell::Cell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
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
            "pitex-shell-live-compile-{tag}-{}-{nanos}",
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
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(path, text).unwrap();
}

/// Feeds results back into the model like the GTK dispatch until `want`
/// matches; the matching message is returned.
fn pump(
    model: &mut WorkspaceModel,
    rx: &Receiver<WorkspaceMessage>,
    want: impl Fn(&WorkspaceMessage) -> bool,
) -> WorkspaceMessage {
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        let message = rx
            .recv_timeout(deadline.saturating_duration_since(Instant::now()))
            .expect("a workspace message arrived before the deadline");
        match &message {
            WorkspaceMessage::SaveFinished { path, result } => {
                model.apply_save_finished(path, result.clone());
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

/// Applies `text` appended at the end of `path`'s open session — the
/// same mutation path the editor adapter drives.
fn edit(model: &mut WorkspaceModel, path: &str, text: &str) {
    let session = model
        .registered_sessions
        .iter()
        .find(|s| s.path().raw_value() == path)
        .unwrap_or_else(|| panic!("session for {path}"))
        .clone();
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
    model.document_snapshot = Some(session.snapshot());
}

/// Deterministic scheduler clock the scenario winds by hand.
fn manual_clock() -> (Rc<Cell<u64>>, Rc<dyn Fn() -> u64>) {
    let cell = Rc::new(Cell::new(0u64));
    let moved = cell.clone();
    (cell, Rc::new(move || moved.get()))
}

/// A local project open with a custom build command the worker can
/// actually execute without a TeX install. `main` may be nested
/// ("manuscript/main.tex"); `command` should use `{file}`/`{outdir}`.
struct Fixture {
    _dir: TempDir,
    model: WorkspaceModel,
    rx: Receiver<WorkspaceMessage>,
    store: SettingsStore,
    now: Rc<Cell<u64>>,
}

impl Fixture {
    fn open(tag: &str, main: &str, command: &str) -> Self {
        let dir = TempDir::new(tag);
        write(
            &dir.join(main),
            "\\documentclass{article}\n\\begin{document}\nhello\n\\end{document}\n",
        );
        let mut store = SettingsStore::new(Preferences::standard());
        store.set_live_compile_enabled(true);
        store.set_live_compile_delay_milliseconds(700);
        store.settings.build.custom_shell_acknowledged = true;
        let mut model = WorkspaceModel::new();
        let (tx, rx) = std::sync::mpsc::channel();
        model.set_event_sink(tx.clone());
        model.open(dir.path().to_path_buf(), tx);
        let message = pump(&mut model, &rx, |m| {
            matches!(m, WorkspaceMessage::OpenFinished(_))
        });
        let WorkspaceMessage::OpenFinished(result) = message else {
            unreachable!()
        };
        let opened = result.unwrap_or_else(|f| {
            panic!(
                "open failed: {}",
                match f {
                    pitex_shell::model::OpenFailure::Error(e) => e,
                    pitex_shell::model::OpenFailure::RemoteOpen { error, .. } => error,
                }
            )
        });
        model.apply_open(&mut store, opened);
        model.build_command_text = command.to_string();
        let (now, clock) = manual_clock();
        model.set_live_clock(clock);
        Self {
            _dir: dir,
            model,
            rx,
            store,
            now,
        }
    }

    /// main.tex-relative edit, noted through the real bridge.
    fn edit(&mut self, path: &str, text: &str) {
        edit(&mut self.model, path, text);
        let requests = self.model.note_source_edit(&self.store);
        self.model
            .dispatch_live_requests(requests, &self.store, "en");
    }

    /// Wind past the debounce and dispatch whatever the scheduler emits.
    fn fire(&mut self) {
        let now = self.now.get() + 700;
        self.now.set(now);
        let requests = self.model.poll_live(false);
        self.model
            .dispatch_live_requests(requests, &self.store, "en");
    }

    /// Pumps until the next `BuildFinished`, applies it to the model and
    /// returns `(build, outcome, output_pdf)`.
    fn finish_next(&mut self) -> (BuildID, Result<BuildOutcome, String>, String) {
        let message = pump(&mut self.model, &self.rx, |m| {
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
        self.model
            .apply_build_finished(build.clone(), outcome.clone(), &output_pdf, &self.store, "en");
        (build, outcome, output_pdf)
    }
}

/// One entry point — `SettingsStore` writes through process-global XDG
/// paths, so scenarios run sequentially rather than as parallel tests.
#[test]
fn live_compile_scenarios() {
    let data = TempDir::new("data");
    std::env::set_var("XDG_DATA_HOME", data.path());
    std::env::set_var("XDG_CONFIG_HOME", data.join("config"));

    edit_coalescing_and_ime();
    live_build_writes_isolated_output();
    nested_source_output_stem();
    own_save_does_not_retrigger();
    manual_build_keeps_sibling_output();
    superseded_failure_keeps_last_good();
    stale_results_never_publish();
    invalidate_then_next_build_starts();
    custom_without_outdir_rejected();
    tectonic_preset_and_quoting();
}

fn edit_coalescing_and_ime() {
    let mut fx = Fixture::open("coalesce", "main.tex", "cp {file} {outdir}/main.pdf");
    // Five rapid edits → one pending deadline; IME holds, never drops.
    for i in 0..5 {
        fx.edit("main.tex", &format!("% edit {i}\n"));
    }
    assert!(fx.model.live.pending_deadline().is_some());
    assert!(fx.model.active_run.is_none());
    fx.now.set(700);
    assert!(fx.model.poll_live(true).is_empty(), "IME must hold the edit");
    assert!(fx.model.live.pending_deadline().is_some());
    let requests = fx.model.poll_live(false);
    assert_eq!(requests.len(), 1);
    fx.model
        .dispatch_live_requests(requests, &fx.store, "en");
    let run = fx.model.active_run.clone().expect("live run started");
    let ctx = run.live.as_ref().expect("live context");
    assert_eq!(ctx.output_pdf, ".pitex-live/main/main.pdf");
    assert!(fx.model.live.pending_deadline().is_none());
    let (_, outcome, _) = fx.finish_next();
    outcome.unwrap_or_else(|e| panic!("live build failed: {e}"));
    assert!(fx._dir.join(".pitex-live/main/main.pdf").exists());
    assert!(fx.model.active_run.is_none());
}

fn live_build_writes_isolated_output() {
    let mut fx = Fixture::open("isolated", "main.tex", "cp {file} {outdir}/main.pdf");
    fx.edit("main.tex", "% live\n");
    fx.fire();
    let (_, outcome, output_pdf) = fx.finish_next();
    outcome.unwrap_or_else(|e| panic!("live build failed: {e}"));
    assert_eq!(output_pdf, ".pitex-live/main/main.pdf");
    assert!(fx._dir.join(".pitex-live/main/main.pdf").exists());
    // The sibling stays empty — live artifacts never land beside the
    // source.
    assert!(!fx._dir.join("main.pdf").exists());
    assert!(matches!(
        fx.model.build_state,
        WorkspaceBuildState::Succeeded { .. }
    ));
    assert_eq!(
        fx.model.latest_built_pdf_name.as_deref(),
        Some(".pitex-live/main/main.pdf")
    );
}

fn nested_source_output_stem() {
    let mut fx = Fixture::open(
        "nested",
        "manuscript/main.tex",
        "cp {file} {outdir}/main.pdf",
    );
    fx.edit("manuscript/main.tex", "% nested\n");
    fx.fire();
    let (_, outcome, output_pdf) = fx.finish_next();
    outcome.unwrap_or_else(|e| panic!("live build failed: {e}"));
    assert_eq!(output_pdf, ".pitex-live/manuscript/main/main.pdf");
    assert!(fx
        ._dir
        .join(".pitex-live/manuscript/main/main.pdf")
        .exists());
}

fn own_save_does_not_retrigger() {
    let mut fx = Fixture::open("ownsave", "main.tex", "cp {file} {outdir}/main.pdf");
    fx.edit("main.tex", "% edit\n");
    fx.fire();
    let (_, outcome, _) = fx.finish_next();
    outcome.unwrap_or_else(|e| panic!("live build failed: {e}"));
    // The build's pre-pass already persisted the session; a follow-up
    // `save()` on a clean document emits nothing — and either way the
    // snapshot republish must not schedule a new run.
    fx.model.save();
    let session = fx
        .model
        .registered_sessions
        .iter()
        .find(|s| s.path().raw_value() == "main.tex")
        .unwrap()
        .clone();
    fx.model.document_snapshot = Some(session.snapshot());
    assert!(fx.model.note_source_edit(&fx.store).is_empty());
    assert!(fx.model.live.pending_deadline().is_none());
    // A dirty-then-clean save mid-debounce also stays silent: edit, then
    // the write commits the same content the signature already saw.
    fx.edit("main.tex", "% another\n");
    fx.model.note_source_edit(&fx.store);
    assert!(fx.model.live.pending_deadline().is_some());
    fx.fire();
    let _ = fx.finish_next();
}

fn manual_build_keeps_sibling_output() {
    let mut fx = Fixture::open("manual", "main.tex", "cp {file} {outdir}/main.pdf");
    // Live first — isolated artifact under .pitex-live.
    fx.edit("main.tex", "% live\n");
    fx.fire();
    let (_, outcome, _) = fx.finish_next();
    outcome.unwrap_or_else(|e| panic!("live build failed: {e}"));
    // Now a manual run: same command text but `{outdir}` maps to the
    // source's own directory — the sentinel sibling PDF proves manual
    // output semantics are unchanged.
    fx.model.start_build(&fx.store, "en");
    let (_, outcome, output_pdf) = fx.finish_next();
    outcome.unwrap_or_else(|e| panic!("manual build failed: {e}"));
    assert_eq!(output_pdf, "main.pdf");
    assert!(fx._dir.join("main.pdf").exists());
    assert!(fx._dir.join(".pitex-live/main/main.pdf").exists());
    assert_eq!(fx.model.latest_built_pdf_name.as_deref(), Some("main.pdf"));
}

fn superseded_failure_keeps_last_good() {
    let mut fx = Fixture::open("superseded", "main.tex", "cp {file} {outdir}/main.pdf");
    fx.edit("main.tex", "% good\n");
    fx.fire();
    let (_, outcome, _) = fx.finish_next();
    outcome.unwrap_or_else(|e| panic!("live build failed: {e}"));
    let WorkspaceBuildState::Succeeded { pdf: good_pdf, .. } = fx.model.build_state.clone()
    else {
        panic!("expected Succeeded")
    };
    let good_name = fx.model.latest_built_pdf_name.clone();

    // A failing command is now live; start its run, then supersede it
    // with a newer edit before its finish lands.
    fx.model.build_command_text = "cp /definitely/missing {outdir}/main.pdf".into();
    fx.edit("main.tex", "% second\n");
    fx.fire();
    assert!(fx.model.is_building());
    fx.edit("main.tex", "% third\n");
    let (_, outcome, _) = fx.finish_next();
    let _ = outcome; // cp fails — exactly what we want superseded.
    // The superseded failure never published: the retained PDF and its
    // artifact name are intact. (The third edit is pending but not yet
    // due on the frozen clock, so nothing dispatched past it.)
    match &fx.model.build_state {
        WorkspaceBuildState::Succeeded { pdf, .. } => {
            assert_eq!(&pdf[..], &good_pdf[..])
        }
        other => panic!("expected retained Succeeded, got {other:?}"),
    }
    assert_eq!(fx.model.latest_built_pdf_name, good_name);
    // Its own debounced run still fires on time — drain it.
    fx.fire();
    let _ = fx.finish_next();
}

fn stale_results_never_publish() {
    let mut fx = Fixture::open("stale", "main.tex", "cp {file} {outdir}/main.pdf");
    // A run id that nothing started — log, issue and finish all land on
    // deaf ears.
    let ghost = BuildID::new("build-never-started").unwrap();
    fx.model.apply_build_event(
        &ghost,
        BuildEvent::Log(BuildLogEntry {
            sequence: 1,
            channel: BuildLogChannel::StandardOutput,
            text: "stale log".into(),
        }),
    );
    assert!(fx.model.build_log_text.is_empty());
    assert!(fx
        .model
        .apply_build_finished(
            ghost.clone(),
            Err("stale error".into()),
            "main.pdf",
            &fx.store,
            "en"
        )
        .is_none());
    assert!(!matches!(
        fx.model.build_state,
        WorkspaceBuildState::Failed(ref r) if r == "stale error"
    ));
    // A fabricated successful outcome from the same ghost id is equally
    // dead — nothing publishes.
    assert!(fx
        .model
        .apply_build_finished(
            ghost,
            Ok(BuildOutcome {
                build_id: BuildID::new("build-never-started").unwrap(),
                lifecycle: BuildLifecycle::Succeeded {
                    exit_code: 0,
                    finished_at_milliseconds: 0,
                },
                issues: Vec::new(),
                artifact_disposition:
                    build_core::BuildArtifactDisposition::ReplacedWithSuccessfulPDF,
            }),
            "main.pdf",
            &fx.store,
            "en"
        )
        .is_none());
}

fn invalidate_then_next_build_starts() {
    let mut fx = Fixture::open(
        "invalidate",
        "main.tex",
        "sleep 1 && cp {file} {outdir}/main.pdf",
    );
    fx.edit("main.tex", "% first\n");
    fx.fire();
    assert!(fx.model.is_building());
    let retired = fx.model.active_run.clone().expect("run started");
    // Same-workspace invalidation (command/target change): pending and
    // active live work retire; the in-flight run becomes an orphan whose
    // late completion must release the busy flag.
    fx.model.build_command_text = "cp {file} {outdir}/main.pdf".into();
    fx.model.invalidate_live();
    assert!(fx.model.active_run.is_none());
    let (finished_id, _, _) = fx.finish_next();
    assert_eq!(finished_id, retired.build);
    // Busy state from the retired run was released — nothing published.
    assert!(!fx.model.is_building());
    // Next build actually starts — the whole point of the fix.
    fx.edit("main.tex", "% second\n");
    fx.fire();
    assert!(fx.model.active_run.is_some());
    assert!(fx.model.is_building());
    let (_, outcome, _) = fx.finish_next();
    outcome.unwrap_or_else(|e| panic!("post-invalidate build failed: {e}"));
    assert!(fx._dir.join(".pitex-live/main/main.pdf").exists());
    // A manual build after invalidation works the same way.
    fx.model.start_build(&fx.store, "en");
    let _ = fx.finish_next();
}

fn custom_without_outdir_rejected() {
    let mut fx = Fixture::open("nooutdir", "main.tex", "latexmk -pdf {file}");
    fx.edit("main.tex", "% edit\n");
    fx.fire();
    // Nothing ran — the reservation completed against a clear,
    // non-modal error and the scheduler slot is free.
    assert!(fx.model.active_run.is_none());
    match &fx.model.build_state {
        WorkspaceBuildState::Failed(reason) => {
            assert!(reason.contains("{outdir}"), "{reason}")
        }
        other => panic!("expected Failed, got {other:?}"),
    }
    assert!(fx.model.build_log_text.contains("{outdir}"));
    assert_eq!(fx.model.console_section, ConsoleSection::Log);
    assert!(!fx.model.bottom_panel_visible);
    assert!(fx.model.live.active().is_none());
    // And the scheduler still accepts the next edit.
    fx.edit("main.tex", "% retry\n");
    assert!(fx.model.live.pending_deadline().is_some());
}

fn tectonic_preset_and_quoting() {
    // The shipped preset rewrites to a known-safe --outdir form.
    assert_eq!(
        WorkspaceModel::live_effective_command(WorkspaceModel::TECTONIC_PRESET_COMMAND)
            .as_deref(),
        Ok("tectonic --synctex --outdir {outdir} {file}")
    );
    assert!(WorkspaceModel::live_effective_command("latexmk -pdf {file}").is_err());
    assert_eq!(
        WorkspaceModel::live_effective_command("latexmk -outdir={outdir} {file}")
            .as_deref(),
        Ok("latexmk -outdir={outdir} {file}")
    );

    let fx = Fixture::open("quoting", "my file.tex", "unused");
    let model = &fx.model;
    // Manual `{file}` stays verbatim — existing templates relied on it.
    assert_eq!(
        model.substitute_command_placeholders("latexmk {file}", "/bin/bash"),
        "latexmk my file.tex"
    );
    // Already-quoted spans collapse to a single quoting layer.
    assert_eq!(
        model.substitute_command_placeholders("latexmk \"{file}\"", "/bin/bash"),
        "latexmk 'my file.tex'"
    );
    // Live mode quotes every value for POSIX shells — also the remote
    // context, which always runs under the device's `/bin/sh`.
    assert_eq!(
        model.substitute_command_placeholders_live(
            "cp {file} {outdir}/o.pdf",
            ".pitex-live/my file",
            "/bin/sh",
        ),
        "cp 'my file.tex' '.pitex-live/my file'/o.pdf"
    );
    // cmd.exe gets double quotes — single quotes are literals there.
    // (Remote builds never reach this branch: they force `/bin/sh`.)
    assert_eq!(
        model.substitute_command_placeholders_live(
            "cp {file} {outdir}",
            ".pitex-live/my file",
            "cmd.exe",
        ),
        "cp \"my file.tex\" \".pitex-live/my file\""
    );
    // PowerShell takes the POSIX single-quote form too.
    assert_eq!(
        model.substitute_command_placeholders("type \"{file}\"", "pwsh"),
        "type 'my file.tex'"
    );
    // Manual `{outdir}` maps to the source's directory.
    assert_eq!(
        model.substitute_command_placeholders("latexmk -outdir={outdir} {file}", "/bin/bash"),
        "latexmk -outdir=. my file.tex"
    );
}
