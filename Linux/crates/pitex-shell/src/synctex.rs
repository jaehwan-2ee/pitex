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
    SyncTeXQueryParser, SyncTeXRevision, SyncTeXTextParser,
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
    /// Raw `Input:` path → relocated file under the project root.
    pub source_paths: HashMap<String, PathBuf>,
}

/// `SyncTeXRunner` — resolves the tool once, runs queries, normalizes output.
pub struct SyncTeXRunner {
    runner: ProcessRunner,
    tool_path: Mutex<Option<String>>,
}
impl SyncTeXRunner {
    const ALLOWED_FIELDS: [&'static str; 11] = [
        "Input", "Line", "Column", "Output", "Page", "x", "y", "h", "v", "W", "H",
    ];

    pub fn new() -> Self {
        Self {
            runner: ProcessRunner::default(),
            tool_path: Mutex::new(None),
        }
    }

    /// `refreshBinding` — resolved root/pdf + SHA-256 output hash + the
    /// `.synctex` fingerprint (first 8 digest bytes, big-endian).
    pub fn refresh_binding(
        &self,
        project_root: &Path,
        pdf_url: &Path,
        build_id: &str,
    ) -> Result<SyncTeXBinding, SyncTeXSupportError> {
        self.resolved_tool()?;
        let root = standardize(project_root.to_path_buf());
        let pdf = standardize(pdf_url.to_path_buf());
        let pdf_data =
            std::fs::read(&pdf).map_err(|_| SyncTeXSupportError::MissingMetadata)?;
        let output_hash = sha256_hex(&pdf_data);
        let fingerprint = Self::synctex_fingerprint(&pdf)?;
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
        // Input tag 1 records the original main source. Preserve relative
        // paths beneath that directory when a downloaded project is moved;
        // never guess using just a chapter's filename.
        let text = String::from_utf8_lossy(&metadata);
        let input_lines = text
            .lines()
            .filter(|l| l.starts_with("Input:"))
            .collect::<Vec<_>>()
            .join("\n");
        let inputs = SyncTeXTextParser::parse(&input_lines)
            .map_err(|_| SyncTeXSupportError::MissingMetadata)?
            .inputs;
        let original_main = inputs
            .iter()
            .find(|i| i.tag == 1)
            .map(|i| i.path.value.clone());
        let original_dir = original_main.as_ref().map(|p| {
            Path::new(p)
                .parent()
                .map(|d| d.to_string_lossy().into_owned())
                .unwrap_or_default()
        });
        let pdf_dir = pdf.parent().map(Path::to_path_buf).unwrap_or_default();
        let mut source_paths = HashMap::new();
        for input in &inputs {
            let path = input.path.value.clone();
            let mut relative = path.clone();
            if path.starts_with('/') {
                if let Some(dir) = &original_dir {
                    if path.starts_with(&format!("{dir}/")) {
                        relative = path[dir.len() + 1..].to_string();
                    }
                }
            }
            let mapped = standardize(if Path::new(&relative).is_absolute() {
                PathBuf::from(&relative)
            } else {
                pdf_dir.join(&relative)
            });
            let under_root = mapped
                .to_string_lossy()
                .starts_with(&format!("{}/", root.to_string_lossy()));
            if !under_root || std::fs::File::open(&mapped).is_err() {
                continue;
            }
            source_paths.insert(path, mapped);
        }
        Ok(SyncTeXBinding {
            revision: SyncTeXRevision::new(build_id, fingerprint)
                .map_err(|e| SyncTeXSupportError::Query(e.to_string()))?,
            output_hash,
            pdf_url: pdf,
            project_root: root,
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
        Self::validate(binding)?;
        let input_path = binding
            .source_paths
            .iter()
            .find(|(_, v)| **v == resolved_source)
            .map(|(k, _)| k.clone())
            .unwrap_or_else(|| resolved_source.to_string_lossy().into_owned());
        let pdf_dir = binding
            .pdf_url
            .parent()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default();
        let plan = DirectCommandPlan::new(
            synctex,
            vec![
                "view".into(),
                "-i".into(),
                format!("{line}:{column}:{input_path}"),
                "-o".into(),
                binding.pdf_url.to_string_lossy().into_owned(),
            ],
            WorkingDirectoryPolicy::Explicit(pdf_dir),
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
        Self::validate(binding)?;
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
        Self::validate(binding)?;
        let pdf_dir = binding
            .pdf_url
            .parent()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default();
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
            WorkingDirectoryPolicy::Explicit(pdf_dir),
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
        Self::validate(binding)?;
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
    fn validate(binding: &SyncTeXBinding) -> Result<(), SyncTeXSupportError> {
        let fingerprint = Self::synctex_fingerprint(&binding.pdf_url)
            .map_err(|_| SyncTeXSupportError::StaleResult)?;
        let data =
            std::fs::read(&binding.pdf_url).map_err(|_| SyncTeXSupportError::StaleResult)?;
        if fingerprint != binding.revision.fingerprint || sha256_hex(&data) != binding.output_hash
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
                let pdf_dir = binding
                    .pdf_url
                    .parent()
                    .map(Path::to_path_buf)
                    .unwrap_or_default();
                let mapped = if key == "Input" {
                    binding.source_paths.get(&path).cloned()
                } else {
                    None
                };
                let candidate = mapped.unwrap_or_else(|| {
                    if Path::new(&value).is_absolute() {
                        PathBuf::from(&value)
                    } else {
                        pdf_dir.join(&value)
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
    fn synctex_fingerprint(pdf_url: &Path) -> Result<u64, SyncTeXSupportError> {
        let gz = pdf_url.with_extension("synctex.gz");
        let plain = pdf_url.with_extension("synctex");
        let metadata = if gz.exists() { gz } else { plain };
        let data =
            std::fs::read(&metadata).map_err(|_| SyncTeXSupportError::MissingMetadata)?;
        if data.is_empty() {
            return Err(SyncTeXSupportError::MissingMetadata);
        }
        let digest = tex_domain::sha256(&data);
        Ok(digest[..8]
            .iter()
            .fold(0u64, |acc, b| (acc << 8) | u64::from(*b)))
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

fn sha256_hex(data: &[u8]) -> String {
    tex_domain::sha256(data)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
