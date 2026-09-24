//! Port of `PiRPC.swift`, `PiAgentProcess.swift`, `PiAuthStore.swift`, and
//! `AgentCoordinator.swift` — the `pi --mode rpc` subprocess plus the
//! transcript/state machine driving the assistant panel.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Duration;
use std::sync::mpsc::{channel, Receiver};
use std::sync::{Arc, Mutex};

use serde_json::{json, Map, Value};

use crate::model::uuid_v4;

// ─── PiPaths (verbatim) ─────────────────────────────────────────────────────

/// App-local pi home — `~/.config/pitex/pi` on Linux, `PI_CODING_AGENT_DIR`
/// wins for debugging.
pub mod pi_paths {
    use std::path::PathBuf;

    pub fn agent_directory() -> PathBuf {
        if let Ok(override_dir) = std::env::var("PI_CODING_AGENT_DIR") {
            if !override_dir.is_empty() {
                let expanded = if let Some(rest) = override_dir.strip_prefix("~/") {
                    dirs::home_dir()
                        .map(|h| h.join(rest))
                        .unwrap_or_else(|| PathBuf::from(&override_dir))
                } else {
                    PathBuf::from(override_dir)
                };
                return expanded;
            }
        }
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("pitex")
            .join("pi")
    }
    pub fn auth_file() -> PathBuf {
        agent_directory().join("auth.json")
    }
    pub fn settings_file() -> PathBuf {
        agent_directory().join("settings.json")
    }
    pub fn models_file() -> PathBuf {
        agent_directory().join("models.json")
    }
    pub fn skills_directory() -> PathBuf {
        agent_directory().join("skills")
    }
    pub fn runtime_directory() -> PathBuf {
        agent_directory()
            .parent()
            .map(|p| p.join("pi-runtime"))
            .unwrap_or_else(|| PathBuf::from("pi-runtime"))
    }
    /// The app-local launcher. Unix publishes a `bin/pi` symlink into the
    /// package's `dist/cli.js`; Windows skips the indirection (no symlink
    /// privilege) and points straight at the installed `cli.js` — `launch`
    /// runs `.js` entry points through Bun/Node on every platform.
    #[cfg(unix)]
    pub fn runtime_executable() -> PathBuf {
        runtime_directory().join("bin/pi")
    }
    #[cfg(windows)]
    pub fn runtime_executable() -> PathBuf {
        let dist = runtime_directory().join("node_modules/@earendil-works/pi-coding-agent/dist");
        let unbundled = dist.join("cli.js");
        if unbundled.exists() {
            unbundled
        } else {
            dist.join("bundle/cli.js")
        }
    }
    /// `configurationFile(customProvider:)` — creates the file when missing.
    pub fn configuration_file(custom_provider: bool) -> std::io::Result<PathBuf> {
        let url = if custom_provider {
            models_file()
        } else {
            settings_file()
        };
        std::fs::create_dir_all(agent_directory())?;
        if !url.exists() {
            let initial = if custom_provider {
                "{\n  \"providers\": {}\n}\n"
            } else {
                "{\n}\n"
            };
            std::fs::write(&url, initial)?;
        }
        Ok(url)
    }
}

/// `PiAuthStore` — true when any provider credential exists in app-local
/// `auth.json`. Token material never leaves the file.
pub fn pi_has_any_credential() -> bool {
    std::fs::read(pi_paths::auth_file())
        .ok()
        .and_then(|d| serde_json::from_slice::<Value>(&d).ok())
        .and_then(|v| v.as_object().map(|o| !o.is_empty()))
        .unwrap_or(false)
}

/// `PiExecutableLocator.resolve(environment:)` — PI_AGENT_PATH wins, then a
/// runtime bundled with the install, then the app-local runtime, then PATH.
pub fn locate_pi_executable_in(environment: &HashMap<String, String>) -> Option<PathBuf> {
    if let Some(env) = environment.get("PI_AGENT_PATH") {
        if !env.is_empty() && is_executable(Path::new(env)) {
            return Some(PathBuf::from(env));
        }
    }
    if let Some(bundled) = bundled_executable() {
        return Some(bundled);
    }
    if is_executable(&pi_paths::runtime_executable()) {
        return Some(pi_paths::runtime_executable());
    }
    PiToolchain::find("pi", environment)
}

/// A pi runtime shipped inside the install image — the Linux counterpart of
/// `Bundle.main.resourceURL/pi-runtime/bin/pi`. Packaging can drop the
/// runtime next to the executable or under the shared data directory.
/// On Windows the launcher is the package's `cli.js` itself (see
/// `runtime_executable`), so the candidates mirror that layout.
#[cfg(unix)]
fn bundled_executable() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?;
    for candidate in [
        dir.join("pi-runtime/bin/pi"),
        dir.join("../share/pitex/pi-runtime/bin/pi"),
        dir.join("../lib/pitex/pi-runtime/bin/pi"),
    ] {
        let candidate = candidate.canonicalize().unwrap_or(candidate);
        if is_executable(&candidate) {
            return Some(candidate);
        }
    }
    None
}
#[cfg(windows)]
fn bundled_executable() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?;
    const ENTRY: &str = "pi-runtime/node_modules/@earendil-works/pi-coding-agent/dist/cli.js";
    for candidate in [
        dir.join(ENTRY),
        dir.join(format!("../share/pitex/{ENTRY}")),
        dir.join(format!("../lib/pitex/{ENTRY}")),
    ] {
        let candidate = candidate.canonicalize().unwrap_or(candidate);
        if is_executable(&candidate) {
            return Some(candidate);
        }
    }
    None
}

pub fn app_local_runtime_installed() -> bool {
    is_executable(&pi_paths::runtime_executable())
}

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path)
        .map(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

/// Windows has no permission-bit executability: a file is launchable when it
/// exists and carries a runnable extension — `launch` routes `.js`-family
/// entry points through Bun/Node, the rest through `cmd`.
#[cfg(windows)]
fn is_executable(path: &Path) -> bool {
    const RUNNABLE: [&str; 7] = ["exe", "com", "bat", "cmd", "ps1", "js", "mjs"];
    path.is_file()
        && path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| RUNNABLE.contains(&e.to_ascii_lowercase().as_str()))
            .unwrap_or(false)
}

/// `PiToolchain` — an app launched from a desktop session misses the PATH
/// entries shell startup files and version managers add. Discover once per
/// operation, then reuse the same PATH for installation, RPC and
/// authentication. `discover` blocks (shell probe + npm prefix probe), so it
/// always runs on a worker thread, never the GTK main loop.
#[derive(Clone, Debug)]
pub struct PiToolchain {
    pub environment: HashMap<String, String>,
    pub bun: Option<PathBuf>,
    pub node: Option<PathBuf>,
    pub npm: Option<PathBuf>,
}

impl PiToolchain {
    /// System directories appended after the ambient PATH — the Linux
    /// counterpart of `/opt/homebrew/bin` & friends in the Swift default.
    #[cfg(unix)]
    pub const SYSTEM_DIRECTORIES: [&'static str; 7] = [
        "/usr/local/bin",
        "/home/linuxbrew/.linuxbrew/bin",
        "/opt/bin",
        "/usr/bin",
        "/bin",
        "/usr/sbin",
        "/sbin",
    ];
    /// The Windows counterpart: the standard Node.js installer targets plus
    /// the `nvm-windows` shim directory (covered again via `NVM_SYMLINK`).
    #[cfg(windows)]
    pub const SYSTEM_DIRECTORIES: [&'static str; 3] = [
        "C:\\Program Files\\nodejs",
        "C:\\Program Files (x86)\\nodejs",
        "C:\\Program Files\\Git\\bin",
    ];

    /// Names a PATH probe should try: on Unix the bare name, on Windows the
    /// name plus each PATHEXT-style runnable suffix (`npm` resolves to
    /// `npm.cmd`, `pi` to `pi.cmd`, …).
    #[cfg(unix)]
    fn executable_names(name: &str) -> Vec<String> {
        vec![name.to_string()]
    }
    #[cfg(windows)]
    fn executable_names(name: &str) -> Vec<String> {
        [".exe", ".cmd", ".bat", ".ps1", ""]
            .iter()
            .map(|ext| format!("{name}{ext}"))
            .collect()
    }

    pub fn find(name: &str, environment: &HashMap<String, String>) -> Option<PathBuf> {
        let path = environment.get("PATH")?;
        std::env::split_paths(path)
            .flat_map(|dir| {
                Self::executable_names(name)
                    .into_iter()
                    .map(move |n| dir.join(&n))
            })
            .find(|candidate| is_executable(candidate))
    }

    /// `PiToolchain.discover(environment:home:systemDirectories:)` — verbatim.
    /// Order matters: ambient PATH, system dirs, version-manager homes,
    /// package-manager prefixes, then an interactive login-shell probe whose
    /// entries lead, then nvm/fnm version directories, and finally npm's
    /// global prefix (for a bun installed globally without PATH).
    pub fn discover(
        environment: HashMap<String, String>,
        home: &Path,
        system_directories: &[&str],
    ) -> PiToolchain {
        // Explorer commonly supplies "Path"; Windows keys are case-insensitive.
        #[cfg(windows)]
        let environment = environment.into_iter().map(|(key, value)| (key.to_ascii_uppercase(), value)).collect();
        let mut result = PiToolchain {
            environment,
            bun: None,
            node: None,
            npm: None,
        };
        let mut paths: Vec<String> = result
            .environment
            .get("PATH")
            .map(|p| {
                std::env::split_paths(std::ffi::OsStr::new(p))
                    .map(|d| d.to_string_lossy().into_owned())
                    .collect()
            })
            .unwrap_or_default();
        paths.extend(system_directories.iter().map(|s| s.to_string()));
        // Version-manager and user-prefix install dirs, relative to home.
        #[cfg(unix)]
        let home_relative: [&str; 8] = [
            ".bun/bin",
            ".npm-global/bin",
            ".local/bin",
            ".volta/bin",
            ".asdf/shims",
            ".local/share/mise/shims",
            ".nix-profile/bin",
            ".local/share/pnpm",
        ];
        #[cfg(windows)]
        let home_relative: [&str; 6] = [
            ".bun/bin",
            "AppData/Roaming/npm",
            "AppData/Local/Programs/bun",
            "AppData/Local/pnpm",
            "scoop/shims",
            ".volta/bin",
        ];
        for rel in home_relative {
            paths.push(home.join(rel).to_string_lossy().into_owned());
        }
        // Prefix env keys — the value is a prefix, the binaries live in bin/
        // (except on Windows, where the npm prefix IS the shim directory).
        #[cfg(unix)]
        let prefix_bin_keys: [&str; 4] = [
            "BUN_INSTALL",
            "NPM_CONFIG_PREFIX",
            "npm_config_prefix",
            "HOMEBREW_PREFIX",
        ];
        #[cfg(windows)]
        let prefix_bin_keys: [&str; 4] = [
            "BUN_INSTALL",
            "NPM_CONFIG_PREFIX",
            "npm_config_prefix",
            "PNPM_HOME",
        ];
        for key in prefix_bin_keys {
            if let Some(prefix) = result.environment.get(key) {
                if !prefix.is_empty() {
                    let expanded = expand_tilde(prefix, home);
                    #[cfg(unix)]
                    paths.push(format!("{expanded}/bin"));
                    #[cfg(windows)]
                    paths.push(expanded);
                }
            }
        }
        // These keys already name the executable directory itself —
        // nvm-windows' active-version symlink is the main one.
        #[cfg(windows)]
        for key in ["NVM_SYMLINK", "NVM_HOME"] {
            if let Some(dir) = result.environment.get(key) {
                if !dir.is_empty() {
                    paths.push(expand_tilde(dir, home));
                }
            }
        }
        update_path(&mut result.environment, &paths);
        // Interactive startup is needed for .zshrc/.bashrc-managed installs.
        // The timeout prevents a shell startup prompt from hanging the app.
        // Windows has no login shell — PATH probing ends here.
        #[cfg(unix)]
        {
            let shell = result
                .environment
                .get("SHELL")
                .cloned()
                .unwrap_or_else(|| "/bin/bash".into());
            if let Some(path) = run_probe(
                &shell,
                &[
                    "-ilc".into(),
                    "/usr/bin/printf '\\0'; /usr/bin/printenv PATH".into(),
                ],
                &result.environment,
                home,
                5,
            )
            .and_then(|out| {
                String::from_utf8_lossy(&out)
                    .rsplit('\0')
                    .next()
                    .and_then(|s| s.lines().next())
                    .map(|s| s.to_string())
            }) {
                let mut shell_paths: Vec<String> =
                    std::env::split_paths(std::ffi::OsStr::new(&path))
                        .map(|d| d.to_string_lossy().into_owned())
                        .collect();
                shell_paths.extend(paths);
                paths = shell_paths;
            }
        }
        // nvm/fnm may only initialize in a terminal with a TTY. Their
        // installed binaries are still usable directly.
        #[cfg(unix)]
        for (directory, suffix) in [
            (".nvm/versions/node", "bin"),
            (".local/share/fnm/node-versions", "installation/bin"),
        ] {
            let root = home.join(directory);
            let mut versions: Vec<String> = std::fs::read_dir(&root)
                .map(|rd| {
                    rd.flatten()
                        .map(|e| e.file_name().to_string_lossy().into_owned())
                        .collect()
                })
                .unwrap_or_default();
            versions.sort_by(|a, b| numeric_compare(b, a));
            for version in versions {
                paths.push(
                    root.join(&version)
                        .join(suffix)
                        .to_string_lossy()
                        .into_owned(),
                );
            }
        }
        update_path(&mut result.environment, &paths);
        result.node = Self::find("node", &result.environment);
        result.npm = Self::find("npm", &result.environment);
        // npm's global prefix can be customized without adding it to PATH.
        if Self::find("bun", &result.environment).is_none() {
            if let Some((exe, args)) =
                result.npm_command(&["prefix".into(), "--global".into()])
            {
                if let Some(prefix) =
                    run_probe(&exe.to_string_lossy(), &args, &result.environment, home, 5)
                        .and_then(|out| {
                            String::from_utf8_lossy(&out)
                                .lines()
                                .filter(|l| !l.is_empty())
                                .last()
                                .filter(|l| Path::new(l).is_absolute())
                                .map(|s| s.to_string())
                        })
                {
                    #[cfg(unix)]
                    paths.insert(0, format!("{prefix}/bin"));
                    #[cfg(windows)]
                    paths.insert(0, prefix);
                    update_path(&mut result.environment, &paths);
                }
            }
        }
        result.bun = Self::find("bun", &result.environment);
        result
    }

    /// npm can be a JS symlink or an executable version-manager shim; a `.js`
    /// script runs through node instead of relying on its `env` shebang.
    pub fn npm_command(&self, arguments: &[String]) -> Option<(PathBuf, Vec<String>)> {
        let (npm, node) = (self.npm.as_ref()?, self.node.as_ref()?);
        let script = crate::model::standardize(npm.clone());
        if script
            .extension()
            .map(|e| e == "js")
            .unwrap_or(false)
        {
            let mut args = vec![script.to_string_lossy().into_owned()];
            args.extend(arguments.iter().cloned());
            Some((node.clone(), args))
        } else {
            Some((npm.clone(), arguments.to_vec()))
        }
    }

    /// `launch(_:arguments:)` — do not rely on a JavaScript executable's
    /// `env node` shebang: Bun-only machines must use Bun for the agent
    /// itself, not just for its installer.
    pub fn launch(
        &self,
        executable: &Path,
        arguments: &[String],
    ) -> Result<(PathBuf, Vec<String>), String> {
        // Node cannot load a main module spelled as a Windows verbatim disk
        // path (\\?\C:\...). Reuse the app's canonical, plain-path helper.
        let script = crate::model::standardize(executable.to_path_buf());
        let header = std::fs::File::open(&script)
            .ok()
            .and_then(|mut f| {
                let mut buf = vec![0u8; 256];
                let n = std::io::Read::read(&mut f, &mut buf).ok()?;
                buf.truncate(n);
                Some(buf)
            })
            .map(|b| {
                String::from_utf8_lossy(&b)
                    .lines()
                    .next()
                    .unwrap_or("")
                    .to_string()
            })
            .unwrap_or_default();
        // Rust handles cmd/bat escaping natively; PowerShell needs an interpreter.
        #[cfg(windows)]
        if let Some(ext) = script.extension().and_then(|e| e.to_str()) {
            let lower = ext.to_ascii_lowercase();
            if lower == "cmd" || lower == "bat" {
                return Ok((script, arguments.to_vec()));
            }
            if lower == "ps1" {
                let mut args = vec![
                    "-NoProfile".into(),
                    "-ExecutionPolicy".into(),
                    "Bypass".into(),
                    "-File".into(),
                    script.to_string_lossy().into_owned(),
                ];
                args.extend(arguments.iter().cloned());
                return Ok((PathBuf::from("powershell"), args));
            }
        }
        let java_script = ["js", "mjs", "cjs"]
            .iter()
            .any(|ext| script.extension().map(|e| e == *ext).unwrap_or(false))
            || (header.starts_with("#!")
                && (header.contains("node") || header.contains("bun")));
        if !java_script {
            return Ok((executable.to_path_buf(), arguments.to_vec()));
        }
        if let Some(bun) = &self.bun {
            // Pi 0.85's bundled undici imports fail under Bun 1.3. The
            // package also ships an unbundled CLI that avoids those imports.
            let unbundled = script
                .parent()
                .and_then(|p| p.parent())
                .map(|p| p.join("cli.js"));
            let entry = if script.file_name().map(|n| n == "cli.js").unwrap_or(false)
                && script
                    .parent()
                    .and_then(|p| p.file_name())
                    .map(|n| n == "bundle")
                    .unwrap_or(false)
                && unbundled.as_ref().map(|p| p.exists()).unwrap_or(false)
            {
                unbundled.unwrap()
            } else {
                script
            };
            let mut args = vec!["--bun".into(), entry.to_string_lossy().into_owned()];
            args.extend(arguments.iter().cloned());
            return Ok((bun.clone(), args));
        }
        if let Some(node) = &self.node {
            let mut args = vec![script.to_string_lossy().into_owned()];
            args.extend(arguments.iter().cloned());
            return Ok((node.clone(), args));
        }
        Err(pi_installer::RUNTIME_MISSING.into())
    }
}

/// Rewrite `environment["PATH"]` to the deduped absolute entries of `paths`
/// (`updatePath` in the Swift closure).
fn update_path(environment: &mut HashMap<String, String>, paths: &[String]) {
    let mut seen = std::collections::HashSet::new();
    let separator = if cfg!(windows) { ";" } else { ":" };
    environment.insert(
        "PATH".into(),
        paths
            .iter()
            .filter(|p| Path::new(p).is_absolute() && seen.insert((*p).clone()))
            .cloned()
            .collect::<Vec<_>>()
            .join(separator),
    );
}

/// `ProcessRunner().run(DirectCommandPlan…)` — a short-lived probe: run the
/// command with the toolchain PATH and return stdout only on a clean
/// completed exit(0).
fn run_probe(
    executable: &str,
    arguments: &[String],
    environment: &HashMap<String, String>,
    home: &Path,
    timeout_secs: u64,
) -> Option<Vec<u8>> {
    let plan = build_core::DirectCommandPlan::new(
        executable,
        arguments.to_vec(),
        build_core::WorkingDirectoryPolicy::Explicit(home.to_string_lossy().into_owned()),
        build_core::EnvironmentPolicy::Inherit {
            overrides: environment.clone(),
        },
    )
    .ok()?;
    let result = build_core::ProcessRunner::default()
        .run(
            &plan,
            home,
            None,
            Some(std::time::Duration::from_secs(timeout_secs)),
            None,
            None,
        )
        .ok()?;
    if result.stop_reason == build_core::ProcessStopReason::Completed
        && result.termination
            == (build_core::ProcessTermination::Exited { code: 0 })
    {
        Some(result.standard_output)
    } else {
        None
    }
}

/// `compare(_:options:.numeric)` — version-directory ordering where digit
/// runs compare numerically ("v9" < "v18").
fn numeric_compare(a: &str, b: &str) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    let (mut ac, mut bc) = (a.chars().peekable(), b.chars().peekable());
    loop {
        match (ac.peek().copied(), bc.peek().copied()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(x), Some(y)) if x.is_ascii_digit() && y.is_ascii_digit() => {
                let take = |it: &mut std::iter::Peekable<std::str::Chars>| -> u64 {
                    let mut v = String::new();
                    while let Some(c) = it.peek() {
                        if !c.is_ascii_digit() {
                            break;
                        }
                        v.push(*c);
                        it.next();
                    }
                    v.parse().unwrap_or(0)
                };
                let (an, bn) = (take(&mut ac), take(&mut bc));
                if an != bn {
                    return an.cmp(&bn);
                }
            }
            (Some(x), Some(y)) => {
                if x != y {
                    return x.cmp(&y);
                }
                ac.next();
                bc.next();
            }
        }
    }
}

/// `expandingTildeInPath` — only a leading `~` expands.
fn expand_tilde(value: &str, home: &Path) -> String {
    if value == "~" {
        return home.to_string_lossy().into_owned();
    }
    if let Some(rest) = value.strip_prefix("~/") {
        return home.join(rest).to_string_lossy().into_owned();
    }
    value.to_string()
}

// ─── PiRPC (verbatim wire protocol) ─────────────────────────────────────────

/// Outbound commands — identical `type` field values.
#[derive(Debug, Clone)]
pub enum PiRPCCommand {
    Prompt {
        message: String,
        streaming_behavior: Option<String>,
    },
    Steer {
        message: String,
    },
    Abort,
    NewSession,
    GetState,
    GetAvailableModels,
    GetAvailableThinkingLevels,
    GetSessionStats,
    GetCommands,
    SetModel {
        provider: String,
        model_id: String,
    },
    SetThinkingLevel {
        level: String,
    },
    ExtensionUICancelled {
        id: String,
    },
    SwitchSession {
        path: String,
    },
    GetMessages,
}
impl PiRPCCommand {
    pub fn object(&self) -> Value {
        match self {
            Self::Prompt {
                message,
                streaming_behavior,
            } => {
                let mut o = json!({"type": "prompt", "message": message});
                if let Some(b) = streaming_behavior {
                    o["streamingBehavior"] = json!(b);
                }
                o
            }
            Self::Steer { message } => json!({"type": "steer", "message": message}),
            Self::Abort => json!({"type": "abort"}),
            Self::NewSession => json!({"type": "new_session"}),
            Self::SwitchSession { path } => json!({"type": "switch_session", "sessionPath": path}),
            Self::GetMessages => json!({"type": "get_messages"}),
            Self::GetState => json!({"type": "get_state"}),
            Self::GetAvailableModels => json!({"type": "get_available_models"}),
            Self::GetAvailableThinkingLevels => {
                json!({"type": "get_available_thinking_levels"})
            }
            Self::GetSessionStats => json!({"type": "get_session_stats"}),
            Self::GetCommands => json!({"type": "get_commands"}),
            Self::SetModel { provider, model_id } => {
                json!({"type": "set_model", "provider": provider, "modelId": model_id})
            }
            Self::SetThinkingLevel { level } => {
                json!({"type": "set_thinking_level", "level": level})
            }
            Self::ExtensionUICancelled { id } => {
                json!({"type": "extension_ui_response", "id": id, "cancelled": true})
            }
        }
    }
}

/// A decoded stdout line.
#[derive(Debug, Clone)]
pub struct PiRPCEvent {
    pub event_type: String,
    pub object: Map<String, Value>,
}
impl PiRPCEvent {
    pub fn new(object: Map<String, Value>) -> Option<Self> {
        let t = object.get("type")?.as_str()?.to_string();
        Some(Self {
            event_type: t,
            object,
        })
    }
    pub fn string(&self, key: &str) -> Option<String> {
        self.object.get(key)?.as_str().map(String::from)
    }
    pub fn bool(&self, key: &str) -> bool {
        self.object.get(key).and_then(|v| v.as_bool()).unwrap_or(false)
    }
    pub fn int(&self, key: &str) -> Option<i64> {
        self.object.get(key)?.as_i64()
    }
    pub fn nested(&self, key: &str) -> Option<&Map<String, Value>> {
        self.object.get(key)?.as_object()
    }
    pub fn array(&self, key: &str) -> Option<&Vec<Value>> {
        self.object.get(key)?.as_array()
    }
    pub fn is_response(&self) -> bool {
        self.event_type == "response"
    }
    pub fn response_succeeded(&self) -> bool {
        self.bool("success")
    }
    pub fn response_error(&self) -> Option<String> {
        self.string("error")
    }
    pub fn response_command(&self) -> Option<String> {
        self.string("command")
    }
    pub fn response_data(&self) -> Option<&Map<String, Value>> {
        self.nested("data")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PiModelDescriptor {
    pub id: String,
    pub provider: String,
    pub name: String,
    /// `contextWindow` from the model JSON — optional upstream.
    pub context_window: Option<u64>,
}
impl PiModelDescriptor {
    pub fn from_value(v: &Value) -> Option<Self> {
        let d = v.as_object()?;
        Some(Self {
            id: d.get("id")?.as_str()?.to_string(),
            provider: d.get("provider")?.as_str()?.to_string(),
            name: d
                .get("name")
                .and_then(|n| n.as_str())
                .map(String::from)
                .unwrap_or_else(|| d.get("id").unwrap().as_str().unwrap().to_string()),
            context_window: d.get("contextWindow").and_then(|v| v.as_u64()),
        })
    }
    pub fn picker_title(&self) -> String {
        if self.name == self.id {
            self.id.clone()
        } else {
            format!("{} ({})", self.name, self.id)
        }
    }
}

/// `get_session_stats` response payload — token totals, cost, and the
/// context-window occupancy pi reports for the current session.
#[derive(Debug, Clone, Default)]
pub struct PiSessionStats {
    pub total_tokens: u64,
    pub cost: f64,
    pub context_tokens: Option<u64>,
    pub context_window: Option<u64>,
    pub context_percent: Option<f64>,
}
impl PiSessionStats {
    pub fn from_value(d: &Map<String, Value>) -> Self {
        let tokens = d.get("tokens").and_then(|t| t.as_object());
        let usage = d.get("contextUsage").and_then(|u| u.as_object());
        Self {
            total_tokens: tokens
                .and_then(|t| t.get("total"))
                .and_then(|v| v.as_u64())
                .unwrap_or(0),
            cost: d.get("cost").and_then(|v| v.as_f64()).unwrap_or(0.0),
            context_tokens: usage
                .and_then(|u| u.get("tokens"))
                .and_then(|v| v.as_u64()),
            context_window: usage
                .and_then(|u| u.get("contextWindow"))
                .and_then(|v| v.as_u64()),
            context_percent: usage
                .and_then(|u| u.get("percent"))
                .and_then(|v| v.as_f64()),
        }
    }
}

/// One entry of the `get_commands` response — a slash command the composer
/// can complete (`source`: "extension" | "prompt" | "skill").
#[derive(Debug, Clone)]
pub struct PiSlashCommand {
    pub name: String,
    pub description: Option<String>,
    pub source: String,
}
impl PiSlashCommand {
    pub fn from_value(v: &Value) -> Option<Self> {
        let d = v.as_object()?;
        Some(Self {
            name: d.get("name")?.as_str()?.to_string(),
            description: d
                .get("description")
                .and_then(|x| x.as_str())
                .map(String::from),
            source: d
                .get("source")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string(),
        })
    }
}

// ─── PiAgentProcess (verbatim I/O model) ────────────────────────────────────

/// One running `pi --mode rpc` subprocess. Commands encode as JSONL on
/// stdin; stdout lines decode into events on the returned receiver. The
/// stream ends when the process exits.
pub struct PiAgentProcess {
    child: Arc<Mutex<Child>>,
    stdin: Arc<Mutex<std::process::ChildStdin>>,
    stderr_tail: Arc<Mutex<Vec<u8>>>,
    /// Shared receivers — the UI drains them via `try_recv`.
    pub events: Arc<Mutex<Receiver<PiRPCEvent>>>,
    /// Fires once when the process exits (sent from the reader thread).
    pub exited: Arc<Mutex<Receiver<()>>>,
    _stdout_thread: std::thread::JoinHandle<()>,
    _stderr_thread: std::thread::JoinHandle<()>,
    _exit_thread: std::thread::JoinHandle<()>,
}
impl PiAgentProcess {
    pub fn start(
        executable: &Path,
        working_directory: &Path,
        arguments: &[String],
        environment: &HashMap<String, String>,
    ) -> std::io::Result<Self> {
        let mut command = Command::new(executable);
        command
            .args(arguments)
            .current_dir(working_directory)
            .env_clear()
            .envs(environment)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        // The agent is a background RPC peer — no console window.
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
        }
        let mut child = command.spawn()?;
        let stdin = child.stdin.take().expect("piped stdin");
        let stdout = child.stdout.take().expect("piped stdout");
        let stderr = child.stderr.take().expect("piped stderr");

        let (event_tx, event_rx) = channel::<PiRPCEvent>();
        let (exit_tx, exit_rx) = channel::<()>();
        let stderr_tail = Arc::new(Mutex::new(Vec::<u8>::new()));

        let stdout_thread = std::thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            loop {
                // Read one LF-delimited line without splitting UTF-8.
                let mut line = Vec::<u8>::new();
                let mut byte = [0u8; 1];
                loop {
                    match reader.read(&mut byte) {
                        Ok(0) | Err(_) => break,
                        Ok(_) => {
                            if byte[0] == b'\n' {
                                break;
                            }
                            line.push(byte[0]);
                        }
                    }
                    if line.len() > 4 * 1024 * 1024 {
                        break;
                    }
                }
                if line.last() == Some(&b'\r') {
                    line.pop();
                }
                if line.is_empty() {
                    if reader.buffer().is_empty() && reader.fill_buf().map(|b| b.is_empty()).unwrap_or(true) {
                        break;
                    }
                    continue;
                }
                if let Ok(value) = serde_json::from_slice::<Value>(&line) {
                    if let Some(object) = value.as_object() {
                        if let Some(event) = PiRPCEvent::new(object.clone()) {
                            if event_tx.send(event).is_err() {
                                break;
                            }
                            crate::app_ui::wake_agent();
                        }
                    }
                }
            }
        });

        let tail = stderr_tail.clone();
        let stderr_thread = std::thread::spawn(move || {
            let mut reader = BufReader::new(stderr);
            let mut chunk = [0u8; 4096];
            while let Ok(n) = reader.read(&mut chunk) {
                if n == 0 {
                    break;
                }
                let mut guard = tail.lock().unwrap();
                guard.extend_from_slice(&chunk[..n]);
                if guard.len() > 65_536 {
                    let drop = guard.len() - 65_536;
                    guard.drain(..drop);
                }
            }
        });

        let child_arc = Arc::new(Mutex::new(child));
        let exit_child = child_arc.clone();
        let exit_thread = std::thread::spawn(move || {
            // Wait in the OS without holding the Child mutex used by send/kill.
            // WNOWAIT leaves reaping to Child, which retains the exit status.
            #[cfg(unix)]
            {
                let pid = exit_child.lock().unwrap().id();
                let mut info: libc::siginfo_t = unsafe { std::mem::zeroed() };
                loop {
                    let result = unsafe { libc::waitid(libc::P_PID, pid, &mut info, libc::WEXITED | libc::WNOWAIT) };
                    if result == 0 || std::io::Error::last_os_error().kind() != std::io::ErrorKind::Interrupted { break; }
                }
            }
            #[cfg(windows)]
            {
                use std::os::windows::io::AsRawHandle;
                #[link(name = "kernel32")]
                extern "system" { fn WaitForSingleObject(handle: *mut std::ffi::c_void, milliseconds: u32) -> u32; }
                let handle = exit_child.lock().unwrap().as_raw_handle();
                unsafe { WaitForSingleObject(handle, u32::MAX); }
            }
            let _ = exit_child.lock().unwrap().try_wait();
            let _ = exit_tx.send(());
            crate::app_ui::wake_agent();
        });

        Ok(Self {
            child: child_arc,
            stdin: Arc::new(Mutex::new(stdin)),
            stderr_tail,
            events: Arc::new(Mutex::new(event_rx)),
            exited: Arc::new(Mutex::new(exit_rx)),
            _stdout_thread: stdout_thread,
            _stderr_thread: stderr_thread,
            _exit_thread: exit_thread,
        })
    }

    /// `send(_:id:)` — the optional id is echoed back on pi's response so the
    /// coordinator can drop replies to superseded model-settings requests.
    pub fn send(&self, command: &PiRPCCommand, id: Option<&str>) -> Result<(), String> {
        if !self.is_running() {
            return Err("The pi agent process is not running.".into());
        }
        let mut object = command.object();
        if let Some(id) = id {
            object["id"] = json!(id);
        }
        let mut data = serde_json::to_vec(&object).map_err(|e| e.to_string())?;
        data.push(b'\n');
        let mut stdin = self.stdin.lock().unwrap();
        stdin
            .write_all(&data)
            .and_then(|_| stdin.flush())
            .map_err(|e| e.to_string())
    }

    pub fn is_running(&self) -> bool {
        self.child
            .lock()
            .unwrap()
            .try_wait()
            .map(|s| s.is_none())
            .unwrap_or(false)
    }

    pub fn stderr_text(&self) -> String {
        String::from_utf8_lossy(&self.stderr_tail.lock().unwrap()).into_owned()
    }

    pub fn terminate(&self) {
        let mut guard = self.child.lock().unwrap();
        let _ = guard.kill();
    }
}

// ─── AgentCoordinator (verbatim state machine) ──────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum Connection {
    Idle,
    Connecting,
    PiMissing,
    Ready,
    Failed(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TranscriptRole {
    User,
    Assistant,
    Thinking,
    Tool,
    Notice,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TranscriptStatus {
    Streaming,
    Running,
    Done,
    Failed,
}
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AgentTranscriptEntry {
    pub role: TranscriptRole,
    pub title: String,
    pub text: String,
    pub detail: String,
    pub status: TranscriptStatus,
}

/// A saved conversation for the current project — one pi session file.
#[derive(Debug, Clone, PartialEq)]
pub struct AgentSessionSummary {
    pub path: PathBuf,
    pub modified: std::time::SystemTime,
    pub title: String,
}

/// Each project's conversations live in their own folder (`--session-dir`),
/// so the history list only shows this project's chats.
pub fn session_directory(root: &Path) -> PathBuf {
    let name: String = root
        .to_string_lossy()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect();
    pi_paths::agent_directory().join("pitex-sessions").join(name)
}

/// A user message as typed: prose prompts carry the `<editor-context>`
/// envelope in front (see `envelope`).
fn user_prompt_text(message: &Map<String, Value>) -> String {
    let raw = match message.get("content") {
        Some(Value::String(text)) => text.clone(),
        Some(Value::Array(blocks)) => blocks
            .iter()
            .filter_map(|b| b.get("text").and_then(|t| t.as_str()))
            .collect::<Vec<_>>()
            .join("\n"),
        _ => String::new(),
    };
    match raw.strip_prefix("<editor-context>").and_then(|r| r.split_once("\n</editor-context>\n")) {
        Some((_, prompt)) => prompt.trim().to_string(),
        None => raw.trim().to_string(),
    }
}

/// Title = the first prompt as typed; sessions without one are skipped.
/// Only the head of the file is read — later tool output can be large.
fn session_summary(path: &Path) -> Option<AgentSessionSummary> {
    let mut head = Vec::new();
    std::fs::File::open(path).ok()?.take(512 * 1024).read_to_end(&mut head).ok()?;
    let head = String::from_utf8_lossy(&head);
    let title = head.lines().find_map(|line| {
        let entry: Value = serde_json::from_str(line).ok()?;
        let message = entry.get("message")?.as_object()?;
        (entry.get("type")?.as_str()? == "message" && message.get("role")?.as_str()? == "user")
            .then(|| user_prompt_text(message))
    })?;
    if title.is_empty() {
        return None;
    }
    Some(AgentSessionSummary {
        path: path.to_path_buf(),
        modified: std::fs::metadata(path).and_then(|m| m.modified()).ok()?,
        title: title.chars().take(120).collect(),
    })
}

#[derive(Debug, Clone, Default)]
pub struct AgentContextSnapshot {
    pub project_root: Option<PathBuf>,
    pub active_path: Option<String>,
    pub active_text: Option<String>,
    pub selection_text: Option<String>,
    pub project_files: Vec<String>,
    pub pdf_path: Option<String>,
    pub pdf_data: Option<Arc<[u8]>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentSelectionAttachment {
    pub path: String,
    pub start_line: usize,
    pub end_line: usize,
    pub text: String,
}

/// The coordinator is main-thread-owned like the Swift `@MainActor` class.
/// The process's event receiver is drained by the UI's idle loop.
pub struct AgentCoordinator {
    pub connection: Connection,
    pub transcript: Vec<AgentTranscriptEntry>,
    /// The history list (`/resume` or the clock button), newest first.
    pub past_sessions: Vec<AgentSessionSummary>,
    /// `/resume` was typed — the panel opens the history popover.
    pub history_requested: bool,
    /// Where pi saves this project's sessions (`--session-dir`).
    session_directory: Option<PathBuf>,
    pub preferred_model_id: Option<String>,
    pub history_limit: usize,
    pub is_running: bool,
    pub models: Vec<PiModelDescriptor>,
    pub current_model: Option<PiModelDescriptor>,
    pub thinking_level: String,
    /// Both the effective level and the model's choices come from pi:
    /// `get_available_thinking_levels` for the active model; empty while a
    /// model-settings refresh is in flight.
    pub thinking_levels: Vec<String>,
    pub is_updating_model_settings: bool,
    /// `get_session_stats` payload — refreshed after spawn and on every
    /// `agent_end`; `None` until the first reply lands.
    pub session_stats: Option<PiSessionStats>,
    /// `get_commands` payload — slash commands the composer completes.
    pub commands: Vec<PiSlashCommand>,
    pub status_message: Option<String>,
    pub attach_active_document: bool,
    pub pending_composer_insertion: Option<String>,
    pub selection_attachment: Option<AgentSelectionAttachment>,
    suppressed_attachment: Option<AgentSelectionAttachment>,
    /// Tracks the in-flight model-settings request; replies echoing a
    /// different id are stale and ignored. Public for integration tests.
    #[doc(hidden)]
    pub model_settings_request_id: Option<String>,
    /// Bumped on every mutation of UI-visible state; the panel rebuilds
    /// only when this changes instead of on every agent event.
    pub ui_revision: u64,
    /// Set by the first `prepare()` — pi spawns lazily when the Assistant
    /// is first shown, and restart/install hooks must not start it for a
    /// window whose Assistant was never opened (macOS `wantsConnection`).
    pub wants_connection: bool,
    pdf_text_cache: std::cell::RefCell<Option<(Arc<[u8]>, Option<String>)>>,

    /// Workspace hooks.
    pub context_provider: Option<Box<dyn Fn() -> AgentContextSnapshot>>,
    pub persist_dirty_sessions: Option<Box<dyn FnMut() -> Option<String>>>,
    /// Fired when a run finishes so the workspace adopts agent edits.
    pub on_agent_activity_finished: Option<Box<dyn FnMut()>>,

    process: Option<PiAgentProcess>,
    intentional_stop: bool,
    /// `prepareTask` equivalent — the in-flight toolchain discovery; the
    /// result arrives through event dispatch via `poll_toolchain`.
    toolchain_rx: Option<std::sync::mpsc::Receiver<(PiToolchain, Option<PathBuf>)>>,
    pending_root: Option<PathBuf>,
    assistant_entry_index: Option<usize>,
    thinking_entry_index: Option<usize>,
    text_content_index: Option<i64>,
    thinking_content_index: Option<i64>,
    tool_index_by_call_id: HashMap<String, usize>,
}

impl AgentCoordinator {
    const ATTACH_DOCUMENT_KEY: &'static str = "ai.attachActiveDocument";
    /// Level choices are reported per model by pi via
    /// `get_available_thinking_levels` — no fixed universe is needed here.

    pub fn new(store: &crate::settings::SettingsStore) -> Self {
        Self {
            connection: Connection::Idle,
            transcript: Vec::new(),
            past_sessions: Vec::new(),
            history_requested: false,
            session_directory: None,
            preferred_model_id: None,
            history_limit: 50,
            is_running: false,
            models: Vec::new(),
            current_model: None,
            thinking_level: "off".into(),
            thinking_levels: Vec::new(),
            is_updating_model_settings: false,
            status_message: None,
            attach_active_document: store
                .prefs()
                .bool(Self::ATTACH_DOCUMENT_KEY)
                .unwrap_or(true),
            pending_composer_insertion: None,
            selection_attachment: None,
            suppressed_attachment: None,
            model_settings_request_id: None,
            ui_revision: 0,
            wants_connection: false,
            pdf_text_cache: std::cell::RefCell::new(None),
            session_stats: None,
            commands: Vec::new(),
            context_provider: None,
            persist_dirty_sessions: None,
            on_agent_activity_finished: None,
            process: None,
            intentional_stop: false,
            toolchain_rx: None,
            pending_root: None,
            assistant_entry_index: None,
            thinking_entry_index: None,
            text_content_index: None,
            thinking_content_index: None,
            tool_index_by_call_id: HashMap::new(),
        }
    }

    pub fn set_attach_active_document(
        &mut self,
        store: &mut crate::settings::SettingsStore,
        value: bool,
    ) {
        self.attach_active_document = value;
        store.prefs_mut().set(Self::ATTACH_DOCUMENT_KEY, value);
        self.ui_revision += 1;
    }

    /// `updateSelectionAttachment` — verbatim suppression logic.
    pub fn update_selection_attachment(&mut self, attachment: Option<AgentSelectionAttachment>) {
        if attachment == self.suppressed_attachment {
            return;
        }
        // Unchanged (every caret move re-reports it): bumping ui_revision
        // re-hashed the whole transcript and scrolled it to the bottom.
        if self.suppressed_attachment.is_none() && attachment == self.selection_attachment {
            return;
        }
        self.suppressed_attachment = None;
        self.selection_attachment = attachment;
        self.ui_revision += 1;
    }
    pub fn clear_selection_attachment(&mut self) {
        self.suppressed_attachment = self.selection_attachment.take();
        self.ui_revision += 1;
    }

    pub fn insert_into_composer(&mut self, text: &str) {
        self.pending_composer_insertion = Some(text.to_string());
        self.ui_revision += 1;
    }

    /// Idempotent startup — `prepare()`. Returns the spawn error if the
    /// process couldn't start; the UI calls this when the panel appears.
    pub fn prepare(&mut self) {
        self.wants_connection = true;
        if matches!(self.connection, Connection::Connecting | Connection::Ready) {
            return;
        }
        let Some(root) = self
            .context_provider
            .as_ref()
            .and_then(|p| (p)().project_root)
        else {
            return;
        };
        if self.toolchain_rx.is_some() {
            // Discovery already in flight (`prepareTask` not yet awaited).
            return;
        }
        let (tx, rx) = std::sync::mpsc::channel();
        self.toolchain_rx = Some(rx);
        self.pending_root = Some(root);
        // `PiToolchain.discover()` blocks on shell probes — keep it off the
        // UI thread like the Swift `await` in `prepareAgent`. `Connecting` is
        // set only inside `spawn` so a cancelled discovery cannot wedge the
        // connection state.
        std::thread::spawn(move || {
            let tools = PiToolchain::discover(
                std::env::vars().collect(),
                &dirs::home_dir().unwrap_or_default(),
                &PiToolchain::SYSTEM_DIRECTORIES,
            );
            let executable = locate_pi_executable_in(&tools.environment);
            let _ = tx.send((tools, executable));
            crate::app_ui::wake_agent();
        });
    }

    /// Delivered on the UI thread when discovery completes — the awaited half of `prepareAgent`.
    /// `guard !Task.isCancelled` maps to the intentional-stop check.
    pub fn poll_toolchain(&mut self) {
        let Some(rx) = &self.toolchain_rx else { return };
        let Ok((tools, executable)) = rx.try_recv() else { return };
        self.toolchain_rx = None;
        let Some(root) = self.pending_root.take() else { return };
        if self.intentional_stop {
            return;
        }
        match executable {
            Some(executable) => self.spawn_with_tools(&executable, &root, tools),
            None => {
                self.connection = Connection::PiMissing;
                self.status_message = None;
            }
        }
        self.ui_revision += 1;
    }

    fn spawn_with_tools(&mut self, executable: &Path, root: &Path, tools: PiToolchain) {
        self.intentional_stop = false;
        self.connection = Connection::Connecting;
        // Every launch starts a new saved session in this project's folder,
        // so past conversations can be resumed after a restart.
        let sessions = session_directory(root);
        let _ = std::fs::create_dir_all(&sessions);
        self.session_directory = Some(sessions.clone());
        let arguments = vec![
            "--mode".into(),
            "rpc".into(),
            "--session-dir".into(),
            sessions.to_string_lossy().into_owned(),
            "--append-system-prompt".into(),
            Self::system_prompt_supplement().to_string(),
        ];
        let launch = match tools.launch(executable, &arguments) {
            Ok(launch) => launch,
            Err(e) => {
                self.connection = Connection::Failed(e);
                return;
            }
        };
        let environment = Self::child_environment(&launch.0, &tools.environment);
        match PiAgentProcess::start(&launch.0, root, &launch.1, &environment) {
            Ok(agent) => {
                self.process = Some(agent);
                self.connection = Connection::Ready;
                self.send_model_settings(PiRPCCommand::GetState, None);
                if let Some(process) = &self.process {
                    self.send_to(process, &PiRPCCommand::GetAvailableModels);
                    self.send_to(process, &PiRPCCommand::GetSessionStats);
                    self.send_to(process, &PiRPCCommand::GetCommands);
                }
            }
            Err(e) => {
                self.connection = Connection::Failed(e.to_string());
            }
        }
    }

    /// Called by the UI when the process's `exited` channel fires.
    pub fn process_did_exit(&mut self) {
        self.process = None;
        self.is_running = false;
        self.toolchain_rx = None;
        self.pending_root = None;
        self.model_settings_request_id = None;
        self.thinking_levels.clear();
        self.is_updating_model_settings = false;
        if !self.intentional_stop {
            self.connection = Connection::Failed("The agent process exited unexpectedly.".into());
        }
        self.ui_revision += 1;
    }

    pub fn shutdown(&mut self) {
        self.intentional_stop = true;
        if let Some(process) = self.process.take() {
            process.terminate();
        }
        self.toolchain_rx = None;
        self.pending_root = None;
        self.model_settings_request_id = None;
        self.thinking_levels.clear();
        self.is_updating_model_settings = false;
        self.ui_revision += 1;
    }

    /// Restart after provider/config changes — `shutdown` leaves
    /// `connection` at `Ready`, which would make `prepare` early-return, so
    /// the state is reset to `Idle` first.
    pub fn restart(&mut self) {
        self.shutdown();
        self.connection = Connection::Idle;
        self.intentional_stop = false;
        self.session_stats = None;
        self.commands.clear();
        if self.wants_connection {
            self.prepare();
        }
        self.ui_revision += 1;
    }

    /// Poll one event (non-blocking) — the UI drains in its idle loop.
    pub fn poll_event(&self) -> Option<PiRPCEvent> {
        self.process
            .as_ref()
            .and_then(|p| p.events.lock().unwrap().try_recv().ok())
    }
    /// True once when the process exit signal fires.
    pub fn poll_exit(&self) -> bool {
        self.process
            .as_ref()
            .map(|p| p.exited.lock().unwrap().try_recv().is_ok())
            .unwrap_or(false)
    }

    /// `send(prompt:)` — persist dirty sessions first, then envelope+prompt.
    /// Returns an error notice to append, or sends the prompt.
    pub fn send(&mut self, prompt: &str) {
        let trimmed = prompt.trim();
        if trimmed.is_empty() {
            return;
        }
        if self.connection == Connection::PiMissing {
            self.status_message = Some(
                "Pitex Agent is not installed yet — it installs automatically on launch; see Settings → AI.".into(),
            );
            return;
        }
        // pi's own /resume is an interactive TUI picker; here it opens the
        // panel's history list instead.
        if trimmed == "/resume" {
            self.load_past_sessions();
            self.history_requested = true;
            self.ui_revision += 1;
            return;
        }
        self.push_entry(AgentTranscriptEntry {
            role: TranscriptRole::User,
            title: "You".into(),
            text: trimmed.to_string(),
            detail: String::new(),
            status: TranscriptStatus::Done,
        });
        self.status_message = None;
        if let Some(persist) = &mut self.persist_dirty_sessions {
            if let Some(problem) = (persist)() {
                self.push_entry(AgentTranscriptEntry {
                    role: TranscriptRole::Notice,
                    title: "Assistant".into(),
                    text: problem,
                    detail: String::new(),
                    status: TranscriptStatus::Failed,
                });
                return;
            }
        }
        if self.process.is_none() {
            self.prepare();
        }
        let Some(process) = &self.process else {
            if self.connection == Connection::Idle {
                self.connection = Connection::Connecting;
            }
            self.status_message = Some("The agent is not running.".into());
            return;
        };
        if !process.is_running() {
            self.status_message = Some("The agent is not running.".into());
            return;
        }
        // Slash commands go raw — pi expands `/skill:name` and extension
        // commands only when the message itself starts with '/'.
        let message = if trimmed.starts_with('/') {
            trimmed.to_string()
        } else {
            self.envelope(trimmed)
        };
        let behavior = if self.is_running {
            Some("followUp".to_string())
        } else {
            None
        };
        if let Err(e) = process.send(
            &PiRPCCommand::Prompt {
                message,
                streaming_behavior: behavior,
            },
            None,
        ) {
            self.status_message = Some(format!("The prompt could not be sent: {e}"));
        }
        self.ui_revision += 1;
    }

    pub fn stop(&mut self) {
        if !self.is_running {
            return;
        }
        if let Some(process) = &self.process {
            let _ = process.send(&PiRPCCommand::Abort, None);
        }
        self.ui_revision += 1;
    }
    pub fn new_session(&mut self) {
        if !self.process.as_ref().is_some_and(|p| p.is_running()) {
            return;
        }
        self.clear_transcript();
        if let Some(process) = &self.process {
            let _ = process.send(&PiRPCCommand::NewSession, None);
        }
        self.ui_revision += 1;
    }

    fn clear_transcript(&mut self) {
        self.transcript.clear();
        self.tool_index_by_call_id.clear();
        self.assistant_entry_index = None;
        self.thinking_entry_index = None;
    }

    pub fn load_past_sessions(&mut self) {
        let files = self
            .session_directory
            .as_deref()
            .and_then(|d| std::fs::read_dir(d).ok())
            .into_iter()
            .flatten()
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|x| x == "jsonl"));
        self.past_sessions = files.filter_map(|p| session_summary(&p)).collect();
        self.past_sessions.sort_by(|a, b| b.modified.cmp(&a.modified));
    }

    /// Loads a saved session into pi, then redraws the transcript from its
    /// messages (`switch_session` → `get_messages`).
    pub fn resume(&mut self, path: &Path) {
        if self.is_running {
            return;
        }
        let Some(process) = &self.process else { return };
        if !process.is_running() {
            return;
        }
        let command = PiRPCCommand::SwitchSession { path: path.to_string_lossy().into_owned() };
        if process.send(&command, None).is_ok() {
            self.clear_transcript();
        }
        self.ui_revision += 1;
    }

    /// The same row shapes the live event stream produces.
    fn transcript_from_messages(&self, messages: &[Value]) -> Vec<AgentTranscriptEntry> {
        let entry = |role, title: &str, text: String, detail: String| AgentTranscriptEntry {
            role,
            title: title.to_string(),
            text,
            detail,
            status: TranscriptStatus::Done,
        };
        let title = self.assistant_title();
        let mut entries: Vec<AgentTranscriptEntry> = Vec::new();
        let mut tool_rows: HashMap<String, usize> = HashMap::new();
        for message in messages.iter().filter_map(|m| m.as_object()) {
            match message.get("role").and_then(|r| r.as_str()) {
                Some("user") => {
                    let text = user_prompt_text(message);
                    if !text.is_empty() {
                        entries.push(entry(TranscriptRole::User, "You", text, String::new()));
                    }
                }
                Some("assistant") => {
                    let blocks = message.get("content").and_then(|c| c.as_array());
                    for block in blocks.into_iter().flatten() {
                        let text = |key: &str| block.get(key).and_then(|t| t.as_str()).unwrap_or("").to_string();
                        match block.get("type").and_then(|t| t.as_str()) {
                            Some("thinking") if !text("thinking").is_empty() => {
                                entries.push(entry(TranscriptRole::Thinking, "Thinking", text("thinking"), String::new()));
                            }
                            Some("text") if !text("text").is_empty() => {
                                entries.push(entry(TranscriptRole::Assistant, &title, text("text"), String::new()));
                            }
                            Some("toolCall") => {
                                let name = block.get("name").and_then(|n| n.as_str()).unwrap_or("tool");
                                let summary = tool_summary(name, block.get("arguments").and_then(|a| a.as_object()));
                                entries.push(entry(TranscriptRole::Tool, name, String::new(), summary));
                                tool_rows.insert(text("id"), entries.len() - 1);
                            }
                            _ => {}
                        }
                    }
                }
                Some("toolResult") => {
                    let id = message.get("toolCallId").and_then(|i| i.as_str()).unwrap_or("");
                    let Some(&index) = tool_rows.get(id) else { continue };
                    if message.get("isError").and_then(|e| e.as_bool()) == Some(true) {
                        entries[index].status = TranscriptStatus::Failed;
                    }
                    if let Some(text) = tool_result_text(Some(message)) {
                        entries[index].detail = bounded(&text, 4_000);
                    }
                }
                _ => {}
            }
        }
        let keep = entries.len().saturating_sub(self.history_limit);
        entries.drain(..keep);
        entries
    }
    pub fn select_model(&mut self, model: &PiModelDescriptor) {
        if self.is_updating_model_settings || self.current_model.as_ref() == Some(model) {
            return;
        }
        self.send_model_settings(
            PiRPCCommand::SetModel {
                provider: model.provider.clone(),
                model_id: model.id.clone(),
            },
            None,
        );
        self.ui_revision += 1;
    }

    /// Reasoning-effort levels the *current model* accepts — reported by pi's
    /// `get_available_thinking_levels`; empty while a model-settings refresh
    /// is in flight.
    pub fn select_thinking_level(&mut self, level: &str) {
        if self.is_updating_model_settings
            || level == self.thinking_level
            || !self.thinking_levels.iter().any(|l| l == level)
        {
            return;
        }
        self.send_model_settings(
            PiRPCCommand::SetThinkingLevel {
                level: level.to_string(),
            },
            None,
        );
        self.ui_revision += 1;
    }

    /// Read state after each change, then query capabilities for that state.
    /// Request IDs prevent replies from an earlier refresh restoring old choices.
    fn send_model_settings(&mut self, command: PiRPCCommand, id: Option<String>) {
        let Some(process) = &self.process else { return };
        if !process.is_running() {
            return;
        }
        let request_id = id.unwrap_or_else(uuid_v4);
        self.model_settings_request_id = Some(request_id.clone());
        self.is_updating_model_settings = true;
        self.thinking_levels.clear();
        if let Err(e) = process.send(&command, Some(&request_id)) {
            self.model_settings_request_id = None;
            self.is_updating_model_settings = false;
            self.status_message = Some(e);
        }
    }

    fn apply_preferred_model_if_ready(&mut self) {
        if self.is_updating_model_settings {
            return;
        }
        let Some(preferred) = self.preferred_model_id.clone() else { return };
        let Some(model) = self.models.iter().find(|m| m.id == preferred).cloned() else {
            return;
        };
        self.preferred_model_id = None;
        if self.current_model.as_ref() != Some(&model) {
            self.select_model(&model);
        }
    }

    fn send_to(&self, process: &PiAgentProcess, command: &PiRPCCommand) {
        let _ = process.send(command, None);
    }

    /// `handle(_:)` — the full event switch, verbatim.
    pub fn handle(&mut self, event: &PiRPCEvent) {
        self.ui_revision += 1;
        match event.event_type.as_str() {
            "response" => self.handle_response(event),
            "agent_start" => self.is_running = true,
            "agent_end" => {
                if let Some(process) = &self.process {
                    self.send_to(process, &PiRPCCommand::GetSessionStats);
                }
                self.is_running = false;
                self.finalize_entries();
                if let Some(failure) = self.last_assistant_error(event) {
                    self.push_entry(AgentTranscriptEntry {
                        role: TranscriptRole::Notice,
                        title: "Assistant".into(),
                        text: failure,
                        detail: String::new(),
                        status: TranscriptStatus::Failed,
                    });
                }
                if let Some(cb) = &mut self.on_agent_activity_finished {
                    cb();
                }
            }
            "message_start" => self.handle_message_start(event),
            "message_update" => self.handle_message_update(event),
            "message_end" => self.handle_message_end(event),
            "turn_end" => self.finalize_entries(),
            "tool_execution_start" => self.handle_tool_start(event),
            "tool_execution_update" => self.handle_tool_update(event),
            "tool_execution_end" => self.handle_tool_end(event),
            "auto_retry_start" => {
                let attempt = event.int("attempt").unwrap_or(1);
                let max_attempts = event.int("maxAttempts").unwrap_or(1);
                let reason = event.string("errorMessage").unwrap_or_else(|| "transient error".into());
                self.push_entry(AgentTranscriptEntry {
                    role: TranscriptRole::Notice,
                    title: "Assistant".into(),
                    text: format!("Retrying ({attempt}/{max_attempts}): {reason}"),
                    detail: String::new(),
                    status: TranscriptStatus::Done,
                });
            }
            "auto_retry_end" => {
                if !event.bool("success") {
                    if let Some(final_error) = event.string("finalError") {
                        self.push_entry(AgentTranscriptEntry {
                            role: TranscriptRole::Notice,
                            title: "Assistant".into(),
                            text: format!("The request failed after retrying: {final_error}"),
                            detail: String::new(),
                            status: TranscriptStatus::Failed,
                        });
                    }
                }
            }
            "compaction_start" => self.push_entry(AgentTranscriptEntry {
                role: TranscriptRole::Notice,
                title: "Assistant".into(),
                text: "Compacting the conversation…".into(),
                detail: String::new(),
                status: TranscriptStatus::Done,
            }),
            "compaction_end" => {
                if event.bool("aborted") {
                    self.push_entry(AgentTranscriptEntry {
                        role: TranscriptRole::Notice,
                        title: "Assistant".into(),
                        text: "Compaction cancelled.".into(),
                        detail: String::new(),
                        status: TranscriptStatus::Done,
                    });
                } else if let Some(message) = event.string("errorMessage") {
                    self.push_entry(AgentTranscriptEntry {
                        role: TranscriptRole::Notice,
                        title: "Assistant".into(),
                        text: format!("Compaction failed: {message}"),
                        detail: String::new(),
                        status: TranscriptStatus::Failed,
                    });
                }
            }
            "thinking_level_changed" => {
                if !self.is_updating_model_settings {
                    self.send_model_settings(PiRPCCommand::GetState, None);
                }
            }
            "extension_ui_request" => self.handle_extension_ui(event),
            "extension_error" => {
                let message = event.string("error").unwrap_or_else(|| "extension error".into());
                self.push_entry(AgentTranscriptEntry {
                    role: TranscriptRole::Notice,
                    title: "Assistant".into(),
                    text: message,
                    detail: String::new(),
                    status: TranscriptStatus::Failed,
                });
            }
            _ => {}
        }
    }

    fn handle_response(&mut self, event: &PiRPCEvent) {
        let is_model_settings_response = matches!(
            event.response_command().as_deref(),
            Some(
                "get_state"
                    | "get_available_thinking_levels"
                    | "set_model"
                    | "set_thinking_level"
            )
        );
        if is_model_settings_response {
            let Some(id) = &self.model_settings_request_id else { return };
            if event.string("id").as_deref() != Some(id.as_str()) {
                return;
            }
        }
        if !event.response_succeeded() {
            if is_model_settings_response {
                self.model_settings_request_id = None;
                self.is_updating_model_settings = false;
                if matches!(
                    event.response_command().as_deref(),
                    Some("set_model" | "set_thinking_level")
                ) {
                    self.send_model_settings(PiRPCCommand::GetState, None);
                }
            }
            let message = event
                .response_error()
                .unwrap_or_else(|| "The agent command failed.".into());
            self.status_message = Some(message.clone());
            if event.response_command().as_deref() == Some("prompt") {
                self.push_entry(AgentTranscriptEntry {
                    role: TranscriptRole::Notice,
                    title: "Assistant".into(),
                    text: message,
                    detail: String::new(),
                    status: TranscriptStatus::Failed,
                });
            }
            return;
        }
        match event.response_command().as_deref() {
            Some("get_state") => {
                if let Some(data) = event.response_data() {
                    self.current_model =
                        data.get("model").and_then(PiModelDescriptor::from_value);
                    if let Some(level) = data.get("thinkingLevel").and_then(|v| v.as_str()) {
                        self.thinking_level = level.to_string();
                    }
                }
                self.send_model_settings(
                    PiRPCCommand::GetAvailableThinkingLevels,
                    self.model_settings_request_id.clone(),
                );
            }
            Some("get_available_thinking_levels") => {
                self.model_settings_request_id = None;
                self.is_updating_model_settings = false;
                let levels: Vec<String> = event
                    .response_data()
                    .and_then(|d| d.get("levels"))
                    .and_then(|l| l.as_array())
                    .map(|a| {
                        a.iter()
                            .filter_map(|v| v.as_str().map(str::to_string))
                            .collect()
                    })
                    .unwrap_or_default();
                let unique = levels
                    .iter()
                    .collect::<std::collections::HashSet<_>>()
                    .len()
                    == levels.len();
                if levels.is_empty()
                    || levels.iter().any(|l| l.is_empty())
                    || !unique
                    || !levels.iter().any(|l| l == &self.thinking_level)
                {
                    self.status_message = Some(
                        "The agent did not return valid reasoning levels for this model.".into(),
                    );
                    return;
                }
                self.thinking_levels = if self.current_model.is_none() {
                    Vec::new()
                } else {
                    levels
                };
                self.apply_preferred_model_if_ready();
            }
            Some("get_available_models") => {
                let raw = event
                    .response_data()
                    .and_then(|d| d.get("models"))
                    .and_then(|m| m.as_array())
                    .cloned()
                    .unwrap_or_default();
                self.models = raw.iter().filter_map(PiModelDescriptor::from_value).collect();
                self.apply_preferred_model_if_ready();
            }
            Some("get_session_stats") => {
                if let Some(data) = event.response_data() {
                    self.session_stats = Some(PiSessionStats::from_value(data));
                }
            }
            Some("get_commands") => {
                self.commands = event
                    .response_data()
                    .and_then(|d| d.get("commands"))
                    .and_then(|c| c.as_array())
                    .map(|a| a.iter().filter_map(PiSlashCommand::from_value).collect())
                    .unwrap_or_default();
                // Handled by the panel itself (see `send`), so the composer's
                // completion offers it instead of swapping in another command.
                self.commands.insert(0, PiSlashCommand {
                    name: "resume".into(),
                    description: Some("Resume a past conversation of this project".into()),
                    source: "pitex".into(),
                });
            }
            Some("set_model" | "set_thinking_level") => {
                self.send_model_settings(
                    PiRPCCommand::GetState,
                    self.model_settings_request_id.clone(),
                );
            }
            Some("new_session") => {
                self.send_model_settings(PiRPCCommand::GetState, None);
            }
            Some("switch_session") => {
                let cancelled = event
                    .response_data()
                    .and_then(|d| d.get("cancelled"))
                    .and_then(|c| c.as_bool())
                    == Some(true);
                if let (false, Some(process)) = (cancelled, &self.process) {
                    let _ = process.send(&PiRPCCommand::GetMessages, None);
                }
                self.send_model_settings(PiRPCCommand::GetState, None);
            }
            Some("get_messages") => {
                let messages = event
                    .response_data()
                    .and_then(|d| d.get("messages"))
                    .and_then(|m| m.as_array())
                    .cloned()
                    .unwrap_or_default();
                self.transcript = self.transcript_from_messages(&messages);
                if let Some(process) = &self.process {
                    let _ = process.send(&PiRPCCommand::GetSessionStats, None);
                }
                self.ui_revision += 1;
            }
            _ => {}
        }
    }

    fn handle_message_start(&mut self, event: &PiRPCEvent) {
        let Some(message) = event.nested("message") else { return };
        if message.get("role").and_then(|r| r.as_str()) != Some("assistant") {
            return;
        }
        self.assistant_entry_index = None;
        self.thinking_entry_index = None;
        self.text_content_index = None;
        self.thinking_content_index = None;
    }

    fn handle_message_update(&mut self, event: &PiRPCEvent) {
        let Some(delta) = event.nested("assistantMessageEvent") else { return };
        let Some(delta_type) = delta.get("type").and_then(|t| t.as_str()) else { return };
        match delta_type {
            "text_start" => {
                self.text_content_index = delta.get("contentIndex").and_then(|v| v.as_i64());
                self.append_assistant_entry();
            }
            "text_delta" => {
                if delta.get("contentIndex").and_then(|v| v.as_i64()) != self.text_content_index {
                    self.text_content_index =
                        delta.get("contentIndex").and_then(|v| v.as_i64());
                    self.append_assistant_entry();
                }
                if let Some(text) = delta.get("delta").and_then(|d| d.as_str()) {
                    if let Some(index) = self.assistant_entry_index {
                        if index < self.transcript.len() {
                            self.transcript[index].text.push_str(text);
                        }
                    }
                }
            }
            "thinking_start" => {
                self.thinking_content_index =
                    delta.get("contentIndex").and_then(|v| v.as_i64());
                self.append_thinking_entry();
            }
            "thinking_delta" => {
                if delta.get("contentIndex").and_then(|v| v.as_i64()) != self.thinking_content_index {
                    self.thinking_content_index =
                        delta.get("contentIndex").and_then(|v| v.as_i64());
                    self.append_thinking_entry();
                }
                if let Some(text) = delta.get("delta").and_then(|d| d.as_str()) {
                    if let Some(index) = self.thinking_entry_index {
                        if index < self.transcript.len() {
                            self.transcript[index].text.push_str(text);
                        }
                    }
                }
            }
            "done" => self.finalize_entries(),
            "error" => {
                let reason = delta
                    .get("errorMessage")
                    .or_else(|| delta.get("reason"))
                    .and_then(|r| r.as_str())
                    .unwrap_or("The response failed.")
                    .to_string();
                if let Some(index) = self.assistant_entry_index {
                    if index < self.transcript.len() {
                        self.transcript[index].status = TranscriptStatus::Failed;
                        if self.transcript[index].text.is_empty() {
                            self.transcript[index].text = reason;
                        }
                    }
                } else {
                    self.push_entry(AgentTranscriptEntry {
                        role: TranscriptRole::Notice,
                        title: "Assistant".into(),
                        text: reason,
                        detail: String::new(),
                        status: TranscriptStatus::Failed,
                    });
                }
            }
            _ => {}
        }
    }

    fn handle_message_end(&mut self, event: &PiRPCEvent) {
        let Some(message) = event.nested("message") else {
            self.finalize_entries();
            return;
        };
        if message.get("role").and_then(|r| r.as_str()) != Some("assistant") {
            self.finalize_entries();
            return;
        }
        let text = message
            .get("content")
            .and_then(|c| c.as_array())
            .map(|content| {
                content
                    .iter()
                    .filter(|c| c.get("type").and_then(|t| t.as_str()) == Some("text"))
                    .filter_map(|c| c.get("text").and_then(|t| t.as_str()))
                    .collect::<Vec<_>>()
                    .join("\n\n")
            });
        let failed = message.get("stopReason").and_then(|s| s.as_str()) == Some("error");
        if let Some(text) = text {
            if let Some(index) = self.assistant_entry_index {
                if index < self.transcript.len() {
                    self.transcript[index].text = text;
                    self.transcript[index].status = if failed {
                        TranscriptStatus::Failed
                    } else {
                        TranscriptStatus::Done
                    };
                }
            } else if !text.is_empty() {
                let mut entry = AgentTranscriptEntry {
                    role: TranscriptRole::Assistant,
                    title: self.assistant_title(),
                    text,
                    detail: String::new(),
                    status: if failed {
                        TranscriptStatus::Failed
                    } else {
                        TranscriptStatus::Done
                    },
                };
                if entry.text.is_empty() {
                    if let Some(e) = message.get("errorMessage").and_then(|m| m.as_str()) {
                        entry.text = e.to_string();
                    }
                }
                self.push_entry(entry);
            }
        }
        self.finalize_entries();
    }

    fn handle_tool_start(&mut self, event: &PiRPCEvent) {
        let Some(call_id) = event.string("toolCallId") else { return };
        let name = event.string("toolName").unwrap_or_else(|| "tool".into());
        let summary = tool_summary(&name, event.nested("args"));
        self.push_entry(AgentTranscriptEntry {
            role: TranscriptRole::Tool,
            title: name,
            text: String::new(),
            detail: summary,
            status: TranscriptStatus::Running,
        });
        self.tool_index_by_call_id
            .insert(call_id, self.transcript.len() - 1);
    }

    fn handle_tool_update(&mut self, event: &PiRPCEvent) {
        let Some(call_id) = event.string("toolCallId") else { return };
        let Some(&index) = self.tool_index_by_call_id.get(&call_id) else { return };
        if index >= self.transcript.len() {
            return;
        }
        if let Some(text) = tool_result_text(event.nested("partialResult")) {
            self.transcript[index].detail = bounded(&text, 4_000);
        }
    }

    fn handle_tool_end(&mut self, event: &PiRPCEvent) {
        let Some(call_id) = event.string("toolCallId") else { return };
        let Some(&index) = self.tool_index_by_call_id.get(&call_id) else { return };
        if index >= self.transcript.len() {
            return;
        }
        self.transcript[index].status = if event.bool("isError") {
            TranscriptStatus::Failed
        } else {
            TranscriptStatus::Done
        };
        if let Some(text) = tool_result_text(event.nested("result")) {
            self.transcript[index].detail = bounded(&text, 4_000);
        }
    }

    fn handle_extension_ui(&mut self, event: &PiRPCEvent) {
        let method = event.string("method").unwrap_or_default();
        match method.as_str() {
            "select" | "confirm" | "input" | "editor" => {
                if let Some(id) = event.string("id") {
                    if let Some(process) = &self.process {
                        let _ = process.send(&PiRPCCommand::ExtensionUICancelled { id }, None);
                    }
                }
                let title = event.string("title").unwrap_or_else(|| method.clone());
                self.push_entry(AgentTranscriptEntry {
                    role: TranscriptRole::Notice,
                    title: "Assistant".into(),
                    text: format!(
                        "An interactive prompt ({title}) is not supported in this panel and was skipped."
                    ),
                    detail: String::new(),
                    status: TranscriptStatus::Done,
                });
            }
            "notify" => {
                if let Some(message) = event.string("message") {
                    self.push_entry(AgentTranscriptEntry {
                        role: TranscriptRole::Notice,
                        title: "Assistant".into(),
                        text: message,
                        detail: String::new(),
                        status: TranscriptStatus::Done,
                    });
                }
            }
            "setStatus" => {
                self.status_message = event.string("statusText");
            }
            _ => {}
        }
    }

    fn last_assistant_error(&self, event: &PiRPCEvent) -> Option<String> {
        let messages = event.object.get("messages")?.as_array()?;
        for message in messages.iter().rev() {
            if message.get("role").and_then(|r| r.as_str()) != Some("assistant") {
                continue;
            }
            if message.get("stopReason").and_then(|s| s.as_str()) == Some("error") {
                return Some(
                    message
                        .get("errorMessage")
                        .and_then(|e| e.as_str())
                        .unwrap_or("The agent run failed.")
                        .to_string(),
                );
            }
            return None;
        }
        None
    }

    fn append_assistant_entry(&mut self) {
        let title = self.assistant_title();
        self.push_entry(AgentTranscriptEntry {
            role: TranscriptRole::Assistant,
            title,
            text: String::new(),
            detail: String::new(),
            status: TranscriptStatus::Streaming,
        });
        self.assistant_entry_index = Some(self.transcript.len() - 1);
    }
    fn append_thinking_entry(&mut self) {
        self.push_entry(AgentTranscriptEntry {
            role: TranscriptRole::Thinking,
            title: "Thinking".into(),
            text: String::new(),
            detail: String::new(),
            status: TranscriptStatus::Streaming,
        });
        self.thinking_entry_index = Some(self.transcript.len() - 1);
    }
    fn finalize_entries(&mut self) {
        if let Some(index) = self.assistant_entry_index {
            if index < self.transcript.len() {
                self.transcript[index].status = TranscriptStatus::Done;
            }
        }
        if let Some(index) = self.thinking_entry_index {
            if index < self.transcript.len() {
                self.transcript[index].status = TranscriptStatus::Done;
            }
        }
        self.assistant_entry_index = None;
        self.thinking_entry_index = None;
        self.text_content_index = None;
        self.thinking_content_index = None;
    }
    fn assistant_title(&self) -> String {
        self.current_model
            .as_ref()
            .map(|m| format!("Pitex Agent · {}", m.name))
            .unwrap_or_else(|| "Pitex Agent".into())
    }

    /// History-limit trimming — verbatim didSet semantics (indices reset).
    fn push_entry(&mut self, entry: AgentTranscriptEntry) {
        self.transcript.push(entry);
        if self.transcript.len() > self.history_limit {
            let drop = self.transcript.len() - self.history_limit;
            self.transcript.drain(..drop);
            self.assistant_entry_index = None;
            self.thinking_entry_index = None;
            self.tool_index_by_call_id.clear();
        }
    }

    /// `envelope(for:)` — verbatim context envelope.
    fn envelope(&self, prompt: &str) -> String {
        let context = self
            .context_provider
            .as_ref()
            .map(|p| (p)())
            .unwrap_or_default();
        let mut parts: Vec<String> = vec![
            "<editor-context>".into(),
            "The user is working inside a native Linux LaTeX editor. The project root is the current working directory; use your file tools to inspect or modify project files when asked.".into(),
        ];
        if self.attach_active_document {
            if let Some(path) = &context.active_path {
                parts.push(format!(
                    "Active document (open in the editor right now): {path}"
                ));
                if let Some(text) = &context.active_text {
                    if !text.is_empty() {
                        parts.push(format!(
                            "Current content of {path}:\n```\n{}\n```",
                            bounded(text, 96_000)
                        ));
                    }
                }
            }
        }
        if let Some(selection) = &self.selection_attachment {
            if !selection.text.is_empty() {
                parts.push(format!(
                    "Text selected in {} (lines {}–{}):\n```\n{}\n```",
                    selection.path,
                    selection.start_line,
                    selection.end_line,
                    bounded(&selection.text, 16_000)
                ));
            }
        }
        if !context.project_files.is_empty() {
            parts.push(format!("Project source files: {}", context.project_files.join(", ")));
        }
        if let Some(pdf_data) = &context.pdf_data {
            let mut cache = self.pdf_text_cache.borrow_mut();
            if !cache.as_ref().map(|(data, _)| Arc::ptr_eq(data, pdf_data)).unwrap_or(false) {
                *cache = Some((pdf_data.clone(), crate::pdf::extract_text(pdf_data)));
            }
            if let Some(pdf_text) = cache.as_ref().and_then(|(_, text)| text.as_ref()) {
                if !pdf_text.is_empty() {
                    let label = context
                        .pdf_path
                        .clone()
                        .unwrap_or_else(|| "the built PDF preview".into());
                    parts.push(format!(
                        "Text extracted from {label}:\n```\n{}\n```",
                        bounded(&pdf_text, 48_000)
                    ));
                }
            }
        }
        parts.push("</editor-context>".into());
        parts.push(String::new());
        parts.push(prompt.to_string());
        parts.join("\n")
    }

    /// `childEnvironment(executable:environment:)` — starts from the
    /// discovered toolchain environment (its PATH already carries the
    /// version-manager dirs), then adds the app-local pi home and the
    /// executable dir + TeX Live bins ahead of it. `pub(crate)` so the
    /// ghost-completion spawner inherits the identical environment.
    pub(crate) fn child_environment(
        executable: &Path,
        environment: &HashMap<String, String>,
    ) -> HashMap<String, String> {
        let mut environment = environment.clone();
        environment.insert(
            "PI_CODING_AGENT_DIR".into(),
            pi_paths::agent_directory().to_string_lossy().into_owned(),
        );
        let mut search: Vec<String> = vec![executable
            .parent()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default()];
        search.extend(
            crate::model::texlive_bin_dirs()
                .iter()
                .map(|p| p.to_string_lossy().into_owned()),
        );
        if let Some(existing) = environment.get("PATH").cloned() {
            search.extend(
                std::env::split_paths(std::ffi::OsStr::new(&existing))
                    .map(|d| d.to_string_lossy().into_owned()),
            );
        }
        #[cfg(unix)]
        search.extend(
            ["/usr/bin", "/bin", "/usr/sbin", "/sbin"]
                .iter()
                .map(|s| s.to_string()),
        );
        let mut seen = std::collections::HashSet::new();
        let separator = if cfg!(windows) { ";" } else { ":" };
        let path = search
            .into_iter()
            .filter(|p| seen.insert(p.clone()))
            .collect::<Vec<_>>()
            .join(separator);
        environment.insert("PATH".into(), path);
        environment
    }

    fn system_prompt_supplement() -> &'static str {
        "You are an expert LaTeX writing and research assistant inside Pitex, \
         an AI-accelerated LaTeX editor. When a user shows code, explain or fix \
         it concisely. Default to xelatex semantics. Prefer minimal, idiomatic \
         LaTeX — no unnecessary packages. When you reply with LaTeX, wrap \
         snippets in ```latex code fences so they paste cleanly. Use math mode \
         for equations even in short answers. Use Humanizer when the user asks \
         to write the phrases, sentences, or paragraphs. Use SciSpace when the \
         user asks to search and organize the papers. Use Latex Doctor when the \
         user asks about tex distributions. Use Latex Compile when the user \
         asks o build, render, regenerate, or compile a `.tex` file. Use Texlive \
         Runtime Installer when the user asks to install or repair LaTeX \
         support."
    }
}

fn bounded(text: &str, limit: usize) -> String {
    if text.len() <= limit {
        return text.to_string();
    }
    let mut end = limit;
    while !text.is_char_boundary(end) && end > 0 {
        end -= 1;
    }
    format!("{}\n…(truncated — read the file for the full content)", &text[..end])
}

fn tool_summary(name: &str, args: Option<&Map<String, Value>>) -> String {
    let Some(args) = args else { return String::new() };
    match name {
        "bash" => args
            .get("command")
            .and_then(|c| c.as_str())
            .map(|c| bounded(c, 2_000))
            .unwrap_or_default(),
        "read" | "edit" | "write" => args
            .get("path")
            .and_then(|p| p.as_str())
            .unwrap_or_default()
            .to_string(),
        "ls" | "find" | "grep" => args
            .get("path")
            .or_else(|| args.get("pattern"))
            .and_then(|p| p.as_str())
            .unwrap_or_default()
            .to_string(),
        _ => serde_json::to_string(args)
            .map(|s| bounded(&s, 500))
            .unwrap_or_default(),
    }
}

fn tool_result_text(result: Option<&Map<String, Value>>) -> Option<String> {
    let content = result?.get("content")?.as_array()?;
    let text = content
        .iter()
        .filter_map(|c| c.get("text").and_then(|t| t.as_str()))
        .collect::<Vec<_>>()
        .join("\n");
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

// ─── PiRuntimeInstaller (verbatim, minus the macOS .command script) ─────────

pub mod pi_installer {
    use super::{app_local_runtime_installed, is_executable, pi_paths, PiToolchain};
    use std::path::{Path, PathBuf};

    /// Matches `PiAgentProcess.packageName` — upstream moved the npm scope
    /// from @mariozechner to @earendil-works after 0.73.x; the legacy scope
    /// tops out at 0.73.1 so installs from it would 404.
    pub const PACKAGE_NAME: &str = "@earendil-works/pi-coding-agent";
    pub const DESIRED_VERSION: &str = "0.85.1";
    static INSTALL_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    pub fn installation_in_progress() -> bool {
        INSTALL_LOCK.try_lock().is_err()
    }

    /// `InstallError.runtimeMissing` — shared with `PiToolchain.launch`.
    pub const RUNTIME_MISSING: &str =
        "Bun or Node.js with npm was not found. Install Bun with Homebrew, npm, or the official installer, then retry.";

    /// Read package metadata without executing an `env node` launcher —
    /// walks up from the resolved launcher to `runtimeDirectory` looking for
    /// the package's own `package.json`.
    pub fn installed_version() -> Option<String> {
        let executable = pi_paths::runtime_executable();
        let resolved = executable.canonicalize().unwrap_or(executable);
        let mut directory = resolved.parent()?.to_path_buf();
        while directory.as_path() != Path::new("/") {
            let package = directory.join("package.json");
            if let Ok(data) = std::fs::read(&package) {
                if let Ok(object) = serde_json::from_slice::<serde_json::Value>(&data) {
                    let name = object.get("name").and_then(|n| n.as_str());
                    let version = object.get("version").and_then(|v| v.as_str());
                    if let (Some(name), Some(version)) = (name, version) {
                        if name == PACKAGE_NAME || name == "@mariozechner/pi-coding-agent" {
                            return Some(version.to_string());
                        }
                    }
                }
            }
            if directory == pi_paths::runtime_directory() || !directory.pop() {
                break;
            }
        }
        None
    }

    /// `isOlder(_:than:)` — semver-ish component compare. Pre-release
    /// suffixes ("0.85.1-beta") contribute their leading digits, not 0.
    pub fn is_older(installed: &str, desired: &str) -> bool {
        fn component(part: &str) -> u64 {
            let digits: String = part.chars().take_while(|c| c.is_ascii_digit()).collect();
            digits.parse().unwrap_or(0)
        }
        let lhs: Vec<u64> = installed.split('.').map(component).collect();
        let rhs: Vec<u64> = desired.split('.').map(component).collect();
        for i in 0..lhs.len().max(rhs.len()) {
            let l = lhs.get(i).copied().unwrap_or(0);
            let r = rhs.get(i).copied().unwrap_or(0);
            if l != r {
                return l < r;
            }
        }
        false
    }

    /// `ensureInstalled` — install when missing or older than desired.
    /// Returns the npm output on failure for the settings pane.
    pub fn ensure_installed() -> Result<Option<String>, String> {
        let needs_install = if !app_local_runtime_installed() {
            true
        } else if let Some(installed) = installed_version() {
            is_older(&installed, DESIRED_VERSION)
        } else {
            false
        };
        if needs_install {
            install()?;
        }
        let _ = install_bundled_skills();
        Ok(None)
    }

    /// `install(toolchain:)` — both package managers install locally; no
    /// global packages or provider credentials are changed. The launcher is
    /// published only after a `--version` smoke check succeeds.
    pub fn install() -> Result<(), String> {
        install_managed(false, None).map(|_| ())
    }

    /// Explicit user update: resolve the registry's stable latest tag.
    pub fn update() -> Result<String, String> {
        install_managed(true, None)
    }

    pub fn update_with_tools(tools: &PiToolchain) -> Result<String, String> {
        install_managed(true, Some(tools))
    }

    pub fn install_with_tools(tools: &PiToolchain) -> Result<(), String> {
        install_managed(false, Some(tools)).map(|_| ())
    }

    fn install_managed(latest: bool, tools: Option<&PiToolchain>) -> Result<String, String> {
        let _guard = INSTALL_LOCK.try_lock()
            .map_err(|_| "An agent installation is already in progress.".to_string())?;
        let discovered;
        let tools = match tools {
            Some(tools) => tools,
            None => {
                discovered = PiToolchain::discover(std::env::vars().collect(),
                    &dirs::home_dir().unwrap_or_default(), &PiToolchain::SYSTEM_DIRECTORIES);
                &discovered
            }
        };
        let runtime = pi_paths::runtime_directory();
        let backup = runtime.with_file_name(format!("pi-runtime-backup-{}", crate::model::uuid_v4()));
        let backed_up = latest && runtime.exists();
        if backed_up { std::fs::rename(&runtime, &backup).map_err(|e| e.to_string())?; }
        let result = install_package(tools, if latest { "latest" } else { DESIRED_VERSION })
            .and_then(|_| installed_version().or_else(|| (!latest).then(|| DESIRED_VERSION.into()))
                .ok_or_else(|| "The installed agent version could not be verified.".to_string()));
        if let Err(error) = result {
            if latest {
                let restore = (|| -> std::io::Result<()> {
                    if runtime.exists() { std::fs::remove_dir_all(&runtime)?; }
                    if backed_up { std::fs::rename(&backup, &runtime)?; }
                    Ok(())
                })();
                if let Err(restore_error) = restore {
                    return Err(format!("{error} Previous runtime preserved at {}; restore failed: {restore_error}", backup.display()));
                }
            }
            return Err(error);
        }
        if backed_up { let _ = std::fs::remove_dir_all(backup); }
        result
    }

    fn install_package(tools: &PiToolchain, version: &str) -> Result<(), String> {
        let runtime = pi_paths::runtime_directory();
        let spec = format!("{PACKAGE_NAME}@{version}");
        let runtime_str = runtime.to_string_lossy().into_owned();
        // npm is a real fallback even when bun exists: `bun add` can exit 0
        // while laying down nothing (stale tree it won't re-extract, corrupt
        // cache, a `bun` that isn't bun, a registry mirror serving a stub).
        let mut attempts: Vec<(&str, PathBuf, Vec<String>)> = Vec::new();
        if let Some(bun) = &tools.bun {
            attempts.push((
                "bun",
                bun.clone(),
                vec![
                    "add".into(),
                    "--cwd".into(),
                    runtime_str.clone(),
                    "--force".into(),
                    "--linker".into(),
                    "hoisted".into(),
                    "--no-cache".into(),
                    "--exact".into(),
                    spec.clone(),
                ],
            ));
        }
        if let Some((npm_exe, npm_args)) = tools.npm_command(&[
            "install".into(),
            "--prefix".into(),
            runtime_str.clone(),
            "--save-exact".into(),
            "--prefer-online".into(),
            "--no-audit".into(),
            "--no-fund".into(),
            spec.clone(),
        ]) {
            attempts.push(("npm", npm_exe.clone(), npm_args.clone()));
            let mut pinned = npm_args;
            pinned.extend(["--registry".into(), "https://registry.npmjs.org".into()]);
            attempts.push(("npm+npmjs", npm_exe, pinned));
        }
        if attempts.is_empty() {
            return Err(RUNTIME_MISSING.into());
        }
        std::fs::create_dir_all(&runtime).map_err(|e| e.to_string())?;
        // Ok(()) when the installed CLI runs `--version` cleanly.
        let smoke_check = |entry: &Path| -> Result<(), String> {
            let (check_exe, check_args) = tools.launch(entry, &["--version".into()])?;
            let check = run_install(&check_exe, &check_args, &tools.environment, &runtime, 15)?;
            if check.termination != (build_core::ProcessTermination::Exited { code: 0 })
                || check.stop_reason != build_core::ProcessStopReason::Completed
            {
                return Err(tail(&String::from_utf8_lossy(&check.standard_error), 2_000));
            }
            Ok(())
        };
        // Every attempt gets a clean slate — including package.json, whose
        // leftover state (e.g. workspaces) can redirect where files land.
        let mut entry: Option<PathBuf> = None;
        let mut failures: Vec<String> = Vec::new();
        for (name, executable, arguments) in &attempts {
            if entry.is_some() {
                break;
            }
            let _ = std::fs::remove_dir_all(runtime.join("node_modules"));
            for file in ["bun.lock", "bun.lockb", "package-lock.json", "package.json"] {
                let _ = std::fs::remove_file(runtime.join(file));
            }
            let result = match run_install(executable, arguments, &tools.environment, &runtime, 300) {
                Ok(result) => result,
                Err(error) => {
                    failures.push(format!("{name}: {error}"));
                    continue;
                }
            };
            let output = format!(
                "{}{}",
                String::from_utf8_lossy(&result.standard_output),
                String::from_utf8_lossy(&result.standard_error)
            );
            if result.stop_reason == build_core::ProcessStopReason::Completed
                && result.termination == (build_core::ProcessTermination::Exited { code: 0 })
            {
                if let Some(found) = installed_entry(&runtime) {
                    match smoke_check(&found) {
                        Ok(()) => entry = Some(found),
                        Err(e) => failures.push(format!("{name}: {e}")),
                    }
                } else {
                    failures.push(format!(
                        "{name}: no CLI entry point — output: {}",
                        tail(&output, 1_000)
                    ));
                }
            } else {
                failures.push(format!("{name}: {}", tail(&output, 1_000)));
            }
        }
        let Some(entry) = entry else {
            return Err(format!("Pitex Agent install failed: {}", failures.join("; ")));
        };
        // Rename a new symlink over the old one atomically, leaving an
        // existing legacy package's CLI and running agent processes
        // untouched. On Windows `runtime_executable` IS the package entry
        // (`symlink` needs a privilege Windows users rarely hold), so there
        // is no launcher to publish.
        #[cfg(unix)]
        {
            let launcher_dir = pi_paths::runtime_executable()
                .parent()
                .map(|p| p.to_path_buf())
                .ok_or_else(|| "The agent launcher directory is invalid.".to_string())?;
            std::fs::create_dir_all(&launcher_dir).map_err(|e| e.to_string())?;
            let temporary = launcher_dir.join(crate::model::uuid_v4());
            std::os::unix::fs::symlink(&entry, &temporary).map_err(|e| e.to_string())?;
            if let Err(e) = std::fs::rename(&temporary, pi_paths::runtime_executable()) {
                let _ = std::fs::remove_file(&temporary);
                return Err(format!("The agent launcher could not be updated: {e}"));
            }
        }
        install_bundled_skills()
    }

    /// The installed package's CLI entry — prefers the unbundled
    /// `dist/cli.js` (see `launch`), falls back to the declared `bin`.
    fn installed_entry(runtime: &Path) -> Option<PathBuf> {
        let dist = runtime.join(format!("node_modules/{PACKAGE_NAME}/dist"));
        ["cli.js", "bundle/cli.js"]
            .iter()
            .map(|name| dist.join(name))
            .find(|path| path.is_file())
    }

    /// `ProcessRunner.run` wrapper that keeps stdout+stderr for the error
    /// path (unlike `run_probe` which discards them).
    fn run_install(
        executable: &Path,
        arguments: &[String],
        environment: &std::collections::HashMap<String, String>,
        cwd: &Path,
        timeout_secs: u64,
    ) -> Result<build_core::ProcessResult, String> {
        let plan = build_core::DirectCommandPlan::new(
            executable.to_string_lossy().into_owned(),
            arguments.to_vec(),
            build_core::WorkingDirectoryPolicy::Explicit(cwd.to_string_lossy().into_owned()),
            build_core::EnvironmentPolicy::Inherit {
                overrides: environment.clone(),
            },
        )
        .map_err(|error| format!("The install command is invalid: {error}"))?;
        build_core::ProcessRunner::default()
            .run(
                &plan,
                cwd,
                None,
                Some(std::time::Duration::from_secs(timeout_secs)),
                None,
                None,
            )
            .map_err(|error| format!("The install command could not be started: {error}"))
    }

    /// Copy bundled skills into the agent skills dir (replacing stale copies).
    pub fn install_bundled_skills() -> Result<(), String> {
        let Some(bundled) = bundled_skills_directory() else { return Ok(()) };
        let destination = pi_paths::skills_directory();
        std::fs::create_dir_all(&destination).map_err(|e| e.to_string())?;
        let entries = std::fs::read_dir(&bundled).map_err(|e| e.to_string())?;
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') {
                continue;
            }
            let target = destination.join(&name);
            if target.exists() {
                let _ = std::fs::remove_dir_all(&target);
            }
            copy_dir_all(&entry.path(), &target).map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    /// `pi install <source>` — install a skill (npm package, git URL, or
    /// local path) into the app-local agent dir. Runs through the same
    /// toolchain discovery + `launch` routing as the agent itself so
    /// Bun/Node shims resolve; `PI_CODING_AGENT_DIR` pins the install to
    /// `pi_paths::agent_directory()`. Blocking — call off the UI thread.
    pub fn install_skill(source: &str) -> Result<(), String> {
        let executable = pi_paths::runtime_executable();
        if !is_executable(&executable) {
            return Err("Pitex Agent is not installed yet.".into());
        }
        let tools = PiToolchain::discover(
            std::env::vars().collect(),
            &dirs::home_dir().unwrap_or_default(),
            &PiToolchain::SYSTEM_DIRECTORIES,
        );
        let (exe, args) = tools.launch(&executable, &["install".into(), source.into()])?;
        let mut environment = tools.environment.clone();
        environment.insert(
            "PI_CODING_AGENT_DIR".into(),
            pi_paths::agent_directory().to_string_lossy().into_owned(),
        );
        let cwd = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        let result = run_install(&exe, &args, &environment, &cwd, 300)?;
        if result.stop_reason != build_core::ProcessStopReason::Completed
            || result.termination != (build_core::ProcessTermination::Exited { code: 0 })
        {
            let text = format!(
                "{}{}",
                String::from_utf8_lossy(&result.standard_output),
                String::from_utf8_lossy(&result.standard_error)
            );
            return Err(tail(&text, 2_000));
        }
        Ok(())
    }

    /// The skills shipped with the app — resolved next to the executable or
    /// under the crate's Resources dir in development.
    fn bundled_skills_directory() -> Option<PathBuf> {
        if let Ok(exe) = std::env::current_exe() {
            for rel in [
                "../share/pitex/PitexAgent/skills",
                "../../share/pitex/PitexAgent/skills",
            ] {
                let candidate = exe.parent()?.join(rel);
                if candidate.is_dir() {
                    return Some(candidate);
                }
            }
        }
        let dev = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../Mac/Resources/PitexAgent/skills");
        if dev.is_dir() {
            return Some(dev);
        }
        None
    }

    /// POSIX single-quote escaping — `quoted(_:)` in the Swift original.
    /// Safe for embedding any path in a shell command line.
    pub fn shell_quote(value: &str) -> String {
        format!("'{}'", value.replace('\'', "'\\''"))
    }

    /// `openAuthenticationInTerminal`/`authenticationScript` equivalent:
    /// writes a login/logout shell script the settings pane hands to the
    /// terminal surface — the embedded VTE on modern builds, an external
    /// terminal emulator on the Ubuntu 22.04 build (no VTE-GTK4 there). The
    /// toolchain is discovered here so the script exports the same PATH and
    /// runs through Bun/Node when the launcher is a JavaScript entry point.
    pub fn authentication_script(logout: bool) -> Result<PathBuf, String> {
        let executable = pi_paths::runtime_executable();
        if !is_executable(&executable) {
            return Err("Pitex Agent is not installed yet.".into());
        }
        std::fs::create_dir_all(pi_paths::agent_directory()).map_err(|e| e.to_string())?;
        let tools = PiToolchain::discover(
            std::env::vars().collect(),
            &dirs::home_dir().unwrap_or_default(),
            &PiToolchain::SYSTEM_DIRECTORIES,
        );
        let launch = tools.launch(&executable, &[])?;
        let action = if logout { "Logout" } else { "Login" };
        let command = if logout { "/logout" } else { "/login" };
        write_authentication_script(&tools, &launch, action, command)
    }

    /// POSIX single-quote escaping — `quoted(_:)` in the Swift original.
    /// Safe for embedding any path in a shell command line.
    #[cfg(unix)]
    fn write_authentication_script(
        tools: &PiToolchain,
        launch: &(PathBuf, Vec<String>),
        action: &str,
        command: &str,
    ) -> Result<PathBuf, String> {
        let launch_command = std::iter::once(launch.0.to_string_lossy().into_owned())
            .chain(launch.1.iter().cloned())
            .map(|part| shell_quote(&part))
            .collect::<Vec<_>>()
            .join(" ");
        let script_path = pi_paths::agent_directory().join(format!("Pitex Agent {action}.sh"));
        // util-linux `script` needs `-c` for the command — the BSD positional
        // form used on macOS does not exist on Ubuntu 24.04.
        let script = format!(
            "#!/bin/bash\nexport PI_CODING_AGENT_DIR={}\nexport PATH={}\necho \"Pitex Agent {action} — pick a provider.\"\necho \"If the provider list does not open on its own, type {command} and press Return.\"\ntrap 'stty sane' EXIT\nstty -icanon -echo min 1 time 0\n( sleep 3; printf '{command}\\r'; cat ) | script -q -c {} /dev/null\n",
            shell_quote(&pi_paths::agent_directory().to_string_lossy()),
            shell_quote(tools.environment.get("PATH").map(String::as_str).unwrap_or("/usr/bin:/bin")),
            shell_quote(&launch_command)
        );
        use std::os::unix::fs::PermissionsExt;
        std::fs::write(&script_path, script).map_err(|e| e.to_string())?;
        std::fs::set_permissions(&script_path, std::fs::Permissions::from_mode(0o700))
            .map_err(|e| e.to_string())?;
        Ok(script_path)
    }

    /// Windows counterpart — a `.bat` file run under `cmd /k` by
    /// `spawn_external_terminal`. conhost gives `pi` a real console, so the
    /// `stty`/`script` plumbing the Unix PTY needs is unnecessary: export the
    /// environment, print the same guidance, launch the agent interactively.
    #[cfg(windows)]
    fn write_authentication_script(
        tools: &PiToolchain,
        launch: &(PathBuf, Vec<String>),
        action: &str,
        command: &str,
    ) -> Result<PathBuf, String> {
        let launch_command = std::iter::once(launch.0.to_string_lossy().into_owned())
            .chain(launch.1.iter().cloned())
            .map(|part| format!("\"{}\"", part.replace('"', "\"\"")))
            .collect::<Vec<_>>()
            .join(" ");
        let script_path = pi_paths::agent_directory().join(format!("Pitex Agent {action}.bat"));
        let script = format!(
            "@echo off\r\nset \"PI_CODING_AGENT_DIR={}\"\r\nset \"PATH={}\"\r\necho Pitex Agent {action} — pick a provider.\r\necho If the provider list does not open on its own, type {command} and press Enter.\r\n{}\r\n",
            pi_paths::agent_directory().to_string_lossy(),
            tools.environment.get("PATH").map(String::as_str).unwrap_or_default(),
            launch_command
        );
        std::fs::write(&script_path, script).map_err(|e| e.to_string())?;
        Ok(script_path)
    }

    fn tail(text: &str, limit: usize) -> String {
        if text.len() <= limit {
            text.to_string()
        } else {
            let mut start = text.len() - limit;
            while !text.is_char_boundary(start) {
                start -= 1;
            }
            text[start..].to_string()
        }
    }
    fn copy_dir_all(from: &PathBuf, to: &PathBuf) -> std::io::Result<()> {
        std::fs::create_dir_all(to)?;
        for entry in std::fs::read_dir(from)? {
            let entry = entry?;
            let dest = to.join(entry.file_name());
            if entry.file_type()?.is_dir() {
                copy_dir_all(&entry.path(), &dest)?;
            } else {
                std::fs::copy(entry.path(), dest)?;
            }
        }
        Ok(())
    }

    // Toolchain-discovery tests stage fake `#!/bin/sh` binaries and chmod
    // bits — Unix semantics throughout, so the module is Unix-only. The
    // Windows port exercises the same functions through its own PATH/PATHEXT
    // branches at build time.
    #[cfg(all(test, unix))]
    mod tests {
        use super::*;
        use std::collections::HashMap;
        use std::sync::{Mutex, OnceLock};

        /// `PI_CODING_AGENT_DIR` is process-global: serialize tests that
        /// mutate it so parallel tests can't observe a transient value.
        fn env_lock() -> std::sync::MutexGuard<'static, ()> {
            static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
            LOCK.get_or_init(|| Mutex::new(())).lock().unwrap()
        }

        #[test]
        fn tail_never_splits_a_utf8_character() {
            // npm output ending in a multi-byte char: the raw byte cut would
            // panic; the boundary walk must back up to the char start.
            let s = format!("{}npm err! 한", "x".repeat(2000));
            assert_eq!(tail(&s, 2), "한");
            assert_eq!(tail(&s, 2000).len(), 2000);
            assert_eq!(tail("short", 2000), "short");
        }

        #[test]
        fn is_older_compares_numeric_prefixes() {
            assert!(is_older("0.84.9", "0.85.1"));
            assert!(!is_older("0.85.1", "0.85.1"));
            assert!(!is_older("0.85.1-beta", "0.85.1"));
            assert!(is_older("0.85.0-rc1", "0.85.1"));
        }

        /// Redirects the app-local pi home into a fresh temp dir and returns
        /// it — `runtimeDirectory` resolves as the `pi-runtime` sibling.
        fn sandbox(tag: &str) -> PathBuf {
            let dir = std::env::temp_dir()
                .join(format!("pitex-installer-{}-{tag}", std::process::id()))
                .join("pi");
            let _ = std::fs::remove_dir_all(dir.parent().unwrap());
            std::fs::create_dir_all(&dir).unwrap();
            std::env::set_var("PI_CODING_AGENT_DIR", &dir);
            dir
        }

        fn leave_sandbox(dir: &Path) {
            std::env::remove_var("PI_CODING_AGENT_DIR");
            let _ = std::fs::remove_dir_all(dir.parent().unwrap_or(dir));
        }

        fn executable(path: &Path, body: &str) {
            use std::os::unix::fs::PermissionsExt;
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, body).unwrap();
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }

        /// A fake bun that answers `add --cwd <dir>` by materializing the
        /// package layout npm would create, and `--bun <script> ...` by
        /// running the script — enough for the launcher smoke check.
        const FAKE_BUN: &str = "#!/bin/sh\n\
            if [ \"$1\" = \"add\" ]; then\n\
              dir=\"$3\"\n\
              pkg=\"$dir/node_modules/@earendil-works/pi-coding-agent\"\n\
              mkdir -p \"$pkg/dist\"\n\
              printf '#!/bin/sh\\necho 0.85.1\\n' > \"$pkg/dist/cli.js\"\n\
              chmod +x \"$pkg/dist/cli.js\"\n\
              printf '{\"name\":\"@earendil-works/pi-coding-agent\",\"version\":\"0.85.1\"}' > \"$pkg/package.json\"\n\
              exit 0\n\
            elif [ \"$1\" = \"--bun\" ]; then\n\
              shift\n\
              exec /bin/sh \"$@\"\n\
            fi\n\
            exit 1\n";

        #[test]
        fn bun_install_publishes_launcher_after_smoke_check() {
            let _guard = env_lock();
            let dir = sandbox("bun");
            let tools_dir = dir.join("tools");
            let bun = tools_dir.join("bun");
            executable(&bun, FAKE_BUN);
            let tools = PiToolchain {
                environment: HashMap::from([(
                    "PATH".to_string(),
                    format!("{}:/usr/bin:/bin", tools_dir.display()),
                )]),
                bun: Some(bun),
                node: None,
                npm: None,
            };
            install_with_tools(&tools).expect("install succeeds");
            let launcher = pi_paths::runtime_executable();
            assert!(launcher.exists(), "launcher must be published");
            let resolved = launcher.canonicalize().unwrap();
            assert!(resolved.ends_with("dist/cli.js"));
            assert_eq!(installed_version().as_deref(), Some("0.85.1"));
            leave_sandbox(&dir);
        }

        #[test]
        fn failed_install_preserves_existing_launcher() {
            let _guard = env_lock();
            let dir = sandbox("fail");
            // Pre-existing launcher pointing at a legacy install — a failed
            // reinstall must not clobber it.
            let legacy = dir.join("legacy-cli.js");
            executable(&legacy, "#!/bin/sh\necho old\n");
            let bin = pi_paths::runtime_directory().join("bin");
            std::fs::create_dir_all(&bin).unwrap();
            std::os::unix::fs::symlink(&legacy, pi_paths::runtime_executable()).unwrap();
            let tools_dir = dir.join("tools");
            let bun = tools_dir.join("bun");
            executable(&bun, "#!/bin/sh\nexit 1\n");
            let tools = PiToolchain {
                environment: HashMap::from([(
                    "PATH".to_string(),
                    format!("{}:/usr/bin:/bin", tools_dir.display()),
                )]),
                bun: Some(bun),
                node: None,
                npm: None,
            };
            assert!(install_with_tools(&tools).is_err());
            assert_eq!(
                pi_paths::runtime_executable().canonicalize().unwrap(),
                legacy.canonicalize().unwrap(),
                "the existing launcher must survive a failed install"
            );
            leave_sandbox(&dir);
        }

        #[test]
        fn failed_smoke_check_preserves_existing_launcher() {
            let _guard = env_lock();
            let dir = sandbox("smoke");
            // bun installs a package whose cli.js exits non-zero — the smoke
            // check fails and the launcher is never replaced.
            let legacy = dir.join("legacy-cli.js");
            executable(&legacy, "#!/bin/sh\necho old\n");
            let bin = pi_paths::runtime_directory().join("bin");
            std::fs::create_dir_all(&bin).unwrap();
            std::os::unix::fs::symlink(&legacy, pi_paths::runtime_executable()).unwrap();
            let tools_dir = dir.join("tools");
            let bun = tools_dir.join("bun");
            executable(
                &bun,
                "#!/bin/sh\n\
                 if [ \"$1\" = \"add\" ]; then\n\
                   dir=\"$3\"\n\
                   pkg=\"$dir/node_modules/@earendil-works/pi-coding-agent\"\n\
                   mkdir -p \"$pkg/dist\"\n\
                   printf '#!/bin/sh\\nexit 1\\n' > \"$pkg/dist/cli.js\"\n\
                   chmod +x \"$pkg/dist/cli.js\"\n\
                   exit 0\n\
                 elif [ \"$1\" = \"--bun\" ]; then\n\
                   shift\n\
                   exec /bin/sh \"$@\"\n\
                 fi\n\
                 exit 1\n",
            );
            let tools = PiToolchain {
                environment: HashMap::from([(
                    "PATH".to_string(),
                    format!("{}:/usr/bin:/bin", tools_dir.display()),
                )]),
                bun: Some(bun),
                node: None,
                npm: None,
            };
            assert!(install_with_tools(&tools).is_err());
            assert_eq!(
                pi_paths::runtime_executable().canonicalize().unwrap(),
                legacy.canonicalize().unwrap(),
                "a broken package must never replace the launcher"
            );
            leave_sandbox(&dir);
        }

        #[test]
        fn install_repairs_a_stale_incomplete_package_tree() {
            let _guard = env_lock();
            let dir = sandbox("stale");
            // A previous run left the package directory without dist/ — bun
            // exits 0 without re-extracting, so the installer must wipe+retry.
            let stale = pi_paths::runtime_directory()
                .join("node_modules/@earendil-works/pi-coding-agent");
            std::fs::create_dir_all(&stale).unwrap();
            std::fs::write(stale.join("package.json"), "{}").unwrap();
            let tools_dir = dir.join("tools");
            let bun = tools_dir.join("bun");
            executable(
                &bun,
                "#!/bin/sh\n\
                 if [ \"$1\" = \"add\" ]; then\n\
                   dir=\"$3\"\n\
                   pkg=\"$dir/node_modules/@earendil-works/pi-coding-agent\"\n\
                   if [ -d \"$pkg\" ]; then exit 0; fi\n\
                   mkdir -p \"$pkg/dist\"\n\
                   printf '#!/bin/sh\\necho 0.85.1\\n' > \"$pkg/dist/cli.js\"\n\
                   chmod +x \"$pkg/dist/cli.js\"\n\
                   printf '{\"name\":\"@earendil-works/pi-coding-agent\",\"version\":\"0.85.1\"}' > \"$pkg/package.json\"\n\
                   exit 0\n\
                 elif [ \"$1\" = \"--bun\" ]; then\n\
                   shift\n\
                   exec /bin/sh \"$@\"\n\
                 fi\n\
                 exit 1\n",
            );
            let tools = PiToolchain {
                environment: HashMap::from([(
                    "PATH".to_string(),
                    format!("{}:/usr/bin:/bin", tools_dir.display()),
                )]),
                bun: Some(bun),
                node: None,
                npm: None,
            };
            install_with_tools(&tools).expect("install repairs the stale tree");
            let launcher = pi_paths::runtime_executable();
            assert!(launcher.exists(), "launcher must be published");
            assert!(launcher.canonicalize().unwrap().ends_with("dist/cli.js"));
            leave_sandbox(&dir);
        }

        #[test]
        fn install_falls_back_to_npm_when_bun_lays_down_nothing() {
            let _guard = env_lock();
            let dir = sandbox("npm-fallback");
            let tools_dir = dir.join("tools");
            // bun exits 0 but creates no files — the observed failure mode.
            executable(&tools_dir.join("bun"), "#!/bin/sh\n\
                 if [ \"$1\" = \"add\" ]; then exit 0; fi\n\
                 if [ \"$1\" = \"--bun\" ]; then shift; exec /bin/sh \"$@\"; fi\n\
                 exit 1\n");
            // npm materializes the package (`install --prefix <dir> ...`).
            executable(&tools_dir.join("npm"), "#!/bin/sh\n\
                 if [ \"$1\" = \"install\" ]; then\n\
                   dir=\"$3\"\n\
                   pkg=\"$dir/node_modules/@earendil-works/pi-coding-agent\"\n\
                   mkdir -p \"$pkg/dist\"\n\
                   printf '#!/bin/sh\\necho 0.85.1\\n' > \"$pkg/dist/cli.js\"\n\
                   chmod +x \"$pkg/dist/cli.js\"\n\
                   printf '{\"name\":\"@earendil-works/pi-coding-agent\",\"version\":\"0.85.1\"}' > \"$pkg/package.json\"\n\
                   exit 0\n\
                 fi\n\
                 exit 1\n");
            let tools = PiToolchain {
                environment: HashMap::from([(
                    "PATH".to_string(),
                    format!("{}:/usr/bin:/bin", tools_dir.display()),
                )]),
                bun: Some(tools_dir.join("bun")),
                node: Some(tools_dir.join("node")),
                npm: Some(tools_dir.join("npm")),
            };
            install_with_tools(&tools).expect("npm fallback installs the runtime");
            assert!(pi_paths::runtime_executable().exists());
            leave_sandbox(&dir);
        }

        #[test]
        fn installed_version_walks_up_from_the_launcher() {
            let _guard = env_lock();
            let dir = sandbox("version");
            let pkg = pi_paths::runtime_directory()
                .join("node_modules/@earendil-works/pi-coding-agent");
            let cli = pkg.join("dist/cli.js");
            executable(&cli, "#!/bin/sh\n");
            std::fs::write(
                pkg.join("package.json"),
                "{\"name\":\"@earendil-works/pi-coding-agent\",\"version\":\"0.84.9\"}",
            )
            .unwrap();
            let bin = pi_paths::runtime_directory().join("bin");
            std::fs::create_dir_all(&bin).unwrap();
            std::os::unix::fs::symlink(&cli, pi_paths::runtime_executable()).unwrap();
            assert_eq!(installed_version().as_deref(), Some("0.84.9"));
            leave_sandbox(&dir);
        }

        #[test]
        fn auth_script_uses_util_linux_script_command_flag() {
            // Ubuntu 24.04's `script` is util-linux: the command must be
            // passed via `-c`; the BSD positional form exits with
            // "unexpected number of arguments".
            // `PI_CODING_AGENT_DIR` points at <tmp>/pi so the derived
            // pi-runtime/ sibling stays inside the temp dir.
            let _guard = env_lock();
            let generated = {
                let dir = std::env::temp_dir()
                    .join(format!("pitex-auth-test-{}", std::process::id()))
                    .join("pi");
                std::fs::create_dir_all(&dir).unwrap();
                dir
            };
            std::env::set_var("PI_CODING_AGENT_DIR", &generated);
            std::fs::create_dir_all(pi_paths::runtime_directory().join("bin")).unwrap();
            let exe = pi_paths::runtime_executable();
            std::fs::write(&exe, "#!/bin/sh\necho ok\n").unwrap();
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&exe, std::fs::Permissions::from_mode(0o755)).unwrap();
            let path = authentication_script(false).unwrap();
            let body = std::fs::read_to_string(path).unwrap();
            assert!(body.contains("script -q -c "));
            assert!(!body.contains("script -q /dev/null"));
            std::env::remove_var("PI_CODING_AGENT_DIR");
            let _ = std::fs::remove_dir_all(generated.parent().unwrap_or(&generated));
        }
    }
}

#[cfg(all(test, unix))]
mod toolchain_tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    fn executable(path: &Path, body: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, body).unwrap();
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }

    fn home(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "pitex-toolchain-{}-{tag}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// Discovery must skip the login-shell probe quickly — `/bin/false`
    /// exits non-zero so `run_probe` yields nothing.
    fn environment(extra_path: &str) -> HashMap<String, String> {
        HashMap::from([
            ("PATH".to_string(), extra_path.to_string()),
            ("SHELL".to_string(), "/bin/false".to_string()),
        ])
    }

    #[test]
    fn discover_finds_bun_in_home_dot_bun() {
        let home = home("bun");
        let bun = home.join(".bun/bin/bun");
        executable(&bun, "#!/bin/sh\n");
        let tools = PiToolchain::discover(environment("/usr/bin"), &home, &[]);
        assert_eq!(tools.bun.as_deref(), Some(bun.as_path()));
        assert!(tools
            .environment
            .get("PATH")
            .unwrap()
            .contains(home.join(".bun/bin").to_str().unwrap()));
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn discover_scans_version_managers_newest_first() {
        let home = home("nvm");
        // v9 must not shadow v18/v20 — Foundation's numeric compare orders
        // digit runs numerically, not lexicographically.
        for version in ["v9.11.2", "v18.20.4", "v20.11.0"] {
            executable(
                &home.join(format!(".nvm/versions/node/{version}/bin/node")),
                "#!/bin/sh\n",
            );
        }
        executable(
            &home.join(".nvm/versions/node/v20.11.0/bin/npm"),
            "#!/bin/sh\n",
        );
        // Ambient PATH with no node — version-manager dirs are appended
        // after it, so the newest nvm install must win.
        let tools = PiToolchain::discover(environment("/nonexistent"), &home, &[]);
        assert_eq!(
            tools.node.as_deref(),
            Some(
                home.join(".nvm/versions/node/v20.11.0/bin/node")
                    .as_path()
                )
        );
        assert_eq!(
            tools.npm.as_deref(),
            Some(
                home.join(".nvm/versions/node/v20.11.0/bin/npm")
                    .as_path()
                )
        );
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn discover_dedupes_and_filters_relative_path_entries() {
        let home = home("dedup");
        let tools = PiToolchain::discover(
            environment("/usr/bin:relative:/usr/bin:"),
            &home,
            &[],
        );
        let path = tools.environment.get("PATH").unwrap();
        let entries: Vec<&str> = path.split(':').collect();
        assert_eq!(
            entries.iter().filter(|e| **e == "/usr/bin").count(),
            1,
            "PATH must be deduplicated: {path}"
        );
        assert!(entries.iter().all(|e| e.starts_with('/')));
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn npm_command_routes_js_launcher_through_node() {
        let home = home("npmjs");
        let node = home.join("bin/node");
        let npm = home.join("bin/npm-cli.js");
        executable(&node, "#!/bin/sh\n");
        executable(&npm, "// js launcher\n");
        let tools = PiToolchain {
            environment: HashMap::new(),
            bun: None,
            node: Some(node.clone()),
            npm: Some(npm.clone()),
        };
        let (exe, args) = tools
            .npm_command(&["install".into(), "pkg".into()])
            .expect("command");
        assert_eq!(exe, node);
        assert_eq!(args[0], npm.to_string_lossy());
        assert_eq!(&args[1..], ["install", "pkg"]);
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn npm_command_uses_binary_directly() {
        let home = home("npmbin");
        let node = home.join("bin/node");
        let npm = home.join("bin/npm");
        executable(&node, "#!/bin/sh\n");
        executable(&npm, "#!/bin/sh\n");
        let tools = PiToolchain {
            environment: HashMap::new(),
            bun: None,
            node: Some(node),
            npm: Some(npm.clone()),
        };
        let (exe, args) = tools.npm_command(&["--version".into()]).expect("command");
        assert_eq!(exe, npm);
        assert_eq!(args, ["--version"]);
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn launch_prefers_bun_for_javascript_entrypoints() {
        let home = home("launchbun");
        let bun = home.join("bin/bun");
        let node = home.join("bin/node");
        let entry = home.join("rt/dist/cli.js");
        executable(&bun, "#!/bin/sh\n");
        executable(&node, "#!/bin/sh\n");
        std::fs::create_dir_all(entry.parent().unwrap()).unwrap();
        std::fs::write(&entry, "#!/usr/bin/env node\nconsole.log(1)\n").unwrap();
        let tools = PiToolchain {
            environment: HashMap::new(),
            bun: Some(bun.clone()),
            node: Some(node),
            npm: None,
        };
        let (exe, args) = tools.launch(&entry, &["--mode".into(), "rpc".into()]).unwrap();
        assert_eq!(exe, bun);
        assert_eq!(args[0], "--bun");
        assert_eq!(args[1], entry.to_string_lossy());
        assert_eq!(&args[2..], ["--mode", "rpc"]);
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn launch_uses_node_for_javascript_without_bun() {
        let home = home("launchnode");
        let node = home.join("bin/node");
        let entry = home.join("rt/cli.mjs");
        executable(&node, "#!/bin/sh\n");
        std::fs::create_dir_all(entry.parent().unwrap()).unwrap();
        std::fs::write(&entry, "console.log(1)\n").unwrap();
        let tools = PiToolchain {
            environment: HashMap::new(),
            bun: None,
            node: Some(node.clone()),
            npm: None,
        };
        let (exe, args) = tools.launch(&entry, &[]).unwrap();
        assert_eq!(exe, node);
        assert_eq!(args[0], entry.to_string_lossy());
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn launch_fails_for_javascript_without_a_runtime() {
        let home = home("launchnone");
        let entry = home.join("rt/cli.js");
        std::fs::create_dir_all(entry.parent().unwrap()).unwrap();
        std::fs::write(&entry, "console.log(1)\n").unwrap();
        let tools = PiToolchain {
            environment: HashMap::new(),
            bun: None,
            node: None,
            npm: None,
        };
        assert!(tools.launch(&entry, &[]).is_err());
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn launch_passes_native_executables_through() {
        let home = home("launchnative");
        let exe = home.join("bin/pi");
        executable(&exe, "#!/bin/sh\necho hi\n");
        let tools = PiToolchain {
            environment: HashMap::new(),
            bun: Some(home.join("bin/bun")),
            node: None,
            npm: None,
        };
        let (launched, args) = tools.launch(&exe, &["x".into()]).unwrap();
        assert_eq!(launched, exe);
        assert_eq!(args, ["x"]);
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn numeric_compare_orders_version_directories() {
        use std::cmp::Ordering;
        assert_eq!(numeric_compare("v9.11.2", "v18.20.4"), Ordering::Less);
        assert_eq!(numeric_compare("v20.11.0", "v18.20.4"), Ordering::Greater);
        assert_eq!(numeric_compare("v20.11.0", "v20.11.0"), Ordering::Equal);
        let mut versions = vec!["v9.11.2", "v20.11.0", "v18.20.4"];
        versions.sort_by(|a, b| numeric_compare(b, a));
        assert_eq!(versions, ["v20.11.0", "v18.20.4", "v9.11.2"]);
    }
}

#[cfg(test)]
mod session_history_tests {
    use super::*;

    #[test]
    fn prompt_text_strips_the_editor_envelope() {
        let wrapped = json!({"role": "user", "content": [{"type": "text",
            "text": "<editor-context>\nActive document: main.tex\n</editor-context>\n\nFix the intro"}]});
        assert_eq!(user_prompt_text(wrapped.as_object().unwrap()), "Fix the intro");
        let raw = json!({"role": "user", "content": "/skill:latex-compile"});
        assert_eq!(user_prompt_text(raw.as_object().unwrap()), "/skill:latex-compile");
    }

    #[test]
    fn summaries_use_the_first_prompt_and_skip_empty_sessions() {
        let dir = std::env::temp_dir().join(format!("pitex-sessions-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let chat = dir.join("a.jsonl");
        std::fs::write(&chat, [
            r#"{"type":"session","version":3,"id":"s","timestamp":"2026-09-23T00:00:00Z","cwd":"/p"}"#,
            r#"{"type":"message","id":"1","message":{"role":"user","content":[{"type":"text","text":"<editor-context>\nx\n</editor-context>\n\n한글 질문"}]}}"#,
            r#"{"type":"message","id":"2","message":{"role":"user","content":"second"}}"#,
        ].join("\n")).unwrap();
        let empty = dir.join("b.jsonl");
        std::fs::write(&empty, r#"{"type":"session","version":3,"id":"t"}"#).unwrap();
        assert_eq!(session_summary(&chat).map(|s| s.title), Some("한글 질문".into()));
        assert_eq!(session_summary(&empty), None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn resumed_messages_rebuild_the_live_row_shapes() {
        let agent = AgentCoordinator::new(&crate::settings::SettingsStore::new(Default::default()));
        let messages = vec![
            json!({"role": "user", "content": "Build it"}),
            json!({"role": "assistant", "content": [
                {"type": "thinking", "thinking": "plan"},
                {"type": "toolCall", "id": "c1", "name": "bash", "arguments": {"command": "latexmk"}},
            ]}),
            json!({"role": "toolResult", "toolCallId": "c1", "toolName": "bash", "isError": true,
                   "content": [{"type": "text", "text": "! Undefined control sequence"}]}),
            json!({"role": "assistant", "content": [{"type": "text", "text": "Fixed."}]}),
        ];
        let rows = agent.transcript_from_messages(&messages);
        let roles: Vec<_> = rows.iter().map(|r| r.role).collect();
        assert_eq!(roles, [TranscriptRole::User, TranscriptRole::Thinking, TranscriptRole::Tool, TranscriptRole::Assistant]);
        assert_eq!(rows[2].title, "bash");
        assert_eq!(rows[2].status, TranscriptStatus::Failed);
        assert!(rows[2].detail.contains("Undefined control sequence"));
        assert_eq!(rows[3].text, "Fixed.");
    }
}
