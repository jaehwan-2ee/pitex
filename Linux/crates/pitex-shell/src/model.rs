//! Port of `WorkspaceModel` (`Mac/Sources/AppShell/PitexApp.swift`) and the
//! build methods of `BuildSupport.swift`. All mutation happens on the UI
//! thread (mirroring `@MainActor`); long-running work — builds, SyncTeX
//! queries — runs on background threads and reports back through
//! `WorkspaceMessage` on the channel installed with `set_event_sink`.

use std::collections::{BTreeSet, HashMap, HashSet};
use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::sync::Arc;

use app_ports::FileCapabilityBroker;
#[cfg(unix)]
use linux_platform::LinuxFileCapabilityBroker as PlatformFileCapabilityBroker;
#[cfg(windows)]
use windows_platform::WindowsFileCapabilityBroker as PlatformFileCapabilityBroker;
use build_core::{
    BuildEvent, BuildID, BuildIssueRecord, BuildLifecycle, BuildOrchestrator, BuildOutcome,
    BuildPipeline, BuildTarget, BuildToolStage, LoginShellCommandPlan, ShellAuthority,
    ShellAuthoritySource, StreamingBuildExecutor, WorkingDirectoryPolicy,
};
use document_session_core::{
    AtomicDocumentStore, DiskContentHash, DocumentConflict, DocumentMutation, DocumentSaveOutcome,
    DocumentSaveState, DocumentSession, DocumentSessionRegistry,
    DocumentSnapshot,
};
use git_core::{GitCommit, GitStatus};
use language_core::{LanguageFileSnapshot, LanguageTokenKind, TeXDialect};
use project_core::ProjectFile;
use tex_domain::NormalizedRelativePath;
use settings_feature::TeXEnginePreference;
use synctex_core::{PDFPoint, SyncTeXQueryCandidate};
use tex_domain::StableDocumentID;

use crate::settings::SettingsStore;
use crate::synctex::{SyncTeXBinding, SyncTeXRunner};

/// `WorkspaceBuildExecutor` (RemoteSupport.swift) — routes each build to
/// this computer or, while a remote project is open, to the device. The
/// model and its `BuildOrchestrator` hold clones sharing one state, so the
/// session can swap executors without touching the orchestrator.
#[derive(Clone)]
pub struct WorkspaceBuildExecutor {
    shared: Arc<WorkspaceExecutorShared>,
}

struct WorkspaceExecutorShared {
    local: StreamingBuildExecutor,
    remote: std::sync::Mutex<Option<Arc<remote_core::RemoteBuildExecutor>>>,
}

impl WorkspaceBuildExecutor {
    pub fn new() -> Self {
        Self {
            shared: Arc::new(WorkspaceExecutorShared {
                local: StreamingBuildExecutor::default(),
                remote: std::sync::Mutex::new(None),
            }),
        }
    }

    /// `use(_:)` — installs the remote executor for the open session.
    pub fn use_remote(&self, remote: Option<Arc<remote_core::RemoteBuildExecutor>>) {
        *self
            .shared
            .remote
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = remote;
    }

    fn remote(&self) -> Option<Arc<remote_core::RemoteBuildExecutor>> {
        self.shared
            .remote
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }
}

impl Default for WorkspaceBuildExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl build_core::BuildProcessExecuting for WorkspaceBuildExecutor {
    fn execute(
        &self,
        request: &build_core::BuildProcessRequest,
        output: &mut (dyn FnMut(build_core::BuildProcessOutput) + Send),
    ) -> Result<build_core::BuildProcessResult, build_core::BuildProcessExecutorError> {
        if let Some(remote) = self.remote() {
            return remote.execute(request, output);
        }
        self.shared.local.execute(request, output)
    }

    fn cancel(&self, build_id: &build_core::BuildID) {
        if let Some(remote) = self.remote() {
            remote.cancel(build_id);
        }
        self.shared.local.cancel(build_id);
    }
}

// ─── State enums (verbatim port) ────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum WorkspacePhase {
    NoProject,
    Loading(PathBuf),
    Ready,
    Failed(String),
}

#[derive(Debug, Clone)]
pub enum WorkspaceBuildState {
    Unavailable(String),
    Building,
    Succeeded { pdf: Arc<[u8]>, log: String, hash: u64 },
    Failed(String),
}

#[derive(Debug, Clone)]
pub enum WorkspaceSyncTeXState {
    Unavailable(String),
    Current,
    Stale(String),
    Ambiguous(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsoleSection {
    Assistant,
    Git,
    Issues,
    Terminal,
    Log,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidebarSection {
    Outline,
    Labels,
    BibTeX,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DocumentOutlineItem {
    pub title: String,
    pub level: usize,
    pub line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DocumentLabelItem {
    pub name: String,
    pub line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BibliographyItem {
    pub key: String,
    pub kind: String,
    pub file: String,
}

/// `DocumentTodoItem` — one `% TODO:`/`% DONE:` comment collected from the
/// project's .tex files.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DocumentTodoItem {
    /// Project-relative path (like `BibliographyItem.file`).
    pub file: String,
    pub url: PathBuf,
    /// 1-based line of the comment.
    pub line: usize,
    pub text: String,
    pub done: bool,
}

/// `TodoLineEdit` — what a todo-row mutation does to its comment line.
#[derive(Debug, Clone)]
pub enum TodoLineEdit {
    ToggleDone,
    Rename(String),
    Delete,
}

#[derive(Debug)]
pub enum WorkspaceOpenError {
    NoTexSources,
    UnreadableProject,
    InvalidUtf8,
    OutsideProject,
    ChangedWhileOpening,
    SavePermissionDenied(String),
    SaveInterrupted(String),
}
impl std::fmt::Display for WorkspaceOpenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoTexSources => {
                write!(f, "The selected project contains no .tex source files.")
            }
            Self::UnreadableProject => {
                write!(f, "The selected project could not be enumerated.")
            }
            Self::InvalidUtf8 => write!(f, "The selected source is not valid UTF-8."),
            Self::OutsideProject => {
                write!(f, "The selected source is outside the project root.")
            }
            Self::ChangedWhileOpening => write!(
                f,
                "The source changed while it was opening. Open it again to avoid losing changes."
            ),
            Self::SavePermissionDenied(path) => write!(
                f,
                "The source could not be saved because access was denied: {path}"
            ),
            Self::SaveInterrupted(path) => {
                write!(f, "The atomic save did not complete: {path}")
            }
        }
    }
}
impl std::error::Error for WorkspaceOpenError {}

enum WorkspaceBuildError {
    NoActiveDocument,
    /// Kept for parity with `WorkspaceBuildError.buildAlreadyRunning` —
    /// `startBuild` now refuses re-entry silently like the Swift version.
    #[allow(dead_code)]
    BuildAlreadyRunning,
    CustomShellNotAcknowledged,
}
impl std::fmt::Display for WorkspaceBuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoActiveDocument => {
                write!(f, "No build target has been configured for this project.")
            }
            Self::BuildAlreadyRunning => write!(f, "A build is already running."),
            Self::CustomShellNotAcknowledged => write!(
                f,
                "Enable shell commands in Settings → Compile to run a custom build command."
            ),
        }
    }
}

/// Messages produced on background threads; the GTK layer drains them on the
/// main loop and calls the matching `apply_*` method.
pub enum WorkspaceMessage {
    /// An off-main save of the session at `path` finished.
    SaveFinished { path: NormalizedRelativePath, result: SaveResult },
    PdfLoaded { hash: u64, info: Option<crate::pdf::PdfInfo> },
    PdfRendered { key: u64, raster: crate::pdf::RenderedPage },
    BuildEvent(BuildEvent),
    BuildFinished {
        outcome: Result<BuildOutcome, String>,
        output_pdf: String,
    },
    ForwardResult(Result<SyncTeXQueryCandidate, String>),
    InverseResult(Result<SyncTeXQueryCandidate, String>),
    BindingRefreshed(Result<SyncTeXBinding, String>),
    /// A watched file changed on disk (debounce handled by the UI monitor).
    DiskChanged(PathBuf),
    OpenFinished(Result<OpenedProject, OpenFailure>),
    /// A `pushRemote` worker finished — the new conflict list or the
    /// failure. `root` identifies the mirror so a stale result cannot
    /// land on a session opened after it.
    RemotePushFinished {
        root: PathBuf,
        result: Result<Vec<String>, String>,
    },
    /// A `pullRemote` worker finished — conflicts plus whether the local
    /// tree changed (sessions then adopt the new bytes like a disk edit).
    RemotePullFinished {
        root: PathBuf,
        result: Result<remote_core::SyncReport, String>,
    },
    /// `resolveRemoteConflict` finished — the remaining conflict paths.
    RemoteResolveFinished {
        root: PathBuf,
        keep_local: bool,
        result: Result<Vec<String>, String>,
    },
    /// `prepareRemoteBuild` failed — the build stops before the executor
    /// ran (upload error, or files changed on both sides).
    RemoteBuildPrepFailed(String),
    ActivateFinished(Result<ActivatedDocument, String>),
    AgentActivityFinished,
    /// Git Integration refresh — `Ok(None)` means the project is not a
    /// repository; `Err` means git itself could not run.
    GitRefreshed(Result<Option<GitRefresh>, String>),
    /// A git mutating op finished — `error` is its stderr on failure;
    /// `clear_commit` empties the message box on success.
    GitOpFinished {
        error: Option<String>,
        clear_commit: bool,
    },
    /// `suggestCommitMessage()` — the one-shot `pi --print` run returned a
    /// drafted message (Ok) or an error string for the panel's error line.
    GitSuggestFinished(Result<String, String>),
    /// `openGitDiff`/`openGitWorkingDiff` — the diff payload arrived; `id`
    /// is the stale-result token matching `GitDiff.id`, `root` the repo
    /// the command ran in.
    GitDiffLoaded {
        id: u64,
        root: String,
        result: Result<Vec<git_core::GitDiffFileSection>, String>,
    },
    /// `toggleGitCommit`'s `commit_files_args` result for one graph row.
    GitCommitFilesLoaded {
        hash: String,
        root: String,
        files: Vec<git_core::GitCommitFile>,
    },
}

/// One `refreshGit()` payload — status + branches + log collected off-thread.
pub struct GitRefresh {
    pub status: Arc<GitStatus>,
    pub commits: Vec<GitCommit>,
    pub branches: Vec<String>,
}

/// `GitDiffSession` — a diff covering the editor area. `sections` stays
/// `None` while the worker runs; `error` carries a failed git call.
pub struct GitDiff {
    /// Stale-result token — the dispatch drops payloads for older ids.
    pub id: u64,
    pub source: GitDiffSource,
    /// `file != nil` → single-file session (the header shows its badge +
    /// path instead of the "N files" caption).
    pub file: Option<git_core::GitCommitFile>,
    pub sections: Option<Vec<git_core::GitDiffFileSection>>,
    pub error: Option<String>,
}

/// What a `GitDiff` session is looking at (`GitDiffSession.Source`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GitDiffSource {
    Commit(git_core::GitCommit),
    WorkingTree { staged: bool },
}
/// What one serialized session write did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SaveResult {
    /// The session moved on while the write waited (already clean, or no
    /// longer conflicted) — nothing was written.
    Skipped,
    Saved,
    /// The disk no longer held the expected baseline; a save conflict was
    /// recorded on the session.
    Conflict,
    PermissionFailure(String),
    InterruptedWrite(String),
    /// The write landed but the session refused the commit.
    CommitFailed(String),
}

/// Per-file write serialization for off-main saves, plus own-write
/// recognition for the file watcher — the macOS `pendingWrites` chain and
/// `lastOwnWrite`. Shared between the model and its save workers.
#[derive(Default)]
struct SessionWrites {
    locks: std::sync::Mutex<HashMap<NormalizedRelativePath, Arc<std::sync::Mutex<()>>>>,
    /// Hash of the bytes each file's newest successful write put on disk.
    own: std::sync::Mutex<HashMap<NormalizedRelativePath, DiskContentHash>>,
}

impl SessionWrites {
    fn own(&self) -> std::sync::MutexGuard<'_, HashMap<NormalizedRelativePath, DiskContentHash>> {
        self.own.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Writes `session` to `url` once the file's previous write — commit
    /// included — has finished. The session is read after waiting, so a
    /// queued save always writes the newest text with the baseline the
    /// previous write left; the commit goes by the revision written, so
    /// typing during the write cannot turn into a false conflict.
    /// `resolving` is the keep-mine path: it writes a conflicted session
    /// against the observed disk hash and clears that conflict.
    fn write(&self, session: &DocumentSession, url: &Path, resolving: bool) -> SaveResult {
        let lock = self
            .locks
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .entry(session.path().clone())
            .or_default()
            .clone();
        let _serialized = lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        // A remote project's sync never replaces this file between the
        // disk check and the rename — the gate ticket makes the
        // check-and-write one step (`MirrorWrites` in RemoteSupport.swift).
        let _write_gate = remote_core::MirrorWrites::shared().ticket();
        let snapshot = session.snapshot();
        let wanted = if resolving {
            DocumentSaveState::Conflicted
        } else {
            DocumentSaveState::Dirty
        };
        if snapshot.save_state != wanted {
            return SaveResult::Skipped;
        }
        let baseline = match (&snapshot.conflict, resolving) {
            (
                Some(
                    DocumentConflict::ExternalModification { observed_disk, .. }
                    | DocumentConflict::SaveCollision { observed_disk, .. },
                ),
                true,
            ) => *observed_disk,
            _ => snapshot.disk_baseline_hash,
        };
        // Recorded before the rename so the watcher event it causes is
        // recognized; restored if the disk never received these bytes.
        let previous_own = self.own().insert(session.path().clone(), snapshot.content_hash);
        let restore_own = || {
            let mut own = self.own();
            match previous_own {
                Some(hash) => own.insert(session.path().clone(), hash),
                None => own.remove(session.path()),
            };
        };
        match AtomicDocumentStore::new().save(&snapshot.text, url, Some(baseline)) {
            DocumentSaveOutcome::Saved(document) => {
                let resolving = if resolving { snapshot.conflict } else { None };
                match session.commit_written_save(document.hash, snapshot.revision, resolving) {
                    Ok(_) => SaveResult::Saved,
                    Err(e) => SaveResult::CommitFailed(e.to_string()),
                }
            }
            DocumentSaveOutcome::StaleBaseline(conflict) => {
                restore_own();
                let observed = conflict
                    .observed_disk
                    .map(|d| d.hash)
                    .unwrap_or_else(|| DiskContentHash::hashing(""));
                let _ = session.apply_text_neutral(DocumentMutation::RecordSaveConflict {
                    observed_disk_hash: observed,
                });
                SaveResult::Conflict
            }
            DocumentSaveOutcome::PermissionFailure { path } => {
                restore_own();
                SaveResult::PermissionFailure(path)
            }
            DocumentSaveOutcome::InterruptedWrite { path } => {
                restore_own();
                SaveResult::InterruptedWrite(path)
            }
        }
    }
}

impl std::fmt::Debug for WorkspaceMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SaveFinished { path, result } => write!(f, "SaveFinished({}, {result:?})", path.raw_value()),
            Self::PdfLoaded { .. } => write!(f, "PdfLoaded"),
            Self::PdfRendered { .. } => write!(f, "PdfRendered"),
            Self::BuildEvent(_) => write!(f, "BuildEvent"),
            Self::BuildFinished { .. } => write!(f, "BuildFinished"),
            Self::ForwardResult(_) => write!(f, "ForwardResult"),
            Self::InverseResult(_) => write!(f, "InverseResult"),
            Self::BindingRefreshed(_) => write!(f, "BindingRefreshed"),
            Self::DiskChanged(p) => write!(f, "DiskChanged({p:?})"),
            Self::OpenFinished(_) => write!(f, "OpenFinished"),
            Self::RemotePushFinished { .. } => write!(f, "RemotePushFinished"),
            Self::RemotePullFinished { .. } => write!(f, "RemotePullFinished"),
            Self::RemoteResolveFinished { .. } => write!(f, "RemoteResolveFinished"),
            Self::RemoteBuildPrepFailed(_) => write!(f, "RemoteBuildPrepFailed"),
            Self::ActivateFinished(_) => write!(f, "ActivateFinished"),
            Self::AgentActivityFinished => write!(f, "AgentActivityFinished"),
            Self::GitRefreshed(_) => write!(f, "GitRefreshed"),
            Self::GitOpFinished { .. } => write!(f, "GitOpFinished"),
            Self::GitSuggestFinished(_) => write!(f, "GitSuggestFinished"),
            Self::GitDiffLoaded { .. } => write!(f, "GitDiffLoaded"),
            Self::GitCommitFilesLoaded { .. } => write!(f, "GitCommitFilesLoaded"),
        }
    }
}

/// Why `open` failed. `RemoteOpen` carries the device name and error
/// separately so the dispatch can format `remote.error.open` with the
/// window's language.
pub enum OpenFailure {
    Error(String),
    RemoteOpen { device: String, error: String },
}
impl From<WorkspaceOpenError> for OpenFailure {
    fn from(e: WorkspaceOpenError) -> Self {
        Self::Error(e.to_string())
    }
}

/// Payload for `open` completed off-thread. Everything expensive —
/// build-target resolution (lexes every project file), bibliography
/// parsing, the built-PDF read — is computed on the worker so the main
/// thread only applies results; a synchronous `apply_open` froze the UI
/// for the whole scan while holding `state.borrow_mut()`.
pub struct OpenedProject {
    pub root: PathBuf,
    pub selected: PathBuf,
    pub files: Vec<PathBuf>,
    pub initial_url: PathBuf,
    pub session: DocumentSession,
    /// Main-document resolution for the initially selected file.
    pub resolution: (Result<Option<PathBuf>, ResolutionError>, project_feature::DocumentProject),
    pub bibliography_items: Vec<BibliographyItem>,
    /// Project-wide `\label` scan — seeded into `label_scan_cache` so the
    /// first `refresh_structure` never re-reads the tree on the main thread.
    pub label_scan: LabelScanCache,
    /// Bytes of the resolved main's PDF when it exists on disk.
    pub built_pdf: Option<Vec<u8>>,
    /// `beginRemoteSession`'s workspace — `Some` when `selected` lies in
    /// a remote mirror and the device answered (or a cache exists).
    pub remote: Option<Box<crate::remote::RemoteWorkspace>>,
}
/// Payload for `activate` completed off-thread — same off-thread contract
/// as `OpenedProject`.
pub struct ActivatedDocument {
    pub url: PathBuf,
    pub session: DocumentSession,
    pub resolution: (Result<Option<PathBuf>, ResolutionError>, project_feature::DocumentProject),
    pub bibliography_items: Vec<BibliographyItem>,
    pub label_scan: LabelScanCache,
    pub built_pdf: Option<Vec<u8>>,
}

/// mtime-keyed per-file `\label` scan — populated off-thread on
/// open/activate, incrementally maintained by `refresh_project_labels`.
pub type LabelScanCache =
    HashMap<PathBuf, (Option<std::time::SystemTime>, Vec<String>)>;

/// Project identity plus file metadata; unchanged bibliographies need no reads.
type BibliographyCacheKey = (
    Option<PathBuf>,
    Vec<(PathBuf, Option<(std::time::SystemTime, u64)>)>,
);

// ─── TeXProjectResolver (BuildSupport.swift port) ────────────────────────────

/// Resolves ordinary TeX inclusion trees without executing TeX or guessing by
/// basename. The existing language parser excludes comments/verbatim examples.
pub struct TeXProjectResolver {
    snapshots: HashMap<PathBuf, LanguageFileSnapshot>,
    /// Unsaved text of the active document, so freshly typed root directives
    /// resolve before the session is persisted.
    pub active_text: Option<(PathBuf, String)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolutionError {
    InvalidRoot,
    Ambiguous,
}
impl std::fmt::Display for ResolutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidRoot => write!(
                f,
                "The TeX root directive points to a missing file or forms a cycle. Check % !TeX root."
            ),
            Self::Ambiguous => write!(
                f,
                "More than one main TeX document uses this file. Open the intended main document and pin it as the build target."
            ),
        }
    }
}
impl std::error::Error for ResolutionError {}

impl TeXProjectResolver {
    pub fn new() -> Self {
        Self {
            snapshots: HashMap::new(),
            active_text: None,
        }
    }

    fn canonical(url: &Path) -> PathBuf {
        standardize(url.to_path_buf())
    }

    fn dialect_for(url: &Path) -> TeXDialect {
        if url
            .extension()
            .map(|e| e.eq_ignore_ascii_case("bib"))
            .unwrap_or(false)
        {
            TeXDialect::Bibtex
        } else {
            TeXDialect::Latex
        }
    }

    fn snapshot(&mut self, url: &Path) -> Option<&LanguageFileSnapshot> {
        let file = Self::canonical(url);
        if self.snapshots.contains_key(&file) {
            return self.snapshots.get(&file);
        }
        let text = self
            .active_text
            .as_ref()
            .filter(|(u, _)| Self::canonical(u) == file)
            .map(|(_, t)| t.clone())
            .or_else(|| std::fs::read_to_string(&file).ok())?;
        let parsed = LanguageFileSnapshot::new(
            &file.to_string_lossy(),
            0,
            &text,
            Self::dialect_for(&file),
        )
        .ok()?;
        self.snapshots.insert(file.clone(), parsed);
        self.snapshots.get(&file)
    }

    /// `captures` — every regex group-1 match, whitespace-trimmed.
    /// Patterns compile once per thread — `root_hint` calls this per file,
    /// so per-call `Regex::new` was O(files) compiles per resolve.
    fn captures(pattern: &'static str, text: &str) -> Vec<String> {
        thread_local! {
            static CACHE: RefCell<HashMap<&'static str, regex::Regex>> =
                RefCell::new(HashMap::new());
        }
        CACHE.with(|cache| {
            let mut cache = cache.borrow_mut();
            let re = cache
                .entry(pattern)
                .or_insert_with(|| regex::Regex::new(pattern).unwrap());
            re.captures_iter(text)
                .filter_map(|c| c.get(1).map(|m| m.as_str().trim().to_string()))
                .collect()
        })
    }

    fn root_hint(&mut self, url: &Path) -> Option<PathBuf> {
        let source = self.snapshot(url)?.source.clone();
        let mut hints =
            Self::captures(r"(?im)^\s*%\s*!\s*tex\s+root\s*=\s*(.+)$", &source);
        hints.extend(Self::captures(
            r"\\documentclass\s*\[([^\]]+)\]\s*\{subfiles\}",
            &source,
        ));
        let mut hint = hints.into_iter().next()?;
        if hint.len() >= 2 && hint.starts_with('"') && hint.ends_with('"') {
            hint = hint[1..hint.len() - 1].to_string();
        }
        if Path::new(&hint).extension().is_none() {
            hint += ".tex";
        }
        Some(Self::canonical(&url.parent()?.join(&hint)))
    }

    fn declared_root(&mut self, url: &Path) -> Result<Option<PathBuf>, ResolutionError> {
        let mut file = Self::canonical(url);
        let mut seen: HashSet<PathBuf> = [file.clone()].into_iter().collect();
        let mut has_hint = false;
        while let Some(next) = self.root_hint(&file) {
            has_hint = true;
            if !seen.insert(next.clone()) || !is_readable_file(&next) {
                return Err(ResolutionError::InvalidRoot);
            }
            file = next;
        }
        Ok(has_hint.then_some(file))
    }

    fn is_main(&mut self, url: &Path) -> bool {
        if url
            .extension()
            .map(|e| !e.eq_ignore_ascii_case("tex"))
            .unwrap_or(true)
        {
            return false;
        }
        self.snapshot(url)
            .map(|s| {
                s.tokens.iter().any(|t| {
                    matches!(&t.kind, LanguageTokenKind::ControlSequence(name) if name == "documentclass")
                })
            })
            .unwrap_or(false)
    }

    /// `directLinks` — include/bibliography targets of one file, resolved
    /// against the main document's directory first, then the including
    /// file's. Only existing files are returned, canonicalized.
    fn direct_links(&mut self, file: &Path, base: &Path, graphics_paths: Option<&[String]>) -> Vec<PathBuf> {
        let Some(parsed) = self.snapshot(file).cloned() else {
            return Vec::new();
        };
        let mut links: Vec<(String, &'static str)> = parsed
            .includes
            .iter()
            .map(|i| (i.target.clone(), "tex"))
            .collect();
        // Remove lexer-recognized comments before scanning the resource
        // commands not yet represented by LanguageFileSnapshot.includes.
        let mut source = parsed.source.clone();
        for token in parsed.tokens.iter().rev() {
            if matches!(token.kind, LanguageTokenKind::Comment(_)) {
                let start = token.range.utf8_offset.max(0) as usize;
                let end = start + token.range.utf8_length.max(0) as usize;
                source.replace_range(start..end, "");
            }
        }
        links.extend(
            Self::captures(r"\\subfile\s*\{([^}]+)\}", &source)
                .into_iter()
                .map(|m| (m, "tex")),
        );
        for group in Self::captures(r"\\bibliography\s*\{([^}]+)\}", &source) {
            links.extend(
                group
                    .split(',')
                    .map(|s| (s.trim().to_string(), "bib")),
            );
        }
        links.extend(
            Self::captures(r"\\addbibresource(?:\s*\[[^\]]*\])?\s*\{([^}]+)\}", &source)
                .into_iter()
                .map(|m| (m, "bib")),
        );
        let mut targets = Vec::new();
        for (name, ext) in links {
            let path = if Path::new(&name).extension().is_none() {
                format!("{name}.{ext}")
            } else {
                name
            };
            // TeX resolves nested \input paths from the main document's
            // working directory. subfiles can additionally use local paths.
            let candidates = [
                base.join(&path),
                file.parent()
                    .map(|p| p.join(&path))
                    .unwrap_or_default(),
            ];
            if let Some(target) = candidates.iter().find(|c| is_readable_file(c)) {
                let target = Self::canonical(target);
                if !targets.contains(&target) {
                    targets.push(target);
                }
            }
        }
        if let Some(graphics_paths) = graphics_paths {
            let mut directories = vec![base.to_path_buf(), file.parent().unwrap_or(base).to_path_buf()];
            directories.extend(graphics_paths.iter().map(|p| base.join(p)));
            directories.extend(Self::graphic_paths(&source).iter().map(|p| base.join(p)));
            for name in Self::captures(r"\\includegraphics\*?(?:\s*\[[^\]]*\])?\s*\{([^}]+)\}", &source) {
                let names: Vec<_> = if Path::new(&name).extension().is_none() {
                    ["pdf", "png", "jpg", "jpeg", "eps", "svg"].iter().map(|ext| format!("{name}.{ext}")).collect()
                } else { vec![name] };
                let target = directories.iter().flat_map(|dir| names.iter().map(move |name| dir.join(name)))
                    .find(|p| is_readable_file(p));
                if let Some(target) = target {
                    let target = Self::canonical(&target);
                    if !targets.contains(&target) { targets.push(target); }
                }
            }
        }
        targets
    }

    fn graphic_paths(source: &str) -> Vec<String> {
        Self::captures(r"\\graphicspath\s*\{((?:\s*\{[^}]*\}\s*)*)\}", source).iter()
            .flat_map(|group| Self::captures(r"\{([^}]+)\}", group)).collect()
    }

    pub fn document_project(&mut self, active: Option<&Path>, main: Option<&Path>, root: Option<&Path>, files: &[PathBuf]) -> project_feature::DocumentProject {
        let (Some(active), Some(root)) = (active, root) else { return Default::default() };
        let markdown = active.extension().map(|e| e.eq_ignore_ascii_case("md") || e.eq_ignore_ascii_case("markdown")).unwrap_or(false);
        let main = Self::canonical(if markdown { active } else { main.unwrap_or(active) });
        let base = main.parent().unwrap_or(root);
        let relative = |p: &Path| p.strip_prefix(root).ok().map(|p| p.to_string_lossy().replace('\\', "/"));
        let Some(main_path) = relative(&main) else { return Default::default() };
        let paths: Vec<_> = files.iter().filter_map(|p| relative(p)).collect();
        let available: HashSet<_> = files.iter().map(|p| Self::canonical(p)).collect();
        let source = self.snapshot(&main).map(|s| s.source.clone()).unwrap_or_default();
        let graphics_paths = Self::graphic_paths(&source);
        let mut links = HashMap::new();
        let mut seen = HashSet::new();
        let mut pending = vec![main.clone()];
        while let Some(file) = pending.pop() {
            if !seen.insert(file.clone()) || !file.extension().map(|e| e.eq_ignore_ascii_case("tex")).unwrap_or(false) { continue; }
            let Some(path) = relative(&file) else { continue };
            let children: Vec<_> = self.direct_links(&file, base, Some(&graphics_paths)).into_iter()
                .filter(|p| available.contains(p)).collect();
            links.insert(path, children.iter().filter_map(|p| relative(p)).collect());
            pending.extend(children);
        }
        project_feature::build_document_project(&main_path, &paths, &links)
    }

    /// `directDependencies` — the main document's own bibliography and
    /// included files, one level only; the sidebar nests these under it.
    pub fn direct_dependencies(&mut self, main: &Path) -> Vec<PathBuf> {
        let file = Self::canonical(main);
        if file
            .extension()
            .map(|e| !e.eq_ignore_ascii_case("tex"))
            .unwrap_or(true)
        {
            return Vec::new();
        }
        let base = file.parent().map(Path::to_path_buf).unwrap_or_default();
        self.direct_links(&file, &base, None)
    }

    pub fn bibliography_files(&mut self, main: Option<&Path>, files: &[PathBuf]) -> Vec<PathBuf> {
        let mut linked: Vec<_> = main.map(|m| self.dependencies(m)).unwrap_or_default().into_iter()
            .filter(|p| p.extension().map(|e| e.eq_ignore_ascii_case("bib")).unwrap_or(false)).collect();
        if !linked.is_empty() { linked.sort(); return linked; }
        files.iter().filter(|p| p.extension().map(|e| e.eq_ignore_ascii_case("bib")).unwrap_or(false)
            && main.map(|m| m.parent() == p.parent()).unwrap_or(true)).cloned().collect()
    }

    fn dependencies(&mut self, main: &Path) -> HashSet<PathBuf> {
        let base = main.parent().map(Path::to_path_buf).unwrap_or_default();
        let mut visited = HashSet::new();
        let mut pending = vec![Self::canonical(main)];
        while let Some(file) = pending.pop() {
            if !visited.insert(file.clone()) {
                continue;
            }
            if file
                .extension()
                .map(|e| !e.eq_ignore_ascii_case("tex"))
                .unwrap_or(true)
            {
                continue;
            }
            pending.extend(self.direct_links(&file, &base, None));
        }
        visited
    }

    pub fn resolve(
        &mut self,
        active: Option<&Path>,
        files: &[PathBuf],
        preferred: Option<&Path>,
    ) -> Result<Option<PathBuf>, ResolutionError> {
        if let Some(active) = active {
            if let Some(declared) = self.declared_root(active)? {
                return Ok(Some(declared));
            }
            if self.is_main(active) {
                return Ok(Some(Self::canonical(active)));
            }
        }
        let mut mains = Vec::new();
        for file in files {
            if self.is_main(file) {
                mains.push(Self::canonical(file));
            }
        }
        if let Some(active) = active {
            if active
                .extension()
                .map(|e| e.eq_ignore_ascii_case("bbl"))
                .unwrap_or(false)
            {
                let stem = Self::canonical(active).with_extension("");
                if let Some(main) = mains.iter().find(|m| m.with_extension("") == stem) {
                    return Ok(Some(main.clone()));
                }
            }
        }
        let mut owners = Vec::new();
        if let Some(file) = active {
            let canon = Self::canonical(file);
            for main in &mains {
                if self.dependencies(main).contains(&canon) {
                    owners.push(main.clone());
                }
            }
        }
        if let Some(preferred) = preferred {
            let canon = Self::canonical(preferred);
            if owners.contains(&canon) {
                return Ok(Some(canon));
            }
        }
        if owners.len() == 1 {
            return Ok(Some(owners[0].clone()));
        }
        if owners.is_empty() && mains.len() == 1 {
            return Ok(Some(mains[0].clone()));
        }
        if mains.len() > 1 {
            return Err(ResolutionError::Ambiguous);
        }
        Ok(None)
    }

    pub fn initial_document(&mut self, files: &[PathBuf]) -> Option<PathBuf> {
        let mut mains = Vec::new();
        for file in files {
            if self.is_main(file) {
                mains.push(file.clone());
            }
        }
        mains
            .iter()
            .find(|m| {
                m.file_name()
                    .map(|n| n.to_string_lossy().eq_ignore_ascii_case("main.tex"))
                    .unwrap_or(false)
            })
            .cloned()
            .or_else(|| mains.into_iter().next())
            // The last-ditch fallback stays textual — .tex first like
            // before, then .bib — now that figures share the project list
            // and can never open in the editor.
            .or_else(|| files.iter().find(|f| WorkspaceModel::is_tex(f)).cloned())
            .or_else(|| files.iter().find(|f| WorkspaceModel::is_source_file(f)).cloned())
    }

    /// Opening a chapter directly still opens its owning project. Parent
    /// directories are inspected shallowly and only a proven include edge or
    /// explicit root directive can expand the workspace beyond that directory.
    pub fn project_root(&mut self, selected: &Path) -> PathBuf {
        let mut main = self
            .declared_root(selected)
            .ok()
            .flatten()
            .or_else(|| {
                if self.is_main(selected) {
                    Some(Self::canonical(selected))
                } else {
                    None
                }
            });
        let mut directory = selected
            .parent()
            .map(|p| standardize(p.to_path_buf()))
            .unwrap_or_default();
        if main.is_none() {
            let canon_selected = Self::canonical(selected);
            while directory != Path::new("/") {
                let files: Vec<PathBuf> = std::fs::read_dir(&directory)
                    .map(|rd| {
                        rd.flatten()
                            .map(|e| e.path())
                            .filter(|p| {
                                !p.file_name()
                                    .map(|n| n.to_string_lossy().starts_with('.'))
                                    .unwrap_or(false)
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                let mut owner = None;
                for file in files {
                    if self.is_main(&file) && self.dependencies(&file).contains(&canon_selected) {
                        owner = Some(file);
                        break;
                    }
                }
                if let Some(owner) = owner {
                    main = Some(Self::canonical(&owner));
                    break;
                }
                let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/nonexistent"));
                if directory == home || directory.join(".git").exists() {
                    break;
                }
                if !directory.pop() {
                    break;
                }
            }
        }
        let Some(main) = main else {
            return selected
                .parent()
                .map(|p| standardize(p.to_path_buf()))
                .unwrap_or_default();
        };
        let mut root = Self::canonical(&main)
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_default();
        let mut scope = self.dependencies(&main);
        scope.insert(Self::canonical(selected));
        for file in scope {
            while !file.starts_with(&root) && root != Path::new("/") {
                if !root.pop() {
                    break;
                }
            }
        }
        root
    }
}

fn is_readable_file(path: &Path) -> bool {
    std::fs::File::open(path).is_ok()
}

// ─── WorkspaceModel ─────────────────────────────────────────────────────────

pub struct WorkspaceModel {
    pub phase: WorkspacePhase,
    pub project_url: Option<PathBuf>,
    pub project_files: Vec<PathBuf>,
    pub document_project: project_feature::DocumentProject,
    /// Relative dir paths the user collapsed in the project tree. Entries
    /// for dirs that disappear on rescan are simply ignored.
    pub collapsed_project_dirs: std::collections::HashSet<String>,
    pub open_documents: Vec<PathBuf>,
    pub active_document_url: Option<PathBuf>,
    pub document_snapshot: Option<DocumentSnapshot>,
    pub build_log_text: String,
    pub build_issues: Vec<BuildIssueRecord>,
    pub build_state: WorkspaceBuildState,
    pub synctex_state: WorkspaceSyncTeXState,
    pub showing_settings: bool,

    pub registry: std::sync::Arc<DocumentSessionRegistry>,
    pub build_orchestrator: Arc<BuildOrchestrator<WorkspaceBuildExecutor>>,
    /// Shares the executor inside `build_orchestrator` — `use_remote`
    /// switches routing without rebuilding the orchestrator.
    pub workspace_executor: WorkspaceBuildExecutor,
    /// `remote` — the SSH session while a remote (mirrored) project is
    /// open (`RemoteWorkspace` in RemoteSupport.swift).
    pub remote: Option<crate::remote::RemoteWorkspace>,
    /// `buildCancelRequested` — Cancel pressed while a remote build was
    /// still in its upload/prep phase (nothing to cancel yet). Shared
    /// with the build thread so the flag lands mid-upload.
    pub build_cancel_requested: Arc<std::sync::atomic::AtomicBool>,
    pub synctex_runner: Arc<SyncTeXRunner>,
    pub registered_sessions: Vec<DocumentSession>,
    pub active_build_id: Option<BuildID>,
    pub latest_built_pdf_name: Option<String>,
    pub synctex_binding: Option<SyncTeXBinding>,

    pub console_section: ConsoleSection,
    pub sidebar_section: SidebarSection,
    pub sidebar_visible: bool,
    pub bottom_panel_visible: bool,
    pub inspector_visible: bool,
    pub pinned_build_target: Option<PathBuf>,
    /// Main document inferred by `TeXProjectResolver` for the active file.
    pub automatic_build_target: Option<PathBuf>,
    /// Why no automatic target exists — surfaced on build failure/tooltips.
    pub build_target_message: Option<String>,
    pub recent_documents: Vec<PathBuf>,
    pub build_command_text: String,
    pub custom_command_text: String,

    pub outline_items: Vec<DocumentOutlineItem>,
    pub label_items: Vec<DocumentLabelItem>,
    pub bibliography_items: Vec<BibliographyItem>,
    pub todo_items: Vec<DocumentTodoItem>,
    /// Git Integration panel state (`gitStatus`/`gitCommits`/`gitBranches`
    /// on the Swift model) — `None` until the first refresh reports.
    pub git_status: Option<Arc<GitStatus>>,
    pub git_commits: Vec<GitCommit>,
    pub git_branches: Vec<String>,
    pub git_commit_message: String,
    pub git_busy: bool,
    /// `gitSuggestBusy` — a pi subprocess, not a git op, so pull/push stay
    /// enabled while a suggestion runs.
    pub git_suggest_busy: bool,
    pub git_error: Option<String>,
    /// `gitDiff` — the diff session covering the editor area; the PDF
    /// inspector hides while this is set.
    pub git_diff: Option<GitDiff>,
    /// `gitExpandedCommits`/`gitCommitFiles`/`gitCommitFilesBusy` — graph
    /// rows expanded to list their changed files, and the lazy per-commit
    /// file cache filled by `toggleGitCommit`.
    pub git_expanded_commits: std::collections::HashSet<String>,
    pub git_commit_files: HashMap<String, Vec<git_core::GitCommitFile>>,
    pub git_commit_files_busy: std::collections::HashSet<String>,
    /// `refreshGit` in-flight guard + the project root the poll was
    /// issued for — a result landing for a stale root re-queues.
    pub(crate) git_refresh_pending: Cell<bool>,
    pub(crate) git_refresh_root: RefCell<Option<PathBuf>>,
    /// `gitDiff` session counter — the `GitDiffLoaded` stale-result token.
    pub(crate) git_diff_seq: Cell<u64>,
    /// `todoCache` — per-file (mtime | snapshot-revision, items) entries so
    /// a per-keystroke refresh only re-parses the file that changed.
    todo_cache: HashMap<PathBuf, ((u128, u64), Vec<DocumentTodoItem>)>,
    /// Merged `\label` keys from every project .tex plus the active
    /// document's unsaved text — feeds `\ref` completion candidates.
    pub project_label_keys: BTreeSet<String>,
    /// Merged `.bib` entry keys — feeds `\cite` completion candidates.
    pub citation_keys: BTreeSet<String>,
    /// Per-file `\label` scan cache keyed on mtime — a structure refresh
    /// re-reads only files that changed since the last scan.
    label_scan_cache: LabelScanCache,
    bibliography_cache_key: Option<BibliographyCacheKey>,
    /// Bumped by `refresh_structure_with` — the sidebar rebuilds its lists
    /// only when this changes instead of on every refresh_sidebar call.
    pub structure_revision: u64,
    cached_word_count: usize,
    /// Bumped when project files, selection, folders, or build-target badges
    /// change — the sidebar's file tree rebuilds only on a bump.
    pub files_revision: u64,

    /// Linux has no sandbox lease — true whenever the filesystem allows it.
    pub read_write: bool,
    /// `capabilityBroker` — the platform capability source (Linux canonical-
    /// path broker), issued on open and released on close like the Swift
    /// security-scoped bookmark flow.
    pub files: PlatformFileCapabilityBroker,
    /// `capabilityLease` — held while a project is open.
    pub capability_lease: Option<app_ports::FileAccessLease>,
    /// Current editor selection in UTF-16 offsets, mirrored from the adapter.
    pub editor_selection: (usize, usize),

    event_sink: Option<Sender<WorkspaceMessage>>,
    /// Save serialization + own-write hashes shared with save workers.
    writes: Arc<SessionWrites>,
    /// Save workers still writing — `close()` and quit wait for them.
    pending_writes: Vec<std::thread::JoinHandle<()>>,
    /// Jump-to-line requested by inverse sync while activation was in flight.
    pending_jump: Option<(usize, usize)>,
    did_restore_session: bool,

    // UI callbacks (fired on the main thread by the owning window).
    /// The canonical snapshot changed — UI pushes it into the editor buffer.
    pub on_snapshot_changed: Option<Box<dyn FnMut(&DocumentSnapshot)>>,
    /// The active document changed — UI rebinds adapter/watcher/tab strip.
    pub on_active_document_changed: Option<Box<dyn FnMut()>>,
    /// Forward sync produced a PDF highlight (page, x, y, w, h in PDF pts).
    pub on_synctex_highlight: Option<Box<dyn FnMut(i64, f64, f64, f64, f64)>>,
    /// Inverse sync requests caret placement (1-based line, column).
    pub on_jump_to: Option<Box<dyn FnMut(usize, usize)>>,
    /// Structure/outline/labels/bibliography changed.
    pub on_structure_changed: Option<Box<dyn FnMut()>>,
    /// Autosave timer should be (re)scheduled or cancelled.
    pub on_autosave_schedule: Option<Box<dyn FnMut()>>,
    /// Selection attachment for the agent (path, start, end, text).
    pub on_selection_attachment: Option<Box<dyn FnMut(Option<(String, usize, usize, String)>)>>,
    /// Text for the terminal feed (status notices), ANSI allowed.
    pub on_terminal_feed: Option<Box<dyn FnMut(String)>>,
    /// Command line for the terminal (as if typed + Return).
    pub on_terminal_send: Option<Box<dyn FnMut(String)>>,
    /// `schedulePush` — a save landed in a remote mirror; the UI schedules
    /// the debounced upload (glib owns the timer).
    pub on_remote_push_schedule: Option<Box<dyn FnMut()>>,
}

impl WorkspaceModel {
    const RECENTS_KEY: &'static str = "pitex.pref.workspace.recentDocuments";

    pub fn new() -> Self {
        let workspace_executor = WorkspaceBuildExecutor::new();
        Self {
            phase: WorkspacePhase::NoProject,
            project_url: None,
            project_files: Vec::new(),
            document_project: Default::default(),
            collapsed_project_dirs: std::collections::HashSet::new(),
            open_documents: Vec::new(),
            active_document_url: None,
            document_snapshot: None,
            build_log_text: String::new(),
            build_issues: Vec::new(),
            build_state: WorkspaceBuildState::Unavailable(
                WorkspaceBuildError::NoActiveDocument.to_string(),
            ),
            synctex_state: WorkspaceSyncTeXState::Unavailable(
                "SyncTeX is unavailable until a successful build produces matching metadata."
                    .into(),
            ),
            showing_settings: false,
            registry: std::sync::Arc::new(DocumentSessionRegistry::new()),
            build_orchestrator: Arc::new(BuildOrchestrator::new(workspace_executor.clone())),
            workspace_executor,
            remote: None,
            build_cancel_requested: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            synctex_runner: Arc::new(SyncTeXRunner::new()),
            registered_sessions: Vec::new(),
            active_build_id: None,
            latest_built_pdf_name: None,
            synctex_binding: None,
            console_section: ConsoleSection::Assistant,
            sidebar_section: SidebarSection::Outline,
            sidebar_visible: true,
            bottom_panel_visible: false,
            inspector_visible: true,
            pinned_build_target: None,
            automatic_build_target: None,
            build_target_message: None,
            recent_documents: Vec::new(),
            build_command_text: "xelatex -interaction=nonstopmode -synctex=1 {file}".into(),
            custom_command_text: String::new(),
            outline_items: Vec::new(),
            label_items: Vec::new(),
            bibliography_items: Vec::new(),
            todo_items: Vec::new(),
            git_status: None,
            git_commits: Vec::new(),
            git_branches: Vec::new(),
            git_commit_message: String::new(),
            git_busy: false,
            git_suggest_busy: false,
            git_error: None,
            git_diff: None,
            git_expanded_commits: std::collections::HashSet::new(),
            git_commit_files: HashMap::new(),
            git_commit_files_busy: std::collections::HashSet::new(),
            git_refresh_pending: Cell::new(false),
            git_refresh_root: RefCell::new(None),
            git_diff_seq: Cell::new(0),
            todo_cache: HashMap::new(),
            project_label_keys: BTreeSet::new(),
            citation_keys: BTreeSet::new(),
            label_scan_cache: HashMap::new(),
            bibliography_cache_key: None,
            structure_revision: 0,
            cached_word_count: 0,
            files_revision: 0,
            read_write: true,
            files: PlatformFileCapabilityBroker::new(),
            capability_lease: None,
            editor_selection: (0, 0),
            event_sink: None,
            writes: Arc::default(),
            pending_writes: Vec::new(),
            pending_jump: None,
            did_restore_session: false,
            on_snapshot_changed: None,
            on_active_document_changed: None,
            on_synctex_highlight: None,
            on_jump_to: None,
            on_structure_changed: None,
            on_autosave_schedule: None,
            on_selection_attachment: None,
            on_terminal_feed: None,
            on_terminal_send: None,
            on_remote_push_schedule: None,
        }
    }

    pub fn set_event_sink(&mut self, sink: Sender<WorkspaceMessage>) {
        self.event_sink = Some(sink);
    }
    pub(crate) fn sink(&self) -> Option<Sender<WorkspaceMessage>> {
        self.event_sink.clone()
    }

    // ── Recents ──
    pub fn load_recents(&mut self, store: &SettingsStore) {
        self.recent_documents = store
            .prefs()
            .string_array(Self::RECENTS_KEY)
            .unwrap_or_default()
            .into_iter()
            .map(PathBuf::from)
            .collect();
    }
    fn record_recent(&mut self, store: &mut SettingsStore, url: &Path) {
        self.recent_documents.retain(|u| u != url);
        self.recent_documents.insert(0, url.to_path_buf());
        self.recent_documents.truncate(10);
        let paths: Vec<String> = self
            .recent_documents
            .iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect();
        store.prefs_mut().set(Self::RECENTS_KEY, paths);
    }
    pub fn clear_recents(&mut self, store: &mut SettingsStore) {
        self.recent_documents.clear();
        store.prefs_mut().remove(Self::RECENTS_KEY);
    }

    pub fn has_project(&self) -> bool {
        self.project_url.is_some()
    }
    pub fn can_save(&self) -> bool {
        matches!(
            self.document_snapshot.as_ref().map(|s| &s.save_state),
            Some(DocumentSaveState::Dirty)
        ) && self.read_write
    }
    pub fn save_unavailable_reason(&self) -> Option<&'static str> {
        let snapshot = self.document_snapshot.as_ref()?;
        if snapshot.save_state == DocumentSaveState::Conflicted {
            return Some("The file changed on disk. Resolve the conflict before saving.");
        }
        if snapshot.save_state == DocumentSaveState::Clean {
            return Some("The document has no unsaved changes.");
        }
        if !self.read_write {
            return Some("This project was opened read-only.");
        }
        None
    }
    pub fn build_unavailable_reason(&self) -> Option<String> {
        if self.is_building() || self.can_build() {
            return None;
        }
        if let Some(message) = &self.build_target_message {
            return Some(message.clone());
        }
        if let WorkspaceBuildState::Unavailable(reason) = &self.build_state {
            return Some(reason.clone());
        }
        Some("Open a project with a .tex source before building.".into())
    }
    pub fn can_build(&self) -> bool {
        matches!(self.phase, WorkspacePhase::Ready)
    }
    /// `isBuilding` derives from the build state, like the Swift computed var.
    pub fn is_building(&self) -> bool {
        matches!(self.build_state, WorkspaceBuildState::Building)
    }

    // ── Open / activate ──

    pub fn open(&mut self, selected: PathBuf, sink: Sender<WorkspaceMessage>) {
        self.close();
        self.phase = WorkspacePhase::Loading(selected.clone());
        let registry = self.registry.clone();
        std::thread::spawn(move || {
            let result = Self::open_worker(registry, &selected);
            let _ = sink.send(WorkspaceMessage::OpenFinished(result));
        });
    }

    fn open_worker(
        registry: std::sync::Arc<DocumentSessionRegistry>,
        selected: &Path,
    ) -> Result<OpenedProject, OpenFailure> {
        // A remote project's mirror is refreshed from the device before any
        // file is read; an unreachable device with nothing cached fails here.
        let remote = match crate::remote::begin_remote_session(selected) {
            crate::remote::RemoteOpen::NotRemote => None,
            crate::remote::RemoteOpen::Failed { device, error } => {
                return Err(OpenFailure::RemoteOpen { device, error });
            }
            crate::remote::RemoteOpen::Ready(workspace) => Some(workspace),
        };
        let is_directory = selected.is_dir();
        // A chapter opened directly still opens its owning project — the
        // resolver walks up for a proven include edge or a root directive.
        let mut resolver = TeXProjectResolver::new();
        let root = standardize(if is_directory {
            selected.to_path_buf()
        } else {
            resolver.project_root(selected)
        });
        let files = Self::discover_tex_files(&root, selected, is_directory)?;
        let initial_url = if is_directory {
            resolver
                .initial_document(&files)
                .ok_or(WorkspaceOpenError::NoTexSources)?
        } else {
            standardize(selected.to_path_buf())
        };
        let initial_text = Self::read_exact_utf8(&initial_url)?;
        let file = Self::project_file(&initial_url, &root)?;
        let session = registry
            .open(
                &root,
                &file,
                initial_text.clone(),
                Some(DiskContentHash::hashing(&initial_text)),
            )
            .map_err(|_| WorkspaceOpenError::UnreadableProject)?;
        let verified = Self::read_exact_utf8(&initial_url)?;
        if verified != initial_text {
            return Err(WorkspaceOpenError::ChangedWhileOpening.into());
        }
        let resolution = Self::resolve_build_target(
            Some(&initial_url),
            Some(initial_text),
            &files,
            None,
            Some(&root),
        );
        let built_pdf = resolution.0
            .as_ref()
            .ok()
            .and_then(|m| m.as_ref())
            .and_then(|main| Self::built_pdf_bytes(&root, main));
        let bib_files = TeXProjectResolver::new().bibliography_files(
            resolution.0.as_ref().ok().and_then(|m| m.as_deref()).or(Some(&initial_url)), &files);
        Ok(OpenedProject {
            bibliography_items: Self::parse_bibliography(&bib_files, Some(&root)),
            label_scan: Self::scan_project_labels(&files),
            resolution,
            built_pdf,
            root,
            selected: selected.to_path_buf(),
            files,
            initial_url,
            session,
            remote,
        })
    }

    /// Main-thread completion of `open`.
    pub fn apply_open(&mut self, store: &mut SettingsStore, opened: OpenedProject) {
        if let Some(workspace) = opened.remote {
            // `remote = workspace; buildExecutor.use(workspace.executor)`.
            self.workspace_executor.use_remote(Some(workspace.executor.clone()));
            self.remote = Some(*workspace);
        }
        self.project_url = Some(opened.root.clone());
        self.project_files = opened.files;
        self.files_revision += 1;
        self.registered_sessions = vec![opened.session.clone()];
        // `capabilityBroker`/`capabilityLease` (PitexApp.swift:369-383):
        // issue readWrite for the selected path, fall back to readOnly, then
        // begin access; the lease gates `canSave`/read-write state.
        let capability = self
            .files
            .issue_capability(&opened.selected, app_ports::FileCapabilityAccess::ReadWrite)
            .or_else(|_| {
                self.files.issue_capability(
                    &opened.selected,
                    app_ports::FileCapabilityAccess::ReadOnly,
                )
            });
        self.capability_lease = capability
            .ok()
            .and_then(|cap| self.files.begin_access(&cap).ok());
        self.read_write = self
            .capability_lease
            .as_ref()
            .map(|lease| lease.access == app_ports::FileCapabilityAccess::ReadWrite)
            .unwrap_or(false);
        self.active_document_url = Some(opened.initial_url.clone());
        self.open_documents = vec![opened.initial_url];
        self.document_snapshot = Some(opened.session.snapshot());
        self.apply_build_resolution(opened.resolution);
        self.build_state =
            WorkspaceBuildState::Unavailable("No build has run yet for this project.".into());
        self.synctex_state = WorkspaceSyncTeXState::Unavailable(
            "SyncTeX is unavailable until a successful build produces matching metadata.".into(),
        );
        self.phase = WorkspacePhase::Ready;
        self.restore_built_preview_with(opened.built_pdf);
        self.record_recent(store, &opened.selected);
        self.label_scan_cache = opened.label_scan;
        self.refresh_structure_with(Some(opened.bibliography_items));
        if let Some(cb) = &mut self.on_active_document_changed {
            cb();
        }
        self.emit_snapshot();
    }

    pub fn open_failed(&mut self, error: String) {
        self.close();
        self.phase = WorkspacePhase::Failed(error);
    }

    pub fn activate_document(&mut self, url: PathBuf, sink: Sender<WorkspaceMessage>) {
        if self.active_document_url.as_ref() == Some(&url) {
            return;
        }
        let Some(root) = self.project_url.clone() else { return };
        if !self.project_files.contains(&url) {
            return;
        }
        // Activating a real document dismisses a commit-diff overlay —
        // the user asked to see the document, not the diff.
        self.git_diff = None;
        // Keep the workspace mounted when switching sources: a loading phase
        // destroys the split view and PDF view, losing their size and position.
        let registry = self.registry.clone();
        let files = self.project_files.clone();
        let pinned = self.pinned_build_target.clone();
        let preferred = self.automatic_build_target.clone();
        std::thread::spawn(move || {
            let result = (|| -> Result<ActivatedDocument, WorkspaceOpenError> {
                let text = Self::read_exact_utf8(&url)?;
                let file = Self::project_file(&url, &root)?;
                let session = registry
                    .open(&root, &file, text.clone(), Some(DiskContentHash::hashing(&text)))
                    .map_err(|_| WorkspaceOpenError::UnreadableProject)?;
                let resolution = Self::resolve_build_target(
                    Some(&url),
                    Some(text),
                    &files,
                    preferred.as_deref(),
                    Some(&root),
                );
                let built_pdf = pinned
                    .as_ref()
                    .or(resolution.0.as_ref().ok().and_then(|m| m.as_ref()))
                    .and_then(|main| Self::built_pdf_bytes(&root, main));
                let bib_files = TeXProjectResolver::new().bibliography_files(
                    pinned.as_deref().or(resolution.0.as_ref().ok().and_then(|m| m.as_deref())).or(Some(&url)), &files);
                Ok(ActivatedDocument {
                    url,
                    session,
                    resolution,
                    bibliography_items: Self::parse_bibliography(&bib_files, Some(&root)),
                    label_scan: Self::scan_project_labels(&files),
                    built_pdf,
                })
            })();
            let _ = sink.send(WorkspaceMessage::ActivateFinished(
                result.map_err(|e| e.to_string()),
            ));
        });
    }

    pub fn apply_activate(&mut self, activated: ActivatedDocument) {
        if !self
            .registered_sessions
            .iter()
            .any(|s| s.same_session(&activated.session))
        {
            self.registered_sessions.push(activated.session.clone());
        }
        self.active_document_url = Some(activated.url.clone());
        self.files_revision += 1; // Refresh the active row even when its build target is unchanged.
        if !self.open_documents.contains(&activated.url) {
            self.open_documents.push(activated.url);
        }
        self.document_snapshot = Some(activated.session.snapshot());
        self.apply_build_resolution(activated.resolution);
        self.phase = WorkspacePhase::Ready;
        self.restore_built_preview_with(activated.built_pdf);
        self.label_scan_cache = activated.label_scan;
        self.refresh_structure_with(Some(activated.bibliography_items));
        if let Some(cb) = &mut self.on_active_document_changed {
            cb();
        }
        self.emit_snapshot();
        // Deferred jump from inverse SyncTeX.
        if let Some((line, column)) = self.pending_jump.take() {
            if let Some(cb) = &mut self.on_jump_to {
                cb(line, column);
            }
        }
    }

    // ── Save / conflict ──

    pub fn save(&mut self) {
        if !self.can_save() {
            return;
        }
        let (Some(url), Some(snapshot)) = (
            self.active_document_url.clone(),
            self.document_snapshot.as_ref(),
        ) else {
            return;
        };
        let Some(session) = self
            .registered_sessions
            .iter()
            .find(|s| s.path() == &snapshot.path)
            .cloned()
        else {
            return;
        };
        let path = snapshot.path.clone();
        let writes = self.writes.clone();
        // The write (two fsyncs) runs off the main thread; the result comes
        // back through `apply_save_finished`. Without an event loop (tests)
        // it runs inline.
        match self.sink() {
            Some(sink) => {
                self.pending_writes.retain(|handle| !handle.is_finished());
                self.pending_writes.push(std::thread::spawn(move || {
                    let result = writes.write(&session, &url, false);
                    let _ = sink.send(WorkspaceMessage::SaveFinished { path, result });
                }));
            }
            None => {
                let result = writes.write(&session, &url, false);
                self.apply_save_finished(&path, result);
            }
        }
    }

    /// Publishes a finished save: the session already holds the committed
    /// state (edits typed during the write included), so the active
    /// document simply re-reads it.
    pub fn apply_save_finished(&mut self, path: &NormalizedRelativePath, result: SaveResult) {
        let Some(session) = self.registered_sessions.iter().find(|s| s.path() == path).cloned()
        else {
            return;
        };
        if self.document_snapshot.as_ref().is_some_and(|s| &s.path == path) {
            self.set_snapshot(session.snapshot());
        }
        match result {
            SaveResult::Saved | SaveResult::Skipped => {
                if result == SaveResult::Saved && self.remote.is_some() {
                    // `schedulePush` — the debounced upload; the UI owns
                    // the timer.
                    if let Some(cb) = &mut self.on_remote_push_schedule {
                        cb();
                    }
                }
            }
            SaveResult::Conflict => {
                self.synctex_state = WorkspaceSyncTeXState::Stale(
                    "The source changed on disk; SyncTeX locations may be stale.".into(),
                );
            }
            SaveResult::PermissionFailure(path) => {
                self.phase = WorkspacePhase::Failed(
                    WorkspaceOpenError::SavePermissionDenied(path).to_string(),
                )
            }
            SaveResult::InterruptedWrite(path) => {
                self.phase = WorkspacePhase::Failed(
                    WorkspaceOpenError::SaveInterrupted(path).to_string(),
                )
            }
            SaveResult::CommitFailed(reason) => self.phase = WorkspacePhase::Failed(reason),
        }
    }

    /// Blocks until every save worker has finished writing and committing.
    pub fn wait_for_pending_writes(&mut self) {
        for handle in self.pending_writes.drain(..) {
            let _ = handle.join();
        }
    }

    /// `resolveConflict(useDiskVersion:)` — verbatim port.
    pub fn resolve_conflict(&mut self, use_disk_version: bool) {
        let Some(snapshot) = self.document_snapshot.clone() else { return };
        if snapshot.save_state != DocumentSaveState::Conflicted {
            return;
        }
        let Some(url) = self.active_document_url.clone() else { return };
        let Some(session) = self
            .registered_sessions
            .iter()
            .find(|s| s.path() == &snapshot.path)
            .cloned()
        else {
            return;
        };
        if use_disk_version {
            // The adopted disk text supersedes whatever this app last wrote.
            self.writes.own().remove(&snapshot.path);
            match Self::read_exact_utf8(&url) {
                Ok(disk_text) => match session.apply(
                    DocumentMutation::ResolveConflict {
                        text: disk_text.clone(),
                        disk_baseline_hash: DiskContentHash::hashing(&disk_text),
                    },
                    snapshot.revision,
                ) {
                    Ok(updated) => self.set_snapshot(updated),
                    Err(e) => {
                        self.phase = WorkspacePhase::Failed(format!(
                            "The conflict could not be resolved: {e}"
                        ))
                    }
                },
                Err(e) => {
                    self.phase = WorkspacePhase::Failed(format!(
                        "The conflict could not be resolved: {e}"
                    ))
                }
            }
        } else {
            // Through the serialized writer: it waits for an in-flight save
            // of this file and clears exactly the conflict it resolves.
            match self.writes.write(&session, &url, true) {
                SaveResult::Saved | SaveResult::Skipped => self.set_snapshot(session.snapshot()),
                SaveResult::CommitFailed(e) => {
                    self.phase = WorkspacePhase::Failed(format!(
                        "The conflict could not be resolved: {e}"
                    ))
                }
                _ => {
                    self.phase = WorkspacePhase::Failed(
                        "The conflicted file could not be written to disk.".into(),
                    )
                }
            }
        }
    }

    // ── Remote session (RemoteSupport.swift port) ────────────────────────

    /// `endRemoteSession` — the last upload, then back to local builds.
    /// Close is async like the macOS one: the push runs detached so an
    /// unreachable device cannot freeze the window for seconds. The
    /// shared engine serializes it with any later open of the same
    /// mirror; only `connect_shutdown`'s `flushRemote` bounds it on quit.
    fn end_remote_session(&mut self) {
        let Some(remote) = self.remote.take() else { return };
        let sync = remote.sync.clone();
        std::thread::spawn(move || {
            let _ = sync.push();
        });
        self.workspace_executor.use_remote(None);
    }

    /// `flushRemote` — the final push, bounded so an unreachable device
    /// cannot hold the window or the app (unsent edits stay in the mirror
    /// and go up on the next open).
    pub fn flush_remote(&mut self, timeout: std::time::Duration) {
        let Some(remote) = &self.remote else { return };
        let sync = remote.sync.clone();
        let (done_tx, done_rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let _ = done_tx.send(sync.push());
        });
        match done_rx.recv_timeout(timeout) {
            Ok(Ok(report)) => {
                if let Some(remote) = self.remote.as_mut() {
                    remote.conflicts = report.conflicts;
                    remote.status = crate::remote::RemoteStatus::Synced;
                }
            }
            Ok(Err(error)) => {
                if let Some(remote) = self.remote.as_mut() {
                    remote.status =
                        crate::remote::RemoteStatus::Offline(error.to_string());
                }
            }
            Err(_) => {} // timed out — the worker finishes on its own
        }
    }

    /// `pushRemote` — upload local edits (after saves, agent runs and
    /// before builds). Runs on a worker; the result lands through
    /// `RemotePushFinished`.
    pub fn push_remote(&mut self) {
        let Some(remote) = self.remote.as_mut() else { return };
        remote.status = crate::remote::RemoteStatus::Syncing;
        let sync = remote.sync.clone();
        let root = remote.sync.mirror.directory.clone();
        if let Some(sink) = self.sink() {
            std::thread::spawn(move || {
                let _ = sink.send(WorkspaceMessage::RemotePushFinished {
                    root,
                    result: sync.push().map(|r| r.conflicts).map_err(|e| e.to_string()),
                });
            });
        }
    }

    /// `pullRemote` — bring down changes made on the device; the dispatch
    /// then adopts them in open documents (clean ones reload, dirty ones
    /// flag a conflict).
    pub fn pull_remote(&mut self) {
        let Some(remote) = self.remote.as_mut() else { return };
        remote.status = crate::remote::RemoteStatus::Syncing;
        let sync = remote.sync.clone();
        let root = remote.sync.mirror.directory.clone();
        if let Some(sink) = self.sink() {
            std::thread::spawn(move || {
                let _ = sink.send(WorkspaceMessage::RemotePullFinished {
                    root,
                    result: sync.pull().map_err(|e| e.to_string()),
                });
            });
        }
    }

    /// Window came forward: pull when the last pull is over a minute old
    /// (`pullRemoteIfStale`).
    pub fn pull_remote_if_stale(&mut self) {
        let stale = self
            .remote
            .as_ref()
            .map(|r| {
                !matches!(r.status, crate::remote::RemoteStatus::Syncing)
                    && r.last_pull
                        .map(|t| t.elapsed() > std::time::Duration::from_secs(60))
                        .unwrap_or(true)
            })
            .unwrap_or(false);
        if stale {
            self.pull_remote();
        }
    }

    /// "Sync with Remote Device" — `await pushRemote(); await pullRemote()`.
    pub fn sync_remote_now(&mut self) {
        let Some(remote) = self.remote.as_mut() else { return };
        remote.status = crate::remote::RemoteStatus::Syncing;
        let sync = remote.sync.clone();
        let root = remote.sync.mirror.directory.clone();
        let Some(sink) = self.sink() else { return };
        std::thread::spawn(move || {
            let _ = sink.send(WorkspaceMessage::RemotePushFinished {
                root: root.clone(),
                result: sync.push().map(|r| r.conflicts).map_err(|e| e.to_string()),
            });
            let _ = sink.send(WorkspaceMessage::RemotePullFinished {
                root,
                result: sync.pull().map_err(|e| e.to_string()),
            });
        });
    }

    /// `resolveRemoteConflict` — settle one conflicting file either way.
    pub fn resolve_remote_conflict(&mut self, path: String, keep_local: bool) {
        let Some(remote) = self.remote.as_mut() else { return };
        remote.status = crate::remote::RemoteStatus::Syncing;
        let sync = remote.sync.clone();
        let root = remote.sync.mirror.directory.clone();
        if let Some(sink) = self.sink() {
            std::thread::spawn(move || {
                let _ = sink.send(WorkspaceMessage::RemoteResolveFinished {
                    root,
                    keep_local,
                    result: sync.resolve(&path, keep_local).map_err(|e| e.to_string()),
                });
            });
        }
    }

    /// Applies a finished push — ignores results from a session that
    /// already closed.
    pub fn apply_remote_push(&mut self, root: &Path, result: Result<Vec<String>, String>) {
        let Some(remote) = self.remote.as_mut() else { return };
        if remote.sync.mirror.directory != root {
            return;
        }
        match result {
            Ok(conflicts) => {
                remote.conflicts = conflicts;
                remote.status = crate::remote::RemoteStatus::Synced;
            }
            Err(error) => remote.status = crate::remote::RemoteStatus::Offline(error),
        }
    }

    /// Applies a finished pull — returns `changedLocally` so the dispatch
    /// can adopt the new bytes in open sessions.
    pub fn apply_remote_pull(
        &mut self,
        root: &Path,
        result: Result<remote_core::SyncReport, String>,
    ) -> bool {
        let Some(remote) = self.remote.as_mut() else { return false };
        if remote.sync.mirror.directory != root {
            return false;
        }
        match result {
            Ok(report) => {
                remote.conflicts = report.conflicts.clone();
                remote.last_pull = Some(std::time::Instant::now());
                remote.status = crate::remote::RemoteStatus::Synced;
                report.changed_locally()
            }
            Err(error) => {
                remote.status = crate::remote::RemoteStatus::Offline(error);
                false
            }
        }
    }

    /// Applies a `GitRefreshed` result — shared by the GTK dispatch and
    /// model-level tests. Returns true when repo state was cleared (the
    /// "not a repository" page and remote-unreachable handling differ:
    /// a remote transport failure keeps the last status and surfaces the
    /// SSH error instead). The stale-root requeue stays in the dispatch.
    pub fn apply_git_refreshed(
        &mut self,
        result: &Result<Option<GitRefresh>, String>,
    ) -> bool {
        self.git_refresh_pending.set(false);
        let cleared = |model: &mut Self| {
            model.git_status = None;
            model.git_commits.clear();
            model.git_branches.clear();
            // `clearGitHistoryState` — expanded rows and an open diff
            // refer to a repo that may be gone.
            model.git_expanded_commits.clear();
            model.git_commit_files.clear();
            model.git_commit_files_busy.clear();
            model.git_diff = None;
        };
        match result {
            Ok(Some(GitRefresh {
                status,
                commits,
                branches,
            })) => {
                let old = self.git_status.replace(status.clone());
                if old.is_some() {
                    std::thread::spawn(move || drop(old));
                }
                self.git_commits = commits.clone();
                self.git_branches = branches.clone();
                if self.remote.is_some() {
                    // A reachable device clears a stale SSH error.
                    self.git_error = None;
                }
                false
            }
            // `Ok(None)` = a real non-zero `rev-parse` — not a repository.
            Ok(None) => {
                cleared(self);
                if self.remote.is_some() {
                    self.git_error = None;
                }
                true
            }
            Err(error) => {
                if self.remote.is_some() {
                    // The device is unreachable — an SSH error, not
                    // "not a repository"; the last status stays and the
                    // error shows in the pane.
                    self.git_error = Some(error.clone());
                    false
                } else {
                    cleared(self);
                    true
                }
            }
        }
    }

    /// Applies a finished conflict resolution — returns true when the
    /// local copy changed (`keep_local: false` adopts the remote bytes).
    pub fn apply_remote_resolve(
        &mut self,
        root: &Path,
        keep_local: bool,
        result: Result<Vec<String>, String>,
    ) -> bool {
        let Some(remote) = self.remote.as_mut() else { return false };
        if remote.sync.mirror.directory != root {
            return false;
        }
        match result {
            Ok(remaining) => {
                remote.conflicts = remaining;
                remote.status = crate::remote::RemoteStatus::Synced;
                !keep_local
            }
            Err(error) => {
                remote.status = crate::remote::RemoteStatus::Offline(error);
                false
            }
        }
    }

    /// `remote.statusText` — the window-title subtitle.
    pub fn remote_status_text(&self, language: &str) -> String {
        self.remote
            .as_ref()
            .map(|remote| crate::remote::status_text(language, remote))
            .unwrap_or_default()
    }

    /// `recentTitle(for:)` — a mirror's Open Recent label names its device.
    pub fn recent_title(language: &str, url: &Path) -> String {
        crate::remote::recent_title(language, url)
    }

    pub fn close_document(&mut self, url: &Path) {
        let Some(root) = self.project_url.clone() else { return };
        if let Some(index) = self.registered_sessions.iter().position(|s| {
            Self::relative_path(url, &root)
                .map(|p| s.path() == &p)
                .unwrap_or(false)
        }) {
            let session = self.registered_sessions.remove(index);
            if self.active_document_url.as_deref() == Some(url) {
                let _ = self.registry.close(&root, &session);
            }
        }
        self.open_documents.retain(|u| u != url);
        if self.active_document_url.as_deref() == Some(url) {
            self.files_revision += 1;
            if let Some(next) = self.open_documents.first().cloned() {
                self.active_document_url = None;
                if let Some(session) = self.session_for_url(&next) {
                    self.active_document_url = Some(next);
                    self.document_snapshot = Some(session.snapshot());
                    self.refresh_build_target();
                    self.phase = WorkspacePhase::Ready;
                    self.restore_built_preview();
                    self.refresh_structure();
                    if let Some(cb) = &mut self.on_active_document_changed {
                        cb();
                    }
                    self.emit_snapshot();
                }
            } else {
                self.active_document_url = None;
                self.document_snapshot = None;
                self.document_project = Default::default();
            }
        }
    }

    fn session_for_url(&self, url: &Path) -> Option<DocumentSession> {
        let root = self.project_url.as_ref()?;
        let relative = Self::relative_path(url, root).ok()?;
        self.registered_sessions
            .iter()
            .find(|s| s.path() == &relative)
            .cloned()
    }

    /// `close` — watchers/monitors are owned by the UI which unsubscribes
    /// first; the model clears all project state like the Swift method.
    pub fn close(&mut self) {
        // In-flight saves land before their sessions go away, and a
        // remote project uploads them before its mirror is let go.
        self.wait_for_pending_writes();
        self.writes.own().clear();
        self.end_remote_session();
        // Release the capability lease first — `endAccess(capabilityLease)`.
        if let Some(lease) = self.capability_lease.take() {
            let _ = self.files.end_access(lease);
        }
        self.read_write = true;
        let root = self.project_url.take();
        let sessions = std::mem::take(&mut self.registered_sessions);
        if let Some(root) = root {
            for session in sessions {
                let _ = self.registry.close(&root, &session);
            }
        }
        self.project_files.clear();
        self.document_project = Default::default();
        self.files_revision += 1;
        self.open_documents.clear();
        self.outline_items.clear();
        self.label_items.clear();
        self.bibliography_items.clear();
        self.todo_items.clear();
        self.git_status = None;
        self.git_commits.clear();
        self.git_branches.clear();
        self.git_commit_message.clear();
        self.git_busy = false;
        self.git_suggest_busy = false;
        self.git_error = None;
        // `clearGitHistoryState` — the history overlays refer to commits
        // that may no longer resolve.
        self.git_diff = None;
        self.git_expanded_commits.clear();
        self.git_commit_files.clear();
        self.git_commit_files_busy.clear();
        self.todo_cache.clear();
        self.project_label_keys.clear();
        self.citation_keys.clear();
        self.label_scan_cache.clear();
        self.bibliography_cache_key = None;
        self.structure_revision += 1;
        self.synctex_binding = None;
        self.pinned_build_target = None;
        self.automatic_build_target = None;
        self.build_target_message = None;
        self.console_section = ConsoleSection::Assistant;
        self.phase = WorkspacePhase::NoProject;
    }

    /// File → New inside the open project.
    pub fn create_document(&mut self, sink: Sender<WorkspaceMessage>) {
        let Some(root) = self.project_url.clone() else { return };
        // Name choice and write as one step against a remote sync commit —
        // the ticket ends with the write so `activate_document` can take
        // its own later.
        let written = {
            let _write_gate = remote_core::MirrorWrites::shared().ticket();
            let mut index = 1;
            let mut url = root.join("untitled.tex");
            while url.exists() {
                index += 1;
                url = root.join(format!("untitled-{index}.tex"));
            }
            std::fs::write(
                &url,
                "\\documentclass{article}\n\\begin{document}\n\n\\end{document}\n",
            )
            .map(|()| url)
        };
        match written {
            Ok(url) => {
                if !self.project_files.contains(&url) {
                    self.project_files.push(url.clone());
                    sort_files(&mut self.project_files);
                    self.files_revision += 1;
                }
                self.activate_document(url, sink);
            }
            Err(e) => self.phase = WorkspacePhase::Failed(e.to_string()),
        }
    }

    /// File → Save As…
    pub fn save_as(&mut self, url: PathBuf, sink: Sender<WorkspaceMessage>) {
        let Some(snapshot) = self.document_snapshot.clone() else { return };
        {
            let _write_gate = remote_core::MirrorWrites::shared().ticket();
            if let Err(e) = std::fs::write(&url, &snapshot.text) {
                self.phase = WorkspacePhase::Failed(e.to_string());
                return;
            }
        }
        if let Some(root) = self.project_url.clone() {
            if Self::relative_path(&url, &root).is_ok() {
                if !self.project_files.contains(&url) {
                    self.project_files.push(url.clone());
                    sort_files(&mut self.project_files);
                    self.files_revision += 1;
                }
                self.activate_document(url, sink);
            }
        }
    }

    /// Edit → Toggle Line Comment: returns the (UTF-16 range, replacement)
    /// the adapter applies, mirroring `lineRange(for:)` + `%` prefix logic.
    pub fn toggle_line_comment_transform(&self) -> Option<(std::ops::Range<usize>, String)> {
        let text = &self.document_snapshot.as_ref()?.text;
        let (loc, len) = self.editor_selection;
        if loc + len > utf16_len(text) {
            return None;
        }
        let (range_start16, range_len16) = line_range_utf16(text, loc, len);
        let (b0, b1) = utf16_range_to_bytes(text, range_start16, range_len16);
        let block = &text[b0..b1];
        let lines: Vec<&str> = block.split('\n').collect();
        let content_lines: Vec<&&str> = lines.iter().filter(|l| !l.trim().is_empty()).collect();
        let all_commented = !content_lines.is_empty()
            && content_lines.iter().all(|l| l.trim_start().starts_with('%'));
        let transformed = lines
            .iter()
            .map(|line| {
                if line.trim().is_empty() {
                    return line.to_string();
                }
                if all_commented {
                    if let Some(idx) = line.find('%') {
                        let mut result = format!("{}{}", &line[..idx], &line[idx + 1..]);
                        if result.starts_with(' ') {
                            result.remove(0);
                        }
                        return result;
                    }
                    return line.to_string();
                }
                format!("%{line}")
            })
            .collect::<Vec<_>>()
            .join("\n");
        Some((range_start16..range_start16 + range_len16, transformed))
    }

    // ── External-change monitoring (debounce owned by the UI) ──

    /// `processDiskChange` — verbatim: clean sessions adopt disk content;
    /// dirty sessions flag a conflict unless confirmOverwrite is off.
    pub fn process_disk_change(&mut self, url: &Path, confirm_overwrite: bool) {
        let Some(root) = self.project_url.clone() else { return };
        let Ok(relative) = Self::relative_path(url, &root) else { return };
        let Some(session) = self
            .registered_sessions
            .iter()
            .find(|s| s.path() == &relative)
            .cloned()
        else {
            return;
        };
        let is_active = self
            .document_snapshot
            .as_ref()
            .map(|s| s.path == relative)
            .unwrap_or(false);
        let Ok(disk_text) = Self::read_exact_utf8(url) else {
            let snapshot = session.snapshot();
            if snapshot.save_state != DocumentSaveState::Clean && snapshot.conflict.is_none() {
                if let Ok(updated) = session.apply(
                    DocumentMutation::RecordExternalChange {
                        observed_disk_hash: DiskContentHash::hashing(""),
                    },
                    snapshot.revision,
                ) {
                    if is_active {
                        self.set_snapshot(updated);
                    }
                }
            }
            return;
        };
        let observed_hash = DiskContentHash::hashing(&disk_text);
        {
            // Exactly the bytes our own last write put there (its commit
            // owns the baseline, possibly still in flight) — not an external
            // change. Anything else means the disk moved past that write, so
            // the record goes: identical bytes arriving later are external.
            let mut own = self.writes.own();
            if own.get(&relative) == Some(&observed_hash) {
                return;
            }
            own.remove(&relative);
        }
        let snapshot = session.snapshot();
        if observed_hash == snapshot.disk_baseline_hash || snapshot.conflict.is_some() {
            return;
        }
        let adopt_disk = snapshot.save_state == DocumentSaveState::Clean || !confirm_overwrite;
        if adopt_disk {
            if let Ok(updated) = session.apply(
                DocumentMutation::ResolveConflict {
                    text: disk_text,
                    disk_baseline_hash: observed_hash,
                },
                snapshot.revision,
            ) {
                if updated.path == relative && is_active {
                    self.set_snapshot(updated);
                }
            }
        } else if let Ok(updated) = session.apply_text_neutral(
            DocumentMutation::RecordExternalChange {
                observed_disk_hash: observed_hash,
            },
        ) {
            if is_active {
                self.set_snapshot(updated);
            }
        }
        // A non-active .tex may have adopted new disk content — its
        // \label contribution to project_label_keys changed with it
        // (the mtime check makes this a no-op for untouched files).
        self.refresh_project_labels();
    }

    /// `persistDirtySessions` — saves every dirty session; returns a
    /// user-facing problem string when one cannot persist.
    pub fn persist_dirty_sessions(&mut self) -> Option<String> {
        let root = self.project_url.clone()?;
        for session in self.registered_sessions.clone() {
            let snapshot = session.snapshot();
            if snapshot.save_state == DocumentSaveState::Conflicted {
                return Some(format!(
                    "The file {} has an unresolved external-change conflict. Resolve it before using the agent.",
                    snapshot.path.raw_value()
                ));
            }
            if snapshot.save_state != DocumentSaveState::Dirty {
                continue;
            }
            let file_url = root.join(snapshot.path.raw_value());
            // Synchronous — builds and agent runs need the files on disk
            // before they start — but serialized behind any in-flight save.
            match self.writes.write(&session, &file_url, false) {
                SaveResult::Saved | SaveResult::Skipped | SaveResult::CommitFailed(_) => {
                    if session.snapshot().save_state == DocumentSaveState::Conflicted {
                        return Some(format!(
                            "The file {} has an unresolved external-change conflict. Resolve it before using the agent.",
                            snapshot.path.raw_value()
                        ));
                    }
                }
                SaveResult::Conflict => {
                    return Some(format!(
                        "The file {} changed on disk while preparing the agent. Resolve the conflict first.",
                        snapshot.path.raw_value()
                    ));
                }
                SaveResult::PermissionFailure(_) | SaveResult::InterruptedWrite(_) => {
                    return Some(format!(
                        "The file {} could not be saved before running the agent.",
                        snapshot.path.raw_value()
                    ));
                }
            }
        }
        if let Some(active) = &self.document_snapshot {
            if let Some(session) = self
                .registered_sessions
                .iter()
                .find(|s| s.path() == &active.path)
            {
                let refreshed = session.snapshot();
                if refreshed.revision != active.revision {
                    self.set_snapshot(refreshed);
                }
            }
        }
        None
    }

    /// `refreshAfterAgentActivity` — adopt agent edits and rescan the tree.
    pub fn refresh_after_agent_activity(&mut self, confirm_overwrite: bool) {
        let Some(root) = self.project_url.clone() else { return };
        for session in self.registered_sessions.clone() {
            let url = root.join(session.path().raw_value());
            self.process_disk_change(&url, confirm_overwrite);
        }
        if let Ok(discovered) = Self::discover_tex_files(&root, &root, true) {
            let known: HashSet<PathBuf> = self.project_files.iter().cloned().collect();
            let additions: Vec<PathBuf> =
                discovered.into_iter().filter(|f| !known.contains(f)).collect();
            if !additions.is_empty() {
                self.project_files.extend(additions);
                self.files_revision += 1;
                sort_files(&mut self.project_files);
            }
        }
        self.refresh_structure();
    }

    // ── Sidebar structure (verbatim port) ──

    pub fn refresh_structure(&mut self) {
        self.refresh_structure_with(None);
    }

    /// `bib` carries items already parsed off-thread; `None` reuses the
    /// cached bibliography unless the project's .bib files changed.
    fn refresh_structure_with(&mut self, bib: Option<Vec<BibliographyItem>>) {
        // Parse from a borrow — cloning the whole document per refresh was
        // an O(doc) alloc on every structure update.
        let (outline, labels, count) = {
            let text = self
                .document_snapshot
                .as_ref()
                .map(|s| s.text.as_str())
                .unwrap_or_default();
            (Self::parse_outline(text), Self::parse_labels(text),
             text.split(|c| c == ' ' || c == '\n' || c == '\t').filter(|w| !w.is_empty()).count())
        };
        self.cached_word_count = count;
        self.outline_items = outline;
        self.label_items = labels;
        if let Some(items) = bib {
            self.bibliography_items = items;
            // Worker results may predate a disk edit. Establish the metadata
            // key on the next read rather than caching old items under it.
            self.bibliography_cache_key = None;
        } else {
            self.refresh_bibliography();
        }
        self.refresh_todos();
        self.citation_keys = self
            .bibliography_items
            .iter()
            .map(|item| item.key.clone())
            .collect();
        // An open .bib contributes its unsaved buffer keys too — parity
        // with `refreshCompletionKeys` on macOS.
        if self
            .active_document_url
            .as_ref()
            .and_then(|u| u.extension())
            .map(|e| e.eq_ignore_ascii_case("bib"))
            .unwrap_or(false)
        {
            if let Some(text) = self.document_snapshot.as_ref().map(|s| s.text.as_str()) {
                self.citation_keys.extend(Self::parse_bibliography_keys(text));
            }
        }
        self.refresh_project_labels();
        self.structure_revision += 1;
        if let Some(cb) = &mut self.on_structure_changed {
            cb();
        }
    }

    fn refresh_bibliography(&mut self) {
        let main = self.build_source_url().or_else(|| self.active_document_url.clone());
        let mut resolver = TeXProjectResolver::new();
        if let (Some(url), Some(snapshot)) = (&self.active_document_url, &self.document_snapshot) {
            resolver.active_text = Some((url.clone(), snapshot.text.clone()));
        }
        let files = resolver.bibliography_files(main.as_deref(), &self.project_files);
        let project = resolver.document_project(self.active_document_url.as_deref(), self.automatic_build_target.as_deref(),
                                                self.project_url.as_deref(), &self.project_files);
        if self.document_project != project {
            self.document_project = project;
            self.files_revision += 1;
        }
        let metadata: Vec<_> = files.iter()
            .map(|file| {
                let stamp = std::fs::metadata(file).ok().and_then(|m| {
                    m.modified().ok().map(|modified| (modified, m.len()))
                });
                (file.clone(), stamp)
            })
            .collect();
        let complete_metadata = metadata.iter().all(|(_, stamp)| stamp.is_some());
        let key = (self.project_url.clone(), metadata);
        if self.bibliography_cache_key.as_ref() == Some(&key) {
            return;
        }
        let (items, complete) = Self::read_bibliography(&files, self.project_url.as_deref());
        self.bibliography_items = items;
        // Retry transient read/metadata errors on the next refresh.
        self.bibliography_cache_key = (complete && complete_metadata).then_some(key);
    }

    /// `refreshTodos` — scans every project .tex file. The active document
    /// parses its live snapshot (unsaved edits included) keyed by revision;
    /// other files cache on mtime so unchanged files are not re-read.
    fn refresh_todos(&mut self) {
        // project_files are standardized at discovery; the active URL may
        // come from a raw root.join — normalize before comparing (same
        // rule as open_todo/edit_todo).
        let active_url = self
            .active_document_url
            .as_ref()
            .map(|u| standardize(u.clone()));
        let active_key = (
            0,
            self.document_snapshot
                .as_ref()
                .map(|s| s.revision)
                .unwrap_or(0),
        );
        let mut items = Vec::new();
        let mut seen: HashSet<PathBuf> = HashSet::new();
        let project_files = self.project_files.clone();
        for url in &project_files {
            if !url
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.eq_ignore_ascii_case("tex"))
                .unwrap_or(false)
            {
                continue;
            }
            seen.insert(url.clone());
            let is_active = active_url.as_deref() == Some(standardize(url.clone()).as_path());
            let (key, text) = if is_active {
                if let Some((cached_key, cached_items)) = self.todo_cache.get(url) {
                    if *cached_key == active_key {
                        items.extend(cached_items.iter().cloned());
                        continue;
                    }
                }
                (
                    active_key,
                    self.document_snapshot
                        .as_ref()
                        .map(|s| s.text.clone())
                        .unwrap_or_else(|| std::fs::read_to_string(url).unwrap_or_default()),
                )
            } else {
                let mtime = std::fs::metadata(url)
                    .and_then(|m| m.modified())
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_millis())
                    .unwrap_or(0);
                let key = (mtime, 0);
                if let Some((cached_key, cached_items)) = self.todo_cache.get(url) {
                    if *cached_key == key {
                        items.extend(cached_items.iter().cloned());
                        continue;
                    }
                }
                (key, std::fs::read_to_string(url).unwrap_or_default())
            };
            let parsed = Self::parse_todos(&text, url, self.project_url.as_deref());
            self.todo_cache.insert(url.clone(), (key, parsed.clone()));
            items.extend(parsed);
        }
        self.todo_cache.retain(|path, _| seen.contains(path));
        self.todo_items = items;
    }

    /// `parseTodos` — every `% TODO:`/`% DONE:` comment in `text`, 1-based
    /// lines, trailing comments included.
    pub fn parse_todos(text: &str, url: &Path, root: Option<&Path>) -> Vec<DocumentTodoItem> {
        let name = root
            .and_then(|r| {
                Self::relative_path(url, r)
                    .ok()
                    .map(|p| p.raw_value().to_string())
            })
            .unwrap_or_else(|| {
                url.file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default()
            });
        let mut items = Vec::new();
        for (index, raw_line) in text.lines().enumerate() {
            let line = raw_line.trim_end_matches('\r');
            if let Some((_, tail_start, done)) = Self::todo_marker(line) {
                items.push(DocumentTodoItem {
                    file: name.clone(),
                    url: url.to_path_buf(),
                    line: index + 1,
                    text: line[tail_start..]
                        .trim_start_matches([' ', '\t'])
                        .to_string(),
                    done,
                });
            }
        }
        items
    }

    /// `todoMarker` — locates `% TODO:`/`% DONE:` in a line like the Swift
    /// version: any `%` can introduce the comment (so trailing
    /// `code % TODO: x` lines count), spaces/tabs may follow it. Returns
    /// (keyword byte offset, text-tail byte offset, done flag).
    fn todo_marker(line: &str) -> Option<(usize, usize, bool)> {
        let bytes = line.as_bytes();
        let mut search = 0usize;
        while let Some(pos) = line[search..].find('%') {
            let at = search + pos;
            let mut cursor = at + 1;
            while cursor < line.len() && (bytes[cursor] == b' ' || bytes[cursor] == b'\t') {
                cursor += 1;
            }
            for (word, done) in [("TODO:", false), ("DONE:", true)] {
                if line[cursor..].starts_with(word) {
                    return Some((cursor, cursor + word.len(), done));
                }
            }
            search = at + 1;
        }
        None
    }

    /// `todoLineBounds` — byte range of the 1-based line plus its newline
    /// (when present), matching the Swift helper's NSString semantics.
    fn todo_line_bounds(text: &str, line: usize) -> Option<(usize, usize)> {
        if line == 0 {
            return None;
        }
        let mut start = 0usize;
        for (n, chunk) in text.split_inclusive('\n').enumerate() {
            let end = start + chunk.len();
            if n + 1 == line {
                return Some((start, end));
            }
            start = end;
        }
        None
    }

    /// Ranged transform for a todo mutation: the byte range of `text` to
    /// replace and its replacement, like `toggleLineCommentTransform`.
    /// `None` when the line no longer holds a marker.
    fn todo_transform_for(
        text: &str,
        item: &DocumentTodoItem,
        edit: &TodoLineEdit,
    ) -> Option<(std::ops::Range<usize>, String)> {
        let (start, end) = Self::todo_line_bounds(text, item.line)?;
        let line = &text[start..end];
        // The first marker wins — same scan order as `todo_marker`.
        let (keyword_start, tail_start, done) = Self::todo_marker(line)?;
        match edit {
            TodoLineEdit::ToggleDone => Some((
                start + keyword_start..start + tail_start,
                if done { "TODO:" } else { "DONE:" }.to_string(),
            )),
            TodoLineEdit::Rename(new_text) => {
                let trailing = line.len() - line.trim_end_matches(['\r', '\n']).len();
                let content_end = end - trailing;
                Some((
                    start + tail_start..content_end,
                    if new_text.is_empty() {
                        String::new()
                    } else {
                        format!(" {new_text}")
                    },
                ))
            }
            TodoLineEdit::Delete => Some((start..end, String::new())),
        }
    }

    /// `todoSessionIsClean` (negated) — only a clean or absent session may
    /// be overwritten by a direct disk write; dirty *and* conflicted
    /// sessions refuse, exactly like the Swift check.
    fn todo_session_is_dirty(&self, url: &Path) -> bool {
        let Some(root) = &self.project_url else { return false };
        let Ok(relative) = Self::relative_path(url, root) else { return false };
        self.registered_sessions.iter().any(|s| {
            s.path() == &relative && s.snapshot().save_state != DocumentSaveState::Clean
        })
    }

    /// Writes a todo mutation to a non-active file: reads disk, applies the
    /// line edit, saves atomically against the read baseline, then runs the
    /// usual external-change pass so a clean open session adopts it.
    fn edit_todo_on_disk(&mut self, item: &DocumentTodoItem, edit: TodoLineEdit) {
        if self.todo_session_is_dirty(&item.url) {
            return;
        }
        // Read-check-write as one step against a remote sync commit —
        // released before `process_disk_change`, which may write itself.
        let outcome = {
            let _write_gate = remote_core::MirrorWrites::shared().ticket();
            let Ok(disk) = Self::read_exact_utf8(&item.url) else { return };
            let Some((range, replacement)) = Self::todo_transform_for(&disk, item, &edit) else {
                return;
            };
            let new_text = format!("{}{}{}", &disk[..range.start], replacement, &disk[range.end..]);
            AtomicDocumentStore::new().save(
                &new_text,
                &item.url,
                Some(DiskContentHash::hashing(&disk)),
            )
        };
        if matches!(outcome, DocumentSaveOutcome::Saved(_)) {
            self.process_disk_change(&item.url, true);
        }
    }

    /// `openTodo` — same-file items publish `on_jump_to` immediately;
    /// cross-file items park the line in `pending_jump` (the inverse-
    /// SyncTeX mechanism) so `apply_activate` replays it once the owning
    /// document is live.
    pub fn open_todo(&mut self, item: &DocumentTodoItem, sink: Sender<WorkspaceMessage>) {
        let already_active = self
            .active_document_url
            .as_ref()
            .map(|a| standardize(a.clone()) == standardize(item.url.clone()))
            .unwrap_or(false);
        if already_active {
            if let Some(cb) = &mut self.on_jump_to {
                cb(item.line.max(1), 0);
            }
        } else {
            self.pending_jump = Some((item.line.max(1), 0));
            self.activate_document(item.url.clone(), sink);
        }
    }

    /// `editTodo` — active-document items return the (UTF-16 range,
    /// replacement) the caller applies through the editor buffer as a user
    /// action; other files write to disk unless a dirty session owns them.
    pub fn edit_todo(
        &mut self,
        item: &DocumentTodoItem,
        edit: TodoLineEdit,
    ) -> Option<(std::ops::Range<usize>, String)> {
        let is_active = self
            .active_document_url
            .as_ref()
            .map(|a| standardize(a.clone()) == standardize(item.url.clone()))
            .unwrap_or(false);
        if is_active {
            let text = self.document_snapshot.as_ref()?.text.clone();
            let (range, replacement) = Self::todo_transform_for(&text, item, &edit)?;
            let start16 = utf16_len(&text[..range.start]);
            let end16 = start16 + utf16_len(&text[range.start..range.end]);
            return Some((start16..end16, replacement));
        }
        self.edit_todo_on_disk(item, edit);
        None
    }

    /// `canAddTodo` — the `+` button is live while the active document is a
    /// .tex file or a build target exists to receive the fallback comment.
    pub fn can_add_todo(&self) -> bool {
        let active_is_tex = self
            .active_document_url
            .as_ref()
            .and_then(|u| u.extension())
            .map(|e| e.eq_ignore_ascii_case("tex"))
            .unwrap_or(false);
        active_is_tex || self.todo_append_target().is_some()
    }

    /// `isSourceFile` — .tex/.bib/.md/.markdown activate in the editor;
    /// every other discovered extension is a figure that opens externally.
    pub fn is_source_file(url: &Path) -> bool {
        url.extension()
            .and_then(|e| e.to_str())
            .map(|e| {
                e.eq_ignore_ascii_case("tex")
                    || e.eq_ignore_ascii_case("bib")
                    || Self::is_markdown(url)
            })
            .unwrap_or(false)
    }

    /// `.md`/`.markdown` — the one Markdown predicate every feature branch
    /// goes through (`isMarkdown` on macOS).
    pub fn is_markdown(url: &Path) -> bool {
        url.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("md") || e.eq_ignore_ascii_case("markdown"))
            .unwrap_or(false)
    }

    /// Active document is Markdown — drives the inspector's preview swap.
    pub fn active_is_markdown(&self) -> bool {
        self.active_document_url
            .as_ref()
            .map(|u| Self::is_markdown(u))
            .unwrap_or(false)
    }

    fn is_tex(url: &Path) -> bool {
        url.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("tex"))
            .unwrap_or(false)
    }

    /// `appendTodoComment` — the `+` fallback when the active document is
    /// not .tex: appends to the build target unless a dirty session owns it.
    /// Swift uses `buildSourceURL()` alone — no wider fallback — so a
    /// project without a resolved target keeps `+` disabled on both sides.
    pub fn todo_append_target(&self) -> Option<PathBuf> {
        self.build_source_url()
            .filter(|f| Self::is_tex(f))
    }

    pub fn append_todo_to_target(&mut self) {
        let Some(target) = self.todo_append_target() else { return };
        if self.todo_session_is_dirty(&target) {
            return;
        }
        let outcome = {
            let _write_gate = remote_core::MirrorWrites::shared().ticket();
            let Ok(disk) = Self::read_exact_utf8(&target) else { return };
            let mut new_text = disk.clone();
            if !new_text.is_empty() && !new_text.ends_with('\n') {
                new_text.push('\n');
            }
            new_text.push_str("% TODO: \n");
            AtomicDocumentStore::new().save(
                &new_text,
                &target,
                Some(DiskContentHash::hashing(&disk)),
            )
        };
        if matches!(outcome, DocumentSaveOutcome::Saved(_)) {
            self.process_disk_change(&target, true);
        }
    }

    /// Rebuild the merged `\label` set the completion provider reads.
    /// Cached per file on mtime — the 120ms-debounced structure refresh
    /// re-reads only what changed, never the whole project per keystroke.
    fn refresh_project_labels(&mut self) {
        let active = self.active_document_url.clone();
        let mut alive = HashSet::new();
        for file in &self.project_files {
            if file
                .extension()
                .map(|e| !e.eq_ignore_ascii_case("tex"))
                .unwrap_or(true)
            {
                continue;
            }
            // The active document contributes its unsaved snapshot text
            // below — a fresh \label{} completes before the file hits disk.
            if active.as_ref() == Some(file) {
                continue;
            }
            alive.insert(file.clone());
            let mtime = std::fs::metadata(file).and_then(|m| m.modified()).ok();
            if self
                .label_scan_cache
                .get(file)
                .map(|(cached, _)| *cached != mtime)
                .unwrap_or(true)
            {
                let labels = std::fs::read_to_string(file)
                    .map(|text| Self::label_names(&text))
                    .unwrap_or_default();
                self.label_scan_cache.insert(file.clone(), (mtime, labels));
            }
        }
        self.label_scan_cache.retain(|file, _| alive.contains(file));
        let mut keys: BTreeSet<String> = self
            .label_scan_cache
            .values()
            .flat_map(|(_, labels)| labels.iter().cloned())
            .collect();
        let active_is_tex = self
            .active_document_url
            .as_ref()
            .and_then(|u| u.extension())
            .map(|e| e.eq_ignore_ascii_case("tex"))
            .unwrap_or(false);
        if active_is_tex {
            if let Some(text) = self.document_snapshot.as_ref().map(|s| s.text.as_str()) {
                keys.extend(Self::label_names(text));
            }
        }
        self.project_label_keys = keys;
    }

    /// Off-thread `\label` scan of every project .tex — returns the
    /// mtime-keyed cache `refresh_project_labels` incrementally reuses.
    fn scan_project_labels(files: &[PathBuf]) -> LabelScanCache {
        let mut cache = HashMap::new();
        for file in files {
            if file
                .extension()
                .map(|e| !e.eq_ignore_ascii_case("tex"))
                .unwrap_or(true)
            {
                continue;
            }
            let mtime = std::fs::metadata(file).and_then(|m| m.modified()).ok();
            let labels = std::fs::read_to_string(file)
                .map(|text| Self::label_names(&text))
                .unwrap_or_default();
            cache.insert(file.clone(), (mtime, labels));
        }
        cache
    }

    fn label_names(text: &str) -> Vec<String> {
        Self::parse_labels(text)
            .into_iter()
            .map(|label| label.name)
            .collect()
    }

    /// Completion candidates for a detected context — the data-source
    /// closure the GTK `TexCompletionProvider` calls per populate (the
    /// macOS adapter's `completionSource` contract).
    pub fn editor_completions(
        &self,
        context: &language_core::CompletionContext,
    ) -> Vec<language_core::LanguageCompletion> {
        language_core::LanguageIndex::completions(
            context,
            &self.project_label_keys,
            &self.citation_keys,
        )
    }

    pub fn rescan_project(&mut self) {
        let Some(root) = self.project_url.clone() else { return };
        if let Ok(discovered) = Self::discover_tex_files(&root, &root, true) {
            self.project_files = discovered;
            self.files_revision += 1;
            self.refresh_build_target();
            self.refresh_structure();
        }
    }

    /// `\\(part|chapter|section|subsection|subsubsection|paragraph)\*?\{…\}`
    /// — longest-name matching so `subsection` doesn't parse as `section`.
    pub fn parse_outline(text: &str) -> Vec<DocumentOutlineItem> {
        const NAMES: [(&str, usize); 6] = [
            ("part", 0),
            ("chapter", 1),
            ("section", 2),
            ("subsection", 3),
            ("subsubsection", 4),
            ("paragraph", 5),
        ];
        let mut items = Vec::new();
        let bytes = text.as_bytes();
        let mut i = 0;
        // Line numbers accrue as the scan advances — recounting
        // `text[..i]` per match made label/section-dense files quadratic.
        let mut counted = 0usize;
        let mut line = 1usize;
        while i < bytes.len() {
            if bytes[i] == b'\\' {
                let rest = &text[i + 1..];
                let name_len: usize = rest
                    .chars()
                    .take_while(|c| c.is_ascii_alphabetic())
                    .map(|c| c.len_utf8())
                    .sum();
                let name = &rest[..name_len];
                if let Some((_, level)) = NAMES.iter().find(|(n, _)| *n == name) {
                    let after_name = &rest[name_len..];
                    let after_star = after_name.strip_prefix('*').unwrap_or(after_name);
                    if let Some(body) = after_star.strip_prefix('{') {
                        if let Some(close) = body.find('}') {
                            line += text[counted..i]
                                .bytes()
                                .filter(|b| *b == b'\n')
                                .count();
                            counted = i;
                            items.push(DocumentOutlineItem {
                                title: body[..close].to_string(),
                                level: *level,
                                line,
                            });
                        }
                    }
                }
            }
            i += 1;
        }
        items
    }

    /// `\\label\{([^}]*)\}`
    pub fn parse_labels(text: &str) -> Vec<DocumentLabelItem> {
        let mut items = Vec::new();
        let mut search = 0usize;
        // Matches arrive in order — carry the line count forward instead
        // of rescanning `text[..start]` per match (quadratic otherwise).
        let mut counted = 0usize;
        let mut line = 1usize;
        while let Some(pos) = text[search..].find("\\label{") {
            let start = search + pos;
            let body = &text[start + 7..];
            if let Some(close) = body.find('}') {
                line += text[counted..start]
                    .bytes()
                    .filter(|b| *b == b'\n')
                    .count();
                counted = start;
                items.push(DocumentLabelItem {
                    name: body[..close].to_string(),
                    line,
                });
            }
            search = start + 7;
        }
        items
    }

    /// `@([A-Za-z]+)\s*\{\s*([^,\s]+)`
    pub fn parse_bibliography(files: &[PathBuf], root: Option<&Path>) -> Vec<BibliographyItem> {
        Self::read_bibliography(files, root).0
    }

    fn read_bibliography(files: &[PathBuf], root: Option<&Path>) -> (Vec<BibliographyItem>, bool) {
        let mut items = Vec::new();
        let mut complete = true;
        for file in files {
            let Ok(text) = std::fs::read_to_string(file) else {
                complete = false;
                continue;
            };
            let name = root
                .and_then(|r| {
                    Self::relative_path(file, r)
                        .ok()
                        .map(|p| p.raw_value().to_string())
                })
                .unwrap_or_else(|| {
                    file.file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_default()
                });
            for (kind, key) in Self::parse_bibliography_entries(&text) {
                items.push(BibliographyItem {
                    key,
                    kind,
                    file: name.clone(),
                });
            }
        }
        (items, complete)
    }

    /// Citation keys in a .bib buffer — the active document's unsaved
    /// bibliography feeds `\cite` completion the same way disk items do.
    pub fn parse_bibliography_keys(text: &str) -> BTreeSet<String> {
        Self::parse_bibliography_entries(text)
            .into_iter()
            .map(|(_, key)| key)
            .collect()
    }

    /// `(lowercased kind, key)` entry pairs — shared by the file scan and
    /// the active-buffer merge so both paths parse identically.
    fn parse_bibliography_entries(text: &str) -> Vec<(String, String)> {
        let mut entries = Vec::new();
        let bytes = text.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'@' {
                i += 1;
                let type_start = i;
                while i < bytes.len() && bytes[i].is_ascii_alphabetic() {
                    i += 1;
                }
                let kind = &text[type_start..i];
                while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                    i += 1;
                }
                if i < bytes.len() && bytes[i] == b'{' && !kind.is_empty() {
                    i += 1;
                    while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                        i += 1;
                    }
                    let key_start = i;
                    while i < bytes.len() && bytes[i] != b',' && !bytes[i].is_ascii_whitespace() {
                        i += 1;
                    }
                    let key = &text[key_start..i];
                    if !key.is_empty() {
                        entries.push((kind.to_lowercase(), key.to_string()));
                    }
                }
            } else {
                i += 1;
            }
        }
        entries
    }

    // ── SyncTeX ──

    pub fn refresh_synctex_binding(&mut self, pdf_url: PathBuf) {
        let Some(root) = self.project_url.clone() else {
            self.synctex_state = WorkspaceSyncTeXState::Unavailable(
                "SyncTeX is unavailable until a successful build produces matching metadata."
                    .into(),
            );
            return;
        };
        let runner = self.synctex_runner.clone();
        let build_id = self
            .active_build_id
            .as_ref()
            .map(|b| b.raw_value.clone())
            .unwrap_or_else(|| "existing-pdf".into());
        if let Some(sink) = self.sink() {
            std::thread::spawn(move || {
                let result = runner
                    .refresh_binding(&root, &pdf_url, &build_id)
                    .map_err(|e| e.to_string());
                let _ = sink.send(WorkspaceMessage::BindingRefreshed(result));
            });
        }
    }

    pub fn apply_binding_refreshed(&mut self, result: Result<SyncTeXBinding, String>) {
        match result {
            Ok(binding) => {
                self.synctex_binding = Some(binding);
                self.synctex_state = WorkspaceSyncTeXState::Current;
            }
            Err(_) => {
                self.synctex_binding = None;
                self.synctex_state = WorkspaceSyncTeXState::Unavailable(
                    "SyncTeX metadata could not be loaded for this build.".into(),
                );
            }
        }
    }

    /// Lazily binds SyncTeX to the PDF on disk so Cmd-click navigation works
    /// for documents rendered before this session as well as fresh builds.
    fn ensure_synctex_binding(&mut self) {
        if self.is_building() {
            return;
        }
        self.restore_built_preview();
    }

    /// Bytes of `<main>.pdf` under the project root — read on workers so
    /// `restore_built_preview` never touches the disk on the main thread.
    fn built_pdf_bytes(root: &Path, main: &Path) -> Option<Vec<u8>> {
        let relative = Self::relative_path(main, root).ok()?;
        let name = Path::new(relative.raw_value())
            .with_extension("pdf")
            .to_string_lossy()
            .into_owned();
        std::fs::read(root.join(&name)).ok()
    }

    /// `restoreBuiltPreview` — reopen the main document's PDF, including when
    /// a chapter or .bib file was opened first. Switching within that
    /// document keeps its preview.
    pub fn restore_built_preview(&mut self) {
        self.restore_built_preview_with(None);
    }

    /// `prefetched` carries PDF bytes already read off-thread; `None` falls
    /// back to a synchronous read (small, user-triggered paths only).
    fn restore_built_preview_with(&mut self, prefetched: Option<Vec<u8>>) {
        if self.is_building() {
            return;
        }
        let (Some(root), Some(source)) =
            (self.project_url.clone(), self.build_source_url())
        else {
            return;
        };
        let Ok(relative) = Self::relative_path(&source, &root) else {
            return;
        };
        let name = Path::new(relative.raw_value())
            .with_extension("pdf")
            .to_string_lossy()
            .into_owned();
        if self.latest_built_pdf_name.as_deref() == Some(name.as_str())
            && self.synctex_binding.is_some()
        {
            return;
        }
        let pdf = root.join(&name);
        let data = prefetched.or_else(|| std::fs::read(&pdf).ok());
        match data {
            Some(data) if data.starts_with(b"%PDF") => {
                self.latest_built_pdf_name = Some(name);
                self.build_state = WorkspaceBuildState::Succeeded {
                    hash: {
                        use std::hash::{Hash, Hasher};
                        let mut h = std::collections::hash_map::DefaultHasher::new();
                        data.hash(&mut h);
                        h.finish()
                    },
                    pdf: data.into(),
                    log: self.build_log_text.clone(),
                };
                self.refresh_synctex_binding(pdf);
            }
            _ => {
                if self.latest_built_pdf_name.as_deref() != Some(name.as_str()) {
                    self.latest_built_pdf_name = None;
                    self.synctex_binding = None;
                    self.build_state = WorkspaceBuildState::Unavailable(
                        "Build the main document to create its PDF preview.".into(),
                    );
                    self.synctex_state = WorkspaceSyncTeXState::Unavailable(
                        "SyncTeX requires a completed build of the main document.".into(),
                    );
                }
            }
        }
    }

    /// `invalidateSyncTeXForBuild` — a starting build makes previous
    /// fingerprint/PDF-hash bindings stale by definition.
    pub fn invalidate_synctex_for_build(&mut self) {
        self.synctex_binding = None;
        self.synctex_state = WorkspaceSyncTeXState::Unavailable(
            "SyncTeX will be refreshed after the build completes.".into(),
        );
    }

    /// Forward sync from the caret — NSString line/column semantics.
    pub fn sync_forward(&mut self) {
        let Some(text) = self.document_snapshot.as_ref().map(|s| s.text.clone()) else {
            self.synctex_state = WorkspaceSyncTeXState::Unavailable(
                "SyncTeX requires a completed build.".into(),
            );
            return;
        };
        let cursor = self.editor_selection.0.min(utf16_len(&text));
        let b0 = utf16_range_to_bytes(&text, 0, cursor).0;
        let line = text[..b0].bytes().filter(|b| *b == b'\n').count() + 1;
        let column = text[..b0]
            .rfind('\n')
            .map(|i| utf16_len(&text[i + 1..b0]))
            .unwrap_or(cursor);
        self.sync_forward_at(line, column);
    }

    /// Forward sync at explicit position (1-based line, 0-based UTF-16 col).
    pub fn sync_forward_at(&mut self, line: usize, column: usize) {
        self.ensure_synctex_binding();
        let (Some(binding), Some(url)) = (
            self.synctex_binding.clone(),
            self.active_document_url.clone(),
        ) else {
            self.synctex_state = WorkspaceSyncTeXState::Unavailable(
                "SyncTeX requires a completed build.".into(),
            );
            return;
        };
        let runner = self.synctex_runner.clone();
        if let Some(sink) = self.sink() {
            std::thread::spawn(move || {
                let result = runner
                    .forward(&binding, &url, line as i64, column as i64)
                    .map_err(|e| e.to_string());
                let _ = sink.send(WorkspaceMessage::ForwardResult(result));
            });
        }
    }

    pub fn apply_forward_result(&mut self, result: Result<SyncTeXQueryCandidate, String>) {
        match result {
            Ok(m) => {
                if let Some(cb) = &mut self.on_synctex_highlight {
                    cb(m.pdf.page, m.h, m.v, m.width, m.height);
                }
            }
            Err(e) => {
                self.synctex_state = if e == "stale" {
                    WorkspaceSyncTeXState::Stale(
                        "SyncTeX results no longer match the current PDF.".into(),
                    )
                } else if let Some(count) = e.strip_prefix("ambiguous:") {
                    WorkspaceSyncTeXState::Ambiguous(format!(
                        "{count} matches; the source is ambiguous."
                    ))
                } else if e == "no_match" {
                    WorkspaceSyncTeXState::Stale(
                        "No SyncTeX location matched the cursor position.".into(),
                    )
                } else {
                    WorkspaceSyncTeXState::Stale(
                        "SyncTeX output could not be parsed.".into(),
                    )
                };
            }
        }
    }

    /// Inverse sync: PDF click → editor position.
    pub fn sync_inverse(&mut self, page: i64, point: PDFPoint) {
        self.ensure_synctex_binding();
        let Some(binding) = self.synctex_binding.clone() else { return };
        let runner = self.synctex_runner.clone();
        if let Some(sink) = self.sink() {
            std::thread::spawn(move || {
                let result = runner
                    .inverse(&binding, page, &point)
                    .map_err(|e| e.to_string());
                let _ = sink.send(WorkspaceMessage::InverseResult(result));
            });
        }
    }

    pub fn apply_inverse_result(
        &mut self,
        result: Result<SyncTeXQueryCandidate, String>,
        sink: Sender<WorkspaceMessage>,
    ) {
        match result {
            Ok(m) => {
                let Some(root) = self.project_url.clone() else { return };
                let file_url = root.join(&m.source.path.value);
                let resolved = standardize(file_url.clone());
                if !self
                    .project_files
                    .iter()
                    .any(|f| standardize(f.clone()) == resolved)
                {
                    self.project_files.push(file_url.clone());
                    sort_files(&mut self.project_files);
                    self.files_revision += 1;
                }
                let already_active = self
                    .active_document_url
                    .as_ref()
                    .map(|a| standardize(a.clone()) == resolved)
                    .unwrap_or(false);
                if !already_active {
                    // The jump lands in apply_activate once the doc is active.
                    self.pending_jump = Some((
                        m.source.line.max(1) as usize,
                        m.source.column.max(0) as usize,
                    ));
                    self.activate_document(file_url, sink);
                } else if let Some(cb) = &mut self.on_jump_to {
                    cb(m.source.line.max(1) as usize, m.source.column.max(0) as usize);
                }
            }
            Err(e) => {
                if e == "stale" {
                    self.synctex_state = WorkspaceSyncTeXState::Stale(
                        "SyncTeX results no longer match the current PDF.".into(),
                    );
                } else if let Some(count) = e.strip_prefix("ambiguous:") {
                    self.synctex_state = WorkspaceSyncTeXState::Ambiguous(format!(
                        "{count} matches; the PDF position is ambiguous."
                    ));
                }
                // no_match / parse failures are silently ignored (verbatim).
            }
        }
    }

    // ── Build (BuildSupport.swift port) ──

    /// The 21 generated-artifact extensions (verbatim list).
    pub const GENERATED_OUTPUT_EXTENSIONS: [&'static str; 21] = [
        "pdf", "aux", "log", "out", "synctex.gz", "fdb_latexmk", "fls", "toc", "bbl", "blg",
        "bcf", "run.xml", "idx", "ind", "ilg", "nav", "snm", "vrb", "xdv", "lof", "lot",
    ];

    /// `engineStage(forCommandPreset:)` — exact preset-string mapping.
    fn engine_stage_for_preset(command: &str) -> Option<BuildToolStage> {
        match command {
            "xelatex -interaction=nonstopmode -synctex=1 {file}" => {
                Some(BuildToolStage::LatexmkXeLaTeX)
            }
            "pdflatex -interaction=nonstopmode -synctex=1 {file}"
            | "latexmk -pdf -interaction=nonstopmode -synctex=1 {file}" => {
                Some(BuildToolStage::Latexmk)
            }
            "lualatex -interaction=nonstopmode -synctex=1 {file}" => {
                Some(BuildToolStage::LatexmkLuaLaTeX)
            }
            _ => None,
        }
    }

    /// `togglePinnedBuildTarget` — only .tex sources can be pinned.
    /// Folds or unfolds a project-tree directory. The sidebar only rebuilds
    /// the project list when `files_revision` moves, so bump it — without
    /// this the click changed the state but the tree never redrew.
    pub fn toggle_project_dir(&mut self, path: &str) {
        if !self.collapsed_project_dirs.remove(path) {
            self.collapsed_project_dirs.insert(path.to_string());
        }
        self.files_revision += 1;
    }

    pub fn toggle_pinned_build_target(&mut self) {
        let Some(url) = self.active_document_url.clone() else { return };
        if url
            .extension()
            .map(|e| !e.eq_ignore_ascii_case("tex"))
            .unwrap_or(true)
        {
            return;
        }
        self.pinned_build_target = if self.pinned_build_target.as_ref() == Some(&url) {
            None
        } else {
            Some(url)
        };
        // The pin badge is part of the tree.
        self.files_revision += 1;
        self.refresh_build_target();
        self.restore_built_preview();
    }

    /// The expensive half of `refresh_build_target`: lexes every project
    /// file to find the main document. Pure
    /// with respect to `self` so workers can run it off the main thread.
    fn resolve_build_target(
        active_url: Option<&Path>,
        active_text: Option<String>,
        files: &[PathBuf],
        preferred: Option<&Path>,
        root: Option<&Path>,
    ) -> (Result<Option<PathBuf>, ResolutionError>, project_feature::DocumentProject) {
        let mut resolver = TeXProjectResolver::new();
        if let (Some(url), Some(text)) = (active_url, active_text) {
            resolver.active_text = Some((url.to_path_buf(), text));
        }
        let result = resolver.resolve(active_url, files, preferred);
        let project = resolver.document_project(active_url, result.as_ref().ok().and_then(|p| p.as_deref()), root, files);
        (result, project)
    }

    /// Applies a `resolve_build_target` result — the cheap, main-thread half.
    fn apply_build_resolution(
        &mut self,
        (result, project): (Result<Option<PathBuf>, ResolutionError>, project_feature::DocumentProject),
    ) {
        let previous_target = self.automatic_build_target.clone();
        match result {
            Ok(target) => {
                self.automatic_build_target = target;
                self.build_target_message = if self.automatic_build_target.is_none() {
                    Some(
                        "No main TeX document was found. Open the project folder or set % !TeX root in the chapter."
                            .into(),
                    )
                } else {
                    None
                };
            }
            Err(e) => {
                self.automatic_build_target = None;
                self.build_target_message = Some(e.to_string());
            }
        }
        if self.automatic_build_target != previous_target || self.document_project != project {
            self.files_revision += 1;
        }
        self.document_project = project;
    }

    /// `refreshBuildTarget` — resolve the main document for the active
    /// file; failures surface a user-facing reason instead of a dead button.
    pub fn refresh_build_target(&mut self) {
        let resolution = Self::resolve_build_target(
            self.active_document_url.as_deref(),
            self.document_snapshot.as_ref().map(|s| s.text.clone()),
            &self.project_files,
            self.automatic_build_target.as_deref(),
            self.project_url.as_deref(),
        );
        self.apply_build_resolution(resolution);
    }

    /// Chapters and bibliography files share their owning main document.
    pub fn build_source_url(&self) -> Option<PathBuf> {
        self.pinned_build_target
            .clone()
            .or_else(|| self.automatic_build_target.clone())
    }
    pub fn build_source_relative_path(&self) -> Option<String> {
        let root = self.project_url.as_ref()?;
        let url = self.build_source_url()?;
        Self::relative_path(&url, root)
            .ok()
            .map(|p| p.raw_value().to_string())
    }

    /// `{file}` / `{filename}` placeholder expansion (verbatim).
    pub fn substitute_command_placeholders(&self, template: &str) -> String {
        let relative = self
            .build_source_relative_path()
            .unwrap_or_else(|| "main.tex".into());
        let stem = Path::new(&relative)
            .with_extension("")
            .to_string_lossy()
            .into_owned();
        template
            .replace("{file}", &relative)
            .replace("{filename}", &stem)
    }

    /// `startBuild` — verbatim port: main-document session adoption, persist
    /// dirty sessions before root discovery, generated-path declaration,
    /// pipeline selection.
    pub fn start_build(&mut self, store: &SettingsStore, language: &'static str) {
        if self.is_building() {
            return;
        }
        if !matches!(self.phase, WorkspacePhase::Ready) {
            return;
        }
        // Root discovery must see edits to inactive main/preamble files too.
        if let Some(problem) = self.persist_dirty_sessions() {
            self.build_state = WorkspaceBuildState::Failed(problem);
            return;
        }
        self.refresh_build_target();
        let (Some(root), Some(source_url)) =
            (self.project_url.clone(), self.build_source_url())
        else {
            let reason = self
                .build_target_message
                .clone()
                .unwrap_or_else(|| WorkspaceBuildError::NoActiveDocument.to_string());
            self.build_state = WorkspaceBuildState::Failed(reason.clone());
            self.build_log_text = reason;
            self.console_section = ConsoleSection::Log;
            self.bottom_panel_visible = true;
            return;
        };
        // Open a session for a main target that has none yet so its disk
        // baseline is tracked like every other open document.
        if Some(&source_url) != self.active_document_url.as_ref() {
            let has_session = self.registered_sessions.iter().any(|s| {
                Self::relative_path(&source_url, &root)
                    .map(|p| s.path() == &p)
                    .unwrap_or(false)
            });
            if !has_session {
                if let (Ok(text), Ok(file)) = (
                    Self::read_exact_utf8(&source_url),
                    Self::project_file(&source_url, &root),
                ) {
                    if let Ok(session) = self.registry.open(
                        &root,
                        &file,
                        text.clone(),
                        Some(DiskContentHash::hashing(&text)),
                    ) {
                        self.registered_sessions.push(session);
                    }
                }
            }
        }
        let Ok(relative) = Self::relative_path(&source_url, &root) else {
            self.build_state =
                WorkspaceBuildState::Unavailable(WorkspaceBuildError::NoActiveDocument.to_string());
            return;
        };
        let has_session = self
            .registered_sessions
            .iter()
            .any(|s| s.path() == &relative);
        if !has_session {
            self.build_state =
                WorkspaceBuildState::Unavailable(WorkspaceBuildError::NoActiveDocument.to_string());
            return;
        }

        let relative_source = relative.raw_value().to_string();
        let stem = Path::new(&relative_source)
            .with_extension("")
            .to_string_lossy()
            .into_owned();
        let generated: HashSet<String> = Self::GENERATED_OUTPUT_EXTENSIONS
            .iter()
            .map(|ext| format!("{stem}.{ext}"))
            .collect();
        // `prepareRemoteBuild(outputs: generated.sorted(), required: pdf)`
        // — computed now; `generated` is consumed by `BuildTarget::new`.
        let generated_sorted: Vec<String> = {
            let mut sorted: Vec<String> = generated.iter().cloned().collect();
            sorted.sort();
            sorted
        };
        let output_pdf = format!("{stem}.pdf");
        let target = match BuildTarget::new(
            &root,
            relative_source.clone(),
            output_pdf.clone(),
            generated,
            None,
        ) {
            Ok(t) => t,
            Err(_) => {
                self.build_state = WorkspaceBuildState::Failed(format!(
                    "The build target could not be constructed for {relative_source}."
                ));
                return;
            }
        };

        let build_prefs = &store.settings.build;
        let command_text = self.build_command_text.trim().to_string();
        let pipeline = if let Some(stage) = Self::engine_stage_for_preset(&command_text) {
            BuildPipeline::single_pass(stage)
        } else if !command_text.is_empty() {
            if !build_prefs.custom_shell_acknowledged {
                self.build_state = WorkspaceBuildState::Failed(
                    WorkspaceBuildError::CustomShellNotAcknowledged.to_string(),
                );
                return;
            }
            let authority = match ShellAuthority::new(
                ShellAuthoritySource::UserConfiguration,
                true,
                "User-configured build command executed via a login shell.",
            ) {
                Ok(a) => a,
                Err(e) => {
                    self.build_state = WorkspaceBuildState::Failed(e.to_string());
                    return;
                }
            };
            match LoginShellCommandPlan::new(
                store.custom_shell_executable(),
                self.substitute_command_placeholders(&command_text),
                WorkingDirectoryPolicy::ProjectRoot,
                build_core::EnvironmentPolicy::Inherit {
                    overrides: std::collections::HashMap::new(),
                },
                authority,
            ) {
                Ok(plan) => BuildPipeline::Custom(plan),
                Err(_) => {
                    self.build_state =
                        WorkspaceBuildState::Failed("The configured build command is not valid.".into());
                    return;
                }
            }
        } else {
            let stage = match build_prefs.engine {
                TeXEnginePreference::PdfLaTeX => BuildToolStage::Latexmk,
                TeXEnginePreference::XeLaTeX => BuildToolStage::LatexmkXeLaTeX,
                TeXEnginePreference::LuaLaTeX => BuildToolStage::LatexmkLuaLaTeX,
            };
            BuildPipeline::single_pass(stage)
        };

        if let Err(e) = self.build_orchestrator.select(target, pipeline) {
            self.build_state =
                WorkspaceBuildState::Failed(format!("The build could not be configured: {e}"));
            return;
        }
        let Some(build_id) = BuildID::new(format!("build-{}", uuid_v4())).ok() else {
            self.build_state =
                WorkspaceBuildState::Failed("The build could not be started.".into());
            return;
        };
        self.active_build_id = Some(build_id.clone());
        self.invalidate_synctex_for_build();
        self.build_state = WorkspaceBuildState::Building;
        self.build_log_text.clear();
        self.build_issues.clear();

        let orchestrator = self.build_orchestrator.clone();
        // A remote project compiles on its device: pending edits go up
        // first, and the executor brings these outputs back afterwards.
        let remote_build = self.remote.as_ref().map(|r| {
            (
                r.sync.clone(),
                r.executor.clone(),
                r.sync.mirror.directory.clone(),
            )
        });
        let cancel_flag = self.build_cancel_requested.clone();
        cancel_flag.store(false, std::sync::atomic::Ordering::SeqCst);
        if let Some(sink) = self.sink() {
            let handler_sink = sink.clone();
            std::thread::spawn(move || {
                if let Some((sync, executor, mirror_root)) = remote_build {
                    // `prepareRemoteBuild` — the upload first, then tell the
                    // executor which outputs to bring back.
                    let problem = match sync.push() {
                        Ok(report) => {
                            let _ = sink.send(WorkspaceMessage::RemotePushFinished {
                                root: mirror_root.clone(),
                                result: Ok(report.conflicts.clone()),
                            });
                            if report.conflicts.is_empty() {
                                None
                            } else {
                                Some(crate::l10n::trn(
                                    language,
                                    "remote.build.conflicts",
                                    &[&report.conflicts.join(", ")],
                                ))
                            }
                        }
                        Err(error) => {
                            let _ = sink.send(WorkspaceMessage::RemotePushFinished {
                                root: mirror_root,
                                result: Err(error.to_string()),
                            });
                            Some(crate::l10n::trn(
                                language,
                                "remote.build.upload_failed",
                                &[&error.to_string()],
                            ))
                        }
                    };
                    if let Some(problem) = problem {
                        let _ = sink.send(WorkspaceMessage::RemoteBuildPrepFailed(problem));
                        return;
                    }
                    executor.set_outputs(generated_sorted, Some(output_pdf.clone()));
                    if cancel_flag.load(std::sync::atomic::Ordering::SeqCst) {
                        let _ = sink.send(WorkspaceMessage::RemoteBuildPrepFailed(
                            "Build cancelled.".into(),
                        ));
                        return;
                    }
                }
                let outcome = orchestrator.build(
                    build_id,
                    Arc::new(move |event| {
                        let _ = handler_sink.send(WorkspaceMessage::BuildEvent(event));
                    }),
                );
                let _ = sink.send(WorkspaceMessage::BuildFinished {
                    outcome: outcome.map_err(|e| e.to_string()),
                    output_pdf,
                });
            });
        }
    }

    /// `handleBuildEvent` — verbatim.
    pub fn apply_build_event(&mut self, event: BuildEvent) {
        match event {
            BuildEvent::Log(entry) => self.build_log_text.push_str(&entry.text),
            BuildEvent::Issue(record) => self.build_issues.push(record),
            BuildEvent::Lifecycle(_) | BuildEvent::StageStarted { .. } => {}
        }
    }

    /// Terminal mapping + `switchToPDFOnBuild` / `jumpToCursorAfterBuild`
    /// follow-ups. Returns `Some(pdf_path)` when the UI should also load the
    /// built PDF (it needs the absolute path).
    pub fn apply_build_finished(
        &mut self,
        outcome: Result<BuildOutcome, String>,
        output_pdf: &str,
        switch_to_pdf: bool,
        jump_to_cursor: bool,
    ) -> Option<PathBuf> {
        self.active_build_id = None;
        match outcome {
            Ok(outcome) => {
                self.build_issues = outcome.issues.clone();
                match outcome.lifecycle {
                    BuildLifecycle::Succeeded { .. } => {
                        let pdf = self
                            .build_orchestrator
                            .successful_pdf()
                            .unwrap_or_default();
                        self.build_state = WorkspaceBuildState::Succeeded {
                            hash: {
                                use std::hash::{Hash, Hasher};
                                let mut h = std::collections::hash_map::DefaultHasher::new();
                                pdf.hash(&mut h);
                                h.finish()
                            },
                            pdf: pdf.into(),
                            log: self.build_log_text.clone(),
                        };
                        self.latest_built_pdf_name = Some(output_pdf.to_string());
                        if let Some(root) = self.project_url.clone() {
                            let pdf_url = root.join(output_pdf);
                            self.refresh_synctex_binding(pdf_url.clone());
                            if switch_to_pdf {
                                self.inspector_visible = true;
                            }
                            if jump_to_cursor {
                                self.sync_forward();
                            }
                            return Some(pdf_url);
                        }
                    }
                    BuildLifecycle::Cancelled { .. } => {
                        self.build_state = WorkspaceBuildState::Failed("Build cancelled.".into());
                    }
                    BuildLifecycle::Failed { exit_code, .. } => {
                        self.build_state = WorkspaceBuildState::Failed(format!(
                            "Build failed{}.",
                            exit_code.map(|c| format!(" (exit {c})")).unwrap_or_default()
                        ));
                    }
                    _ => {
                        self.build_state = WorkspaceBuildState::Failed(
                            "The build ended in an unexpected state.".into(),
                        );
                    }
                }
            }
            Err(e) => {
                self.build_state = WorkspaceBuildState::Failed(e);
            }
        }
        None
    }

    pub fn cancel_build(&mut self) {
        // A remote build may still be uploading, with nothing running yet
        // for the orchestrator to cancel.
        if self.is_building() {
            self.build_cancel_requested
                .store(true, std::sync::atomic::Ordering::SeqCst);
        }
        let _ = self.build_orchestrator.cancel(2_000);
    }

    /// Build → Clean — removes generated artifacts and feeds the terminal.
    pub fn clean_build_artifacts(&mut self) {
        let Some(root) = self.project_url.clone() else { return };
        let Some(relative) = self.build_source_relative_path() else { return };
        let stem = Path::new(&relative)
            .with_extension("")
            .to_string_lossy()
            .into_owned();
        for ext in Self::GENERATED_OUTPUT_EXTENSIONS {
            let _ = std::fs::remove_file(root.join(format!("{stem}.{ext}")));
        }
        self.latest_built_pdf_name = None;
        self.terminal_feed(format!(
            "\u{1b}[90mcleaned generated files for {stem}\u{1b}[0m\r\n"
        ));
        self.console_section = ConsoleSection::Terminal;
        self.bottom_panel_visible = true;
    }

    /// `runCustomCommand` (⌃⌘B) — runs the saved command line in the
    /// terminal; requires the custom-shell acknowledgement.
    pub fn run_custom_command(&mut self, acknowledged: bool) {
        let template = self.custom_command_text.trim().to_string();
        if template.is_empty() {
            return;
        }
        if !acknowledged {
            self.terminal_feed(
                "\u{1b}[33mEnable shell commands in Settings → Compile first.\u{1b}[0m\r\n".into(),
            );
            self.console_section = ConsoleSection::Terminal;
            self.bottom_panel_visible = true;
            return;
        }
        let command = self.substitute_command_placeholders(&template);
        if let Some(cb) = &mut self.on_terminal_send {
            cb(command);
        }
        self.console_section = ConsoleSection::Terminal;
        self.bottom_panel_visible = true;
    }

    /// `runTerminalInput` — sends a line to the embedded shell.
    pub fn run_terminal_input(&mut self, command: &str) {
        let trimmed = command.trim();
        if trimmed.is_empty() {
            return;
        }
        if let Some(cb) = &mut self.on_terminal_send {
            cb(trimmed.to_string());
        }
        self.console_section = ConsoleSection::Terminal;
        self.bottom_panel_visible = true;
    }

    fn terminal_feed(&mut self, text: String) {
        if let Some(cb) = &mut self.on_terminal_feed {
            cb(text);
        }
    }

    // ── Project commands persistence (verbatim key scheme) ──

    pub fn load_project_commands(&mut self, store: &SettingsStore, root: &Path) {
        let key = format!("commands.{}", standardize(root.to_path_buf()).display());
        let stored = store.prefs().dictionary(&key).cloned();
        self.build_command_text = stored
            .as_ref()
            .and_then(|d| d.get("build").and_then(|v| v.as_str()).map(String::from))
            .unwrap_or_else(|| store.default_build_command());
        self.custom_command_text = stored
            .and_then(|d| d.get("custom").and_then(|v| v.as_str()).map(String::from))
            .unwrap_or_else(|| store.default_custom_command());
    }
    pub fn persist_commands(&mut self, store: &mut SettingsStore) {
        let Some(root) = self.project_url.clone() else { return };
        let key = format!("commands.{}", standardize(root).display());
        let mut map = serde_json::Map::new();
        map.insert("build".into(), self.build_command_text.clone().into());
        map.insert("custom".into(), self.custom_command_text.clone().into());
        store.prefs_mut().set(&key, serde_json::Value::Object(map));
    }

    /// Session restore on launch (once).
    pub fn restore_session_if_needed(
        &mut self,
        store: &SettingsStore,
        sink: Sender<WorkspaceMessage>,
    ) {
        if self.did_restore_session || self.has_project() || !store.restore_session() {
            return;
        }
        let Some(recent) = self.recent_documents.first().cloned() else { return };
        if !recent.exists() {
            return;
        }
        self.did_restore_session = true;
        self.open(recent, sink);
    }

    /// Re-reads the active document from disk (editor toolbar reload button).
    pub fn reload_active_document_from_disk(&mut self, confirm_overwrite: bool) {
        if let Some(url) = self.active_document_url.clone() {
            self.process_disk_change(&url, confirm_overwrite);
        }
    }

    // ── Agent context (verbatim envelope inputs) ──

    pub fn agent_context(&self) -> crate::agent::AgentContextSnapshot {
        let mut context = crate::agent::AgentContextSnapshot::default();
        context.project_root = self.project_url.clone();
        context.active_path = self
            .document_snapshot
            .as_ref()
            .map(|s| s.path.raw_value().to_string());
        context.active_text = self.document_snapshot.as_ref().map(|s| s.text.clone());
        if let Some(text) = self.document_snapshot.as_ref().map(|s| &s.text) {
            let (loc, len) = self.editor_selection;
            if len > 0 && loc + len <= utf16_len(&text) {
                let (b0, b1) = utf16_range_to_bytes(&text, loc, len);
                context.selection_text = Some(text[b0..b1].to_string());
            }
        }
        if let Some(root) = &self.project_url {
            context.project_files = self
                .project_files
                .iter()
                .filter_map(|f| {
                    Self::relative_path(f, root)
                        .ok()
                        .map(|p| p.raw_value().to_string())
                })
                .collect();
        }
        if let WorkspaceBuildState::Succeeded { pdf, .. } = &self.build_state {
            context.pdf_data = Some(pdf.clone());
            context.pdf_path = self.latest_built_pdf_name.clone();
        }
        context
    }

    /// `syncSelectionAttachment` — mirrors the dragged range into the agent's
    /// attachment chip.
    pub fn sync_selection_attachment(&mut self) {
        let attachment = (|| {
            // Borrow the text — cloning the whole document per selection
            // change was an O(doc) alloc on every caret move.
            let text = &self.document_snapshot.as_ref()?.text;
            let (loc, len) = self.editor_selection;
            if len == 0 || loc + len > utf16_len(text) {
                return None;
            }
            let (b0, b1) = utf16_range_to_bytes(text, loc, len);
            let selected = text[b0..b1].to_string();
            let start_line = text[..b0].bytes().filter(|b| *b == b'\n').count() + 1;
            let end_line = text[..b1].bytes().filter(|b| *b == b'\n').count() + 1;
            let path = self
                .document_snapshot
                .as_ref()
                .map(|s| s.path.raw_value().to_string())
                .or_else(|| {
                    self.active_document_url.as_ref().and_then(|u| {
                        u.file_name().map(|n| n.to_string_lossy().into_owned())
                    })
                })
                .unwrap_or_else(|| "document".into());
            Some((path, start_line, end_line, selected))
        })();
        if let Some(cb) = &mut self.on_selection_attachment {
            cb(attachment);
        }
    }

    /// `sendSelectionToAssistant` payload — fenced selection block or None.
    pub fn selection_for_assistant(&self) -> Option<String> {
        let text = self.document_snapshot.as_ref()?.text.clone();
        let (loc, len) = self.editor_selection;
        if len == 0 || loc + len > utf16_len(&text) {
            return None;
        }
        let (b0, b1) = utf16_range_to_bytes(&text, loc, len);
        Some(format!("```\n{}\n```\n", &text[b0..b1]))
    }

    // ── Word count / footer ──
    pub fn word_count(&self) -> usize {
        if self.document_snapshot.is_some() { self.cached_word_count } else { 0 }
    }
    pub fn active_document_relative_path(&self) -> Option<String> {
        let url = self.active_document_url.as_ref()?;
        let root = self.project_url.as_ref()?;
        Self::relative_path(url, root)
            .ok()
            .map(|p| p.raw_value().to_string())
    }

    /// `revealAssistant` / `toggleAssistant`.
    pub fn reveal_assistant(&mut self) {
        self.bottom_panel_visible = true;
        self.console_section = ConsoleSection::Assistant;
    }
    pub fn toggle_assistant(&mut self) {
        if self.bottom_panel_visible && self.console_section == ConsoleSection::Assistant {
            self.bottom_panel_visible = false;
        } else {
            self.reveal_assistant();
        }
    }

    // ── Static helpers (verbatim port) ──

    /// `discoverTexFiles` — recursive enumeration of sources (.tex/.bib) and
    /// figures (`PROJECT_FILE_EXTENSIONS`), hidden files skipped,
    /// path-sorted. The resolver-chosen root is scanned in full so nested
    /// source trees appear alongside the opened file.
    pub fn discover_tex_files(
        root: &Path,
        selected: &Path,
        is_directory: bool,
    ) -> Result<Vec<PathBuf>, WorkspaceOpenError> {
        let scan_root = root.to_path_buf();
        let mut files = Vec::new();
        let mut seen = HashSet::new();
        let mut stack = vec![scan_root];
        while let Some(dir) = stack.pop() {
            let Ok(entries) = std::fs::read_dir(&dir) else { continue };
            for entry in entries.flatten() {
                let path = entry.path();
                let name = entry.file_name().to_string_lossy().into_owned();
                if name.starts_with('.') {
                    continue;
                }
                let Ok(meta) = entry.metadata() else { continue };
                // Never descend into a symlinked directory — the macOS
                // enumerator lists the link itself but does not traverse it,
                // and a link to an ancestor would otherwise loop forever.
                // `metadata()` still resolves a symlinked *file* to the
                // target's type, so those keep listing (isRegularFileKey
                // resolves through links the same way).
                if meta.is_dir() && !path.is_symlink() {
                    stack.push(path);
                } else if meta.is_file() {
                    let ext = path
                        .extension()
                        .map(|e| e.to_string_lossy().to_lowercase())
                        .unwrap_or_default();
                    if PROJECT_FILE_EXTENSIONS.contains(&ext.as_str()) {
                        // A symlinked source resolves to its target: skip
                        // targets that escape the project root (they could
                        // never satisfy the relative-path check anyway) and
                        // dedupe canonical paths already listed.
                        let resolved = standardize(path);
                        if Self::relative_path(&resolved, root).is_ok()
                            && seen.insert(resolved.clone())
                        {
                            files.push(resolved);
                        }
                    }
                }
            }
        }
        if !is_directory {
            let sel = standardize(selected.to_path_buf());
            if !files.contains(&sel) {
                files.push(sel);
            }
        }
        sort_files(&mut files);
        Ok(files)
    }

    pub fn relative_path(
        url: &Path,
        root: &Path,
    ) -> Result<NormalizedRelativePath, WorkspaceOpenError> {
        // Component-wise comparison — a string prefix check cannot work on
        // Windows, where `canonicalize` produces `\`-separated `\\?\C:\…`
        // verbatim paths while the prefix was appended with '/'.
        let root = standardize(root.to_path_buf());
        let file = standardize(url.to_path_buf());
        let relative = file
            .strip_prefix(&root)
            .map_err(|_| WorkspaceOpenError::OutsideProject)?;
        // `NormalizedRelativePath` is a POSIX path and rejects '\', so the
        // components rejoin with '/' regardless of the platform separator.
        let raw = relative
            .components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join("/");
        NormalizedRelativePath::new(&raw).map_err(|_| WorkspaceOpenError::OutsideProject)
    }

    pub fn read_exact_utf8(url: &Path) -> Result<String, WorkspaceOpenError> {
        let data = std::fs::read(url).map_err(|_| WorkspaceOpenError::UnreadableProject)?;
        String::from_utf8(data).map_err(|_| WorkspaceOpenError::InvalidUtf8)
    }

    fn project_file(url: &Path, root: &Path) -> Result<ProjectFile, WorkspaceOpenError> {
        let relative = Self::relative_path(url, root)?;
        Ok(ProjectFile {
            document_id: StableDocumentID::new(Self::document_id_for(relative.raw_value()))
                .map_err(|_| WorkspaceOpenError::UnreadableProject)?,
            path: relative,
        })
    }

    /// FNV-1a document ID, identical to the Swift implementation.
    pub fn document_id_for(path: &str) -> String {
        let mut hash: u64 = 14_695_981_039_346_656_037;
        for byte in path.as_bytes() {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(1_099_511_628_211);
        }
        format!("document-{hash:x}")
    }

    fn set_snapshot(&mut self, snapshot: DocumentSnapshot) {
        self.document_snapshot = Some(snapshot);
        self.refresh_structure();
        self.emit_snapshot();
        if let Some(cb) = &mut self.on_autosave_schedule {
            cb();
        }
    }
    fn emit_snapshot(&mut self) {
        if let Some(cb) = &mut self.on_snapshot_changed {
            if let Some(s) = &self.document_snapshot {
                cb(s);
            }
        }
    }
}

// ─── Free helpers ────────────────────────────────────────────────────────────

/// `projectFileExtensions` — everything `discoverTexFiles` lists: sources
/// plus the figure formats LaTeX documents include. Build artifacts stay
/// excluded except .pdf, which figures legitimately use.
const PROJECT_FILE_EXTENSIONS: [&str; 15] = [
    "tex", "bib", "md", "markdown", "png", "jpg", "jpeg", "pdf", "eps", "svg", "gif", "tif", "tiff",
    "bmp", "webp",
];

/// `standardizedFileURL` + `resolvingSymlinksInPath`: canonicalize when the
/// path exists, otherwise strip `.`/`..` lexically.
pub fn standardize(path: PathBuf) -> PathBuf {
    if let Ok(c) = path.canonicalize() {
        return deverbatim(c);
    }
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                // A `..` that cannot resolve lexically is dropped for
                // absolute paths (`/../x` → `/x`) but must be kept for
                // relative ones (`../a` stays `../a`) — like
                // stringByStandardizingPath.
                if !out.pop() && !path.is_absolute() {
                    out.push("..");
                }
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// Windows `canonicalize` yields verbatim `\\?\C:\…` paths; re-express plain
/// disk paths without the verbatim prefix so a canonical path compares equal
/// to the same path spelled lexically (e.g. a not-yet-created file).
/// `\\?\UNC\…` shares are kept verbatim — they have no plain spelling.
#[cfg(windows)]
fn deverbatim(path: PathBuf) -> PathBuf {
    if let Some(std::path::Component::Prefix(prefix)) = path.components().next() {
        if let std::path::Prefix::VerbatimDisk(letter) = prefix.kind() {
            let mut out = PathBuf::from(format!("{}:", letter as char));
            out.extend(path.components().skip(1));
            return out;
        }
    }
    path
}

#[cfg(not(windows))]
fn deverbatim(path: PathBuf) -> PathBuf {
    path
}

/// The Linux answer to macOS `/Library/TeX/texbin`: upstream TeX Live
/// installs under `/usr/local/texlive/<year>/bin/<arch>-linux` (the apt
/// package lands in `/usr/bin` directly). Newest year first.
pub fn texlive_bin_dirs() -> Vec<PathBuf> {
    build_core::texlive_bin_dirs()
}

/// `localizedStandardCompare` approximation — natural sort (numeric runs
/// compare by value, case-insensitive otherwise).
fn sort_files(files: &mut Vec<PathBuf>) {
    files.sort_by(|a, b| natcmp(&a.to_string_lossy(), &b.to_string_lossy()));
}
fn natcmp(a: &str, b: &str) -> std::cmp::Ordering {
    let mut ai = a.chars().peekable();
    let mut bi = b.chars().peekable();
    loop {
        match (ai.peek().copied(), bi.peek().copied()) {
            (None, None) => return std::cmp::Ordering::Equal,
            (None, Some(_)) => return std::cmp::Ordering::Less,
            (Some(_), None) => return std::cmp::Ordering::Greater,
            (Some(ca), Some(cb)) => {
                if ca.is_ascii_digit() && cb.is_ascii_digit() {
                    let na: u64 = take_number(&mut ai);
                    let nb: u64 = take_number(&mut bi);
                    let ord = na.cmp(&nb);
                    if ord != std::cmp::Ordering::Equal {
                        return ord;
                    }
                } else {
                    let ord = ca.to_lowercase().cmp(cb.to_lowercase());
                    if ord != std::cmp::Ordering::Equal {
                        return ord;
                    }
                    ai.next();
                    bi.next();
                }
            }
        }
    }
}
fn take_number(it: &mut std::iter::Peekable<std::str::Chars>) -> u64 {
    let mut n = 0u64;
    while let Some(c) = it.peek().copied() {
        if !c.is_ascii_digit() {
            break;
        }
        n = n.saturating_mul(10).saturating_add(c as u64 - '0' as u64);
        it.next();
    }
    n
}

pub fn utf16_len(text: &str) -> usize {
    text.encode_utf16().count()
}

/// Maps a UTF-16 (offset,length) range to byte offsets in `text`.
pub fn utf16_range_to_bytes(text: &str, offset16: usize, len16: usize) -> (usize, usize) {
    let mut c = 0usize;
    let mut start = text.len();
    let mut end = text.len();
    for (i, ch) in text.char_indices() {
        if c == offset16 {
            start = i;
        }
        c += ch.len_utf16();
        if c == offset16 + len16 {
            end = i + ch.len_utf8();
            return (start, end);
        }
    }
    if c == offset16 {
        start = text.len();
    }
    (start, end)
}

/// `NSString.lineRange(for:)`: the UTF-16 (start,length) covering the lines
/// intersecting [loc, loc+len), including the trailing line terminator.
fn line_range_utf16(text: &str, loc: usize, len: usize) -> (usize, usize) {
    let (b0, b1) = utf16_range_to_bytes(text, loc, len);
    let line_start_byte = text[..b0].rfind('\n').map(|i| i + 1).unwrap_or(0);
    // The probe char is the last char of the range (or the caret position).
    let probe = if len > 0 && b1 > b0 { b1 - 1 } else { b0.min(text.len()) };
    let line_end_byte = text[probe.min(text.len())..]
        .find('\n')
        .map(|i| probe + i + 1)
        .unwrap_or(text.len());
    // Convert byte range back to UTF-16 units.
    let start16 = utf16_len(&text[..line_start_byte]);
    let len16 = utf16_len(&text[line_start_byte..line_end_byte]);
    (start16, len16)
}

#[cfg(unix)]
pub(crate) fn uuid_v4() -> String {
    let mut bytes = [0u8; 16];
    // getrandom(2) needs no fd and works without /dev; /dev/urandom is the
    // second source; the last resort spreads time/pid/address entropy so the
    // bytes are not all shifts of a single nanosecond value.
    let filled = unsafe {
        libc::getrandom(bytes.as_mut_ptr().cast(), bytes.len(), 0) as usize == bytes.len()
    } || std::fs::File::open("/dev/urandom")
        .and_then(|mut f| {
            use std::io::Read;
            f.read_exact(&mut bytes)
        })
        .is_ok();
    if !filled {
        fill_bytes_fallback(&mut bytes);
    }
    finish_uuid_v4(bytes)
}

/// Windows counterpart — no `getrandom`/`/dev/urandom`. `RandomState` keys
/// are seeded from OS entropy per process (they back `HashMap` DoS
/// resistance), so hashing a fresh `RandomState` plus time/pid spreads real
/// entropy across the bytes.
#[cfg(windows)]
pub(crate) fn uuid_v4() -> String {
    let mut bytes = [0u8; 16];
    use std::hash::{BuildHasher, Hasher};
    let build = std::collections::hash_map::RandomState::new();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0) as u64;
    for (i, chunk) in bytes.chunks_mut(8).enumerate() {
        let mut hasher = build.build_hasher();
        hasher.write_u64(nanos);
        hasher.write_u64(i as u64);
        hasher.write_u32(std::process::id());
        chunk.copy_from_slice(&hasher.finish().to_le_bytes());
    }
    finish_uuid_v4(bytes)
}

#[cfg(unix)]
fn fill_bytes_fallback(bytes: &mut [u8; 16]) {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0) as u64;
    let mut state = nanos
        ^ (std::process::id() as u64) << 32
        ^ (&*bytes as *const _ as usize) as u64;
    for b in bytes.iter_mut() {
        // xorshift64* — decorrelates the mixed sources across bytes.
        state ^= state >> 12;
        state ^= state << 25;
        state ^= state >> 27;
        *b = (state.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 56) as u8;
    }
}

fn finish_uuid_v4(mut bytes: [u8; 16]) -> String {
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15]
    )
}
