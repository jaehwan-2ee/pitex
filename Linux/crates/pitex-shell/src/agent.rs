//! Port of `PiRPC.swift`, `PiAgentProcess.swift`, `PiAuthStore.swift`, and
//! `AgentCoordinator.swift` — the `pi --mode rpc` subprocess plus the
//! transcript/state machine driving the assistant panel.

use std::collections::{HashMap, VecDeque};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
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
            .join(crate::identity::current().xdg_dir_name)
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
    /// Node.js that Pitex installs for itself (`pi_installer::install_pinned_node`).
    pub fn pinned_node_directory() -> PathBuf {
        runtime_directory().with_file_name("node")
    }
    /// The pinned Node's executable directory (Windows archives have no bin/).
    pub fn pinned_node_bin() -> PathBuf {
        if cfg!(windows) { pinned_node_directory() } else { pinned_node_directory().join("bin") }
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
        dir.join(format!("../share/{}/pi-runtime/bin/pi", crate::identity::current().linux_share_dir_name)),
        dir.join(format!("../lib/{}/pi-runtime/bin/pi", crate::identity::current().linux_share_dir_name)),
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
        dir.join(format!("../share/{}/{ENTRY}", crate::identity::current().linux_share_dir_name)),
        dir.join(format!("../lib/{}/{ENTRY}", crate::identity::current().linux_share_dir_name)),
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
        // Last, so a user's own Bun or Node wins over the one Pitex installed.
        paths.push(pi_paths::pinned_node_bin().to_string_lossy().into_owned());
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
        // A Node older than pi's minimum fails the install's smoke check:
        // put the pinned Node first instead, or report none so the
        // installer downloads it.
        if result.bun.is_none()
            && result.node.as_ref().is_some_and(|node| !node_meets_minimum(node, &result.environment, home))
        {
            paths.insert(0, pi_paths::pinned_node_bin().to_string_lossy().into_owned());
            update_path(&mut result.environment, &paths);
            result.node = Self::find("node", &result.environment)
                .filter(|node| node_meets_minimum(node, &result.environment, home));
            result.npm = result.node.as_ref().and_then(|_| Self::find("npm", &result.environment));
        }
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
        let java_script = ["js", "mjs", "cjs", "ts", "mts", "cts"]
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
/// pi's `engines.node` (`@earendil-works/pi-coding-agent` 1.5.2 and its
/// pi-codemode dependency both declare `>=22.19.0`).
pub const MINIMUM_NODE_VERSION: &str = "22.19.0";

/// `node --version` prints `v22.19.0`; an unreadable answer counts as too old.
fn node_version_meets_minimum(output: &str) -> bool {
    let version = output.trim().strip_prefix('v').unwrap_or("");
    version.split('.').count() == 3
        && version.split('.').all(|part| part.starts_with(|c: char| c.is_ascii_digit()))
        && !pi_installer::is_older(version, MINIMUM_NODE_VERSION)
}

fn node_meets_minimum(node: &Path, environment: &HashMap<String, String>, home: &Path) -> bool {
    run_probe(&node.to_string_lossy(), &["--version".into()], environment, home, 5)
        .is_some_and(|output| node_version_meets_minimum(&String::from_utf8_lossy(&output)))
}

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
        images: Vec<crate::composer::AgentImage>,
        streaming_behavior: Option<String>,
    },
    /// pi's own `steer`: into the running turn now (Claude Code, Codex).
    Steer {
        message: String,
        images: Vec<crate::composer::AgentImage>,
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
    /// Reply to a CLI `approval_request`. `decision` is `approve`, `deny`,
    /// or `always_allow`; the card's button id, question answers, and
    /// feedback text ride along when the card had them.
    ApprovalResponse {
        id: String,
        answer: ApprovalAnswer,
    },
    /// Bridge backends: pick a permission mode from `get_state.mode.options`.
    SetMode {
        mode_id: String,
    },
    SwitchSession {
        path: String,
    },
    /// Bridge backends with `capabilities.fork`: a new chat holding the first `keep_turns` turns.
    /// `turns` is how many prompts the chat on screen has; the bridge refuses a cut when its count differs.
    ForkSession {
        keep_turns: usize,
        turns: usize,
    },
    /// Bridge backends with `capabilities.rewind`: drop the turns after the first `keep_turns`,
    /// restoring the conversation and/or the files the agent changed; `dry_run` only lists the files.
    Rewind {
        keep_turns: usize,
        turns: usize,
        conversation: bool,
        files: bool,
        dry_run: bool,
    },
    GetMessages,
    /// Bridge backends: mirror the CLI's own sessions before the history list opens.
    RefreshSessions,
}
impl PiRPCCommand {
    pub fn object(&self) -> Value {
        match self {
            Self::Prompt {
                message,
                images,
                streaming_behavior,
            } => {
                let mut o = json!({"type": "prompt", "message": message});
                if !images.is_empty() {
                    o["images"] = images.iter().map(|i| i.wire()).collect();
                }
                if let Some(b) = streaming_behavior {
                    o["streamingBehavior"] = json!(b);
                }
                o
            }
            Self::Steer { message, images } => {
                let mut o = json!({"type": "steer", "message": message});
                if !images.is_empty() {
                    o["images"] = images.iter().map(|i| i.wire()).collect();
                }
                o
            }
            Self::Abort => json!({"type": "abort"}),
            Self::NewSession => json!({"type": "new_session"}),
            Self::SwitchSession { path } => json!({"type": "switch_session", "sessionPath": path}),
            Self::ForkSession { keep_turns, turns } => json!({"type": "fork_session", "keepTurns": keep_turns, "turns": turns}),
            Self::Rewind { keep_turns, turns, conversation, files, dry_run } => {
                let mut o = json!({"type": "rewind", "keepTurns": keep_turns, "turns": turns, "conversation": conversation, "files": files});
                if *dry_run {
                    o["dryRun"] = json!(true);
                }
                o
            }
            Self::GetMessages => json!({"type": "get_messages"}),
            Self::RefreshSessions => json!({"type": "refresh_sessions"}),
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
            Self::ApprovalResponse { id, answer } => {
                let mut o = json!({"type": "approval_response", "id": id, "decision": answer.decision});
                if let Some(action) = &answer.action_id {
                    o["actionId"] = json!(action);
                }
                if !answer.answers.is_empty() {
                    o["answers"] = Value::Object(answer.answers.clone());
                }
                if let Some(message) = answer.message.as_ref().filter(|m| !m.trim().is_empty()) {
                    o["message"] = json!(message.trim());
                }
                o
            }
            Self::SetMode { mode_id } => json!({"type": "set_mode", "modeId": mode_id}),
        }
    }
}

/// `get_state.capabilities`: may the composer steer a running turn, and attach images.
/// Defaults to nothing the backend has not promised (before the first `get_state`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct AgentCapabilities {
    pub steer: bool,
    pub images: bool,
    /// `fork_session` works; otherwise Fork quotes the earlier chat into a new one.
    pub fork: bool,
    /// What `rewind` can restore.
    pub rewind_conversation: bool,
    pub rewind_files: bool,
}

impl AgentCapabilities {
    /// The bridges report `capabilities`; pi does not, but has `steer` and takes
    /// images when the model's `input` lists them.
    pub fn from_state(data: &serde_json::Map<String, Value>) -> Self {
        if let Some(caps) = data.get("capabilities") {
            let flag = |value: Option<&Value>| value.and_then(Value::as_bool).unwrap_or(false);
            let rewind = caps.get("rewind");
            return Self {
                steer: flag(caps.get("steer")),
                images: flag(caps.get("images")),
                fork: flag(caps.get("fork")),
                rewind_conversation: flag(rewind.and_then(|r| r.get("conversation"))),
                rewind_files: flag(rewind.and_then(|r| r.get("files"))),
            };
        }
        let images = match data.get("model").and_then(|m| m.get("input")).and_then(Value::as_array) {
            Some(input) => input.iter().any(|kind| kind.as_str() == Some("image")),
            None => true,
        };
        Self { steer: true, images, ..Default::default() }
    }
}

/// The rewind confirmation the panel shows for one message.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct RewindOffer {
    /// Whole turns kept (the `rewind.keepTurns`).
    pub keep_turns: usize,
    /// A rewound prompt goes back into the composer.
    pub draft: Option<String>,
    /// Files a file rewind would restore; None while the dry run is out.
    pub files: Option<Vec<String>>,
    /// Why files cannot be restored (no checkpoint, outside the project, unsupported).
    pub files_note: Option<String>,
    /// The dry run, then the confirmed rewind, in flight.
    pub request_id: Option<String>,
}

impl RewindOffer {
    /// The confirmed rewind is out (the dry run, if any, has answered).
    pub fn is_rewinding(&self) -> bool {
        self.files.is_some() && self.request_id.is_some()
    }
}

/// One CLI approval the transcript is waiting on. pi does not use this.
/// `kind` is command, file, permissions, plan, or question.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AgentApprovalRequest {
    pub id: String,
    pub kind: String,
    pub title: String,
    pub detail: String,
    /// The typed tool detail (the edit diff, the command line).
    pub tool: Option<ToolDetail>,
    /// Buttons from the bridge; empty means Approve / Deny / Always allow.
    pub actions: Vec<ApprovalAction>,
    pub questions: Vec<ApprovalQuestion>,
    /// Markdown of a plan to review.
    pub plan: Option<String>,
}

/// One button on an approval card.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalAction {
    pub id: String,
    pub label: String,
    pub allow: bool,
    /// primary, secondary, danger, or empty.
    pub variant: String,
}

/// One AskUserQuestion item: choose an option (or several), or type "Other".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalQuestion {
    pub id: String,
    pub header: String,
    pub question: String,
    /// (label, description)
    pub options: Vec<(String, String)>,
    pub multi_select: bool,
    pub allow_other: bool,
}

/// What the user picked on the card on screen.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ApprovalAnswer {
    /// approve, deny, or always_allow.
    pub decision: String,
    pub action_id: Option<String>,
    /// Question id → label string, or an array of labels for multi-select.
    pub answers: Map<String, Value>,
    pub message: Option<String>,
}

impl ApprovalAnswer {
    pub fn decision(decision: &str) -> Self {
        Self { decision: decision.to_string(), ..Default::default() }
    }

    /// A card button: its id goes along with the matching legacy decision.
    pub fn action(action: &ApprovalAction, message: Option<String>) -> Self {
        let decision = match action.id.as_str() {
            "approve" | "deny" | "always_allow" => action.id.as_str(),
            _ if action.allow => "approve",
            _ => "deny",
        };
        Self { decision: decision.into(), action_id: Some(action.id.clone()), message, ..Default::default() }
    }
}

impl AgentApprovalRequest {
    pub fn decode(object: &Map<String, Value>) -> Self {
        let text = |value: Option<&Value>| value.and_then(|v| v.as_str()).unwrap_or_default().trim().to_string();
        let array = |key: &str| object.get(key).and_then(|v| v.as_array()).cloned().unwrap_or_default();
        let id = text(object.get("id"));
        let actions = array("actions")
            .iter()
            .filter_map(|row| {
                let id = text(row.get("id"));
                let behavior = text(row.get("behavior"));
                if id.is_empty() || !matches!(behavior.as_str(), "allow" | "deny") {
                    return None;
                }
                let label = text(row.get("label"));
                Some(ApprovalAction {
                    label: if label.is_empty() { id.clone() } else { label },
                    id,
                    allow: behavior == "allow",
                    variant: text(row.get("variant")),
                })
            })
            .collect();
        let questions = array("questions")
            .iter()
            .filter_map(|row| {
                let question = text(row.get("question"));
                let id = text(row.get("id"));
                if question.is_empty() || id.is_empty() {
                    return None;
                }
                let options = row
                    .get("options")
                    .and_then(|v| v.as_array())
                    .map(|options| {
                        options
                            .iter()
                            .map(|o| (text(o.get("label")), text(o.get("description"))))
                            .filter(|(label, _)| !label.is_empty())
                            .collect()
                    })
                    .unwrap_or_default();
                Some(ApprovalQuestion {
                    id,
                    header: text(row.get("header")),
                    question,
                    options,
                    multi_select: row.get("multiSelect").and_then(|v| v.as_bool()).unwrap_or(false),
                    allow_other: row.get("allowOther").and_then(|v| v.as_bool()).unwrap_or(false),
                })
            })
            .collect();
        let title = text(object.get("title"));
        Self {
            tool: object.get("toolDetail").and_then(|v| v.as_object()).and_then(|d| ToolDetail::decode(&id, d)),
            id,
            kind: Some(text(object.get("kind"))).filter(|k| !k.is_empty()).unwrap_or_else(|| "command".into()),
            title: if title.is_empty() { "Approval".into() } else { title },
            detail: text(object.get("detail")),
            actions,
            questions,
            plan: object.get("plan").and_then(|v| v.as_str()).filter(|p| !p.trim().is_empty()).map(|p| bounded(p, 32_000)),
        }
    }
}

/// One row of the agent's todo list (`todo_update.items`, `get_messages.todos`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TodoItem {
    pub id: Option<String>,
    pub text: String,
    pub status: TodoStatus,
    pub active_form: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TodoStatus {
    Pending,
    InProgress,
    Completed,
}

/// One transcript line for a change between two todo snapshots (Paseo's task activity).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TodoActivity {
    Created(usize),
    Added(String),
    Started(String),
    Completed(String),
}

impl TodoActivity {
    /// The `assistant.todo.<kind>` string key and its argument.
    pub fn key_and_arg(&self) -> (&'static str, String) {
        match self {
            Self::Created(count) => ("assistant.todo.created", count.to_string()),
            Self::Added(text) => ("assistant.todo.added", text.clone()),
            Self::Started(text) => ("assistant.todo.started", text.clone()),
            Self::Completed(text) => ("assistant.todo.completed", text.clone()),
        }
    }
}

impl TodoItem {
    /// The bridge's list; rows without text are dropped.
    pub fn decode_list(value: Option<&Value>) -> Vec<Self> {
        let Some(rows) = value.and_then(|v| v.as_array()) else { return Vec::new() };
        let text = |row: &Value, key: &str| {
            row.get(key).and_then(|v| v.as_str()).map(str::trim).filter(|t| !t.is_empty()).map(str::to_string)
        };
        rows.iter()
            .filter_map(|row| {
                Some(Self {
                    id: text(row, "id"),
                    text: text(row, "text")?,
                    status: match row.get("status").and_then(|v| v.as_str()) {
                        Some("completed") => TodoStatus::Completed,
                        Some("in_progress") => TodoStatus::InProgress,
                        _ => TodoStatus::Pending,
                    },
                    active_form: text(row, "activeForm"),
                })
            })
            .collect()
    }
}

/// What changed from `previous` to `current`: a new list, new rows, and rows that
/// started or finished. Rows match by id, else by text (the first unmatched row
/// with it), so a dropped or moved row does not shift the rest. A list that shares
/// no row with the previous one is a new list.
pub fn todo_activities(previous: &[TodoItem], current: &[TodoItem]) -> Vec<TodoActivity> {
    if current.is_empty() {
        return Vec::new();
    }
    let mut unmatched: Vec<&TodoItem> = previous.iter().collect();
    let activities: Vec<TodoActivity> = current
        .iter()
        .filter_map(|item| {
            let found = unmatched.iter().position(|was| match (&was.id, &item.id) {
                (Some(a), Some(b)) => a == b,
                (None, None) => was.text == item.text,
                _ => false,
            });
            let was = found.map(|at| unmatched.remove(at).status);
            match (was, item.status) {
                (None, _) => Some(TodoActivity::Added(item.text.clone())),
                (Some(was), now) if was == now => None,
                (Some(_), TodoStatus::Completed) => Some(TodoActivity::Completed(item.text.clone())),
                (Some(_), TodoStatus::InProgress) => Some(TodoActivity::Started(item.text.clone())),
                (Some(_), TodoStatus::Pending) => None,
            }
        })
        .collect();
    if unmatched.len() == previous.len() {
        return vec![TodoActivity::Created(current.len())];
    }
    activities
}

/// (done, total) for the checklist header and its accessible label.
pub fn todo_progress(items: &[TodoItem]) -> (usize, usize) {
    (items.iter().filter(|item| item.status == TodoStatus::Completed).count(), items.len())
}

/// What the agent is doing now: the first in-progress row's active form, or its text.
pub fn todo_current(items: &[TodoItem]) -> Option<&str> {
    let item = items.iter().find(|item| item.status == TodoStatus::InProgress)?;
    Some(item.active_form.as_deref().unwrap_or(&item.text))
}

/// The bridge's permission mode (`get_state.mode`, `mode_changed.mode`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AgentMode {
    pub id: String,
    /// (id, label) in the bridge's order.
    pub options: Vec<(String, String)>,
}

impl AgentMode {
    pub fn decode(value: Option<&Value>) -> Option<Self> {
        let object = value?.as_object()?;
        let id = object.get("id")?.as_str()?.to_string();
        let options: Vec<(String, String)> = object
            .get("options")?
            .as_array()?
            .iter()
            .filter_map(|o| {
                let id = o.get("id")?.as_str()?.to_string();
                let label = o.get("label").and_then(|v| v.as_str()).unwrap_or(&id).to_string();
                Some((id, label))
            })
            .collect();
        (!options.is_empty()).then_some(Self { id, options })
    }

    /// Localization key for a mode (Claude's or agy's spelling), if known.
    /// Only Claude's default asks before edits (the bridge labels it so);
    /// agy's default edits project files without asking, so it reads "Default".
    pub fn label_key(id: &str, label: &str) -> Option<&'static str> {
        match id {
            "default" if label == "Ask before edits" => Some("assistant.mode.default"),
            "default" => Some("assistant.mode.cli_default"),
            "acceptEdits" | "accept-edits" => Some("assistant.mode.accept_edits"),
            "plan" => Some("assistant.mode.plan"),
            _ => None,
        }
    }

    /// Edits apply without asking (accept-edits, or agy's default): the
    /// picker is tinted so it stays visible.
    pub fn auto_applies_edits(&self) -> bool {
        let label = self.options.iter().find(|(id, _)| *id == self.id).map_or("", |(_, label)| label.as_str());
        matches!(
            Self::label_key(&self.id, label),
            Some("assistant.mode.accept_edits" | "assistant.mode.cli_default")
        )
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
    /// Claude Code's `argumentHint` ("<optional instructions>").
    pub argument_hint: Option<String>,
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
            argument_hint: d
                .get("argumentHint")
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
    /// Commands queue to a writer thread: a child that stops reading stdin
    /// must not block the UI thread that sends.
    stdin: std::sync::mpsc::Sender<Vec<u8>>,
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
        // Own the tool subprocesses too, so a forced SSH Save can stop all
        // writers before taking its final snapshot.
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            command.process_group(0);
        }
        // The agent is a background RPC peer — no console window.
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
        }
        let mut child = command.spawn()?;
        let mut stdin = child.stdin.take().expect("piped stdin");
        let stdout = child.stdout.take().expect("piped stdout");
        let stderr = child.stderr.take().expect("piped stderr");

        let (event_tx, event_rx) = channel::<PiRPCEvent>();
        let (exit_tx, exit_rx) = channel::<()>();
        let stderr_tail = Arc::new(Mutex::new(Vec::<u8>::new()));

        let stdout_thread = std::thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            while let Some(line) = next_agent_stdout_line(&mut reader, 4 * 1024 * 1024) {
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
        let (stdin_tx, stdin_rx) = channel::<Vec<u8>>();
        let writer_child = child_arc.clone();
        std::thread::spawn(move || {
            for data in stdin_rx {
                if stdin.write_all(&data).and_then(|_| stdin.flush()).is_err() {
                    // A child that closed stdin cannot take commands: end it so the exit path settles them.
                    let _ = writer_child.lock().unwrap().kill();
                    break;
                }
            }
        });
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
            stdin: stdin_tx,
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
        self.stdin
            .send(data)
            .map_err(|_| "The pi agent process is not running.".into())
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
        #[cfg(unix)]
        unsafe { libc::kill(-(guard.id() as i32), libc::SIGKILL); }
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            let _ = Command::new("taskkill")
                .args(["/PID", &guard.id().to_string(), "/T", "/F"])
                .creation_flags(0x0800_0000)
                .status();
        }
        let _ = guard.kill();
        // Forced SSH Save must snapshot only after the coordinator stops.
        let _ = guard.wait();
    }
}

#[cfg(all(test, unix))]
mod process_group_tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn terminating_agent_stops_descendant_file_writes() {
        let directory = std::env::temp_dir().join(format!(
            "pitex-agent-group-{}-{}", std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
        ));
        std::fs::create_dir(&directory).unwrap();
        let ready = directory.join("ready");
        let late_write = directory.join("late-write");
        let process = PiAgentProcess::start(
            Path::new("/bin/sh"),
            &directory,
            &[
                "-c".into(),
                "(sleep 0.3; printf 'late write' > \"$1\") & printf 'ready' > \"$2\"; wait".into(),
                "pitex-test".into(),
                late_write.to_string_lossy().into_owned(),
                ready.to_string_lossy().into_owned(),
            ],
            &std::env::vars().collect(),
        ).unwrap();
        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        while !ready.exists() && std::time::Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(5));
        }
        process.terminate();
        assert!(ready.exists(), "the agent started its background writer");
        assert!(!process.is_running(), "termination waits for the agent to exit");
        process.exited.lock().unwrap().recv_timeout(Duration::from_secs(2)).unwrap();
        std::thread::sleep(Duration::from_millis(400));
        assert!(!late_write.exists(), "a tool descendant must not write after forced Save");
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn a_prompt_larger_than_the_pipe_does_not_block_while_the_agent_stops_reading() {
        let process = Arc::new(PiAgentProcess::start(
            Path::new("/bin/sh"),
            &std::env::temp_dir(),
            &["-c".into(), "sleep 30".into()],
            &std::env::vars().collect(),
        ).unwrap());
        let command = PiRPCCommand::Prompt {
            message: "x".repeat(4 * 1024 * 1024),
            images: Vec::new(),
            streaming_behavior: None,
        };
        let (done_tx, done_rx) = channel();
        let sender = process.clone();
        std::thread::spawn(move || { let _ = done_tx.send(sender.send(&command, None)); });
        let returned = done_rx.recv_timeout(Duration::from_secs(5));
        process.terminate();
        assert_eq!(returned, Ok(Ok(())), "send must return while the agent withholds stdin reads");
        process.exited.lock().unwrap().recv_timeout(Duration::from_secs(5)).unwrap();
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
    /// A todo-list change. `title` holds the `assistant.todo.*` key and `text` its argument,
    /// so the row is localized when drawn.
    Activity,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TranscriptStatus {
    Streaming,
    Running,
    Done,
    Failed,
    /// A tool call cut off by stop/interrupt (`status: "canceled"`).
    Canceled,
}
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AgentTranscriptEntry {
    pub role: TranscriptRole,
    pub title: String,
    pub text: String,
    pub detail: String,
    pub status: TranscriptStatus,
    /// Tool-call detail; `kind: "other"` with only the call id for pi's own
    /// provider. `None` on non-tool rows and history-rebuilt tool rows.
    pub tool: Option<ToolDetail>,
    /// Images the user sent with this message (thumbnails in the transcript).
    pub images: Vec<crate::composer::AgentImage>,
    pub meta: EntryMeta,
}

/// Row state beyond role and status: how long a thinking block took, or a failed turn.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub enum EntryMeta {
    #[default]
    None,
    /// Seconds from the first reasoning delta to the next non-reasoning event.
    Thought { seconds: Option<u64> },
    /// On a prompt: how long its turn ran (the folded turn's "Worked for").
    Worked { seconds: u64 },
    /// `text` is the raw error. `backend` is the provider id for the wording;
    /// `retry` is true on the newest failure while its message can be resent.
    TurnError { code: crate::turn_state::ErrorCode, backend: String, can_compact: bool, retry: bool },
}

/// Cap for the diff text a tool card keeps (the bridge caps at the same size).
pub const TOOL_DIFF_CAP: usize = 12_000;

/// The optional `detail` object on `tool_execution_*` events. `oldText` and
/// `newText` are not kept: the diff carries the change.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct ToolDetail {
    pub call_id: String,
    /// shell, read, edit, write, search, fetch, sub_agent, think or other.
    pub kind: String,
    pub path: Option<String>,
    pub command: Option<String>,
    pub cwd: Option<String>,
    pub exit_code: Option<i64>,
    pub diff: Option<String>,
    pub query: Option<String>,
    pub url: Option<String>,
    pub num_matches: Option<i64>,
    pub label: Option<String>,
    pub start_line: Option<i64>,
    pub end_line: Option<i64>,
    /// The Task/Agent call this one ran inside (`parentToolCallId`).
    pub parent_call_id: Option<String>,
    /// sub_agent: the agent type and how many calls it started.
    pub sub_agent_type: Option<String>,
    pub tool_uses: Option<i64>,
}

impl ToolDetail {
    pub fn decode(call_id: &str, object: &Map<String, Value>) -> Option<Self> {
        let text = |key: &str| {
            object
                .get(key)
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .map(String::from)
        };
        let int = |key: &str| object.get(key).and_then(|v| v.as_i64());
        Some(Self {
            call_id: call_id.to_string(),
            kind: text("kind")?,
            path: text("path"),
            command: text("command").map(|c| bounded(&c, 2_000)),
            cwd: text("cwd"),
            exit_code: int("exitCode"),
            diff: text("diff").map(|d| bounded_diff(&d)),
            query: text("query"),
            url: text("url"),
            num_matches: int("numMatches"),
            label: text("label"),
            start_line: int("startLine"),
            end_line: int("endLine"),
            parent_call_id: None,
            sub_agent_type: text("subAgentType"),
            tool_uses: int("toolUses"),
        })
    }

    /// Paseo's merge rule: a later event fills in fields; an untyped
    /// (`other`) detail never replaces a typed one.
    pub fn merge(&mut self, incoming: ToolDetail) {
        if incoming.kind == "other" && self.kind != "other" {
            return;
        }
        self.kind = incoming.kind;
        macro_rules! take {
            ($($field:ident),*) => { $(if incoming.$field.is_some() { self.$field = incoming.$field; })* };
        }
        take!(path, command, cwd, exit_code, diff, query, url, num_matches, label, start_line, end_line);
        take!(parent_call_id, sub_agent_type, tool_uses);
    }

    /// No typed fields for the card body (pi's provider, MCP tools, sub-agents).
    pub fn is_untyped(&self) -> bool {
        matches!(self.kind.as_str(), "other" | "sub_agent")
    }

    /// Added and removed line counts of the diff.
    pub fn diff_counts(&self) -> (usize, usize) {
        let Some(diff) = &self.diff else { return (0, 0) };
        let mut counts = (0, 0);
        let mut in_hunk = false;
        for line in diff.lines() {
            if line.starts_with("@@") {
                in_hunk = true;
            } else if in_hunk && line.starts_with('+') {
                counts.0 += 1;
            } else if in_hunk && line.starts_with('-') {
                counts.1 += 1;
            }
        }
        counts
    }

    /// The file path relative to `root` when inside it, else as given.
    pub fn display_path(&self, root: Option<&Path>) -> String {
        let Some(path) = &self.path else { return String::new() };
        root.and_then(|root| Path::new(path).strip_prefix(root).ok())
            .filter(|rel| !rel.as_os_str().is_empty())
            .map(|rel| rel.display().to_string())
            .unwrap_or_else(|| path.clone())
    }

    /// The absolute file this card can open in the editor: inside `root` only.
    pub fn openable_path(&self, root: Option<&Path>) -> Option<PathBuf> {
        if !matches!(self.kind.as_str(), "edit" | "write" | "read") {
            return None;
        }
        let root = root?;
        let path = Path::new(self.path.as_ref()?);
        let path = if path.is_absolute() { path.to_path_buf() } else { root.join(path) };
        let canonical = |p: &Path| std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
        canonical(&path).starts_with(canonical(root)).then_some(path)
    }

    /// One-line card title, e.g. `Ran latexmk -pdf main.tex`,
    /// `Read main.tex (lines 10-40)`, `Searched "\cite" · 12 matches`.
    pub fn title(&self, fallback: &str, root: Option<&Path>, lang: &str) -> String {
        use crate::l10n::{tr, tr1};
        let path = self.display_path(root);
        let first_line = |s: &str| s.lines().next().unwrap_or_default().to_string();
        match self.kind.as_str() {
            "shell" => tr1(lang, "assistant.tool.ran", &first_line(self.command.as_deref().unwrap_or(fallback))),
            "edit" if !path.is_empty() => tr1(lang, "assistant.tool.edited", &path),
            "write" if !path.is_empty() => tr1(lang, "assistant.tool.wrote", &path),
            "read" if !path.is_empty() => {
                let target = match (self.start_line, self.end_line) {
                    (Some(start), Some(end)) => {
                        format!("{path} ({})", tr1(lang, "assistant.tool.lines", &format!("{start}-{end}")))
                    }
                    (Some(start), None) => format!("{path} ({})", tr1(lang, "assistant.tool.lines", &format!("{start}-"))),
                    _ => path,
                };
                tr1(lang, "assistant.tool.read", &target)
            }
            "search" => {
                let query = self.query.as_deref().or(self.label.as_deref()).unwrap_or(fallback);
                let mut title = tr1(lang, "assistant.tool.searched", &format!("\"{query}\""));
                if let Some(count) = self.num_matches {
                    title.push_str(" · ");
                    title.push_str(&tr1(lang, "assistant.tool.matches", &count.to_string()));
                }
                title
            }
            "fetch" => {
                let url = self.url.as_deref().unwrap_or(fallback);
                let host = url
                    .split("://")
                    .nth(1)
                    .and_then(|rest| rest.split('/').next())
                    .filter(|host| !host.is_empty())
                    .unwrap_or(url);
                tr1(lang, "assistant.tool.fetched", host)
            }
            "sub_agent" => tr1(lang, "assistant.tool.subagent", self.label.as_deref().unwrap_or(fallback)),
            "think" => self.label.clone().unwrap_or_else(|| tr(lang, "assistant.tool.thought")),
            _ => self.label.clone().unwrap_or_else(|| fallback.to_string()),
        }
    }

    /// The header badge: `exit 1` for a failing command, `+3 -1` for a diff.
    pub fn badge(&self, lang: &str) -> Option<String> {
        if self.kind == "sub_agent" {
            return self.sub_agent_type.clone();
        }
        if self.kind == "shell" {
            return self
                .exit_code
                .filter(|code| *code != 0)
                .map(|code| crate::l10n::tr1(lang, "assistant.tool.exit", &code.to_string()));
        }
        let (added, removed) = self.diff_counts();
        (added + removed > 0).then(|| format!("+{added} -{removed}"))
    }
}

fn bounded_diff(diff: &str) -> String {
    if diff.len() <= TOOL_DIFF_CAP {
        return diff.to_string();
    }
    let mut end = TOOL_DIFF_CAP;
    while !diff.is_char_boundary(end) {
        end -= 1;
    }
    let end = diff[..end].rfind('\n').unwrap_or(end);
    // A leading backslash is a meta line for the diff parser.
    format!("{}\n\\ diff truncated", &diff[..end])
}

fn tool_status(event: &PiRPCEvent) -> Option<TranscriptStatus> {
    status_named(&event.string("status")?)
}

/// The bridge's `status` (live events and replayed `toolResult` rows).
fn status_named(status: &str) -> Option<TranscriptStatus> {
    match status {
        "running" => Some(TranscriptStatus::Running),
        "completed" => Some(TranscriptStatus::Done),
        "failed" => Some(TranscriptStatus::Failed),
        "canceled" => Some(TranscriptStatus::Canceled),
        _ => None,
    }
}

/// A saved conversation for the current project — one pi session file.
#[derive(Debug, Clone, PartialEq)]
pub struct AgentSessionSummary {
    pub path: PathBuf,
    pub modified: std::time::SystemTime,
    pub title: String,
    /// The backend that owns it (`session_source`): pi, claude, agy, codex, or grok.
    pub source: &'static str,
}

/// Bridge mirrors are named `<backend>-<id>.jsonl`; any other file is pi's own.
pub fn session_source(path: &Path) -> &'static str {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    ["claude", "agy", "codex", "grok"]
        .into_iter()
        .find(|id| name.strip_prefix(id).is_some_and(|rest| rest.starts_with('-')))
        .unwrap_or("pi")
}

/// The history row badge: product names, the same in every language.
pub fn session_source_label(source: &str) -> &'static str {
    match source {
        "claude" => "Claude Code",
        "agy" => "Antigravity",
        "codex" => "Codex",
        "grok" => "Grok",
        _ => "pi",
    }
}

/// "Just now", "5 min ago", "3 h ago", "Yesterday", "4 days ago", then the date.
pub fn relative_time(lang: &str, now: std::time::SystemTime, then: std::time::SystemTime) -> String {
    let seconds = now.duration_since(then).map(|d| d.as_secs()).unwrap_or(0);
    let count = |key: &str, n: u64| crate::l10n::trn(lang, key, &[&n.to_string()]);
    match seconds {
        0..=59 => crate::l10n::tr(lang, "assistant.time_now"),
        60..=3_599 => count("assistant.time_minutes", seconds / 60),
        3_600..=86_399 => count("assistant.time_hours", seconds / 3_600),
        86_400..=172_799 => crate::l10n::tr(lang, "assistant.time_yesterday"),
        172_800..=604_799 => count("assistant.time_days", seconds / 86_400),
        _ => {
            let secs = then.duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);
            gtk4::glib::DateTime::from_unix_local(secs)
                .and_then(|d| d.format("%Y-%m-%d"))
                .map(|s| s.to_string())
                .unwrap_or_default()
        }
    }
}

/// The history search: every word of `query` appears in the title, ignoring case.
pub fn session_matches(title: &str, query: &str) -> bool {
    let title = title.to_lowercase();
    query.to_lowercase().split_whitespace().all(|word| title.contains(word))
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
        source: session_source(path),
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

/// Resolved command line for one agent spawn.
struct AgentLaunch {
    executable: PathBuf,
    arguments: Vec<String>,
    environment: HashMap<String, String>,
}

/// The bundled CLI bridge (`PitexAgent/bridge/cli.ts`), next to the install
/// or the repo's Mac resources during development. Same lookup as skills.
fn bridge_script() -> Option<PathBuf> {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            for rel in [
                format!(
                    "../share/{}/PitexAgent/bridge/cli.ts",
                    crate::identity::current().linux_share_dir_name
                ),
                format!(
                    "../../share/{}/PitexAgent/bridge/cli.ts",
                    crate::identity::current().linux_share_dir_name
                ),
            ] {
                let candidate = dir.join(rel);
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }
    }
    let dev = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../Mac/Resources/PitexAgent/bridge/cli.ts");
    if dev.is_file() {
        return Some(dev);
    }
    None
}

fn bridge_missing_message(provider: &str) -> String {
    if provider == "claude" {
        "The Claude Code bridge is not installed with this Pitex build.".to_string()
    } else if provider == "grok" {
        "The Grok CLI bridge is not installed with this Pitex build.".to_string()
    } else if provider == "agy" {
        "The Antigravity CLI bridge is not installed with this Pitex build.".to_string()
    } else {
        "The Codex bridge is not installed with this Pitex build.".to_string()
    }
}

/// The coordinator is main-thread-owned like the Swift `@MainActor` class.
/// The process's event receiver is drained by the UI's idle loop.
pub struct AgentCoordinator {
    pub connection: Connection,
    pub transcript: Vec<AgentTranscriptEntry>,
    /// The history list (`/resume` or the clock button), newest first.
    pub past_sessions: Vec<AgentSessionSummary>,
    /// `/resume` was typed or the history button clicked, and the list is
    /// fresh — the panel opens the history popover.
    pub history_requested: bool,
    /// The `refresh_sessions` reply the history list waits for.
    refresh_request_id: Option<String>,
    /// A history row of another backend: opened once that backend is up.
    pending_resume: Option<PathBuf>,
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
    /// Latest pushed usage (`usage_update`); the meter falls back to `session_stats` (pi).
    pub usage: crate::turn_state::Usage,
    /// The model that wrote the current answer (`message_start.message.modelName`).
    live_model_name: Option<String>,
    /// When the open thinking block got its first delta.
    thinking_started: Option<std::time::Instant>,
    /// The last message sent as a prompt, with its images: what Retry resends.
    last_sent: Option<(String, Vec<crate::composer::AgentImage>)>,
    /// Retry after a crash: the message goes out once the restarted process is up.
    pending_retry: bool,
    /// `get_commands` payload — slash commands the composer completes.
    pub commands: Vec<PiSlashCommand>,
    pub status_message: Option<String>,
    agent_provider: String,
    /// Unanswered CLI approvals, oldest first. The panel shows the head.
    approval_queue: VecDeque<AgentApprovalRequest>,
    /// Approval replies the coordinator tried to send. Tests read this
    /// because a unit test has no child process to capture stdin.
    #[cfg(test)]
    approval_sent: Vec<(String, String)>,
    #[cfg(test)]
    approval_wire: Vec<Value>,
    #[cfg(test)]
    prompt_wire: Vec<Value>,
    /// The approval the panel is showing. Further requests wait behind it.
    pub pending_approval: Option<AgentApprovalRequest>,
    /// Bridge backends' permission mode; None hides the picker (pi).
    pub mode: Option<AgentMode>,
    /// The conversation's todo list; empty hides the checklist.
    pub todos: Vec<TodoItem>,
    pub attach_active_document: bool,
    pub pending_composer_insertion: Option<String>,
    pub selection_attachment: Option<AgentSelectionAttachment>,
    suppressed_attachment: Option<AgentSelectionAttachment>,
    /// Tracks the in-flight model-settings request; replies echoing a
    /// different id are stale and ignored. Public for integration tests.
    #[doc(hidden)]
    pub model_settings_request_id: Option<String>,
    /// The get_messages request still in force. An older reply must not
    /// replace the transcript of the session now on screen.
    messages_request_id: Option<String>,
    /// Bumped on every mutation of UI-visible state; the panel rebuilds
    /// only when this changes instead of on every agent event.
    pub ui_revision: u64,
    /// `get_state.capabilities`; pi omits it (see `AgentCapabilities::from_state`).
    pub capabilities: AgentCapabilities,
    /// The rewind confirmation on screen (Rewind to here).
    pub rewind_offer: Option<RewindOffer>,
    /// The `fork_session` in flight, with the prompt a fork from a user message puts back.
    fork_request: Option<(String, Option<String>)>,
    /// Replaces the composer text once (a forked or rewound prompt, a quoted chat).
    pub composer_replacement: Option<String>,
    /// When the running turn started (`agent_start`); its prompt row gets `Worked`.
    turn_started: Option<std::time::Instant>,
    /// Prompt rows trimmed off the top by `history_limit`, so turn counts stay right.
    dropped_prompts: usize,
    /// A fork or rewind notice, shown under the chat once it is reloaded.
    notice_after_reload: Option<String>,
    /// Messages typed while a turn runs, sent one per agent_end.
    pub queue: crate::composer::MessageQueue,
    /// The `prompt` id of the queued message in flight, and that message's queue id.
    drain_request: Option<(String, u64)>,
    /// Images attached to the draft, shown as chips above the composer.
    pub draft_images: Vec<crate::composer::AgentImage>,
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
    catalog_slots: Vec<crate::catalog::Slot>,
    catalog_generation: u64,
    switch_ticket: Option<SwitchTicket>,
    catalog_tools: Option<PiToolchain>,
    catalog_pi: Option<PathBuf>,
    catalog_root: Option<PathBuf>,
    probes: Vec<CatalogProbe>,
    probe_generation: u64,
    /// Why a backend's catalog failed: the bridge's text, or None for the
    /// generic "could not start". Shown until the backend lists models; a
    /// re-armed (Pending) slot keeps showing it while it is checked again.
    catalog_reasons: HashMap<String, Option<String>>,
    /// The new session's model list arrived while a settings round trip was
    /// still in flight. Commit it once that round trip goes idle.
    catalog_commit_pending: bool,
    launch_idle: bool,
    #[cfg(test)]
    model_requests: Vec<(String, String)>,
}

#[derive(Clone)]
struct SwitchTicket {
    backend: String,
    model: Option<crate::catalog::CatalogModel>,
}

struct CatalogProbe {
    backend: String,
    generation: u64,
    process: PiAgentProcess,
    started: std::time::Instant,
    sessions: std::path::PathBuf,
    /// The first `extension_error` the probe printed (e.g. the CLI is missing).
    error: Option<String>,
}

const CATALOG_PROBE_LIMIT: std::time::Duration = std::time::Duration::from_secs(15);

fn catalog_models_from_event(
    event: &PiRPCEvent,
) -> Option<Result<Vec<crate::catalog::CatalogModel>, Option<String>>> {
    if event.event_type != "response"
        || event.response_command().as_deref() != Some("get_available_models")
    {
        return None;
    }
    if !event.response_succeeded() {
        return Some(Err(event.response_error()));
    }
    let raw = event
        .response_data()
        .and_then(|data| data.get("models"))
        .and_then(|models| models.as_array())
        .cloned()
        .unwrap_or_default();
    let models: Vec<_> = raw
        .iter()
        .filter_map(PiModelDescriptor::from_value)
        .map(|model| crate::catalog::CatalogModel {
            id: model.id,
            provider: model.provider,
            name: model.name,
            context_window: model.context_window,
        })
        .collect();
    if models.is_empty() {
        Some(Err(None))
    } else {
        Some(Ok(models))
    }
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
            refresh_request_id: None,
            pending_resume: None,
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
            agent_provider: store.agent_provider(),
            approval_queue: VecDeque::new(),
            #[cfg(test)]
            approval_sent: Vec::new(),
            #[cfg(test)]
            approval_wire: Vec::new(),
            #[cfg(test)]
            prompt_wire: Vec::new(),
            pending_approval: None,
            mode: None,
            todos: Vec::new(),
            attach_active_document: store
                .prefs()
                .bool(Self::ATTACH_DOCUMENT_KEY)
                .unwrap_or(true),
            pending_composer_insertion: None,
            selection_attachment: None,
            suppressed_attachment: None,
            model_settings_request_id: None,
            messages_request_id: None,
            ui_revision: 0,
            capabilities: AgentCapabilities::default(),
            rewind_offer: None,
            fork_request: None,
            composer_replacement: None,
            turn_started: None,
            dropped_prompts: 0,
            notice_after_reload: None,
            queue: Default::default(),
            drain_request: None,
            draft_images: Vec::new(),
            wants_connection: false,
            pdf_text_cache: std::cell::RefCell::new(None),
            session_stats: None,
            usage: Default::default(),
            live_model_name: None,
            thinking_started: None,
            last_sent: None,
            pending_retry: false,
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
            catalog_slots: crate::catalog::gtk_slots(),
            catalog_generation: 1,
            switch_ticket: None,
            catalog_tools: None,
            catalog_pi: None,
            catalog_root: None,
            probes: Vec::new(),
            probe_generation: 1,
            catalog_reasons: HashMap::new(),
            catalog_commit_pending: false,
            launch_idle: false,
            #[cfg(test)]
            model_requests: Vec::new(),
        }
    }

    /// The active backend id (`pi`, `claude`, `agy`, ...).
    pub(crate) fn provider(&self) -> &str {
        &self.agent_provider
    }

    pub fn set_agent_provider(&mut self, value: &str) {
        self.agent_provider = crate::settings::normalize_agent_provider(Some(value)).into();
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
        if self.launch_idle {
            return;
        }
        if matches!(self.connection, Connection::Connecting | Connection::Ready) {
            if crate::catalog::rearm_unavailable(&mut self.catalog_slots) {
                self.refresh_probes();
            }
            return;
        }
        crate::catalog::rearm_unavailable(&mut self.catalog_slots);
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
        self.catalog_tools = Some(tools.clone());
        self.catalog_pi = executable.clone();
        self.catalog_root = Some(root.clone());
        if self.intentional_stop {
            return;
        }
        if crate::settings::agent_provider_is_on_device(&self.agent_provider) {
            self.fail_connection(crate::settings::ON_DEVICE_UNAVAILABLE_MESSAGE.into());
        } else if crate::settings::agent_provider_uses_bridge(&self.agent_provider) {
            self.spawn_with_tools(Path::new(""), &root, tools);
        } else {
            match executable {
                Some(executable) => self.spawn_with_tools(&executable, &root, tools),
                None => {
                    self.connection = Connection::PiMissing;
                    self.status_message = None;
                }
            }
        }
        self.refresh_probes();
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
        let plan = match Self::plan_launch(&self.agent_provider, executable, &sessions, &tools) {
            Ok(plan) => plan,
            Err(e) => {
                self.fail_connection(e);
                return;
            }
        };
        match PiAgentProcess::start(&plan.executable, root, &plan.arguments, &plan.environment) {
            Ok(agent) => {
                self.process = Some(agent);
                self.connection = Connection::Ready;
                if std::mem::take(&mut self.pending_retry) {
                    if let Some((text, images)) = self.last_sent.clone() {
                        self.dispatch(&text, images, false, None);
                    }
                }
                if let Some(path) = self.pending_resume.take() {
                    self.resume(&path);
                }
                self.send_model_settings(PiRPCCommand::GetState, None);
                if let Some(process) = &self.process {
                    self.send_to(process, &PiRPCCommand::GetAvailableModels);
                    self.send_to(process, &PiRPCCommand::GetSessionStats);
                    self.send_to(process, &PiRPCCommand::GetCommands);
                }
            }
            Err(e) => {
                self.fail_connection(e.to_string());
            }
        }
    }

    /// Records a spawn failure. The empty transcript shows `connection`;
    /// once the user has typed, the same text moves to the status line.
    pub(crate) fn fail_connection(&mut self, message: String) {
        self.connection = Connection::Failed(message.clone());
        if !self.transcript.is_empty() {
            self.status_message = Some(message);
        }
    }

    /// What `spawn_with_tools` launches. Bridge providers ignore `pi_executable`
    /// and run the bundled bridge with the same Bun/Node runtime.
    fn plan_launch(
        provider: &str,
        pi_executable: &Path,
        sessions: &Path,
        tools: &PiToolchain,
    ) -> Result<AgentLaunch, String> {
        if crate::settings::agent_provider_is_on_device(provider) {
            return Err(crate::settings::ON_DEVICE_UNAVAILABLE_MESSAGE.to_string());
        }
        if crate::settings::agent_provider_uses_bridge(provider) {
            let script = bridge_script().ok_or_else(|| bridge_missing_message(provider))?;
            let (executable, arguments) = tools.launch(&script, &[])?;
            let mut environment = Self::child_environment(&executable, &tools.environment);
            environment.insert(
                "PITEX_SESSION_DIR".into(),
                sessions.to_string_lossy().into_owned(),
            );
            environment.insert(
                "PITEX_DEVELOPER_INSTRUCTIONS".into(),
                Self::system_prompt_supplement().to_string(),
            );
            environment.insert("PITEX_CLI_ADAPTER".into(), provider.to_string());
            return Ok(AgentLaunch {
                executable,
                arguments,
                environment,
            });
        }
        let arguments = vec![
            "--mode".into(),
            "rpc".into(),
            "--session-dir".into(),
            sessions.to_string_lossy().into_owned(),
            "--append-system-prompt".into(),
            Self::system_prompt_supplement().to_string(),
        ];
        let (executable, arguments) = tools.launch(pi_executable, &arguments)?;
        let environment = Self::child_environment(&executable, &tools.environment);
        Ok(AgentLaunch {
            executable,
            arguments,
            environment,
        })
    }

    /// Called by the UI when the process's `exited` channel fires.
    pub fn process_did_exit(&mut self) {
        self.abandon_drain();
        // Retry only resends a message the exit cut off (a turn running or not yet started).
        let unanswered = self.is_running || self.transcript.last().is_some_and(|e| e.role == TranscriptRole::User);
        self.process = None;
        self.is_running = false;
        self.toolchain_rx = None;
        self.pending_root = None;
        self.model_settings_request_id = None;
        self.thinking_levels.clear();
        self.is_updating_model_settings = false;
        self.clear_approvals();
        self.mode = None;
        // The list belonged to that process's session; the next one starts empty.
        self.todos.clear();
        // A fork or rewind the process took with it never answers.
        self.fork_request = None;
        self.rewind_offer = None;
        // A message cut off by the exit gets its final (non-streaming) render.
        self.finalize_entries();
        if !self.intentional_stop {
            self.connection = Connection::Failed("The agent process exited unexpectedly.".into());
            if !self.transcript.is_empty() {
                self.push_turn_error(crate::turn_state::ErrorCode::Crashed, "The agent process exited unexpectedly.".into());
                if !unanswered {
                    self.clear_retry_offers();
                }
            }
        }
        self.ui_revision += 1;
    }

    pub fn shutdown(&mut self) {
        self.abandon_drain();
        self.intentional_stop = true;
        self.is_running = false;
        self.stop_catalog_probes();
        if let Some(process) = self.process.take() {
            process.terminate();
        }
        self.toolchain_rx = None;
        self.pending_root = None;
        self.model_settings_request_id = None;
        self.thinking_levels.clear();
        self.is_updating_model_settings = false;
        self.clear_approvals();
        self.mode = None;
        self.todos.clear();
        self.fork_request = None;
        self.rewind_offer = None;
        self.finalize_entries();
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
        self.usage = Default::default();
        // Another backend may name no model; the picked one titles its answers.
        self.live_model_name = None;
        self.commands.clear();
        if self.wants_connection {
            self.prepare();
        }
        self.ui_revision += 1;
    }

    pub(crate) fn catalog_generation(&self) -> u64 {
        self.catalog_generation
    }

    /// One "<backend>: <reason>" line per enabled backend whose catalog failed.
    pub(crate) fn catalog_notes(&self, lang: &str) -> Vec<String> {
        self.catalog_slots
            .iter()
            .filter(|slot| !matches!(slot.state, crate::catalog::SlotState::Ready(_)))
            .filter_map(|slot| {
                let reason = self.catalog_reasons.get(&slot.backend)?.clone();
                let reason =
                    reason.unwrap_or_else(|| crate::l10n::tr(lang, "assistant.backend_unavailable"));
                Some(format!("{}: {reason}", self.backend_title(lang, &slot.backend)))
            })
            .collect()
    }

    pub(crate) fn catalog_menu(&self, lang: &str) -> crate::catalog::CatalogMenu {
        crate::catalog::menu(&self.catalog_facts(), |backend| {
            self.backend_title(lang, backend)
        })
    }

    /// The model menu comes through here.
    /// A cross-backend choice shuts the turn down before the next session starts.
    pub(crate) fn apply_catalog(
        &mut self,
        intent: &crate::catalog::SelectionIntent,
        shown_generation: u64,
        store: &mut crate::settings::SettingsStore,
        notice: &str,
    ) {
        let effect = crate::catalog::effect(intent, &self.catalog_facts(), shown_generation);
        let before = crate::catalog::SessionSnapshot {
            is_running: self.is_running,
            backend: self.agent_provider.clone(),
            current: self.current_model.as_ref().map(Self::to_catalog),
        };
        if crate::catalog::session_after(&effect, &before) == before {
            return;
        }
        match effect {
            crate::catalog::PickEffect::None => {}
            crate::catalog::PickEffect::SetModel(model) => {
                if let Some(ticket) = self.switch_ticket.as_mut() {
                    if ticket.backend == self.agent_provider {
                        ticket.model = Some(model.clone());
                        store.set_ai_default_model(&model.id);
                        self.preferred_model_id = None;
                        self.ui_revision += 1;
                        return;
                    }
                }
                store.set_ai_default_model(&model.id);
                self.preferred_model_id = Some(model.id.clone());
                self.select_model(&Self::from_catalog(model));
            }
            crate::catalog::PickEffect::Switch { backend, model } => {
                self.begin_switch(backend, model, store, notice);
            }
        }
    }

    fn begin_switch(
        &mut self,
        backend: String,
        model: Option<crate::catalog::CatalogModel>,
        store: &mut crate::settings::SettingsStore,
        notice: &str,
    ) {
        // A row picked for the previous backend must not open in this one.
        self.pending_resume = None;
        self.switch_ticket = Some(SwitchTicket {
            backend: backend.clone(),
            model: model.clone(),
        });
        self.preferred_model_id = None;
        match &model {
            Some(model) => store.set_ai_default_model(&model.id),
            None => store.set_ai_default_model(""),
        }
        self.models.clear();
        self.current_model = None;
        self.catalog_generation = self.catalog_generation.wrapping_add(1);
        self.set_agent_provider(&backend);
        store.set_agent_provider(&backend);
        if !self.transcript.is_empty() && !notice.is_empty() {
            self.push_entry(AgentTranscriptEntry {
                role: TranscriptRole::Notice,
                title: "Assistant".into(),
                text: notice.to_string(),
                detail: String::new(),
                status: TranscriptStatus::Done,
                tool: None,
                images: Vec::new(),
                meta: EntryMeta::None,
            });
        }
        self.restart();
    }

    pub(crate) fn reconcile_enablement(
        &mut self,
        store: &mut crate::settings::SettingsStore,
        notice: &str,
    ) {
        let was_idle = self.launch_idle;
        let offered = crate::catalog::gtk_offered();
        let stored = store.disabled_backend_ids();
        let plan = crate::catalog::enablement_plan(&crate::catalog::EnablementFacts {
            offered: &offered,
            stored_disabled: stored.as_deref(),
            normalized_provider: &self.agent_provider,
            normalized_is_on_device: crate::settings::agent_provider_is_on_device(&self.agent_provider),
            session_open: self.session_is_open(),
            previous_slots: &self.catalog_slots,
            seeded: &[],
        });
        self.launch_idle = matches!(plan.target, crate::catalog::SessionTarget::Idle);
        self.catalog_slots = plan.slots;
        self.catalog_generation = self.catalog_generation.wrapping_add(1);
        self.stop_probes_outside(&plan.enabled);
        match plan.session {
            crate::catalog::SessionPlan::Keep => {
                let stored_is_enabled = matches!(
                    &plan.target,
                    crate::catalog::SessionTarget::Active(id) if id == &self.agent_provider
                );
                if was_idle && stored_is_enabled && self.wants_connection {
                    self.prepare();
                } else {
                    self.refresh_probes();
                    self.ui_revision += 1;
                }
            }
            crate::catalog::SessionPlan::Switch(backend) => {
                self.begin_switch(backend, None, store, notice);
            }
            crate::catalog::SessionPlan::Stop => self.restart(),
        }
    }

    fn session_is_open(&self) -> bool {
        self.is_running
            || self.process.is_some()
            || self.toolchain_rx.is_some()
            || matches!(
                self.connection,
                Connection::Connecting | Connection::Ready
            )
    }

    fn stop_probes_outside(&mut self, enabled: &[String]) {
        let mut kept = Vec::new();
        for probe in self.probes.drain(..) {
            if enabled.iter().any(|id| id == &probe.backend) {
                kept.push(probe);
            } else {
                probe.process.terminate();
                let _ = std::fs::remove_dir_all(&probe.sessions);
            }
        }
        self.probes = kept;
    }

    fn catalog_facts(&self) -> crate::catalog::CatalogFacts<'_> {
        crate::catalog::CatalogFacts {
            slots: &self.catalog_slots,
            active_backend: &self.agent_provider,
            current: self.current_model.as_ref().map(Self::to_catalog),
            generation: self.catalog_generation,
            accepts_picks: !self.is_updating_model_settings,
        }
    }

    fn backend_title(&self, lang: &str, backend: &str) -> String {
        crate::settings::AGENT_PROVIDER_CHOICES
            .iter()
            .find(|choice| choice.id == backend)
            .map(|choice| crate::l10n::tr(lang, choice.label_key))
            .unwrap_or_else(|| backend.to_string())
    }

    /// Context used of the window: pushed `usage_update`, else pi's `get_session_stats`.
    pub fn context_meter(&self) -> Option<crate::turn_state::ContextMeter> {
        let (used, max) = match (self.usage.context_used, self.usage.context_max) {
            (Some(used), Some(max)) => (used, max),
            _ => {
                let stats = self.session_stats.as_ref()?;
                (stats.context_tokens?, stats.context_window?)
            }
        };
        crate::turn_state::ContextMeter::new(used, max)
    }

    fn note_live_models(&mut self) {
        let models: Vec<_> = self.models.iter().map(Self::to_catalog).collect();
        let state = if models.is_empty() {
            crate::catalog::SlotState::Unavailable
        } else {
            crate::catalog::SlotState::Ready(models)
        };
        if matches!(state, crate::catalog::SlotState::Ready(_)) {
            self.catalog_reasons.remove(&self.agent_provider);
        }
        crate::catalog::note(&mut self.catalog_slots, &self.agent_provider, state);
        self.catalog_generation = self.catalog_generation.wrapping_add(1);
    }

    fn retry_catalog_commit(&mut self) {
        if self.catalog_commit_pending {
            self.commit_switch_ticket();
        }
    }

    fn commit_switch_ticket(&mut self) {
        let Some(ticket) = self.switch_ticket.clone() else {
            self.catalog_commit_pending = false;
            return;
        };
        if ticket.backend != self.agent_provider {
            return;
        }
        let Some(wanted) = ticket.model else {
            self.catalog_commit_pending = false;
            self.switch_ticket = None;
            self.send_model_settings(PiRPCCommand::GetState, None);
            return;
        };
        if self.is_updating_model_settings {
            self.catalog_commit_pending = true;
            return;
        }
        self.catalog_commit_pending = false;
        if let Some(found) = self
            .models
            .iter()
            .find(|model| model.id == wanted.id && model.provider == wanted.provider)
            .cloned()
        {
            self.switch_ticket = None;
            self.select_model(&found);
        } else {
            self.switch_ticket = None;
            self.current_model = None;
            self.send_model_settings(PiRPCCommand::GetState, None);
        }
    }

    fn note_catalog_report(
        &mut self,
        backend: &str,
        report: Result<Vec<crate::catalog::CatalogModel>, Option<String>>,
    ) {
        let state = match report {
            Ok(models) if !models.is_empty() => {
                self.catalog_reasons.remove(backend);
                crate::catalog::SlotState::Ready(models)
            }
            Ok(_) => crate::catalog::SlotState::Unavailable,
            Err(reason) => {
                self.catalog_reasons.insert(backend.to_string(), reason);
                crate::catalog::SlotState::Unavailable
            }
        };
        crate::catalog::note(&mut self.catalog_slots, backend, state);
        self.catalog_generation = self.catalog_generation.wrapping_add(1);
        self.ui_revision += 1;
    }

    fn refresh_probes(&mut self) {
        if self.catalog_tools.is_none() {
            return;
        }
        let active = self.agent_provider.clone();
        let pending: Vec<String> = self
            .catalog_slots
            .iter()
            .filter(|slot| {
                slot.backend != active
                    && !self.probe_running(&slot.backend)
                    && matches!(slot.state, crate::catalog::SlotState::Pending)
            })
            .map(|slot| slot.backend.clone())
            .collect();
        for backend in pending {
            self.start_probe(&backend);
        }
    }

    fn probe_running(&self, backend: &str) -> bool {
        self.probes
            .iter()
            .any(|probe| probe.backend == backend && probe.generation == self.probe_generation)
    }

    fn start_probe(&mut self, backend: &str) {
        let Some(tools) = self.catalog_tools.clone() else {
            return;
        };
        let root = self.catalog_root.clone().unwrap_or_else(std::env::temp_dir);
        let sessions = std::env::temp_dir().join(format!("pitex-catalog-{backend}-{}", uuid_v4()));
        // A throwaway directory keeps the probe out of the project history.
        // The CLI still uses its own login. This path is not a credential file.
        if std::fs::create_dir_all(&sessions).is_err() {
            self.note_catalog_report(backend, Err(None));
            return;
        }
        let pi = self.catalog_pi.clone().unwrap_or_default();
        let mut plan = match Self::plan_launch(backend, &pi, &sessions, &tools) {
            Ok(plan) => plan,
            Err(_) => {
                let _ = std::fs::remove_dir_all(&sessions);
                self.note_catalog_report(backend, Err(None));
                return;
            }
        };
        // The bridge reads this and does not write a Claude session for the project.
        plan.environment
            .insert("PITEX_CATALOG_PROBE".into(), "1".into());
        match PiAgentProcess::start(&plan.executable, &root, &plan.arguments, &plan.environment) {
            Ok(process) => {
                let _ = process.send(&PiRPCCommand::GetAvailableModels, None);
                self.probes.push(CatalogProbe {
                    backend: backend.to_string(),
                    generation: self.probe_generation,
                    process,
                    started: std::time::Instant::now(),
                    sessions,
                    error: None,
                });
            }
            Err(_) => {
                let _ = std::fs::remove_dir_all(&sessions);
                self.note_catalog_report(backend, Err(None));
            }
        }
    }

    fn stop_catalog_probes(&mut self) {
        self.probe_generation = self.probe_generation.wrapping_add(1);
        for probe in self.probes.drain(..) {
            probe.process.terminate();
            let _ = std::fs::remove_dir_all(&probe.sessions);
        }
    }

    pub fn poll_catalog_probes(&mut self) {
        let generation = self.probe_generation;
        let mut finished = Vec::new();
        for (index, probe) in self.probes.iter_mut().enumerate() {
            if probe.generation != generation {
                finished.push((index, None));
                continue;
            }
            let mut outcome = None;
            {
                let events = probe.process.events.lock().unwrap();
                while let Ok(event) = events.try_recv() {
                    if event.event_type == "extension_error" && probe.error.is_none() {
                        probe.error = event.string("error");
                    }
                    if let Some(report) = catalog_models_from_event(&event) {
                        // An empty but successful reply was not refused: a notice
                        // printed before it (agy's permission warning) is not the reason.
                        let earlier = probe.error.clone().filter(|_| !event.response_succeeded());
                        outcome = Some(report.map_err(|reason| reason.or(earlier)));
                        break;
                    }
                }
            }
            // A process can exit before its stdout thread delivers the line.
            // Wait briefly so a fast catalog reply is not recorded as a failure.
            if outcome.is_none()
                && (probe.started.elapsed() > CATALOG_PROBE_LIMIT
                    || (!probe.process.is_running()
                        && probe.started.elapsed() > std::time::Duration::from_millis(200)))
            {
                outcome = Some(Err(probe.error.clone()));
            }
            if outcome.is_some() {
                finished.push((index, outcome));
            }
        }
        for (index, report) in finished.into_iter().rev() {
            let probe = self.probes.remove(index);
            probe.process.terminate();
            let _ = std::fs::remove_dir_all(&probe.sessions);
            if probe.generation != self.probe_generation {
                continue;
            }
            if let Some(report) = report {
                self.note_catalog_report(&probe.backend, report);
            }
        }
    }

    fn to_catalog(model: &PiModelDescriptor) -> crate::catalog::CatalogModel {
        crate::catalog::CatalogModel {
            id: model.id.clone(),
            provider: model.provider.clone(),
            name: model.name.clone(),
            context_window: model.context_window,
        }
    }

    fn from_catalog(model: crate::catalog::CatalogModel) -> PiModelDescriptor {
        PiModelDescriptor {
            id: model.id,
            provider: model.provider,
            name: model.name,
            context_window: model.context_window,
        }
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

    /// Return (`now` false) or Cmd/Ctrl+Return (`now` true): send, steer the
    /// running turn, or queue (`composer::submit_route`).
    pub fn submit(&mut self, prompt: &str, images: Vec<crate::composer::AgentImage>, now: bool) {
        let trimmed = prompt.trim();
        if (trimmed.is_empty() && images.is_empty()) || self.is_branching() {
            return;
        }
        // pi's own /resume is an interactive TUI picker; here it opens the
        // panel's history list instead. /clear is a new chat, so the panel and
        // the backend's context agree. Either way the attached images stay.
        if trimmed == "/resume" || trimmed == "/clear" {
            self.draft_images = images;
            if trimmed == "/resume" {
                self.open_history();
            } else {
                self.new_session();
            }
            return;
        }
        self.queue.paused = false;
        use crate::composer::SubmitRoute;
        // A drained message not yet accepted counts as running: a second plain prompt would be refused.
        let busy = self.is_running || self.drain_request.is_some();
        match crate::composer::submit_route(busy, self.pending_approval.is_some(), now, self.capabilities.steer && self.is_running) {
            SubmitRoute::Send => {
                self.dispatch(trimmed, images, false, None);
            }
            SubmitRoute::Steer => {
                self.dispatch(trimmed, images, true, None);
            }
            SubmitRoute::Queue => {
                self.queue.enqueue(trimmed, images);
                self.ui_revision += 1;
            }
        }
    }

    /// "Send now" on a queued row is offered: steer into a running turn, or send
    /// when idle (resumes a queue paused by Stop or a refused send).
    pub fn can_send_queued_now(&self) -> bool {
        self.pending_approval.is_none()
            && if self.is_running { self.capabilities.steer } else { self.drain_request.is_none() }
    }

    pub fn send_queued_now(&mut self, id: u64) {
        if !self.can_send_queued_now() {
            return;
        }
        let Some(item) = self.queue.remove(id) else { return };
        self.queue.paused = false;
        self.dispatch(&item.text, item.images, self.is_running, None);
    }

    /// Edit pulls the row back into the composer; the caller puts the text there.
    pub fn take_queued(&mut self, id: u64) -> Option<crate::composer::QueuedMessage> {
        let item = self.queue.remove(id);
        self.ui_revision += 1;
        item
    }

    /// The process went away with a queued message in flight: it goes back to the front.
    fn abandon_drain(&mut self) {
        if let Some((_, item)) = self.drain_request.take() {
            self.queue.failed(item);
        }
    }

    pub fn clear_queue(&mut self) {
        self.queue.clear();
        self.drain_request = None;
        self.ui_revision += 1;
    }

    /// One queued message per agent_end. A refused send goes back to the front
    /// (`handle_response`) and the queue waits for the user.
    fn drain_queue(&mut self) {
        if self.pending_approval.is_some() || self.drain_request.is_some() {
            return;
        }
        let Some(item) = self.queue.take_next() else { return };
        let id = uuid_v4();
        self.drain_request = Some((id.clone(), item.id));
        if !self.dispatch(&item.text, item.images, false, Some(&id)) {
            self.drain_request = None;
            self.queue.failed(item.id);
        }
    }

    /// Persist dirty sessions first, then envelope + prompt (or steer). False when
    /// nothing went out.
    fn dispatch(&mut self, trimmed: &str, images: Vec<crate::composer::AgentImage>, steer: bool, id: Option<&str>) -> bool {
        if self.connection == Connection::PiMissing {
            self.status_message = Some(
                "Pitex Agent is not installed yet — it installs automatically on launch; see Settings → AI.".into(),
            );
            return false;
        }
        // The fork or rewind would close the query this message went to.
        if self.is_branching() {
            return false;
        }
        // A drained message gets its bubble once the backend accepts it (`handle_response`),
        // so a refused one, back at the front of the queue, does not leave a duplicate.
        if id.is_none() {
            self.push_user_entry(trimmed, images.clone());
        }
        if !steer {
            self.last_sent = Some((trimmed.to_string(), images.clone()));
            // A new message replaces a crash Retry still waiting for the relaunch.
            self.pending_retry = false;
            self.clear_retry_offers();
        }
        self.status_message = None;
        if let Some(persist) = &mut self.persist_dirty_sessions {
            if let Some(problem) = (persist)() {
                self.push_entry(AgentTranscriptEntry {
                    role: TranscriptRole::Notice,
                    title: "Assistant".into(),
                    text: problem,
                    detail: String::new(),
                    status: TranscriptStatus::Failed,
                    tool: None,
                    images: Vec::new(),
                    meta: EntryMeta::None,
                });
                return false;
            }
        }
        // Slash commands go raw — pi expands `/skill:name` and extension
        // commands only when the message itself starts with '/'.
        let message = if trimmed.starts_with('/') {
            trimmed.to_string()
        } else {
            self.envelope(trimmed)
        };
        // A drained message may meet pi still settling (retry, compaction):
        // followUp waits for it there, and the bridges ignore it when idle.
        let command = if steer {
            PiRPCCommand::Steer { message, images }
        } else {
            PiRPCCommand::Prompt {
                message,
                images,
                streaming_behavior: (self.is_running || id.is_some()).then(|| "followUp".to_string()),
            }
        };
        // Unit tests have no child process; they read what would have gone out.
        #[cfg(test)]
        if self.process.is_none() {
            self.prompt_wire.push(command.object());
            self.ui_revision += 1;
            return true;
        }
        if self.process.is_none() {
            self.prepare();
        }
        let Some(process) = &self.process else {
            if self.connection == Connection::Idle {
                self.connection = Connection::Connecting;
            }
            self.status_message = Some(self.not_running_message());
            return false;
        };
        if !process.is_running() {
            self.status_message = Some(self.not_running_message());
            return false;
        }
        let sent = match process.send(&command, id) {
            Ok(()) => true,
            Err(e) => {
                self.status_message = Some(format!("The prompt could not be sent: {e}"));
                false
            }
        };
        self.ui_revision += 1;
        sent
    }

    fn push_user_entry(&mut self, text: &str, images: Vec<crate::composer::AgentImage>) {
        self.push_entry(AgentTranscriptEntry {
            role: TranscriptRole::User,
            title: "You".into(),
            text: text.to_string(),
            detail: String::new(),
            status: TranscriptStatus::Done,
            tool: None,
            images,
            meta: EntryMeta::None,
        });
    }

    fn not_running_message(&self) -> String {
        if let Connection::Failed(message) = &self.connection {
            if !message.is_empty() {
                return message.clone();
            }
        }
        "The agent is not running.".into()
    }

    pub fn stop(&mut self) {
        self.deny_queued_approvals();
        // Stop keeps the queue but does not let agent_end send the next one.
        self.queue.paused = true;
        if !self.is_running {
            return;
        }
        if let Some(process) = &self.process {
            let _ = process.send(&PiRPCCommand::Abort, None);
        }
        self.ui_revision += 1;
    }
    pub fn new_session(&mut self) {
        if self.is_branching() {
            return;
        }
        self.deny_queued_approvals();
        if !self.process.as_ref().is_some_and(|p| p.is_running()) {
            return;
        }
        self.clear_transcript();
        if let Some(process) = &self.process {
            let _ = process.send(&PiRPCCommand::NewSession, None);
        }
        self.ui_revision += 1;
    }

    /// Answers the approval on screen with a bare decision (tests).
    #[cfg(test)]
    pub fn respond_to_approval(&mut self, decision: &str) {
        let Some(id) = self.approval_queue.front().map(|a| a.id.clone()) else { return };
        self.answer_approval(&id, ApprovalAnswer::decision(decision));
    }

    /// Answers card `id` with a button, question answers, or feedback. A card
    /// the bridge already settled is ignored, so the click cannot answer the
    /// request queued behind it. The next queued request takes its place.
    pub fn answer_approval(&mut self, id: &str, answer: ApprovalAnswer) {
        if self.approval_queue.front().map(|a| a.id.as_str()) != Some(id) {
            return;
        }
        let Some(pending) = self.approval_queue.pop_front() else {
            return;
        };
        self.pending_approval = self.approval_queue.front().cloned();
        self.send_approval(&pending.id, answer);
        self.ui_revision += 1;
    }

    /// The bridge settled a request itself (abort, turn end, process exit): drop its card.
    fn approval_resolved(&mut self, id: &str) {
        let before = self.approval_queue.len();
        self.approval_queue.retain(|item| item.id != id);
        if self.approval_queue.len() != before {
            self.pending_approval = self.approval_queue.front().cloned();
            self.ui_revision += 1;
        }
    }

    /// Picks a permission mode the bridge listed. The bridge answers with
    /// `mode_changed`; a refused pick re-reads the state.
    pub fn select_mode(&mut self, id: &str) {
        let Some(mode) = self.mode.as_mut() else { return };
        if mode.id == id || !mode.options.iter().any(|(option, _)| option == id) {
            return;
        }
        mode.id = id.to_string();
        if let Some(process) = &self.process {
            let _ = process.send(&PiRPCCommand::SetMode { mode_id: id.to_string() }, None);
        }
        self.ui_revision += 1;
    }

    /// Stop and new-session deny every outstanding request, not only the
    /// one the panel is showing.
    fn deny_queued_approvals(&mut self) {
        let queued: Vec<_> = self.approval_queue.drain(..).collect();
        if queued.is_empty() {
            return;
        }
        self.pending_approval = None;
        for pending in &queued {
            self.send_approval(&pending.id, ApprovalAnswer::decision("deny"));
        }
        self.ui_revision += 1;
    }

    fn clear_approvals(&mut self) {
        self.approval_queue.clear();
        self.pending_approval = None;
    }

    fn note_approval(&mut self, request: AgentApprovalRequest) {
        if let Some(existing) = self.approval_queue.iter_mut().find(|item| item.id == request.id) {
            *existing = request;
        } else {
            self.approval_queue.push_back(request);
        }
        self.pending_approval = self.approval_queue.front().cloned();
    }

    fn send_approval(&mut self, id: &str, answer: ApprovalAnswer) {
        let command = PiRPCCommand::ApprovalResponse { id: id.to_string(), answer };
        #[cfg(test)]
        {
            let wire = command.object();
            self.approval_sent.push((id.to_string(), wire["decision"].as_str().unwrap_or_default().to_string()));
            self.approval_wire.push(wire);
        }
        let Some(process) = &self.process else { return };
        let _ = process.send(&command, None);
    }

    /// A new todo snapshot: one activity row per change, then the checklist follows it.
    fn note_todos(&mut self, items: Vec<TodoItem>) {
        if items == self.todos {
            return;
        }
        for activity in todo_activities(&self.todos, &items) {
            // A list built one TaskCreate at a time reads as one "created" row.
            let all_pending = items.iter().all(|item| item.status == TodoStatus::Pending);
            if let (TodoActivity::Added(_), true, Some(last)) = (&activity, all_pending, self.transcript.last_mut()) {
                if last.role == TranscriptRole::Activity && last.title == "assistant.todo.created" {
                    last.text = items.len().to_string();
                    continue;
                }
            }
            let (key, arg) = activity.key_and_arg();
            self.push_entry(AgentTranscriptEntry {
                role: TranscriptRole::Activity,
                title: key.into(),
                text: arg,
                detail: String::new(),
                status: TranscriptStatus::Done,
                tool: None,
                images: Vec::new(),
                meta: EntryMeta::None,
            });
        }
        self.todos = items;
        self.ui_revision += 1;
    }

    fn clear_transcript(&mut self) {
        // A new or another chat: the queue belonged to this one.
        self.queue.clear();
        self.drain_request = None;
        self.transcript.clear();
        self.todos.clear();
        self.tool_index_by_call_id.clear();
        self.assistant_entry_index = None;
        self.thinking_entry_index = None;
        self.thinking_started = None;
        self.live_model_name = None;
        // Retry belongs to that chat.
        self.last_sent = None;
        self.pending_retry = false;
        // The meter belongs to that chat; the next usage_update or stats reply refills it.
        self.usage = Default::default();
        self.session_stats = None;
        self.rewind_offer = None;
        self.turn_started = None;
        self.dropped_prompts = 0;
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
        // Mirrors of hidden backends (direct Codex and Grok) cannot be opened from any setting.
        self.past_sessions = files
            .filter_map(|p| session_summary(&p))
            .filter(|s| crate::settings::visible_agent_providers().any(|c| c.id == s.source))
            .collect();
        self.past_sessions.sort_by(|a, b| b.modified.cmp(&a.modified));
    }

    /// Opens the history list. A bridge backend first mirrors sessions the
    /// CLI started elsewhere (`refresh_sessions`); the list opens on its reply.
    pub fn open_history(&mut self) {
        if self.is_running {
            return;
        }
        let process = self.process.as_ref().filter(|p| p.is_running());
        if let (true, Some(process)) = (crate::settings::agent_provider_uses_bridge(&self.agent_provider), process) {
            let request_id = uuid_v4();
            if process.send(&PiRPCCommand::RefreshSessions, Some(&request_id)).is_ok() {
                self.refresh_request_id = Some(request_id);
                return;
            }
        }
        self.show_history_now();
    }

    fn show_history_now(&mut self) {
        self.refresh_request_id = None;
        self.load_past_sessions();
        self.history_requested = true;
        self.ui_revision += 1;
    }

    /// A history row was picked. Another backend's conversation switches the
    /// backend first and opens once it is up; a turned-off backend says so.
    pub(crate) fn open_session(
        &mut self,
        session: &AgentSessionSummary,
        store: &mut crate::settings::SettingsStore,
        lang: &str,
    ) {
        if self.is_branching() {
            return;
        }
        if session.source == self.agent_provider {
            self.resume(&session.path);
        } else if !self.is_running && self.catalog_slots.iter().any(|slot| slot.backend == session.source) {
            self.begin_switch(session.source.to_string(), None, store, "");
            // After the switch: the new process starts later, on the toolchain poll.
            self.pending_resume = Some(session.path.clone());
        } else if !self.is_running {
            let name = session_source_label(session.source);
            self.push_entry(AgentTranscriptEntry {
                role: TranscriptRole::Notice,
                title: "Assistant".into(),
                text: crate::l10n::tr1(lang, "assistant.history_backend_off", name),
                detail: String::new(),
                status: TranscriptStatus::Done,
                tool: None,
                images: Vec::new(),
                meta: EntryMeta::None,
            });
        }
        self.ui_revision += 1;
    }

    /// Loads a saved session into pi, then redraws the transcript from its
    /// messages (`switch_session` → `get_messages`).
    pub fn resume(&mut self, path: &Path) {
        if self.is_running || self.is_branching() {
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
            tool: None,
            images: Vec::new(),
            meta: EntryMeta::None,
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
                    // The model that wrote it, when the backend saved one (Claude), not the picker's alias.
                    let title = message
                        .get("modelName")
                        .and_then(|n| n.as_str())
                        .filter(|n| !n.is_empty())
                        .map(|n| format!("Pitex Agent · {n}"))
                        .unwrap_or_else(|| title.clone());
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
                    // Bridge backends keep the typed card; the args summary goes, as live.
                    if let Some(status) = message.get("status").and_then(|s| s.as_str()).and_then(status_named) {
                        entries[index].status = status;
                    }
                    let typed = message.get("detail").and_then(|d| d.as_object()).and_then(|d| ToolDetail::decode(id, d));
                    if let Some(mut tool) = typed {
                        tool.parent_call_id = message.get("parentToolCallId").and_then(|p| p.as_str()).map(String::from);
                        if !tool.is_untyped() {
                            entries[index].detail.clear();
                        }
                        entries[index].tool = Some(tool);
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

    /// Prompt rows `transcript_from_messages` leaves out above `history_limit`.
    fn prompts_above_limit(&self, messages: &[Value]) -> usize {
        let prompts = messages
            .iter()
            .filter_map(|m| m.as_object())
            .filter(|m| m.get("role").and_then(|r| r.as_str()) == Some("user") && !user_prompt_text(m).is_empty())
            .count();
        let shown = self.transcript.iter().filter(|e| e.role == TranscriptRole::User).count();
        prompts.saturating_sub(shown)
    }
    pub fn select_model(&mut self, model: &PiModelDescriptor) {
        if self.is_updating_model_settings || self.current_model.as_ref() == Some(model) {
            return;
        }
        #[cfg(test)]
        self.model_requests.push((model.provider.clone(), model.id.clone()));
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
            "agent_start" => {
                self.is_running = true;
                self.turn_started.get_or_insert_with(std::time::Instant::now);
            }
            "agent_end" => {
                self.note_turn_duration();
                if let Some(process) = &self.process {
                    self.send_to(process, &PiRPCCommand::GetSessionStats);
                }
                self.is_running = false;
                self.finalize_entries();
                self.note_turn_end(event);
                if let Some(cb) = &mut self.on_agent_activity_finished {
                    cb();
                }
                self.drain_queue();
            }
            "message_start" => self.handle_message_start(event),
            "usage_update" => self.usage.merge(&event.object),
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
                    tool: None,
                    images: Vec::new(),
                    meta: EntryMeta::None,
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
                            tool: None,
                            images: Vec::new(),
                            meta: EntryMeta::None,
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
                tool: None,
                images: Vec::new(),
                meta: EntryMeta::None,
            }),
            "compaction_end" => {
                if event.bool("aborted") {
                    self.push_entry(AgentTranscriptEntry {
                        role: TranscriptRole::Notice,
                        title: "Assistant".into(),
                        text: "Compaction cancelled.".into(),
                        detail: String::new(),
                        status: TranscriptStatus::Done,
                        tool: None,
                        images: Vec::new(),
                        meta: EntryMeta::None,
                    });
                } else if let Some(message) = event.string("errorMessage") {
                    self.push_entry(AgentTranscriptEntry {
                        role: TranscriptRole::Notice,
                        title: "Assistant".into(),
                        text: format!("Compaction failed: {message}"),
                        detail: String::new(),
                        status: TranscriptStatus::Failed,
                        tool: None,
                        images: Vec::new(),
                        meta: EntryMeta::None,
                    });
                }
            }
            "thinking_level_changed" => {
                if !self.is_updating_model_settings {
                    self.send_model_settings(PiRPCCommand::GetState, None);
                }
            }
            "extension_ui_request" => self.handle_extension_ui(event),
            "approval_request" => self.note_approval(AgentApprovalRequest::decode(&event.object)),
            "approval_resolved" => self.approval_resolved(&event.string("id").unwrap_or_default()),
            "mode_changed" => {
                self.mode = AgentMode::decode(event.object.get("mode"));
                self.ui_revision += 1;
            }
            "todo_update" => self.note_todos(TodoItem::decode_list(event.object.get("items"))),
            "extension_error" => {
                let message = event.string("error").unwrap_or_else(|| "extension error".into());
                self.push_entry(AgentTranscriptEntry {
                    role: TranscriptRole::Notice,
                    title: "Assistant".into(),
                    text: message,
                    detail: String::new(),
                    status: TranscriptStatus::Failed,
                    tool: None,
                    images: Vec::new(),
                    meta: EntryMeta::None,
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
        match event.response_command().as_deref() {
            Some("fork_session") => return self.handle_fork_response(event),
            Some("rewind") => return self.handle_rewind_response(event),
            _ => {}
        }
        if event.response_command().as_deref() == Some("refresh_sessions") {
            // Success or not (a signed-out backend refuses), the list shows what is on disk now;
            // a turn started meanwhile wins, since rows cannot be opened during it.
            if self.refresh_request_id.is_some() && event.string("id") == self.refresh_request_id {
                self.refresh_request_id = None;
                if !self.is_running {
                    self.show_history_now();
                }
            }
            return;
        }
        if event.response_command().as_deref() == Some("prompt") {
            if let Some((id, queued)) = self.drain_request.clone() {
                if event.string("id").as_deref() == Some(id.as_str()) {
                    self.drain_request = None;
                    if event.response_succeeded() {
                        if let Some(item) = self.queue.items.iter().find(|item| item.id == queued).cloned() {
                            self.push_user_entry(&item.text, item.images);
                        }
                        self.queue.sent(queued);
                    } else {
                        self.queue.failed(queued);
                    }
                }
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
                } else {
                    self.retry_catalog_commit();
                }
            }
            let message = event
                .response_error()
                .unwrap_or_else(|| "The agent command failed.".into());
            if event.response_command().as_deref() == Some("get_available_models") {
                let backend = self.agent_provider.clone();
                self.note_catalog_report(&backend, Err(event.response_error()));
            }
            self.status_message = Some(message.clone());
            if event.response_command().as_deref() == Some("set_mode") {
                // The picker showed the refused pick; read the mode back.
                self.send_model_settings(PiRPCCommand::GetState, None);
            }
            if event.response_command().as_deref() == Some("prompt") {
                self.push_entry(AgentTranscriptEntry {
                    role: TranscriptRole::Notice,
                    title: "Assistant".into(),
                    text: message,
                    detail: String::new(),
                    status: TranscriptStatus::Failed,
                    tool: None,
                    images: Vec::new(),
                    meta: EntryMeta::None,
                });
            }
            return;
        }
        match event.response_command().as_deref() {
            Some("get_state") => {
                if let Some(data) = event.response_data() {
                    if self.switch_ticket.is_none() {
                        self.current_model =
                            data.get("model").and_then(PiModelDescriptor::from_value);
                    }
                    if let Some(level) = data.get("thinkingLevel").and_then(|v| v.as_str()) {
                        self.thinking_level = level.to_string();
                    }
                    self.mode = AgentMode::decode(data.get("mode"));
                    self.capabilities = AgentCapabilities::from_state(data);
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
                    self.retry_catalog_commit();
                    return;
                }
                self.thinking_levels = if self.current_model.is_none() {
                    Vec::new()
                } else {
                    levels
                };
                self.apply_preferred_model_if_ready();
                self.retry_catalog_commit();
            }
            Some("get_available_models") => {
                let raw = event
                    .response_data()
                    .and_then(|d| d.get("models"))
                    .and_then(|m| m.as_array())
                    .cloned()
                    .unwrap_or_default();
                self.models = raw.iter().filter_map(PiModelDescriptor::from_value).collect();
                self.note_live_models();
                self.apply_preferred_model_if_ready();
                self.commit_switch_ticket();
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
                    argument_hint: None,
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
                if !cancelled {
                    self.reload_messages();
                } else {
                    self.send_model_settings(PiRPCCommand::GetState, None);
                }
            }
            Some("get_messages") => {
                let Some(latest) = self.messages_request_id.as_deref() else {
                    return;
                };
                if event.string("id").as_deref() != Some(latest) {
                    return;
                }
                let messages = event
                    .response_data()
                    .and_then(|d| d.get("messages"))
                    .and_then(|m| m.as_array())
                    .cloned()
                    .unwrap_or_default();
                self.transcript = self.transcript_from_messages(&messages);
                self.dropped_prompts = self.prompts_above_limit(&messages);
                if let Some(notice) = self.notice_after_reload.take() {
                    self.push_notice(notice, TranscriptStatus::Done);
                }
                // The last list of a resumed chat; no activity rows for it.
                self.todos = TodoItem::decode_list(event.response_data().and_then(|d| d.get("todos")));
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
        if let Some(name) = message.get("modelName").and_then(|n| n.as_str()).filter(|n| !n.is_empty()) {
            self.live_model_name = Some(name.to_string());
        }
    }

    fn handle_message_update(&mut self, event: &PiRPCEvent) {
        let Some(delta) = event.nested("assistantMessageEvent") else { return };
        let Some(delta_type) = delta.get("type").and_then(|t| t.as_str()) else { return };
        match delta_type {
            "text_start" => {
                self.settle_thinking();
                self.text_content_index = delta.get("contentIndex").and_then(|v| v.as_i64());
                self.append_assistant_entry();
            }
            "text_delta" => {
                self.settle_thinking();
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
                        tool: None,
                        images: Vec::new(),
                        meta: EntryMeta::None,
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
                    // The streamed row already holds its own segment. message_end can carry a
                    // whole turn (text, tools, more text), which would repeat earlier rows here.
                    if self.transcript[index].text.is_empty() {
                        self.transcript[index].text = text;
                    }
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
                    tool: None,
                    images: Vec::new(),
                    meta: EntryMeta::None,
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
        self.settle_thinking();
        let Some(call_id) = event.string("toolCallId") else { return };
        let name = event.string("toolName").unwrap_or_else(|| "tool".into());
        let summary = tool_summary(&name, event.nested("args"));
        // pi's own provider sends no detail: an untyped card that still
        // carries the call id, so its open state survives row rebuilds.
        let tool = event
            .nested("detail")
            .and_then(|detail| ToolDetail::decode(&call_id, detail))
            .unwrap_or_else(|| ToolDetail { call_id: call_id.clone(), kind: "other".into(), ..Default::default() });
        let tool = ToolDetail { parent_call_id: event.string("parentToolCallId"), ..tool };
        // A typed card shows its fields in the body; untyped (pi, MCP) and
        // sub-agent cards have no such fields, so they keep the args summary.
        let detail = if tool.is_untyped() { summary } else { String::new() };
        self.push_entry(AgentTranscriptEntry {
            role: TranscriptRole::Tool,
            title: name,
            text: String::new(),
            detail,
            status: tool_status(event).unwrap_or(TranscriptStatus::Running),
            tool: Some(tool),
            images: Vec::new(),
            meta: EntryMeta::None,
        });
        self.tool_index_by_call_id
            .insert(call_id, self.transcript.len() - 1);
    }

    /// Merges a later event's `detail` into the card for its call.
    fn merge_tool_detail(&mut self, index: usize, call_id: &str, event: &PiRPCEvent) {
        let Some(incoming) = event
            .nested("detail")
            .and_then(|detail| ToolDetail::decode(call_id, detail))
        else {
            return;
        };
        match &mut self.transcript[index].tool {
            Some(tool) => tool.merge(incoming),
            slot @ None => *slot = Some(incoming),
        }
    }

    fn handle_tool_update(&mut self, event: &PiRPCEvent) {
        let Some(call_id) = event.string("toolCallId") else { return };
        let Some(&index) = self.tool_index_by_call_id.get(&call_id) else { return };
        if index >= self.transcript.len() {
            return;
        }
        self.merge_tool_detail(index, &call_id, event);
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
        self.transcript[index].status = tool_status(event).unwrap_or(if event.bool("isError") {
            TranscriptStatus::Failed
        } else {
            TranscriptStatus::Done
        });
        self.merge_tool_detail(index, &call_id, event);
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
                    tool: None,
                    images: Vec::new(),
                    meta: EntryMeta::None,
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
                        tool: None,
                        images: Vec::new(),
                        meta: EntryMeta::None,
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
        // The previous segment of this message is complete once a new one starts.
        if let Some(entry) = self.assistant_entry_index.and_then(|i| self.transcript.get_mut(i)) {
            if entry.status == TranscriptStatus::Streaming {
                entry.status = TranscriptStatus::Done;
            }
        }
        let title = self.assistant_title();
        self.push_entry(AgentTranscriptEntry {
            role: TranscriptRole::Assistant,
            title,
            text: String::new(),
            detail: String::new(),
            status: TranscriptStatus::Streaming,
            tool: None,
            images: Vec::new(),
            meta: EntryMeta::None,
        });
        self.assistant_entry_index = Some(self.transcript.len() - 1);
    }
    fn append_thinking_entry(&mut self) {
        self.settle_thinking();
        self.push_entry(AgentTranscriptEntry {
            role: TranscriptRole::Thinking,
            title: "Thinking".into(),
            text: String::new(),
            detail: String::new(),
            status: TranscriptStatus::Streaming,
            tool: None,
            images: Vec::new(),
            meta: EntryMeta::Thought { seconds: None },
        });
        self.thinking_entry_index = Some(self.transcript.len() - 1);
        self.thinking_started = Some(std::time::Instant::now());
    }
    /// Paseo: a thought is loading until the next event that is not reasoning; then it
    /// closes as "Thought for Ns".
    fn settle_thinking(&mut self) {
        let Some(started) = self.thinking_started.take() else { return };
        let Some(entry) = self.thinking_entry_index.and_then(|i| self.transcript.get_mut(i)) else { return };
        entry.status = TranscriptStatus::Done;
        entry.meta = EntryMeta::Thought { seconds: Some(started.elapsed().as_secs()) };
    }
    fn finalize_entries(&mut self) {
        self.settle_thinking();
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
    /// The model that answered when the backend says (Claude's "Default" alias hides it),
    /// else the picked one.
    fn assistant_title(&self) -> String {
        self.live_model_name
            .clone()
            .or_else(|| self.current_model.as_ref().map(|m| m.name.clone()))
            .map(|name| format!("Pitex Agent · {name}"))
            .unwrap_or_else(|| "Pitex Agent".into())
    }

    /// `agent_end`: a failed turn gets an error row (friendly line, raw detail, Retry);
    /// a stopped one a plain notice. pi sends no `outcome`; its errors read as `other`.
    fn note_turn_end(&mut self, event: &PiRPCEvent) {
        let outcome = event.string("outcome");
        let failure = event.string("error").or_else(|| self.last_assistant_error(event));
        match (outcome.as_deref(), failure) {
            (Some("canceled"), Some(text)) => self.push_entry(AgentTranscriptEntry {
                role: TranscriptRole::Notice,
                title: "Assistant".into(),
                text,
                detail: String::new(),
                status: TranscriptStatus::Done,
                tool: None,
                images: Vec::new(),
                meta: EntryMeta::None,
            }),
            // Mac (`AgentTurnError.failure`) agrees: a completed turn shows no error row.
            (Some("completed"), _) | (Some("canceled"), None) | (None, None) => {}
            (_, failure) => {
                let code = crate::turn_state::ErrorCode::decode(event.string("code").as_deref());
                self.push_turn_error(code, failure.unwrap_or_else(|| "The agent run failed.".into()));
            }
        }
    }

    fn push_turn_error(&mut self, code: crate::turn_state::ErrorCode, raw: String) {
        self.clear_retry_offers();
        let can_compact = self.commands.iter().any(|c| c.name == "compact");
        self.push_entry(AgentTranscriptEntry {
            role: TranscriptRole::Notice,
            title: "Assistant".into(),
            text: raw,
            detail: String::new(),
            status: TranscriptStatus::Failed,
            tool: None,
            images: Vec::new(),
            meta: EntryMeta::TurnError { code, backend: self.agent_provider.clone(), can_compact, retry: self.last_sent.is_some() },
        });
    }

    /// Only the newest failure offers Retry, and none once another message went out.
    fn clear_retry_offers(&mut self) {
        for entry in &mut self.transcript {
            if let EntryMeta::TurnError { retry, .. } = &mut entry.meta {
                *retry = false;
            }
        }
    }

    /// Resends the last message (with its images) through the normal send path. After a
    /// crash the process starts again first and the message follows once it is up.
    pub fn retry(&mut self) {
        let Some((text, images)) = self.last_sent.clone() else { return };
        if self.is_running {
            return;
        }
        if !matches!(self.connection, Connection::Failed(_)) {
            self.dispatch(&text, images, false, None);
            return;
        }
        // shortcut: the restarted process opens a new session, so the retried message
        // goes without the earlier context; resuming the crashed session would keep it.
        self.clear_retry_offers();
        self.pending_retry = true;
        self.restart();
    }

    /// `agent_end`: the turn's prompt row learns how long the turn ran.
    fn note_turn_duration(&mut self) {
        let Some(started) = self.turn_started.take() else { return };
        if let Some(prompt) = self.transcript.iter_mut().rev().find(|e| e.role == TranscriptRole::User) {
            prompt.meta = EntryMeta::Worked { seconds: started.elapsed().as_secs() };
        }
    }

    /// A fork or a confirmed rewind is out: sending, switching chats and branching again wait.
    pub fn is_branching(&self) -> bool {
        self.fork_request.is_some() || self.rewind_offer.as_ref().is_some_and(RewindOffer::is_rewinding)
    }

    /// Prompts in this chat, counting rows trimmed off the top.
    pub fn prompt_count(&self) -> usize {
        self.dropped_prompts + self.transcript.iter().filter(|e| e.role == TranscriptRole::User).count()
    }

    /// Whole turns a Fork or Rewind on row `index` keeps: those before a prompt (whose
    /// text goes back into the composer), or those through an answer's turn.
    pub fn turn_cut(&self, index: usize) -> Option<(usize, Option<String>)> {
        let entry = self.transcript.get(index)?;
        let before = self.dropped_prompts + self.transcript[..index].iter().filter(|e| e.role == TranscriptRole::User).count();
        match entry.role {
            TranscriptRole::User => Some((before, Some(entry.text.clone()))),
            TranscriptRole::Assistant if before > 0 => Some((before, None)),
            _ => None,
        }
    }

    /// "Fork from here": the bridge copies the first turns into a new chat when it can;
    /// otherwise a new chat starts with those turns quoted into the draft.
    pub fn fork_from(&mut self, index: usize, lang: &str) {
        if self.is_running || self.is_branching() {
            return;
        }
        let Some((keep, draft)) = self.turn_cut(index) else { return };
        let Some(process) = self.process.as_ref().filter(|p| p.is_running()) else { return };
        if keep > 0 && self.capabilities.fork {
            let id = uuid_v4();
            if process.send(&PiRPCCommand::ForkSession { keep_turns: keep, turns: self.prompt_count() }, Some(&id)).is_ok() {
                self.fork_request = Some((id, draft));
            }
            self.ui_revision += 1;
            return;
        }
        let quoted = quote_turns(&self.transcript, keep.saturating_sub(self.dropped_prompts), lang);
        self.new_session();
        let text = [quoted, draft.unwrap_or_default()].into_iter().filter(|t| !t.is_empty()).collect::<Vec<_>>().join("\n\n");
        self.composer_replacement = Some(text);
        if keep > 0 {
            let name = crate::turn_state::cli_name(&self.agent_provider);
            self.push_notice(crate::l10n::tr1(lang, "assistant.fork.quoted", name), TranscriptStatus::Done);
        }
        self.ui_revision += 1;
    }

    /// "Rewind to here": opens the confirmation, after a dry run lists the files a file
    /// rewind would restore.
    pub fn request_rewind(&mut self, index: usize, lang: &str) {
        if self.is_running || self.is_branching() || !self.capabilities.rewind_conversation {
            return;
        }
        let Some((keep, draft)) = self.turn_cut(index) else { return };
        if keep >= self.prompt_count() {
            return;
        }
        let mut offer = RewindOffer { keep_turns: keep, draft, ..Default::default() };
        if self.capabilities.rewind_files {
            let id = uuid_v4();
            let command = PiRPCCommand::Rewind { keep_turns: keep, turns: self.prompt_count(), conversation: false, files: true, dry_run: true };
            if self.process.as_ref().is_some_and(|p| p.send(&command, Some(&id)).is_ok()) {
                offer.request_id = Some(id);
            }
        }
        if offer.request_id.is_none() {
            offer.files = Some(Vec::new());
            offer.files_note = Some(crate::l10n::tr(lang, "assistant.rewind.files_unsupported"));
        }
        self.rewind_offer = Some(offer);
        self.ui_revision += 1;
    }

    /// The confirmation's button: the conversation, and the files when `files`.
    pub fn confirm_rewind(&mut self, files: bool) {
        if self.is_running {
            return;
        }
        // One request at a time: a reply to the dry run must not be read as the rewind's.
        let turns = self.prompt_count();
        let Some(offer) = self.rewind_offer.as_mut().filter(|o| o.request_id.is_none()) else { return };
        let id = uuid_v4();
        let command = PiRPCCommand::Rewind { keep_turns: offer.keep_turns, turns, conversation: true, files, dry_run: false };
        if self.process.as_ref().is_some_and(|p| p.send(&command, Some(&id)).is_ok()) {
            offer.request_id = Some(id);
        }
        self.ui_revision += 1;
    }

    /// The confirmed rewind cannot be called back: its reply reloads the chat and the files.
    pub fn cancel_rewind(&mut self) {
        if self.rewind_offer.as_ref().is_some_and(RewindOffer::is_rewinding) {
            return;
        }
        self.rewind_offer = None;
        self.ui_revision += 1;
    }

    fn handle_fork_response(&mut self, event: &PiRPCEvent) {
        let Some((id, draft)) = self.fork_request.clone() else { return };
        if event.string("id").as_deref() != Some(id.as_str()) {
            return;
        }
        self.fork_request = None;
        let lang = crate::app_ui::language();
        if !event.response_succeeded() {
            let error = event.response_error().unwrap_or_default();
            self.push_notice(crate::l10n::tr1(lang, "assistant.fork.failed", &error), TranscriptStatus::Failed);
            return;
        }
        self.clear_transcript();
        self.reload_messages();
        self.notice_after_reload = Some(crate::l10n::tr(lang, "assistant.fork.done"));
        self.composer_replacement = draft;
    }

    fn handle_rewind_response(&mut self, event: &PiRPCEvent) {
        let Some(offer) = self.rewind_offer.as_mut() else { return };
        if offer.request_id.is_none() || event.string("id") != offer.request_id {
            return;
        }
        offer.request_id = None;
        let lang = crate::app_ui::language();
        let data = event.response_data();
        let files: Vec<String> = data
            .and_then(|d| d.get("filesChanged"))
            .and_then(|f| f.as_array())
            .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();
        if offer.files.is_none() {
            // The dry run: the list, or why files cannot be restored.
            if event.response_succeeded() {
                offer.files = Some(files);
            } else {
                offer.files = Some(Vec::new());
                offer.files_note = Some(crate::l10n::tr1(lang, "assistant.rewind.files_note", &event.response_error().unwrap_or_default()));
            }
            self.ui_revision += 1;
            return;
        }
        if !event.response_succeeded() {
            let error = event.response_error().unwrap_or_default();
            self.rewind_offer = None;
            self.push_notice(crate::l10n::tr1(lang, "assistant.rewind.failed", &error), TranscriptStatus::Failed);
            return;
        }
        let draft = offer.draft.clone();
        self.clear_transcript();
        self.reload_messages();
        let root = self.context_provider.as_ref().and_then(|p| (p)().project_root);
        let shown: Vec<String> = files
            .iter()
            .map(|f| root.as_deref().and_then(|r| Path::new(f).strip_prefix(r).ok()).map(|p| p.display().to_string()).unwrap_or_else(|| f.clone()))
            .collect();
        self.notice_after_reload = Some(if shown.is_empty() {
            crate::l10n::tr(lang, "assistant.rewind.done")
        } else {
            crate::l10n::tr1(lang, "assistant.rewind.done_files", &shown.join(", "))
        });
        self.composer_replacement = draft;
        if let Some(cb) = &mut self.on_agent_activity_finished {
            // Restored files: the workspace reloads them like an agent edit.
            cb();
        }
    }

    /// Redraws the chat from the backend (after a switch, fork or rewind).
    fn reload_messages(&mut self) {
        if let Some(process) = &self.process {
            let request_id = uuid_v4();
            self.messages_request_id = Some(request_id.clone());
            let _ = process.send(&PiRPCCommand::GetMessages, Some(&request_id));
        }
        self.send_model_settings(PiRPCCommand::GetState, None);
        self.ui_revision += 1;
    }

    fn push_notice(&mut self, text: String, status: TranscriptStatus) {
        self.push_entry(AgentTranscriptEntry {
            role: TranscriptRole::Notice,
            title: "Assistant".into(),
            text,
            detail: String::new(),
            status,
            tool: None,
            images: Vec::new(),
            meta: EntryMeta::None,
        });
        self.ui_revision += 1;
    }

    /// History-limit trimming — verbatim didSet semantics (indices reset).
    fn push_entry(&mut self, entry: AgentTranscriptEntry) {
        self.transcript.push(entry);
        if self.transcript.len() > self.history_limit {
            let drop = self.transcript.len() - self.history_limit;
            self.dropped_prompts += self.transcript[..drop].iter().filter(|e| e.role == TranscriptRole::User).count();
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
        _ => {
            if args.len() == 1 {
                if let Some(summary) = args.get("summary").and_then(|value| value.as_str()) {
                    return bounded(summary, 500);
                }
            }
            serde_json::to_string(args)
                .map(|s| bounded(&s, 500))
                .unwrap_or_default()
        }
    }
}

/// The first `keep` turns of `entries` (relative to the rows on screen) as a quote for a new chat.
pub fn quote_turns(entries: &[AgentTranscriptEntry], keep: usize, lang: &str) -> String {
    let mut prompts = 0;
    let mut lines = Vec::new();
    for entry in entries {
        if entry.role == TranscriptRole::User {
            if prompts == keep {
                break;
            }
            prompts += 1;
        }
        let who = match entry.role {
            TranscriptRole::User => crate::l10n::tr(lang, "assistant.fork.quote_you"),
            TranscriptRole::Assistant => crate::l10n::tr(lang, "assistant.fork.quote_assistant"),
            _ => continue,
        };
        if entry.text.trim().is_empty() {
            continue;
        }
        let body = entry.text.trim().lines().map(|line| format!("> {line}")).collect::<Vec<_>>().join("\n");
        lines.push(format!("> **{who}:**\n{body}"));
    }
    if lines.is_empty() {
        return String::new();
    }
    format!("{}\n\n{}", crate::l10n::tr(lang, "assistant.fork.quote_header"), lines.join("\n>\n"))
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
    /// 1.5.2 keeps the RPC surface (`get_state`, `set_thinking_level`,
    /// `get_session_stats`, `get_commands`, `set_model`). RPC types, JSON
    /// events, and the JSONL codec match 1.4.0. The package depends on
    /// pi-codemode 1.5.2 exactly. The Node requirement is unchanged
    /// (`>=22.19.0`).
    pub const DESIRED_VERSION: &str = "1.5.2";
    static INSTALL_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    pub fn installation_in_progress() -> bool {
        INSTALL_LOCK.try_lock().is_err()
    }

    /// `InstallError.runtimeMissing` — shared with `PiToolchain.launch`.
    pub const RUNTIME_MISSING: &str =
        "Pitex Agent's JavaScript runtime is not installed yet. Click Install Pitex Agent in the Assistant or in Settings → AI.";

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
        // A second caller (the Install button during the launch-time
        // install) waits for the running install instead of failing.
        drop(INSTALL_LOCK.lock());
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
        let with_node;
        let tools = if tools.bun.is_none() && tools.npm_command(&[]).is_none() {
            install_pinned_node()?;
            with_node = PiToolchain::discover(std::env::vars().collect(),
                &dirs::home_dir().unwrap_or_default(), &PiToolchain::SYSTEM_DIRECTORIES);
            &with_node
        } else {
            tools
        };
        let runtime = pi_paths::runtime_directory();
        let backup = runtime.with_file_name(format!("pi-runtime-backup-{}", crate::model::uuid_v4()));
        let backed_up = runtime.exists();
        if backed_up { std::fs::rename(&runtime, &backup).map_err(|e| e.to_string())?; }
        let result = install_package(tools, if latest { "latest" } else { DESIRED_VERSION })
            .and_then(|_| installed_version().or_else(|| (!latest).then(|| DESIRED_VERSION.into()))
                .ok_or_else(|| "The installed agent version could not be verified.".to_string()));
        if let Err(error) = result {
            if backed_up || latest {
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
                    "--ignore-scripts".into(),
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
            "--ignore-scripts".into(),
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
        // is no launcher to publish and `entry` is only the success check.
        #[cfg(not(unix))]
        {
            let _ = entry;
        }
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

    /// Keep in step with `PinnedNode` in Mac/Sources/Features/PiAgentProcess.swift.
    pub const PINNED_NODE_VERSION: &str = "24.22.2";
    /// (platform, archive, SHA-256) from nodejs.org/dist/v24.22.2/SHASUMS256.txt.asc
    /// (release-key signed).
    const PINNED_NODE: [(&str, &str, &str); 4] = [
        ("linux-x64", "tar.gz", "361718c444e49af2ca5a110c83ba6a8901143f23ab285ebbef0341f391f482a9"),
        ("linux-arm64", "tar.gz", "95c264150aceb7f3437a29fd6984924f5f8d1511ea3cbd6c508b83f6c555dae2"),
        ("win-x64", "zip", "ca1bff5d8ead13c9f2ea404a176db46bd430704d5b5113734049fa365e7799e2"),
        ("win-arm64", "zip", "f47968c213105077f64c6510f007632c0ca2574802a0b5067c327c49abe1c468"),
    ];

    /// Neither Bun nor Node was found: download the pinned Node.js, verify
    /// its SHA-256, unpack it next to pi-runtime, then swap it in.
    fn install_pinned_node() -> Result<(), String> {
        let arch = match std::env::consts::ARCH { "x86_64" => "x64", "aarch64" => "arm64", other => other };
        let platform = format!("{}-{arch}", if cfg!(windows) { "win" } else { "linux" });
        let Some((_, extension, expected)) = PINNED_NODE.iter().find(|(p, ..)| *p == platform) else {
            return Err(format!("Pitex has no Node.js build for {platform}. Install Bun or Node.js, then retry."));
        };
        let name = format!("node-v{PINNED_NODE_VERSION}-{platform}");
        let target = pi_paths::pinned_node_directory();
        let staging = target.with_file_name(format!("node-staging-{}", crate::model::uuid_v4()));
        std::fs::create_dir_all(&staging).map_err(|e| e.to_string())?;
        let result = (|| {
            let archive = staging.join(format!("{name}.{extension}"));
            let url = format!("https://nodejs.org/dist/v{PINNED_NODE_VERSION}/{name}.{extension}");
            crate::update::curl(&["-fsSL", "-o", &archive.to_string_lossy(), &url], 600)
                .map_err(|e| format!("Node.js could not be downloaded from nodejs.org: {e}"))?;
            let data = std::fs::read(&archive).map_err(|e| e.to_string())?;
            if hex(&tex_domain::sha256(&data)) != *expected {
                return Err("The Node.js download did not match its pinned checksum, so it was not installed.".to_string());
            }
            // Windows' own bsdtar reads .zip; Git for Windows' GNU tar on PATH would not.
            let tar = if cfg!(windows) {
                format!("{}\\System32\\tar.exe", std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".into()))
            } else {
                "tar".into()
            };
            let unpack = run_install(Path::new(&tar),
                &["-xf".into(), archive.to_string_lossy().into_owned(), "-C".into(), staging.to_string_lossy().into_owned()],
                &std::collections::HashMap::new(), &staging, 300)?;
            let unpacked = staging.join(&name);
            let node = if cfg!(windows) { unpacked.join("node.exe") } else { unpacked.join("bin/node") };
            if unpack.termination != (build_core::ProcessTermination::Exited { code: 0 }) || !node.is_file() {
                return Err(format!("Node.js could not be unpacked: {}",
                    tail(&String::from_utf8_lossy(&unpack.standard_error), 1_000)));
            }
            // C headers are only for native addons, which the install skips (--ignore-scripts).
            let _ = std::fs::remove_dir_all(unpacked.join("include"));
            if target.exists() { std::fs::remove_dir_all(&target).map_err(|e| e.to_string())?; }
            std::fs::rename(&unpacked, &target).map_err(|e| e.to_string())
        })();
        let _ = std::fs::remove_dir_all(&staging);
        result
    }

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{b:02x}")).collect()
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
                format!("../share/{}/PitexAgent/skills", crate::identity::current().linux_share_dir_name),
                format!("../../share/{}/PitexAgent/skills", crate::identity::current().linux_share_dir_name),
            ] {
                let candidate = exe.parent()?.join(rel);
                if candidate.is_dir() {
                    return Some(candidate);
                }
            }
        }
        let dev = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../Mac/Resources/PitexAgent/skills");
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

    /// Windows counterpart — a `.bat` file typed into the embedded
    /// terminal's shell (`cmd` by default; an external emulator is the
    /// fallback). ConPTY gives `pi` a real console, so the `stty`/`script`
    /// plumbing the Unix PTY needs is unnecessary: export the environment,
    /// print the same guidance, launch the agent interactively.
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
            assert!(is_older("0.85.1", "1.0.2"));
            assert!(is_older("1.0.0", "1.0.2"));
            assert!(!is_older("1.0.2", "1.0.2"));
            assert!(!is_older("1.1.0", "1.0.2"));
            assert!(is_older("0.84.9", "0.85.1"));
            assert!(!is_older("0.85.1", "0.85.1"));
            assert!(!is_older("0.85.1-beta", "0.85.1"));
            assert!(is_older("0.85.0-rc1", "0.85.1"));
            assert!(is_older("1.0.2", "1.4.0"));
            assert!(!is_older("1.4.0", "1.0.2"));
            assert!(is_older("1.4.0", DESIRED_VERSION));
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

        #[test]
        fn discover_replaces_a_too_old_node_with_the_pinned_one() {
            let _env = env_lock();
            let dir = sandbox("old-node");
            let system = dir.parent().unwrap().join("system-bin");
            executable(&system.join("node"), "#!/bin/sh\necho v18.19.1\n");
            executable(&system.join("npm"), "#!/bin/sh\n");
            let environment = HashMap::from([
                ("PATH".to_string(), system.to_string_lossy().into_owned()),
                ("SHELL".to_string(), "/bin/false".to_string()),
            ]);
            // Without the pinned Node there is no usable runtime, so the
            // installer downloads it instead of failing pi's smoke check.
            let tools = PiToolchain::discover(environment.clone(), dir.parent().unwrap(), &[]);
            assert_eq!(tools.node, None);
            assert!(tools.npm_command(&[]).is_none());
            let pinned = pi_paths::pinned_node_bin();
            executable(&pinned.join("node"), "#!/bin/sh\necho v24.22.2\n");
            executable(&pinned.join("npm"), "#!/bin/sh\n");
            let tools = PiToolchain::discover(environment, dir.parent().unwrap(), &[]);
            assert_eq!(tools.node, Some(pinned.join("node")));
            assert_eq!(tools.npm, Some(pinned.join("npm")));
            let path = tools.environment.get("PATH").unwrap();
            assert!(path.starts_with(&*pinned.to_string_lossy()), "pinned Node must lead PATH: {path}");
            leave_sandbox(&dir);
        }

        #[test]
        #[ignore = "downloads Node.js and pi from the network; run alone"]
        fn pinned_node_runs_the_agent_install_without_a_system_runtime() {
            let _env = env_lock();
            let dir = sandbox("pinned-node");
            install_pinned_node().unwrap();
            // An empty PATH and no login shell: only the pinned Node exists.
            let environment = HashMap::from([("PATH".to_string(), String::new()), ("SHELL".to_string(), "/bin/false".to_string())]);
            let tools = PiToolchain::discover(environment, dir.parent().unwrap(), &[]);
            assert!(tools.bun.is_none());
            assert!(tools.node.as_ref().unwrap().starts_with(pi_paths::pinned_node_directory()));
            install_with_tools(&tools).unwrap();
            assert_eq!(installed_version().as_deref(), Some(DESIRED_VERSION));
            leave_sandbox(&dir);
        }

        fn leave_sandbox(dir: &Path) {
            std::env::remove_var("PI_CODING_AGENT_DIR");
            let _ = std::fs::remove_dir_all(dir.parent().unwrap_or(dir));
        }

        fn executable(path: &Path, body: &str) {
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            crate::write_test_executable(path, body, 0o755);
        }

        /// A fake bun that answers `add --cwd <dir>` by materializing the
        /// package layout npm would create, and `--bun <script> ...` by
        /// running the script — enough for the launcher smoke check.
        const FAKE_BUN: &str = "#!/bin/sh\n\
            if [ \"$1\" = \"add\" ]; then\n\
              dir=\"$3\"\n\
              pkg=\"$dir/node_modules/@earendil-works/pi-coding-agent\"\n\
              mkdir -p \"$pkg/dist\"\n\
              printf '#!/bin/sh\\necho 1.4.0\\n' > \"$pkg/dist/cli.js\"\n\
              chmod +x \"$pkg/dist/cli.js\"\n\
              printf '{\"name\":\"@earendil-works/pi-coding-agent\",\"version\":\"1.4.0\"}' > \"$pkg/package.json\"\n\
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
            executable(&bun, &FAKE_BUN.replace("1.4.0", DESIRED_VERSION));
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
            assert_eq!(installed_version().as_deref(), Some(DESIRED_VERSION));
            leave_sandbox(&dir);
        }

        #[test]
        fn failed_install_preserves_existing_launcher() {
            let _guard = env_lock();
            let dir = sandbox("fail");
            // Pre-existing launcher pointing at a legacy install — a failed
            // reinstall must not clobber it.
            // Keep the old entry INSIDE the managed tree: wiping packages
            // before a failed startup upgrade must not destroy this entry.
            let legacy = pi_paths::runtime_directory().join("node_modules/legacy/dist/cli.js");
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
            assert_eq!(std::fs::read_to_string(&legacy).unwrap(), "#!/bin/sh\necho old\n");
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
            let script = "#!/bin/sh\n\
                 if [ \"$1\" = \"add\" ]; then\n\
                   dir=\"$3\"\n\
                   pkg=\"$dir/node_modules/@earendil-works/pi-coding-agent\"\n\
                   if [ -d \"$pkg\" ]; then exit 0; fi\n\
                   mkdir -p \"$pkg/dist\"\n\
                   printf '#!/bin/sh\\necho 1.4.0\\n' > \"$pkg/dist/cli.js\"\n\
                   chmod +x \"$pkg/dist/cli.js\"\n\
                   printf '{\"name\":\"@earendil-works/pi-coding-agent\",\"version\":\"1.4.0\"}' > \"$pkg/package.json\"\n\
                   exit 0\n\
                 elif [ \"$1\" = \"--bun\" ]; then\n\
                   shift\n\
                   exec /bin/sh \"$@\"\n\
                 fi\n\
                 exit 1\n"
                .replace("1.4.0", DESIRED_VERSION);
            executable(&bun, &script);
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
            let script = "#!/bin/sh\n\
                 if [ \"$1\" = \"install\" ]; then\n\
                   dir=\"$3\"\n\
                   pkg=\"$dir/node_modules/@earendil-works/pi-coding-agent\"\n\
                   mkdir -p \"$pkg/dist\"\n\
                   printf '#!/bin/sh\\necho 1.4.0\\n' > \"$pkg/dist/cli.js\"\n\
                   chmod +x \"$pkg/dist/cli.js\"\n\
                   printf '{\"name\":\"@earendil-works/pi-coding-agent\",\"version\":\"1.4.0\"}' > \"$pkg/package.json\"\n\
                   exit 0\n\
                 fi\n\
                 exit 1\n"
                .replace("1.4.0", DESIRED_VERSION);
            executable(&tools_dir.join("npm"), &script);
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

    fn executable(path: &Path, body: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        crate::write_test_executable(path, body, 0o755);
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
        for version in ["v9.11.2", "v18.20.4", "v24.0.0"] {
            executable(
                &home.join(format!(".nvm/versions/node/{version}/bin/node")),
                &format!("#!/bin/sh\necho {version}\n"),
            );
        }
        executable(
            &home.join(".nvm/versions/node/v24.0.0/bin/npm"),
            "#!/bin/sh\n",
        );
        // Ambient PATH with no node — version-manager dirs are appended
        // after it, so the newest nvm install must win.
        let tools = PiToolchain::discover(environment("/nonexistent"), &home, &[]);
        assert_eq!(
            tools.node.as_deref(),
            Some(
                home.join(".nvm/versions/node/v24.0.0/bin/node")
                    .as_path()
                )
        );
        assert_eq!(
            tools.npm.as_deref(),
            Some(
                home.join(".nvm/versions/node/v24.0.0/bin/npm")
                    .as_path()
                )
        );
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn node_version_check_follows_pi_engines() {
        for (output, supported) in [
            ("v22.19.0\n", true),
            ("v22.20.1", true),
            ("v24.22.2", true),
            ("v100.0.0", true),
            ("v22.18.9", false),
            ("v22.9.0", false),
            ("v18.19.1", false),
            ("v9.11.2", false),
            ("22.19.0", false),
            ("", false),
            ("node: not found", false),
        ] {
            assert_eq!(node_version_meets_minimum(output), supported, "{output:?}");
        }
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
    fn launch_uses_node_for_typescript_without_bun() {
        let home = home("launchts");
        let node = home.join("bin/node");
        let entry = home.join("bridge/cli.ts");
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
    fn codex_launch_plan_runs_the_bridge_with_session_env() {
        let home = home("launchcodex");
        let node = home.join("bin/node");
        executable(&node, "#!/bin/sh\n");
        let script = bridge_script().expect("bundled bridge");
        assert!(script.ends_with("PitexAgent/bridge/cli.ts"));
        let tools = PiToolchain {
            environment: HashMap::new(),
            bun: None,
            node: Some(node.clone()),
            npm: None,
        };
        let sessions = home.join("sessions");
        let plan = AgentCoordinator::plan_launch("codex", Path::new("pi"), &sessions, &tools)
            .expect("plan");
        assert_eq!(plan.executable, node);
        assert!(plan.arguments.iter().any(|arg| arg.ends_with("bridge/cli.ts")));
        assert!(!plan.arguments.iter().any(|arg| arg == "--mode"));
        assert_eq!(
            plan.environment.get("PITEX_CLI_ADAPTER").map(String::as_str),
            Some("codex")
        );
        assert_eq!(
            plan.environment.get("PITEX_SESSION_DIR").map(String::as_str),
            Some(sessions.to_str().unwrap())
        );
        assert!(plan.environment.contains_key("PITEX_DEVELOPER_INSTRUCTIONS"));
        let pi = AgentCoordinator::plan_launch("pi", &node, &sessions, &tools).expect("pi plan");
        assert!(pi.arguments.iter().any(|arg| arg == "--mode"));
        assert!(!pi.environment.contains_key("PITEX_CLI_ADAPTER"));
        let agy = AgentCoordinator::plan_launch("agy", &node, &sessions, &tools).expect("agy plan");
        assert_eq!(agy.executable, node);
        assert!(agy.arguments.iter().any(|arg| arg.ends_with("bridge/cli.ts")));
        assert!(!agy.arguments.iter().any(|arg| arg == "--mode"));
        assert_eq!(
            agy.environment.get("PITEX_CLI_ADAPTER").map(String::as_str),
            Some("agy")
        );
        assert_eq!(
            agy.environment.get("PITEX_SESSION_DIR").map(String::as_str),
            Some(sessions.to_str().unwrap())
        );
        assert_eq!(
            bridge_missing_message("agy"),
            "The Antigravity CLI bridge is not installed with this Pitex build."
        );
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn claude_launch_plan_runs_the_bridge_with_session_env() {
        let home = home("launchclaude");
        let node = home.join("bin/node");
        executable(&node, "#!/bin/sh\n");
        let tools = PiToolchain {
            environment: HashMap::new(),
            bun: None,
            node: Some(node.clone()),
            npm: None,
        };
        let sessions = home.join("sessions");
        let plan = AgentCoordinator::plan_launch("claude", Path::new("pi"), &sessions, &tools)
            .expect("plan");
        assert_eq!(plan.executable, node);
        assert!(plan.arguments.iter().any(|arg| arg.ends_with("bridge/cli.ts")));
        assert!(!plan.arguments.iter().any(|arg| arg == "--mode"));
        assert_eq!(
            plan.environment.get("PITEX_CLI_ADAPTER").map(String::as_str),
            Some("claude")
        );
        assert_eq!(
            plan.environment.get("PITEX_SESSION_DIR").map(String::as_str),
            Some(sessions.to_str().unwrap())
        );
        assert!(plan.environment.contains_key("PITEX_DEVELOPER_INSTRUCTIONS"));
        assert_eq!(
            bridge_missing_message("claude"),
            "The Claude Code bridge is not installed with this Pitex build."
        );
        assert_eq!(
            bridge_missing_message("codex"),
            "The Codex bridge is not installed with this Pitex build."
        );
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn grok_launch_plan_runs_the_bridge_with_session_env() {
        let home = home("launchgrok");
        let node = home.join("bin/node");
        executable(&node, "#!/bin/sh\n");
        let tools = PiToolchain {
            environment: HashMap::new(),
            bun: None,
            node: Some(node.clone()),
            npm: None,
        };
        let sessions = home.join("sessions");
        let plan = AgentCoordinator::plan_launch("grok", Path::new("pi"), &sessions, &tools)
            .expect("plan");
        assert_eq!(plan.executable, node);
        assert!(plan.arguments.iter().any(|arg| arg.ends_with("bridge/cli.ts")));
        assert!(!plan.arguments.iter().any(|arg| arg == "--mode"));
        assert_eq!(
            plan.environment.get("PITEX_CLI_ADAPTER").map(String::as_str),
            Some("grok")
        );
        assert_eq!(
            plan.environment.get("PITEX_SESSION_DIR").map(String::as_str),
            Some(sessions.to_str().unwrap())
        );
        assert!(plan.environment.contains_key("PITEX_DEVELOPER_INSTRUCTIONS"));
        assert_eq!(
            bridge_missing_message("grok"),
            "The Grok CLI bridge is not installed with this Pitex build."
        );
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn apple_fm_launch_fails_closed() {
        let home = home("launchapple");
        let node = home.join("bin/node");
        executable(&node, "#!/bin/sh\n");
        let tools = PiToolchain {
            environment: HashMap::new(),
            bun: None,
            node: Some(node.clone()),
            npm: None,
        };
        let sessions = home.join("sessions");
        let error = match AgentCoordinator::plan_launch("apple-fm", &node, &sessions, &tools) {
            Ok(_) => panic!("apple-fm must not launch a process"),
            Err(error) => error,
        };
        assert_eq!(error, crate::settings::ON_DEVICE_UNAVAILABLE_MESSAGE);
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

/// One stdout line. A line past `cap` is discarded through its newline
/// and is not returned, so the next event stays aligned.
fn next_agent_stdout_line(reader: &mut impl BufRead, cap: usize) -> Option<Vec<u8>> {
    loop {
        let mut line = Vec::new();
        let mut skipped = false;
        loop {
            let mut byte = [0u8; 1];
            match reader.read(&mut byte) {
                Ok(0) | Err(_) => {
                    if skipped || line.is_empty() {
                        return None;
                    }
                    return Some(line);
                }
                Ok(_) if byte[0] == b'\n' => break,
                Ok(_) if skipped => {}
                Ok(_) => {
                    line.push(byte[0]);
                    if line.len() > cap {
                        skipped = true;
                        line.clear();
                    }
                }
            }
        }
        if line.last() == Some(&b'\r') {
            line.pop();
        }
        if skipped || line.is_empty() {
            continue;
        }
        return Some(line);
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

    #[test]
    fn resumed_bridge_results_keep_their_typed_cards() {
        let agent = AgentCoordinator::new(&crate::settings::SettingsStore::new(Default::default()));
        let messages = vec![
            json!({"role": "assistant", "content": [
                {"type": "toolCall", "id": "e1", "name": "edit", "arguments": {"path": "main.tex"}},
                {"type": "toolCall", "id": "b1", "name": "bash", "arguments": {"command": "latexmk"}},
            ]}),
            json!({"role": "toolResult", "toolCallId": "e1", "toolName": "edit", "isError": false,
                   "content": [{"type": "text", "text": "Edited"}], "status": "completed",
                   "detail": {"kind": "edit", "path": "/p/main.tex", "diff": "@@ -1 +1 @@\n-a\n+b"}}),
            json!({"role": "toolResult", "toolCallId": "b1", "toolName": "bash", "isError": true,
                   "content": [{"type": "text", "text": ""}], "status": "canceled",
                   "detail": {"kind": "shell", "command": "latexmk"}}),
        ];
        let rows = agent.transcript_from_messages(&messages);
        let edit = rows[0].tool.as_ref().expect("typed edit card");
        assert_eq!(edit.kind, "edit");
        assert_eq!(edit.diff_counts(), (1, 1));
        assert_eq!(rows[0].status, TranscriptStatus::Done);
        assert_eq!(rows[1].tool.as_ref().map(|t| t.kind.as_str()), Some("shell"));
        assert_eq!(rows[1].status, TranscriptStatus::Canceled);
        assert!(rows[1].detail.is_empty(), "a typed card drops the args summary, as live");
    }

    #[test]
    fn history_rows_badge_their_backend_from_the_file_name() {
        assert_eq!(session_source(Path::new("/s/claude-0b1c.jsonl")), "claude");
        assert_eq!(session_source(Path::new("/s/agy-x.jsonl")), "agy");
        assert_eq!(session_source(Path::new("/s/codex-x.jsonl")), "codex");
        assert_eq!(session_source(Path::new("/s/grok-x.jsonl")), "grok");
        assert_eq!(session_source(Path::new("/s/2026-11-20T10-00-00_abc.jsonl")), "pi");
        assert_eq!(session_source(Path::new("/s/claudette.jsonl")), "pi");
        assert_eq!(session_source_label("claude"), "Claude Code");
        assert_eq!(session_source_label("agy"), "Antigravity");
        assert_eq!(session_source_label("pi"), "pi");
    }

    #[test]
    fn history_rows_show_a_localized_relative_time() {
        let now = std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_800_000_000);
        let ago = |secs: u64| now - std::time::Duration::from_secs(secs);
        assert_eq!(relative_time("en", now, ago(20)), "Just now");
        assert_eq!(relative_time("en", now, ago(5 * 60)), "5 min ago");
        assert_eq!(relative_time("en", now, ago(3 * 3_600 + 59)), "3 h ago");
        assert_eq!(relative_time("en", now, ago(30 * 3_600)), "Yesterday");
        assert_eq!(relative_time("en", now, ago(4 * 86_400)), "4 days ago");
        assert_eq!(relative_time("ko", now, ago(5 * 60)), "5분 전");
        assert_eq!(relative_time("en", now, now + std::time::Duration::from_secs(9)), "Just now");
        let old = relative_time("en", now, ago(40 * 86_400));
        assert!(old.len() == 10 && old.starts_with("2026-"), "{old}");
    }

    #[test]
    fn history_search_matches_every_word_ignoring_case() {
        assert!(session_matches("Fix the Bibliography", "bib fix"));
        assert!(session_matches("anything", "  "));
        assert!(!session_matches("Fix the intro", "bib"));
        assert!(session_matches("한글 질문", "질문"));
    }

    #[test]
    fn the_history_list_opens_after_the_bridge_refreshes_its_sessions() {
        let mut agent = AgentCoordinator::new(&crate::settings::SettingsStore::new(Default::default()));
        let dir = std::env::temp_dir().join(format!("pitex-history-refresh-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        agent.session_directory = Some(dir.clone());
        agent.refresh_request_id = Some("r1".into());
        let reply = |id: &str| json!({"type": "response", "id": id, "command": "refresh_sessions", "success": true, "data": {"imported": 1}});
        // Written by the bridge before it replies.
        std::fs::write(dir.join("claude-new.jsonl"), [
            r#"{"type":"session","version":1,"id":"x","cwd":"/p","source":"claude"}"#,
            r#"{"type":"message","message":{"role":"user","content":[{"type":"text","text":"reply with ok"}]}}"#,
        ].join("\n")).unwrap();
        // A direct Codex mirror from an older build: no setting can open it, so it is not listed.
        std::fs::write(dir.join("codex-old.jsonl"), [
            r#"{"type":"session","version":1,"id":"y","cwd":"/p","source":"codex"}"#,
            r#"{"type":"message","message":{"role":"user","content":[{"type":"text","text":"old codex chat"}]}}"#,
        ].join("\n")).unwrap();
        agent.handle(&response(reply("stale")));
        assert!(!agent.history_requested, "only the reply the list waits for opens it");
        agent.handle(&response(reply("r1")));
        assert!(agent.history_requested);
        assert_eq!(agent.past_sessions.len(), 1);
        // A signed-out backend refuses the refresh; the list still opens from disk.
        agent.history_requested = false;
        agent.refresh_request_id = Some("r2".into());
        agent.handle(&response(json!({"type": "response", "id": "r2", "command": "refresh_sessions", "success": false, "error": "Sign in to Claude Code"})));
        assert!(agent.history_requested);
        // A turn started before the reply keeps the list closed.
        agent.history_requested = false;
        agent.refresh_request_id = Some("r3".into());
        agent.is_running = true;
        agent.handle(&response(reply("r3")));
        assert!(!agent.history_requested);
        assert!(agent.refresh_request_id.is_none());
        agent.is_running = false;
        assert_eq!(agent.past_sessions[0].title, "reply with ok");
        assert_eq!(agent.past_sessions[0].source, "claude");
        // pi (no bridge) lists at once.
        agent.history_requested = false;
        agent.open_history();
        assert!(agent.history_requested);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_turned_off_backend_row_says_so_instead_of_failing() {
        let mut store = crate::settings::SettingsStore::new(Default::default());
        let mut agent = AgentCoordinator::new(&store);
        agent.catalog_slots.retain(|slot| slot.backend != "agy");
        let session = AgentSessionSummary {
            path: PathBuf::from("/s/agy-x.jsonl"),
            modified: std::time::SystemTime::now(),
            title: "Old".into(),
            source: "agy",
        };
        agent.open_session(&session, &mut store, "en");
        let last = agent.transcript.last().expect("a notice");
        assert_eq!(last.role, TranscriptRole::Notice);
        assert!(last.text.contains("Antigravity") && last.text.contains("turned off"), "{}", last.text);
        assert!(agent.pending_resume.is_none());

        // An enabled backend: switch first, open the row once it is up.
        let claude = AgentSessionSummary { path: PathBuf::from("/s/claude-x.jsonl"), source: "claude", ..session };
        assert!(agent.catalog_slots.iter().any(|slot| slot.backend == "claude"));
        agent.open_session(&claude, &mut store, "en");
        assert_eq!(agent.provider(), "claude");
        assert_eq!(store.agent_provider(), "claude");
        assert_eq!(agent.pending_resume.as_deref(), Some(Path::new("/s/claude-x.jsonl")));
    }

    #[test]
    fn tool_summary_prints_a_compact_fallback_without_json() {
        let summary = json!({"summary": "WebSearch: pitex latex"});
        assert_eq!(
            tool_summary("WebSearch", summary.as_object()),
            "WebSearch: pitex latex"
        );
        let bash = json!({"command": "latexmk main.tex", "description": "Compile the document"});
        assert_eq!(tool_summary("bash", bash.as_object()), "latexmk main.tex");
        let extra = json!({"command": "ls", "description": "list"});
        assert!(tool_summary("custom", extra.as_object()).contains("description"));
    }

    fn feed(agent: &mut AgentCoordinator, event: Value) {
        agent.handle(&PiRPCEvent::new(event.as_object().unwrap().clone()).unwrap());
    }

    #[test]
    fn typed_tool_events_merge_into_one_card() {
        let mut agent = AgentCoordinator::new(&crate::settings::SettingsStore::new(Default::default()));
        feed(&mut agent, json!({"type": "agent_start", "turnId": "t1"}));
        feed(&mut agent, json!({
            "type": "tool_execution_start", "toolCallId": "e1", "toolName": "edit", "turnId": "t1",
            "args": {"path": "/p/chapters/intro.tex"}, "status": "running",
            "detail": {"kind": "edit", "path": "/p/chapters/intro.tex", "oldText": "a", "newText": "b",
                       "diff": "@@ -1,2 +1,3 @@\n a\n-b\n+B\n+C"}
        }));
        let card = agent.transcript.last().unwrap();
        assert_eq!(card.status, TranscriptStatus::Running);
        assert!(card.detail.is_empty(), "a typed card does not repeat the args summary");
        // A later untyped update never replaces the typed kind.
        feed(&mut agent, json!({
            "type": "tool_execution_update", "toolCallId": "e1",
            "partialResult": {"content": [{"type": "text", "text": "working"}]},
            "detail": {"kind": "other", "label": "edit"}
        }));
        feed(&mut agent, json!({
            "type": "tool_execution_end", "toolCallId": "e1", "isError": false, "status": "completed",
            "result": {"content": [{"type": "text", "text": "ok"}]},
            "detail": {"kind": "edit", "diff": "@@ -10,1 +10,2 @@\n-b\n+B\n+C"}
        }));
        let card = agent.transcript.last().unwrap();
        let tool = card.tool.as_ref().unwrap();
        assert_eq!(card.status, TranscriptStatus::Done);
        assert_eq!(tool.kind, "edit");
        assert_eq!(tool.path.as_deref(), Some("/p/chapters/intro.tex"));
        assert_eq!(tool.diff.as_deref(), Some("@@ -10,1 +10,2 @@\n-b\n+B\n+C"));
        assert_eq!(tool.diff_counts(), (2, 1));
        let root = Path::new("/p");
        assert_eq!(tool.title("edit", Some(root), "en"), "Edited chapters/intro.tex");
        assert_eq!(tool.badge("en").as_deref(), Some("+2 -1"));
    }

    #[test]
    fn tool_titles_and_badges_per_kind() {
        let detail = |value: Value| ToolDetail::decode("id", value.as_object().unwrap()).unwrap();
        let root = Some(Path::new("/p"));
        let shell = detail(json!({"kind": "shell", "command": "latexmk -pdf main.tex", "exitCode": 1}));
        assert_eq!(shell.title("bash", root, "en"), "Ran latexmk -pdf main.tex");
        assert_eq!(shell.badge("en").as_deref(), Some("exit 1"));
        let ok = detail(json!({"kind": "shell", "command": "ls", "exitCode": 0}));
        assert_eq!(ok.badge("en"), None);
        let write = detail(json!({"kind": "write", "path": "/p/refs.bib", "diff": "@@ -0,0 +1,1 @@\n+@a{b}"}));
        assert_eq!(write.title("write", root, "en"), "Wrote refs.bib");
        assert_eq!(write.badge("en").as_deref(), Some("+1 -0"));
        let read = detail(json!({"kind": "read", "path": "/p/main.tex", "startLine": 10, "endLine": 40}));
        assert_eq!(read.title("read", root, "en"), "Read main.tex (lines 10-40)");
        let search = detail(json!({"kind": "search", "query": "\\cite", "numMatches": 12}));
        assert_eq!(search.title("Grep", root, "en"), "Searched \"\\cite\" · 12 matches");
        let fetch = detail(json!({"kind": "fetch", "url": "https://ctan.org/pkg/biblatex"}));
        assert_eq!(fetch.title("WebFetch", root, "en"), "Fetched ctan.org");
        let other = detail(json!({"kind": "other", "label": "TodoWrite"}));
        assert_eq!(other.title("TodoWrite", root, "en"), "TodoWrite");
        // Outside the project: the full path, and no Open file target.
        let outside = detail(json!({"kind": "edit", "path": "/etc/hosts"}));
        assert_eq!(outside.title("edit", root, "en"), "Edited /etc/hosts");
        assert_eq!(outside.openable_path(root), None);
        assert_eq!(write.openable_path(root), Some(PathBuf::from("/p/refs.bib")));
        assert_eq!(shell.openable_path(root), None);
        // A huge diff is capped with a meta line the diff parser keeps.
        let huge = format!("@@ -1,1 +1,9000 @@\n{}", "+line\n".repeat(9_000));
        let capped = detail(json!({"kind": "write", "diff": huge})).diff.unwrap();
        assert!(capped.len() <= TOOL_DIFF_CAP + 20);
        assert!(capped.ends_with("\\ diff truncated"));
    }

    #[test]
    fn canceled_tool_end_marks_the_card_canceled_and_pi_events_still_work() {
        let mut agent = AgentCoordinator::new(&crate::settings::SettingsStore::new(Default::default()));
        feed(&mut agent, json!({
            "type": "tool_execution_start", "toolCallId": "b1", "toolName": "bash",
            "args": {"command": "sleep 9"}, "detail": {"kind": "shell", "command": "sleep 9"}, "status": "running"
        }));
        feed(&mut agent, json!({
            "type": "tool_execution_end", "toolCallId": "b1", "isError": true, "status": "canceled",
            "result": {"content": [{"type": "text", "text": ""}]}
        }));
        assert_eq!(agent.transcript.last().unwrap().status, TranscriptStatus::Canceled);
        // pi's own provider: no detail, no status — today's name + summary row.
        feed(&mut agent, json!({"type": "tool_execution_start", "toolCallId": "p1", "toolName": "bash", "args": {"command": "ls"}}));
        feed(&mut agent, json!({"type": "tool_execution_end", "toolCallId": "p1", "isError": true}));
        let pi = agent.transcript.last().unwrap();
        // An untyped card that keeps the call id, so its open state survives updates.
        let tool = pi.tool.as_ref().unwrap();
        assert_eq!((tool.kind.as_str(), tool.call_id.as_str()), ("other", "p1"));
        assert_eq!(pi.detail, "ls");
        assert_eq!(pi.status, TranscriptStatus::Failed);
        // An untyped bridge card (MCP tool) keeps its arguments too.
        feed(&mut agent, json!({
            "type": "tool_execution_start", "toolCallId": "m1", "toolName": "mcp__x__search",
            "args": {"query": "biblatex"}, "detail": {"kind": "other", "label": "mcp__x__search"}
        }));
        assert!(agent.transcript.last().unwrap().detail.contains("biblatex"));
    }

    #[test]
    fn approval_request_waits_until_the_user_answers() {
        let mut agent = AgentCoordinator::new(&crate::settings::SettingsStore::new(Default::default()));
        let request = json!({
            "type": "approval_request",
            "id": "42",
            "kind": "command",
            "title": "Run command",
            "detail": "ls"
        });
        agent.handle(&PiRPCEvent::new(request.as_object().unwrap().clone()).unwrap());
        let pending = agent.pending_approval.clone().expect("pending");
        assert_eq!(pending.id, "42");
        assert_eq!(pending.detail, "ls");
        agent.respond_to_approval("always_allow");
        assert!(agent.pending_approval.is_none());
        assert_eq!(
            agent.approval_wire,
            vec![json!({"type": "approval_response", "id": "42", "decision": "always_allow"})]
        );
    }

    fn event(value: Value) -> PiRPCEvent {
        PiRPCEvent::new(value.as_object().unwrap().clone()).unwrap()
    }

    #[test]
    fn approval_cards_decode_actions_questions_plan_and_tool_detail() {
        let plan = AgentApprovalRequest::decode(
            json!({
                "type": "approval_request", "id": "p", "kind": "plan", "title": "Review the plan",
                "detail": "## Plan", "plan": "## Plan\n1. Edit",
                "actions": [
                    {"id": "approve_plan", "label": "Approve plan", "behavior": "allow", "variant": "primary"},
                    {"id": "bogus", "label": "x", "behavior": "maybe"},
                    {"id": "keep_planning", "label": "Keep planning", "behavior": "deny"}
                ]
            })
            .as_object()
            .unwrap(),
        );
        assert_eq!(plan.kind, "plan");
        assert_eq!(plan.plan.as_deref(), Some("## Plan\n1. Edit"));
        assert_eq!(
            plan.actions.iter().map(|a| (a.id.as_str(), a.allow, a.variant.as_str())).collect::<Vec<_>>(),
            vec![("approve_plan", true, "primary"), ("keep_planning", false, "")],
            "an action with an unknown behavior is dropped"
        );
        let question = AgentApprovalRequest::decode(
            json!({
                "id": "q", "kind": "question", "title": "Q",
                "questions": [
                    {"id": "0", "header": "Engine", "question": "Which engine?", "multiSelect": false, "allowOther": true,
                     "options": [{"label": "pdfLaTeX", "description": "Fast"}, {"label": ""}, {"label": "LuaLaTeX"}]},
                    {"id": "1", "header": "", "question": ""}
                ]
            })
            .as_object()
            .unwrap(),
        );
        assert_eq!(question.questions.len(), 1, "a question without text is dropped");
        let q = &question.questions[0];
        assert_eq!(q.options, vec![("pdfLaTeX".into(), "Fast".into()), ("LuaLaTeX".into(), String::new())]);
        assert!(q.allow_other && !q.multi_select);
        let edit = AgentApprovalRequest::decode(
            json!({"id": "e", "kind": "file", "title": "Edit files", "detail": "a.tex",
                   "toolDetail": {"kind": "edit", "path": "a.tex", "diff": "@@ -1 +1 @@\n-a\n+b"}})
            .as_object()
            .unwrap(),
        );
        let tool = edit.tool.expect("tool detail");
        assert_eq!((tool.kind.as_str(), tool.diff.as_deref()), ("edit", Some("@@ -1 +1 @@\n-a\n+b")));
        assert!(edit.actions.is_empty() && edit.plan.is_none());
        // Old bridges and pi: only the original four fields.
        let legacy = AgentApprovalRequest::decode(json!({"id": "1"}).as_object().unwrap());
        assert_eq!((legacy.kind.as_str(), legacy.title.as_str()), ("command", "Approval"));
    }

    #[test]
    fn card_answers_carry_the_action_answers_and_feedback() {
        let mut agent = AgentCoordinator::new(&crate::settings::SettingsStore::new(Default::default()));
        agent.handle(&event(json!({"type": "approval_request", "id": "q", "kind": "question", "title": "Q",
            "questions": [{"id": "0", "header": "H", "question": "Which?", "options": [{"label": "A"}], "multiSelect": true}]})));
        agent.handle(&event(json!({"type": "approval_request", "id": "p", "kind": "plan", "title": "Plan", "plan": "x",
            "actions": [{"id": "keep_planning", "label": "Keep planning", "behavior": "deny"}]})));
        let mut answers = Map::new();
        answers.insert("0".into(), json!(["A", "Other text"]));
        agent.answer_approval("q", ApprovalAnswer { decision: "approve".into(), answers, ..Default::default() });
        let keep = agent.pending_approval.clone().unwrap().actions[0].clone();
        agent.answer_approval("p", ApprovalAnswer::action(&keep, Some("  Also the abstract. ".into())));
        assert_eq!(
            agent.approval_wire,
            vec![
                json!({"type": "approval_response", "id": "q", "decision": "approve", "answers": {"0": ["A", "Other text"]}}),
                json!({"type": "approval_response", "id": "p", "decision": "deny", "actionId": "keep_planning",
                       "message": "Also the abstract."}),
            ]
        );
        let always = ApprovalAction { id: "always_allow".into(), label: String::new(), allow: true, variant: String::new() };
        assert_eq!(ApprovalAnswer::action(&always, None).decision, "always_allow");
        let once = ApprovalAction { id: "allow_once".into(), label: String::new(), allow: true, variant: String::new() };
        assert_eq!(ApprovalAnswer::action(&once, None).decision, "approve");
    }

    #[test]
    fn approval_resolved_drops_the_card_the_bridge_settled() {
        let mut agent = AgentCoordinator::new(&crate::settings::SettingsStore::new(Default::default()));
        agent.handle(&approval_event("1", "first"));
        agent.handle(&approval_event("2", "second"));
        agent.handle(&event(json!({"type": "approval_resolved", "id": "1", "outcome": "cancelled"})));
        assert_eq!(agent.pending_approval.as_ref().map(|a| a.id.as_str()), Some("2"));
        agent.handle(&event(json!({"type": "approval_resolved", "id": "9", "outcome": "cancelled"})));
        agent.handle(&event(json!({"type": "approval_resolved", "id": "2", "outcome": "denied"})));
        assert!(agent.pending_approval.is_none());
        assert!(agent.approval_sent.is_empty(), "a settled card is never answered again");
    }

    #[test]
    fn an_answer_for_a_settled_card_does_not_reach_the_next_request() {
        let mut agent = AgentCoordinator::new(&crate::settings::SettingsStore::new(Default::default()));
        agent.handle(&approval_event("1", "question"));
        agent.handle(&approval_event("2", "edit"));
        agent.handle(&event(json!({"type": "approval_resolved", "id": "1", "outcome": "cancelled"})));
        agent.answer_approval("1", ApprovalAnswer::decision("approve"));
        assert!(agent.approval_sent.is_empty(), "the click on card 1 is dropped");
        assert_eq!(agent.pending_approval.as_ref().map(|a| a.id.as_str()), Some("2"));
        agent.answer_approval("2", ApprovalAnswer::decision("deny"));
        assert_eq!(agent.approval_sent, vec![("2".into(), "deny".into())]);
    }

    #[test]
    fn mode_comes_from_mode_changed_and_select_mode_only_picks_listed_modes() {
        let mut agent = AgentCoordinator::new(&crate::settings::SettingsStore::new(Default::default()));
        assert!(agent.mode.is_none(), "pi reports no mode: no picker");
        agent.handle(&event(json!({"type": "mode_changed", "mode": {"id": "default", "options": [
            {"id": "default", "label": "Ask before edits"}, {"id": "accept-edits"}, {"id": "plan", "label": "Plan"}]}})));
        let mode = agent.mode.clone().expect("mode");
        assert_eq!(mode.options[1], ("accept-edits".into(), "accept-edits".into()));
        assert!(!mode.auto_applies_edits());
        agent.select_mode("bypassPermissions");
        assert_eq!(agent.mode.as_ref().unwrap().id, "default", "a mode the bridge did not list is ignored");
        agent.select_mode("accept-edits");
        assert!(agent.mode.as_ref().unwrap().auto_applies_edits());
        assert_eq!(AgentMode::label_key("acceptEdits", "Accept edits"), Some("assistant.mode.accept_edits"));
        assert_eq!(AgentMode::label_key("default", "Ask before edits"), Some("assistant.mode.default"));
        assert_eq!(AgentMode::label_key("default", "Default"), Some("assistant.mode.cli_default"), "agy's default");
        assert_eq!(AgentMode::label_key("auto", "Auto"), None);
        let agy = AgentMode { id: "default".into(), options: vec![("default".into(), "Default".into())] };
        assert!(agy.auto_applies_edits(), "agy's default edits without asking");
        assert_eq!(
            PiRPCCommand::SetMode { mode_id: "plan".into() }.object(),
            json!({"type": "set_mode", "modeId": "plan"})
        );
        agent.handle(&event(json!({"type": "mode_changed", "mode": {"id": "x", "options": []}})));
        assert!(agent.mode.is_none(), "no options: no picker");
    }

    #[test]
    fn approval_requests_queue_in_order() {
        let mut agent = AgentCoordinator::new(&crate::settings::SettingsStore::new(Default::default()));
        agent.handle(&approval_event("1", "first"));
        agent.handle(&approval_event("2", "second"));
        let head = agent.pending_approval.clone().expect("head");
        assert_eq!(head.id, "1");
        assert_eq!(head.detail, "first");
        agent.respond_to_approval("approve");
        let next = agent.pending_approval.clone().expect("second");
        assert_eq!(next.id, "2");
        assert_eq!(next.detail, "second");
        agent.respond_to_approval("deny");
        assert!(agent.pending_approval.is_none());
    }

    #[test]
    fn repeated_approval_id_keeps_its_place() {
        let mut agent = AgentCoordinator::new(&crate::settings::SettingsStore::new(Default::default()));
        agent.handle(&approval_event("1", "old"));
        agent.handle(&approval_event("2", "other"));
        agent.handle(&approval_event("1", "updated"));
        let head = agent.pending_approval.clone().expect("head");
        assert_eq!(head.id, "1");
        assert_eq!(head.detail, "updated");
        agent.respond_to_approval("approve");
        assert_eq!(agent.pending_approval.as_ref().map(|item| item.id.as_str()), Some("2"));
    }

    #[test]
    fn stop_and_new_session_deny_every_queued_approval() {
        let mut agent = AgentCoordinator::new(&crate::settings::SettingsStore::new(Default::default()));
        agent.handle(&approval_event("7", "edit"));
        agent.handle(&approval_event("8", "shell"));
        agent.stop();
        assert!(agent.pending_approval.is_none());
        assert_eq!(
            agent.approval_sent,
            vec![("7".into(), "deny".into()), ("8".into(), "deny".into())]
        );
        agent.respond_to_approval("approve");
        assert!(agent.pending_approval.is_none());

        agent.approval_sent.clear();
        agent.handle(&approval_event("9", "again"));
        agent.handle(&approval_event("10", "later"));
        agent.new_session();
        assert!(agent.pending_approval.is_none());
        assert_eq!(
            agent.approval_sent,
            vec![("9".into(), "deny".into()), ("10".into(), "deny".into())]
        );
    }

    fn queue_agent(steer: bool) -> AgentCoordinator {
        let mut agent = AgentCoordinator::new(&crate::settings::SettingsStore::new(Default::default()));
        agent.capabilities = AgentCapabilities { steer, images: true, ..Default::default() };
        agent
    }

    fn png() -> crate::composer::AgentImage {
        crate::composer::AgentImage::new("red.png", "image/png", &[137, 80, 78, 71], 0).unwrap()
    }

    fn delta(kind: &str, index: i64, text: &str) -> PiRPCEvent {
        event(json!({"type": "message_update", "assistantMessageEvent": {"type": kind, "contentIndex": index, "delta": text}}))
    }

    #[test]
    fn text_then_a_tool_then_more_text_is_two_rows_without_a_repeat() {
        let mut agent = queue_agent(false);
        agent.handle(&event(json!({"type": "agent_start"})));
        agent.handle(&event(json!({"type": "message_start", "message": {"role": "assistant", "modelName": "Claude Sonnet 4.5"}})));
        agent.handle(&delta("text_start", 0, ""));
        agent.handle(&delta("text_delta", 0, "First."));
        agent.handle(&event(json!({"type": "tool_execution_start", "toolCallId": "t1", "toolName": "read", "args": {"path": "main.tex"}})));
        agent.handle(&event(json!({"type": "tool_execution_end", "toolCallId": "t1", "isError": false, "result": {"content": []}})));
        agent.handle(&delta("text_start", 1, ""));
        agent.handle(&delta("text_delta", 1, "Second."));
        // An older bridge put the whole turn's text in message_end.
        agent.handle(&event(json!({"type": "message_end", "message": {"role": "assistant", "content": [{"type": "text", "text": "First.Second."}]}})));
        agent.handle(&event(json!({"type": "agent_end", "outcome": "completed"})));
        let rows: Vec<(TranscriptRole, &str)> = agent.transcript.iter().map(|e| (e.role, e.text.as_str())).collect();
        assert_eq!(rows, vec![(TranscriptRole::Assistant, "First."), (TranscriptRole::Tool, ""), (TranscriptRole::Assistant, "Second.")]);
        assert!(agent.transcript.iter().all(|e| e.status != TranscriptStatus::Streaming), "no row is left streaming");
        assert_eq!(agent.transcript[0].title, "Pitex Agent · Claude Sonnet 4.5", "the answering model, not the picker alias");
        // A message_end for a row that never streamed still fills it.
        agent.handle(&event(json!({"type": "message_start", "message": {"role": "assistant"}})));
        agent.handle(&event(json!({"type": "message_end", "message": {"role": "assistant", "content": [{"type": "text", "text": "Only end"}]}})));
        assert_eq!(agent.transcript.last().unwrap().text, "Only end");
    }

    #[test]
    fn thinking_closes_with_its_duration_when_the_answer_starts() {
        let mut agent = queue_agent(false);
        agent.handle(&event(json!({"type": "message_start", "message": {"role": "assistant"}})));
        agent.handle(&delta("thinking_start", 0, ""));
        agent.handle(&delta("thinking_delta", 0, "plan"));
        assert_eq!(agent.transcript[0].status, TranscriptStatus::Streaming);
        assert_eq!(agent.transcript[0].meta, EntryMeta::Thought { seconds: None });
        agent.handle(&delta("text_delta", 1, "Answer"));
        assert_eq!(agent.transcript[0].status, TranscriptStatus::Done);
        assert_eq!(agent.transcript[0].meta, EntryMeta::Thought { seconds: Some(0) });
        // A tool call after a thought also closes it.
        agent.handle(&delta("thinking_start", 2, ""));
        agent.handle(&event(json!({"type": "tool_execution_start", "toolCallId": "t", "toolName": "bash", "args": {}})));
        assert_eq!(agent.transcript[2].status, TranscriptStatus::Done);
    }

    #[test]
    fn usage_update_feeds_the_meter_and_stats_are_the_fallback() {
        let mut agent = queue_agent(false);
        assert!(agent.context_meter().is_none());
        agent.session_stats = Some(PiSessionStats { context_tokens: Some(50), context_window: Some(100), ..Default::default() });
        assert_eq!(agent.context_meter().unwrap().percent, 50.0, "pi: pulled stats");
        agent.handle(&event(json!({"type": "usage_update", "contextUsed": 150_000, "contextMax": 200_000, "costUsd": 0.5})));
        let meter = agent.context_meter().unwrap();
        assert_eq!((meter.label().as_str(), meter.band()), ("75% · 150k/200k", crate::turn_state::MeterBand::Warning));
        agent.handle(&event(json!({"type": "usage_update", "contextUsed": 190_000})));
        assert_eq!(agent.context_meter().unwrap().band(), crate::turn_state::MeterBand::Danger);
        assert_eq!(agent.usage.cost_usd, Some(0.5));
    }

    #[test]
    fn a_failed_turn_offers_retry_with_the_same_message_and_images() {
        let mut agent = queue_agent(false);
        agent.submit("fix the bib", vec![png()], false);
        agent.handle(&event(json!({"type": "agent_start"})));
        agent.handle(&event(json!({"type": "agent_end", "outcome": "failed", "code": "rate_limited", "error": "API Error: 429"})));
        let error = agent.transcript.last().unwrap();
        assert_eq!(error.text, "API Error: 429");
        assert!(matches!(&error.meta, EntryMeta::TurnError { code: crate::turn_state::ErrorCode::RateLimited, retry: true, .. }));
        agent.retry();
        assert_eq!(agent.prompt_wire.len(), 2);
        assert_eq!(agent.prompt_wire[0]["images"], agent.prompt_wire[1]["images"]);
        assert!(agent.prompt_wire[1]["message"].as_str().unwrap().contains("fix the bib"));
        let user = agent.transcript.last().unwrap();
        assert_eq!((user.role, user.text.as_str(), user.images.len()), (TranscriptRole::User, "fix the bib", 1));
        assert!(
            agent.transcript.iter().all(|e| !matches!(e.meta, EntryMeta::TurnError { retry: true, .. })),
            "Retry goes away once the message went out again"
        );
        // Stop is not a failure: a plain notice, no Retry.
        agent.handle(&event(json!({"type": "agent_start"})));
        agent.handle(&event(json!({"type": "agent_end", "outcome": "canceled", "messages": [{"role": "assistant", "stopReason": "error", "errorMessage": "[Request interrupted by user]"}]})));
        let stopped = agent.transcript.last().unwrap();
        assert_eq!((stopped.role, stopped.status, &stopped.meta), (TranscriptRole::Notice, TranscriptStatus::Done, &EntryMeta::None));
        // pi sends no outcome: its error still gets the row, as `other`.
        agent.handle(&event(json!({"type": "agent_end", "messages": [{"role": "assistant", "stopReason": "error", "errorMessage": "boom"}]})));
        assert!(matches!(agent.transcript.last().unwrap().meta, EntryMeta::TurnError { code: crate::turn_state::ErrorCode::Other, .. }));
        // Like Mac: a completed turn gets no error row, whatever its last message says.
        let rows = agent.transcript.len();
        agent.handle(&event(json!({"type": "agent_end", "outcome": "completed", "messages": [{"role": "assistant", "stopReason": "error", "errorMessage": "late"}]})));
        assert_eq!(agent.transcript.len(), rows);
    }

    #[test]
    fn a_crash_mid_chat_leaves_an_error_row_with_retry() {
        let mut agent = queue_agent(false);
        agent.submit("hello", Vec::new(), false);
        agent.process_did_exit();
        assert!(matches!(
            agent.transcript.last().unwrap().meta,
            EntryMeta::TurnError { code: crate::turn_state::ErrorCode::Crashed, retry: true, .. }
        ));
        // A new message before the relaunch replaces the pending Retry (no double send).
        agent.retry();
        assert!(agent.pending_retry);
        agent.submit("another", Vec::new(), false);
        assert!(!agent.pending_retry);
    }

    #[test]
    fn a_crash_after_the_answer_offers_no_retry_and_a_restart_drops_the_live_model() {
        let mut agent = queue_agent(false);
        agent.submit("hello", Vec::new(), false);
        agent.handle(&event(json!({"type": "agent_start"})));
        agent.handle(&event(json!({"type": "message_start", "message": {"role": "assistant", "modelName": "Claude Opus 4.1"}})));
        agent.handle(&delta("text_delta", 0, "Hi"));
        agent.handle(&event(json!({"type": "agent_end", "outcome": "completed"})));
        agent.process_did_exit();
        assert!(matches!(
            agent.transcript.last().unwrap().meta,
            EntryMeta::TurnError { code: crate::turn_state::ErrorCode::Crashed, retry: false, .. }
        ));
        assert_eq!(agent.live_model_name.as_deref(), Some("Claude Opus 4.1"));
        agent.restart();
        assert_eq!(agent.live_model_name, None, "another backend names no model");
        agent.clear_transcript();
        assert!(agent.last_sent.is_none(), "Retry belongs to the chat it was sent in");
    }

    #[test]
    fn return_during_a_turn_queues_and_agent_end_sends_one_at_a_time() {
        let mut agent = queue_agent(true);
        agent.submit("start", Vec::new(), false);
        assert_eq!(agent.prompt_wire.len(), 1, "idle: sent at once");
        assert!(agent.prompt_wire[0].get("streamingBehavior").is_none());
        agent.handle(&event(json!({"type": "agent_start"})));
        agent.submit("first", vec![png()], false);
        agent.submit("second", Vec::new(), false);
        assert_eq!(agent.prompt_wire.len(), 1, "running: Return queues");
        assert_eq!(agent.queue.items.iter().map(|i| i.text.as_str()).collect::<Vec<_>>(), vec!["first", "second"]);
        assert_eq!(agent.transcript.iter().filter(|e| e.role == TranscriptRole::User).count(), 1, "queued rows are not in the transcript yet");

        agent.handle(&event(json!({"type": "agent_end"})));
        assert_eq!(agent.prompt_wire.len(), 2);
        let wire = &agent.prompt_wire[1];
        assert_eq!(wire["type"], "prompt");
        assert_eq!(wire["streamingBehavior"], "followUp");
        assert_eq!(wire["images"][0], json!({"type": "image", "mimeType": "image/png", "data": png().data.to_string()}));
        assert_eq!(agent.queue.items[0].state, crate::composer::QueuedState::Sending);
        // Return before the drained prompt is accepted queues too: pi would refuse a second plain prompt.
        agent.submit("third", Vec::new(), false);
        assert_eq!(agent.prompt_wire.len(), 2);
        assert_eq!(agent.queue.items.len(), 3);
        assert!(!agent.can_send_queued_now(), "no Send now while a drained prompt is in flight");
        assert_eq!(agent.transcript.iter().filter(|e| e.role == TranscriptRole::User).count(), 1, "the bubble waits for the response");
        let id = agent.drain_request.clone().unwrap().0;
        agent.handle(&event(json!({"type": "response", "command": "prompt", "success": true, "id": id})));
        let user = agent.transcript.iter().rev().find(|e| e.role == TranscriptRole::User).unwrap();
        assert_eq!((user.text.as_str(), user.images.len()), ("first", 1));
        assert_eq!(agent.queue.items.len(), 2);
        agent.handle(&event(json!({"type": "agent_start"})));
        agent.handle(&event(json!({"type": "agent_end"})));
        assert!(agent.prompt_wire[2]["message"].as_str().unwrap().ends_with("
second"), "the envelope wraps the text");
    }

    #[test]
    fn a_refused_drain_goes_back_to_the_front_and_waits() {
        let mut agent = queue_agent(true);
        agent.handle(&event(json!({"type": "agent_start"})));
        agent.submit("one", Vec::new(), false);
        agent.submit("two", Vec::new(), false);
        agent.handle(&event(json!({"type": "agent_end"})));
        let id = agent.drain_request.clone().unwrap().0;
        agent.handle(&event(json!({"type": "response", "command": "prompt", "success": false, "id": id, "error": "busy"})));
        assert_eq!(agent.queue.items[0].text, "one");
        assert_eq!(agent.queue.items[0].state, crate::composer::QueuedState::Queued);
        assert!(agent.queue.paused);
        assert!(agent.transcript.iter().all(|e| e.role != TranscriptRole::User), "a refused drain leaves no bubble");
        agent.handle(&event(json!({"type": "agent_end"})));
        assert_eq!(agent.prompt_wire.len(), 1, "paused: nothing more goes out");
        // Send now on the row, while idle, sends it as a plain prompt.
        assert!(agent.can_send_queued_now(), "an idle paused queue offers Send now");
        let head = agent.queue.items[0].id;
        agent.send_queued_now(head);
        assert!(agent.prompt_wire[1]["message"].as_str().unwrap().ends_with("
one"), "the envelope wraps the text");
        assert_eq!(agent.queue.items.len(), 1);
    }

    #[test]
    fn steer_sends_into_the_turn_and_an_approval_forces_the_queue() {
        let mut agent = queue_agent(true);
        agent.handle(&event(json!({"type": "agent_start"})));
        agent.submit("now please", Vec::new(), true);
        assert_eq!(agent.prompt_wire[0]["type"], "steer");
        assert!(agent.prompt_wire[0]["message"].as_str().unwrap().ends_with("
now please"), "the envelope wraps the text");
        agent.submit("later", Vec::new(), false);
        let queued = agent.queue.items[0].id;
        agent.send_queued_now(queued);
        assert_eq!(agent.prompt_wire[1]["type"], "steer");
        assert!(agent.queue.items.is_empty());

        agent.handle(&approval_event("1", "first"));
        agent.submit("while asked", Vec::new(), true);
        assert_eq!(agent.prompt_wire.len(), 2, "an approval card holds new text back");
        assert_eq!(agent.queue.items.len(), 1);
        let id = agent.queue.items[0].id;
        agent.send_queued_now(id);
        assert_eq!(agent.queue.items.len(), 1, "no Send now while the card waits");

        let mut acp = queue_agent(false);
        acp.handle(&event(json!({"type": "agent_start"})));
        acp.submit("no steer", Vec::new(), true);
        assert!(acp.prompt_wire.is_empty(), "no steer: Ctrl+Return queues");
        assert_eq!(acp.queue.items.len(), 1);
    }

    #[test]
    fn stop_keeps_the_queue_paused_edit_and_new_chat_clear_it() {
        let mut agent = queue_agent(true);
        agent.handle(&event(json!({"type": "agent_start"})));
        agent.submit("a", Vec::new(), false);
        agent.submit("b", vec![png()], false);
        agent.stop();
        agent.handle(&event(json!({"type": "agent_end"})));
        assert!(agent.prompt_wire.is_empty(), "stop: the queue stays put");
        assert_eq!(agent.queue.items.len(), 2);
        let b = agent.queue.items[1].id;
        let edited = agent.take_queued(b).unwrap();
        assert_eq!((edited.text.as_str(), edited.images.len()), ("b", 1));
        agent.clear_transcript();
        assert!(agent.queue.items.is_empty(), "a new chat drops the queue");
    }

    #[test]
    fn images_alone_send_and_resume_or_clear_keep_them() {
        let mut agent = queue_agent(false);
        agent.submit("  ", vec![png()], false);
        assert_eq!(agent.prompt_wire.len(), 1, "an image without text is a message");
        assert_eq!(agent.prompt_wire[0]["images"].as_array().unwrap().len(), 1);
        agent.submit("/resume", vec![png()], false);
        assert_eq!(agent.draft_images.len(), 1, "/resume opens history and keeps the chips");
        agent.submit("/clear", vec![png(), png()], false);
        assert_eq!(agent.draft_images.len(), 2);
        assert_eq!(agent.prompt_wire.len(), 1, "/clear never reaches the backend");
    }

    #[test]
    fn capabilities_come_from_get_state_or_the_pi_model() {
        let state = |v: Value| AgentCapabilities::from_state(v.as_object().unwrap());
        assert_eq!(state(json!({"capabilities": {"steer": false, "images": true}})), AgentCapabilities { steer: false, images: true, ..Default::default() });
        assert_eq!(state(json!({"model": {"input": ["text"]}})), AgentCapabilities { steer: true, images: false, ..Default::default() });
        assert_eq!(state(json!({"model": {"input": ["text", "image"]}})), AgentCapabilities { steer: true, images: true, ..Default::default() });
        assert_eq!(state(json!({})), AgentCapabilities { steer: true, images: true, ..Default::default() });
        assert_eq!(
            PiRPCCommand::Steer { message: "x".into(), images: Vec::new() }.object(),
            json!({"type": "steer", "message": "x"})
        );
        let command = PiSlashCommand::from_value(&json!({"name": "compact", "argumentHint": "<instructions>", "source": "claude"})).unwrap();
        assert_eq!(command.argument_hint.as_deref(), Some("<instructions>"));
    }

    fn approval_event(id: &str, detail: &str) -> PiRPCEvent {
        let request = json!({
            "type": "approval_request",
            "id": id,
            "kind": "command",
            "title": "Run command",
            "detail": detail
        });
        PiRPCEvent::new(request.as_object().unwrap().clone()).unwrap()
    }

    #[test]
    fn stop_denies_a_pending_approval() {
        let mut agent = AgentCoordinator::new(&crate::settings::SettingsStore::new(Default::default()));
        let request = json!({"type": "approval_request", "id": "7", "title": "Edit"});
        agent.handle(&PiRPCEvent::new(request.as_object().unwrap().clone()).unwrap());
        agent.stop();
        assert!(agent.pending_approval.is_none());
    }

    #[test]
    fn switching_provider_mid_turn_returns_the_turn_to_idle_and_stop_stays_idle() {
        let mut agent =
            AgentCoordinator::new(&crate::settings::SettingsStore::new(Default::default()));
        let start = json!({"type": "agent_start"});
        agent.handle(&PiRPCEvent::new(start.as_object().unwrap().clone()).unwrap());
        agent.handle(&approval_event("9", "rm build"));
        assert!(agent.is_running, "a live turn shows the stop button");
        assert!(agent.pending_approval.is_some());

        agent.wants_connection = true;
        agent.set_agent_provider("claude");
        agent.restart();

        assert!(
            !agent.is_running,
            "switching provider mid-turn must restore Send without waiting for the old process"
        );
        assert!(
            agent.pending_approval.is_none(),
            "a pending approval from the retired session must be gone"
        );
        assert_eq!(agent.connection, Connection::Idle);

        agent.stop();
        assert!(
            !agent.is_running,
            "stop after the switch stays idle and does not wait for agent_end"
        );

        let start = json!({"type": "agent_start"});
        agent.handle(&PiRPCEvent::new(start.as_object().unwrap().clone()).unwrap());
        assert!(agent.is_running, "a new session can start a turn");
        let end = json!({"type": "agent_end"});
        agent.handle(&PiRPCEvent::new(end.as_object().unwrap().clone()).unwrap());
        assert!(!agent.is_running, "agent_end still ends the new turn");
    }

    #[test]
    fn switching_to_or_from_apple_fm_mid_turn_returns_the_turn_to_idle() {
        let mut agent =
            AgentCoordinator::new(&crate::settings::SettingsStore::new(Default::default()));
        for next in ["apple-fm", "pi", "claude"] {
            let start = json!({"type": "agent_start"});
            agent.handle(&PiRPCEvent::new(start.as_object().unwrap().clone()).unwrap());
            agent.handle(&approval_event("9", "write notes.tex"));
            assert!(agent.is_running, "a live turn shows the stop button");
            assert!(agent.pending_approval.is_some());

            agent.wants_connection = true;
            agent.set_agent_provider(next);
            agent.restart();

            assert!(
                !agent.is_running,
                "switching to {next} mid-turn must restore Send"
            );
            assert!(
                agent.pending_approval.is_none(),
                "a pending approval from the retired session must be gone"
            );
            agent.stop();
            assert!(!agent.is_running, "stop after the switch stays idle");
        }
    }

    #[test]
    fn extension_ui_request_stays_auto_cancelled() {
        let mut agent = AgentCoordinator::new(&crate::settings::SettingsStore::new(Default::default()));
        let request = json!({
            "type": "extension_ui_request",
            "id": "ui-1",
            "method": "confirm",
            "title": "Continue?"
        });
        agent.handle(&PiRPCEvent::new(request.as_object().unwrap().clone()).unwrap());
        assert!(agent.pending_approval.is_none());
        assert!(agent.transcript[0].text.contains("not supported"));
    }

    #[test]
    fn agent_provider_lists_pi_claude_and_agy_and_reads_hidden_cli_as_pi() {
        assert_eq!(
            crate::settings::AGENT_PROVIDER_CHOICES
                .iter()
                .map(|choice| choice.id)
                .collect::<Vec<_>>(),
            ["pi", "codex", "claude", "agy", "grok", "apple-fm"]
        );
        assert_eq!(
            crate::settings::visible_agent_providers()
                .map(|choice| choice.id)
                .collect::<Vec<_>>(),
            ["pi", "claude", "agy"]
        );
        assert_eq!(
            crate::settings::AGENT_PROVIDER_CHOICES
                .iter()
                .filter(|choice| choice.transport == crate::settings::AgentTransport::CliBridge)
                .map(|choice| choice.id)
                .collect::<Vec<_>>(),
            ["codex", "claude", "agy", "grok"]
        );
        let mut store = crate::settings::SettingsStore::new(Default::default());
        assert_eq!(store.agent_provider(), "pi");
        store.set_agent_provider("claude");
        assert_eq!(store.agent_provider(), "claude");
        assert_eq!(
            store.prefs().string("ai.agentProvider").as_deref(),
            Some("claude")
        );
        store.set_agent_provider("codex");
        assert_eq!(store.agent_provider(), "pi");
        assert_eq!(
            store.prefs().string("ai.agentProvider").as_deref(),
            Some("pi")
        );
        store.set_agent_provider("grok");
        assert_eq!(store.agent_provider(), "pi");
        assert_eq!(
            store.prefs().string("ai.agentProvider").as_deref(),
            Some("pi")
        );
        for raw in ["codex", "grok"] {
            store.prefs_mut().set("ai.agentProvider", raw);
            assert_eq!(store.agent_provider(), "pi");
            assert_eq!(
                store.prefs().string("ai.agentProvider").as_deref(),
                Some(raw)
            );
        }
        store.prefs_mut().set("ai.agentProvider", "apple-fm");
        assert_eq!(store.agent_provider(), "apple-fm");
        assert_eq!(
            store.prefs().string("ai.agentProvider").as_deref(),
            Some("apple-fm")
        );
        store.set_agent_provider("apple-fm");
        assert_eq!(store.agent_provider(), "apple-fm");
        assert_eq!(
            store.prefs().string("ai.agentProvider").as_deref(),
            Some("apple-fm")
        );
        store.set_agent_provider("agy");
        assert_eq!(store.agent_provider(), "agy");
        assert_eq!(
            store.prefs().string("ai.agentProvider").as_deref(),
            Some("agy")
        );
        store.set_agent_provider("pi");
        let mut agent = AgentCoordinator::new(&store);
        assert_eq!(agent.agent_provider, "pi");
        agent.set_agent_provider("codex");
        assert_eq!(agent.agent_provider, "pi");
        agent.set_agent_provider("grok");
        assert_eq!(agent.agent_provider, "pi");
        agent.set_agent_provider("claude");
        assert_eq!(agent.agent_provider, "claude");
        agent.set_agent_provider("agy");
        assert_eq!(agent.agent_provider, "agy");
        agent.set_agent_provider("apple-fm");
        assert_eq!(agent.agent_provider, "apple-fm");
        store.set_agent_provider("other");
        assert_eq!(store.agent_provider(), "pi");
        agent.set_agent_provider("other");
        assert_eq!(agent.agent_provider, "pi");
    }

    fn catalog_model(id: &str, provider: &str, name: &str) -> crate::catalog::CatalogModel {
        crate::catalog::CatalogModel {
            id: id.into(),
            provider: provider.into(),
            name: name.into(),
            context_window: None,
        }
    }

    fn response(value: serde_json::Value) -> PiRPCEvent {
        PiRPCEvent::new(value.as_object().unwrap().clone()).unwrap()
    }

    #[test]
    fn a_stale_get_messages_reply_does_not_replace_the_transcript() {
        let mut agent =
            AgentCoordinator::new(&crate::settings::SettingsStore::new(Default::default()));
        agent.messages_request_id = Some("current".into());
        agent.transcript = vec![AgentTranscriptEntry {
            role: TranscriptRole::User,
            title: "You".into(),
            text: "kept".into(),
            detail: String::new(),
            status: TranscriptStatus::Done,
            tool: None,
            images: Vec::new(),
            meta: EntryMeta::None,
        }];
        let stale = json!({
            "type": "response",
            "id": "previous",
            "command": "get_messages",
            "success": true,
            "data": {"messages": [{"role": "user", "content": "old"}]}
        });
        agent.handle(&response(stale));
        assert_eq!(agent.transcript[0].text, "kept");
        let current = json!({
            "type": "response",
            "id": "current",
            "command": "get_messages",
            "success": true,
            "data": {"messages": [{"role": "user", "content": "fresh"}]}
        });
        agent.handle(&response(current));
        assert_eq!(agent.transcript[0].text, "fresh");
    }

    fn todo(text: &str, status: TodoStatus) -> TodoItem {
        TodoItem { id: None, text: text.into(), status, active_form: None }
    }

    #[test]
    fn todo_snapshot_diffs_become_activity_rows() {
        use TodoStatus::*;
        let pending = [todo("Build the PDF", Pending), todo("Fix the citation", Pending), todo("Rebuild", Pending)];
        assert_eq!(todo_activities(&[], &pending), [TodoActivity::Created(3)]);
        assert!(todo_activities(&[], &[]).is_empty());
        let started = [todo("Build the PDF", InProgress), todo("Fix the citation", Pending), todo("Rebuild", Pending)];
        assert_eq!(todo_activities(&pending, &started), [TodoActivity::Started("Build the PDF".into())]);
        let two_done = [todo("Build the PDF", Completed), todo("Fix the citation", Completed), todo("Rebuild", InProgress)];
        assert_eq!(
            todo_activities(&started, &two_done),
            [
                TodoActivity::Completed("Build the PDF".into()),
                TodoActivity::Completed("Fix the citation".into()),
                TodoActivity::Started("Rebuild".into()),
            ]
        );
        let mut grown = two_done.to_vec();
        grown.push(todo("Check the log", Pending));
        assert_eq!(todo_activities(&two_done, &grown), [TodoActivity::Added("Check the log".into())]);
        // Back to pending is not news; the same list is no change.
        assert!(todo_activities(&started, &pending).is_empty());
        assert!(todo_activities(&two_done, &two_done).is_empty());
        // Rows with ids match by id even when they move.
        let a = TodoItem { id: Some("1".into()), ..todo("A", Pending) };
        let b = TodoItem { id: Some("2".into()), ..todo("B", Pending) };
        let moved = [TodoItem { status: Completed, ..b.clone() }, a.clone()];
        assert_eq!(todo_activities(&[a, b], &moved), [TodoActivity::Completed("B".into())]);
        // Rows without ids match by text, so dropping one does not shift the rest.
        let dropped = [todo("Fix the citation", Completed), todo("Rebuild", InProgress)];
        assert_eq!(
            todo_activities(&[todo("Build the PDF", Pending), todo("Fix the citation", InProgress), todo("Rebuild", Pending)], &dropped),
            [TodoActivity::Completed("Fix the citation".into()), TodoActivity::Started("Rebuild".into())]
        );
        // A list that shares no row with the old one is a new list, not one "Added" row per item.
        assert_eq!(todo_activities(&dropped, &[todo("Write the abstract", InProgress)]), [TodoActivity::Created(1)]);
        // Duplicate texts pair in order.
        let twice = [todo("Rebuild", Completed), todo("Rebuild", Pending)];
        assert_eq!(
            todo_activities(&[todo("Rebuild", Pending), todo("Rebuild", Pending)], &twice),
            [TodoActivity::Completed("Rebuild".into())]
        );
    }

    #[test]
    fn a_backend_switch_or_exit_clears_the_checklist() {
        let mut agent = AgentCoordinator::new(&crate::settings::SettingsStore::new(Default::default()));
        agent.handle(&event(json!({"type": "todo_update", "items": [{"text": "A", "status": "in_progress"}]})));
        assert_eq!(agent.todos.len(), 1);
        agent.restart();
        assert!(agent.todos.is_empty(), "a restart (backend switch) starts a fresh session");
        agent.handle(&event(json!({"type": "todo_update", "items": [{"text": "A", "status": "in_progress"}]})));
        agent.process_did_exit();
        assert!(agent.todos.is_empty());
    }

    #[test]
    fn todo_progress_counts_done_rows_and_names_the_current_step() {
        use TodoStatus::*;
        let items = [
            todo("Build the PDF", Completed),
            todo("Fix the citation", Completed),
            TodoItem { active_form: Some("Rebuilding".into()), ..todo("Rebuild", InProgress) },
            todo("Check the log", Pending),
            todo("Commit", Pending),
        ];
        assert_eq!(todo_progress(&items), (2, 5));
        assert_eq!(todo_current(&items), Some("Rebuilding"));
        assert_eq!(todo_current(&[todo("Fix", InProgress)]), Some("Fix"));
        assert_eq!(todo_current(&items[..2]), None);
        assert_eq!(todo_progress(&[]), (0, 0));
        let summary = crate::l10n::trn("en", "assistant.todo.summary", &["2", "5"]);
        assert_eq!(summary, "Tasks 2/5");
        assert_eq!(crate::l10n::trn("en", "assistant.todo.a11y", &["2", "5"]), "Task list, 2 of 5 done");
        assert_eq!(crate::l10n::trn("ko", "assistant.todo.a11y", &["2", "5"]), "작업 목록, 5개 중 2개 완료");
    }

    #[test]
    fn todo_update_events_drive_the_checklist_and_resume_restores_it() {
        let mut agent = AgentCoordinator::new(&crate::settings::SettingsStore::new(Default::default()));
        let list = |statuses: [&str; 3]| {
            json!({"type": "todo_update", "turnId": "t", "items": [
                {"text": "Build the PDF", "status": statuses[0], "activeForm": "Building the PDF"},
                {"text": "Fix the citation", "status": statuses[1]},
                {"text": "Rebuild", "status": statuses[2]},
                {"text": "  ", "status": "pending"}]})
        };
        agent.handle(&event(list(["pending", "pending", "pending"])));
        agent.handle(&event(list(["in_progress", "pending", "pending"])));
        agent.handle(&event(list(["in_progress", "pending", "pending"])));
        agent.handle(&event(list(["completed", "completed", "in_progress"])));
        assert_eq!(agent.todos.len(), 3, "a row without text is dropped");
        assert_eq!(todo_progress(&agent.todos), (2, 3));
        let rows: Vec<(String, String)> = agent
            .transcript
            .iter()
            .filter(|entry| entry.role == TranscriptRole::Activity)
            .map(|entry| (entry.title.clone(), entry.text.clone()))
            .collect();
        assert_eq!(
            rows,
            [
                ("assistant.todo.created".into(), "3".into()),
                ("assistant.todo.started".into(), "Build the PDF".into()),
                ("assistant.todo.completed".into(), "Build the PDF".into()),
                ("assistant.todo.completed".into(), "Fix the citation".into()),
                ("assistant.todo.started".into(), "Rebuild".into()),
            ]
        );
        assert_eq!(crate::l10n::trn("en", &rows[1].0, &[&rows[1].1]), "Started: Build the PDF");

        // A list built one row at a time stays one "created" row.
        agent.clear_transcript();
        assert!(agent.todos.is_empty(), "a new chat clears the checklist");
        agent.handle(&event(json!({"type": "todo_update", "items": [{"id": "1", "text": "A", "status": "pending"}]})));
        agent.handle(&event(json!({"type": "todo_update", "items": [{"id": "1", "text": "A", "status": "pending"}, {"id": "2", "text": "B", "status": "pending"}]})));
        assert_eq!(agent.transcript.len(), 1);
        assert_eq!(agent.transcript[0].text, "2");

        // Resume: get_messages carries the last list and adds no activity rows.
        agent.clear_transcript();
        agent.messages_request_id = Some("m".into());
        agent.handle(&response(json!({
            "type": "response", "id": "m", "command": "get_messages", "success": true,
            "data": {"messages": [{"role": "user", "content": "Fix it"}],
                     "todos": [{"text": "Build the PDF", "status": "completed"}, {"text": "Rebuild", "status": "in_progress"}]}
        })));
        assert_eq!(todo_progress(&agent.todos), (1, 2));
        assert!(agent.transcript.iter().all(|entry| entry.role != TranscriptRole::Activity));
        agent.handle(&response(json!({
            "type": "response", "id": "m", "command": "get_messages", "success": true, "data": {"messages": []}
        })));
        assert!(agent.todos.is_empty(), "a chat without a list shows none");
    }

    #[test]
    fn a_line_over_the_cap_is_dropped_and_the_next_line_is_kept() {
        let mut input = Vec::new();
        input.extend_from_slice(b"{\"ok\":1}\n");
        input.extend(std::iter::repeat(b'x').take(20));
        input.extend_from_slice(b"\n{\"ok\":2}\n");
        let mut reader = std::io::Cursor::new(input);
        assert_eq!(next_agent_stdout_line(&mut reader, 8).unwrap(), b"{\"ok\":1}");
        assert_eq!(next_agent_stdout_line(&mut reader, 8).unwrap(), b"{\"ok\":2}");
        assert!(next_agent_stdout_line(&mut reader, 8).is_none());
    }

    #[test]
    fn catalog_merges_ready_backends_and_omits_one_that_cannot_start() {
        let mut agent =
            AgentCoordinator::new(&crate::settings::SettingsStore::new(Default::default()));
        agent.note_catalog_report(
            "pi",
            Ok(vec![catalog_model("sonnet", "anthropic", "Sonnet")]),
        );
        agent.note_catalog_report("claude", Err(None));
        let menu = agent.catalog_menu("en");
        assert_eq!(menu.rows.len(), 1);
        assert_eq!(menu.rows[0].id.backend, "pi");
        assert_eq!(menu.rows[0].id.model_id, "sonnet");
        assert!(menu.rows[0].label.contains("pi (provider sign-in)"));
        assert!(menu.rows[0].label.contains("Sonnet (sonnet)"));
        assert!(menu.rows.iter().all(|row| row.id.backend != "claude"));
    }

    #[test]
    fn same_backend_catalog_pick_changes_only_the_model() {
        let mut store = crate::settings::SettingsStore::new(Default::default());
        let mut agent = AgentCoordinator::new(&store);
        let start = json!({"type": "agent_start"});
        agent.handle(&response(start));
        agent.note_catalog_report(
            "pi",
            Ok(vec![
                catalog_model("sonnet", "anthropic", "Sonnet"),
                catalog_model("opus", "anthropic", "Opus"),
            ]),
        );
        agent.current_model = Some(PiModelDescriptor {
            id: "sonnet".into(),
            provider: "anthropic".into(),
            name: "Sonnet".into(),
            context_window: None,
        });
        let opus = agent
            .catalog_menu("en")
            .rows
            .into_iter()
            .find(|row| row.id.model_id == "opus")
            .unwrap()
            .id;
        let generation = agent.catalog_generation();
        agent.apply_catalog(
            &crate::catalog::SelectionIntent::Row(opus),
            generation,
            &mut store,
            "Switched assistant. The next message starts a new session.",
        );
        assert_eq!(agent.agent_provider, "pi");
        assert_eq!(store.agent_provider(), "pi");
        assert!(
            agent.is_running,
            "a same-backend pick leaves the turn running"
        );
        assert_eq!(
            agent.model_requests,
            vec![("anthropic".into(), "opus".into())]
        );
        assert!(agent
            .transcript
            .iter()
            .all(|entry| !entry.text.contains("Switched")));
    }

    #[test]
    fn catalog_unchecking_the_live_backend_mid_turn_switches_and_returns_to_idle() {
        let mut store = crate::settings::SettingsStore::new(Default::default());
        let mut agent = AgentCoordinator::new(&store);
        agent.transcript.push(AgentTranscriptEntry {
            role: TranscriptRole::User,
            title: "You".into(),
            text: "draft the abstract".into(),
            detail: String::new(),
            status: TranscriptStatus::Done,
            tool: None,
            images: Vec::new(),
            meta: EntryMeta::None,
        });
        let start = json!({"type": "agent_start"});
        agent.handle(&response(start));
        agent.note_catalog_report("pi", Ok(vec![catalog_model("sonnet", "anthropic", "Sonnet")]));
        agent.note_catalog_report(
            "claude",
            Ok(vec![catalog_model("claude-sonnet", "claude", "Claude Sonnet")]),
        );
        assert!(agent.is_running);
        store.set_disabled_backend_ids(&["pi".into()]);
        agent.reconcile_enablement(
            &mut store,
            "Switched assistant. The next message starts a new session.",
        );
        assert!(!agent.is_running, "unchecking the live backend ends the turn");
        assert_eq!(agent.agent_provider, "claude");
        assert_eq!(store.agent_provider(), "claude");
        assert!(agent.model_requests.is_empty());
        assert!(agent
            .transcript
            .iter()
            .any(|entry| entry.text.contains("new session")));
        let menu = agent.catalog_menu("en");
        assert!(menu.rows.iter().all(|row| row.id.backend != "pi"));
    }

    #[test]
    fn catalog_unchecking_a_backend_that_is_not_the_session_leaves_the_turn_running() {
        let mut store = crate::settings::SettingsStore::new(Default::default());
        let mut agent = AgentCoordinator::new(&store);
        let start = json!({"type": "agent_start"});
        agent.handle(&response(start));
        agent.note_catalog_report("pi", Ok(vec![catalog_model("sonnet", "anthropic", "Sonnet")]));
        agent.note_catalog_report(
            "claude",
            Ok(vec![catalog_model("claude-sonnet", "claude", "Claude Sonnet")]),
        );
        store.set_disabled_backend_ids(&["claude".into()]);
        agent.reconcile_enablement(&mut store, "Switched assistant. The next message starts a new session.");
        assert!(agent.is_running);
        assert_eq!(agent.agent_provider, "pi");
        assert!(agent.catalog_menu("en").rows.iter().all(|row| row.id.backend != "claude"));
    }

    #[test]
    fn catalog_rechecking_the_stored_backend_after_a_full_stop_launches_it() {
        let mut store = crate::settings::SettingsStore::new(Default::default());
        let mut agent = AgentCoordinator::new(&store);
        agent.context_provider = Some(Box::new(|| AgentContextSnapshot {
            project_root: Some(std::env::temp_dir()),
            active_path: None,
            active_text: None,
            selection_text: None,
            project_files: Vec::new(),
            pdf_path: None,
            pdf_data: None,
        }));
        agent.handle(&response(json!({"type": "agent_start"})));
        store.set_disabled_backend_ids(&["pi".into(), "claude".into(), "agy".into()]);
        agent.reconcile_enablement(&mut store, "Switched assistant. The next message starts a new session.");
        assert!(!agent.is_running, "unchecking every backend ends the turn");
        assert_eq!(agent.agent_provider, "pi");
        assert!(agent.catalog_menu("en").rows.is_empty());
        agent.prepare();
        assert!(agent.toolchain_rx.is_none(), "an empty catalog does not spawn");
        store.set_disabled_backend_ids(&[]);
        agent.reconcile_enablement(&mut store, "Switched assistant. The next message starts a new session.");
        assert!(agent.toolchain_rx.is_some(), "rechecking the stored backend starts it");
        assert_eq!(agent.agent_provider, "pi");
    }

    #[test]
    fn cross_backend_catalog_pick_switches_backend_and_model_and_mid_turn_returns_to_idle() {
        let mut store = crate::settings::SettingsStore::new(Default::default());
        let mut agent = AgentCoordinator::new(&store);
        agent.transcript.push(AgentTranscriptEntry {
            role: TranscriptRole::User,
            title: "You".into(),
            text: "draft the abstract".into(),
            detail: String::new(),
            status: TranscriptStatus::Done,
            tool: None,
            images: Vec::new(),
            meta: EntryMeta::None,
        });
        let start = json!({"type": "agent_start"});
        agent.handle(&response(start));
        agent.handle(&approval_event("9", "rm build"));
        agent.note_catalog_report(
            "pi",
            Ok(vec![catalog_model("sonnet", "anthropic", "Sonnet")]),
        );
        agent.note_catalog_report(
            "claude",
            Ok(vec![catalog_model(
                "claude-sonnet",
                "claude",
                "Claude Sonnet",
            )]),
        );
        agent.current_model = Some(PiModelDescriptor {
            id: "sonnet".into(),
            provider: "anthropic".into(),
            name: "Sonnet".into(),
            context_window: None,
        });
        assert!(agent.is_running);
        let claude = agent
            .catalog_menu("en")
            .rows
            .into_iter()
            .find(|row| row.id.backend == "claude")
            .unwrap()
            .id;
        agent.wants_connection = true;
        let generation = agent.catalog_generation();
        agent.apply_catalog(
            &crate::catalog::SelectionIntent::Row(claude),
            generation,
            &mut store,
            "Switched assistant. The next message starts a new session.",
        );
        assert_eq!(agent.agent_provider, "claude");
        assert_eq!(store.agent_provider(), "claude");
        assert!(
            !agent.is_running,
            "a cross-backend pick mid-turn must restore Send before the new session starts"
        );
        assert!(agent.pending_approval.is_none());
        assert!(
            agent.model_requests.is_empty(),
            "the retired session must not receive set_model"
        );
        assert!(agent
            .transcript
            .iter()
            .any(|entry| entry.text.contains("new session")));
        assert_eq!(
            agent
                .switch_ticket
                .as_ref()
                .map(|ticket| ticket.backend.as_str()),
            Some("claude")
        );
        assert!(
            agent.catalog_menu("en").accepts_picks,
            "the model menu stays openable while the switch waits for the new session's model list"
        );

        let models = json!({
            "type": "response",
            "command": "get_available_models",
            "success": true,
            "data": {"models": [{"id": "claude-sonnet", "provider": "claude", "name": "Claude Sonnet"}]}
        });
        agent.handle(&response(models));
        assert!(agent.switch_ticket.is_none());
        assert_eq!(
            agent.model_requests,
            vec![("claude".into(), "claude-sonnet".into())]
        );
        agent.stop();
        assert!(!agent.is_running, "stop after the switch stays idle");
    }

    #[test]
    fn a_second_pick_during_a_switch_replaces_the_ticket_model() {
        let mut store = crate::settings::SettingsStore::new(Default::default());
        let mut agent = AgentCoordinator::new(&store);
        agent.note_catalog_report("pi", Ok(vec![catalog_model("sonnet", "anthropic", "Sonnet")]));
        agent.note_catalog_report(
            "claude",
            Ok(vec![
                catalog_model("claude-haiku", "claude", "Haiku"),
                catalog_model("claude-sonnet", "claude", "Sonnet"),
            ]),
        );
        let haiku = agent
            .catalog_menu("en")
            .rows
            .iter()
            .find(|row| row.id.model_id == "claude-haiku")
            .unwrap()
            .id
            .clone();
        let generation = agent.catalog_generation();
        agent.apply_catalog(
            &crate::catalog::SelectionIntent::Row(haiku),
            generation,
            &mut store,
            "switched",
        );
        let sonnet = agent
            .catalog_menu("en")
            .rows
            .iter()
            .find(|row| row.id.model_id == "claude-sonnet")
            .unwrap()
            .id
            .clone();
        let generation = agent.catalog_generation();
        agent.apply_catalog(
            &crate::catalog::SelectionIntent::Row(sonnet),
            generation,
            &mut store,
            "switched",
        );
        assert!(agent.model_requests.is_empty());
        assert_eq!(
            agent
                .switch_ticket
                .as_ref()
                .and_then(|ticket| ticket.model.as_ref())
                .map(|model| model.id.as_str()),
            Some("claude-sonnet")
        );
        let models = json!({
            "type": "response",
            "command": "get_available_models",
            "success": true,
            "data": {"models": [
                {"id": "claude-haiku", "provider": "claude", "name": "Haiku"},
                {"id": "claude-sonnet", "provider": "claude", "name": "Sonnet"}
            ]}
        });
        agent.handle(&response(models));
        assert!(agent.switch_ticket.is_none());
        assert_eq!(
            agent.model_requests,
            vec![("claude".into(), "claude-sonnet".into())]
        );
    }

    #[test]
    fn cross_backend_catalog_pick_keeps_the_ticket_until_the_model_list_arrives() {
        let mut store = crate::settings::SettingsStore::new(Default::default());
        let mut agent = AgentCoordinator::new(&store);
        agent.note_catalog_report("pi", Ok(vec![catalog_model("sonnet", "anthropic", "Sonnet")]));
        agent.note_catalog_report(
            "claude",
            Ok(vec![catalog_model("claude-sonnet", "claude", "Claude Sonnet")]),
        );
        let claude = agent
            .catalog_menu("en")
            .rows
            .into_iter()
            .find(|row| row.id.backend == "claude")
            .unwrap()
            .id;
        let generation = agent.catalog_generation();
        agent.apply_catalog(
            &crate::catalog::SelectionIntent::Row(claude),
            generation,
            &mut store,
            "Switched assistant. The next message starts a new session.",
        );
        agent.model_settings_request_id = Some("levels".into());
        agent.is_updating_model_settings = true;
        let levels = json!({
            "type": "response",
            "id": "levels",
            "command": "get_available_thinking_levels",
            "success": true,
            "data": {"levels": ["off"]}
        });
        agent.handle(&response(levels));
        assert!(agent.switch_ticket.is_some());
        assert!(agent.model_requests.is_empty());
        let models = json!({
            "type": "response",
            "command": "get_available_models",
            "success": true,
            "data": {"models": [{"id": "claude-sonnet", "provider": "claude", "name": "Claude Sonnet"}]}
        });
        agent.handle(&response(models));
        assert!(agent.switch_ticket.is_none());
        assert_eq!(
            agent.model_requests,
            vec![("claude".into(), "claude-sonnet".into())]
        );
    }

    #[test]
    fn cross_backend_catalog_pick_drops_a_model_the_new_session_does_not_list() {
        let mut store = crate::settings::SettingsStore::new(Default::default());
        let mut agent = AgentCoordinator::new(&store);
        agent.note_catalog_report("pi", Ok(vec![catalog_model("sonnet", "anthropic", "Sonnet")]));
        agent.note_catalog_report(
            "claude",
            Ok(vec![catalog_model("claude-sonnet", "claude", "Claude Sonnet")]),
        );
        let claude = agent
            .catalog_menu("en")
            .rows
            .into_iter()
            .find(|row| row.id.backend == "claude")
            .unwrap()
            .id;
        let generation = agent.catalog_generation();
        agent.apply_catalog(
            &crate::catalog::SelectionIntent::Row(claude),
            generation,
            &mut store,
            "Switched assistant. The next message starts a new session.",
        );
        agent.current_model = Some(PiModelDescriptor {
            id: "claude-sonnet".into(),
            provider: "claude".into(),
            name: "Claude Sonnet".into(),
            context_window: None,
        });
        let models = json!({
            "type": "response",
            "command": "get_available_models",
            "success": true,
            "data": {"models": [{"id": "claude-opus", "provider": "claude", "name": "Claude Opus"}]}
        });
        agent.handle(&response(models));
        assert!(agent.switch_ticket.is_none());
        assert!(agent.current_model.is_none());
        assert!(agent.model_requests.is_empty());
    }

    #[test]
    fn cross_backend_catalog_pick_waits_for_idle_model_settings_before_set_model() {
        let mut store = crate::settings::SettingsStore::new(Default::default());
        let mut agent = AgentCoordinator::new(&store);
        agent.note_catalog_report(
            "pi",
            Ok(vec![catalog_model("sonnet", "anthropic", "Sonnet")]),
        );
        agent.note_catalog_report(
            "claude",
            Ok(vec![catalog_model(
                "claude-sonnet",
                "claude",
                "Claude Sonnet",
            )]),
        );
        agent.current_model = Some(PiModelDescriptor {
            id: "sonnet".into(),
            provider: "anthropic".into(),
            name: "Sonnet".into(),
            context_window: None,
        });
        let claude = agent
            .catalog_menu("en")
            .rows
            .into_iter()
            .find(|row| row.id.backend == "claude")
            .unwrap()
            .id;
        let generation = agent.catalog_generation();
        agent.apply_catalog(
            &crate::catalog::SelectionIntent::Row(claude),
            generation,
            &mut store,
            "Switched assistant. The next message starts a new session.",
        );
        agent.is_updating_model_settings = true;
        let models = json!({
            "type": "response",
            "command": "get_available_models",
            "success": true,
            "data": {"models": [{"id": "claude-sonnet", "provider": "claude", "name": "Claude Sonnet"}]}
        });
        agent.handle(&response(models));
        assert!(
            agent.switch_ticket.is_some(),
            "the ticket stays until model settings are idle"
        );
        assert!(agent.model_requests.is_empty());
        agent.model_settings_request_id = Some("levels".into());
        let levels = json!({
            "type": "response",
            "id": "levels",
            "command": "get_available_thinking_levels",
            "success": true,
            "data": {"levels": ["off"]}
        });
        agent.handle(&response(levels));
        assert!(agent.switch_ticket.is_none());
        assert_eq!(
            agent.model_requests,
            vec![("claude".into(), "claude-sonnet".into())]
        );
    }

    /// Runs a just-written script through /bin/sh: exec'ing the script itself can fail
    /// with ETXTBSY while another test thread's fork still holds its write descriptor.
    #[cfg(unix)]
    fn start_script(script: &Path, dir: &Path) -> PiAgentProcess {
        let args = [script.to_string_lossy().into_owned()];
        PiAgentProcess::start(Path::new("/bin/sh"), dir, &args, &std::collections::HashMap::new()).unwrap()
    }

    /// Runs `lines` (JSONL) as a "claude" probe and returns the catalog notes it leaves.
    #[cfg(unix)]
    fn notes_after_probe(agent: &mut AgentCoordinator, lines: &[&str]) -> Vec<String> {
        let dir = std::env::temp_dir().join(format!("pitex-catalog-probe-{}", uuid_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let script = dir.join("probe.sh");
        let body: String = lines.iter().map(|line| format!("printf '%s\\n' '{line}'\n")).collect();
        std::fs::write(&script, body).unwrap();
        let process = start_script(&script, &dir);
        agent.probes.push(CatalogProbe {
            backend: "claude".into(),
            generation: agent.probe_generation,
            process,
            started: std::time::Instant::now(),
            sessions: dir.clone(),
            error: None,
        });
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        while !agent.probes.is_empty() && std::time::Instant::now() < deadline {
            agent.poll_catalog_probes();
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        assert!(agent.probes.is_empty(), "the probe did not finish");
        agent.catalog_notes("en")
    }

    #[cfg(unix)]
    #[test]
    fn a_failed_probe_keeps_its_reason_until_the_backend_lists_models() {
        let mut agent =
            AgentCoordinator::new(&crate::settings::SettingsStore::new(Default::default()));
        assert!(agent.catalog_notes("en").is_empty());
        let generic = format!(
            "{}: {}",
            agent.backend_title("en", "claude"),
            crate::l10n::tr("en", "assistant.backend_unavailable")
        );
        // The response has no error text, so the earlier extension_error is the reason.
        let notes = notes_after_probe(
            &mut agent,
            &[
                r#"{"type":"extension_error","error":"Claude Code was not found."}"#,
                r#"{"type":"response","command":"get_available_models","success":false}"#,
            ],
        );
        assert_eq!(notes.len(), 1, "{notes:?}");
        assert!(notes[0].starts_with("Claude Code"), "{notes:?}");
        assert!(notes[0].ends_with(": Claude Code was not found."), "{notes:?}");

        // The reply's own error beats an earlier extension_error.
        let notes = notes_after_probe(
            &mut agent,
            &[
                r#"{"type":"extension_error","error":"first"}"#,
                r#"{"type":"response","command":"get_available_models","success":false,"error":"X"}"#,
            ],
        );
        assert!(notes.len() == 1 && notes[0].ends_with(": X"), "{notes:?}");

        // A notice before an empty but successful reply is not the reason.
        let notes = notes_after_probe(
            &mut agent,
            &[
                r#"{"type":"extension_error","error":"asks before it runs a command"}"#,
                r#"{"type":"response","command":"get_available_models","success":true,"data":{"models":[]}}"#,
            ],
        );
        assert_eq!(notes, vec![generic.clone()]);

        // No bridge text at all falls back to the generic localized reason.
        agent.note_catalog_report("claude", Err(None));
        assert_eq!(agent.catalog_notes("en"), vec![generic]);
        // Re-arming for another check keeps the last reason on screen.
        crate::catalog::rearm_unavailable(&mut agent.catalog_slots);
        assert_eq!(agent.catalog_notes("en").len(), 1);

        agent.note_catalog_report(
            "claude",
            Ok(vec![catalog_model("claude-sonnet", "claude", "Claude Sonnet")]),
        );
        assert!(agent.catalog_notes("en").is_empty());
        assert!(!agent.catalog_reasons.contains_key("claude"));
    }

    #[test]
    fn the_active_sessions_failed_model_list_shows_its_reason() {
        let mut agent =
            AgentCoordinator::new(&crate::settings::SettingsStore::new(Default::default()));
        let title = agent.backend_title("en", &agent.agent_provider.clone());
        for (error, reason) in [
            (Some("Claude Code was not found."), "Claude Code was not found.".to_string()),
            (None, crate::l10n::tr("en", "assistant.backend_unavailable")),
        ] {
            agent.model_settings_request_id = Some("models".into());
            let mut reply = serde_json::json!({
                "type": "response", "id": "models",
                "command": "get_available_models", "success": false,
            });
            if let Some(error) = error {
                reply["error"] = error.into();
            }
            agent.handle(&PiRPCEvent::new(reply.as_object().unwrap().clone()).unwrap());
            assert_eq!(agent.catalog_notes("en"), vec![format!("{title}: {reason}")]);
        }
    }

    #[cfg(unix)]
    #[test]
    fn a_short_lived_probe_merges_the_other_backends_models() {
        let dir = std::env::temp_dir().join(format!("pitex-catalog-probe-{}", uuid_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let script = dir.join("reply.sh");
        std::fs::write(
            &script,
            "printf '%s\\n' '{\"type\":\"response\",\"command\":\"get_available_models\",\"success\":true,\"data\":{\"models\":[{\"id\":\"claude-sonnet\",\"provider\":\"claude\",\"name\":\"Claude Sonnet\"}]}}'\n",
        )
        .unwrap();
        let mut agent =
            AgentCoordinator::new(&crate::settings::SettingsStore::new(Default::default()));
        agent.note_catalog_report(
            "pi",
            Ok(vec![catalog_model("sonnet", "anthropic", "Sonnet")]),
        );
        let process = start_script(&script, &dir);
        agent.probes.push(CatalogProbe {
            backend: "claude".into(),
            generation: agent.probe_generation,
            process,
            started: std::time::Instant::now(),
            sessions: dir.clone(),
            error: None,
        });
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        while agent.catalog_menu("en").rows.len() < 2 && std::time::Instant::now() < deadline {
            agent.poll_catalog_probes();
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        let menu = agent.catalog_menu("en");
        assert!(
            menu.rows
                .iter()
                .any(|row| row.id.backend == "claude" && row.id.model_id == "claude-sonnet"),
            "probe models should join the live pi row, got {menu:?}"
        );
        assert!(agent.probes.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }


    #[test]
    fn pinned_pi_version_matches_swift_notice_and_flag() {
        let version = pi_installer::DESIRED_VERSION;
        let swift = include_str!("../../../../Mac/Sources/Features/PiAgentProcess.swift");
        assert!(
            swift.contains(&format!("static let desiredVersion = \"{version}\"")),
            "PiAgentProcess.desiredVersion"
        );
        let settings = include_str!("../../../../Mac/Sources/Features/SettingsView.swift");
        let flag = if crate::settings::SHOW_DIRECT_CODEX_AND_GROK {
            "true"
        } else {
            "false"
        };
        assert!(settings.contains(&format!("static let showDirectCodexAndGrok = {flag}")));
        assert!(settings.contains("ForEach(ShippedAgentProvider.selectableCases"));
        let notice: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../Mac/Resources/PitexAgent/NOTICE-PROVENANCE.json"
        ))
        .expect("NOTICE-PROVENANCE.json");
        let component = notice["components"]
            .as_array()
            .and_then(|items| {
                items
                    .iter()
                    .find(|item| item["name"].as_str() == Some("@earendil-works/pi-coding-agent"))
            })
            .expect("pi component");
        assert_eq!(component["version"].as_str(), Some(version));
        let revision = format!("v{version}");
        assert_eq!(component["revision"].as_str(), Some(revision.as_str()));
        let third_party =
            include_str!("../../../../Mac/Resources/PitexAgent/THIRD-PARTY-NOTICES.md");
        assert!(third_party.contains(&format!("## Pi coding agent {version}")));
        assert!(third_party.contains(&format!(
            "https://github.com/earendil-works/pi/tree/v{version}"
        )));
    }
}

#[cfg(test)]
mod branch_tests {
    use super::*;

    fn event(value: Value) -> PiRPCEvent {
        PiRPCEvent::new(value.as_object().unwrap().clone()).unwrap()
    }

    fn coordinator() -> AgentCoordinator {
        AgentCoordinator::new(&crate::settings::SettingsStore::new(Default::default()))
    }

    #[test]
    fn fork_and_rewind_commands_and_capabilities_go_over_the_wire() {
        assert_eq!(
            PiRPCCommand::ForkSession { keep_turns: 2, turns: 3 }.object(),
            json!({"type": "fork_session", "keepTurns": 2, "turns": 3})
        );
        assert_eq!(
            PiRPCCommand::Rewind { keep_turns: 1, turns: 2, conversation: false, files: true, dry_run: true }.object(),
            json!({"type": "rewind", "keepTurns": 1, "turns": 2, "conversation": false, "files": true, "dryRun": true})
        );
        assert_eq!(
            PiRPCCommand::Rewind { keep_turns: 0, turns: 1, conversation: true, files: false, dry_run: false }.object(),
            json!({"type": "rewind", "keepTurns": 0, "turns": 1, "conversation": true, "files": false})
        );
        let claude = json!({"capabilities": {"steer": false, "images": true, "fork": true, "rewind": {"conversation": true, "files": true}}});
        let caps = AgentCapabilities::from_state(claude.as_object().unwrap());
        assert!(caps.fork && caps.rewind_conversation && caps.rewind_files);
        let acp = json!({"capabilities": {"steer": false, "images": true}});
        let caps = AgentCapabilities::from_state(acp.as_object().unwrap());
        assert!(!caps.fork && !caps.rewind_conversation && !caps.rewind_files);
        let pi = json!({"model": {"input": ["text"]}});
        assert!(!AgentCapabilities::from_state(pi.as_object().unwrap()).fork, "pi forks by quoting");
    }

    #[test]
    fn subagent_calls_keep_their_parent_live_and_after_a_reload() {
        let mut agent = coordinator();
        agent.handle(&event(json!({"type": "tool_execution_start", "toolCallId": "a", "toolName": "Agent", "args": {},
            "detail": {"kind": "sub_agent", "label": "Find", "description": "Find", "subAgentType": "Explore"}})));
        agent.handle(&event(json!({"type": "tool_execution_start", "toolCallId": "r", "toolName": "Read", "args": {},
            "detail": {"kind": "read", "path": "/p/x"}, "parentToolCallId": "a"})));
        agent.handle(&event(json!({"type": "tool_execution_update", "toolCallId": "a", "detail": {"kind": "sub_agent", "toolUses": 1}})));
        let card = agent.transcript[0].tool.clone().unwrap();
        assert_eq!((card.sub_agent_type.as_deref(), card.tool_uses, card.badge("en").as_deref()), (Some("Explore"), Some(1), Some("Explore")));
        assert_eq!(agent.transcript[1].tool.as_ref().unwrap().parent_call_id.as_deref(), Some("a"));
        let rows = agent.transcript_from_messages(&[
            json!({"role": "assistant", "content": [{"type": "toolCall", "id": "r", "name": "Read", "arguments": {}, "parentToolCallId": "a"}]}),
            json!({"role": "toolResult", "toolCallId": "r", "detail": {"kind": "read"}, "status": "completed", "parentToolCallId": "a", "content": []}),
        ]);
        assert_eq!(rows[0].tool.as_ref().unwrap().parent_call_id.as_deref(), Some("a"));
    }

    fn two_turns(agent: &mut AgentCoordinator) {
        for (prompt, answer) in [("First", "One"), ("Second", "Two")] {
            agent.push_user_entry(prompt, Vec::new());
            agent.handle(&event(json!({"type": "agent_start"})));
            agent.handle(&event(json!({"type": "message_start", "message": {"role": "assistant"}})));
            agent.handle(&event(json!({"type": "message_update", "assistantMessageEvent": {"type": "text_delta", "contentIndex": 0, "delta": answer}})));
            agent.handle(&event(json!({"type": "agent_end", "outcome": "completed"})));
        }
    }

    #[test]
    fn turn_cuts_count_prompts_and_prompts_learn_how_long_their_turn_ran() {
        let mut agent = coordinator();
        two_turns(&mut agent);
        assert!(matches!(agent.transcript[0].meta, EntryMeta::Worked { .. }));
        assert_eq!(agent.turn_cut(0), Some((0, Some("First".into()))));
        assert_eq!(agent.turn_cut(1), Some((1, None)), "an answer keeps its own turn");
        assert_eq!(agent.turn_cut(2), Some((1, Some("Second".into()))));
        assert_eq!(agent.turn_cut(3), Some((2, None)));
        assert_eq!(agent.prompt_count(), 2);
        // Rows trimmed off the top still count.
        agent.dropped_prompts = 3;
        assert_eq!(agent.turn_cut(2), Some((4, Some("Second".into()))));
        assert_eq!(quote_turns(&agent.transcript, 1, "en"), "Continue from this earlier conversation:\n\n> **You:**\n> First\n>\n> **Assistant:**\n> One");
    }

    #[test]
    fn a_rewind_lists_files_then_reloads_the_chat_and_puts_the_prompt_back() {
        let mut agent = coordinator();
        two_turns(&mut agent);
        agent.rewind_offer = Some(RewindOffer { keep_turns: 1, draft: Some("Second".into()), request_id: Some("dry".into()), ..Default::default() });
        agent.handle(&event(json!({"type": "response", "command": "rewind", "id": "other", "success": true, "data": {"filesChanged": ["x"]}})));
        assert_eq!(agent.rewind_offer.as_ref().unwrap().files, None, "another request's reply is ignored");
        agent.handle(&event(json!({"type": "response", "command": "rewind", "id": "dry", "success": true, "data": {"filesChanged": ["/p/a.tex"], "dryRun": true}})));
        assert_eq!(agent.rewind_offer.as_ref().unwrap().files, Some(vec!["/p/a.tex".to_string()]));
        agent.rewind_offer.as_mut().unwrap().request_id = Some("go".into());
        // The confirmed rewind is out: it cannot be cancelled, and nothing else goes to the backend.
        assert!(agent.is_branching());
        agent.cancel_rewind();
        agent.new_session();
        agent.submit("meanwhile", Vec::new(), false);
        assert!(agent.rewind_offer.is_some(), "its reply must still reload the chat");
        assert_eq!(agent.transcript.len(), 4, "no new chat and no message while it runs");
        agent.handle(&event(json!({"type": "response", "command": "rewind", "id": "go", "success": true, "data": {"filesChanged": ["/p/a.tex"], "conversation": true}})));
        assert!(agent.rewind_offer.is_none());
        assert_eq!(agent.composer_replacement.as_deref(), Some("Second"));
        assert!(agent.transcript.is_empty(), "the chat reloads from the backend");
        agent.messages_request_id = Some("m".into());
        agent.handle(&event(json!({"type": "response", "command": "get_messages", "id": "m", "success": true,
            "data": {"messages": [{"role": "user", "content": [{"type": "text", "text": "First"}]}]}})));
        assert_eq!(agent.transcript.last().unwrap().text, "Rewound the conversation and restored /p/a.tex.", "the notice follows the reloaded chat");

        // A refused dry run leaves only the conversation choice, with the reason.
        agent.rewind_offer = Some(RewindOffer { keep_turns: 0, request_id: Some("dry2".into()), ..Default::default() });
        agent.handle(&event(json!({"type": "response", "command": "rewind", "id": "dry2", "success": false, "error": "outside the project"})));
        let offer = agent.rewind_offer.clone().unwrap();
        assert_eq!(offer.files, Some(Vec::new()));
        assert_eq!(offer.files_note.as_deref(), Some("Files cannot be restored: outside the project"));
    }

    #[test]
    fn a_fork_reply_clears_the_chat_and_says_so() {
        let mut agent = coordinator();
        two_turns(&mut agent);
        agent.fork_request = Some(("f".into(), Some("Second".into())));
        agent.handle(&event(json!({"type": "response", "command": "fork_session", "id": "f", "success": true, "data": {"sessionPath": "/s/claude-x.jsonl"}})));
        assert!(agent.transcript.is_empty());
        assert!(agent.notice_after_reload.as_deref().is_some_and(|n| n.starts_with("Forked into a new chat.")));
        assert_eq!(agent.composer_replacement.as_deref(), Some("Second"));
        agent.fork_request = Some(("g".into(), None));
        agent.handle(&event(json!({"type": "response", "command": "fork_session", "id": "g", "success": false, "error": "boom"})));
        assert_eq!(agent.transcript.last().unwrap().text, "Could not fork the chat: boom");
        assert_eq!(agent.transcript.last().unwrap().status, TranscriptStatus::Failed);
    }
}
