//! Embedded editing preview — the session with the `pitex-preview` helper
//! (TeXpresso-derived checkpointing XeTeX engine + Pitex XDV→PDF writer,
//! see `PreviewEngine/PROTOCOL.md`).
//!
//! The helper runs in its own process group and only ever writes into an
//! owned per-session temporary directory; every publication is a separate
//! directory the helper deletes once we `release` it. Buffer updates are
//! streamed as soon as they are coalesced, independent of the final-build
//! slot: the helper rolls back to its nearest checkpoint (killing a looping
//! engine) and keeps publishing intermediate snapshots.

use std::collections::{BTreeSet, HashMap, VecDeque};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Condvar, Mutex};

use crate::model::WorkspaceMessage;

/// One publication the helper made available.
#[derive(Debug, Clone)]
pub struct Publication {
    pub seq: u64,
    pub generation: u64,
    pub complete: bool,
    pub pages: u64,
    pub errors: u64,
    pub first_error: Option<String>,
    pub pdf: PathBuf,
    pub synctex: Option<PathBuf>,
    /// Every page comes from this pass (no stale tail).
    pub coherent: bool,
    /// PDF bytes read by the reader thread; `None` if the file vanished.
    pub bytes: Option<std::sync::Arc<[u8]>>,
}

#[derive(Debug, Clone)]
pub enum PreviewEvent {
    Published(Publication),
    Failed { generation: u64, message: String },
    Idle { generation: u64, seq: u64 },
    Error { code: String, message: String },
    /// The helper closed its output (exited or crashed).
    Exited,
}

/// Progress of the editing preview — separate from the final-build status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreviewStatus {
    Off,
    /// Newest sent generation is displayed and complete.
    Current,
    /// A newer generation is being typeset (or only a partial snapshot of
    /// it is on screen).
    Updating,
    /// Displayed preview has TeX errors (first one attached).
    Errors(String),
    Failed(String),
    Unavailable(String),
}

/// Modified buffers to override in the helper: (absolute path, content
/// hash, text).
type Overrides = Vec<(PathBuf, u64, String)>;

/// What the writer thread still has to send. Memory stays bounded while
/// the helper is slow or stuck: at most the line being written plus the
/// newest override set (a newer `update` replaces a queued one) and the
/// distinct publication seqs to release (each publication is released
/// once, and the helper holds at most a few unreleased).
#[derive(Default)]
struct Outbox {
    update: Option<(u64, Overrides)>,
    releases: BTreeSet<u64>,
    quit: bool,
}

type SharedOutbox = Arc<(Mutex<Outbox>, Condvar)>;

struct Session {
    id: u64,
    child: Option<Child>,
    outbox: SharedOutbox,
    dir: PathBuf,
}

impl Session {
    fn post(&self, f: impl FnOnce(&mut Outbox)) {
        let (lock, ready) = &*self.outbox;
        if let Ok(mut outbox) = lock.lock() {
            f(&mut outbox);
            ready.notify_one();
        }
    }
}

/// Writer loop: the only place that knows which overrides the helper has
/// actually received, so `files`/`closed` are always relative to that —
/// never to an update that was replaced before it was written.
fn write_outbox(outbox: SharedOutbox, mut stdin: impl Write) {
    let mut flushed: HashMap<PathBuf, u64> = HashMap::new();
    loop {
        let (update, releases, quit) = {
            let (lock, ready) = &*outbox;
            let Ok(mut o) = lock.lock() else { return };
            while o.update.is_none() && o.releases.is_empty() && !o.quit {
                o = match ready.wait(o) {
                    Ok(o) => o,
                    Err(_) => return,
                };
            }
            (o.update.take(), std::mem::take(&mut o.releases), o.quit)
        };
        if quit {
            let _ = stdin.write_all(b"{\"op\":\"quit\"}\n");
            return;
        }
        let mut text = String::new();
        for seq in releases {
            text.push_str(&format!("{{\"op\":\"release\",\"seq\":{seq}}}\n"));
        }
        if let Some((generation, overrides)) = update {
            text.push_str(&update_line(generation, overrides, &mut flushed));
            text.push('\n');
        }
        if stdin.write_all(text.as_bytes()).and_then(|_| stdin.flush()).is_err() {
            return; // helper gone; the reader reports the exit
        }
    }
}

/// `update` message for `overrides`, as a delta against what the helper
/// already holds (`flushed`, updated to the new set).
fn update_line(generation: u64, overrides: Overrides, flushed: &mut HashMap<PathBuf, u64>) -> String {
    let mut files = Vec::new();
    let mut current = HashMap::with_capacity(overrides.len());
    for (path, hash, text) in overrides {
        if flushed.get(&path) != Some(&hash) {
            files.push(serde_json::json!({ "path": path.to_string_lossy(), "text": text }));
        }
        current.insert(path, hash);
    }
    let mut closed: Vec<String> = flushed
        .keys()
        .filter(|p| !current.contains_key(*p))
        .map(|p| p.to_string_lossy().into_owned())
        .collect();
    closed.sort();
    *flushed = current;
    serde_json::json!({
        "op": "update",
        "generation": generation,
        "files": files,
        "closed": closed,
    })
    .to_string()
}

impl Session {
    /// App shutdown: no reaper thread would outlive the process, so kill
    /// the group and remove the session directory now.
    fn terminate_now(mut self) {
        if let Some(mut child) = self.child.take() {
            kill_group(child.id());
            let _ = child.kill();
            let _ = child.wait();
        }
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        self.post(|o| o.quit = true);
        let Some(mut child) = self.child.take() else { return };
        let pid = child.id();
        let dir = self.dir.clone();
        // Reap off the main thread: give the helper a moment to exit on its
        // own, then remove its whole process group (engine checkpoints
        // included) and the session directory.
        std::thread::spawn(move || {
            let deadline = std::time::Instant::now() + std::time::Duration::from_millis(800);
            while std::time::Instant::now() < deadline {
                if let Ok(Some(_)) = child.try_wait() {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(40));
            }
            kill_group(pid);
            let _ = child.kill();
            let _ = child.wait();
            let _ = std::fs::remove_dir_all(&dir);
        });
    }
}

#[cfg(unix)]
fn kill_group(pid: u32) {
    // SAFETY: plain syscall on a process group we created for the helper.
    unsafe {
        libc::kill(-(pid as i32), libc::SIGKILL);
    }
}
#[cfg(not(unix))]
fn kill_group(_pid: u32) {}

#[cfg(unix)]
fn pid_alive(pid: i32) -> bool {
    // SAFETY: signal 0 only probes for existence.
    let alive = unsafe { libc::kill(pid, 0) } == 0;
    alive || std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
}

/// `pitex-preview` shipped with this install: env override, next to the
/// executable, or `../lib/pitex/` (deb layout `/usr/lib/pitex/`).
pub fn helper_path() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("PITEX_PREVIEW_HELPER") {
        let p = PathBuf::from(p);
        if is_executable(&p) {
            return Some(p);
        }
    }
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?;
    [dir.join("pitex-preview"), dir.join("../lib/pitex/pitex-preview")]
        .into_iter()
        .map(|c| c.canonicalize().unwrap_or(c))
        .find(|c| is_executable(c))
}

fn is_executable(path: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::metadata(path)
            .map(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
    }
    #[cfg(not(unix))]
    {
        path.is_file()
    }
}

/// Owned root of all session directories (per user, mode 0700).
pub fn session_root() -> PathBuf {
    if let Some(runtime) = std::env::var_os("XDG_RUNTIME_DIR").filter(|v| !v.is_empty()) {
        return PathBuf::from(runtime).join("pitex-preview");
    }
    #[cfg(unix)]
    let uid = unsafe { libc::getuid() };
    #[cfg(not(unix))]
    let uid = 0;
    std::env::temp_dir().join(format!("pitex-preview-{uid}"))
}

/// True for an artifact path inside the embedded preview's session root.
pub fn is_preview_artifact(artifact: &str) -> bool {
    Path::new(artifact).starts_with(session_root())
}

fn create_private_dir(path: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))?;
    }
    Ok(())
}

/// Remove session directories left by Pitex processes that no longer run.
fn remove_stale_sessions(root: &Path) {
    let Ok(entries) = std::fs::read_dir(root) else { return };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(pid) = name
            .to_str()
            .and_then(|n| n.split('-').next())
            .and_then(|p| p.parse::<i32>().ok())
        else {
            continue;
        };
        #[cfg(unix)]
        let stale = pid != std::process::id() as i32 && !pid_alive(pid);
        #[cfg(not(unix))]
        let stale = false;
        if stale {
            let _ = std::fs::remove_dir_all(entry.path());
        }
    }
}

fn parse_event(line: &str) -> Option<PreviewEvent> {
    let v: serde_json::Value = serde_json::from_str(line).ok()?;
    let num = |k: &str| v.get(k).and_then(|x| x.as_u64()).unwrap_or(0);
    let text = |k: &str| v.get(k).and_then(|x| x.as_str()).map(str::to_string);
    match v.get("event")?.as_str()? {
        "published" => {
            let pdf = PathBuf::from(text("pdf")?);
            let bytes = std::fs::read(&pdf)
                .ok()
                .filter(|b| b.starts_with(b"%PDF"))
                .map(std::sync::Arc::<[u8]>::from);
            Some(PreviewEvent::Published(Publication {
                seq: num("seq"),
                generation: num("generation"),
                complete: v.get("complete").and_then(|x| x.as_bool()).unwrap_or(false),
                pages: num("pages"),
                errors: num("errors"),
                first_error: text("first_error"),
                pdf,
                synctex: text("synctex").map(PathBuf::from),
                coherent: v.get("coherent").and_then(|x| x.as_bool()).unwrap_or(false),
                bytes,
            }))
        }
        "failed" => Some(PreviewEvent::Failed {
            generation: num("generation"),
            message: text("message").unwrap_or_default(),
        }),
        "idle" => Some(PreviewEvent::Idle {
            generation: num("generation"),
            seq: num("seq"),
        }),
        "error" => Some(PreviewEvent::Error {
            code: text("code").unwrap_or_default(),
            message: text("message").unwrap_or_default(),
        }),
        _ => None, // "ready" and future events
    }
}

/// Source identity of a generation: absolute path → content hash of every
/// open TeX-like editor buffer (modified ones as sent to the helper, clean
/// ones equal to the disk text it reads). A save keeps the identity, so
/// SyncTeX survives it.
pub type BufferSnapshot = HashMap<PathBuf, u64>;

/// Bounded per-generation history for SyncTeX validation and floors.
const SNAPSHOT_HISTORY: usize = 32;

struct SentUpdate {
    generation: u64,
    /// Newest source-edit revision whose text this update carries.
    revision: u64,
    snapshot: BufferSnapshot,
}

struct Displayed {
    seq: u64,
    generation: u64,
    revision: u64,
    publication: Publication,
}

/// Controller state held by the workspace model.
///
/// Two counters: `generation` numbers transport updates (saves, closes and
/// rescans send updates too), `edit_revision` counts real source edits.
/// The final-build floor is an edit revision, so an update sent after a
/// manual build started — a queued pre-build edit flushed late, or the
/// save-only update the build's pre-save causes — can never unlock preview
/// display over the final PDF; only an edit made after the build started
/// (even while it runs) does.
pub struct EmbeddedPreview {
    session: Option<Session>,
    next_session_id: u64,
    /// Last generation sent (monotonic across helper restarts).
    pub generation: u64,
    /// Real source edits noted so far.
    pub edit_revision: u64,
    history: VecDeque<SentUpdate>,
    displayed: Option<Displayed>,
    /// Compiled buffer snapshot of the publication SyncTeX is bound to.
    pub bound_snapshot: Option<BufferSnapshot>,
    /// Previews carrying edit revisions at or below this lost to a final
    /// build.
    final_floor: Option<u64>,
    pending_floor: Option<u64>,
    /// Monotonic ms of the first unsent edit.
    pub pending_since: Option<u64>,
    pub status: PreviewStatus,
}

impl Default for EmbeddedPreview {
    fn default() -> Self {
        Self {
            session: None,
            next_session_id: 1,
            generation: 0,
            edit_revision: 0,
            history: VecDeque::new(),
            displayed: None,
            bound_snapshot: None,
            final_floor: None,
            pending_floor: None,
            pending_since: None,
            status: PreviewStatus::Off,
        }
    }
}

/// This build ships and supports the embedded preview backend (the
/// Ubuntu 24.04 package; not the 22.04 compatibility build or Windows).
pub const SUPPORTED: bool = cfg!(all(unix, feature = "embedded-preview"));

/// Coalescing window before a burst of edits is streamed.
pub const COALESCE_MS: u64 = 120;
/// Retry interval while an IME composition is open.
pub const COMPOSING_RETRY_MS: u64 = 150;

pub struct UpdateContext<'a> {
    pub root: &'a Path,
    pub main_relative: &'a str,
    /// (absolute path, content hash, text) of every modified TeX buffer —
    /// the overrides streamed to the helper.
    pub buffers: Vec<(PathBuf, u64, String)>,
    /// Identity of all open TeX buffers at this update (SyncTeX check).
    pub identity: BufferSnapshot,
    pub sink: Sender<WorkspaceMessage>,
}

impl EmbeddedPreview {
    pub fn is_running(&self) -> bool {
        self.session.is_some()
    }

    pub fn session_id(&self) -> Option<u64> {
        self.session.as_ref().map(|s| s.id)
    }

    /// A real source edit arrived: count it and arm (never extend) the
    /// coalescing window.
    pub fn note_edit(&mut self, now_ms: u64) {
        self.edit_revision += 1;
        self.request_flush(now_ms);
    }

    /// Something besides an edit needs streaming (save, backend enable).
    pub fn request_flush(&mut self, now_ms: u64) {
        if self.pending_since.is_none() {
            self.pending_since = Some(now_ms);
        }
    }

    pub fn poll_delay(&self, now_ms: u64, composing: bool) -> Option<u64> {
        let since = self.pending_since?;
        let due = since + COALESCE_MS;
        Some(if now_ms < due {
            due - now_ms
        } else if composing {
            COMPOSING_RETRY_MS
        } else {
            0
        })
    }

    /// Stop the session (backend/live toggle, helper failure). The retained
    /// PDF bytes may stay on screen; the caller drops the SyncTeX binding
    /// because the session directory is removed. A manual build in flight
    /// keeps its pending floor.
    pub fn stop(&mut self) {
        self.session = None;
        self.reset_session_state();
        self.pending_since = None;
        self.status = PreviewStatus::Off;
    }

    /// App shutdown: end the helper synchronously (see `Session`).
    pub fn terminate_now(&mut self) {
        if let Some(session) = self.session.take() {
            session.terminate_now();
        }
    }

    /// Stop for a context change (workspace close, build target or pin
    /// change): the final-build floors belonged to the old context.
    pub fn reset_context(&mut self) {
        self.stop();
        self.final_floor = None;
        self.pending_floor = None;
    }

    fn reset_session_state(&mut self) {
        self.history.clear();
        self.displayed = None;
        self.bound_snapshot = None;
    }

    fn sent_update(&self, generation: u64) -> Option<&SentUpdate> {
        self.history.iter().find(|u| u.generation == generation)
    }

    /// Source identity compiled for `generation`, if still in history.
    pub fn snapshot_for(&self, generation: u64) -> Option<&BufferSnapshot> {
        self.sent_update(generation).map(|u| &u.snapshot)
    }

    /// True when the displayed artifact is this controller's preview.
    pub fn is_displaying(&self) -> bool {
        self.displayed.is_some()
    }

    fn spawn(&mut self, ctx: &UpdateContext) -> Result<(), String> {
        #[cfg(not(unix))]
        {
            let _ = ctx;
            return Err("The embedded preview engine is available on macOS and Linux only.".into());
        }
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            let helper = helper_path().ok_or_else(|| "pitex-preview helper not found".to_string())?;
            let root = session_root();
            create_private_dir(&root).map_err(|e| e.to_string())?;
            remove_stale_sessions(&root);
            let id = self.next_session_id;
            self.next_session_id += 1;
            let dir = root.join(format!("{}-{}", std::process::id(), id));
            create_private_dir(&dir).map_err(|e| e.to_string())?;
            let cache = dirs::cache_dir()
                .unwrap_or_else(std::env::temp_dir)
                .join("pitex/preview-engine");
            let _ = std::fs::create_dir_all(&cache);
            // Same TeX search path as final builds (TeX Live user installs
            // are not always on the desktop session's PATH).
            let mut paths: Vec<PathBuf> =
                std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default()).collect();
            for extra in crate::model::texlive_bin_dirs()
                .into_iter()
                .chain([PathBuf::from("/usr/local/bin"), PathBuf::from("/usr/bin")])
            {
                if !paths.contains(&extra) {
                    paths.push(extra);
                }
            }
            let path_env = std::env::join_paths(paths).map_err(|e| e.to_string())?;
            let mut child = Command::new(&helper)
                .arg("--root")
                .arg(ctx.root)
                .arg("--main")
                .arg(ctx.main_relative)
                .arg("--out")
                .arg(&dir)
                .arg("--cache")
                .arg(&cache)
                .env("PATH", path_env)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .process_group(0)
                .spawn()
                .map_err(|e| format!("{}: {e}", helper.display()))?;
            let stdout = child.stdout.take().ok_or("no helper stdout")?;
            let stdin = child.stdin.take().ok_or("no helper stdin")?;
            let outbox: SharedOutbox = Arc::default();
            // Writer thread: large buffer updates never block the UI.
            {
                let outbox = outbox.clone();
                std::thread::spawn(move || write_outbox(outbox, stdin));
            }
            let sink = ctx.sink.clone();
            std::thread::spawn(move || {
                let reader = BufReader::new(stdout);
                for line in reader.lines() {
                    let Ok(line) = line else { break };
                    if let Some(event) = parse_event(&line) {
                        if sink
                            .send(WorkspaceMessage::EmbeddedPreview { session: id, event })
                            .is_err()
                        {
                            return;
                        }
                    }
                }
                let _ = sink.send(WorkspaceMessage::EmbeddedPreview {
                    session: id,
                    event: PreviewEvent::Exited,
                });
            });
            self.session = Some(Session { id, child: Some(child), outbox, dir });
            // Generations stay monotonic across respawns (final floor).
            self.reset_session_state();
            Ok(())
        }
    }

    /// Stream the newest buffer state (starting the helper if needed).
    pub fn flush(&mut self, ctx: UpdateContext) {
        self.pending_since = None;
        if self.session.is_none() {
            if let Err(e) = self.spawn(&ctx) {
                self.status = PreviewStatus::Unavailable(e);
                return;
            }
        }
        let Some(session) = self.session.as_ref() else { return };
        self.generation += 1;
        self.history.push_back(SentUpdate {
            generation: self.generation,
            revision: self.edit_revision,
            snapshot: ctx.identity,
        });
        while self.history.len() > SNAPSHOT_HISTORY {
            self.history.pop_front();
        }
        let generation = self.generation;
        session.post(move |o| o.update = Some((generation, ctx.buffers)));
        self.status = PreviewStatus::Updating;
    }

    fn release(&self, seq: u64) {
        if let Some(s) = &self.session {
            s.post(|o| {
                o.releases.insert(seq);
            });
        }
    }

    /// A manual (final) build is starting. Every edit noted so far —
    /// including ones still waiting in the coalescing window — is part of
    /// what it compiles (its pre-save writes them to disk).
    pub fn note_final_build_started(&mut self) {
        self.pending_floor = Some(self.edit_revision);
    }

    /// The manual build succeeded and its PDF replaced the display.
    /// Returns the displayed preview to show again when it already carries
    /// an edit made after the build started (drafting continues);
    /// otherwise the preview is released.
    pub fn note_final_build_published(&mut self) -> Option<Publication> {
        let floor = self.pending_floor.take().unwrap_or(self.edit_revision);
        self.final_floor = Some(self.final_floor.map_or(floor, |f| f.max(floor)));
        if self.session.is_some() && self.status == PreviewStatus::Updating {
            // stay "updating" only if a later edit is still in flight
            if self.edit_revision <= floor {
                self.status = PreviewStatus::Current;
            }
        }
        match &self.displayed {
            Some(d) if d.revision > floor => Some(d.publication.clone()),
            Some(d) => {
                let seq = d.seq;
                self.displayed = None;
                self.bound_snapshot = None;
                self.release(seq);
                None
            }
            None => None,
        }
    }

    /// Edit revision carried by a publication of `generation`; `None` when
    /// the generation is no longer known (treated as ineligible once a
    /// final build set a floor).
    fn revision_of(&self, generation: u64) -> Option<u64> {
        self.sent_update(generation).map(|u| u.revision)
    }

    fn eligible_over_final(&self, generation: u64) -> bool {
        match self.final_floor {
            None => true,
            Some(floor) => self.revision_of(generation).map_or(false, |r| r > floor),
        }
    }

    /// Apply a helper event.
    pub fn apply(&mut self, session: u64, event: PreviewEvent) -> ApplyOutcome {
        if self.session_id() != Some(session) {
            return ApplyOutcome::Nothing; // late event from a stopped session
        }
        match event {
            PreviewEvent::Published(p) => {
                // Same-context publications may lag the newest sent
                // generation; they must advance seq and generation, answer
                // an update that was sent, and carry an edit newer than
                // the last final build.
                let advances = self
                    .displayed
                    .as_ref()
                    .map(|d| p.seq > d.seq && p.generation >= d.generation)
                    .unwrap_or(true);
                let floor_ok = self.eligible_over_final(p.generation);
                if !advances || !floor_ok || p.generation > self.generation || p.bytes.is_none() {
                    self.release(p.seq);
                    if !floor_ok && p.generation >= self.generation && p.complete {
                        // The final PDF on screen is current for the newest
                        // sources (only save/rescan updates since).
                        self.status = PreviewStatus::Current;
                    }
                    return ApplyOutcome::Nothing;
                }
                // An evicted generation counts as carrying no edits, so it
                // can never be kept over a newer final PDF.
                let revision = self.revision_of(p.generation).unwrap_or(0);
                let previous = self.displayed.replace(Displayed {
                    seq: p.seq,
                    generation: p.generation,
                    revision,
                    publication: p.clone(),
                });
                if let Some(old) = previous {
                    self.release(old.seq);
                }
                self.bound_snapshot = None;
                self.status = if !p.complete || p.generation < self.generation {
                    PreviewStatus::Updating
                } else if p.errors > 0 {
                    PreviewStatus::Errors(p.first_error.clone().unwrap_or_default())
                } else {
                    PreviewStatus::Current
                };
                ApplyOutcome::Display(p)
            }
            PreviewEvent::Failed { generation, message } => {
                if generation >= self.generation {
                    self.status = PreviewStatus::Failed(message);
                }
                ApplyOutcome::Nothing
            }
            PreviewEvent::Idle { generation, seq } => {
                // Nothing TeX read changed: the displayed publication (if it
                // is the helper's newest) is current for `generation`, and
                // may be bound to SyncTeX against that generation's sources.
                let latest = generation >= self.generation;
                let Some(d) = self.displayed.as_mut() else {
                    // seq0 idle with nothing on screen must not erase a
                    // failure (parity with the Mac client).
                    if latest
                        && !matches!(
                            self.status,
                            PreviewStatus::Failed(_)
                                | PreviewStatus::Errors(_)
                                | PreviewStatus::Unavailable(_)
                        )
                    {
                        self.status = PreviewStatus::Current;
                    }
                    return ApplyOutcome::Nothing;
                };
                if d.seq != seq || generation < d.generation {
                    return ApplyOutcome::Nothing;
                }
                d.generation = generation;
                if latest && d.publication.complete {
                    self.status = if d.publication.errors > 0 {
                        PreviewStatus::Errors(d.publication.first_error.clone().unwrap_or_default())
                    } else {
                        PreviewStatus::Current
                    };
                }
                let mut p = d.publication.clone();
                p.generation = generation;
                ApplyOutcome::Rebind(p)
            }
            PreviewEvent::Error { code, message } => {
                if matches!(code.as_str(), "no_tex" | "no_engine" | "usage") {
                    self.session = None;
                    self.reset_session_state();
                    self.status = PreviewStatus::Unavailable(message);
                } else {
                    self.status = PreviewStatus::Failed(message);
                }
                ApplyOutcome::Nothing
            }
            PreviewEvent::Exited => {
                if !matches!(self.status, PreviewStatus::Unavailable(_)) {
                    self.status =
                        PreviewStatus::Failed("The embedded preview engine stopped.".into());
                }
                // Respawned on the next edit.
                self.session = None;
                self.reset_session_state();
                ApplyOutcome::Nothing
            }
        }
    }
}

/// What the model should do after a helper event.
pub enum ApplyOutcome {
    Nothing,
    /// A newer publication to put on screen.
    Display(Publication),
    /// The displayed publication is current for a newer generation (an
    /// `idle` update): bind SyncTeX against that generation if it matches.
    Rebind(Publication),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Default)]
    struct Pipe(Arc<Mutex<Vec<u8>>>);
    impl Write for Pipe {
        fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(b);
            Ok(b.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    impl Pipe {
        /// Waits for the written line whose generation is `generation`.
        fn line(&self, generation: u64) -> serde_json::Value {
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
            loop {
                let text = String::from_utf8(self.0.lock().unwrap().clone()).unwrap();
                let found = text
                    .lines()
                    .map(|l| serde_json::from_str::<serde_json::Value>(l).unwrap())
                    .find(|v| v["generation"] == generation);
                if let Some(v) = found {
                    return v;
                }
                assert!(std::time::Instant::now() < deadline, "generation {generation} written");
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
        }
        fn lines(&self) -> Vec<String> {
            let text = String::from_utf8(self.0.lock().unwrap().clone()).unwrap();
            text.lines().map(str::to_owned).collect()
        }
    }

    fn post(outbox: &SharedOutbox, f: impl FnOnce(&mut Outbox)) {
        f(&mut outbox.0.lock().unwrap());
        outbox.1.notify_one();
    }

    /// Updates replaced while the writer is busy are never sent, and the
    /// next delta is against what the helper actually received: a close
    /// followed by a reopen with the same text must not drop the override,
    /// and a later close must still send it.
    #[test]
    fn coalesced_updates_diff_against_written_overrides() {
        let a = PathBuf::from("/p/chapter.tex");
        let outbox: SharedOutbox = Arc::default();
        let pipe = Pipe::default();
        post(&outbox, |o| o.update = Some((1, vec![(a.clone(), 7, "edited".into())])));
        let writer = {
            let (outbox, pipe) = (outbox.clone(), pipe.clone());
            std::thread::spawn(move || write_outbox(outbox, pipe))
        };
        assert_eq!(pipe.line(1)["files"][0]["text"], "edited");

        // Close (gen 2) replaced by a reopen with the same text (gen 3)
        // before the writer picks it up; two releases, one duplicated.
        post(&outbox, |o| {
            o.update = Some((2, vec![]));
            o.update = Some((3, vec![(a.clone(), 7, "edited".into())]));
            o.releases.extend([4, 4, 5]);
        });
        let third = pipe.line(3);
        assert_eq!(third["files"], serde_json::json!([]));
        assert_eq!(third["closed"], serde_json::json!([]));

        post(&outbox, |o| o.update = Some((4, vec![])));
        assert_eq!(pipe.line(4)["closed"], serde_json::json!(["/p/chapter.tex"]));

        post(&outbox, |o| o.quit = true);
        writer.join().unwrap();
        let lines = pipe.lines();
        assert!(!lines.iter().any(|l| l.contains("\"generation\":2")), "replaced update not sent");
        let releases: Vec<_> = lines.iter().filter(|l| l.contains("release")).collect();
        assert_eq!(releases.len(), 2, "each seq released once: {releases:?}");
        assert_eq!(lines.last().unwrap(), "{\"op\":\"quit\"}");
    }

    /// A helper that stops reading: the writer is stuck on one large line
    /// while the client keeps flushing. Only the newest update and the
    /// distinct releases wait, and the update finally written is diffed
    /// against the stuck line (the replaced updates never reached the
    /// helper), so it still closes `a`.
    #[test]
    fn stalled_helper_keeps_one_pending_update() {
        use std::os::unix::net::UnixStream;
        let (helper, client) = UnixStream::pair().unwrap();
        let (a, b) = (PathBuf::from("/p/a.tex"), PathBuf::from("/p/b.tex"));
        let outbox: SharedOutbox = Arc::default();
        post(&outbox, |o| o.update = Some((1, vec![(a.clone(), 1, "x".repeat(16 << 20))])));
        let writer = {
            let outbox = outbox.clone();
            std::thread::spawn(move || write_outbox(outbox, client))
        };
        while outbox.0.lock().unwrap().update.is_some() {
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        for g in 2..=300u64 {
            post(&outbox, |o| {
                o.update = Some((g, vec![(b.clone(), g, "y".repeat(64 << 10))]));
                o.releases.insert(g % 3 + 1);
            });
        }
        {
            let o = outbox.0.lock().unwrap();
            assert_eq!(o.update.as_ref().map(|u| u.0), Some(300), "only the newest update waits");
            assert_eq!(o.releases.len(), 3);
        }

        let mut reader = BufReader::new(helper);
        let mut lines: Vec<serde_json::Value> = Vec::new();
        while lines.last().map_or(true, |v| v["generation"] != 300) {
            let mut line = String::new();
            assert!(reader.read_line(&mut line).unwrap() > 0, "writer ended early");
            lines.push(serde_json::from_str(&line).unwrap());
        }
        let ops: Vec<String> = lines
            .iter()
            .map(|v| format!("{}{}", v["op"].as_str().unwrap(), v["generation"].as_u64().or(v["seq"].as_u64()).unwrap()))
            .collect();
        assert_eq!(ops, ["update1", "release1", "release2", "release3", "update300"]);
        assert_eq!(lines[4]["closed"], serde_json::json!(["/p/a.tex"]));
        assert_eq!(lines[4]["files"][0]["path"], "/p/b.tex");

        post(&outbox, |o| o.quit = true);
        writer.join().unwrap();
        let mut rest = String::new();
        std::io::Read::read_to_string(&mut reader, &mut rest).unwrap();
        assert_eq!(rest, "{\"op\":\"quit\"}\n");
    }
}
