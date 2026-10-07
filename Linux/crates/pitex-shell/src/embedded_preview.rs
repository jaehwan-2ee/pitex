//! Embedded editing preview — the session with the `pitex-preview` helper
//! (checkpointing XeTeX on Unix, cancellable XeLaTeX snapshots on Windows;
//! see `PreviewEngine/PROTOCOL.md` and `PreviewEngine/Windows/README.md`).
//!
//! The helper runs in its own process group (Windows Job Object) and writes into an
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
    pub warnings: Option<String>,
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
    Draft { generation: u64 },
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
    /// Rendering succeeded with reported compatibility limitations.
    Warnings(String),
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
    #[cfg(all(windows, feature = "embedded-preview"))]
    job: Option<WindowsJob>,
}

/// Assigned before any input is sent: closing the app or the handle also
/// removes helper descendants, even when the helper itself has crashed.
#[cfg(all(windows, feature = "embedded-preview"))]
struct WindowsJob(usize);

#[cfg(all(windows, feature = "embedded-preview"))]
impl WindowsJob {
    fn assign(child: &Child) -> Result<Self, String> {
        use std::os::windows::io::AsRawHandle;
        use windows::Win32::Foundation::{CloseHandle, HANDLE};
        use windows::Win32::System::JobObjects::{
            AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
            SetInformationJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
            JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
        };
        // The unnamed handle is not inherited; it has exactly one owner.
        let job = unsafe { CreateJobObjectW(None, windows::core::PCWSTR::null()) }
            .map_err(|e| e.to_string())?;
        let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        let result = unsafe {
            SetInformationJobObject(
                job,
                JobObjectExtendedLimitInformation,
                (&limits as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
                std::mem::size_of_val(&limits) as u32,
            )
            .and_then(|_| AssignProcessToJobObject(job, HANDLE(child.as_raw_handle())))
        };
        if let Err(e) = result {
            let _ = unsafe { CloseHandle(job) };
            return Err(e.to_string());
        }
        // Store an integer so the owned handle can move to the reaper thread.
        Ok(Self(job.0 as usize))
    }
}

#[cfg(all(windows, feature = "embedded-preview"))]
impl Drop for WindowsJob {
    fn drop(&mut self) {
        use windows::Win32::Foundation::{CloseHandle, HANDLE};
        let _ = unsafe { CloseHandle(HANDLE(self.0 as *mut std::ffi::c_void)) };
    }
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
        #[cfg(all(windows, feature = "embedded-preview"))]
        drop(self.job.take());
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
        #[cfg(all(windows, feature = "embedded-preview"))]
        let job = self.job.take();
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
            #[cfg(all(windows, feature = "embedded-preview"))]
            drop(job);
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
#[cfg(windows)]
fn kill_group(pid: u32) {
    use std::os::windows::process::CommandExt;
    let _ = Command::new("taskkill")
        .args(["/T", "/F", "/PID", &pid.to_string()])
        .creation_flags(0x0800_0000)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}
#[cfg(not(any(unix, windows)))]
fn kill_group(_pid: u32) {}

#[cfg(unix)]
fn pid_alive(pid: i32) -> bool {
    // SAFETY: signal 0 only probes for existence.
    let alive = unsafe { libc::kill(pid, 0) } == 0;
    alive || std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
}

#[cfg(all(windows, feature = "embedded-preview"))]
fn pid_alive(pid: i32) -> bool {
    use windows::Win32::Foundation::{CloseHandle, ERROR_ACCESS_DENIED};
    use windows::Win32::System::Threading::{GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};
    let process = match unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid as u32) } {
        Ok(process) => process,
        Err(error) => return error.code() == windows::core::HRESULT::from_win32(ERROR_ACCESS_DENIED.0),
    };
    let mut code = 0;
    let alive = unsafe { GetExitCodeProcess(process, &mut code) }.is_err() || code == 259; // STILL_ACTIVE
    let _ = unsafe { CloseHandle(process) };
    alive
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
    let helper = if cfg!(windows) { "pitex-preview.exe" } else { "pitex-preview" };
    [dir.join(helper), dir.join(format!("../lib/{}", crate::identity::current().linux_share_dir_name)).join(helper)]
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
    let prefix = format!("{}-preview", crate::identity::current().temp_prefix);
    if let Some(runtime) = std::env::var_os("XDG_RUNTIME_DIR").filter(|v| !v.is_empty()) {
        return PathBuf::from(runtime).join(&prefix);
    }
    #[cfg(unix)]
    let uid = unsafe { libc::getuid() };
    #[cfg(not(unix))]
    let uid = 0;
    std::env::temp_dir().join(format!("{prefix}-{uid}"))
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

fn create_session_dir(root: &Path) -> std::io::Result<(u64, PathBuf)> {
    #[cfg(unix)]
    use std::os::unix::fs::DirBuilderExt;
    use std::sync::atomic::{AtomicU64, Ordering};
    // A previous controller can still be cleaning up on its reaper thread.
    // Never reuse its ID or directory, including a directory left by a
    // previous process that had this PID.
    static NEXT_SESSION_ID: AtomicU64 = AtomicU64::new(1);
    loop {
        let id = NEXT_SESSION_ID.fetch_add(1, Ordering::Relaxed);
        let dir = root.join(format!("{}-{id}", std::process::id()));
        let mut builder = std::fs::DirBuilder::new();
        #[cfg(unix)]
        builder.mode(0o700);
        match builder.create(&dir) {
            Ok(()) => return Ok((id, dir)),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }
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
        #[cfg(any(unix, all(windows, feature = "embedded-preview")))]
        let stale = pid != std::process::id() as i32 && !pid_alive(pid);
        #[cfg(not(any(unix, all(windows, feature = "embedded-preview"))))]
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
                warnings: text("warnings").filter(|s| !s.trim().is_empty()),
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
        "draft" => Some(PreviewEvent::Draft { generation: num("generation") }),
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

/// What an update carries minus what cannot change the helper's result: the project and main file, the edit revision, and the identity of every open
/// TeX source (path → content hash). Dirtiness is not part of it (a save leaves the hash alone); the revision is (an edit followed by its undo restores
/// the hash but is two edits). An update with the same key as the last SENT one tells the helper nothing new.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlushKey {
    root: PathBuf,
    main_relative: String,
    revision: u64,
    identity: BufferSnapshot,
}

/// What `EmbeddedPreview::flush` did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlushOutcome {
    /// An update was posted to the helper (generation incremented, status Updating).
    Sent,
    /// The key equals the last sent one and the request was not forced: nothing was posted, nothing else changed.
    Skipped,
    /// No helper session could be started (the status says why).
    NotStarted,
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
    /// Last generation sent (monotonic across helper restarts).
    pub generation: u64,
    /// Real source edits noted so far.
    pub edit_revision: u64,
    history: VecDeque<SentUpdate>,
    displayed: Option<Displayed>,
    draft_generation: Option<u64>,
    /// Compiled buffer snapshot of the publication SyncTeX is bound to.
    pub bound_snapshot: Option<BufferSnapshot>,
    /// Previews carrying edit revisions at or below this lost to a final
    /// build.
    final_floor: Option<u64>,
    pending_floor: Option<u64>,
    /// Monotonic ms of the first unsent edit.
    pub pending_since: Option<u64>,
    pub status: PreviewStatus,
    /// Key of the last update SENT in this session; `None` before the first one and after the session ends or the helper reports a failure/error.
    last_flush: Option<FlushKey>,
    /// A request that must send even when the key is unchanged; consumed by the next `flush`.
    pending_force: bool,
    /// A build-target change retired a preview that should come back once the target is non-nil again (set by the model, which alone knows what is on screen); every `stop` clears it.
    pub restart_wanted: bool,
}

impl Default for EmbeddedPreview {
    fn default() -> Self {
        Self {
            session: None,
            generation: 0,
            edit_revision: 0,
            history: VecDeque::new(),
            displayed: None,
            draft_generation: None,
            bound_snapshot: None,
            final_floor: None,
            pending_floor: None,
            pending_since: None,
            status: PreviewStatus::Off,
            last_flush: None,
            pending_force: false,
            restart_wanted: false,
        }
    }
}

/// This build ships and supports the embedded preview backend.
pub const SUPPORTED: bool = cfg!(all(any(unix, windows), feature = "embedded-preview"));

/// Coalescing window before a burst of edits is streamed.
pub const COALESCE_MS: u64 = 16;
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

    /// Like `request_flush`, but the next flush sends even when its key equals the last sent one: for a cause the key cannot see (a file that is not
    /// an open source: an unopened `\input`, an open `.md`; agent completion; remote pull).
    pub fn request_flush_forced(&mut self, now_ms: u64) {
        self.pending_force = true;
        self.request_flush(now_ms);
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
        self.restart_wanted = false;
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
        self.last_flush = None;
        self.history.clear();
        self.displayed = None;
        self.draft_generation = None;
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
        #[cfg(not(any(unix, all(windows, feature = "embedded-preview"))))]
        {
            let _ = ctx;
            return Err("This build does not include the embedded preview engine.".into());
        }
        #[cfg(any(unix, all(windows, feature = "embedded-preview")))]
        {
            #[cfg(unix)]
            use std::os::unix::process::CommandExt;
            let helper = helper_path().ok_or_else(|| "pitex-preview helper not found".to_string())?;
            let root = session_root();
            create_private_dir(&root).map_err(|e| e.to_string())?;
            remove_stale_sessions(&root);
            let (id, dir) = create_session_dir(&root).map_err(|e| e.to_string())?;
            let cache = dirs::cache_dir()
                .unwrap_or_else(std::env::temp_dir)
                .join(crate::identity::current().xdg_dir_name)
                .join("preview-engine");
            let _ = std::fs::create_dir_all(&cache);
            // Same TeX search path as final builds (TeX Live user installs
            // are not always on the desktop session's PATH).
            let environment = build_core::augmented_environment(&HashMap::new());
            let mut command = Command::new(&helper);
            command
                .arg("--root")
                .arg(ctx.root)
                .arg("--main")
                .arg(ctx.main_relative)
                .arg("--out")
                .arg(&dir)
                .arg("--cache")
                .arg(&cache)
                .envs(environment)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::null());
            #[cfg(unix)]
            command.process_group(0);
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                command.creation_flags(0x0800_0000);
            }
            let mut child = command.spawn()
                .map_err(|e| format!("{}: {e}", helper.display()))?;
            #[cfg(all(windows, feature = "embedded-preview"))]
            let job = match WindowsJob::assign(&child) {
                Ok(job) => Some(job),
                Err(error) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    let _ = std::fs::remove_dir_all(&dir);
                    return Err(format!("Unable to own preview process tree: {error}"));
                }
            };
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
            self.session = Some(Session {
                id, child: Some(child), outbox, dir,
                #[cfg(all(windows, feature = "embedded-preview"))]
                job,
            });
            // Generations stay monotonic across respawns (final floor).
            self.reset_session_state();
            Ok(())
        }
    }

    /// Whether an update with `key` must go to the helper: always when forced, otherwise only when it differs from the last sent one. Records the key
    /// when it says yes.
    pub fn should_send(&mut self, key: &FlushKey, force: bool) -> bool {
        if force || self.last_flush.as_ref() != Some(key) {
            self.last_flush = Some(key.clone());
            true
        } else {
            false
        }
    }

    /// After a skipped flush no `idle` reply will come, so the publication on screen may bind SyncTeX now — the step an `idle` outcome performs
    /// (`ApplyOutcome::Rebind`). `None` when it is already bound or nothing is displayed.
    pub fn rebind_candidate(&self) -> Option<Publication> {
        if self.bound_snapshot.is_some() || self.draft_generation.is_some() {
            return None;
        }
        let d = self.displayed.as_ref()?;
        let mut p = d.publication.clone();
        p.generation = d.generation;
        Some(p)
    }

    /// Stream the newest buffer state (starting the helper if needed), unless its key equals the last sent one and the request was not forced.
    pub fn flush(&mut self, ctx: UpdateContext) -> FlushOutcome {
        self.pending_since = None;
        let force = std::mem::take(&mut self.pending_force);
        if self.session.is_none() {
            if let Err(e) = self.spawn(&ctx) {
                self.status = PreviewStatus::Unavailable(e);
                return FlushOutcome::NotStarted;
            }
        }
        if self.session.is_none() {
            return FlushOutcome::NotStarted;
        }
        let key = FlushKey {
            root: ctx.root.to_path_buf(),
            main_relative: ctx.main_relative.to_string(),
            revision: self.edit_revision,
            identity: ctx.identity.clone(),
        };
        if !self.should_send(&key, force) {
            return FlushOutcome::Skipped;
        }
        let Some(session) = self.session.as_ref() else { return FlushOutcome::NotStarted };
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
        FlushOutcome::Sent
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
        self.note_final_build_started_at(self.edit_revision);
    }

    /// SSH saves can finish uploading after later draft edits arrive.
    pub fn note_final_build_started_at(&mut self, saved_revision: u64) {
        self.pending_floor = Some(saved_revision);
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
                if self.draft_generation.is_some_and(|generation|p.generation>=generation) {self.draft_generation=None;}
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
                self.status = if self.draft_generation.is_some() {
                    PreviewStatus::Warnings(r"PDF output is disabled by \pdfdraftmode.".into())
                } else if !p.complete || p.generation < self.generation {
                    PreviewStatus::Updating
                } else if p.errors > 0 {
                    PreviewStatus::Errors(p.first_error.clone().unwrap_or_default())
                } else if let Some(warnings) = &p.warnings {
                    PreviewStatus::Warnings(warnings.clone())
                } else {
                    PreviewStatus::Current
                };
                ApplyOutcome::Display(p)
            }
            PreviewEvent::Draft { generation } => {
                if generation >= self.generation {
                    self.draft_generation = Some(generation);
                    self.bound_snapshot = None;
                    self.status = PreviewStatus::Warnings(r"PDF output is disabled by \pdfdraftmode.".into());
                }
                ApplyOutcome::Nothing
            }
            PreviewEvent::Failed { generation, message } => {
                self.last_flush = None; // the next request retries even with an unchanged key
                if generation >= self.generation {
                    self.status = PreviewStatus::Failed(message);
                }
                ApplyOutcome::Nothing
            }
            PreviewEvent::Idle { generation, seq } => {
                if self.draft_generation.is_some() {
                    if generation >= self.generation { self.status = PreviewStatus::Warnings(r"PDF output is disabled by \pdfdraftmode.".into()); }
                    return ApplyOutcome::Nothing;
                }
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
                                | PreviewStatus::Warnings(_)
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
                    } else if let Some(warnings) = &d.publication.warnings {
                        PreviewStatus::Warnings(warnings.clone())
                    } else {
                        PreviewStatus::Current
                    };
                }
                let mut p = d.publication.clone();
                p.generation = generation;
                ApplyOutcome::Rebind(p)
            }
            PreviewEvent::Error { code, message } => {
                self.last_flush = None;
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
    #[cfg(unix)]
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

    // ---- the flush key (C30L): an update the helper already has is not sent again ----

    fn fake_session() -> Session {
        Session {
            id: 1, child: None, outbox: Arc::default(), dir: PathBuf::from("/nonexistent/pitex-test-session"),
            #[cfg(all(windows, feature = "embedded-preview"))]
            job: None,
        }
    }

    fn running() -> EmbeddedPreview {
        let mut ep = EmbeddedPreview::default();
        ep.session = Some(fake_session());
        ep
    }

    fn ctx<'a>(main: &'a str, files: &[(&str, u64)]) -> UpdateContext<'a> {
        UpdateContext {
            root: Path::new("/p"),
            main_relative: main,
            buffers: Vec::new(),
            identity: files.iter().map(|(p, h)| (PathBuf::from(*p), *h)).collect(),
            sink: std::sync::mpsc::channel().0,
        }
    }

    fn queued_generation(ep: &EmbeddedPreview) -> Option<u64> {
        ep.session.as_ref().unwrap().outbox.0.lock().unwrap().update.as_ref().map(|u| u.0)
    }

    const MAIN: &[(&str, u64)] = &[("/p/main.tex", 7)];

    #[test]
    fn continuous_edits_flush_within_one_frame_but_wait_for_ime_commit() {
        let mut ep = EmbeddedPreview::default();
        ep.note_edit(100);
        ep.note_edit(115);
        assert_eq!(ep.poll_delay(116, false), Some(0), "typing cannot extend the window");
        assert_eq!(ep.poll_delay(116, true), Some(COMPOSING_RETRY_MS), "don't compile partial IME text");
    }

    #[test]
    fn equal_unforced_key_sends_nothing() {
        let mut ep = running();
        assert_eq!(ep.flush(ctx("main.tex", MAIN)), FlushOutcome::Sent);
        ep.status = PreviewStatus::Current;
        let generation = ep.generation;
        ep.request_flush(100);
        assert_eq!(ep.pending_since, Some(100));
        assert_eq!(ep.flush(ctx("main.tex", MAIN)), FlushOutcome::Skipped);
        assert_eq!(ep.generation, generation, "no generation churn");
        assert_eq!(ep.status, PreviewStatus::Current, "no Updating flash");
        assert_eq!(ep.history.len(), 1, "no history entry");
        assert_eq!(queued_generation(&ep), Some(generation), "nothing new posted to the helper");
        assert_eq!(ep.pending_since, None, "a skipped flush clears the window, or poll_delay would busy-loop");
        assert_eq!(ep.poll_delay(1_000, false), None);
    }

    #[test]
    fn changed_hash_sends_once() {
        let mut ep = running();
        assert_eq!(ep.flush(ctx("main.tex", MAIN)), FlushOutcome::Sent);
        let changed = &[("/p/main.tex", 8)][..];
        assert_eq!(ep.flush(ctx("main.tex", changed)), FlushOutcome::Sent);
        assert_eq!(ep.flush(ctx("main.tex", changed)), FlushOutcome::Skipped);
        assert_eq!(ep.generation, 2);
        // A source opened or closed changes the identity too.
        let two = &[("/p/main.tex", 8), ("/p/chapter.tex", 1)][..];
        assert_eq!(ep.flush(ctx("main.tex", two)), FlushOutcome::Sent);
    }

    #[test]
    fn revision_only_change_sends() {
        let mut ep = running();
        assert_eq!(ep.flush(ctx("main.tex", MAIN)), FlushOutcome::Sent);
        ep.note_edit(0); // an edit, then its undo: same hash, a new revision
        assert_eq!(ep.flush(ctx("main.tex", MAIN)), FlushOutcome::Sent);
        assert_eq!(ep.generation, 2);
    }

    #[test]
    fn forced_request_sends_even_with_an_equal_key() {
        let mut ep = running();
        assert_eq!(ep.flush(ctx("main.tex", MAIN)), FlushOutcome::Sent);
        ep.request_flush_forced(5);
        assert!(ep.pending_force);
        assert_eq!(ep.pending_since, Some(5));
        assert_eq!(ep.flush(ctx("main.tex", MAIN)), FlushOutcome::Sent);
        assert!(!ep.pending_force, "the force flag is consumed");
        assert_eq!(ep.flush(ctx("main.tex", MAIN)), FlushOutcome::Skipped, "the next unforced request skips again");
        assert_eq!(ep.generation, 2);
    }

    #[test]
    fn another_main_file_sends() {
        let mut ep = running();
        assert_eq!(ep.flush(ctx("main.tex", MAIN)), FlushOutcome::Sent);
        assert_eq!(ep.flush(ctx("other.tex", MAIN)), FlushOutcome::Sent);
    }

    #[test]
    fn key_is_cleared_when_the_session_ends_or_the_helper_fails() {
        // stop (respawn on the next request)
        let mut ep = running();
        assert_eq!(ep.flush(ctx("main.tex", MAIN)), FlushOutcome::Sent);
        ep.stop();
        ep.session = Some(fake_session());
        assert_eq!(ep.flush(ctx("main.tex", MAIN)), FlushOutcome::Sent, "first flush after stop");
        // the helper exits
        assert!(matches!(ep.apply(1, PreviewEvent::Exited), ApplyOutcome::Nothing));
        assert!(ep.session.is_none());
        ep.session = Some(fake_session());
        assert_eq!(ep.flush(ctx("main.tex", MAIN)), FlushOutcome::Sent, "first flush after the helper exited");
        // a failed pass
        assert_eq!(ep.flush(ctx("main.tex", MAIN)), FlushOutcome::Skipped);
        ep.apply(1, PreviewEvent::Failed { generation: ep.generation, message: "boom".into() });
        assert_eq!(ep.flush(ctx("main.tex", MAIN)), FlushOutcome::Sent, "retry after a failure");
        // a non-fatal helper error
        assert_eq!(ep.flush(ctx("main.tex", MAIN)), FlushOutcome::Skipped);
        ep.apply(1, PreviewEvent::Error { code: "oops".into(), message: "x".into() });
        assert_eq!(ep.flush(ctx("main.tex", MAIN)), FlushOutcome::Sent, "retry after an error");
        // a fatal helper error ends the session
        ep.apply(1, PreviewEvent::Error { code: "no_tex".into(), message: "x".into() });
        assert!(ep.session.is_none());
        ep.session = Some(fake_session());
        assert_eq!(ep.flush(ctx("main.tex", MAIN)), FlushOutcome::Sent, "first flush after a fatal error");
    }

    fn publication(generation: u64) -> Publication {
        Publication {
            seq: 1,
            generation,
            complete: true,
            pages: 1,
            errors: 0,
            first_error: None,
            warnings: None,
            pdf: PathBuf::from("/s/p1/main.pdf"),
            synctex: Some(PathBuf::from("/s/p1/main.synctex")),
            coherent: true,
            bytes: None,
        }
    }

    #[test]
    fn compatibility_warnings_survive_idle_and_clear_after_a_clean_publication() {
        let mut ep = running();
        ep.note_edit(0);
        assert_eq!(ep.flush(ctx("main.tex", MAIN)), FlushOutcome::Sent);
        let mut p = publication(1);
        p.bytes = Some(std::sync::Arc::from(&b"%PDF-1.7\n"[..]));
        p.warnings = Some("Unsupported PostScript operator".into());
        ep.apply(1, PreviewEvent::Published(p));
        assert_eq!(ep.status, PreviewStatus::Warnings("Unsupported PostScript operator".into()));
        ep.apply(1, PreviewEvent::Idle {generation: 1, seq: 1});
        assert!(matches!(ep.status, PreviewStatus::Warnings(_)));
        ep.note_edit(10);
        assert_eq!(ep.flush(ctx("main.tex", &[("/p/main.tex", 8)])), FlushOutcome::Sent);
        let mut clean = publication(2);
        clean.seq = 2;
        clean.bytes = Some(std::sync::Arc::from(&b"%PDF-1.7\n"[..]));
        ep.apply(1, PreviewEvent::Published(clean));
        assert_eq!(ep.status, PreviewStatus::Current);
    }

    #[test]
    fn pdf_draft_mode_blocks_idle_rebinding_and_resumes_after_publication() {
        let mut ep = running();
        ep.note_edit(0);
        assert_eq!(ep.flush(ctx("main.tex", MAIN)), FlushOutcome::Sent);
        let mut initial = publication(1);
        initial.bytes = Some(std::sync::Arc::from(&b"%PDF-1.7\n"[..]));
        ep.apply(1, PreviewEvent::Published(initial));
        ep.note_edit(10);
        assert_eq!(ep.flush(ctx("main.tex", &[("/p/main.tex", 8)])), FlushOutcome::Sent);
        ep.apply(1, PreviewEvent::Draft {generation:2});
        ep.status = PreviewStatus::Updating;
        assert!(matches!(ep.apply(1, PreviewEvent::Idle {generation:2,seq:1}),ApplyOutcome::Nothing));
        assert!(matches!(ep.status,PreviewStatus::Warnings(_)));
        assert!(ep.rebind_candidate().is_none());
        let mut delayed = publication(1);delayed.seq = 2;
        delayed.bytes = Some(std::sync::Arc::from(&b"%PDF-1.7\n"[..]));
        ep.apply(1,PreviewEvent::Published(delayed));
        assert!(matches!(ep.status,PreviewStatus::Warnings(_)));
        assert!(ep.rebind_candidate().is_none());
        let mut resumed = publication(2);resumed.seq = 3;
        resumed.bytes = Some(std::sync::Arc::from(&b"%PDF-1.7\n"[..]));
        ep.apply(1, PreviewEvent::Published(resumed));
        assert_eq!(ep.status, PreviewStatus::Current);
        assert!(ep.draft_generation.is_none());
    }

    #[test]
    fn newer_draft_remains_visible_over_an_older_remote_save() {
        let mut ep = running();
        ep.note_edit(0);
        assert_eq!(ep.flush(ctx("main.tex", MAIN)), FlushOutcome::Sent);
        let saved_revision = ep.edit_revision;

        // A new draft arrives while SSH is still publishing the saved
        // revision. Its preview is already visible before the build starts.
        ep.note_edit(100);
        assert_eq!(ep.flush(ctx("main.tex", &[("/p/main.tex", 8)])), FlushOutcome::Sent);
        let generation = ep.generation;
        let mut draft = publication(generation);
        draft.bytes = Some(Arc::from(&b"%PDF draft"[..]));
        assert!(matches!(ep.apply(1, PreviewEvent::Published(draft)), ApplyOutcome::Display(_)));

        ep.note_final_build_started_at(saved_revision);
        let retained = ep.note_final_build_published().expect("the later draft stays visible");
        assert_eq!(retained.generation, generation);
        assert!(ep.is_displaying());
        assert_eq!(ep.final_floor, Some(saved_revision));
    }

    #[test]
    fn skipped_flush_may_rebind_synctex_only_when_unbound_and_displaying() {
        let mut ep = running();
        assert!(ep.rebind_candidate().is_none(), "nothing displayed");
        ep.displayed = Some(Displayed { seq: 1, generation: 3, revision: 0, publication: publication(1) });
        let p = ep.rebind_candidate().expect("displayed and unbound");
        assert_eq!((p.seq, p.generation), (1, 3), "bound against the generation the idle reply confirmed");
        ep.bound_snapshot = Some(BufferSnapshot::new());
        assert!(ep.rebind_candidate().is_none(), "already bound");
    }

    #[test]
    fn restart_after_a_target_change_needs_no_force_and_every_stop_forgets_it() {
        let mut ep = running();
        assert_eq!(ep.flush(ctx("main.tex", MAIN)), FlushOutcome::Sent);
        // The model marks the retirement AFTER `reset_context`, which clears the flag with the session.
        ep.restart_wanted = true;
        ep.reset_context();
        assert!(!ep.restart_wanted, "reset_context forgets a flag set before it (workspace close)");
        ep.restart_wanted = true;
        ep.stop();
        assert!(!ep.restart_wanted, "a plain stop (backend off, helper retired) forgets it");
        // The restart is an ordinary request: the stop cleared the key, so the first flush sends.
        ep.restart_wanted = true;
        ep.session = Some(fake_session());
        ep.request_flush(500);
        assert!(!ep.pending_force, "the restart is not forced");
        assert_eq!(ep.flush(ctx("main.tex", MAIN)), FlushOutcome::Sent, "equal key, but the stop cleared it");
    }
}
