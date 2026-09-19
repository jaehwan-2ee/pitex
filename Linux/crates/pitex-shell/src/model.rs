//! Port of `WorkspaceModel` (`Mac/Sources/AppShell/PitexApp.swift`) and the
//! build methods of `BuildSupport.swift`. All mutation happens on the UI
//! thread (mirroring `@MainActor`); long-running work — builds, SyncTeX
//! queries — runs on background threads and reports back through
//! `WorkspaceMessage` on the channel installed with `set_event_sink`.

use std::collections::{HashMap, HashSet};
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
    DocumentSaveState, DocumentSession, DocumentSessionError, DocumentSessionRegistry,
    DocumentSnapshot,
};
use language_core::{LanguageFileSnapshot, LanguageTokenKind, TeXDialect};
use project_core::ProjectFile;
use tex_domain::NormalizedRelativePath;
use settings_feature::TeXEnginePreference;
use synctex_core::{PDFPoint, SyncTeXQueryCandidate};
use tex_domain::StableDocumentID;

use crate::settings::SettingsStore;
use crate::synctex::{SyncTeXBinding, SyncTeXRunner};

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
    Succeeded { pdf: Vec<u8>, log: String },
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

#[derive(Debug, Clone)]
pub struct DocumentOutlineItem {
    pub title: String,
    pub level: usize,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub struct DocumentLabelItem {
    pub name: String,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub struct BibliographyItem {
    pub key: String,
    pub kind: String,
    pub file: String,
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
    OpenFinished(Result<OpenedProject, String>),
    ActivateFinished(Result<ActivatedDocument, String>),
    AgentActivityFinished,
}
impl std::fmt::Debug for WorkspaceMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BuildEvent(_) => write!(f, "BuildEvent"),
            Self::BuildFinished { .. } => write!(f, "BuildFinished"),
            Self::ForwardResult(_) => write!(f, "ForwardResult"),
            Self::InverseResult(_) => write!(f, "InverseResult"),
            Self::BindingRefreshed(_) => write!(f, "BindingRefreshed"),
            Self::DiskChanged(p) => write!(f, "DiskChanged({p:?})"),
            Self::OpenFinished(_) => write!(f, "OpenFinished"),
            Self::ActivateFinished(_) => write!(f, "ActivateFinished"),
            Self::AgentActivityFinished => write!(f, "AgentActivityFinished"),
        }
    }
}

/// Payload for `open` completed off-thread.
pub struct OpenedProject {
    pub root: PathBuf,
    pub selected: PathBuf,
    pub files: Vec<PathBuf>,
    pub initial_url: PathBuf,
    pub session: DocumentSession,
}
/// Payload for `activate` completed off-thread.
pub struct ActivatedDocument {
    pub url: PathBuf,
    pub session: DocumentSession,
}

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
    fn captures(pattern: &str, text: &str) -> Vec<String> {
        let Ok(re) = regex::Regex::new(pattern) else {
            return Vec::new();
        };
        re.captures_iter(text)
            .filter_map(|c| c.get(1).map(|m| m.as_str().trim().to_string()))
            .collect()
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
            let Some(parsed) = self.snapshot(&file).cloned() else {
                continue;
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
                    pending.push(Self::canonical(target));
                }
            }
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
            .or_else(|| {
                files
                    .iter()
                    .find(|f| {
                        f.extension()
                            .map(|e| e.eq_ignore_ascii_case("tex"))
                            .unwrap_or(false)
                    })
                    .cloned()
            })
            .or_else(|| files.first().cloned())
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
    pub build_orchestrator: Arc<BuildOrchestrator<StreamingBuildExecutor>>,
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
}

impl WorkspaceModel {
    const RECENTS_KEY: &'static str = "pitex.pref.workspace.recentDocuments";

    pub fn new() -> Self {
        Self {
            phase: WorkspacePhase::NoProject,
            project_url: None,
            project_files: Vec::new(),
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
            build_orchestrator: Arc::new(BuildOrchestrator::new(
                StreamingBuildExecutor::default(),
            )),
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
            read_write: true,
            files: PlatformFileCapabilityBroker::new(),
            capability_lease: None,
            editor_selection: (0, 0),
            event_sink: None,
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
        }
    }

    pub fn set_event_sink(&mut self, sink: Sender<WorkspaceMessage>) {
        self.event_sink = Some(sink);
    }
    fn sink(&self) -> Option<Sender<WorkspaceMessage>> {
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
            let _ = sink.send(WorkspaceMessage::OpenFinished(
                result.map_err(|e| e.to_string()),
            ));
        });
    }

    fn open_worker(
        registry: std::sync::Arc<DocumentSessionRegistry>,
        selected: &Path,
    ) -> Result<OpenedProject, WorkspaceOpenError> {
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
            return Err(WorkspaceOpenError::ChangedWhileOpening);
        }
        Ok(OpenedProject {
            root,
            selected: selected.to_path_buf(),
            files,
            initial_url,
            session,
        })
    }

    /// Main-thread completion of `open`.
    pub fn apply_open(&mut self, store: &mut SettingsStore, opened: OpenedProject) {
        self.project_url = Some(opened.root.clone());
        self.project_files = opened.files;
        self.load_project_commands(store, &opened.root);
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
        self.refresh_build_target();
        self.build_state =
            WorkspaceBuildState::Unavailable("No build has run yet for this project.".into());
        self.synctex_state = WorkspaceSyncTeXState::Unavailable(
            "SyncTeX is unavailable until a successful build produces matching metadata.".into(),
        );
        self.phase = WorkspacePhase::Ready;
        self.restore_built_preview();
        self.record_recent(store, &opened.selected);
        self.refresh_structure();
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
        // Keep the workspace mounted when switching sources: a loading phase
        // destroys the split view and PDF view, losing their size and position.
        let registry = self.registry.clone();
        std::thread::spawn(move || {
            let result = (|| -> Result<ActivatedDocument, WorkspaceOpenError> {
                let text = Self::read_exact_utf8(&url)?;
                let file = Self::project_file(&url, &root)?;
                let session = registry
                    .open(&root, &file, text.clone(), Some(DiskContentHash::hashing(&text)))
                    .map_err(|_| WorkspaceOpenError::UnreadableProject)?;
                Ok(ActivatedDocument { url, session })
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
        if !self.open_documents.contains(&activated.url) {
            self.open_documents.push(activated.url);
        }
        self.document_snapshot = Some(activated.session.snapshot());
        self.refresh_build_target();
        self.phase = WorkspacePhase::Ready;
        self.restore_built_preview();
        self.refresh_structure();
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
            self.document_snapshot.clone(),
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
        let outcome = AtomicDocumentStore::new().save(
            &snapshot.text,
            &url,
            Some(snapshot.disk_baseline_hash),
        );
        match outcome {
            DocumentSaveOutcome::Saved(document) => {
                match session.apply(
                    DocumentMutation::CommitSave {
                        written_disk_hash: document.hash,
                    },
                    snapshot.revision,
                ) {
                    Ok(saved) => self.set_snapshot(saved),
                    Err(DocumentSessionError::StaleRevision { .. }) => {
                        self.set_snapshot(session.snapshot())
                    }
                    Err(e) => self.phase = WorkspacePhase::Failed(e.to_string()),
                }
            }
            DocumentSaveOutcome::StaleBaseline(conflict) => {
                let observed = conflict
                    .observed_disk
                    .map(|d| d.hash)
                    .unwrap_or_else(|| DiskContentHash::hashing(""));
                match session.apply(
                    DocumentMutation::RecordSaveConflict {
                        observed_disk_hash: observed,
                    },
                    snapshot.revision,
                ) {
                    Ok(conflicted) => self.set_snapshot(conflicted),
                    Err(_) => self.set_snapshot(session.snapshot()),
                }
                self.synctex_state = WorkspaceSyncTeXState::Stale(
                    "The source changed on disk; SyncTeX locations may be stale.".into(),
                );
            }
            DocumentSaveOutcome::PermissionFailure { path } => {
                self.phase = WorkspacePhase::Failed(
                    WorkspaceOpenError::SavePermissionDenied(path).to_string(),
                )
            }
            DocumentSaveOutcome::InterruptedWrite { path } => {
                self.phase = WorkspacePhase::Failed(
                    WorkspaceOpenError::SaveInterrupted(path).to_string(),
                )
            }
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
            let baseline = match &snapshot.conflict {
                Some(
                    DocumentConflict::ExternalModification { observed_disk, .. }
                    | DocumentConflict::SaveCollision { observed_disk, .. },
                ) => *observed_disk,
                None => snapshot.disk_baseline_hash,
            };
            let outcome = AtomicDocumentStore::new().save(&snapshot.text, &url, Some(baseline));
            match outcome {
                DocumentSaveOutcome::Saved(document) => match session.apply(
                    DocumentMutation::CommitSave {
                        written_disk_hash: document.hash,
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
                _ => {
                    self.phase = WorkspacePhase::Failed(
                        "The conflicted file could not be written to disk.".into(),
                    )
                }
            }
        }
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
        self.open_documents.clear();
        self.active_document_url = None;
        self.document_snapshot = None;
        self.latest_built_pdf_name = None;
        self.synctex_binding = None;
        self.pinned_build_target = None;
        self.automatic_build_target = None;
        self.build_target_message = None;
        self.console_section = ConsoleSection::Assistant;
        self.outline_items.clear();
        self.label_items.clear();
        self.bibliography_items.clear();
        self.phase = WorkspacePhase::NoProject;
    }

    /// File → New inside the open project.
    pub fn create_document(&mut self, sink: Sender<WorkspaceMessage>) {
        let Some(root) = self.project_url.clone() else { return };
        let mut index = 1;
        let mut url = root.join("untitled.tex");
        while url.exists() {
            index += 1;
            url = root.join(format!("untitled-{index}.tex"));
        }
        match std::fs::write(
            &url,
            "\\documentclass{article}\n\\begin{document}\n\n\\end{document}\n",
        ) {
            Ok(()) => {
                if !self.project_files.contains(&url) {
                    self.project_files.push(url.clone());
                    sort_files(&mut self.project_files);
                }
                self.activate_document(url, sink);
            }
            Err(e) => self.phase = WorkspacePhase::Failed(e.to_string()),
        }
    }

    /// File → Save As…
    pub fn save_as(&mut self, url: PathBuf, sink: Sender<WorkspaceMessage>) {
        let Some(snapshot) = self.document_snapshot.clone() else { return };
        if let Err(e) = std::fs::write(&url, &snapshot.text) {
            self.phase = WorkspacePhase::Failed(e.to_string());
            return;
        }
        if let Some(root) = self.project_url.clone() {
            if Self::relative_path(&url, &root).is_ok() {
                if !self.project_files.contains(&url) {
                    self.project_files.push(url.clone());
                    sort_files(&mut self.project_files);
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
        } else if let Ok(updated) = session.apply(
            DocumentMutation::RecordExternalChange {
                observed_disk_hash: observed_hash,
            },
            snapshot.revision,
        ) {
            if is_active {
                self.set_snapshot(updated);
            }
        }
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
            let outcome = AtomicDocumentStore::new().save(
                &snapshot.text,
                &file_url,
                Some(snapshot.disk_baseline_hash),
            );
            match outcome {
                DocumentSaveOutcome::Saved(document) => {
                    let _ = session.apply(
                        DocumentMutation::CommitSave {
                            written_disk_hash: document.hash,
                        },
                        snapshot.revision,
                    );
                }
                DocumentSaveOutcome::StaleBaseline(_) => {
                    let disk = Self::read_exact_utf8(&file_url).unwrap_or_default();
                    let _ = session.apply(
                        DocumentMutation::RecordSaveConflict {
                            observed_disk_hash: DiskContentHash::hashing(&disk),
                        },
                        snapshot.revision,
                    );
                    return Some(format!(
                        "The file {} changed on disk while preparing the agent. Resolve the conflict first.",
                        snapshot.path.raw_value()
                    ));
                }
                _ => {
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
                sort_files(&mut self.project_files);
            }
        }
        self.refresh_structure();
    }

    // ── Sidebar structure (verbatim port) ──

    pub fn refresh_structure(&mut self) {
        let text = self
            .document_snapshot
            .as_ref()
            .map(|s| s.text.clone())
            .unwrap_or_default();
        self.outline_items = Self::parse_outline(&text);
        self.label_items = Self::parse_labels(&text);
        let bib_files: Vec<PathBuf> = self
            .project_files
            .iter()
            .filter(|f| {
                f.extension()
                    .map(|e| e.eq_ignore_ascii_case("bib"))
                    .unwrap_or(false)
            })
            .cloned()
            .collect();
        self.bibliography_items =
            Self::parse_bibliography(&bib_files, self.project_url.as_deref());
        if let Some(cb) = &mut self.on_structure_changed {
            cb();
        }
    }

    pub fn rescan_project(&mut self) {
        let Some(root) = self.project_url.clone() else { return };
        if let Ok(discovered) = Self::discover_tex_files(&root, &root, true) {
            self.project_files = discovered;
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
                            let line =
                                text[..i].bytes().filter(|b| *b == b'\n').count() + 1;
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
        while let Some(pos) = text[search..].find("\\label{") {
            let start = search + pos;
            let body = &text[start + 7..];
            if let Some(close) = body.find('}') {
                let line = text[..start].bytes().filter(|b| *b == b'\n').count() + 1;
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
        let mut items = Vec::new();
        for file in files {
            let Ok(text) = std::fs::read_to_string(file) else { continue };
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
                            items.push(BibliographyItem {
                                key: key.to_string(),
                                kind: kind.to_lowercase(),
                                file: name.clone(),
                            });
                        }
                    }
                } else {
                    i += 1;
                }
            }
        }
        items
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

    /// `restoreBuiltPreview` — reopen the main document's PDF, including when
    /// a chapter or .bib file was opened first. Switching within that
    /// document keeps its preview.
    pub fn restore_built_preview(&mut self) {
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
        match std::fs::read(&pdf) {
            Ok(data) if data.starts_with(b"%PDF") => {
                self.latest_built_pdf_name = Some(name);
                self.build_state = WorkspaceBuildState::Succeeded {
                    pdf: data,
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
        self.refresh_build_target();
        self.restore_built_preview();
    }

    /// Chapters and bibliography files share their owning main document.
    pub fn build_source_url(&self) -> Option<PathBuf> {
        self.pinned_build_target
            .clone()
            .or_else(|| self.automatic_build_target.clone())
    }

    /// `refreshBuildTarget` — resolve the owning main document for the active
    /// file; failures surface a user-facing reason instead of a dead button.
    pub fn refresh_build_target(&mut self) {
        let mut resolver = TeXProjectResolver::new();
        if let (Some(url), Some(snapshot)) = (
            self.active_document_url.clone(),
            self.document_snapshot.clone(),
        ) {
            resolver.active_text = Some((url, snapshot.text));
        }
        match resolver.resolve(
            self.active_document_url.as_deref(),
            &self.project_files,
            self.automatic_build_target.as_deref(),
        ) {
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
    pub fn start_build(&mut self, store: &SettingsStore) {
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
        if let Some(sink) = self.sink() {
            let handler_sink = sink.clone();
            std::thread::spawn(move || {
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
                            pdf,
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
        if let Some(text) = self.document_snapshot.as_ref().map(|s| s.text.clone()) {
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
            let text = self.document_snapshot.as_ref()?.text.clone();
            let (loc, len) = self.editor_selection;
            if len == 0 || loc + len > utf16_len(&text) {
                return None;
            }
            let (b0, b1) = utf16_range_to_bytes(&text, loc, len);
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
        self.document_snapshot
            .as_ref()
            .map(|s| {
                s.text
                    .split(|c| c == ' ' || c == '\n' || c == '\t')
                    .filter(|w| !w.is_empty())
                    .count()
            })
            .unwrap_or(0)
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

    /// `discoverTexFiles` — recursive .tex/.bib enumeration, hidden files
    /// skipped, path-sorted. The resolver-chosen root is scanned in full so
    /// nested source trees appear alongside the opened file.
    pub fn discover_tex_files(
        root: &Path,
        selected: &Path,
        is_directory: bool,
    ) -> Result<Vec<PathBuf>, WorkspaceOpenError> {
        let scan_root = root.to_path_buf();
        let mut files = Vec::new();
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
                    if ext == "tex" || ext == "bib" {
                        // A symlinked source resolves to its target: skip
                        // targets that escape the project root (they could
                        // never satisfy the relative-path check anyway) and
                        // dedupe canonical paths already listed.
                        let resolved = standardize(path);
                        if Self::relative_path(&resolved, root).is_ok()
                            && !files.contains(&resolved)
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
