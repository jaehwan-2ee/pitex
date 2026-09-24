//! Port of `Packages/TexCore/Sources/BuildCore` — command plans, POSIX process
//! runner with process-group cancellation, the LaTeX log parser, and the
//! build orchestrator.
//!
//! The Swift implementation is async/actor based; this port preserves the
//! same semantics with synchronous calls, scoped reader threads, a
//! `CancellationToken` standing in for task cancellation, and mutex-guarded
//! orchestrator state standing in for the actor.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fmt;
#[cfg(windows)]
use std::os::windows::process::CommandExt;
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// `pitex` is a GUI-subsystem app on Windows — a console child spawned
/// without `CREATE_NO_WINDOW` pops a console window per call (a 4 s git
/// poll, every engine launch…). The child still gets a hidden console.
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

// ===========================================================================
// BuildCore.swift
// ===========================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuildCoreError {
    EmptyExecutable,
    InvalidExecutable,
    InvalidArgument { index: usize },
    InvalidEnvironmentKey(String),
    InvalidEnvironmentValue { key: String },
    InvalidWorkingDirectory,
    InvalidTransition,
    CustomShellRequiresAuthority,
}

impl fmt::Display for BuildCoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for BuildCoreError {}

fn validate_command_field(value: &str, empty_allowed: bool) -> bool {
    (empty_allowed || !value.is_empty()) && !value.chars().any(|c| c == '\0')
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WorkingDirectoryPolicy {
    ProjectRoot,
    SourceDirectory,
    Explicit(String),
}

impl WorkingDirectoryPolicy {
    pub fn validated(self) -> Result<Self, BuildCoreError> {
        if let Self::Explicit(path) = &self {
            if path.is_empty() || path.chars().any(|c| c == '\0') {
                return Err(BuildCoreError::InvalidWorkingDirectory);
            }
        }
        Ok(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EnvironmentPolicy {
    Inherit { overrides: HashMap<String, String> },
    Replace(HashMap<String, String>),
}

impl EnvironmentPolicy {
    pub fn validated(self) -> Result<Self, BuildCoreError> {
        let values: &HashMap<String, String> = match &self {
            Self::Inherit { overrides } => overrides,
            Self::Replace(v) => v,
        };
        for (key, value) in values {
            // Windows exports drive working directories as keys such as =C:.
            // Match std::process::Command: only a leading '=' is permitted.
            let name = if cfg!(windows) { key.strip_prefix('=').unwrap_or(key) } else { key };
            if key.is_empty() || name.contains('=') || key.chars().any(|c| c == '\0') {
                return Err(BuildCoreError::InvalidEnvironmentKey(key.clone()));
            }
            if value.chars().any(|c| c == '\0') {
                return Err(BuildCoreError::InvalidEnvironmentValue { key: key.clone() });
            }
        }
        Ok(self)
    }
}

// `Hashable` parity with Swift — std HashMap is not Hash, so hash sorted pairs.
impl std::hash::Hash for EnvironmentPolicy {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            Self::Inherit { overrides } => {
                state.write_u8(0);
                let mut pairs: Vec<_> = overrides.iter().collect();
                pairs.sort();
                for (k, v) in pairs {
                    k.hash(state);
                    v.hash(state);
                }
            }
            Self::Replace(values) => {
                state.write_u8(1);
                let mut pairs: Vec<_> = values.iter().collect();
                pairs.sort();
                for (k, v) in pairs {
                    k.hash(state);
                    v.hash(state);
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DirectCommandPlan {
    pub executable: String,
    pub arguments: Vec<String>,
    #[serde(rename = "workingDirectory")]
    pub working_directory: WorkingDirectoryPolicy,
    pub environment: EnvironmentPolicy,
}

impl DirectCommandPlan {
    pub fn new(
        executable: impl Into<String>,
        arguments: Vec<String>,
        working_directory: WorkingDirectoryPolicy,
        environment: EnvironmentPolicy,
    ) -> Result<Self, BuildCoreError> {
        let executable = executable.into();
        if executable.is_empty() {
            return Err(BuildCoreError::EmptyExecutable);
        }
        if !validate_command_field(&executable, false) {
            return Err(BuildCoreError::InvalidExecutable);
        }
        for (index, argument) in arguments.iter().enumerate() {
            if !validate_command_field(argument, true) {
                return Err(BuildCoreError::InvalidArgument { index });
            }
        }
        Ok(Self {
            executable,
            arguments,
            working_directory: working_directory.validated()?,
            environment: environment.validated()?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ShellAuthority {
    pub source: ShellAuthoritySource,
    #[serde(rename = "approvedByUser")]
    pub approved_by_user: bool,
    pub disclosure: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ShellAuthoritySource {
    BuiltIn,
    ProjectManifest,
    UserConfiguration,
}

impl ShellAuthority {
    pub fn new(
        source: ShellAuthoritySource,
        approved_by_user: bool,
        disclosure: impl Into<String>,
    ) -> Result<Self, BuildCoreError> {
        let disclosure = disclosure.into();
        if !(source == ShellAuthoritySource::BuiltIn || approved_by_user) || disclosure.is_empty() {
            return Err(BuildCoreError::CustomShellRequiresAuthority);
        }
        Ok(Self {
            source,
            approved_by_user,
            disclosure,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LoginShellCommandPlan {
    #[serde(rename = "shellExecutable")]
    pub shell_executable: String,
    pub command: String,
    #[serde(rename = "workingDirectory")]
    pub working_directory: WorkingDirectoryPolicy,
    pub environment: EnvironmentPolicy,
    pub authority: ShellAuthority,
}

impl LoginShellCommandPlan {
    pub fn new(
        shell_executable: impl Into<String>,
        command: impl Into<String>,
        working_directory: WorkingDirectoryPolicy,
        environment: EnvironmentPolicy,
        authority: ShellAuthority,
    ) -> Result<Self, BuildCoreError> {
        let shell_executable = shell_executable.into();
        let command = command.into();
        if !validate_command_field(&shell_executable, false) {
            return Err(BuildCoreError::InvalidExecutable);
        }
        if !validate_command_field(&command, false) {
            return Err(BuildCoreError::InvalidArgument { index: 0 });
        }
        Ok(Self {
            shell_executable,
            command,
            working_directory: working_directory.validated()?,
            environment: environment.validated()?,
            authority,
        })
    }

    /// `-l -c` for POSIX shells. On Windows the flag depends on the
    /// configured shell: `cmd /c` for cmd.exe, `-Command` for PowerShell —
    /// resolved from the shell executable's file name.
    #[cfg(unix)]
    pub fn invocation_arguments(&self) -> Vec<String> {
        vec!["-l".into(), "-c".into(), self.command.clone()]
    }
    #[cfg(windows)]
    pub fn invocation_arguments(&self) -> Vec<String> {
        let name = std::path::Path::new(&self.shell_executable)
            .file_name()
            .map(|n| n.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        if name.starts_with("powershell") || name.starts_with("pwsh") {
            vec!["-NoProfile".into(), "-Command".into(), self.command.clone()]
        } else {
            vec!["/c".into(), self.command.clone()]
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BuildToolTemplate {
    Latexmk,
    PdfLaTeX,
    XeLaTeX,
    LuaLaTeX,
    BibTeX,
    Biber,
}

impl BuildToolTemplate {
    pub fn plan(&self, input: &str) -> Result<DirectCommandPlan, BuildCoreError> {
        if input.is_empty() || input.starts_with('-') || !validate_command_field(input, false) {
            return Err(BuildCoreError::InvalidArgument { index: 0 });
        }
        let input = input.to_string();
        match self {
            Self::Latexmk => DirectCommandPlan::new(
                "latexmk",
                vec![
                    "-pdf".into(),
                    "-synctex=1".into(),
                    "-interaction=nonstopmode".into(),
                    "-file-line-error".into(),
                    input,
                ],
                WorkingDirectoryPolicy::ProjectRoot,
                EnvironmentPolicy::Inherit {
                    overrides: HashMap::new(),
                },
            ),
            Self::PdfLaTeX => DirectCommandPlan::new(
                "pdflatex",
                vec![
                    "-synctex=1".into(),
                    "-interaction=nonstopmode".into(),
                    "-file-line-error".into(),
                    input,
                ],
                WorkingDirectoryPolicy::ProjectRoot,
                EnvironmentPolicy::Inherit {
                    overrides: HashMap::new(),
                },
            ),
            Self::XeLaTeX => DirectCommandPlan::new(
                "xelatex",
                vec![
                    "-synctex=1".into(),
                    "-interaction=nonstopmode".into(),
                    "-file-line-error".into(),
                    input,
                ],
                WorkingDirectoryPolicy::ProjectRoot,
                EnvironmentPolicy::Inherit {
                    overrides: HashMap::new(),
                },
            ),
            Self::LuaLaTeX => DirectCommandPlan::new(
                "lualatex",
                vec![
                    "-synctex=1".into(),
                    "--interaction=nonstopmode".into(),
                    "--file-line-error".into(),
                    input,
                ],
                WorkingDirectoryPolicy::ProjectRoot,
                EnvironmentPolicy::Inherit {
                    overrides: HashMap::new(),
                },
            ),
            Self::BibTeX => DirectCommandPlan::new(
                "bibtex",
                vec![input],
                WorkingDirectoryPolicy::ProjectRoot,
                EnvironmentPolicy::Inherit {
                    overrides: HashMap::new(),
                },
            ),
            Self::Biber => DirectCommandPlan::new(
                "biber",
                vec![input],
                WorkingDirectoryPolicy::ProjectRoot,
                EnvironmentPolicy::Inherit {
                    overrides: HashMap::new(),
                },
            ),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BuildID {
    pub raw_value: String,
}

impl BuildID {
    pub fn new(raw_value: impl Into<String>) -> Result<Self, BuildCoreError> {
        let raw_value = raw_value.into();
        if raw_value.is_empty()
            || raw_value.len() > 128
            || !raw_value.bytes().all(|b| {
                b == b'-' || b == b'_' || b.is_ascii_digit() || b.is_ascii_uppercase() || b.is_ascii_lowercase()
            })
        {
            return Err(BuildCoreError::InvalidExecutable);
        }
        Ok(Self { raw_value })
    }
}

impl fmt::Display for BuildID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.raw_value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BuildLifecycle {
    Queued,
    Running {
        #[serde(rename = "startedAtMilliseconds")]
        started_at_milliseconds: u64,
    },
    Cancelling,
    Succeeded {
        #[serde(rename = "exitCode")]
        exit_code: i32,
        #[serde(rename = "finishedAtMilliseconds")]
        finished_at_milliseconds: u64,
    },
    Failed {
        #[serde(rename = "exitCode")]
        exit_code: Option<i32>,
        #[serde(rename = "finishedAtMilliseconds")]
        finished_at_milliseconds: u64,
    },
    Cancelled {
        #[serde(rename = "finishedAtMilliseconds")]
        finished_at_milliseconds: u64,
    },
}

impl BuildLifecycle {
    pub fn transitioning(&self, next: Self) -> Result<Self, BuildCoreError> {
        use BuildLifecycle::*;
        let allowed = matches!(
            (self, next),
            (Queued, Running { .. })
                | (Queued, Cancelled { .. })
                | (Running { .. }, Cancelling)
                | (Running { .. }, Succeeded { .. })
                | (Running { .. }, Failed { .. })
                | (Cancelling, Cancelled { .. })
                | (Cancelling, Failed { .. })
        );
        if !allowed {
            return Err(BuildCoreError::InvalidTransition);
        }
        Ok(next)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BuildLogChannel {
    StandardOutput,
    StandardError,
    System,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BuildLogEntry {
    pub sequence: u64,
    pub channel: BuildLogChannel,
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BuildIssueSeverity {
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BuildIssue {
    pub severity: BuildIssueSeverity,
    pub message: String,
    pub file: Option<String>,
    pub line: Option<i64>,
}

impl BuildIssue {
    pub fn new(
        severity: BuildIssueSeverity,
        message: impl Into<String>,
        file: Option<String>,
        line: Option<i64>,
    ) -> Result<Self, BuildCoreError> {
        let message = message.into();
        if message.is_empty() || line.map(|l| l <= 0).unwrap_or(false) {
            return Err(BuildCoreError::InvalidArgument { index: 0 });
        }
        Ok(Self {
            severity,
            message,
            file,
            line,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BuildCancellation {
    #[serde(rename = "requestedAtMilliseconds")]
    pub requested_at_milliseconds: u64,
    #[serde(rename = "gracePeriodMilliseconds")]
    pub grace_period_milliseconds: u64,
}

impl BuildCancellation {
    pub fn new(
        requested_at_milliseconds: u64,
        grace_period_milliseconds: u64,
    ) -> Result<Self, BuildCoreError> {
        if requested_at_milliseconds.checked_add(grace_period_milliseconds).is_none() {
            return Err(BuildCoreError::InvalidArgument { index: 0 });
        }
        Ok(Self {
            requested_at_milliseconds,
            grace_period_milliseconds,
        })
    }

    pub fn force_termination_at_milliseconds(&self) -> u64 {
        self.requested_at_milliseconds + self.grace_period_milliseconds
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CleanupPolicy {
    PreserveAll,
    RemoveKnownAuxiliaryFiles(HashSet<String>),
    RemoveBuildDirectory,
}

// ===========================================================================
// BuildLogParser.swift
// ===========================================================================

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BuildIssueRecord {
    pub sequence: u64,
    pub severity: BuildIssueSeverity,
    pub message: String,
    pub file: Option<String>,
    pub line: Option<i64>,
    pub column: Option<i64>,
}

impl BuildIssueRecord {
    pub fn is_clickable(&self) -> bool {
        self.file.is_some() && self.line.is_some()
    }

    pub fn issue(&self) -> Result<BuildIssue, BuildCoreError> {
        BuildIssue::new(
            self.severity,
            self.message.clone(),
            self.file.clone(),
            self.line,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PendingIssue {
    severity: BuildIssueSeverity,
    message: String,
    file: Option<String>,
    line: Option<i64>,
    column: Option<i64>,
}

pub struct BuildLogParser {
    project_root: PathBuf,
    buffers: HashMap<BuildLogChannel, Vec<u8>>,
    file_stack: Vec<String>,
    parenthesis_stack: Vec<bool>,
    pending_classic_error: Option<PendingIssue>,
    next_sequence: u64,
}

impl BuildLogParser {
    pub fn new(project_root: &Path) -> Self {
        Self {
            project_root: standardized_path(project_root),
            buffers: HashMap::new(),
            file_stack: Vec::new(),
            parenthesis_stack: Vec::new(),
            pending_classic_error: None,
            next_sequence: 0,
        }
    }

    pub fn consume_bytes(
        &mut self,
        bytes: &[u8],
        channel: BuildLogChannel,
    ) -> Vec<BuildIssueRecord> {
        let mut buffer = self.buffers.remove(&channel).unwrap_or_default();
        buffer.extend_from_slice(bytes);
        let mut records: Vec<BuildIssueRecord> = Vec::new();

        while let Some(newline) = buffer.iter().position(|b| *b == 0x0A) {
            let mut line_data = &buffer[..newline];
            if line_data.last() == Some(&0x0D) {
                line_data = &line_data[..line_data.len() - 1];
            }
            let line_text = String::from_utf8_lossy(line_data).into_owned();
            records.extend(self.parse_line(&line_text));
            buffer.drain(..=newline);
        }
        self.buffers.insert(channel, buffer);
        records
    }

    pub fn consume(&mut self, text: &str, channel: BuildLogChannel) -> Vec<BuildIssueRecord> {
        self.consume_bytes(text.as_bytes(), channel)
    }

    pub fn finish(&mut self) -> Vec<BuildIssueRecord> {
        let mut records: Vec<BuildIssueRecord> = Vec::new();
        for channel in [
            BuildLogChannel::StandardOutput,
            BuildLogChannel::StandardError,
            BuildLogChannel::System,
        ] {
            if let Some(data) = self.buffers.get(&channel) {
                if !data.is_empty() {
                    let text = String::from_utf8_lossy(&data).into_owned();
                    records.extend(self.parse_line(&text));
                }
            }
            self.buffers.remove(&channel);
        }
        if let Some(pending) = self.pending_classic_error.take() {
            records.push(self.make_record(pending));
        }
        records.sort_by_key(|r| r.sequence);
        records
    }

    fn parse_line(&mut self, raw_line: &str) -> Vec<BuildIssueRecord> {
        let line = escape_controls(raw_line);
        let location_file = self.file_stack.last().cloned();
        self.update_file_stack(&line);

        if let Some(pending) = self.pending_classic_error.clone() {
            if let Some(source_line) = classic_line_number(&line) {
                self.pending_classic_error = None;
                return vec![self.make_record(PendingIssue {
                    severity: pending.severity,
                    message: pending.message,
                    file: pending.file,
                    line: Some(source_line),
                    column: None,
                })];
            }
            if self.is_diagnostic_start(&line) {
                self.pending_classic_error = None;
                let mut result = vec![self.make_record(pending)];
                result.extend(self.parse_fresh_line(&line, location_file));
                return result;
            }
            return vec![];
        }
        self.parse_fresh_line(&line, location_file)
    }

    fn parse_fresh_line(
        &mut self,
        line: &str,
        location_file: Option<String>,
    ) -> Vec<BuildIssueRecord> {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return vec![];
        }

        if let Some(parsed) = self.parse_file_line_diagnostic(trimmed) {
            return vec![self.make_record(parsed)];
        }
        if let Some(rest) = trimmed.strip_prefix('!') {
            let message = rest.trim();
            if message.is_empty() {
                return vec![];
            }
            self.pending_classic_error = Some(PendingIssue {
                severity: BuildIssueSeverity::Error,
                message: message.to_string(),
                file: self.normalized_display_path(location_file.as_deref()),
                line: None,
                column: None,
            });
            return vec![];
        }
        if let Some(diagnostic) = self.parse_tool_diagnostic(trimmed, location_file) {
            return vec![self.make_record(diagnostic)];
        }
        vec![]
    }

    fn parse_file_line_diagnostic(&self, line: &str) -> Option<PendingIssue> {
        let chars: Vec<char> = line.chars().collect();
        let mut search = 0usize;
        loop {
            let colon = chars[search..].iter().position(|c| *c == ':').map(|p| p + search)?;
            let after_colon = colon + 1;
            let mut digits_end = after_colon;
            while digits_end < chars.len() && chars[digits_end].is_numeric() {
                digits_end += 1;
            }
            if !(digits_end > after_colon && digits_end < chars.len() && chars[digits_end] == ':') {
                search = after_colon;
                continue;
            }
            let file: String = chars[..colon].iter().collect();
            let digits: String = chars[after_colon..digits_end].iter().collect();
            let line_number: i64 = match digits.parse() {
                Ok(v) => v,
                Err(_) => {
                    search = after_colon;
                    continue;
                }
            };
            if file.is_empty() || line_number <= 0 {
                search = after_colon;
                continue;
            }

            let mut message_start = digits_end + 1;
            let mut column: Option<i64> = None;
            let mut column_end = message_start;
            while column_end < chars.len() && chars[column_end].is_numeric() {
                column_end += 1;
            }
            if column_end > message_start && column_end < chars.len() && chars[column_end] == ':' {
                column = chars[message_start..column_end]
                    .iter()
                    .collect::<String>()
                    .parse()
                    .ok();
                message_start = column_end + 1;
            }
            let message: String = chars[message_start..].iter().collect::<String>().trim().to_string();
            if message.is_empty() {
                return None;
            }
            return Some(PendingIssue {
                severity: severity_for(&message),
                message,
                file: self.normalized_display_path(Some(&file)),
                line: Some(line_number),
                column: column.filter(|c| *c > 0),
            });
        }
    }

    fn parse_tool_diagnostic(
        &self,
        line: &str,
        location_file: Option<String>,
    ) -> Option<PendingIssue> {
        let lower = line.to_lowercase();
        let severity = if lower.starts_with("warning--")
            || lower.contains(" warning:")
            || lower.starts_with("warning:")
            || lower.contains("warning (file")
            || lower.contains("overfull \\hbox")
            || lower.contains("underfull \\hbox")
        {
            BuildIssueSeverity::Warning
        } else if lower.starts_with("error:")
            || lower.starts_with("error--")
            || lower.starts_with("!!")
            || lower.contains("error message")
            || lower.contains("not found")
            || lower.contains("couldn't open")
            || lower.contains("cannot open")
        {
            BuildIssueSeverity::Error
        } else {
            return None;
        };

        let box_line = first_positive_integer("at lines ", &lower)
            .or_else(|| first_positive_integer("at line ", &lower));
        Some(PendingIssue {
            severity,
            message: line.to_string(),
            file: self.normalized_display_path(location_file.as_deref()),
            line: box_line,
            column: None,
        })
    }

    fn is_diagnostic_start(&self, line: &str) -> bool {
        let trimmed = line.trim();
        trimmed.starts_with('!')
            || self.parse_file_line_diagnostic(trimmed).is_some()
            || self.parse_tool_diagnostic(trimmed, None).is_some()
    }

    fn make_record(&mut self, pending: PendingIssue) -> BuildIssueRecord {
        let sequence = self.next_sequence;
        self.next_sequence += 1;
        BuildIssueRecord {
            sequence,
            severity: pending.severity,
            message: pending.message,
            file: pending.file,
            line: pending.line,
            column: pending.column,
        }
    }

    fn update_file_stack(&mut self, line: &str) {
        let chars: Vec<char> = line.chars().collect();
        let mut index = 0usize;
        while index < chars.len() {
            let character = chars[index];
            if character == '(' {
                let start = index + 1;
                let mut cursor = start;
                let mut file_end: Option<usize> = None;
                while cursor < chars.len() && chars[cursor] != ')' && chars[cursor] != '(' {
                    cursor += 1;
                    if is_tex_file_token(&chars[start..cursor].iter().collect::<String>()) {
                        file_end = Some(cursor);
                        break;
                    }
                }
                let token: Option<String> =
                    file_end.map(|end| chars[start..end].iter().collect());
                let is_file = token.is_some();
                self.parenthesis_stack.push(is_file);
                if let Some(token) = token {
                    self.file_stack.push(
                        self.normalized_display_path(Some(&token)).unwrap_or(token),
                    );
                }
                index = file_end.unwrap_or(start);
                continue;
            }
            if character == ')' {
                if let Some(opened_file) = self.parenthesis_stack.pop() {
                    if opened_file {
                        self.file_stack.pop();
                    }
                }
            }
            index += 1;
        }
    }

    fn normalized_display_path(&self, path: Option<&str>) -> Option<String> {
        let path = path?;
        if path.is_empty() {
            return None;
        }
        let candidate = if path.starts_with('/') {
            standardized_path(Path::new(path))
        } else {
            standardized_path(&self.project_root.join(path))
        };
        let root_string = self.project_root.to_string_lossy();
        let root_path = if root_string == "/" {
            "/".to_string()
        } else {
            format!("{root_string}/")
        };
        let candidate_string = candidate.to_string_lossy();
        if candidate_string != root_string && !candidate_string.starts_with(&root_path) {
            return Some(path.to_string());
        }
        if candidate_string == root_string {
            return Some(".".to_string());
        }
        Some(candidate_string[root_path.len()..].to_string())
    }
}

fn severity_for(message: &str) -> BuildIssueSeverity {
    let lower = message.to_lowercase();
    if lower.contains("warning") || lower.contains("overfull") || lower.contains("underfull") {
        BuildIssueSeverity::Warning
    } else {
        BuildIssueSeverity::Error
    }
}

fn first_positive_integer(marker: &str, text: &str) -> Option<i64> {
    let position = text.find(marker)?;
    let suffix = &text[position + marker.len()..];
    let digits: String = suffix.chars().take_while(|c| c.is_numeric()).collect();
    let value: i64 = digits.parse().ok()?;
    if value > 0 {
        Some(value)
    } else {
        None
    }
}

fn classic_line_number(line: &str) -> Option<i64> {
    let trimmed = line.trim();
    let rest = trimmed.strip_prefix("l.")?;
    let digits: String = rest.chars().take_while(|c| c.is_numeric()).collect();
    let value: i64 = digits.parse().ok()?;
    if value > 0 {
        Some(value)
    } else {
        None
    }
}

fn is_tex_file_token(token: &str) -> bool {
    let lower = token.to_lowercase();
    lower.ends_with(".tex")
        || lower.ends_with(".sty")
        || lower.ends_with(".cls")
        || lower.ends_with(".bib")
        || lower.ends_with(".idx")
}

fn escape_controls(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for scalar in text.chars() {
        if (scalar < '\u{20}' && scalar != '\t') || scalar == '\u{7F}' {
            escaped.push_str(&format!("\\u{{{:X}}}", scalar as u32));
        } else {
            escaped.push(scalar);
        }
    }
    escaped
}

// ---------------------------------------------------------------------------
// Path helpers mirroring Foundation URL semantics.

/// `URL.standardizedFileURL` — lexical `.`/`..`/duplicate-separator cleanup
/// without touching the filesystem.
pub(crate) fn standardized_path(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// `URL.resolvingSymlinksInPath` — canonicalize the longest existing prefix,
/// then append the unresolved tail.
pub(crate) fn resolving_symlinks_in_path(path: &Path) -> PathBuf {
    for ancestor in path.ancestors() {
        if ancestor.exists() {
            if let Ok(canon) = ancestor.canonicalize() {
                if let Ok(rest) = path.strip_prefix(ancestor) {
                    if rest.as_os_str().is_empty() {
                        return canon;
                    }
                    return canon.join(rest);
                }
                return canon;
            }
        }
    }
    path.to_path_buf()
}

/// `NSString.deletingPathExtension` on the last path component.
pub(crate) fn deleting_path_extension(name: &str) -> String {
    let base = name.rsplit('/').next().unwrap_or(name);
    // NSString treats leading-dot basenames as extensionless.
    match base.rfind('.') {
        Some(dot) if dot > 0 => {
            let stem = &base[..dot];
            let prefix = &name[..name.len() - base.len()];
            format!("{prefix}{stem}")
        }
        _ => name.to_string(),
    }
}

pub(crate) fn remove_item(path: &Path) -> std::io::Result<()> {
    if path.is_dir() && !path.is_symlink() {
        std::fs::remove_dir_all(path)
    } else {
        std::fs::remove_file(path)
    }
}

pub(crate) fn atomic_write(path: &Path, data: &[u8]) -> std::io::Result<()> {
    let parent = path.parent().map(Path::to_path_buf).unwrap_or_else(|| PathBuf::from("."));
    let file_name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let temp = parent.join(format!(".{file_name}.atomic-{}.tmp", std::process::id()));
    std::fs::write(&temp, data)?;
    std::fs::rename(&temp, path)
}

pub(crate) fn now_milliseconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| (d.as_secs_f64() * 1_000.0) as u64)
        .unwrap_or(0)
}

// ===========================================================================
// ProcessRunner.swift
// ===========================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProcessRunnerError {
    InvalidWorkingDirectory(String),
    SpawnFailed { code: i32 },
    PipeFailed { code: i32 },
    WaitFailed { code: i32 },
}

impl fmt::Display for ProcessRunnerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidWorkingDirectory(d) => write!(f, "invalid working directory: {d}"),
            Self::SpawnFailed { code } => write!(f, "spawn failed: {code}"),
            Self::PipeFailed { code } => write!(f, "pipe failed: {code}"),
            Self::WaitFailed { code } => write!(f, "wait failed: {code}"),
        }
    }
}
impl std::error::Error for ProcessRunnerError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ProcessOutputChannel {
    StandardOutput,
    StandardError,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessOutputChunk {
    pub sequence: u64,
    pub channel: ProcessOutputChannel,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessTermination {
    Exited { code: i32 },
    Signaled { signal: i32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessStopReason {
    Completed,
    TimedOut,
    Cancelled,
}

#[derive(Debug)]
pub struct ProcessResult {
    pub process_identifier: i32,
    pub termination: ProcessTermination,
    pub stop_reason: ProcessStopReason,
    pub standard_output: Vec<u8>,
    pub standard_error: Vec<u8>,
    pub shell_authority: Option<ShellAuthority>,
}

/// Caller-side cancellation, equivalent to cancelling the Swift `Task` that
/// wraps `run`. Cloneable and safe to trigger from any thread.
#[derive(Clone)]
pub struct CancellationToken {
    inner: Arc<(Mutex<bool>, Condvar)>,
}

impl Default for CancellationToken {
    fn default() -> Self {
        Self::new()
    }
}

impl CancellationToken {
    pub fn new() -> Self {
        Self {
            inner: Arc::new((Mutex::new(false), Condvar::new())),
        }
    }

    pub fn cancel(&self) {
        let (lock, condvar) = &*self.inner;
        *lock.lock().unwrap() = true;
        condvar.notify_all();
    }

    pub fn is_cancelled(&self) -> bool {
        *self.inner.0.lock().unwrap()
    }

    /// Blocks until cancelled or the timeout elapses. Returns true when the
    /// token was cancelled.
    fn wait_cancelled(&self, timeout: Duration) -> bool {
        let (lock, condvar) = &*self.inner;
        let guard = lock.lock().unwrap();
        if *guard {
            return true;
        }
        let (guard, _) = condvar.wait_timeout(guard, timeout).unwrap();
        *guard
    }
}

struct ProcessControl {
    pid: i32,
    grace_period: Duration,
    state: Mutex<ControlState>,
}

struct ControlState {
    exited: bool,
    stop_reason: ProcessStopReason,
}

impl ProcessControl {
    fn new(pid: i32, grace_period: Duration) -> Self {
        Self {
            pid,
            grace_period,
            state: Mutex::new(ControlState {
                exited: false,
                stop_reason: ProcessStopReason::Completed,
            }),
        }
    }

    fn request_stop(self: &Arc<Self>, reason: ProcessStopReason) {
        {
            let mut state = self.state.lock().unwrap();
            if state.exited || state.stop_reason != ProcessStopReason::Completed {
                return;
            }
            state.stop_reason = reason;
        }
        terminate_group(self.pid);
        let this = Arc::clone(self);
        std::thread::spawn(move || {
            std::thread::sleep(this.grace_period);
            this.force_stop_if_running();
        });
    }

    fn process_did_exit(self: &Arc<Self>) -> ProcessStopReason {
        let reason;
        {
            let mut state = self.state.lock().unwrap();
            state.exited = true;
            reason = state.stop_reason;
        }
        // Reap stragglers in the group even on normal completion, mirroring
        // the Swift escalation: SIGTERM now, SIGKILL after the grace period
        // if the group still exists.
        terminate_group(self.pid);
        let pid = self.pid;
        let grace = self.grace_period;
        std::thread::spawn(move || {
            std::thread::sleep(grace);
            if process_group_exists(pid) {
                kill_group(pid);
            }
        });
        reason
    }

    fn force_stop_if_running(&self) {
        let exited = self.state.lock().unwrap().exited;
        if !exited {
            kill_group(self.pid);
        }
    }

    fn is_exited(&self) -> bool {
        self.state.lock().unwrap().exited
    }
}

#[cfg(unix)]
fn process_group_exists(pid: i32) -> bool {
    unsafe { libc::kill(-pid, 0) == 0 || std::io::Error::last_os_error().raw_os_error() != Some(libc::ESRCH) }
}

/// Windows has no process-group signals — `taskkill /T` walks the process
/// tree instead. `tasklist` reports the leader while it still runs.
#[cfg(windows)]
fn process_group_exists(pid: i32) -> bool {
    std::process::Command::new("tasklist")
        .args(["/FI", &format!("PID eq {pid}"), "/FO", "CSV", "/NH"])
        .creation_flags(CREATE_NO_WINDOW)
        .stdin(std::process::Stdio::null())
        .output()
        .map(|out| {
            String::from_utf8_lossy(&out.stdout).contains(&format!(",\"{pid}\","))
        })
        .unwrap_or(false)
}

/// SIGTERM to the process group — graceful shutdown request.
#[cfg(unix)]
fn terminate_group(pid: i32) {
    unsafe {
        libc::kill(-pid, libc::SIGTERM);
    }
}

/// SIGKILL to the process group — forced teardown after the grace period.
#[cfg(unix)]
fn kill_group(pid: i32) {
    unsafe {
        libc::kill(-pid, libc::SIGKILL);
    }
}

/// Windows cannot signal a foreign console process gracefully (`taskkill`
/// without `/F` only reaches GUI message loops), so the TERM stage is a
/// best-effort request and the forced stage after the grace period does the
/// real work — same two-phase shape as the Unix escalation.
#[cfg(windows)]
fn terminate_group(pid: i32) {
    let _ = std::process::Command::new("taskkill")
        .args(["/PID", &pid.to_string(), "/T"])
        .creation_flags(CREATE_NO_WINDOW)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();
}

#[cfg(windows)]
fn kill_group(pid: i32) {
    let _ = std::process::Command::new("taskkill")
        .args(["/PID", &pid.to_string(), "/T", "/F"])
        .creation_flags(CREATE_NO_WINDOW)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();
}

#[cfg(unix)]
fn wait_for_process(pid: i32) -> Result<i32, ProcessRunnerError> {
    let mut status: i32 = 0;
    loop {
        let result = unsafe { libc::waitpid(pid, &mut status, 0) };
        if result == pid {
            return Ok(status);
        }
        if result == -1 {
            let err = std::io::Error::last_os_error().raw_os_error().unwrap_or(0);
            if err == libc::EINTR {
                continue;
            }
            return Err(ProcessRunnerError::WaitFailed { code: err });
        }
    }
}

/// Windows counterpart — the `Child` handle waits directly; the exit code
/// carries the same information waitpid packs into `status`. Takes the
/// child field rather than `SpawnedProcess`, which is partially moved once
/// the reader threads own the output pipes.
#[cfg(windows)]
fn wait_for_process(child: &mut std::process::Child) -> Result<i32, ProcessRunnerError> {
    child
        .wait()
        .map(|s| s.code().unwrap_or(1))
        .map_err(|e| ProcessRunnerError::WaitFailed {
            code: e.raw_os_error().unwrap_or(1),
        })
}

#[cfg(unix)]
fn decode_wait_status(status: i32) -> ProcessTermination {
    let signal = status & 0x7f;
    if signal == 0 {
        ProcessTermination::Exited {
            code: (status >> 8) & 0xff,
        }
    } else {
        ProcessTermination::Signaled { signal }
    }
}

/// Windows reports exit codes directly — a `taskkill`-terminated process
/// surfaces as exit code 1, matching how the Unix `Signaled` case is
/// consumed downstream (non-zero ⇒ failed build).
#[cfg(windows)]
fn decode_wait_status(status: i32) -> ProcessTermination {
    ProcessTermination::Exited { code: status }
}

#[cfg(unix)]
struct SpawnedProcess {
    pid: i32,
    standard_output: i32,
    standard_error: i32,
}

#[cfg(windows)]
struct SpawnedProcess {
    pid: i32,
    /// Kept for `wait()` — the reader threads own the piped streams.
    child: std::process::Child,
    standard_output: std::process::ChildStdout,
    standard_error: std::process::ChildStderr,
}

fn resolve_working_directory(
    policy: &WorkingDirectoryPolicy,
    project_root: &Path,
    source_directory: Option<&Path>,
) -> Result<PathBuf, ProcessRunnerError> {
    let url = match policy {
        WorkingDirectoryPolicy::ProjectRoot => project_root.to_path_buf(),
        WorkingDirectoryPolicy::SourceDirectory => match source_directory {
            Some(directory) => directory.to_path_buf(),
            None => {
                return Err(ProcessRunnerError::InvalidWorkingDirectory(
                    "source directory was not supplied".into(),
                ))
            }
        },
        WorkingDirectoryPolicy::Explicit(path) => {
            if Path::new(path).is_absolute() {
                PathBuf::from(path)
            } else {
                project_root.join(path)
            }
        }
    };

    let normalized = standardized_path(&url);
    if !normalized.is_dir() {
        return Err(ProcessRunnerError::InvalidWorkingDirectory(
            normalized.to_string_lossy().into_owned(),
        ));
    }
    Ok(normalized)
}

fn resolved_environment(policy: &EnvironmentPolicy) -> HashMap<String, String> {
    match policy {
        EnvironmentPolicy::Inherit { overrides } => {
            let mut env: HashMap<String, String> = std::env::vars().collect();
            for (key, value) in overrides {
                #[cfg(windows)]
                env.retain(|existing, _| !existing.eq_ignore_ascii_case(key));
                env.insert(key.clone(), value.clone());
            }
            env
        }
        EnvironmentPolicy::Replace(values) => values.clone(),
    }
}

/// posix_spawnp searches the parent's PATH, ignoring PATH in its envp. Resolve
/// against the command's environment so GUI launches and overrides agree.
#[cfg(unix)]
fn resolved_executable(
    executable: &str,
    environment: &HashMap<String, String>,
    directory: &Path,
) -> Result<String, ProcessRunnerError> {
    if executable.contains('/') {
        return Ok(executable.to_string());
    }
    let default_path = "/usr/bin:/bin".to_string();
    let path = environment.get("PATH").unwrap_or(&default_path);
    let mut error_code = libc::ENOENT;
    for entry in path.split(':') {
        let folder = if entry.is_empty() {
            directory.to_path_buf()
        } else {
            let entry_path = Path::new(entry);
            if entry_path.is_absolute() {
                entry_path.to_path_buf()
            } else {
                directory.join(entry_path)
            }
        };
        let candidate = folder.join(executable);
        if let Ok(meta) = std::fs::metadata(&candidate) {
            if !meta.is_dir() && is_executable_file(&candidate) {
                return Ok(candidate.to_string_lossy().into_owned());
            }
            error_code = libc::EACCES;
        }
    }
    Err(ProcessRunnerError::SpawnFailed { code: error_code })
}

/// Windows counterpart — `std::env::split_paths` handles the `;` separator
/// and a bare name expands through `PATHEXT` (`latexmk` → `latexmk.exe`,
/// `npm` → `npm.cmd`).
#[cfg(windows)]
fn resolved_executable(
    executable: &str,
    environment: &HashMap<String, String>,
    directory: &Path,
) -> Result<String, ProcessRunnerError> {
    if Path::new(executable).is_absolute() {
        return Ok(executable.to_string());
    }
    let names = pathext_candidates(executable, environment);
    let path = environment.get("PATH").cloned().unwrap_or_default();
    for entry in std::env::split_paths(&path) {
        let folder = if entry.as_os_str().is_empty() {
            directory.to_path_buf()
        } else if entry.is_absolute() {
            entry
        } else {
            directory.join(entry)
        };
        for name in &names {
            let candidate = folder.join(name);
            if candidate.is_file() {
                return Ok(candidate.to_string_lossy().into_owned());
            }
        }
    }
    // ERROR_FILE_NOT_FOUND — matches the Unix ENOENT reporting slot.
    Err(ProcessRunnerError::SpawnFailed { code: 2 })
}

/// `name` → `[name.exe, name.cmd, name.bat, name.com, name.ps1, name]` in
/// PATHEXT order (extension-bearing names pass through unchanged).
#[cfg(windows)]
fn pathext_candidates(executable: &str, environment: &HashMap<String, String>) -> Vec<String> {
    if Path::new(executable).extension().is_some() {
        return vec![executable.to_string()];
    }
    let pathext = environment
        .get("PATHEXT")
        .map(|s| s.as_str())
        .unwrap_or(".COM;.EXE;.BAT;.CMD");
    pathext
        .split(';')
        .filter(|e| !e.is_empty())
        .map(|e| format!("{executable}{e}"))
        .chain(std::iter::once(executable.to_string()))
        .collect()
}

#[cfg(unix)]
fn is_executable_file(path: &Path) -> bool {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;
    match CString::new(path.as_os_str().as_bytes()) {
        Ok(c) => unsafe { libc::access(c.as_ptr(), libc::X_OK) == 0 },
        Err(_) => false,
    }
}

#[cfg(unix)]
fn spawn(
    executable: &str,
    arguments: &[String],
    directory: &Path,
    environment: &HashMap<String, String>,
) -> Result<SpawnedProcess, ProcessRunnerError> {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;

    let mut output_pipe = [-1i32; 2];
    let mut error_pipe = [-1i32; 2];
    if unsafe { libc::pipe(output_pipe.as_mut_ptr()) } != 0 {
        return Err(ProcessRunnerError::PipeFailed {
            code: std::io::Error::last_os_error().raw_os_error().unwrap_or(0),
        });
    }
    if unsafe { libc::pipe(error_pipe.as_mut_ptr()) } != 0 {
        let code = std::io::Error::last_os_error().raw_os_error().unwrap_or(0);
        unsafe {
            libc::close(output_pipe[0]);
            libc::close(output_pipe[1]);
        }
        return Err(ProcessRunnerError::PipeFailed { code });
    }
    for fd in output_pipe.iter().chain(error_pipe.iter()) {
        unsafe {
            libc::fcntl(*fd, libc::F_SETFD, libc::FD_CLOEXEC);
        }
    }

    let mut actions: libc::posix_spawn_file_actions_t = unsafe { std::mem::zeroed() };
    let mut attributes: libc::posix_spawnattr_t = unsafe { std::mem::zeroed() };
    unsafe {
        libc::posix_spawn_file_actions_init(&mut actions);
        libc::posix_spawnattr_init(&mut attributes);
    }
    struct AttrGuard(*mut libc::posix_spawn_file_actions_t, *mut libc::posix_spawnattr_t);
    impl Drop for AttrGuard {
        fn drop(&mut self) {
            unsafe {
                libc::posix_spawn_file_actions_destroy(self.0);
                libc::posix_spawnattr_destroy(self.1);
            }
        }
    }
    let _guard = AttrGuard(&mut actions, &mut attributes);

    let directory_c = CString::new(directory.as_os_str().as_bytes()).map_err(|_| {
        ProcessRunnerError::InvalidWorkingDirectory(directory.to_string_lossy().into_owned())
    })?;
    unsafe {
        libc::posix_spawn_file_actions_adddup2(&mut actions, output_pipe[1], libc::STDOUT_FILENO);
        libc::posix_spawn_file_actions_adddup2(&mut actions, error_pipe[1], libc::STDERR_FILENO);
        for fd in output_pipe.iter().chain(error_pipe.iter()) {
            libc::posix_spawn_file_actions_addclose(&mut actions, *fd);
        }
        libc::posix_spawn_file_actions_addchdir_np(&mut actions, directory_c.as_ptr());

        let mut default_signals: libc::sigset_t = std::mem::zeroed();
        libc::sigemptyset(&mut default_signals);
        for signal in [
            libc::SIGTERM,
            libc::SIGINT,
            libc::SIGQUIT,
            libc::SIGHUP,
            libc::SIGPIPE,
        ] {
            libc::sigaddset(&mut default_signals, signal);
        }
        let mut signal_mask: libc::sigset_t = std::mem::zeroed();
        libc::sigemptyset(&mut signal_mask);

        libc::posix_spawnattr_setflags(
            &mut attributes,
            (libc::POSIX_SPAWN_SETPGROUP | libc::POSIX_SPAWN_SETSIGDEF | libc::POSIX_SPAWN_SETSIGMASK)
                as i16,
        );
        libc::posix_spawnattr_setpgroup(&mut attributes, 0);
        libc::posix_spawnattr_setsigdefault(&mut attributes, &default_signals);
        libc::posix_spawnattr_setsigmask(&mut attributes, &signal_mask);
    }

    let executable_c = CString::new(executable)
        .map_err(|_| ProcessRunnerError::SpawnFailed { code: libc::EINVAL })?;
    let mut argument_strings = vec![executable.to_string()];
    argument_strings.extend(arguments.iter().cloned());
    let argv_c: Vec<CString> = argument_strings
        .iter()
        .map(|s| CString::new(s.as_str()))
        .collect::<Result<_, _>>()
        .map_err(|_| ProcessRunnerError::SpawnFailed { code: libc::EINVAL })?;
    let mut argv: Vec<*mut i8> = argv_c.iter().map(|s| s.as_ptr() as *mut i8).collect();
    argv.push(std::ptr::null_mut());

    let mut keys: Vec<&String> = environment.keys().collect();
    keys.sort();
    let env_c: Vec<CString> = keys
        .iter()
        .map(|k| CString::new(format!("{k}={}", environment[*k])))
        .collect::<Result<_, _>>()
        .map_err(|_| ProcessRunnerError::SpawnFailed { code: libc::EINVAL })?;
    let mut envp: Vec<*mut i8> = env_c.iter().map(|s| s.as_ptr() as *mut i8).collect();
    envp.push(std::ptr::null_mut());

    let mut pid: libc::pid_t = 0;
    let spawn_code = unsafe {
        libc::posix_spawn(
            &mut pid,
            executable_c.as_ptr(),
            &actions,
            &attributes,
            argv.as_ptr(),
            envp.as_ptr(),
        )
    };
    unsafe {
        libc::close(output_pipe[1]);
        libc::close(error_pipe[1]);
    }
    if spawn_code != 0 {
        unsafe {
            libc::close(output_pipe[0]);
            libc::close(error_pipe[0]);
        }
        return Err(ProcessRunnerError::SpawnFailed { code: spawn_code });
    }
    Ok(SpawnedProcess {
        pid,
        standard_output: output_pipe[0],
        standard_error: error_pipe[0],
    })
}

/// Windows counterpart — `std::process::Command` with piped output. Rust
/// routes .cmd/.bat shims through cmd.exe with its native argument escaping.
/// PowerShell scripts still need an explicit interpreter.
/// `taskkill /T` covers the child tree, matching the Unix process group.
#[cfg(windows)]
fn spawn(
    executable: &str,
    arguments: &[String],
    directory: &Path,
    environment: &HashMap<String, String>,
) -> Result<SpawnedProcess, ProcessRunnerError> {
    let extension = Path::new(executable)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .unwrap_or_default();
    let mut command = match extension.as_str() {
        "ps1" => {
            let mut c = std::process::Command::new("powershell");
            c.args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"])
                .arg(executable)
                .args(arguments);
            c
        }
        _ => {
            let mut c = std::process::Command::new(executable);
            c.args(arguments);
            c
        }
    };
    command
        .creation_flags(CREATE_NO_WINDOW)
        .current_dir(directory)
        .env_clear()
        .envs(environment)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    let mut child = command.spawn().map_err(|e| ProcessRunnerError::SpawnFailed {
        code: e.raw_os_error().unwrap_or(1),
    })?;
    let standard_output = child.stdout.take().ok_or(ProcessRunnerError::PipeFailed { code: 0 })?;
    let standard_error = child.stderr.take().ok_or(ProcessRunnerError::PipeFailed { code: 0 })?;
    Ok(SpawnedProcess {
        pid: child.id() as i32,
        child,
        standard_output,
        standard_error,
    })
}

/// Sequences chunks across both reader threads exactly like the Swift actor.
struct OutputSequencer<'a> {
    sequence: Mutex<u64>,
    handler: Option<&'a (dyn Fn(ProcessOutputChunk) + Send + Sync)>,
}

impl<'a> OutputSequencer<'a> {
    fn emit(&self, channel: ProcessOutputChannel, bytes: Vec<u8>) {
        let chunk = {
            let mut sequence = self.sequence.lock().unwrap();
            let chunk = ProcessOutputChunk {
                sequence: *sequence,
                channel,
                bytes,
            };
            *sequence += 1;
            chunk
        };
        if let Some(handler) = self.handler {
            handler(chunk);
        }
    }
}

#[cfg(unix)]
fn read_fd_into(fd: i32, channel: ProcessOutputChannel, sequencer: &OutputSequencer) -> Vec<u8> {
    let mut collected = Vec::new();
    let mut buffer = [0u8; 4096];
    loop {
        let count = unsafe {
            libc::read(fd, buffer.as_mut_ptr() as *mut libc::c_void, buffer.len())
        };
        if count > 0 {
            let bytes = buffer[..count as usize].to_vec();
            collected.extend_from_slice(&bytes);
            sequencer.emit(channel, bytes);
        } else if count == 0 {
            break;
        } else {
            let err = std::io::Error::last_os_error().raw_os_error().unwrap_or(0);
            if err != libc::EINTR {
                break;
            }
        }
    }
    unsafe {
        libc::close(fd);
    }
    collected
}

/// Windows counterpart — the piped `ChildStdout`/`ChildStderr` implement
/// `Read`, so the loop is plain `std::io`; EOF/closed pipe both end it.
#[cfg(windows)]
fn read_fd_into(
    mut fd: impl std::io::Read,
    channel: ProcessOutputChannel,
    sequencer: &OutputSequencer,
) -> Vec<u8> {
    let mut collected = Vec::new();
    let mut buffer = [0u8; 4096];
    loop {
        match fd.read(&mut buffer) {
            Ok(0) => break,
            Ok(count) => {
                let bytes = buffer[..count].to_vec();
                collected.extend_from_slice(&bytes);
                sequencer.emit(channel, bytes);
            }
            Err(_) => break,
        }
    }
    collected
}

pub struct ProcessRunner {
    pub termination_grace_period: Duration,
}

impl Default for ProcessRunner {
    fn default() -> Self {
        Self::new(Duration::from_millis(500))
    }
}

impl ProcessRunner {
    pub fn new(termination_grace_period: Duration) -> Self {
        Self {
            termination_grace_period,
        }
    }

    pub fn run(
        &self,
        plan: &DirectCommandPlan,
        project_root: &Path,
        source_directory: Option<&Path>,
        timeout: Option<Duration>,
        cancel: Option<&CancellationToken>,
        output_handler: Option<&(dyn Fn(ProcessOutputChunk) + Send + Sync)>,
    ) -> Result<ProcessResult, ProcessRunnerError> {
        self.run_invocation(
            &plan.executable,
            &plan.arguments,
            &plan.working_directory,
            &plan.environment,
            project_root,
            source_directory,
            timeout,
            cancel,
            None,
            output_handler,
        )
    }

    /// The only API that interprets command text. The authority is returned
    /// with the result.
    pub fn run_login_shell(
        &self,
        plan: &LoginShellCommandPlan,
        project_root: &Path,
        source_directory: Option<&Path>,
        timeout: Option<Duration>,
        cancel: Option<&CancellationToken>,
        output_handler: Option<&(dyn Fn(ProcessOutputChunk) + Send + Sync)>,
    ) -> Result<ProcessResult, ProcessRunnerError> {
        self.run_invocation(
            &plan.shell_executable,
            &plan.invocation_arguments(),
            &plan.working_directory,
            &plan.environment,
            project_root,
            source_directory,
            timeout,
            cancel,
            Some(plan.authority.clone()),
            output_handler,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn run_invocation(
        &self,
        executable: &str,
        arguments: &[String],
        working_directory: &WorkingDirectoryPolicy,
        environment: &EnvironmentPolicy,
        project_root: &Path,
        source_directory: Option<&Path>,
        timeout: Option<Duration>,
        cancel: Option<&CancellationToken>,
        shell_authority: Option<ShellAuthority>,
        output_handler: Option<&(dyn Fn(ProcessOutputChunk) + Send + Sync)>,
    ) -> Result<ProcessResult, ProcessRunnerError> {
        let directory = resolve_working_directory(working_directory, project_root, source_directory)?;
        let environment_values = resolved_environment(environment);
        // `mut` only matters on Windows, where `wait` takes `&mut Child`.
        #[allow(unused_mut)]
        let mut spawned = spawn(
            &resolved_executable(executable, &environment_values, &directory)?,
            arguments,
            &directory,
            &environment_values,
        )?;
        let control = Arc::new(ProcessControl::new(spawned.pid, self.termination_grace_period));
        let sequencer = OutputSequencer {
            sequence: Mutex::new(0),
            handler: output_handler,
        };

        std::thread::scope(|scope| {
            let stdout_reader = scope.spawn(|| {
                read_fd_into(spawned.standard_output, ProcessOutputChannel::StandardOutput, &sequencer)
            });
            let stderr_reader = scope.spawn(|| {
                read_fd_into(spawned.standard_error, ProcessOutputChannel::StandardError, &sequencer)
            });
            if let Some(duration) = timeout {
                let control = Arc::clone(&control);
                scope.spawn(move || {
                    let deadline = std::time::Instant::now() + duration;
                    loop {
                        if control.is_exited() {
                            return;
                        }
                        let now = std::time::Instant::now();
                        if now >= deadline {
                            control.request_stop(ProcessStopReason::TimedOut);
                            return;
                        }
                        std::thread::sleep((deadline - now).min(Duration::from_millis(20)));
                    }
                });
            }
            if let Some(token) = cancel {
                let control = Arc::clone(&control);
                scope.spawn(move || {
                    // Equivalent of the Swift `onCancel` handler.
                    while !control.is_exited() {
                        if token.wait_cancelled(Duration::from_millis(20)) {
                            control.request_stop(ProcessStopReason::Cancelled);
                            return;
                        }
                    }
                });
            }

            #[cfg(unix)]
            let status = wait_for_process(spawned.pid)?;
            #[cfg(windows)]
            let status = wait_for_process(&mut spawned.child)?;
            let reason = control.process_did_exit();
            let standard_output = stdout_reader.join().unwrap_or_default();
            let standard_error = stderr_reader.join().unwrap_or_default();
            Ok(ProcessResult {
                process_identifier: spawned.pid,
                termination: decode_wait_status(status),
                stop_reason: reason,
                standard_output,
                standard_error,
                shell_authority,
            })
        })
    }
}

// ===========================================================================
// BuildOrchestrator.swift
// ===========================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BuildToolStage {
    Xelatex,
    Pdflatex,
    Lualatex,
    Latexmk,
    #[serde(rename = "latexmkXeLaTeX")]
    LatexmkXeLaTeX,
    #[serde(rename = "latexmkLuaLaTeX")]
    LatexmkLuaLaTeX,
    Bibtex,
    Makeindex,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BuildStagePlan {
    pub tool: BuildToolStage,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuildPipeline {
    Stages(Vec<BuildStagePlan>),
    Custom(LoginShellCommandPlan),
}

impl BuildPipeline {
    pub fn single_pass(tool: BuildToolStage) -> Self {
        Self::Stages(vec![BuildStagePlan { tool }])
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuildOrchestratorError {
    NoSelection,
    BuildAlreadyRunning,
    EmptyPipeline,
    InvalidProjectRoot,
    PathTraversal(String),
    UndeclaredGeneratedPath(String),
    SourceDeletionRisk,
    ProcessFailed { stage: usize, exit_code: i32 },
    ProcessError(String),
    MissingPDF,
    ArtifactRestorationFailed,
    Cancelled,
    NoBuildRunning,
}

impl fmt::Display for BuildOrchestratorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for BuildOrchestratorError {}

#[derive(Debug, Clone)]
pub struct BuildTarget {
    pub project_root: PathBuf,
    pub source_path: String,
    pub output_pdf_path: String,
    pub declared_generated_paths: HashSet<String>,
    pub build_directory_path: Option<String>,
}

impl BuildTarget {
    pub fn new(
        project_root: &Path,
        source_path: impl Into<String>,
        output_pdf_path: impl Into<String>,
        declared_generated_paths: HashSet<String>,
        build_directory_path: Option<String>,
    ) -> Result<Self, BuildOrchestratorError> {
        let source_path = source_path.into();
        let output_pdf_path = output_pdf_path.into();
        if !project_root.is_absolute() {
            return Err(BuildOrchestratorError::InvalidProjectRoot);
        }
        Self::validate_relative(&source_path)?;
        Self::validate_relative(&output_pdf_path)?;
        for path in &declared_generated_paths {
            Self::validate_relative(path)?;
        }
        if let Some(directory) = &build_directory_path {
            Self::validate_relative(directory)?;
        }
        if source_path == output_pdf_path || !declared_generated_paths.contains(&output_pdf_path) {
            return Err(BuildOrchestratorError::UndeclaredGeneratedPath(output_pdf_path));
        }
        if let Some(directory) = &build_directory_path {
            if !declared_generated_paths.contains(directory) {
                return Err(BuildOrchestratorError::UndeclaredGeneratedPath(
                    directory.clone(),
                ));
            }
            let prefix = format!("{directory}/");
            if source_path == *directory || source_path.starts_with(&prefix) {
                return Err(BuildOrchestratorError::SourceDeletionRisk);
            }
        }
        Ok(Self {
            project_root: resolving_symlinks_in_path(&standardized_path(project_root)),
            source_path,
            output_pdf_path,
            declared_generated_paths,
            build_directory_path,
        })
    }

    pub fn validate_relative(path: &str) -> Result<(), BuildOrchestratorError> {
        if path.is_empty()
            || path.starts_with('/')
            || path.starts_with('~')
            || path.contains('\\')
            || path.chars().any(|c| c == '\0')
        {
            return Err(BuildOrchestratorError::PathTraversal(path.to_string()));
        }
        let components: Vec<&str> = path.split('/').collect();
        if !components
            .iter()
            .all(|c| !c.is_empty() && *c != "." && *c != "..")
            || components.first().map(|c| c.contains(':')).unwrap_or(false)
        {
            return Err(BuildOrchestratorError::PathTraversal(path.to_string()));
        }
        Ok(())
    }

    pub fn url(&self, relative_path: &str) -> Result<PathBuf, BuildOrchestratorError> {
        Self::validate_relative(relative_path)?;
        let candidate =
            resolving_symlinks_in_path(&standardized_path(&self.project_root.join(relative_path)));
        let root_string = self.project_root.to_string_lossy();
        let prefix = if root_string == "/" {
            "/".to_string()
        } else {
            format!("{root_string}/")
        };
        if !candidate.to_string_lossy().starts_with(&prefix) {
            return Err(BuildOrchestratorError::PathTraversal(
                relative_path.to_string(),
            ));
        }
        Ok(candidate)
    }
}

#[derive(Debug, Clone)]
pub enum BuildProcessCommand {
    Direct(DirectCommandPlan),
    LoginShell(LoginShellCommandPlan),
}

#[derive(Debug, Clone)]
pub struct BuildProcessRequest {
    pub build_id: BuildID,
    pub stage_index: usize,
    pub command: BuildProcessCommand,
    pub project_root: PathBuf,
    pub source_directory: PathBuf,
}

#[derive(Debug, Clone)]
pub struct BuildProcessOutput {
    pub channel: BuildLogChannel,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BuildProcessResult {
    pub exit_code: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuildProcessExecutorError {
    ToolNotFound(String),
    LaunchFailed(String),
}

impl fmt::Display for BuildProcessExecutorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for BuildProcessExecutorError {}

pub trait BuildProcessExecuting: Send + Sync {
    fn execute(
        &self,
        request: &BuildProcessRequest,
        output: &mut (dyn FnMut(BuildProcessOutput) + Send),
    ) -> Result<BuildProcessResult, BuildProcessExecutorError>;

    fn cancel(&self, build_id: &BuildID);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BuildArtifactDisposition {
    ReplacedWithSuccessfulPDF,
    PreservedLastSuccessfulPDF {
        #[serde(rename = "partialOutputWasDiscarded")]
        partial_output_was_discarded: bool,
    },
    NoPDF {
        #[serde(rename = "partialOutputWasDiscarded")]
        partial_output_was_discarded: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildOutcome {
    pub build_id: BuildID,
    pub lifecycle: BuildLifecycle,
    pub issues: Vec<BuildIssueRecord>,
    pub artifact_disposition: BuildArtifactDisposition,
}

#[derive(Debug, Clone)]
pub enum BuildEvent {
    Lifecycle(BuildLifecycle),
    Log(BuildLogEntry),
    Issue(BuildIssueRecord),
    StageStarted { index: usize, tool: Option<BuildToolStage> },
}

pub type EventHandler = Arc<dyn Fn(BuildEvent) + Send + Sync>;

struct ActiveBuild {
    id: BuildID,
    lifecycle: BuildLifecycle,
    cancellation_requested: bool,
    parser: BuildLogParser,
    issues: Vec<BuildIssueRecord>,
    issue_keys: HashSet<IssueKey>,
    event_handler: EventHandler,
    /// Bytes of a UTF-8 sequence cut off at the end of a pipe chunk, per
    /// channel — decoded with the next chunk instead of as U+FFFD.
    log_carry: HashMap<BuildLogChannel, Vec<u8>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct IssueKey {
    severity: BuildIssueSeverity,
    message: String,
    file: Option<String>,
    line: Option<i64>,
    column: Option<i64>,
}

impl From<&BuildIssueRecord> for IssueKey {
    fn from(record: &BuildIssueRecord) -> Self {
        Self {
            severity: record.severity,
            message: record.message.clone(),
            file: record.file.clone(),
            line: record.line,
            column: record.column,
        }
    }
}

/// Length of the longest prefix of `bytes` that does not end inside a
/// UTF-8 sequence — only a truncated sequence at the very end is held
/// back; invalid bytes elsewhere still decode to U+FFFD as before.
fn utf8_complete_prefix(bytes: &[u8]) -> usize {
    for back in 1..=bytes.len().min(3) {
        let byte = bytes[bytes.len() - back];
        if byte & 0xC0 == 0x80 {
            continue; // continuation byte: keep looking for its lead
        }
        let needed = match byte {
            0xF0..=0xF7 => 4,
            0xE0..=0xEF => 3,
            0xC0..=0xDF => 2,
            _ => 1,
        };
        return if back < needed { bytes.len() - back } else { bytes.len() };
    }
    bytes.len()
}

struct OrchestratorState {
    target: Option<BuildTarget>,
    pipeline: Option<BuildPipeline>,
    active: Option<ActiveBuild>,
    last_successful_pdf: Option<Vec<u8>>,
    last_successful_pdf_path: Option<PathBuf>,
    most_recent_outcome: Option<BuildOutcome>,
    log_sequence: u64,
}

/// The canonical build orchestrator. All public state transitions are
/// serialized through `state`, exactly like the Swift actor.
pub struct BuildOrchestrator<E: BuildProcessExecuting> {
    executor: E,
    state: Mutex<OrchestratorState>,
    /// Serializes event-handler invocations across the build thread and a
    /// concurrent `cancel` call — the ordering the Swift actor guarantees.
    handler_lock: Mutex<()>,
}

impl<E: BuildProcessExecuting> BuildOrchestrator<E> {
    pub fn new(executor: E) -> Self {
        Self {
            executor,
            handler_lock: Mutex::new(()),
            state: Mutex::new(OrchestratorState {
                target: None,
                pipeline: None,
                active: None,
                last_successful_pdf: None,
                last_successful_pdf_path: None,
                most_recent_outcome: None,
                log_sequence: 0,
            }),
        }
    }

    pub fn select(
        &self,
        target: BuildTarget,
        pipeline: BuildPipeline,
    ) -> Result<(), BuildOrchestratorError> {
        let mut state = self.state.lock().unwrap();
        if state.active.is_some() {
            return Err(BuildOrchestratorError::BuildAlreadyRunning);
        }
        if let BuildPipeline::Stages(stages) = &pipeline {
            if stages.is_empty() {
                return Err(BuildOrchestratorError::EmptyPipeline);
            }
        }
        state.target = Some(target);
        state.pipeline = Some(pipeline);
        Ok(())
    }

    pub fn selected_target(&self) -> Option<BuildTarget> {
        self.state.lock().unwrap().target.clone()
    }

    pub fn selected_pipeline(&self) -> Option<BuildPipeline> {
        self.state.lock().unwrap().pipeline.clone()
    }

    pub fn current_lifecycle(&self) -> Option<BuildLifecycle> {
        self.state
            .lock()
            .unwrap()
            .active
            .as_ref()
            .map(|a| a.lifecycle)
    }

    pub fn successful_pdf(&self) -> Option<Vec<u8>> {
        self.state.lock().unwrap().last_successful_pdf.clone()
    }

    pub fn last_outcome(&self) -> Option<BuildOutcome> {
        self.state.lock().unwrap().most_recent_outcome.clone()
    }

    pub fn build(
        &self,
        id: BuildID,
        event_handler: EventHandler,
    ) -> Result<BuildOutcome, BuildOrchestratorError> {
        let (target, pipeline) = {
            let mut state = self.state.lock().unwrap();
            if state.active.is_some() {
                return Err(BuildOrchestratorError::BuildAlreadyRunning);
            }
            let (Some(target), Some(pipeline)) = (state.target.clone(), state.pipeline.clone()) else {
                return Err(BuildOrchestratorError::NoSelection);
            };
            state.active = Some(ActiveBuild {
                id: id.clone(),
                lifecycle: BuildLifecycle::Running {
                    started_at_milliseconds: now_milliseconds(),
                },
                cancellation_requested: false,
                parser: BuildLogParser::new(&target.project_root),
                issues: Vec::new(),
                issue_keys: HashSet::new(),
                event_handler,
                log_carry: HashMap::new(),
            });
            state.log_sequence = 0;
            (target, pipeline)
        };
        let started = match self.current_lifecycle() {
            Some(BuildLifecycle::Running { started_at_milliseconds }) => started_at_milliseconds,
            _ => unreachable!(),
        };
        self.emit(BuildEvent::Lifecycle(BuildLifecycle::Running {
            started_at_milliseconds: started,
        }));

        let output_url = target.url(&target.output_pdf_path)?;
        let mut preexisting_pdf: Option<Vec<u8>> = None;
        if output_url.exists() {
            match std::fs::read(&output_url) {
                Ok(data) => preexisting_pdf = Some(data),
                Err(_) => {
                    let lifecycle = BuildLifecycle::Failed {
                        exit_code: None,
                        finished_at_milliseconds: now_milliseconds(),
                    };
                    let _ = self.finish(
                        &id,
                        lifecycle,
                        BuildArtifactDisposition::NoPDF {
                            partial_output_was_discarded: false,
                        },
                    );
                    return Err(BuildOrchestratorError::ProcessError(
                        "Unable to preserve the existing PDF before building".into(),
                    ));
                }
            }
        }
        {
            let state = self.state.lock().unwrap();
            if state.last_successful_pdf_path.as_deref() == Some(output_url.as_path()) {
                if let Some(last) = &state.last_successful_pdf {
                    preexisting_pdf = Some(last.clone());
                }
            }
        }

        let run_result = (|| -> Result<(), BuildOrchestratorError> {
            match &pipeline {
                BuildPipeline::Stages(stages) => {
                    for (index, stage) in stages.iter().enumerate() {
                        self.ensure_not_cancelled(&id)?;
                        self.emit(BuildEvent::StageStarted {
                            index,
                            tool: Some(stage.tool),
                        });
                        let plan = command_plan_for(stage.tool, &target)?;
                        let request = BuildProcessRequest {
                            build_id: id.clone(),
                            stage_index: index,
                            command: BuildProcessCommand::Direct(plan),
                            project_root: target.project_root.clone(),
                            source_directory: source_directory_for(&target)?,
                        };
                        let result = self.execute_with_receive(&request, &id)?;
                        self.ensure_not_cancelled(&id)?;
                        if result.exit_code != 0 {
                            return Err(BuildOrchestratorError::ProcessFailed {
                                stage: index,
                                exit_code: result.exit_code,
                            });
                        }
                    }
                }
                BuildPipeline::Custom(command) => {
                    self.ensure_not_cancelled(&id)?;
                    self.emit(BuildEvent::StageStarted {
                        index: 0,
                        tool: None,
                    });
                    let request = BuildProcessRequest {
                        build_id: id.clone(),
                        stage_index: 0,
                        command: BuildProcessCommand::LoginShell(command.clone()),
                        project_root: target.project_root.clone(),
                        source_directory: source_directory_for(&target)?,
                    };
                    let result = self.execute_with_receive(&request, &id)?;
                    self.ensure_not_cancelled(&id)?;
                    if result.exit_code != 0 {
                        return Err(BuildOrchestratorError::ProcessFailed {
                            stage: 0,
                            exit_code: result.exit_code,
                        });
                    }
                }
            }
            Ok(())
        })();

        let mut stage_failure = run_result.err();

        self.flush_parser(&id);
        let partial = output_changed(&output_url, preexisting_pdf.as_deref());
        let cancelled = self
            .state
            .lock()
            .unwrap()
            .active
            .as_ref()
            .map(|a| a.cancellation_requested)
            .unwrap_or(false);

        if cancelled {
            let disposition = self.restore_for_build(
                &id,
                preexisting_pdf.as_deref(),
                &output_url,
                partial,
            )?;
            let lifecycle = BuildLifecycle::Cancelled {
                finished_at_milliseconds: now_milliseconds(),
            };
            return Ok(self.finish(&id, lifecycle, disposition));
        }
        if let Some(failure) = stage_failure.take() {
            let disposition = self.restore_for_build(
                &id,
                preexisting_pdf.as_deref(),
                &output_url,
                partial,
            )?;
            let lifecycle = BuildLifecycle::Failed {
                exit_code: failure_exit_code(&failure),
                finished_at_milliseconds: now_milliseconds(),
            };
            let _ = self.finish(&id, lifecycle, disposition);
            return Err(failure);
        }
        let successful_data = match std::fs::read(&output_url) {
            Ok(data) => data,
            Err(_) => {
                let disposition = self.restore_for_build(
                    &id,
                    preexisting_pdf.as_deref(),
                    &output_url,
                    partial,
                )?;
                let lifecycle = BuildLifecycle::Failed {
                    exit_code: None,
                    finished_at_milliseconds: now_milliseconds(),
                };
                let _ = self.finish(&id, lifecycle, disposition);
                return Err(BuildOrchestratorError::MissingPDF);
            }
        };

        {
            let mut state = self.state.lock().unwrap();
            state.last_successful_pdf = Some(successful_data);
            state.last_successful_pdf_path = Some(output_url.clone());
        }
        let lifecycle = BuildLifecycle::Succeeded {
            exit_code: 0,
            finished_at_milliseconds: now_milliseconds(),
        };
        Ok(self.finish(
            &id,
            lifecycle,
            BuildArtifactDisposition::ReplacedWithSuccessfulPDF,
        ))
    }

    fn execute_with_receive(
        &self,
        request: &BuildProcessRequest,
        id: &BuildID,
    ) -> Result<BuildProcessResult, BuildOrchestratorError> {
        let result = self.executor.execute(request, &mut |output| {
            self.receive(output, id);
        });
        result.map_err(|error| {
            // Swift: `catch let error as BuildProcessExecutorError` ->
            // processError(String(describing:)).
            BuildOrchestratorError::ProcessError(format!("{error:?}"))
        })
    }

    pub fn cancel(&self, grace_period_milliseconds: u64) -> Result<(), BuildOrchestratorError> {
        let id = {
            let mut state = self.state.lock().unwrap();
            let Some(active) = state.active.as_mut() else {
                return Err(BuildOrchestratorError::NoBuildRunning);
            };
            if active.cancellation_requested {
                return Ok(());
            }
            BuildCancellation::new(now_milliseconds(), grace_period_milliseconds)
                .map_err(|_| BuildOrchestratorError::ProcessError("invalid cancellation".into()))?;
            active.cancellation_requested = true;
            active.lifecycle = BuildLifecycle::Cancelling;
            active.id.clone()
        };
        self.emit(BuildEvent::Lifecycle(BuildLifecycle::Cancelling));
        self.executor.cancel(&id);
        Ok(())
    }

    pub fn cleanup(&self, policy: &CleanupPolicy) -> Result<Vec<String>, BuildOrchestratorError> {
        let target = {
            let state = self.state.lock().unwrap();
            if state.active.is_some() {
                return Err(BuildOrchestratorError::BuildAlreadyRunning);
            }
            let Some(target) = state.target.clone() else {
                return Err(BuildOrchestratorError::NoSelection);
            };
            target
        };
        let candidates: HashSet<String> = match policy {
            CleanupPolicy::PreserveAll => return Ok(vec![]),
            CleanupPolicy::RemoveKnownAuxiliaryFiles(paths) => paths.clone(),
            CleanupPolicy::RemoveBuildDirectory => match &target.build_directory_path {
                Some(directory) => [directory.clone()].into_iter().collect(),
                None => return Ok(vec![]),
            },
        };

        let mut removed: Vec<String> = Vec::new();
        let mut sorted: Vec<String> = candidates.into_iter().collect();
        sorted.sort();
        for path in sorted {
            BuildTarget::validate_relative(&path)?;
            if !target.declared_generated_paths.contains(&path) {
                return Err(BuildOrchestratorError::UndeclaredGeneratedPath(path));
            }
            if path == target.source_path || path == target.output_pdf_path {
                if path == target.source_path {
                    return Err(BuildOrchestratorError::SourceDeletionRisk);
                }
                continue;
            }
            let url = target.url(&path)?;
            if url.exists() {
                remove_item(&url).map_err(|_| {
                    BuildOrchestratorError::ProcessError(format!("unable to remove {path}"))
                })?;
                removed.push(path);
            }
        }
        Ok(removed)
    }

    fn receive(&self, output: BuildProcessOutput, id: &BuildID) {
        let (log, parsed) = {
            let mut state = self.state.lock().unwrap();
            let active = match state.active.as_mut() {
                Some(active) if active.id == *id => active,
                _ => return,
            };
            // Pipes cut chunks at arbitrary bytes: a multi-byte character
            // (Korean in xelatex/lualatex logs) split across two chunks
            // used to show up as U+FFFD twice in the log view.
            let carry = active.log_carry.entry(output.channel).or_default();
            carry.extend_from_slice(&output.bytes);
            let complete = utf8_complete_prefix(carry);
            let text = String::from_utf8_lossy(&carry[..complete]).into_owned();
            carry.drain(..complete);
            let parsed = active.parser.consume_bytes(&output.bytes, output.channel);
            let log = (!text.is_empty()).then(|| {
                let log = BuildLogEntry {
                    sequence: state.log_sequence,
                    channel: output.channel,
                    text,
                };
                state.log_sequence += 1;
                log
            });
            (log, parsed)
        };
        if let Some(log) = log {
            self.emit(BuildEvent::Log(log));
        }
        self.emit_issues(parsed, id);
    }

    fn flush_parser(&self, id: &BuildID) {
        let (parsed, logs) = {
            let mut state = self.state.lock().unwrap();
            let active = match state.active.as_mut() {
                Some(active) if active.id == *id => active,
                _ => return,
            };
            let parsed = active.parser.finish();
            // A sequence still incomplete when the process ended is kept in
            // the log as-is (lossily), exactly as before.
            let mut tails: Vec<(BuildLogChannel, String)> = active
                .log_carry
                .drain()
                .filter(|(_, bytes)| !bytes.is_empty())
                .map(|(channel, bytes)| (channel, String::from_utf8_lossy(&bytes).into_owned()))
                .collect();
            tails.sort_by_key(|(channel, _)| *channel as u8);
            let logs: Vec<BuildLogEntry> = tails
                .into_iter()
                .map(|(channel, text)| {
                    let log = BuildLogEntry { sequence: state.log_sequence, channel, text };
                    state.log_sequence += 1;
                    log
                })
                .collect();
            (parsed, logs)
        };
        for log in logs {
            self.emit(BuildEvent::Log(log));
        }
        self.emit_issues(parsed, id);
    }

    fn emit_issues(&self, records: Vec<BuildIssueRecord>, id: &BuildID) {
        for record in records {
            let emit = {
                let mut state = self.state.lock().unwrap();
                match state.active.as_mut() {
                    Some(active) if active.id == *id => {
                        let key = IssueKey::from(&record);
                        if active.issue_keys.insert(key) {
                            active.issues.push(record.clone());
                            true
                        } else {
                            false
                        }
                    }
                    _ => return,
                }
            };
            if emit {
                self.emit(BuildEvent::Issue(record));
            }
        }
    }

    fn finish(
        &self,
        id: &BuildID,
        lifecycle: BuildLifecycle,
        disposition: BuildArtifactDisposition,
    ) -> BuildOutcome {
        let (outcome, handler) = {
            let mut state = self.state.lock().unwrap();
            let issues = match state.active.as_ref() {
                Some(active) if active.id == *id => active.issues.clone(),
                _ => Vec::new(),
            };
            let handler = state.active.as_ref().map(|a| Arc::clone(&a.event_handler));
            state.active = None;
            let outcome = BuildOutcome {
                build_id: id.clone(),
                lifecycle,
                issues,
                artifact_disposition: disposition,
            };
            state.most_recent_outcome = Some(outcome.clone());
            (outcome, handler)
        };
        if let Some(handler) = handler {
            let _guard = self.handler_lock.lock().unwrap();
            handler(BuildEvent::Lifecycle(lifecycle));
        }
        outcome
    }

    /// Every event emission funnels through here so `cancel` on another
    /// thread interleaves deterministically, as on the Swift actor.
    fn emit(&self, event: BuildEvent) {
        let handler = {
            self.state
                .lock()
                .unwrap()
                .active
                .as_ref()
                .map(|a| Arc::clone(&a.event_handler))
        };
        if let Some(handler) = handler {
            let _guard = self.handler_lock.lock().unwrap();
            handler(event);
        }
    }

    fn ensure_not_cancelled(&self, id: &BuildID) -> Result<(), BuildOrchestratorError> {
        let state = self.state.lock().unwrap();
        match state.active.as_ref() {
            Some(active) if active.id == *id && !active.cancellation_requested => Ok(()),
            _ => Err(BuildOrchestratorError::Cancelled),
        }
    }

    fn restore(
        &self,
        previous: Option<&[u8]>,
        output_url: &Path,
        partial: bool,
    ) -> Result<BuildArtifactDisposition, BuildOrchestratorError> {
        if partial {
            let result = if let Some(previous) = previous {
                atomic_write(output_url, previous)
            } else if output_url.exists() {
                remove_item(output_url)
            } else {
                Ok(())
            };
            if result.is_err() {
                return Err(BuildOrchestratorError::ArtifactRestorationFailed);
            }
        }
        let had_last = {
            let state = self.state.lock().unwrap();
            state.last_successful_pdf_path.as_deref() == Some(output_url)
        };
        if previous.is_some() || had_last {
            return Ok(BuildArtifactDisposition::PreservedLastSuccessfulPDF {
                partial_output_was_discarded: partial,
            });
        }
        Ok(BuildArtifactDisposition::NoPDF {
            partial_output_was_discarded: partial,
        })
    }

    fn restore_for_build(
        &self,
        id: &BuildID,
        previous: Option<&[u8]>,
        output_url: &Path,
        partial: bool,
    ) -> Result<BuildArtifactDisposition, BuildOrchestratorError> {
        match self.restore(previous, output_url, partial) {
            Ok(disposition) => Ok(disposition),
            Err(error) => {
                let lifecycle = BuildLifecycle::Failed {
                    exit_code: None,
                    finished_at_milliseconds: now_milliseconds(),
                };
                let disposition = if previous.is_none() {
                    BuildArtifactDisposition::NoPDF {
                        partial_output_was_discarded: partial,
                    }
                } else {
                    BuildArtifactDisposition::PreservedLastSuccessfulPDF {
                        partial_output_was_discarded: partial,
                    }
                };
                let _ = self.finish(id, lifecycle, disposition);
                Err(error)
            }
        }
    }
}


fn failure_exit_code(error: &BuildOrchestratorError) -> Option<i32> {
    if let BuildOrchestratorError::ProcessFailed { exit_code, .. } = error {
        Some(*exit_code)
    } else {
        None
    }
}

fn command_plan_for(
    tool: BuildToolStage,
    target: &BuildTarget,
) -> Result<DirectCommandPlan, BuildOrchestratorError> {
    let source = target.source_path.clone();
    let plan = match tool {
        BuildToolStage::Xelatex => DirectCommandPlan::new(
            "xelatex",
            vec![
                "-synctex=1".into(),
                "-interaction=nonstopmode".into(),
                "-file-line-error".into(),
                source,
            ],
            WorkingDirectoryPolicy::ProjectRoot,
            EnvironmentPolicy::Inherit {
                overrides: HashMap::new(),
            },
        ),
        BuildToolStage::Pdflatex => DirectCommandPlan::new(
            "pdflatex",
            vec![
                "-synctex=1".into(),
                "-interaction=nonstopmode".into(),
                "-file-line-error".into(),
                source,
            ],
            WorkingDirectoryPolicy::ProjectRoot,
            EnvironmentPolicy::Inherit {
                overrides: HashMap::new(),
            },
        ),
        BuildToolStage::Lualatex => DirectCommandPlan::new(
            "lualatex",
            vec![
                "-synctex=1".into(),
                "--interaction=nonstopmode".into(),
                "--file-line-error".into(),
                source,
            ],
            WorkingDirectoryPolicy::ProjectRoot,
            EnvironmentPolicy::Inherit {
                overrides: HashMap::new(),
            },
        ),
        BuildToolStage::Latexmk
        | BuildToolStage::LatexmkXeLaTeX
        | BuildToolStage::LatexmkLuaLaTeX => {
            let engine = match tool {
                BuildToolStage::LatexmkXeLaTeX => "-pdfxe",
                BuildToolStage::LatexmkLuaLaTeX => "-pdflua",
                _ => "-pdf",
            };
            // latexmk owns bibliography/index dependencies and reruns. -cd
            // keeps relative inputs and output files beside the main source,
            // even when the workspace contains a nested manuscript directory.
            DirectCommandPlan::new(
                "latexmk",
                vec![
                    engine.into(),
                    "-cd".into(),
                    "-synctex=1".into(),
                    "-interaction=nonstopmode".into(),
                    "-file-line-error".into(),
                    source,
                ],
                WorkingDirectoryPolicy::ProjectRoot,
                EnvironmentPolicy::Inherit {
                    overrides: HashMap::new(),
                },
            )
        }
        BuildToolStage::Bibtex => DirectCommandPlan::new(
            "bibtex",
            vec![job_name(&target.source_path)],
            WorkingDirectoryPolicy::ProjectRoot,
            EnvironmentPolicy::Inherit {
                overrides: HashMap::new(),
            },
        ),
        BuildToolStage::Makeindex => DirectCommandPlan::new(
            "makeindex",
            vec![job_name(&target.source_path)],
            WorkingDirectoryPolicy::ProjectRoot,
            EnvironmentPolicy::Inherit {
                overrides: HashMap::new(),
            },
        ),
    };
    plan.map_err(|e| BuildOrchestratorError::ProcessError(format!("{e:?}")))
}

fn job_name(source_path: &str) -> String {
    match source_path.rfind('/') {
        Some(slash) => {
            let directory = &source_path[..slash];
            let base = &source_path[slash + 1..];
            format!("{directory}/{}", deleting_path_extension(base))
        }
        None => deleting_path_extension(source_path),
    }
}

fn source_directory_for(target: &BuildTarget) -> Result<PathBuf, BuildOrchestratorError> {
    let source_url = target.url(&target.source_path)?;
    Ok(source_url
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| target.project_root.clone()))
}

fn output_changed(url: &Path, previous: Option<&[u8]>) -> bool {
    let current = std::fs::read(url).ok();
    current.as_deref() != previous
}

// ===========================================================================
// StreamingBuildExecutor (Mac/Sources/Features/BuildSupport.swift)
// ===========================================================================

/// Streams build output through the portable POSIX ProcessRunner so the log
/// updates live and cancellation terminates the whole spawned process group.
pub struct StreamingBuildExecutor {
    runner: ProcessRunner,
    running: Mutex<HashMap<String, CancellationToken>>,
}

impl Default for StreamingBuildExecutor {
    fn default() -> Self {
        Self::new(ProcessRunner::default())
    }
}

impl StreamingBuildExecutor {
    pub fn new(runner: ProcessRunner) -> Self {
        Self {
            runner,
            running: Mutex::new(HashMap::new()),
        }
    }

    /// App launches do not load shell profiles. Include the standard TeX Live
    /// locations for both the engine and subprocesses such as bibtex —
    /// the Linux counterpart of `/Library/TeX/texbin`.
    ///
    /// The macOS build additionally extracts a native Biber slice into an
    /// app-owned cache because MacTeX's universal launchers invoke a removed
    /// `lipo` option. Linux biber binaries are already native, so the PATH
    /// augmentation below is the complete counterpart here.
    fn build_environment(policy: &EnvironmentPolicy) -> EnvironmentPolicy {
        let EnvironmentPolicy::Inherit { overrides } = policy else {
            return policy.clone();
        };
        let mut overrides = overrides.clone();
        let path = overrides
            .get("PATH")
            .cloned()
            .or_else(|| std::env::var("PATH").ok())
            .unwrap_or_else(|| default_path_fallback().to_string());
        let mut extra: Vec<String> = texlive_bin_dirs()
            .iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect();
        if cfg!(unix) {
            extra.push("/usr/local/bin".to_string());
        }
        if extra.is_empty() {
            overrides.insert("PATH".to_string(), path);
        } else {
            let separator = if cfg!(windows) { ";" } else { ":" };
            overrides.insert(
                "PATH".to_string(),
                format!("{path}{}{}", separator, extra.join(separator)),
            );
        }
        EnvironmentPolicy::Inherit { overrides }
    }
}

/// PATH used when the environment carries none at all.
fn default_path_fallback() -> &'static str {
    if cfg!(windows) {
        ""
    } else {
        "/usr/bin:/bin:/usr/sbin:/sbin"
    }
}

/// Upstream TeX Live installs under `/usr/local/texlive/<year>/bin/<arch>-linux`
/// (the distro package lands in `/usr/bin` directly). Newest year first.
#[cfg(unix)]
pub fn texlive_bin_dirs() -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = std::fs::read_dir("/usr/local/texlive")
        .map(|entries| {
            entries
                .flatten()
                .map(|e| e.path().join("bin"))
                .flat_map(|bin| {
                    std::fs::read_dir(&bin)
                        .map(|arches| {
                            arches
                                .flatten()
                                .map(|a| a.path())
                                .filter(|p| {
                                    p.is_dir()
                                        && p.file_name()
                                            .map(|n| n.to_string_lossy().ends_with("-linux"))
                                            .unwrap_or(false)
                                })
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default()
                })
                .collect()
        })
        .unwrap_or_default();
    dirs.sort();
    dirs.reverse();
    dirs
}

/// Windows counterpart — TeX Live installs under `C:\texlive\<year>\bin`
/// (`windows` on 2023+, `win32` before); MiKTeX lands under Program Files
/// or the per-user `%LOCALAPPDATA%\Programs` prefix. Newest TeX Live year
/// first, MiKTeX after.
#[cfg(windows)]
pub fn texlive_bin_dirs() -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = Vec::new();
    let mut years: Vec<PathBuf> = std::fs::read_dir("C:/texlive")
        .map(|entries| entries.flatten().map(|e| e.path()).collect())
        .unwrap_or_default();
    years.sort_by(|a, b| b.cmp(a));
    for year in years {
        for arch in ["windows", "win32"] {
            let candidate = year.join("bin").join(arch);
            if candidate.is_dir() {
                dirs.push(candidate);
            }
        }
    }
    for candidate in [
        PathBuf::from("C:/Program Files/MiKTeX/miktex/bin/x64"),
        std::env::var("LOCALAPPDATA")
            .map(|d| PathBuf::from(d).join("Programs/MiKTeX/miktex/bin/x64"))
            .unwrap_or_default(),
    ] {
        if candidate.is_dir() {
            dirs.push(candidate);
        }
    }
    dirs
}

impl BuildProcessExecuting for StreamingBuildExecutor {
    fn execute(
        &self,
        request: &BuildProcessRequest,
        output: &mut (dyn FnMut(BuildProcessOutput) + Send),
    ) -> Result<BuildProcessResult, BuildProcessExecutorError> {
        let token = CancellationToken::new();
        self.running
            .lock()
            .unwrap()
            .insert(request.build_id.raw_value.clone(), token.clone());
        let output = Mutex::new(output);
        let handler = |chunk: ProcessOutputChunk| {
            (output.lock().unwrap())(BuildProcessOutput {
                channel: match chunk.channel {
                    ProcessOutputChannel::StandardOutput => BuildLogChannel::StandardOutput,
                    ProcessOutputChannel::StandardError => BuildLogChannel::StandardError,
                },
                bytes: chunk.bytes,
            });
        };
        let result = match &request.command {
            BuildProcessCommand::Direct(plan) => self.runner.run(
                &DirectCommandPlan {
                    environment: Self::build_environment(&plan.environment),
                    ..plan.clone()
                },
                &request.project_root,
                Some(&request.source_directory),
                None,
                Some(&token),
                Some(&handler),
            ),
            BuildProcessCommand::LoginShell(plan) => self.runner.run_login_shell(
                &LoginShellCommandPlan {
                    environment: Self::build_environment(&plan.environment),
                    ..plan.clone()
                },
                &request.project_root,
                Some(&request.source_directory),
                None,
                Some(&token),
                Some(&handler),
            ),
        };
        self.running
            .lock()
            .unwrap()
            .remove(&request.build_id.raw_value);
        let result = result.map_err(|e| BuildProcessExecutorError::LaunchFailed(format!("{e}")))?;
        Ok(BuildProcessResult {
            exit_code: match result.termination {
                ProcessTermination::Exited { code } => code,
                ProcessTermination::Signaled { signal } => 128 + signal,
            },
        })
    }

    fn cancel(&self, build_id: &BuildID) {
        if let Some(token) = self.running.lock().unwrap().get(&build_id.raw_value) {
            token.cancel();
        }
    }
}
