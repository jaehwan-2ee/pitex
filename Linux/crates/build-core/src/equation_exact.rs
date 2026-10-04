//! Port of `EquationExactRenderer.swift` — the optional exact equation
//! preview: one minimal document compiled with the project's own TeX engine
//! and shell-escape policy, in a private per-request temporary directory.
//! Never a full-project build, never a shell, never on every keystroke (the
//! preview engine decides when).

use crate::{
    CancellationToken, DirectCommandPlan, EnvironmentPolicy, ProcessRunner, ProcessRunnerError,
    ProcessStopReason, ProcessTermination, WorkingDirectoryPolicy,
};
use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExactEquationEngine {
    PdfLaTeX,
    XeLaTeX,
    LuaLaTeX,
}

impl ExactEquationEngine {
    pub fn raw_value(self) -> &'static str {
        match self {
            Self::PdfLaTeX => "pdflatex",
            Self::XeLaTeX => "xelatex",
            Self::LuaLaTeX => "lualatex",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExactEquationShellEscape {
    /// No flag in the build command — the TeX distribution's default applies
    /// to both the build and the preview.
    ProjectDefault,
    Enabled,
    Restricted,
    Disabled,
}

impl ExactEquationShellEscape {
    pub fn raw_value(self) -> &'static str {
        match self {
            Self::ProjectDefault => "projectDefault",
            Self::Enabled => "enabled",
            Self::Restricted => "restricted",
            Self::Disabled => "disabled",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ExactEquationProfile {
    pub engine: ExactEquationEngine,
    pub shell_escape: ExactEquationShellEscape,
}

impl ExactEquationProfile {
    pub const JOB_NAME: &'static str = "pitex-equation";

    /// Cache/identity string for the preview engine.
    pub fn identity(&self) -> String {
        format!("{}|{}", self.engine.raw_value(), self.shell_escape.raw_value())
    }

    /// The engine and shell-escape policy the project's build command uses,
    /// or None when it is not a plain pdflatex/xelatex/lualatex/latexmk
    /// invocation (custom scripts, tectonic, chained shell commands, a
    /// latexmk whose engine selection depends on user config, or quoting
    /// whitespace-splitting cannot safely tokenize) — the preview never
    /// substitutes a different engine.
    pub fn resolve(build_command: &str) -> Option<ExactEquationProfile> {
        const SHELL_SYNTAX: [&str; 9] = ["&&", "||", ";", "|", "`", "$(", ">", "<", "\n"];
        if SHELL_SYNTAX.iter().any(|s| build_command.contains(s)) {
            return None;
        }
        let tokens: Vec<&str> =
            build_command.split([' ', '\t']).filter(|t| !t.is_empty()).collect();
        // Quotes mean real shell tokenization that whitespace-splitting
        // would misread (`-pdflatex="lualatex -shell-escape"` would leak a
        // fake `-shell-escape` flag); refuse rather than substitute.
        if tokens.iter().any(|t| t.contains('"') || t.contains('\'')) {
            return None;
        }
        let first = tokens.first()?;
        let program = first.rsplit('/').next().unwrap_or(first);
        let flags: Vec<&str> = tokens
            .iter()
            .skip(1)
            // Drop ONE dash from `--` spellings: `--no-x` → `-no-x`.
            .map(|t| if t.starts_with("--") { &t[1..] } else { *t })
            .collect();

        let engine = match program {
            "pdflatex" => ExactEquationEngine::PdfLaTeX,
            "xelatex" => ExactEquationEngine::XeLaTeX,
            "lualatex" => ExactEquationEngine::LuaLaTeX,
            "latexmk" => {
                let mut selected: Option<ExactEquationEngine> = None;
                for flag in &flags {
                    let picked = match *flag {
                        "-xelatex" | "-pdfxe" => Some(ExactEquationEngine::XeLaTeX),
                        "-lualatex" | "-pdflua" => Some(ExactEquationEngine::LuaLaTeX),
                        // Explicit pdf-via-pdflatex modes.
                        "-pdf" => Some(ExactEquationEngine::PdfLaTeX),
                        // Non-pdflatex latexmk routes are out of scope.
                        _ if flag.starts_with("-ps")
                            || flag.starts_with("-dvi")
                            || flag.starts_with("-pdfps")
                            || flag.starts_with("-pdfdvi") =>
                        {
                            return None;
                        }
                        _ => {
                            if let Some(value) = flag.strip_prefix("-pdflatex=") {
                                match value.trim() {
                                    "pdflatex" => Some(ExactEquationEngine::PdfLaTeX),
                                    "xelatex" => Some(ExactEquationEngine::XeLaTeX),
                                    "lualatex" => Some(ExactEquationEngine::LuaLaTeX),
                                    // Anything else is a program string we
                                    // cannot safely interpret.
                                    _ => return None,
                                }
                            } else {
                                None
                            }
                        }
                    };
                    if let Some(engine) = picked {
                        // Conflicting explicit selections are ambiguous.
                        if selected.is_some_and(|s| s != engine) {
                            return None;
                        }
                        selected = Some(engine);
                    }
                }
                // Bare latexmk reads its engine from user config
                // ($pdf_mode): never guess it.
                selected?
            }
            _ => return None,
        };

        let mut shell_escape = ExactEquationShellEscape::ProjectDefault;
        for flag in flags {
            match flag {
                "-shell-escape" | "-enable-write18" => {
                    shell_escape = ExactEquationShellEscape::Enabled
                }
                "-no-shell-escape" | "-disable-write18" => {
                    shell_escape = ExactEquationShellEscape::Disabled
                }
                "-shell-restricted" => shell_escape = ExactEquationShellEscape::Restricted,
                _ => {}
            }
        }
        Some(ExactEquationProfile { engine, shell_escape })
    }

    /// Fixed argv — nothing in it comes from the document text.
    pub fn arguments(&self, output_directory: &str) -> Vec<String> {
        let dash = if self.engine == ExactEquationEngine::LuaLaTeX { "--" } else { "-" };
        let mut arguments = vec![
            format!("{dash}interaction=batchmode"),
            format!("{dash}halt-on-error"),
            format!("{dash}output-directory={output_directory}"),
            format!("{dash}jobname={}", Self::JOB_NAME),
        ];
        match self.shell_escape {
            ExactEquationShellEscape::ProjectDefault => {}
            ExactEquationShellEscape::Enabled => arguments.push(format!("{dash}shell-escape")),
            ExactEquationShellEscape::Restricted => arguments.push(format!("{dash}shell-restricted")),
            ExactEquationShellEscape::Disabled => arguments.push(format!("{dash}no-shell-escape")),
        }
        arguments.push(format!("{}.tex", Self::JOB_NAME));
        arguments
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExactEquationRenderError {
    /// TeX exit ≠ 0 or a missing/empty PDF; `log` is the last 4 KiB of the
    /// job log (stdout tail when no .log was written).
    CompileFailed { log: String },
    TimedOut,
    Cancelled,
    OutputTooLarge,
    TemporaryDirectoryFailed,
    /// The process could not be spawned/waited (Swift surfaces the runner's
    /// thrown error directly; no exact-preview variant exists for it).
    RunnerFailed { detail: String },
}

impl fmt::Display for ExactEquationRenderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CompileFailed { log } => write!(f, "compile failed: {log}"),
            Self::TimedOut => write!(f, "timed out"),
            Self::Cancelled => write!(f, "cancelled"),
            Self::OutputTooLarge => write!(f, "output too large"),
            Self::TemporaryDirectoryFailed => write!(f, "temporary directory failed"),
            Self::RunnerFailed { detail } => write!(f, "process runner failed: {detail}"),
        }
    }
}
impl std::error::Error for ExactEquationRenderError {}

/// Unique request subdirectory without a uuid dependency — PID + counter
/// cannot collide within a process, and the base is per-workspace.
static REQUEST_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Clone)]
pub struct ExactEquationRenderer {
    pub profile: ExactEquationProfile,
    /// The main document's directory: `\input`/`\usepackage` of project
    /// files in the preamble resolve through TEXINPUTS, never by running
    /// inside the project.
    pub main_directory: PathBuf,
    /// Host additions (PATH to the TeX distribution).
    pub environment: HashMap<String, String>,
    /// Opaque per-workspace token keeping concurrent windows apart.
    pub workspace_token: String,
    pub timeout: Duration,
}

impl ExactEquationRenderer {
    pub const MAXIMUM_PDF_BYTES: usize = 4 * 1024 * 1024;

    pub fn new(
        profile: ExactEquationProfile,
        main_directory: PathBuf,
        environment: HashMap<String, String>,
        workspace_token: String,
    ) -> Self {
        Self {
            profile,
            main_directory,
            environment,
            workspace_token,
            timeout: Duration::from_secs(20),
        }
    }

    /// `$XDG_RUNTIME_DIR/pitex-equation` when the runtime dir is valid and
    /// owned by us, else `$TMPDIR/pitex-equation-<uid>` — then `<workspace>`.
    /// The result is only a *candidate*: [`render`] re-validates the base
    /// before every use (no writes under attacker-owned paths).
    pub fn workspace_root(token: &str) -> PathBuf {
        #[cfg(unix)]
        let uid = unsafe { libc::getuid() };
        #[cfg(not(unix))]
        let uid = 0u32;
        // XDG_RUNTIME_DIR is per-user, private, and (usually) tmpfs — prefer
        // it over world-writable /tmp when it meets the safety bar.
        let parent = std::env::var_os("XDG_RUNTIME_DIR")
            .map(PathBuf::from)
            .filter(|dir| Self::is_safe_runtime_dir(dir))
            .map(|dir| dir.join("pitex-equation"))
            .unwrap_or_else(|| std::env::temp_dir().join(format!("pitex-equation-{uid}")));
        parent.join(token)
    }

    /// The parent we drop request directories into must be ours: a real
    /// directory (never a symlink), owned by this uid, no group/other bits.
    /// On `cfg(unix)` hosts; elsewhere owner checks don't exist, so only the
    /// directory/symlink shape is verified.
    fn is_safe_private_dir(path: &std::path::Path) -> bool {
        let Ok(meta) = std::fs::symlink_metadata(path) else { return false };
        if !meta.is_dir() || meta.file_type().is_symlink() {
            return false;
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            if meta.uid() != unsafe { libc::getuid() } || meta.mode() & 0o077 != 0 {
                return false;
            }
        }
        true
    }

    fn is_safe_runtime_dir(path: &std::path::Path) -> bool {
        Self::is_safe_private_dir(path)
    }

    /// Compiles `document` and returns the PDF bytes. The request directory
    /// is always removed, including on cancellation (the process group is
    /// terminated first by [`ProcessRunner`]).
    pub fn render(
        &self,
        document: &str,
        cancel: Option<&CancellationToken>,
    ) -> Result<Vec<u8>, ExactEquationRenderError> {
        let base = Self::workspace_root(&self.workspace_token);
        // The base's parent (pitex-equation / pitex-equation-<uid>) is
        // validated BEFORE the base is created under it — otherwise a
        // foreign-owned lax dir would already receive our writes.
        let parent = base.parent().ok_or(ExactEquationRenderError::TemporaryDirectoryFailed)?;
        create_private_dir(parent).map_err(|_| ExactEquationRenderError::TemporaryDirectoryFailed)?;
        if !Self::is_safe_private_dir(parent) {
            return Err(ExactEquationRenderError::TemporaryDirectoryFailed);
        }
        create_private_dir(&base).map_err(|_| ExactEquationRenderError::TemporaryDirectoryFailed)?;
        // A pre-created base under shared /tmp may be foreign-owned, lax or
        // a symlink — fail closed rather than trust it.
        if !Self::is_safe_private_dir(&base) {
            return Err(ExactEquationRenderError::TemporaryDirectoryFailed);
        }
        let request_id = REQUEST_COUNTER.fetch_add(1, Ordering::Relaxed);
        let directory = base.join(format!("{}-{request_id}", std::process::id()));
        create_private_dir(&directory).map_err(|_| ExactEquationRenderError::TemporaryDirectoryFailed)?;
        if !Self::is_safe_private_dir(&directory) {
            return Err(ExactEquationRenderError::TemporaryDirectoryFailed);
        }
        let result = self.render_in(&directory, document, cancel);
        let _ = std::fs::remove_dir_all(&directory); // Swift `defer`
        result
    }

    fn render_in(
        &self,
        directory: &std::path::Path,
        document: &str,
        cancel: Option<&CancellationToken>,
    ) -> Result<Vec<u8>, ExactEquationRenderError> {
        let source = directory.join(format!("{}.tex", ExactEquationProfile::JOB_NAME));
        std::fs::write(&source, document.as_bytes())
            .map_err(|_| ExactEquationRenderError::TemporaryDirectoryFailed)?;

        let mut overrides = self.environment.clone();
        let inherited = overrides
            .get("TEXINPUTS")
            .cloned()
            .or_else(|| std::env::var("TEXINPUTS").ok());
        // Trailing separator keeps the distribution's default search path.
        let directory_string = directory.to_string_lossy().into_owned();
        overrides.insert(
            "TEXINPUTS".to_string(),
            format!("{}:{}", self.main_directory.to_string_lossy(), inherited.unwrap_or_default()),
        );
        // Paranoid writes: \openout stays inside the request directory even
        // if a user texmf.cnf relaxed it.
        overrides.insert("openout_any".to_string(), "p".to_string());

        // ponytail: stdout capture is UNBOUNDED in bytes — batchmode
        // silences the engine itself, but a shell-escape child can still
        // write until the 20 s timeout. If that matters, ProcessRunner
        // needs a discard/cap option; the job .log is tailed on failure.
        let plan = DirectCommandPlan::new(
            self.profile.engine.raw_value().to_string(),
            self.profile.arguments(&directory_string),
            WorkingDirectoryPolicy::Explicit(directory_string.clone()),
            EnvironmentPolicy::Inherit { overrides },
        )
        .map_err(|error| ExactEquationRenderError::RunnerFailed { detail: error.to_string() })?;
        let result = ProcessRunner::default()
            .run(&plan, directory, None, Some(self.timeout), cancel, None)
            .map_err(|error: ProcessRunnerError| ExactEquationRenderError::RunnerFailed {
                detail: error.to_string(),
            })?;
        match result.stop_reason {
            ProcessStopReason::TimedOut => return Err(ExactEquationRenderError::TimedOut),
            ProcessStopReason::Cancelled => return Err(ExactEquationRenderError::Cancelled),
            ProcessStopReason::Completed => {}
        }
        if cancel.map_or(false, |t| t.is_cancelled()) {
            return Err(ExactEquationRenderError::Cancelled);
        }
        let pdf = directory.join(format!("{}.pdf", ExactEquationProfile::JOB_NAME));
        let data = std::fs::read(&pdf).ok();
        let failed = result.termination != ProcessTermination::Exited { code: 0 }
            || data.as_ref().map_or(true, |d| d.is_empty());
        if failed {
            // batchmode puts the diagnostic stream in <jobname>.log; read
            // only the tail (no unbounded capture).
            let log_path = directory.join(format!("{}.log", ExactEquationProfile::JOB_NAME));
            let log = tail_bytes(&log_path, 4096).unwrap_or_else(|| {
                let out = &result.standard_output;
                out[out.len().saturating_sub(4096)..].to_vec()
            });
            return Err(ExactEquationRenderError::CompileFailed {
                log: String::from_utf8_lossy(&log).into_owned(),
            });
        }
        let data = data.unwrap();
        if data.len() > Self::MAXIMUM_PDF_BYTES {
            return Err(ExactEquationRenderError::OutputTooLarge);
        }
        Ok(data)
    }

    /// Removes this workspace's leftovers (a crashed or killed app).
    pub fn remove_workspace_artifacts(token: &str) {
        // Never recursively delete inside a foreign-owned/symlinked root:
        // re-validate the parent chain before touching it.
        let root = Self::workspace_root(token);
        if root.parent().map_or(false, |p| Self::is_safe_private_dir(p))
            && Self::is_safe_private_dir(&root)
        {
            let _ = std::fs::remove_dir_all(&root);
        }
    }
}

/// 0700 like the Swift `posixPermissions` attribute.
fn create_private_dir(path: &std::path::Path) -> std::io::Result<()> {
    let mut builder = std::fs::DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder.create(path)
}


/// Last `n` bytes of a file without loading it whole.
fn tail_bytes(path: &std::path::Path, n: u64) -> Option<Vec<u8>> {
    use std::io::{Read, Seek, SeekFrom};
    let mut file = std::fs::File::open(path).ok()?;
    let len = file.metadata().ok()?.len();
    file.seek(SeekFrom::End(-(n.min(len) as i64))).ok()?;
    let mut buf = Vec::new();
    file.take(n).read_to_end(&mut buf).ok()?;
    Some(buf)
}