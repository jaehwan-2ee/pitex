//! Port of `SyncTeXSupport.swift` — the `synctex` CLI runner with raw-output
//! normalization into the strict `SyncTeXQueryParser` record shape, and the
//! fingerprint binding derived from the `.synctex(.gz)` bytes.

use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

use build_core::{
    DirectCommandPlan, EnvironmentPolicy, ProcessRunner, ProcessTermination,
    WorkingDirectoryPolicy,
};
use synctex_core::{
    ExactSyncTeXQuerySelector, ForwardSyncQuery, InverseSyncQuery, NormalizedSourcePath,
    PDFLocation, PDFPoint, SourceLocation, SyncTeXOutputBinding, SyncTeXQueryCandidate,
    SyncTeXQueryParser, SyncTeXRevision, SyncTeXInput,
};

use crate::model::standardize;

#[derive(Debug)]
pub enum SyncTeXSupportError {
    ToolUnavailable,
    NoBinding,
    NoResult,
    StaleResult,
    AmbiguousResult(usize),
    MissingMetadata,
    Query(String),
}
impl std::fmt::Display for SyncTeXSupportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ToolUnavailable => {
                write!(f, "The synctex command-line tool is not installed.")
            }
            Self::NoBinding => write!(
                f,
                "SyncTeX metadata has not been captured for the latest build."
            ),
            Self::NoResult => write!(f, "SyncTeX found no matching location."),
            Self::StaleResult => write!(f, "stale"),
            Self::AmbiguousResult(n) => write!(f, "ambiguous:{n}"),
            Self::MissingMetadata => write!(
                f,
                "The build did not produce a .synctex file next to the PDF."
            ),
            Self::Query(e) => write!(f, "{e}"),
        }
    }
}
impl std::error::Error for SyncTeXSupportError {}

#[derive(Debug, Clone)]
pub struct SyncTeXBinding {
    pub revision: SyncTeXRevision,
    pub output_hash: String,
    pub pdf_url: PathBuf,
    pub project_root: PathBuf,
    /// Absolute directory the build was invoked in — the base every
    /// recorded *relative* `Input:` path resolves against. For a
    /// `.pitex-live` output this is the source's directory, never the
    /// PDF's directory.
    pub source_root: PathBuf,
    /// Raw `Input:` path → relocated file under the project root.
    pub source_paths: HashMap<String, PathBuf>,
}

/// mtime alone can't flag a rewrite that restores it; the inode + ctime
/// pair pins the file on Unix. Stable `std` exposes no Windows equivalent
/// (`MetadataExt::file_index`/`change_time` are still unstable), so the
/// Windows stamp is size + last-write time: a rebuild always moves the
/// write time, which is what the cache needs to see.
#[cfg(unix)]
type FileStamp = (u64, u64, u64, i64, i64, i64, i64);
#[cfg(windows)]
type FileStamp = (u64, u64);

#[cfg(unix)]
fn file_stamp(path: &Path) -> Result<FileStamp, SyncTeXSupportError> {
    use std::os::unix::fs::MetadataExt;
    let m = std::fs::metadata(path).map_err(|_| SyncTeXSupportError::MissingMetadata)?;
    Ok((m.dev(), m.ino(), m.len(), m.mtime(), m.mtime_nsec(), m.ctime(), m.ctime_nsec()))
}

#[cfg(windows)]
fn file_stamp(path: &Path) -> Result<FileStamp, SyncTeXSupportError> {
    use std::os::windows::fs::MetadataExt;
    let m = std::fs::metadata(path).map_err(|_| SyncTeXSupportError::MissingMetadata)?;
    Ok((m.file_size(), m.last_write_time()))
}

/// `SyncTeXRunner` — resolves the tool once, runs queries, normalizes output.
pub struct SyncTeXRunner {
    runner: ProcessRunner,
    tool_path: Mutex<Option<String>>,
    digest_cache: Mutex<HashMap<PathBuf, (FileStamp, String)>>,
}
impl SyncTeXRunner {
    const ALLOWED_FIELDS: [&'static str; 11] = [
        "Input", "Line", "Column", "Output", "Page", "x", "y", "h", "v", "W", "H",
    ];

    pub fn new() -> Self {
        Self {
            runner: ProcessRunner::default(),
            tool_path: Mutex::new(None),
            digest_cache: Mutex::new(HashMap::new()),
        }
    }

    /// `refreshBinding` — resolved root/pdf + SHA-256 output hash + the
    /// `.synctex` fingerprint (first 8 digest bytes, big-endian).
    /// `main_relative` is the project-relative build source
    /// ("manuscript/main.tex"): recorded *relative* inputs resolve
    /// against its directory — the directory the engine ran in — never
    /// the PDF's output directory.
    pub fn refresh_binding(
        &self,
        project_root: &Path,
        pdf_url: &Path,
        build_id: &str,
        main_relative: &str,
    ) -> Result<SyncTeXBinding, SyncTeXSupportError> {
        self.resolved_tool()?;
        let root = standardize(project_root.to_path_buf());
        let pdf = standardize(pdf_url.to_path_buf());
        self.digest_cache.lock().unwrap().clear();
        let output_hash = self.file_digest(&pdf)?;
        let fingerprint = self.synctex_fingerprint(&pdf)?;
        let plain = pdf.with_extension("synctex");
        let gz = pdf.with_extension("synctex.gz");
        let metadata: Vec<u8> = if gz.exists() {
            // Native decode of the gzipped metadata — the Linux equivalent
            // of `/usr/bin/gzip -cd` in the macOS implementation.
            let data =
                std::fs::read(&gz).map_err(|_| SyncTeXSupportError::MissingMetadata)?;
            let mut out = Vec::new();
            flate2::read::MultiGzDecoder::new(&data[..])
                .read_to_end(&mut out)
                .map_err(|_| SyncTeXSupportError::MissingMetadata)?;
            out
        } else {
            std::fs::read(&plain).map_err(|_| SyncTeXSupportError::MissingMetadata)?
        };
        let text = String::from_utf8_lossy(&metadata);
        // Parse inputs line by line — one malformed or unresolvable entry
        // (a relative path with a leading `..`, an empty path) is skipped
        // rather than discarding the whole metadata file.
        let mut inputs: Vec<SyncTeXInput> = Vec::new();
        let mut seen_tags = std::collections::HashSet::new();
        for line in text.lines().filter(|l| l.starts_with("Input:")) {
            let body = &line["Input:".len()..];
            let parsed = body.find(':').and_then(|separator| {
                let tag = body[..separator].trim().parse::<i64>().ok()?;
                let path = NormalizedSourcePath::new(body[separator + 1..].trim()).ok()?;
                seen_tags.insert(tag).then(|| SyncTeXInput { tag, path })
            });
            if let Some(input) = parsed {
                inputs.push(input);
            }
        }
        if inputs.is_empty() {
            return Err(SyncTeXSupportError::MissingMetadata);
        }
        inputs.sort_by_key(|i| i.tag);
        // The directory the engine was invoked in — `.` components
        // normalized, `..` kept only where they can't escape the root.
        let source_root = standardize(root.join(
            Path::new(main_relative)
                .parent()
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_default(),
        ));
        let original_root = derive_original_root(main_relative, &inputs);
        let source_paths =
            map_source_paths(&root, &source_root, original_root.as_deref(), &inputs);
        Ok(SyncTeXBinding {
            revision: SyncTeXRevision::new(build_id, fingerprint)
                .map_err(|e| SyncTeXSupportError::Query(e.to_string()))?,
            output_hash,
            pdf_url: pdf,
            project_root: root,
            source_root,
            source_paths,
        })
    }

    /// `forward` — `synctex view -i line:col:source -o pdf`.
    pub fn forward(
        &self,
        binding: &SyncTeXBinding,
        source_url: &Path,
        line: i64,
        column: i64,
    ) -> Result<SyncTeXQueryCandidate, SyncTeXSupportError> {
        let synctex = self.resolved_tool()?;
        let resolved_source = standardize(source_url.to_path_buf());
        self.validate(binding)?;
        let input_path = binding
            .source_paths
            .iter()
            .find(|(_, v)| **v == resolved_source)
            .map(|(k, _)| k.clone())
            .unwrap_or_else(|| resolved_source.to_string_lossy().into_owned());
        let plan = DirectCommandPlan::new(
            synctex,
            vec![
                "view".into(),
                "-i".into(),
                format!("{line}:{column}:{input_path}"),
                "-o".into(),
                binding.pdf_url.to_string_lossy().into_owned(),
            ],
            // The CLI resolves recorded relative Input paths against the
            // working directory — the invocation directory, not the PDF's.
            WorkingDirectoryPolicy::Explicit(binding.source_root.to_string_lossy().into_owned()),
            EnvironmentPolicy::Inherit {
                overrides: Default::default(),
            },
        )
        .map_err(|e| SyncTeXSupportError::Query(e.to_string()))?;
        let result = self
            .runner
            .run(
                &plan,
                &binding.project_root,
                None,
                Some(Duration::from_secs(15)),
                None,
                None,
            )
            .map_err(|e| SyncTeXSupportError::Query(e.to_string()))?;
        self.validate(binding)?;
        let normalized = Self::normalize(
            &String::from_utf8_lossy(&result.standard_output),
            &[
                ("Input", resolved_source.to_string_lossy().into_owned()),
                ("Line", line.to_string()),
                ("Column", column.to_string()),
                ("Output", binding.pdf_url.to_string_lossy().into_owned()),
            ],
            binding,
        );
        let document = SyncTeXQueryParser::parse(
            &normalized,
            &binding.project_root.to_string_lossy(),
            &SyncTeXOutputBinding::new(binding.revision.clone(), &binding.output_hash)
                .map_err(|e| SyncTeXSupportError::Query(e.to_string()))?,
        )
        .map_err(|e| SyncTeXSupportError::Query(e.to_string()))?;
        let source_path = NormalizedSourcePath::new(
            resolved_source
                .to_string_lossy()
                .strip_prefix(&format!("{}/", binding.project_root.to_string_lossy()))
                .unwrap_or(&resolved_source.to_string_lossy()),
        )
        .map_err(|e| SyncTeXSupportError::Query(e.to_string()))?;
        let pdf_path = NormalizedSourcePath::new(
            binding
                .pdf_url
                .to_string_lossy()
                .strip_prefix(&format!("{}/", binding.project_root.to_string_lossy()))
                .unwrap_or(&binding.pdf_url.to_string_lossy()),
        )
        .map_err(|e| SyncTeXSupportError::Query(e.to_string()))?;
        let candidate = ExactSyncTeXQuerySelector::forward(
            &document.candidates[..document.candidates.len().min(1)],
            &ForwardSyncQuery {
                revision: binding.revision.clone(),
                source: SourceLocation::new(source_path, line, column)
                    .map_err(|e| SyncTeXSupportError::Query(e.to_string()))?,
                expected_pdf: pdf_path,
            },
            &binding.output_hash,
        )
        .map_err(map_selector_error)?;
        Ok(candidate)
    }

    /// `inverse` — `synctex edit -o page:x:y:pdf`.
    pub fn inverse(
        &self,
        binding: &SyncTeXBinding,
        page: i64,
        point: &PDFPoint,
    ) -> Result<SyncTeXQueryCandidate, SyncTeXSupportError> {
        let synctex = self.resolved_tool()?;
        self.validate(binding)?;
        let plan = DirectCommandPlan::new(
            synctex,
            vec![
                "edit".into(),
                "-o".into(),
                format!(
                    "{page}:{}:{}:{}",
                    point.x,
                    point.y,
                    binding.pdf_url.to_string_lossy()
                ),
            ],
            WorkingDirectoryPolicy::Explicit(binding.source_root.to_string_lossy().into_owned()),
            EnvironmentPolicy::Inherit {
                overrides: Default::default(),
            },
        )
        .map_err(|e| SyncTeXSupportError::Query(e.to_string()))?;
        let result = self
            .runner
            .run(
                &plan,
                &binding.project_root,
                None,
                Some(Duration::from_secs(15)),
                None,
                None,
            )
            .map_err(|e| SyncTeXSupportError::Query(e.to_string()))?;
        self.validate(binding)?;
        let normalized = Self::normalize(
            &String::from_utf8_lossy(&result.standard_output),
            &[
                ("Column", "0".to_string()),
                ("Page", page.to_string()),
                ("x", point.x.to_string()),
                ("y", point.y.to_string()),
                ("h", point.x.to_string()),
                ("v", point.y.to_string()),
                ("W", "0".to_string()),
                ("H", "0".to_string()),
                ("Output", binding.pdf_url.to_string_lossy().into_owned()),
            ],
            binding,
        );
        let document = SyncTeXQueryParser::parse(
            &normalized,
            &binding.project_root.to_string_lossy(),
            &SyncTeXOutputBinding::new(binding.revision.clone(), &binding.output_hash)
                .map_err(|e| SyncTeXSupportError::Query(e.to_string()))?,
        )
        .map_err(|e| SyncTeXSupportError::Query(e.to_string()))?;
        let pdf_path = NormalizedSourcePath::new(
            binding
                .pdf_url
                .to_string_lossy()
                .strip_prefix(&format!("{}/", binding.project_root.to_string_lossy()))
                .unwrap_or(&binding.pdf_url.to_string_lossy()),
        )
        .map_err(|e| SyncTeXSupportError::Query(e.to_string()))?;
        let candidate = ExactSyncTeXQuerySelector::inverse(
            &document.candidates[..document.candidates.len().min(1)],
            &InverseSyncQuery {
                revision: binding.revision.clone(),
                pdf: PDFLocation::new(pdf_path, page, point.clone())
                    .map_err(|e| SyncTeXSupportError::Query(e.to_string()))?,
            },
            &binding.output_hash,
            2.0,
        )
        .map_err(map_selector_error)?;
        Ok(candidate)
    }

    /// `validate` — the fingerprint and the PDF bytes must still match the
    /// bound artifacts or every query result is stale.
    fn validate(&self, binding: &SyncTeXBinding) -> Result<(), SyncTeXSupportError> {
        let fingerprint = self.synctex_fingerprint(&binding.pdf_url)
            .map_err(|_| SyncTeXSupportError::StaleResult)?;
        let digest = self.file_digest(&binding.pdf_url).map_err(|_| SyncTeXSupportError::StaleResult)?;
        if fingerprint != binding.revision.fingerprint || digest != binding.output_hash
        {
            return Err(SyncTeXSupportError::StaleResult);
        }
        Ok(())
    }

    /// `normalize` — verbatim: drop non-allowed fields, fill missing from the
    /// query, prepend version+fingerprint headers, resolve Input/Output paths.
    fn normalize(
        raw: &str,
        query_fields: &[(&str, String)],
        binding: &SyncTeXBinding,
    ) -> String {
        let mut normalized = format!(
            "SyncTeX Version:1\nSyncTeX Fingerprint:{}\n",
            binding.revision.fingerprint
        );
        let mut in_block = false;
        let mut fields: Vec<(String, String)> = Vec::new();

        let flush = |fields: &[(String, String)], normalized: &mut String| {
            if fields.is_empty() {
                return;
            }
            normalized.push_str("SyncTeX result begin\n");
            for key in Self::ALLOWED_FIELDS {
                let value = fields
                    .iter()
                    .find(|(k, _)| k == key)
                    .map(|(_, v)| v.clone())
                    .or_else(|| {
                        query_fields
                            .iter()
                            .find(|(k, _)| *k == key)
                            .map(|(_, v)| v.clone())
                    });
                if let Some(value) = value {
                    normalized.push_str(&format!("{key}:{value}\n"));
                }
            }
            normalized.push_str("SyncTeX result end\n");
        };

        for raw_line in raw.split('\n') {
            let line = raw_line.strip_suffix('\r').unwrap_or(raw_line);
            if line == "SyncTeX result begin" {
                in_block = true;
                fields.clear();
                continue;
            }
            if line == "SyncTeX result end" {
                if in_block {
                    flush(&fields, &mut normalized);
                }
                in_block = false;
                continue;
            }
            if !in_block {
                continue;
            }
            let Some(sep) = line.find(':') else { continue };
            let key = &line[..sep];
            let mut value = line[sep + 1..].to_string();
            // The CLI encloses all ranked hits in one begin/end pair. Each
            // repeated Output starts another hit; keep that ranking instead
            // of overwriting the best hit with the last one.
            if key == "Output" && fields.iter().any(|(k, _)| k == "Output") {
                flush(&fields, &mut normalized);
                fields.clear();
            }
            // `synctex` emits Column:-1 for whole-line hits; the normalized
            // format requires nonnegative values, so the query fallback wins.
            if key == "Column" && value.starts_with('-') {
                continue;
            }
            // Canonicalize CLI paths exactly like the binding root, relocating
            // recorded Input paths through the source map when possible.
            // Leave invalid paths intact for the strict parser to reject.
            if (key == "Input" || key == "Output")
                && !value.contains('\\')
                && !value.contains('\0')
            {
                let path = NormalizedSourcePath::new(&value)
                    .map(|p| p.value)
                    .unwrap_or_else(|_| value.clone());
                let mapped = if key == "Input" {
                    binding.source_paths.get(&path).cloned()
                } else {
                    None
                };
                let candidate = mapped.unwrap_or_else(|| {
                    if Path::new(&value).is_absolute() {
                        PathBuf::from(&value)
                    } else {
                        binding.source_root.join(&value)
                    }
                });
                value = standardize(candidate).to_string_lossy().into_owned();
            }
            if Self::ALLOWED_FIELDS.contains(&key) {
                fields.retain(|(k, _)| k != key);
                fields.push((key.to_string(), value));
            }
        }
        normalized
    }

    /// First 8 SHA-256 bytes of the `.synctex(.gz)` file next to the PDF.
    fn synctex_fingerprint(&self, pdf_url: &Path) -> Result<u64, SyncTeXSupportError> {
        let gz = pdf_url.with_extension("synctex.gz");
        let plain = pdf_url.with_extension("synctex");
        let metadata = if gz.exists() { gz } else { plain };
        let digest = self.file_digest(&metadata)?;
        u64::from_str_radix(&digest[..16], 16).map_err(|_| SyncTeXSupportError::MissingMetadata)
    }

    fn file_digest(&self, path: &Path) -> Result<String, SyncTeXSupportError> {
        let before = file_stamp(path)?;
        if let Some((stamp, digest)) = self.digest_cache.lock().unwrap().get(path) {
            if *stamp == before { return Ok(digest.clone()); }
        }
        let data = std::fs::read(path).map_err(|_| SyncTeXSupportError::MissingMetadata)?;
        if data.is_empty() { return Err(SyncTeXSupportError::MissingMetadata); }
        let digest = sha256_hex(&data);
        if file_stamp(path)? != before { return Err(SyncTeXSupportError::StaleResult); }
        self.digest_cache.lock().unwrap().insert(path.to_path_buf(), (before, digest.clone()));
        Ok(digest)
    }

    /// Tool resolution: fixed candidates, then `env synctex --version` probe.
    fn resolved_tool(&self) -> Result<String, SyncTeXSupportError> {
        if let Some(path) = self.tool_path.lock().unwrap().clone() {
            return Ok(path);
        }
        #[cfg(unix)]
        let mut candidates = vec![
            "/usr/bin/synctex".to_string(),
            "/usr/local/bin/synctex".to_string(),
            "/home/linuxbrew/.linuxbrew/bin/synctex".to_string(),
        ];
        #[cfg(windows)]
        let mut candidates = vec![
            "C:/texlive/2025/bin/windows/synctex.exe".to_string(),
            "C:/texlive/2024/bin/windows/synctex.exe".to_string(),
            "C:/Program Files/MiKTeX/miktex/bin/x64/synctex.exe".to_string(),
        ];
        candidates.extend(
            crate::model::texlive_bin_dirs()
                .iter()
                .map(|d| d.join(synctex_binary_name()).to_string_lossy().into_owned()),
        );
        for candidate in &candidates {
            if Path::new(candidate).exists() && is_executable(candidate) {
                *self.tool_path.lock().unwrap() = Some(candidate.clone());
                return Ok(candidate.clone());
            }
        }
        // PATH probe — `/usr/bin/env` on Unix, the bare name on Windows
        // (resolved through the runner's own PATH/PATHEXT search).
        #[cfg(unix)]
        let (probe_exe, probe_args) = ("/usr/bin/env", vec!["synctex".into(), "--version".into()]);
        #[cfg(windows)]
        let (probe_exe, probe_args) = ("synctex", vec!["--version".into()]);
        let plan = DirectCommandPlan::new(
            probe_exe,
            probe_args,
            WorkingDirectoryPolicy::ProjectRoot,
            EnvironmentPolicy::Inherit {
                overrides: Default::default(),
            },
        )
        .map_err(|e| SyncTeXSupportError::Query(e.to_string()))?;
        if let Ok(probe) = self.runner.run(
            &plan,
            &std::env::temp_dir(),
            None,
            Some(Duration::from_secs(10)),
            None,
            None,
        ) {
            if let ProcessTermination::Exited { code: 0 } = probe.termination {
                *self.tool_path.lock().unwrap() = Some("synctex".to_string());
                return Ok("synctex".to_string());
            }
        }
        Err(SyncTeXSupportError::ToolUnavailable)
    }
}

fn map_selector_error(e: synctex_core::SyncTeXQueryError) -> SyncTeXSupportError {
    use synctex_core::SyncTeXQueryError::*;
    match e {
        StaleResult => SyncTeXSupportError::StaleResult,
        AmbiguousMatch { count } => SyncTeXSupportError::AmbiguousResult(count),
        NoMatch => SyncTeXSupportError::Query("no_match".into()),
        other => SyncTeXSupportError::Query(format!("{other}")),
    }
}

/// TeX Live installs `synctex` on Unix and `synctex.exe` on Windows.
fn synctex_binary_name() -> &'static str {
    if cfg!(windows) {
        "synctex.exe"
    } else {
        "synctex"
    }
}

#[cfg(unix)]
fn is_executable(path: &str) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path)
        .map(|m| m.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(windows)]
fn is_executable(path: &str) -> bool {
    Path::new(path).is_file()
}

// ─── SHA-256 (reuses the shared implementation in tex-domain) ───────────────

/// Lexical normalization of a *recorded* path string — `.`/`..` resolved
/// against `/` separators regardless of the host. SyncTeX metadata
/// records POSIX paths on remote devices and native paths locally;
/// cross-referencing them must never depend on the client's separators.
fn normalize_recorded(raw: &str) -> String {
    let slashed = raw.replace('\\', "/");
    let absolute = slashed.starts_with('/');
    let mut parts: Vec<&str> = Vec::new();
    for part in slashed.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                if parts.last().is_some_and(|p| *p != "..") {
                    parts.pop();
                } else if !absolute {
                    parts.push("..");
                }
            }
            p => parts.push(p),
        }
    }
    format!("{}{}", if absolute { "/" } else { "" }, parts.join("/"))
}

/// The project root recorded on the build device — remote builds write
/// device paths. Input 1 is the original main source: normalize its
/// `.`/`..` segments lexically, then strip the known project-relative
/// main. The derivation works on the recorded STRING (`/`-separated)
/// so a POSIX device path maps identically on every client platform —
/// never guessed by basename.
fn derive_original_root(main_relative: &str, inputs: &[SyncTeXInput]) -> Option<String> {
    let suffix = format!("/{}", main_relative.replace('\\', "/"));
    let normalized = inputs
        .iter()
        .find(|i| i.tag == 1)
        .map(|i| normalize_recorded(&i.path.value))?;
    normalized.strip_suffix(&suffix).map(str::to_string)
}

/// Relocate every recorded `Input:` path into the project root.
/// Containment uses native `Path` components; remote-root suffix
/// matching uses the recorded `/`-separated string — the two never mix.
/// Mapped targets must exist on disk and stay inside the root.
fn map_source_paths(
    root: &Path,
    source_root: &Path,
    original_root: Option<&str>,
    inputs: &[SyncTeXInput],
) -> HashMap<String, PathBuf> {
    let mut source_paths = HashMap::new();
    for input in inputs {
        let raw = input.path.value.clone();
        let mapped = if Path::new(&raw).is_absolute() {
            // Native-absolute — a local path on this client's platform
            // (and on Unix the remote POSIX path parses the same way, so
            // it falls through to the remote-root suffix when it isn't
            // under the local root).
            let normalized = standardize(PathBuf::from(&raw));
            if normalized.starts_with(root) {
                normalized
            } else {
                let recorded = normalize_recorded(&raw);
                match original_root
                    .and_then(|o| recorded.strip_prefix(&format!("{o}/")))
                {
                    Some(tail) => standardize(root.join(tail)),
                    None => continue,
                }
            }
        } else if raw.starts_with('/') {
            // Rooted POSIX but not native-absolute — the signature of a
            // remote device path read on a Windows client. Normalize it
            // lexically and map through the remote original root.
            let recorded = normalize_recorded(&raw);
            match original_root
                .and_then(|o| recorded.strip_prefix(&format!("{o}/")))
            {
                Some(tail) => standardize(root.join(tail)),
                None => continue,
            }
        } else {
            // Relative input — recorded relative to the invocation
            // directory, not the PDF directory.
            standardize(source_root.join(&raw))
        };
        if !mapped.starts_with(root) || std::fs::File::open(&mapped).is_err() {
            continue;
        }
        source_paths.insert(raw, mapped);
    }
    source_paths
}

fn sha256_hex(data: &[u8]) -> String {
    tex_domain::sha256(data)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

#[cfg(test)]
mod binding_tests {
    use super::*;

    /// Self-contained project tree under /tmp; removed on drop.
    struct Dir(PathBuf);
    impl Dir {
        fn new(tag: &str) -> Self {
            let path = std::env::temp_dir().join(format!(
                "pitex-stx-{tag}-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            std::fs::create_dir_all(&path).unwrap();
            Self(path)
        }
        fn write(&self, rel: &str, text: &str) -> PathBuf {
            let path = self.0.join(rel);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, text).unwrap();
            path
        }
    }
    impl Drop for Dir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn input(tag: i64, path: &str) -> SyncTeXInput {
        SyncTeXInput {
            tag,
            path: NormalizedSourcePath::new(path).unwrap(),
        }
    }

    /// A Windows-local absolute spelling for assertions — on Unix the
    /// native absolute form is the POSIX path itself; on Windows the
    /// remote POSIX inputs below still exercise the host-lexical branch.
    /// These tests build `SyncTeXInput`s directly and exercise the pure
    /// mapping functions, so they run on Windows CI without a synctex
    /// binary.
    #[test]
    fn nested_relative_inputs_use_invocation_directory() {
        let dir = Dir::new("nested");
        dir.write("manuscript/main.tex", "main");
        dir.write("manuscript/sections/intro.tex", "nested");
        // Same spelling under the project root — must NOT win.
        dir.write("sections/intro.tex", "collider");
        dir.write("shared/common.tex", "common");
        let root = standardize(dir.0.clone());
        let source_root = standardize(root.join("manuscript"));
        let root_slash = root.to_string_lossy().replace('\\', "/");
        let inputs = vec![
            input(1, &format!("{root_slash}/manuscript/./main.tex")),
            input(2, "sections/intro.tex"),
            input(3, &format!("{root_slash}/manuscript/../shared/common.tex")),
            input(4, &format!("{root_slash}/manuscript/sections/intro.tex")),
            input(5, &format!("{root_slash}/sections/intro.tex")),
        ];
        let original = derive_original_root("manuscript/main.tex", &inputs);
        assert_eq!(original.as_deref(), Some(root_slash.as_str()));
        let paths = map_source_paths(&root, &source_root, original.as_deref(), &inputs);
        // Relative input → manuscript/sections/intro.tex, not the root
        // collider.
        assert_eq!(
            paths["sections/intro.tex"],
            root.join("manuscript").join("sections").join("intro.tex")
        );
        // Absolute input under the root keeps identity — the root
        // collider is only reached through its own absolute spelling.
        assert_eq!(
            paths[&format!("{root_slash}/sections/intro.tex")],
            root.join("sections").join("intro.tex")
        );
        // `../` inside an absolute input normalizes to the shared sibling.
        assert_eq!(
            paths[&format!("{root_slash}/shared/common.tex")],
            root.join("shared").join("common.tex")
        );
    }

    /// Remote metadata records device paths. Input 1 — normalized for
    /// `.`/`..` first — pins the original project root; every other
    /// remote-absolute input maps into the local mirror by exact suffix
    /// (nested mains, `../shared` siblings) while paths outside that
    /// root are rejected. POSIX device spellings map identically on a
    /// Windows client, where they are not native-absolute.
    #[test]
    fn remote_absolute_inputs_map_by_original_root_suffix() {
        let dir = Dir::new("remote");
        dir.write("manuscript/main.tex", "main");
        dir.write("manuscript/sections/intro.tex", "nested");
        dir.write("shared/common.tex", "common");
        let root = standardize(dir.0.clone());
        let source_root = standardize(root.join("manuscript"));
        let inputs = vec![
            input(1, "/device/./proj/manuscript/main.tex"),
            input(2, "/device/proj/manuscript/sections/intro.tex"),
            input(3, "/device/proj/manuscript/../shared/common.tex"),
            input(4, "/elsewhere/proj/manuscript/sections/intro.tex"),
            input(5, "/device/proj/../outside.tex"),
        ];
        let original = derive_original_root("manuscript/main.tex", &inputs);
        assert_eq!(original.as_deref(), Some("/device/proj"));
        let paths = map_source_paths(&root, &source_root, original.as_deref(), &inputs);
        assert_eq!(
            paths["/device/proj/manuscript/sections/intro.tex"],
            root.join("manuscript").join("sections").join("intro.tex")
        );
        // Normalized: /device/proj/shared/common.tex → root/shared/...
        assert_eq!(
            paths["/device/proj/shared/common.tex"],
            root.join("shared").join("common.tex")
        );
        // Outside the derived remote root — and the `..` escape — never
        // map, even to a file that exists locally.
        assert!(!paths.contains_key("/elsewhere/proj/manuscript/sections/intro.tex"));
        assert!(!paths.contains_key("/device/outside.tex"));
    }

    /// Inputs that normalize outside the project root are rejected even
    /// when the target exists on disk — absolute `..` escapes and
    /// unresolvable relative paths alike.
    #[test]
    fn escaping_inputs_are_rejected() {
        let dir = Dir::new("escape");
        dir.write("manuscript/main.tex", "main");
        let root = standardize(dir.0.clone());
        let source_root = standardize(root.join("manuscript"));
        // A real file one level above the root.
        let outside = dir.0.parent().unwrap().join("escape-target.tex");
        std::fs::write(&outside, "outside").unwrap();
        let root_slash = root.to_string_lossy().replace('\\', "/");
        let inputs = vec![
            input(1, &format!("{root_slash}/manuscript/main.tex")),
            input(2, &format!("{root_slash}/../escape-target.tex")),
        ];
        let original = derive_original_root("manuscript/main.tex", &inputs);
        let paths = map_source_paths(&root, &source_root, original.as_deref(), &inputs);
        // The main mapped; the absolute escape did not.
        assert_eq!(paths.len(), 1);
        assert!(!paths
            .values()
            .any(|p| p.ends_with("escape-target.tex")));
        // A relative input carrying leading `..` can't even be
        // constructed — NormalizedSourcePath rejects it upstream.
        assert!(NormalizedSourcePath::new("../../../escape-target.tex").is_err());
        let _ = std::fs::remove_file(&outside);
    }
}

#[cfg(test)]
mod digest_cache_tests {
    use super::*;

    fn temp_file(tag: &str) -> PathBuf {
        std::env::temp_dir().join(format!("pitex-digest-{tag}-{}-{}", std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()))
    }

    /// An untouched file hits the cache; a rewrite moves the stamp and a
    /// delete makes the digest fail — the portable core of the cache.
    #[test]
    fn digest_cache_tracks_content_changes() {
        let path = temp_file("rewrite");
        std::fs::write(&path, b"first").unwrap();
        let runner = SyncTeXRunner::new();
        let original = runner.file_digest(&path).unwrap();
        assert_eq!(original, runner.file_digest(&path).unwrap());
        std::fs::write(&path, b"other content, longer").unwrap();
        assert_ne!(original, runner.file_digest(&path).unwrap());
        std::fs::remove_file(&path).unwrap();
        assert!(runner.file_digest(&path).is_err());
    }

    /// Same-size rewrite with the mtime forged back, done the way editors
    /// actually save — write sibling + rename over. The new inode changes
    /// the Unix stamp deterministically; the pure-truncate variant of this
    /// test relied on ctime moving, which a jiffy-granular clock can merge
    /// into one tick.
    #[cfg(unix)]
    #[test]
    fn digest_cache_detects_same_size_edits_with_restored_mtime() {
        let path = temp_file("forged");
        let staged = temp_file("forged-staged");
        std::fs::write(&path, b"first").unwrap();
        let modified = std::fs::metadata(&path).unwrap().modified().unwrap();
        let runner = SyncTeXRunner::new();
        let original = runner.file_digest(&path).unwrap();
        assert_eq!(original, runner.file_digest(&path).unwrap());
        std::fs::write(&staged, b"other").unwrap();
        std::fs::rename(&staged, &path).unwrap();
        std::fs::File::open(&path).unwrap().set_times(std::fs::FileTimes::new().set_modified(modified)).unwrap();
        assert_ne!(original, runner.file_digest(&path).unwrap());
        std::fs::remove_file(&path).unwrap();
        assert!(runner.file_digest(&path).is_err());
    }
}
