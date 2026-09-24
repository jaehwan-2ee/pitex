//! SSH remote editing: `~/.ssh/config` parsing, the `ssh` client wrapper,
//! mirror bookkeeping and the three-way sync engine, plus the remote build
//! executor. Port of `Packages/TexCore/Sources/RemoteCore`. Unix-only —
//! the crate is empty on Windows, which keeps `pitex-shell` portable.
#![cfg(unix)]

use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap, HashSet};
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, OnceLock};
use std::time::SystemTime;

use build_core::{
    BuildID, BuildLogChannel, BuildProcessCommand, BuildProcessExecutorError,
    BuildProcessExecuting, BuildProcessOutput, BuildProcessRequest, BuildProcessResult,
    EnvironmentPolicy, WorkingDirectoryPolicy,
};

fn home_dir() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"))
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

// ─── SSHConfig.swift ─────────────────────────────────────────────────────────

/// One concrete `Host` alias from an OpenSSH client config — the entries the
/// Settings "Add SSH Connection" sheet offers. Wildcard/negated patterns
/// (`Host *`, `Host !foo`) and `Match` blocks are defaults, not devices, so
/// they never become entries.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SshHostEntry {
    pub alias: String,
    #[serde(rename = "hostName", skip_serializing_if = "Option::is_none")]
    pub host_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<i64>,
}

impl SshHostEntry {
    pub fn new(
        alias: impl Into<String>,
        host_name: Option<String>,
        user: Option<String>,
        port: Option<i64>,
    ) -> Self {
        Self {
            alias: alias.into(),
            host_name,
            user,
            port,
        }
    }

    /// "user@hostname:port" as ssh would resolve it — the sheet's subtitle.
    pub fn summary(&self) -> String {
        let mut text = self.host_name.clone().unwrap_or_else(|| self.alias.clone());
        if let Some(user) = &self.user {
            text = format!("{user}@{text}");
        }
        if let Some(port) = self.port {
            if port != 22 {
                text += &format!(":{port}");
            }
        }
        text
    }
}

pub enum SshConfigParser {}

impl SshConfigParser {
    /// `~/.ssh/config` of the current user.
    pub fn default_config_path() -> PathBuf {
        home_dir().join(".ssh/config")
    }

    /// Hosts from `path`, following `Include` like ssh does (relative paths
    /// resolve against `~/.ssh`, `*`/`?` globs expand, depth-limited so an
    /// include cycle cannot loop). A missing file yields no hosts.
    pub fn load_hosts(from: &Path) -> Vec<SshHostEntry> {
        let mut hosts = Vec::new();
        let mut seen = HashSet::new();
        Self::collect(from, 0, &mut hosts, &mut seen);
        hosts
    }

    /// Hosts declared in one config text; `include` resolves `Include`
    /// arguments to further config texts.
    pub fn hosts_in(
        text: &str,
        include: impl Fn(&str) -> Vec<String>,
    ) -> Vec<SshHostEntry> {
        let mut hosts = Vec::new();
        let mut seen = HashSet::new();
        Self::parse(text, &include, &mut hosts, &mut seen, 0);
        hosts
    }

    fn collect(path: &Path, depth: usize, hosts: &mut Vec<SshHostEntry>, seen: &mut HashSet<String>) {
        if depth >= 8 {
            return;
        }
        let Ok(text) = std::fs::read_to_string(path) else { return };
        Self::parse(
            &text,
            &|argument| {
                Self::expand_include(argument)
                    .iter()
                    .filter_map(|u| std::fs::read_to_string(u).ok())
                    .collect()
            },
            hosts,
            seen,
            depth,
        );
    }

    fn parse(
        text: &str,
        include: &dyn Fn(&str) -> Vec<String>,
        hosts: &mut Vec<SshHostEntry>,
        seen: &mut HashSet<String>,
        depth: usize,
    ) {
        // Indexes into `hosts` the current Host block applies to; None while
        // inside a Match block (or before any Host line: global defaults).
        let mut current: Option<Vec<usize>> = None;
        for raw_line in split_newlines(text) {
            let Some((keyword, arguments)) = Self::split(raw_line) else { continue };
            match keyword.as_str() {
                "host" => {
                    let mut indexes = Vec::new();
                    for alias in &arguments {
                        if Self::is_concrete_alias(alias) && seen.insert(alias.clone()) {
                            hosts.push(SshHostEntry::new(alias.clone(), None, None, None));
                            indexes.push(hosts.len() - 1);
                        }
                    }
                    current = Some(indexes);
                }
                "match" => current = None,
                "include" => {
                    // ssh splices the included files right here; host entries
                    // they declare are hosts too.
                    if depth >= 8 {
                        continue;
                    }
                    for argument in &arguments {
                        for included in include(argument) {
                            Self::parse(&included, include, hosts, seen, depth + 1);
                        }
                    }
                }
                "hostname" | "user" | "port" => {
                    // First value wins, as in ssh.
                    let (Some(indexes), Some(value)) = (&current, arguments.first()) else {
                        continue;
                    };
                    for &index in indexes {
                        let entry = &mut hosts[index];
                        match keyword.as_str() {
                            "hostname" if entry.host_name.is_none() => {
                                entry.host_name = Some(value.clone())
                            }
                            "user" if entry.user.is_none() => entry.user = Some(value.clone()),
                            "port" if entry.port.is_none() => entry.port = value.parse().ok(),
                            _ => {}
                        }
                    }
                }
                _ => continue,
            }
        }
    }

    /// Keyword (lowercased) and arguments of one config line, or None for a
    /// blank/comment line. Keywords are separated from values by whitespace
    /// or `=`; `"quoted values"` keep their spaces; `#` starts a comment
    /// outside quotes.
    pub fn split(line: &str) -> Option<(String, Vec<String>)> {
        let mut tokens: Vec<String> = Vec::new();
        let mut token = String::new();
        let mut in_quotes = false;
        let mut has_token = false;
        for character in line.chars() {
            if in_quotes {
                if character == '"' {
                    in_quotes = false;
                } else {
                    token.push(character);
                }
                continue;
            }
            match character {
                '"' => {
                    in_quotes = true;
                    has_token = true;
                }
                '#' => {
                    if has_token {
                        tokens.push(std::mem::take(&mut token));
                    }
                    return Self::finish(tokens);
                }
                ' ' | '\t' => {
                    if has_token {
                        tokens.push(std::mem::take(&mut token));
                        has_token = false;
                    }
                }
                '=' if (tokens.is_empty() && has_token) || (tokens.len() == 1 && !has_token) => {
                    // `Keyword=value` / `Keyword = value`: only the first `=`
                    // after the keyword separates; later ones are literal.
                    if has_token {
                        tokens.push(std::mem::take(&mut token));
                        has_token = false;
                    }
                }
                _ => {
                    token.push(character);
                    has_token = true;
                }
            }
        }
        if has_token {
            tokens.push(token);
        }
        Self::finish(tokens)
    }

    fn finish(tokens: Vec<String>) -> Option<(String, Vec<String>)> {
        let (first, rest) = tokens.split_first()?;
        Some((first.to_lowercase(), rest.to_vec()))
    }

    fn is_concrete_alias(alias: &str) -> bool {
        !alias.is_empty() && !alias.chars().any(|c| "*?!".contains(c))
    }

    /// Include paths: `~` expands, relative paths are under `~/.ssh`, and a
    /// `*`/`?` glob in the last component matches files in that directory.
    pub fn expand_include(argument: &str) -> Vec<PathBuf> {
        let home = home_dir();
        let mut path = argument.to_string();
        if let Some(rest) = path.strip_prefix("~/") {
            path = format!("{}/{rest}", home.display());
        }
        if !path.starts_with('/') {
            path = format!("{}/.ssh/{path}", home.display());
        }
        let url = PathBuf::from(&path);
        let pattern = url
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        if !pattern.chars().any(|c| "*?".contains(c)) {
            return vec![url];
        }
        let directory = url.parent().map(|p| p.to_path_buf()).unwrap_or_default();
        let mut names: Vec<String> = std::fs::read_dir(&directory)
            .map(|entries| {
                entries
                    .flatten()
                    .map(|e| e.file_name().to_string_lossy().into_owned())
                    .collect()
            })
            .unwrap_or_default();
        names.sort();
        names
            .into_iter()
            .filter(|name| Self::matches(&pattern, name))
            .map(|name| directory.join(name))
            .collect()
    }

    /// `*` / `?` wildcard match (no character classes — ssh configs rarely
    /// use them in Include).
    pub fn matches(pattern: &str, name: &str) -> bool {
        let p: Vec<char> = pattern.chars().collect();
        let n: Vec<char> = name.chars().collect();
        let mut memo = vec![vec![None; n.len() + 1]; p.len() + 1];
        fn matches_at(p: &[char], n: &[char], memo: &mut [Vec<Option<bool>>], i: usize, j: usize) -> bool {
            if let Some(known) = memo[i][j] {
                return known;
            }
            let result = if i == p.len() {
                j == n.len()
            } else if p[i] == '*' {
                matches_at(p, n, memo, i + 1, j) || (j < n.len() && matches_at(p, n, memo, i, j + 1))
            } else {
                j < n.len() && (p[i] == '?' || p[i] == n[j]) && matches_at(p, n, memo, i + 1, j + 1)
            };
            memo[i][j] = Some(result);
            result
        }
        matches_at(&p, &n, &mut memo, 0, 0)
    }
}

/// `text` split on the same separators as Swift's `Character.isNewline`:
/// LF, CR, CRLF (one separator), VT, FF, NEL, LS, PS.
fn split_newlines(text: &str) -> Vec<&str> {
    let mut lines = Vec::new();
    let mut start = 0;
    let mut iter = text.char_indices().peekable();
    while let Some((i, c)) = iter.next() {
        let newline = matches!(c, '\n' | '\r' | '\u{0b}' | '\u{0c}' | '\u{85}' | '\u{2028}' | '\u{2029}');
        if !newline {
            continue;
        }
        let mut end = i + c.len_utf8();
        if c == '\r' && iter.peek().is_some_and(|(_, next)| *next == '\n') {
            iter.next();
            end += 1;
        }
        lines.push(&text[start..i]);
        start = end;
    }
    lines.push(&text[start..]);
    lines
}

// ─── SSHClient.swift ─────────────────────────────────────────────────────────

/// A saved device the user can open folders on — an `~/.ssh/config` alias
/// (ssh resolves HostName/User/Port/keys itself) or a manually entered host.
/// `id` carries the uppercase-hex UUID text Swift's `UUID` encodes, so a
/// `remote.json` written on macOS decodes identically.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SshConnection {
    pub id: String,
    /// Shown in pickers ("Mac mini").
    pub name: String,
    /// The ssh destination: a config alias or a host name / address.
    pub destination: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<i64>,
    /// Private key for manually added hosts (config aliases carry their own).
    #[serde(rename = "identityFile", skip_serializing_if = "Option::is_none")]
    pub identity_file: Option<String>,
}

impl SshConnection {
    pub fn new(
        name: impl Into<String>,
        destination: impl Into<String>,
        user: Option<String>,
        port: Option<i64>,
        identity_file: Option<String>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string().to_uppercase(),
            name: name.into(),
            destination: destination.into(),
            user,
            port,
            identity_file,
        }
    }

    pub fn from_config_host(entry: &SshHostEntry) -> Self {
        Self::new(entry.alias.clone(), entry.alias.clone(), None, None, None)
    }

    /// Rejects values that ssh could read as options or that would not
    /// survive argv intact: no leading `-`, no whitespace or control
    /// characters, ports in range.
    pub fn validation_error(&self) -> Option<&'static str> {
        fn unsafe_value(value: &str) -> bool {
            value.starts_with('-')
                || value
                    .chars()
                    .any(|c| c.is_whitespace() || c.is_control())
        }
        if self.destination.is_empty() {
            return Some("Enter a host.");
        }
        if unsafe_value(&self.destination) {
            return Some("The host may not start with '-' or contain spaces.");
        }
        if let Some(user) = &self.user {
            if !user.is_empty() && (unsafe_value(user) || user.contains('@')) {
                return Some("The user name may not start with '-' or contain spaces or '@'.");
            }
        }
        if let Some(port) = self.port {
            if !(1..=65535).contains(&port) {
                return Some("The port must be between 1 and 65535.");
            }
        }
        if let Some(identity) = &self.identity_file {
            if identity.chars().any(|c| c.is_control()) {
                return Some("The key path contains control characters.");
            }
        }
        None
    }

    /// ssh options this connection adds before the destination.
    fn connection_arguments(&self) -> Vec<String> {
        let mut arguments: Vec<String> = Vec::new();
        if let Some(user) = &self.user {
            if !user.is_empty() {
                arguments.extend(["-l".to_string(), user.clone()]);
            }
        }
        if let Some(port) = self.port {
            arguments.extend(["-p".to_string(), port.to_string()]);
        }
        if let Some(identity) = &self.identity_file {
            if !identity.is_empty() {
                arguments.extend([
                    "-i".to_string(),
                    expand_tilde(identity),
                    "-o".to_string(),
                    "IdentitiesOnly=yes".to_string(),
                ]);
            }
        }
        arguments
    }
}

/// `NSString.expandingTildeInPath`: `~`/`~/x` resolve to the user's home,
/// `~name` to that user's home (unchanged when the name is unknown).
fn expand_tilde(path: &str) -> String {
    if path == "~" {
        return home_dir().to_string_lossy().into_owned();
    }
    if let Some(rest) = path.strip_prefix("~/") {
        return home_dir().join(rest).to_string_lossy().into_owned();
    }
    if path.starts_with('~') {
        let (name, rest) = match path[1..].find('/') {
            Some(slash) => (&path[1..1 + slash], &path[1 + slash..]),
            None => (&path[1..], ""),
        };
        if !name.is_empty() {
            if let Ok(name_c) = std::ffi::CString::new(name) {
                let pw = unsafe { libc::getpwnam(name_c.as_ptr()) };
                if !pw.is_null() {
                    let dir = unsafe { std::ffi::CStr::from_ptr((*pw).pw_dir) }
                        .to_string_lossy()
                        .into_owned();
                    return format!("{dir}{rest}");
                }
            }
        }
    }
    path.to_string()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SshError {
    /// ssh itself failed (exit 255): unreachable, auth, host key…
    Connection(String),
    /// The remote command failed.
    Remote { status: i32, message: String },
    InvalidConnection(String),
    Cancelled,
}

impl std::fmt::Display for SshError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Connection(message) => write!(f, "Could not connect over SSH: {message}"),
            Self::Remote { status, message } => {
                if message.is_empty() {
                    write!(f, "The remote command failed (exit {status}).")
                } else {
                    f.write_str(message)
                }
            }
            Self::InvalidConnection(message) => f.write_str(message),
            Self::Cancelled => f.write_str("Cancelled."),
        }
    }
}
impl std::error::Error for SshError {}

pub struct SshCommandResult {
    pub status: i32,
    pub standard_output: Vec<u8>,
    pub standard_error: Vec<u8>,
}

impl SshCommandResult {
    pub fn stdout_text(&self) -> String {
        String::from_utf8_lossy(&self.standard_output).into_owned()
    }
    pub fn stderr_text(&self) -> String {
        String::from_utf8_lossy(&self.standard_error).into_owned()
    }
}

/// Writing to a pipe whose reader exited raises SIGPIPE, which would kill
/// the app; ignore it so the write fails with EPIPE instead.
fn ignore_sigpipe() {
    static ONCE: OnceLock<()> = OnceLock::new();
    ONCE.get_or_init(|| unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_IGN);
    });
}

/// `ProcessPipe.run`: input, stdout and stderr are pumped on separate
/// threads so a large transfer can never deadlock on a full pipe.
fn run_process(
    executable: &Path,
    arguments: &[String],
    input: Option<&[u8]>,
    output_file: Option<&Path>,
    current_directory: Option<&Path>,
    environment: &[(&str, &str)],
) -> Result<SshCommandResult, SshError> {
    ignore_sigpipe();
    let mut command = Command::new(executable);
    command.args(arguments);
    if let Some(directory) = current_directory {
        command.current_dir(directory);
    }
    for (key, value) in environment {
        command.env(key, value);
    }
    command.stdin(if input.is_some() { Stdio::piped() } else { Stdio::null() });
    let mut output_handle = None;
    if let Some(file) = output_file {
        let handle = std::fs::File::create(file)
            .map_err(|_| SshError::Remote {
                status: -1,
                message: format!("Cannot write {}.", file.display()),
            })?;
        command.stdout(Stdio::from(handle.try_clone().map_err(|_| SshError::Remote {
            status: -1,
            message: format!("Cannot write {}.", file.display()),
        })?));
        output_handle = Some(handle);
    } else {
        command.stdout(Stdio::piped());
    }
    command.stderr(Stdio::piped());
    let mut child = command.spawn().map_err(|_| {
        SshError::Connection(format!("{} could not be started.", executable.display()))
    })?;
    let stdin = child.stdin.take();
    let input_writer = stdin.map(|mut pipe| {
        let bytes = input.map(|b| b.to_vec());
        std::thread::spawn(move || {
            use std::io::Write;
            if let Some(bytes) = bytes {
                let _ = pipe.write_all(&bytes);
            }
            // Dropping the pipe closes stdin.
        })
    });
    let stdout_reader = child.stdout.take().map(|mut pipe| {
        std::thread::spawn(move || {
            let mut data = Vec::new();
            let _ = pipe.read_to_end(&mut data);
            data
        })
    });
    let stderr_reader = child.stderr.take().map(|mut pipe| {
        std::thread::spawn(move || {
            let mut data = Vec::new();
            let _ = pipe.read_to_end(&mut data);
            data
        })
    });
    let status = child
        .wait()
        .map(|s| {
            s.code().unwrap_or_else(|| {
                use std::os::unix::process::ExitStatusExt;
                s.signal().map(|signal| 128 + signal).unwrap_or(1)
            })
        })
        .unwrap_or(1);
    if let Some(writer) = input_writer {
        let _ = writer.join();
    }
    let output = stdout_reader.map(|r| r.join().unwrap_or_default()).unwrap_or_default();
    let error = stderr_reader.map(|r| r.join().unwrap_or_default()).unwrap_or_default();
    drop(output_handle);
    Ok(SshCommandResult {
        status,
        standard_output: output,
        standard_error: error,
    })
}

/// Cancellation for `SshClient::stream` (builds): cancelling kills the ssh
/// process — with `-tt` the remote command then gets SIGHUP.
pub struct CancelScope {
    cancelled: AtomicBool,
    child: Mutex<Option<Arc<Mutex<Child>>>>,
}

impl CancelScope {
    pub fn new() -> Self {
        Self {
            cancelled: AtomicBool::new(false),
            child: Mutex::new(None),
        }
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
        if let Some(child) = lock(&self.child).as_ref() {
            if let Ok(mut process) = child.lock() {
                let _ = process.kill();
            }
        }
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
}

impl Default for CancelScope {
    fn default() -> Self {
        Self::new()
    }
}

/// Runs scripts on one connection through the system `ssh`. Every call is
/// non-interactive (`BatchMode`: key/agent auth only, never a password or
/// host-key prompt that would hang the app), keeps strict host key
/// checking, and reuses one multiplexed connection per host.
///
/// Scripts always run as `/bin/sh -c SCRIPT pitex ARGS…` on the remote:
/// paths travel as positional parameters, never spliced into shell text,
/// whatever the user's login shell is.
#[derive(Debug, Clone)]
pub struct SshClient {
    pub connection: SshConnection,
    pub ssh_executable: PathBuf,
    /// Directory for ControlMaster sockets (None disables multiplexing).
    pub control_directory: Option<PathBuf>,
    /// Extra `-o`/`-i` arguments (tests point at a private sshd).
    pub extra_arguments: Vec<String>,
}

impl SshClient {
    pub fn new(connection: SshConnection) -> Self {
        Self {
            connection,
            ssh_executable: PathBuf::from("/usr/bin/ssh"),
            control_directory: Self::default_control_directory(),
            extra_arguments: Vec::new(),
        }
    }

    /// `$XDG_RUNTIME_DIR/pitex-ssh`, or `/tmp/pitex-ssh-<uid>` when it is
    /// unset (`TMPDIR`/`std::env::temp_dir` can be long, and a socket path
    /// is the directory + `/` + the 40-hex `%C` + ssh's 17-character
    /// temporary suffix under a 104-byte cap).
    pub fn default_control_directory() -> Option<PathBuf> {
        if let Ok(runtime) = std::env::var("XDG_RUNTIME_DIR") {
            if !runtime.is_empty() {
                return Some(PathBuf::from(runtime).join("pitex-ssh"));
            }
        }
        Some(PathBuf::from(format!("/tmp/pitex-ssh-{}", unsafe {
            libc::getuid()
        })))
    }

    /// `directory` when multiplexing through it is safe: short enough for
    /// the socket path, and a real directory owned by this user with no
    /// group or other access — in shared /tmp anything else could let
    /// another user plant a control socket. Created on first use; None
    /// means plain connections.
    pub fn usable_control_directory(directory: &Path) -> Option<PathBuf> {
        use std::os::unix::ffi::OsStrExt;
        use std::os::unix::fs::MetadataExt;
        if directory.as_os_str().as_bytes().len() + 1 + 40 + 17 >= 104 {
            return None;
        }
        if std::fs::symlink_metadata(directory).is_err() {
            use std::os::unix::fs::DirBuilderExt;
            // No intermediate directories: a planted parent fails the checks.
            if std::fs::DirBuilder::new()
                .mode(0o700)
                .create(directory)
                .is_err()
            {
                return None;
            }
        }
        // symlink_metadata describes a symlink itself, not its target.
        let metadata = std::fs::symlink_metadata(directory).ok()?;
        if !metadata.is_dir() || metadata.uid() != unsafe { libc::getuid() } {
            return None;
        }
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o077 != 0 {
            return None;
        }
        Some(directory.to_path_buf())
    }

    /// Full ssh argv for `script` with positional `arguments`.
    pub fn arguments(
        &self,
        script: &str,
        arguments: &[String],
        tty: bool,
    ) -> Result<Vec<String>, SshError> {
        if let Some(problem) = self.connection.validation_error() {
            return Err(SshError::InvalidConnection(problem.to_string()));
        }
        let mut argv = vec![
            "-o".to_string(),
            "BatchMode=yes".to_string(),
            "-o".to_string(),
            "ConnectTimeout=15".to_string(),
            "-o".to_string(),
            "ServerAliveInterval=15".to_string(),
        ];
        if let Some(control_directory) = &self.control_directory {
            if let Some(directory) = Self::usable_control_directory(control_directory) {
                argv.extend([
                    "-o".to_string(),
                    "ControlMaster=auto".to_string(),
                    "-o".to_string(),
                    format!("ControlPath={}/%C", directory.display()),
                    "-o".to_string(),
                    "ControlPersist=300".to_string(),
                ]);
            }
        }
        argv.push(if tty { "-tt" } else { "-T" }.to_string());
        argv.extend(self.extra_arguments.iter().cloned());
        argv.extend(self.connection.connection_arguments());
        // The remote login shell parses this one string; everything inside
        // is single-quoted, so only /bin/sh interprets the script.
        let remote = ["/bin/sh".to_string(), "-c".to_string(), script.to_string(), "pitex".to_string()]
            .into_iter()
            .chain(arguments.iter().cloned())
            .map(|word| Self::quote(&word))
            .collect::<Vec<_>>()
            .join(" ");
        argv.extend(["--".to_string(), self.connection.destination.clone(), remote]);
        Ok(argv)
    }

    /// POSIX single-quoting: `'` → `'\''`.
    pub fn quote(value: &str) -> String {
        format!("'{}'", value.replace('\'', "'\\''"))
    }

    /// Runs `script` to completion; `input` is written to its stdin, and
    /// stdout goes to `output_file` instead of memory when given (tar
    /// streams can be large).
    pub fn run(
        &self,
        script: &str,
        arguments: &[String],
        input: Option<&[u8]>,
        output_file: Option<&Path>,
    ) -> Result<SshCommandResult, SshError> {
        let argv = self.arguments(script, arguments, false)?;
        let result = run_process(&self.ssh_executable, &argv, input, output_file, None, &[])?;
        if result.status == 255 {
            return Err(SshError::Connection(
                Self::first_line(&result.stderr_text())
                    .unwrap_or_else(|| "ssh exited with status 255".to_string()),
            ));
        }
        Ok(result)
    }

    /// Like `run`, but a non-zero exit is an error carrying stderr.
    pub fn run_checked(
        &self,
        script: &str,
        arguments: &[String],
        input: Option<&[u8]>,
        output_file: Option<&Path>,
    ) -> Result<SshCommandResult, SshError> {
        let result = self.run(script, arguments, input, output_file)?;
        if result.status != 0 {
            return Err(SshError::Remote {
                status: result.status,
                message: Self::first_line(&result.stderr_text()).unwrap_or_default(),
            });
        }
        Ok(result)
    }

    /// Streams stdout/stderr chunks as they arrive (builds). With `tty`,
    /// the remote runs under a pseudo-terminal so it gets SIGHUP when the
    /// local ssh is terminated — cancelling a build stops it remotely too.
    pub fn stream(
        &self,
        script: &str,
        arguments: &[String],
        tty: bool,
        cancel: &CancelScope,
        output: &mut dyn FnMut(&[u8], bool),
    ) -> Result<i32, SshError> {
        ignore_sigpipe();
        let argv = self.arguments(script, arguments, tty)?;
        let mut command = Command::new(&self.ssh_executable);
        command
            .args(&argv)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let child = {
            let mut slot = lock(&cancel.child);
            if cancel.is_cancelled() {
                return Err(SshError::Cancelled);
            }
            let spawned = command.spawn().map_err(|_| {
                SshError::Connection(format!("{} could not be started.", self.ssh_executable.display()))
            })?;
            *slot = Some(Arc::new(Mutex::new(spawned)));
            slot.as_ref().expect("stored").clone()
        };
        let (tx, rx) = std::sync::mpsc::channel::<(Vec<u8>, bool)>();
        let mut readers = Vec::new();
        {
            let mut owned = lock(&child);
            let mut pipes: Vec<(Box<dyn Read + Send>, bool)> = Vec::new();
            if let Some(pipe) = owned.stdout.take() {
                pipes.push((Box::new(pipe), false));
            }
            if let Some(pipe) = owned.stderr.take() {
                pipes.push((Box::new(pipe), true));
            }
            for (mut pipe, is_error) in pipes {
                let tx = tx.clone();
                readers.push(std::thread::spawn(move || {
                    let mut buffer = [0u8; 8192];
                    loop {
                        match pipe.read(&mut buffer) {
                            Ok(0) | Err(_) => break,
                            Ok(count) => {
                                if tx.send((buffer[..count].to_vec(), is_error)).is_err() {
                                    break;
                                }
                            }
                        }
                    }
                }));
            }
        }
        drop(tx);
        for (data, is_error) in rx {
            output(&data, is_error);
        }
        for reader in readers {
            let _ = reader.join();
        }
        let status = lock(&child)
            .wait()
            .map(|s| {
                s.code().unwrap_or_else(|| {
                    use std::os::unix::process::ExitStatusExt;
                    s.signal().map(|signal| 128 + signal).unwrap_or(1)
                })
            })
            .unwrap_or(1);
        *lock(&cancel.child) = None;
        if cancel.is_cancelled() {
            return Err(SshError::Cancelled);
        }
        Ok(status)
    }

    /// Verifies the connection end to end; returns the remote home path.
    pub fn check(&self) -> Result<String, SshError> {
        let result = self.run_checked("cd && pwd -P", &[], None, None)?;
        Ok(result.stdout_text().trim().to_string())
    }

    /// Lists `path` on the remote; empty or `~` means the home directory,
    /// `~/x` is relative to it.
    pub fn list_directory(&self, path: &str) -> Result<RemoteDirectoryListing, SshError> {
        let mut target = path.trim().to_string();
        if target == "~" {
            target.clear();
        } else if let Some(rest) = target.strip_prefix("~/") {
            target = rest.to_string();
        }
        let result = self.run(&remote_scripts::list_directory(), &[target], None, None)?;
        if result.status != 0 {
            return Err(SshError::Remote {
                status: result.status,
                message: format!(
                    "{path} is not a readable folder on {}.",
                    self.connection.name
                ),
            });
        }
        RemoteDirectoryListing::parse(&result.stdout_text()).ok_or_else(|| SshError::Remote {
            status: result.status,
            message: format!("{path} is not a readable folder on {}.", self.connection.name),
        })
    }

    /// The login-shell path of `tool` on the remote, or None.
    pub fn which(&self, tool: &str) -> Result<Option<String>, SshError> {
        let result = self.run(
            &remote_scripts::login_exec(),
            &[
                ".".to_string(),
                "/bin/sh".to_string(),
                "-c".to_string(),
                r#"command -v "$1""#.to_string(),
                "sh".to_string(),
                tool.to_string(),
            ],
            None,
            None,
        )?;
        let path = result.stdout_text().trim().to_string();
        if result.status == 0 && !path.is_empty() {
            Ok(path.lines().last().map(|line| line.to_string()))
        } else {
            Ok(None)
        }
    }

    fn first_line(text: &str) -> Option<String> {
        split_newlines(text)
            .into_iter()
            .map(|line| line.trim().to_string())
            .find(|line| !line.is_empty())
    }
}

// ─── RemoteSync.swift ────────────────────────────────────────────────────────

/// A folder on a remote device, as opened through "Open via SSH".
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RemoteProject {
    pub connection: SshConnection,
    /// Absolute, symlink-resolved remote path (`pwd -P`).
    #[serde(rename = "remoteRoot")]
    pub remote_root: String,
}

impl RemoteProject {
    pub fn new(connection: SshConnection, remote_root: impl Into<String>) -> Self {
        Self {
            connection,
            remote_root: remote_root.into(),
        }
    }

    pub fn folder_name(&self) -> String {
        let name = last_path_component(&self.remote_root);
        if name.is_empty() || name == "/" {
            "root".to_string()
        } else {
            name
        }
    }
}

/// `NSString.lastPathComponent`: trailing slashes are stripped first;
/// "/" yields "/", "" yields "".
fn last_path_component(path: &str) -> String {
    let trimmed = path.trim_end_matches('/');
    if trimmed.is_empty() {
        return if path.starts_with('/') {
            "/".to_string()
        } else {
            String::new()
        };
    }
    trimmed.rsplit('/').next().unwrap_or_default().to_string()
}

/// The local working copy of a remote folder. Pitex edits, previews and
/// indexes this directory like any local project; `RemoteSync` keeps it
/// and the remote folder in step. Layout under the store — the project
/// sits alone under `project/`, so no folder name can collide with the
/// bookkeeping beside it:
///
/// ```text
/// <store>/<slug>/remote.json              — which device/folder this mirrors
/// <store>/<slug>/manifest.json            — content hashes at the last sync
/// <store>/<slug>/work/                    — transfer staging
/// <store>/<slug>/project/<folder name>/   — the project root the editor opens
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RemoteMirror {
    pub directory: PathBuf,
    pub project: RemoteProject,
}

impl RemoteMirror {
    pub fn root(&self) -> PathBuf {
        self.directory.join("project").join(self.project.folder_name())
    }
    pub fn metadata_path(&self) -> PathBuf {
        self.directory.join("remote.json")
    }
    pub fn manifest_path(&self) -> PathBuf {
        self.directory.join("manifest.json")
    }
    pub fn work_dir(&self) -> PathBuf {
        self.directory.join("work")
    }
    pub fn staging_dir(&self) -> PathBuf {
        self.work_dir().join("staging")
    }

    /// `~/.local/share/pitex/Remote`.
    pub fn default_store() -> PathBuf {
        dirs::data_dir()
            .unwrap_or_else(|| home_dir().join(".local/share"))
            .join("pitex")
            .join("Remote")
    }

    /// The mirror for `project`, created (with its metadata) on first use.
    pub fn prepare(project: RemoteProject, store: &Path) -> Result<RemoteMirror, SshError> {
        let key = [
            project.connection.destination.clone(),
            project.connection.user.clone().unwrap_or_default(),
            (project.connection.port.unwrap_or(22)).to_string(),
            project.remote_root.clone(),
        ]
        .join("\u{0}");
        let digest = sha256_hex(key.as_bytes())[..12].to_string();
        let slug = format!(
            "{}-{}-{digest}",
            Self::sanitized(&project.connection.name),
            Self::sanitized(&project.folder_name())
        );
        let mirror = RemoteMirror {
            directory: store.join(slug),
            project,
        };
        std::fs::create_dir_all(mirror.root()).map_err(|e| SshError::Remote {
            status: -1,
            message: e.to_string(),
        })?;
        write_atomic(&mirror.metadata_path(), &serde_json::to_vec(&mirror.project).map_err(
            |e| SshError::Remote {
                status: -1,
                message: e.to_string(),
            },
        )?)?;
        Ok(mirror)
    }

    /// The mirror containing `url`, if `url` lies inside one — how a recent
    /// or re-opened path is recognized as a remote project.
    pub fn containing(url: &Path, store: &Path) -> Option<RemoteMirror> {
        let store_path = resolved(store);
        let path = resolved(url);
        let rest = path.strip_prefix(&store_path).ok()?;
        if rest.as_os_str().is_empty() {
            return None;
        }
        let slug = rest.components().next()?;
        let directory = store_path.join(slug.as_os_str());
        let data = std::fs::read(directory.join("remote.json")).ok()?;
        let project: RemoteProject = serde_json::from_slice(&data).ok()?;
        let mirror = RemoteMirror {
            directory,
            project,
        };
        let root = mirror.root();
        if path != root && !path.starts_with(&root) {
            return None;
        }
        Some(mirror)
    }

    fn sanitized(text: &str) -> String {
        let mapped: String = text
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '.' || c == '_' || c == '-' {
                    c
                } else {
                    '-'
                }
            })
            .collect();
        mapped.chars().take(40).collect()
    }
}

/// `.atomic` writes: a sibling temp file renamed over the destination.
fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), SshError> {
    let temporary = path.with_extension("tmp");
    let written = std::fs::write(&temporary, bytes).and_then(|()| std::fs::rename(&temporary, path));
    if let Err(error) = written {
        let _ = std::fs::remove_file(&temporary);
        return Err(SshError::Remote {
            status: -1,
            message: error.to_string(),
        });
    }
    Ok(())
}

/// `standardizedFileURL`: `..`/`.` resolved textually (no filesystem touch).
fn standardize(path: &Path) -> PathBuf {
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

/// `resolvingSymlinksInPath().standardizedFileURL`: canonicalize what
/// exists, append and standardize whatever does not.
fn resolved(path: &Path) -> PathBuf {
    if let Ok(canonical) = std::fs::canonicalize(path) {
        return canonical;
    }
    let mut prefix = path.to_path_buf();
    let mut rest: Vec<std::ffi::OsString> = Vec::new();
    loop {
        if let Ok(canonical) = std::fs::canonicalize(&prefix) {
            let mut out = canonical;
            for component in rest.iter().rev() {
                out.push(component);
            }
            return standardize(&out);
        }
        match prefix.file_name() {
            Some(name) => {
                rest.push(name.to_os_string());
                prefix.pop();
            }
            None => return standardize(path),
        }
    }
}

/// Which files are mirrored, applied identically on both sides: VCS and
/// dependency trees, macOS litter, Pitex's own atomic-save temp files and
/// anything ≥ 50 MB stay out.
pub enum RemoteSyncRules {}

impl RemoteSyncRules {
    pub const MAXIMUM_FILE_SIZE: u64 = 50 * 1024 * 1024;

    /// Sorted once for the find(1) prune expression (Swift sorts a Set).
    pub fn pruned_names() -> &'static [&'static str] {
        &[
            ".DS_Store",
            ".Trash",
            ".git",
            ".hg",
            ".svn",
            ".venv",
            "__pycache__",
            "node_modules",
            "venv",
        ]
    }

    pub fn is_excluded(name: &str) -> bool {
        Self::pruned_names().contains(&name)
            || name.starts_with(".pitex-upload.")
            || (name.starts_with('.') && name.contains(".texspark-") && name.ends_with(".tmp"))
    }

    /// A relative path from the remote is only ever used inside the mirror:
    /// no absolute paths, `..`, empty or control-character components.
    pub fn is_safe_relative_path(path: &str) -> bool {
        if path.is_empty() || path.starts_with('/') {
            return false;
        }
        path.split('/').all(|component| {
            !component.is_empty()
                && component != "."
                && component != ".."
                && !component.chars().any(|c| c.is_control())
        })
    }
}

/// Pure pull decisions over three hash maps (path → SHA-256): the remote
/// folder, the manifest (both sides at the last sync) and the local mirror.
/// Push needs no plan here: the device decides per file at the moment of
/// replacement (`RemoteScripts.commitUpload`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PullPlan {
    /// Remote changed, local untouched → download.
    pub download: Vec<String>,
    /// Both sides already equal → record in the manifest only.
    pub adopt: HashMap<String, String>,
    /// Missing from the remote listing, local untouched → delete locally
    /// once the device confirms the file is really gone.
    pub delete: Vec<String>,
    /// Changed on both sides → keep local, report.
    pub conflicts: Vec<String>,
}

pub enum SyncPlanner {}

impl SyncPlanner {
    pub fn pull(
        remote: &HashMap<String, String>,
        manifest: &HashMap<String, String>,
        local: &HashMap<String, String>,
    ) -> PullPlan {
        let mut plan = PullPlan::default();
        for (path, remote_hash) in remote {
            let base = manifest.get(path);
            let mine = local.get(path);
            if Some(remote_hash) == base {
                continue;
            }
            if mine == Some(remote_hash) {
                plan.adopt.insert(path.clone(), remote_hash.clone());
            } else if mine == base {
                // Untouched locally (or absent both locally and in the
                // manifest: a brand-new remote file).
                plan.download.push(path.clone());
            } else {
                plan.conflicts.push(path.clone());
            }
        }
        for (path, base) in manifest {
            if remote.contains_key(path) {
                continue;
            }
            match local.get(path) {
                None => plan.delete.push(path.clone()),
                Some(mine) if mine == base => plan.delete.push(path.clone()),
                _ => plan.conflicts.push(path.clone()),
            }
        }
        plan.download.sort();
        plan.delete.sort();
        plan.conflicts.sort();
        plan
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SyncReport {
    pub downloaded: Vec<String>,
    pub uploaded: Vec<String>,
    pub deleted: Vec<String>,
    /// Every path of this mirror currently changed on both sides (not just
    /// those found by this operation).
    pub conflicts: Vec<String>,
}

impl SyncReport {
    pub fn changed_locally(&self) -> bool {
        !self.downloaded.is_empty() || !self.deleted.is_empty()
    }
}

/// What the device holds at a path, as `RemoteScripts.probe` reports it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemoteFileState {
    File(String),
    /// Confirmed absent: the nearest existing ancestor is a searchable
    /// directory.
    Absent,
    /// Exists but cannot be hashed (unreadable, a directory, a symlink) or
    /// its absence cannot be confirmed.
    Unavailable,
}

/// One remote directory listing for the folder browser.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteDirectoryListing {
    pub path: String,
    pub folders: Vec<String>,
    pub files: Vec<String>,
}

/// Finder-ish ordering for the browser: case-insensitive first pass with a
/// numeric run comparison, so `file2` precedes `file10`.
fn localized_standard_compare(a: &str, b: &str) -> std::cmp::Ordering {
    let mut ac = a.chars().peekable();
    let mut bc = b.chars().peekable();
    loop {
        match (ac.peek().copied(), bc.peek().copied()) {
            (None, None) => return std::cmp::Ordering::Equal,
            (None, Some(_)) => return std::cmp::Ordering::Less,
            (Some(_), None) => return std::cmp::Ordering::Greater,
            (Some(x), Some(y)) => {
                if x.is_ascii_digit() && y.is_ascii_digit() {
                    let take_digits = |it: &mut std::iter::Peekable<std::str::Chars>| -> u64 {
                        let mut value: u64 = 0;
                        while let Some(d) = it.peek().and_then(|c| c.to_digit(10)) {
                            it.next();
                            value = value.saturating_mul(10).saturating_add(d as u64);
                        }
                        value
                    };
                    let order = take_digits(&mut ac).cmp(&take_digits(&mut bc));
                    if order != std::cmp::Ordering::Equal {
                        return order;
                    }
                } else {
                    let order = x.to_lowercase().cmp(y.to_lowercase());
                    if order != std::cmp::Ordering::Equal {
                        return order;
                    }
                    ac.next();
                    bc.next();
                }
            }
        }
    }
}

impl RemoteDirectoryListing {
    pub fn parse(output: &str) -> Option<RemoteDirectoryListing> {
        let mut lines = output.split('\n');
        let path = lines.next()?;
        if !path.starts_with('/') {
            return None;
        }
        let mut folders = Vec::new();
        let mut files = Vec::new();
        for line in lines {
            if line.chars().count() <= 2 {
                continue;
            }
            if let Some(name) = line.strip_prefix("D/") {
                folders.push(name.to_string());
            } else if let Some(name) = line.strip_prefix("F/") {
                files.push(name.to_string());
            }
        }
        folders.sort_by(|a, b| localized_standard_compare(a, b));
        files.sort_by(|a, b| localized_standard_compare(a, b));
        Some(RemoteDirectoryListing {
            path: path.to_string(),
            folders,
            files,
        })
    }
}

/// The POSIX sh scripts run on the remote. Single-line so a csh/tcsh login
/// shell can pass them through; every path arrives as a positional
/// parameter (`$1`…) or on stdin, never as script text. Byte-identical to
/// Swift `RemoteScripts` — the golden fixtures under
/// `Fixtures/expected/remote-scripts/` enforce that on both platforms.
pub mod remote_scripts {
    use super::{RemoteSyncRules, SshClient};
    use std::collections::HashMap;

    const HASH_COMMAND: &str =
        "if command -v sha256sum >/dev/null 2>&1; then h=sha256sum; else h='shasum -a 256'; fi";

    fn prune() -> String {
        let mut names: Vec<String> = RemoteSyncRules::pruned_names()
            .iter()
            .map(|name| format!("-name {}", SshClient::quote(name)))
            .collect();
        names.push("-name '.*.texspark-*.tmp'".to_string());
        names.push("-name '.pitex-upload.*'".to_string());
        format!("\\( {} \\) -prune", names.join(" -o "))
    }

    /// `a PATH` checks PATH's parent components top-down: 0 = all real,
    /// searchable directories; 2 = one is missing below real ones (so PATH
    /// is absent); 1 = a symlink, non-directory or unsearchable directory
    /// is in the way — nothing about PATH can be concluded or written.
    const ANCESTRY: &str = r#"a() { q=.; z=$1; while :; do case $z in */*) q=$q/${z%%/*}; z=${z#*/} ;; *) return 0 ;; esac; if [ -L "$q" ]; then return 1; elif [ -d "$q" ]; then [ -x "$q" ] || return 1; elif [ -e "$q" ]; then return 1; else return 2; fi; done; }"#;

    /// `hash  ./path` for every mirrored file under $1. A file missing here
    /// is only a deletion candidate (`probe` decides).
    pub fn hash_tree() -> String {
        format!(
            "cd -- \"$1\" || exit 3; {HASH_COMMAND}; find . {} -o -type f -size -{}c -print0 | xargs -0 $h",
            prune(),
            RemoteSyncRules::MAXIMUM_FILE_SIZE
        )
    }

    /// For each newline-separated relative path on stdin: `H<hash>/path`,
    /// `A/path` (confirmed absent) or `E/path` (not a plain file below real
    /// directories, unreadable, or absence not confirmable).
    pub fn probe() -> String {
        format!("cd -- \"$1\" || exit 3; {HASH_COMMAND}; {ANCESTRY}; while IFS= read -r p; do a \"$p\"; v=$?; if [ $v = 2 ]; then printf 'A/%s\\n' \"$p\"; elif [ $v = 1 ] || [ -L \"$p\" ] || [ -d \"$p\" ]; then printf 'E/%s\\n' \"$p\"; elif [ -e \"$p\" ]; then if c=$($h < \"$p\"); then printf 'H%s/%s\\n' \"${{c%% *}}\" \"$p\"; else printf 'E/%s\\n' \"$p\"; fi; else printf 'A/%s\\n' \"$p\"; fi; done")
    }

    /// Commits an upload: the archive on stdin holds `expect` (lines of
    /// `BASE NEW path`, `-` = absent) and `data/<path>`. Prints per path
    /// `U/` (placed), `S/` (already NEW), `C/` (conflict), `E/` (not
    /// placeable) or `L/` (a hard link was refused in a writable folder —
    /// no hard links there, or no space/quota — so it cannot be done
    /// safely). The live path is never missing or partially written:
    ///
    /// - the archive is unpacked into a private directory first, so nothing
    ///   changes until it arrived complete;
    /// - a lock directory (owner PID inside; stale once that process is
    ///   gone) serializes Pitex clients;
    /// - `a` must find only real directories above the path;
    /// - an existing file gets a backup hard link and is hashed; it is
    ///   replaced by one rename(2), only if it is still BASE and still that
    ///   same file. A write into the old file during that instant survives
    ///   as `<path>.pitex-conflict-<pid>`;
    /// - a new file is published complete by link(2), which never replaces
    ///   anything that appeared meanwhile (a directory that appeared is
    ///   caught by the `-ef` check and our stray link removed).
    ///
    /// ponytail: a writer that renames its own file over the path between
    /// the `-ef` check and our rename (microseconds) is still replaced —
    /// closing that needs renameat2/renamex_np, which sh cannot reach.
    pub fn commit_upload() -> String {
        let setup = [
            r#"cd -- "$1" || exit 3"#.to_string(),
            HASH_COMMAND.to_string(),
            ANCESTRY.to_string(),
            r#"t=$(mktemp -d ./.pitex-upload.XXXXXX) || exit 4"#.to_string(),
            "w=; o=".to_string(),
            r#"trap 'rm -f -- ${w:+"$w"} ${o:+"$o"}; rm -rf "$t"; [ "$(cat ./.pitex-upload.lock/pid 2>/dev/null)" = "$$" ] && rm -rf ./.pitex-upload.lock' EXIT"#.to_string(),
            "trap 'exit 1' HUP INT TERM".to_string(),
            r#"tar -xf - -C "$t" || exit 5"#.to_string(),
            r#"[ -f "$t/expect" ] || exit 5"#.to_string(),
            r#"k=0; until mkdir ./.pitex-upload.lock 2>/dev/null; do x=$(cat ./.pitex-upload.lock/pid 2>/dev/null); if [ -n "$x" ]; then ps -p "$x" >/dev/null 2>&1 || { rm -rf ./.pitex-upload.lock; continue; }; fi; k=$((k+1)); [ $k -lt 60 ] || exit 6; sleep 1; done"#.to_string(),
            r#"echo $$ > ./.pitex-upload.lock/pid"#.to_string(),
        ];
        let per_file = [
            r#"b=${l%% *}; r=${l#* }; n=${r%% *}; p=${r#* }"#.to_string(),
            r#"a "$p"; v=$?"#.to_string(),
            r#"if [ $v = 1 ]; then printf 'E/%s\n' "$p"; continue; fi"#.to_string(),
            r#"if [ -L "$p" ] || [ -d "$p" ]; then printf 'C/%s\n' "$p"; continue; fi"#.to_string(),
            r#"case $p in */*) d=${p%/*} ;; *) d=. ;; esac"#.to_string(),
            r#"if [ $v = 2 ]; then mkdir -p -- "$d" || { printf 'E/%s\n' "$p"; continue; }; fi"#.to_string(),
            // Beside the target, so the final rename stays on one filesystem.
            r#"w=$d/.pitex-upload.new.$$; o=$d/.pitex-upload.old.$$"#.to_string(),
            r#"mv -f -- "$t/data/$p" "$w" || { w=; printf 'E/%s\n' "$p"; continue; }"#.to_string(),
            // `f`: why a link could not be made — no write access (E), or
            // refused in a writable folder (L: no hard links, space, quota).
            r#"f() { if [ -w "$d" ]; then printf 'L/%s\n' "$p"; else printf 'E/%s\n' "$p"; fi; }"#.to_string(),
            r#"if [ -e "$p" ]; then ln -f -- "$p" "$o" 2>/dev/null || { f; o=; rm -f -- "$w"; w=; continue; }; c=$($h < "$o") || c=x; c=${c%% *}; if [ "$c" = "$n" ]; then printf 'S/%s\n' "$p"; elif [ "$c" != "$b" ]; then printf 'C/%s\n' "$p"; elif [ ! "$p" -ef "$o" ]; then printf 'C/%s\n' "$p"; elif mv -f -- "$w" "$p"; then e=$($h < "$o") || e=x; if [ "${e%% *}" != "$c" ]; then mv -f -- "$o" "$p.pitex-conflict-$$"; o=; printf 'C/%s\n' "$p"; else printf 'U/%s\n' "$p"; fi; else printf 'E/%s\n' "$p"; fi"#.to_string(),
            r#"elif [ "$b" != - ]; then printf 'C/%s\n' "$p"; elif ln -- "$w" "$p" 2>/dev/null; then if [ "$p" -ef "$w" ]; then printf 'U/%s\n' "$p"; else rm -f -- "$p/${w##*/}"; printf 'C/%s\n' "$p"; fi; elif [ -e "$p" ] || [ -L "$p" ]; then printf 'C/%s\n' "$p"; else f; fi"#.to_string(),
            r#"rm -f -- ${w:+"$w"} ${o:+"$o"}; w=; o="#.to_string(),
        ];
        format!(
            "{}; while IFS= read -r l; do {}; done < \"$t/expect\"",
            setup.join("; "),
            per_file.join("; ")
        )
    }

    /// A tar of the NUL-separated relative paths on stdin that exist.
    pub fn tar_out() -> String {
        r#"cd -- "$1" || exit 3; tar -cf - --null -T -"#.to_string()
    }

    /// `pwd -P`, then `D/name` / `F/name` for each visible entry of $1
    /// (empty = home).
    pub fn list_directory() -> String {
        r#"if [ -n "$1" ]; then cd -- "$1" || exit 3; else cd || exit 3; fi; pwd -P; for f in *; do [ -e "$f" ] || continue; if [ -d "$f" ]; then printf 'D/%s\n' "$f"; else printf 'F/%s\n' "$f"; fi; done"#.to_string()
    }

    /// Runs "$2" "$3"… in $1 through the user's login shell, so PATH comes
    /// from their profile (TeX installs usually add themselves there), with
    /// the standard TeX locations appended as a fallback. The fallback is
    /// computed here in /bin/sh and handed over in a variable: the login
    /// shell may be zsh, which aborts on an unmatched glob.
    pub fn login_exec() -> String {
        r#"cd -- "$1" || exit 127; shift; t=/Library/TeX/texbin:/usr/texbin:/opt/homebrew/bin:/usr/local/bin; for d in /usr/local/texlive/*/bin/*; do [ -d "$d" ] && t="$t:$d"; done; PITEX_TEX_PATH=$t; export PITEX_TEX_PATH; PATH="$PATH:$t"; export PATH; case "${SHELL##*/}" in bash|zsh|sh|dash|ksh) s=$SHELL ;; *) s=/bin/sh ;; esac; exec "$s" -lc 'PATH="$PATH:$PITEX_TEX_PATH"; export PATH; exec "$@"' pitex "$@""#.to_string()
    }

    /// Parses `sha256sum`/`shasum` lines into relative path → hash. Lines
    /// for escaped names (leading `\`), stdin (`-`) and unsafe paths drop.
    pub fn parse_hashes(output: &str) -> HashMap<String, String> {
        let mut hashes = HashMap::new();
        for line in output.split('\n') {
            if line.chars().count() <= 66 || line.starts_with('\\') {
                continue;
            }
            let mut chars = line.chars();
            let hash: String = chars.by_ref().take(64).collect();
            if !hash.chars().all(|c| c.is_ascii_hexdigit()) {
                continue;
            }
            let mut path: String = chars.skip(2).collect();
            if let Some(stripped) = path.strip_prefix("./") {
                path = stripped.to_string();
            }
            // `-` is the hasher reading an empty stdin (xargs with no input).
            if path == "-" || !RemoteSyncRules::is_safe_relative_path(&path) {
                continue;
            }
            hashes.insert(path, hash.to_lowercase());
        }
        hashes
    }

    pub fn parse_probe(output: &str) -> HashMap<String, super::RemoteFileState> {
        use super::RemoteFileState;
        let mut states = HashMap::new();
        for line in output.split('\n') {
            if let Some(path) = line.strip_prefix("A/") {
                states.insert(path.to_string(), RemoteFileState::Absent);
            } else if let Some(path) = line.strip_prefix("E/") {
                states.insert(path.to_string(), RemoteFileState::Unavailable);
            } else if line.starts_with('H')
                && line.chars().count() > 66
                && line.chars().nth(65) == Some('/')
            {
                let hash: String = line.chars().skip(1).take(64).collect();
                if !hash.chars().all(|c| c.is_ascii_hexdigit()) {
                    continue;
                }
                let path: String = line.chars().skip(66).collect();
                states.insert(path, RemoteFileState::File(hash.to_lowercase()));
            }
        }
        states
    }
}

/// Lets the app hold its own editor saves back while sync replaces or
/// deletes mirror files, so a save can never land between sync's "is this
/// file still untouched?" check and the replacement.
pub trait MirrorWriteGate: Send + Sync {
    /// Returns once no app write is in flight; new ones wait for `end_sync_commit`.
    fn begin_sync_commit(&self);
    fn end_sync_commit(&self);
}

/// SHA-256 hex for hashing in the port — tex-domain's implementation is
/// the same one `git-core` and session baselines use.
pub fn sha256_hex(data: &[u8]) -> String {
    tex_domain::hex_lower(&tex_domain::sha256(data))
}

fn sha256_hex_file(path: &Path) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;
    Some(sha256_hex(&bytes))
}

/// Keeps one mirror and its remote folder in step. Pull brings remote
/// changes down without touching files edited locally since the last sync;
/// push uploads local edits only where the remote is still what was last
/// synced. Anything changed on both sides is reported, never overwritten.
///
/// Operations serialize on `inner`, held for a whole operation — the Rust
/// counterpart of the Swift actor's acquire/release — while `local_cache`
/// sits outside it so `resolve` can snapshot the local hash before
/// queueing behind another transfer.
pub struct RemoteSync {
    pub mirror: RemoteMirror,
    pub client: SshClient,
    gate: Option<Arc<dyn MirrorWriteGate>>,
    inner: Mutex<SyncInner>,
    local_cache: Mutex<HashMap<String, LocalCacheEntry>>,
}

struct SyncInner {
    manifest: HashMap<String, String>,
    /// Paths changed on both sides, kept here so every window sharing the
    /// engine sees one list, and entries leave it once the sides agree.
    conflicts: BTreeSet<String>,
}

#[derive(Clone)]
struct LocalCacheEntry {
    size: u64,
    modified: SystemTime,
    hash: String,
}

/// A staged download: `path` below the mirror's `work/staging` plus the
/// hash of the bytes actually received — what the manifest records.
struct StagedFile {
    url: PathBuf,
    hash: String,
}

enum UploadOutcome {
    /// The device holds these bytes now (`replaced`: by this upload).
    Stored(String, bool),
    Conflict,
    Failed,
    /// A hard link was refused (no support, space or quota): no safe replacement.
    Unsupported,
}

fn registry() -> &'static Mutex<HashMap<PathBuf, Arc<RemoteSync>>> {
    static REGISTRY: OnceLock<Mutex<HashMap<PathBuf, Arc<RemoteSync>>>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

impl RemoteSync {
    pub fn new(mirror: RemoteMirror, client: SshClient, gate: Option<Arc<dyn MirrorWriteGate>>) -> Self {
        let manifest = std::fs::read(mirror.manifest_path())
            .ok()
            .and_then(|data| serde_json::from_slice::<HashMap<String, String>>(&data).ok())
            .unwrap_or_default();
        Self {
            mirror,
            client,
            gate,
            inner: Mutex::new(SyncInner {
                manifest,
                conflicts: BTreeSet::new(),
            }),
            local_cache: Mutex::new(HashMap::new()),
        }
    }

    /// The one engine for `mirror` in this process: windows showing the
    /// same remote folder share its manifest, staging area and ordering.
    pub fn shared(mirror: RemoteMirror, gate: Option<Arc<dyn MirrorWriteGate>>) -> Arc<RemoteSync> {
        let key = standardize(&mirror.directory);
        let mut engines = lock(registry());
        if let Some(engine) = engines.get(&key) {
            return engine.clone();
        }
        let engine = Arc::new(RemoteSync::new(
            mirror.clone(),
            SshClient::new(mirror.project.connection.clone()),
            gate,
        ));
        engines.insert(key, engine.clone());
        engine
    }

    /// Test/embedder hook: pre-registers an engine built on `client` for
    /// `mirror` when none exists yet — the way a live test gives the
    /// engine a client carrying test-only `extra_arguments` before
    /// `RemoteSync::shared` would build the default one. Returns what the
    /// registry holds either way, so it never displaces a live engine.
    pub fn install(
        mirror: RemoteMirror,
        client: SshClient,
        gate: Option<Arc<dyn MirrorWriteGate>>,
    ) -> Arc<RemoteSync> {
        let key = standardize(&mirror.directory);
        let mut engines = lock(registry());
        if let Some(engine) = engines.get(&key) {
            return engine.clone();
        }
        let engine = Arc::new(RemoteSync::new(mirror, client, gate));
        engines.insert(key, engine.clone());
        engine
    }

    fn remote_root(&self) -> &str {
        &self.mirror.project.remote_root
    }
    fn device_name(&self) -> &str {
        &self.mirror.project.connection.name
    }

    /// Remote → local.
    pub fn pull(&self) -> Result<SyncReport, SshError> {
        let mut inner = lock(&self.inner);
        let listing = self.client.run_checked(
            &remote_scripts::hash_tree(),
            &[self.remote_root().to_string()],
            None,
            None,
        )?;
        let remote = remote_scripts::parse_hashes(&listing.stdout_text());
        let mut local: HashMap<String, String> = HashMap::new();
        let candidates: BTreeSet<String> = remote
            .keys()
            .chain(inner.manifest.keys())
            .chain(inner.conflicts.iter())
            .cloned()
            .collect();
        for path in candidates {
            if let Some(hash) = self.local_hash(&path) {
                local.insert(path, hash);
            }
        }
        let plan = SyncPlanner::pull(&remote, &inner.manifest, &local);
        let mut report = SyncReport::default();
        // Sides that agree again (even back on the old baseline) are settled.
        let settled: Vec<String> = inner
            .conflicts
            .iter()
            .filter(|path| local.get(*path) == remote.get(*path))
            .cloned()
            .collect();
        for path in settled {
            inner.conflicts.remove(&path);
        }
        inner.conflicts.extend(plan.conflicts.iter().cloned());
        for (path, hash) in &plan.adopt {
            inner.manifest.insert(path.clone(), hash.clone());
            inner.conflicts.remove(path);
        }
        let _staging_cleanup = RemoveOnDrop(self.mirror.staging_dir());
        let staged = if plan.download.is_empty() {
            HashMap::new()
        } else {
            self.download(&plan.download)?
        };
        // Missing from the listing is not proof of deletion (too large,
        // unreadable directory…): only a confirmed absence deletes.
        let mut gone: Vec<String> = if plan.delete.is_empty() {
            Vec::new()
        } else {
            self.probe(&plan.delete)?
                .iter()
                .filter(|(_, state)| **state == RemoteFileState::Absent)
                .map(|(path, _)| path.clone())
                .collect()
        };
        gone.sort();
        let mut failed: Vec<String> = Vec::new();
        self.commit_locally(&mut inner, |sync, inner| {
            for path in &plan.download {
                let Some(file) = staged.get(path) else { continue };
                // Re-check under the gate: an edit saved while the download
                // ran makes this a conflict instead of a loss.
                if sync.local_hash(path).as_deref() != local.get(path).map(String::as_str) {
                    inner.conflicts.insert(path.clone());
                    continue;
                }
                if sync.place(&file.url, path).is_err() {
                    failed.push(path.clone());
                    continue;
                }
                inner.manifest.insert(path.clone(), file.hash.clone());
                inner.conflicts.remove(path);
                report.downloaded.push(path.clone());
            }
            for path in &gone {
                let mut can_delete = true;
                if let Some(url) = sync.contained_url(path, false) {
                    if item_exists(&url) {
                        if sync.local_hash(path).as_deref() != inner.manifest.get(path).map(String::as_str) {
                            inner.conflicts.insert(path.clone());
                            can_delete = false;
                        } else if std::fs::remove_file(&url).is_err() {
                            failed.push(path.clone());
                            can_delete = false;
                        }
                    }
                }
                if can_delete {
                    inner.manifest.remove(path);
                    sync.drop_local_cache(path);
                    inner.conflicts.remove(path);
                    report.deleted.push(path.clone());
                }
            }
            sync.save_manifest(inner)
        })?;
        report.conflicts = inner.conflicts.iter().cloned().collect();
        if !failed.is_empty() {
            return Err(SshError::Remote {
                status: -1,
                message: format!("Could not update {} in the local copy.", failed.join(", ")),
            });
        }
        Ok(report)
    }

    /// Local → remote. Each changed file replaces the remote copy only if
    /// that is still what the manifest recorded, checked on the device at
    /// the moment of the move; otherwise it is a conflict.
    pub fn push(&self) -> Result<SyncReport, SshError> {
        let mut inner = lock(&self.inner);
        let changed: Vec<String> = {
            let local = self.local_hashes();
            let mut changed: Vec<String> = local
                .iter()
                .filter(|(path, hash)| inner.manifest.get(*path) != Some(*hash))
                .map(|(path, _)| path.clone())
                .collect();
            changed.sort();
            changed
        };
        let mut report = SyncReport::default();
        report.conflicts = inner.conflicts.iter().cloned().collect();
        if changed.is_empty() {
            return Ok(report);
        }
        let outcomes = self.upload(&changed, &inner.manifest)?;
        let mut failed: Vec<String> = Vec::new();
        let mut unsupported: Vec<String> = Vec::new();
        for path in &changed {
            match outcomes.get(path) {
                Some(UploadOutcome::Stored(hash, replaced)) => {
                    inner.manifest.insert(path.clone(), hash.clone());
                    inner.conflicts.remove(path);
                    if *replaced {
                        report.uploaded.push(path.clone());
                    }
                }
                Some(UploadOutcome::Conflict) => {
                    inner.conflicts.insert(path.clone());
                }
                Some(UploadOutcome::Failed) => failed.push(path.clone()),
                Some(UploadOutcome::Unsupported) => unsupported.push(path.clone()),
                None => {
                    // Vanished or became unreadable since the scan.
                    continue;
                }
            }
        }
        report.conflicts = inner.conflicts.iter().cloned().collect();
        self.save_manifest(&mut inner)?;
        if !unsupported.is_empty() {
            return Err(SshError::Remote {
                status: -1,
                message: format!("Could not upload {} to {}: its folder refused a hard link (a filesystem without them, or out of space or quota), and Pitex only replaces files that way.", unsupported.join(", "), self.device_name()),
            });
        }
        if !failed.is_empty() {
            return Err(SshError::Remote {
                status: -1,
                message: format!(
                    "Could not upload {} to {}.",
                    failed.join(", "),
                    self.device_name()
                ),
            });
        }
        Ok(report)
    }

    /// Build outputs (PDF, SyncTeX, log…) from the remote, overwriting the
    /// local copies: they are generated, never edited, so no conflict rule.
    /// Missing paths are skipped. Returns the paths fetched.
    pub fn fetch(&self, paths: &[String]) -> Result<Vec<String>, SshError> {
        let mut inner = lock(&self.inner);
        let safe: Vec<String> = paths
            .iter()
            .filter(|path| RemoteSyncRules::is_safe_relative_path(path))
            .cloned()
            .collect();
        if safe.is_empty() {
            return Ok(Vec::new());
        }
        let _staging_cleanup = RemoveOnDrop(self.mirror.staging_dir());
        let staged = self.download(&safe)?;
        let mut fetched = Vec::new();
        self.commit_locally(&mut inner, |sync, inner| {
            for path in &safe {
                let Some(file) = staged.get(path) else { continue };
                sync.place(&file.url, path)?;
                inner.manifest.insert(path.clone(), file.hash.clone());
                inner.conflicts.remove(path);
                fetched.push(path.clone());
            }
            sync.save_manifest(inner)
        })?;
        Ok(fetched)
    }

    /// Settles a conflict: `keep_local` uploads this computer's copy over
    /// the remote one, otherwise the remote copy replaces the local file
    /// (or removes it when the remote file is confirmed gone). Neither
    /// side is overwritten if it changed again after the choice was made
    /// (the call fails and the conflict stays). Returns the remaining
    /// conflicts.
    pub fn resolve(&self, path: &str, keep_local: bool) -> Result<Vec<String>, SshError> {
        if !RemoteSyncRules::is_safe_relative_path(path) {
            return Ok(lock(&self.inner).conflicts.iter().cloned().collect());
        }
        // The local version the user chose against — taken before queueing
        // behind another transfer, so a save made meanwhile is not lost.
        let mine = self.local_hash(path);
        let mut inner = lock(&self.inner);
        let state = self
            .probe(&[path.to_string()])?
            .get(path)
            .cloned()
            .unwrap_or(RemoteFileState::Unavailable);
        if state == RemoteFileState::Unavailable {
            return Err(SshError::Remote {
                status: -1,
                message: format!("{path} cannot be read on {}.", self.device_name()),
            });
        }
        if keep_local {
            if mine.is_none() {
                // Deleted here. Deletions are not sent to the device, so
                // the device's copy becomes the baseline and stays put.
                if let RemoteFileState::File(hash) = &state {
                    inner.manifest.insert(path.to_string(), hash.clone());
                } else {
                    inner.manifest.remove(path);
                }
            } else {
                let mut expected = HashMap::new();
                if let RemoteFileState::File(hash) = &state {
                    expected.insert(path.to_string(), hash.clone());
                }
                match self.upload(&[path.to_string()], &expected)?.get(path) {
                    Some(UploadOutcome::Stored(hash, _)) => {
                        inner.manifest.insert(path.to_string(), hash.clone());
                    }
                    Some(UploadOutcome::Conflict) => {
                        return Err(SshError::Remote {
                            status: -1,
                            message: format!(
                                "{path} changed on {} again — try again.",
                                self.device_name()
                            ),
                        });
                    }
                    Some(UploadOutcome::Unsupported) => {
                        return Err(SshError::Remote {
                            status: -1,
                            message: format!("Could not upload {path} to {}: its folder refused a hard link (a filesystem without them, or out of space or quota), and Pitex only replaces files that way.", self.device_name()),
                        });
                    }
                    Some(UploadOutcome::Failed) | None => {
                        return Err(SshError::Remote {
                            status: -1,
                            message: format!(
                                "Could not upload {path} to {}.",
                                self.device_name()
                            ),
                        });
                    }
                }
            }
            inner.conflicts.remove(path);
            self.save_manifest(&mut inner)?;
            return Ok(inner.conflicts.iter().cloned().collect());
        }
        let changed_here = || SshError::Remote {
            status: -1,
            message: format!("{path} was saved here meanwhile — try again."),
        };
        if state == RemoteFileState::Absent {
            self.commit_locally(&mut inner, |sync, inner| {
                if sync.local_hash(path) != mine {
                    return Err(changed_here());
                }
                if let Some(url) = sync.contained_url(path, false) {
                    if item_exists(&url) {
                        std::fs::remove_file(&url).map_err(|e| SshError::Remote {
                            status: -1,
                            message: e.to_string(),
                        })?;
                    }
                }
                inner.manifest.remove(path);
                sync.drop_local_cache(path);
                inner.conflicts.remove(path);
                sync.save_manifest(inner)
            })?;
        } else {
            let _staging_cleanup = RemoveOnDrop(self.mirror.staging_dir());
            let staged = self.download(&[path.to_string()])?;
            let Some(file) = staged.get(path) else {
                return Err(SshError::Remote {
                    status: -1,
                    message: format!("{path} could not be downloaded."),
                });
            };
            self.commit_locally(&mut inner, |sync, inner| {
                if sync.local_hash(path) != mine {
                    return Err(changed_here());
                }
                sync.place(&file.url, path)?;
                inner.manifest.insert(path.to_string(), file.hash.clone());
                inner.conflicts.remove(path);
                sync.save_manifest(inner)
            })?;
        }
        Ok(inner.conflicts.iter().cloned().collect())
    }

    /// The remote counterpart of a local path inside the mirror.
    pub fn remote_path(&self, url: &Path) -> Option<String> {
        let root = standardize(&self.mirror.root());
        let path = standardize(url);
        if path == root {
            return Some(self.remote_root().to_string());
        }
        let rest = path.strip_prefix(&root).ok()?;
        if rest.as_os_str().is_empty() {
            return None;
        }
        Some(format!(
            "{}/{}",
            self.remote_root().trim_end_matches('/'),
            rest.to_string_lossy()
        ))
    }

    // ── Transfers ──

    fn download(&self, paths: &[String]) -> Result<HashMap<String, StagedFile>, SshError> {
        let _ = std::fs::remove_dir_all(self.mirror.staging_dir());
        std::fs::create_dir_all(self.mirror.staging_dir()).map_err(remote_io)?;
        std::fs::create_dir_all(self.mirror.work_dir()).map_err(remote_io)?;
        let archive = self.mirror.work_dir().join("download.tar");
        let _archive_cleanup = RemoveOnDrop(archive.clone());
        let list = paths.join("\u{0}");
        // tar exits non-zero when a listed file vanished meanwhile; what it
        // did archive is still usable, so only an empty archive is an error.
        let result = self.client.run(
            &remote_scripts::tar_out(),
            &[self.remote_root().to_string()],
            Some(list.as_bytes()),
            Some(&archive),
        )?;
        let size = std::fs::metadata(&archive).map(|m| m.len()).unwrap_or(0);
        if size == 0 {
            return Err(SshError::Remote {
                status: result.status,
                message: SshClient::first_line(&result.stderr_text())
                    .unwrap_or_else(|| "Download failed.".to_string()),
            });
        }
        // GNU tar refuses absolute and `..` members by default; only regular
        // files at expected paths are taken from staging.
        let untar = run_process(
            Path::new("/usr/bin/tar"),
            &[
                "-xf".to_string(),
                archive.to_string_lossy().into_owned(),
                "-C".to_string(),
                self.mirror.staging_dir().to_string_lossy().into_owned(),
            ],
            None,
            None,
            None,
            &[],
        )?;
        if untar.status != 0 && untar.status != 1 {
            return Err(SshError::Remote {
                status: untar.status,
                message: SshClient::first_line(&untar.stderr_text())
                    .unwrap_or_else(|| "The download could not be unpacked.".to_string()),
            });
        }
        let mut staged = HashMap::new();
        let staging_root = resolved(&self.mirror.staging_dir());
        for path in paths {
            if !RemoteSyncRules::is_safe_relative_path(path) {
                continue;
            }
            let url = staging_root.join(path);
            // A symlink anywhere on the way (a hostile archive's `dir ->
            // /home/me` followed by `dir/.ssh/id_ed25519`) would make the
            // move into the mirror pull an arbitrary local file along.
            if !is_free_of_symlinks(&url) || !is_regular_file(&url) {
                continue;
            }
            if let Some(hash) = sha256_hex_file(&url) {
                staged.insert(path.clone(), StagedFile { url, hash });
            }
        }
        Ok(staged)
    }

    /// Uploads snapshots of `paths`; on the device each replaces the remote
    /// file only if that still hashes to `expected[path]` (missing = the
    /// file must not exist). Paths that could not be snapshotted are left
    /// out of the result.
    fn upload(
        &self,
        paths: &[String],
        expected: &HashMap<String, String>,
    ) -> Result<HashMap<String, UploadOutcome>, SshError> {
        // `upload/expect` + `upload/data/<path>`: the archive's metadata
        // never shares a namespace with project paths.
        let snapshot = self.mirror.work_dir().join("upload");
        let data = snapshot.join("data");
        let list = self.mirror.work_dir().join("upload.list");
        let archive = self.mirror.work_dir().join("upload.tar");
        let _cleanup = RemoveOnDrop3(
            snapshot.clone(),
            list.clone(),
            archive.clone(),
        );
        let _ = std::fs::remove_dir_all(&snapshot);
        std::fs::create_dir_all(&data).map_err(remote_io)?;
        // Copies are what travel and what the manifest records: a save
        // landing mid-upload changes the mirror, not these bytes.
        let mut sent: HashMap<String, String> = HashMap::new();
        let mut expectations = String::new();
        for path in paths {
            let Some(source) = self.contained_url(path, false) else { continue };
            if !is_regular_file(&source) {
                continue;
            }
            let Ok(bytes) = std::fs::read(&source) else { continue };
            let copy = data.join(path);
            if let Some(parent) = copy.parent() {
                std::fs::create_dir_all(parent).map_err(remote_io)?;
            }
            std::fs::write(&copy, &bytes).map_err(remote_io)?;
            if let Ok(metadata) = std::fs::metadata(&source) {
                let _ = std::fs::set_permissions(&copy, metadata.permissions());
            }
            let hash = sha256_hex(&bytes);
            sent.insert(path.clone(), hash.clone());
            let base = expected.get(path).map(String::as_str).unwrap_or("-");
            expectations += &format!("{base} {hash} {path}\n");
        }
        if sent.is_empty() {
            return Ok(HashMap::new());
        }
        std::fs::write(snapshot.join("expect"), expectations.as_bytes()).map_err(remote_io)?;
        let mut members = vec!["expect".to_string()];
        let mut sorted: Vec<&String> = sent.keys().collect();
        sorted.sort();
        members.extend(sorted.into_iter().map(|path| format!("data/{path}")));
        std::fs::write(&list, members.join("\u{0}").as_bytes()).map_err(remote_io)?;
        let pack = run_process(
            Path::new("/usr/bin/tar"),
            &[
                "-cf".to_string(),
                archive.to_string_lossy().into_owned(),
                "--null".to_string(),
                "-T".to_string(),
                list.to_string_lossy().into_owned(),
            ],
            None,
            None,
            Some(&snapshot),
            &[("COPYFILE_DISABLE", "1")],
        )?;
        if pack.status != 0 {
            return Err(SshError::Remote {
                status: pack.status,
                message: SshClient::first_line(&pack.stderr_text())
                    .unwrap_or_else(|| "The upload could not be packed.".to_string()),
            });
        }
        let archive_bytes = std::fs::read(&archive).map_err(remote_io)?;
        let result = self.client.run_checked(
            &remote_scripts::commit_upload(),
            &[self.remote_root().to_string()],
            Some(&archive_bytes),
            None,
        )?;
        let mut outcomes: HashMap<String, UploadOutcome> = sent
            .iter()
            .map(|(path, _)| (path.clone(), UploadOutcome::Failed))
            .collect();
        for line in result.stdout_text().split('\n') {
            if line.chars().count() <= 2 {
                continue;
            }
            let mut chars = line.chars();
            let kind = match (chars.next(), chars.next()) {
                (Some(letter), Some('/')) => letter,
                _ => continue,
            };
            let path = chars.as_str();
            let Some(hash) = sent.get(path) else { continue };
            match kind {
                'U' => {
                    outcomes.insert(path.to_string(), UploadOutcome::Stored(hash.clone(), true));
                }
                'S' => {
                    outcomes.insert(path.to_string(), UploadOutcome::Stored(hash.clone(), false));
                }
                'C' => {
                    outcomes.insert(path.to_string(), UploadOutcome::Conflict);
                }
                'L' => {
                    outcomes.insert(path.to_string(), UploadOutcome::Unsupported);
                }
                _ => {
                    outcomes.insert(path.to_string(), UploadOutcome::Failed);
                }
            }
        }
        Ok(outcomes)
    }

    fn probe(&self, paths: &[String]) -> Result<HashMap<String, RemoteFileState>, SshError> {
        let list = paths
            .iter()
            .map(|path| format!("{path}\n"))
            .collect::<String>();
        let result = self.client.run_checked(
            &remote_scripts::probe(),
            &[self.remote_root().to_string()],
            Some(list.as_bytes()),
            None,
        )?;
        Ok(remote_scripts::parse_probe(&result.stdout_text()))
    }

    // ── Local side ──

    /// Runs local replacements/deletions with the app's own saves held
    /// back (see `MirrorWriteGate`).
    fn commit_locally<R>(
        &self,
        inner: &mut SyncInner,
        body: impl FnOnce(&RemoteSync, &mut SyncInner) -> Result<R, SshError>,
    ) -> Result<R, SshError> {
        // RAII: a panicking body still releases the gate — otherwise one
        // panic would leave `committing` set and every later save blocked.
        struct CommitGuard<'a>(&'a Option<Arc<dyn MirrorWriteGate>>);
        impl Drop for CommitGuard<'_> {
            fn drop(&mut self) {
                if let Some(gate) = self.0 {
                    gate.end_sync_commit();
                }
            }
        }
        if let Some(gate) = &self.gate {
            gate.begin_sync_commit();
        }
        let _guard = CommitGuard(&self.gate);
        body(self, inner)
    }

    /// The mirror URL for `path`, or None when an existing component on the
    /// way is anything but a real directory — a symlink there (made by an
    /// agent or tool) would send a write or delete outside the mirror.
    /// `create` makes missing directories; otherwise a missing one is None.
    fn contained_url(&self, path: &str, create: bool) -> Option<PathBuf> {
        if !RemoteSyncRules::is_safe_relative_path(path) {
            return None;
        }
        let components: Vec<&str> = path.split('/').collect();
        let mut url = self.mirror.root();
        for component in &components[..components.len() - 1] {
            url.push(component);
            // symlink_metadata does not follow a symlink.
            match std::fs::symlink_metadata(&url) {
                Ok(metadata) => {
                    if !metadata.is_dir() {
                        return None;
                    }
                }
                Err(_) => {
                    if !create || std::fs::create_dir(&url).is_err() {
                        return None;
                    }
                }
            }
        }
        Some(url.join(components[components.len() - 1]))
    }

    fn local_hash(&self, path: &str) -> Option<String> {
        let url = self.contained_url(path, false)?;
        if !is_regular_file(&url) {
            return None;
        }
        let metadata = std::fs::metadata(&url).ok()?;
        let size = metadata.len();
        let modified = metadata.modified().ok()?;
        {
            let cache = lock(&self.local_cache);
            if let Some(cached) = cache.get(path) {
                if cached.size == size && cached.modified == modified {
                    return Some(cached.hash.clone());
                }
            }
        }
        let hash = sha256_hex_file(&url)?;
        lock(&self.local_cache).insert(
            path.to_string(),
            LocalCacheEntry {
                size,
                modified,
                hash: hash.clone(),
            },
        );
        Some(hash)
    }

    /// Every mirrored local file (same exclusions as the remote listing).
    fn local_hashes(&self) -> HashMap<String, String> {
        let mut hashes = HashMap::new();
        let root = self.mirror.root();
        let mut stack = vec![root.clone()];
        while let Some(directory) = stack.pop() {
            let Ok(entries) = std::fs::read_dir(&directory) else { continue };
            for entry in entries.flatten() {
                let url = entry.path();
                let name = entry.file_name();
                let name = name.to_string_lossy();
                if RemoteSyncRules::is_excluded(&name) {
                    continue;
                }
                // symlink_metadata: a symlinked directory is neither
                // descended nor hashed, like FileManager's enumerator.
                let Ok(metadata) = std::fs::symlink_metadata(&url) else { continue };
                if metadata.is_dir() {
                    stack.push(url);
                    continue;
                }
                if !metadata.is_file() || metadata.len() >= RemoteSyncRules::MAXIMUM_FILE_SIZE {
                    continue;
                }
                let Ok(relative) = url.strip_prefix(&root) else { continue };
                let Some(path) = relative.to_str().map(|s| s.to_string()) else {
                    continue;
                };
                if let Some(hash) = self.local_hash(&path) {
                    hashes.insert(path, hash);
                }
            }
        }
        hashes
    }

    /// Moves a staged regular file into the mirror, replacing the old one.
    fn place(&self, staged: &Path, path: &str) -> Result<(), SshError> {
        let Some(destination) = self.contained_url(path, true) else {
            return Err(SshError::Remote {
                status: -1,
                message: format!("{path} is not inside the project folder."),
            });
        };
        // rename(2) replaces atomically (a symlink at the final component
        // is replaced itself, not followed; a directory there fails);
        // staging sits beside the mirror on the same volume.
        if let Err(error) = std::fs::rename(staged, &destination) {
            return Err(SshError::Remote {
                status: -1,
                message: format!("Could not update {path}: {error}"),
            });
        }
        lock(&self.local_cache).remove(path);
        Ok(())
    }

    fn drop_local_cache(&self, path: &str) {
        lock(&self.local_cache).remove(path);
    }

    fn save_manifest(&self, inner: &mut SyncInner) -> Result<(), SshError> {
        let bytes = serde_json::to_vec(&inner.manifest).map_err(remote_io_json)?;
        write_atomic(&self.mirror.manifest_path(), &bytes)
    }
}

fn remote_io(error: std::io::Error) -> SshError {
    SshError::Remote {
        status: -1,
        message: error.to_string(),
    }
}
fn remote_io_json(error: serde_json::Error) -> SshError {
    SshError::Remote {
        status: -1,
        message: error.to_string(),
    }
}

fn item_exists(url: &Path) -> bool {
    std::fs::symlink_metadata(url).is_ok()
}

/// A regular file itself, not a symlink to one.
fn is_regular_file(url: &Path) -> bool {
    std::fs::symlink_metadata(url)
        .map(|m| m.is_file())
        .unwrap_or(false)
}

/// `resolvingSymlinksInPath() == standardizedFileURL` — every component
/// must be real, so a staged file pulled along a symlink never escapes.
fn is_free_of_symlinks(url: &Path) -> bool {
    std::fs::canonicalize(url).map(|c| c == standardize(url)).unwrap_or(false)
}

struct RemoveOnDrop(PathBuf);
impl Drop for RemoveOnDrop {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
        let _ = std::fs::remove_file(&self.0);
    }
}

struct RemoveOnDrop3(PathBuf, PathBuf, PathBuf);
impl Drop for RemoveOnDrop3 {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
        let _ = std::fs::remove_file(&self.1);
        let _ = std::fs::remove_file(&self.2);
    }
}

// ─── RemoteBuildExecutor.swift ───────────────────────────────────────────────

/// Runs build stages on the remote device instead of this computer: each
/// stage's command executes in the matching remote directory through the
/// remote user's login shell (their TeX installation), output streams back
/// live, and after every stage the build outputs are fetched into the
/// mirror so the orchestrator's PDF/SyncTeX handling works on local files
/// as usual.
pub struct RemoteBuildExecutor {
    pub sync: Arc<RemoteSync>,
    /// Project-relative paths to fetch after each stage (the target's PDF,
    /// SyncTeX data, log…); set before each build.
    outputs: Mutex<Vec<String>>,
    /// The output a successful stage must deliver (the target's PDF).
    required_output: Mutex<Option<String>>,
    running: Mutex<HashMap<String, Arc<CancelScope>>>,
}

impl RemoteBuildExecutor {
    pub fn new(sync: Arc<RemoteSync>) -> Self {
        Self {
            sync,
            outputs: Mutex::new(Vec::new()),
            required_output: Mutex::new(None),
            running: Mutex::new(HashMap::new()),
        }
    }

    pub fn set_outputs(&self, paths: Vec<String>, required: Option<String>) {
        *lock(&self.outputs) = paths;
        *lock(&self.required_output) = required;
    }
}

impl BuildProcessExecuting for RemoteBuildExecutor {
    fn execute(
        &self,
        request: &BuildProcessRequest,
        output: &mut (dyn FnMut(BuildProcessOutput) + Send),
    ) -> Result<BuildProcessResult, BuildProcessExecutorError> {
        let remote_root = self.sync.mirror.project.remote_root.clone();
        let device = self.sync.mirror.project.connection.name.clone();
        let (working_directory, environment, command) = match &request.command {
            BuildProcessCommand::Direct(plan) => (
                plan.working_directory.clone(),
                plan.environment.clone(),
                std::iter::once(plan.executable.clone())
                    .chain(plan.arguments.iter().cloned())
                    .collect::<Vec<_>>(),
            ),
            BuildProcessCommand::LoginShell(plan) => (
                plan.working_directory.clone(),
                plan.environment.clone(),
                vec![
                    "/bin/sh".to_string(),
                    "-c".to_string(),
                    plan.command.clone(),
                ],
            ),
        };
        let directory = match &working_directory {
            WorkingDirectoryPolicy::ProjectRoot => remote_root.clone(),
            WorkingDirectoryPolicy::SourceDirectory => self
                .sync
                .remote_path(&request.source_directory)
                .unwrap_or_else(|| remote_root.clone()),
            WorkingDirectoryPolicy::Explicit(path) => self
                .sync
                .remote_path(Path::new(path))
                .ok_or_else(|| {
                    BuildProcessExecutorError::LaunchFailed(format!(
                        "{path} is outside the remote project."
                    ))
                })?,
        };
        // Local environment values (this computer's PATH, TEXINPUTS…) mean
        // nothing on the remote; only explicit overrides travel.
        let overrides: Vec<String> = match &environment {
            EnvironmentPolicy::Inherit { overrides } => overrides
                .iter()
                .filter(|(key, _)| key.as_str() != "PATH")
                .map(|(key, value)| format!("{key}={value}"))
                .collect::<Vec<_>>()
                .tap_sorted(),
            EnvironmentPolicy::Replace(values) => std::iter::once("-i".to_string())
                .chain(
                    values
                        .iter()
                        .filter(|(key, _)| key.as_str() != "PATH")
                        .map(|(key, value)| format!("{key}={value}")),
                )
                .collect::<Vec<_>>()
                .tap_sorted_tail(),
        };
        let words = if overrides.is_empty() {
            command
        } else {
            std::iter::once("env".to_string())
                .chain(overrides)
                .chain(command)
                .collect()
        };
        if request.stage_index == 0 {
            output(BuildProcessOutput {
                channel: BuildLogChannel::System,
                bytes: format!("Building on {device}: {directory}\n").into_bytes(),
            });
        }
        let scope = Arc::new(CancelScope::new());
        lock(&self.running).insert(request.build_id.raw_value.clone(), scope.clone());
        let client = self.sync.client.clone();
        let mut arguments = vec![directory];
        arguments.extend(words);
        let mut emit = |data: &[u8], is_error: bool| {
            output(BuildProcessOutput {
                channel: if is_error {
                    BuildLogChannel::StandardError
                } else {
                    BuildLogChannel::StandardOutput
                },
                bytes: data.to_vec(),
            });
        };
        let streamed = client.stream(
            &remote_scripts::login_exec(),
            &arguments,
            true,
            &scope,
            &mut emit,
        );
        lock(&self.running).remove(&request.build_id.raw_value);
        let status = match streamed {
            Err(SshError::Cancelled) => return Ok(BuildProcessResult { exit_code: 130 }),
            Err(error) => {
                return Err(BuildProcessExecutorError::LaunchFailed(error.to_string()))
            }
            Ok(status) => status,
        };
        if status == 255 {
            return Err(BuildProcessExecutorError::LaunchFailed(format!(
                "The SSH connection to {device} failed."
            )));
        }
        // Fetch even after a failed stage: a partial PDF and the log are
        // what the user needs to see why.
        let outputs = lock(&self.outputs).clone();
        let mut fetched: Vec<String> = Vec::new();
        let mut failure: Option<String> = None;
        match self.sync.fetch(&outputs) {
            Ok(done) => fetched = done,
            Err(error) => {
                failure = Some(format!("Could not download the build outputs: {error}"));
            }
        }
        let required = lock(&self.required_output).clone();
        if failure.is_none() {
            if let Some(required) = &required {
                if !fetched.contains(required) {
                    failure =
                        Some(format!("The build on {device} did not produce {required}."));
                }
            }
        }
        if let Some(failure) = failure {
            // A successful stage without its PDF must not pass: the PDF on
            // screen would still be the previous build's.
            if status == 0 {
                return Err(BuildProcessExecutorError::LaunchFailed(failure));
            }
            output(BuildProcessOutput {
                channel: BuildLogChannel::System,
                bytes: format!("{failure}\n").into_bytes(),
            });
        }
        Ok(BuildProcessResult { exit_code: status })
    }

    fn cancel(&self, build_id: &BuildID) {
        if let Some(scope) = lock(&self.running).get(&build_id.raw_value) {
            scope.cancel();
        }
    }
}

/// Sorts all elements (for `Inherit` overrides).
trait TapSorted {
    fn tap_sorted(self) -> Self;
    fn tap_sorted_tail(self) -> Self;
}
impl TapSorted for Vec<String> {
    fn tap_sorted(mut self) -> Self {
        self.sort();
        self
    }
    /// Keeps the first element in place, sorts the rest (for `-i` + values).
    fn tap_sorted_tail(mut self) -> Self {
        if self.len() > 1 {
            self[1..].sort();
        }
        self
    }
}

/// The app side of `MirrorWriteGate`, global like Swift's
/// `MirrorWrites.shared`: an editor save holds a ticket while it checks
/// and replaces its file; a sync commit waits for the tickets to drain and
/// holds new saves back until it ends. Waiting commits go first, so a
/// stream of saves cannot starve them.
pub struct MirrorWrites {
    state: Mutex<MirrorWriteState>,
    changed: Condvar,
}

#[derive(Default)]
struct MirrorWriteState {
    writers: usize,
    committing: bool,
    pending_commits: usize,
}

impl MirrorWrites {
    const fn new() -> Self {
        Self {
            state: Mutex::new(MirrorWriteState {
                writers: 0,
                committing: false,
                pending_commits: 0,
            }),
            changed: Condvar::new(),
        }
    }

    pub fn shared() -> &'static MirrorWrites {
        static SHARED: MirrorWrites = MirrorWrites::new();
        &SHARED
    }

    pub fn enter(&self) {
        let mut state = lock(&self.state);
        while state.committing || state.pending_commits > 0 {
            state = self.changed.wait(state).unwrap_or_else(|e| e.into_inner());
        }
        state.writers += 1;
    }

    pub fn leave(&self) {
        let mut state = lock(&self.state);
        state.writers = state.writers.saturating_sub(1);
        if state.writers == 0 {
            self.changed.notify_all();
        }
    }

    /// RAII ticket — `leave` on drop.
    pub fn ticket(&self) -> MirrorWriteTicket<'_> {
        self.enter();
        MirrorWriteTicket { gate: self }
    }
}

pub struct MirrorWriteTicket<'a> {
    gate: &'a MirrorWrites,
}
impl Drop for MirrorWriteTicket<'_> {
    fn drop(&mut self) {
        self.gate.leave();
    }
}

impl MirrorWriteGate for MirrorWrites {
    fn begin_sync_commit(&self) {
        let mut state = lock(&self.state);
        state.pending_commits += 1;
        while state.committing || state.writers > 0 {
            state = self.changed.wait(state).unwrap_or_else(|e| e.into_inner());
        }
        state.pending_commits -= 1;
        state.committing = true;
    }

    fn end_sync_commit(&self) {
        let mut state = lock(&self.state);
        state.committing = false;
        self.changed.notify_all();
    }
}
