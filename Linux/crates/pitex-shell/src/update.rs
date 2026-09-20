//! Self-update — check GitHub Releases for a newer tag, download the
//! platform asset, and hand it to the platform installer.
//!
//! `curl` does the HTTP work on every platform (it ships with
//! ubuntu-standard, macOS, and Windows 10+), so the feature needs no extra
//! crate and follows the same process-run model as the rest of the app.
//! Everything here is blocking — callers always run it on a worker thread,
//! never the GTK main loop.

use std::path::{Path, PathBuf};
use std::process::Command;

const RELEASE_API: &str =
    "https://api.github.com/repos/jaehwan-2ee/pitex/releases/latest";
const USER_AGENT: &str = "pitex-update";

/// One release entry reduced to what the updater needs.
#[derive(Clone, Debug)]
pub struct UpdateInfo {
    /// Tag as published (`v1.0.3`).
    pub tag: String,
    /// Chosen asset's file name (`Pitex-v1.0.3-windows-amd64.zip`).
    pub asset_name: String,
    /// Direct download URL for the chosen asset.
    pub asset_url: String,
}

/// What `install` decided to do — the UI maps each variant to a toast.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InstallOutcome {
    /// Package installed — a restart finishes the update.
    AwaitingRestart,
    /// The updater script was launched; the caller should exit so it can
    /// swap the binaries.
    ExitRequested,
    /// An interactive terminal was opened with the install command.
    HandedToTerminal,
    /// Automatic install was not possible; the downloaded file was opened
    /// for a manual install.
    ManualFallback,
}

fn curl(arguments: &[&str], timeout_secs: u64) -> Result<Vec<u8>, String> {
    let output = Command::new("curl")
        .arg("--max-time")
        .arg(timeout_secs.to_string())
        .args(arguments)
        .stdin(std::process::Stdio::null())
        .output()
        .map_err(|e| format!("curl could not be started: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "Download failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(output.stdout)
}

/// `GET` the latest release and pick this platform's asset. Runs the same
/// `releases/latest` lookup on every OS; only the asset filter differs.
fn latest_release() -> Result<UpdateInfo, String> {
    let body = curl(
        &[
            "-fsSL",
            "-H",
            "Accept: application/vnd.github+json",
            "-H",
            &format!("User-Agent: {USER_AGENT}"),
            RELEASE_API,
        ],
        20,
    )?;
    let json: serde_json::Value =
        serde_json::from_slice(&body).map_err(|e| format!("Bad release response: {e}"))?;
    let tag = json
        .get("tag_name")
        .and_then(|t| t.as_str())
        .ok_or_else(|| "The latest release has no tag.".to_string())?;
    // Preferred asset first — e.g. Windows favors the setup.exe installer
    // and only falls back to the portable zip if a release lacks one.
    let asset = asset_suffixes()
        .iter()
        .find_map(|suffix| {
            json.get("assets")
                .and_then(|a| a.as_array())
                .into_iter()
                .flatten()
                .find_map(|asset| {
                    let name = asset.get("name")?.as_str()?;
                    if !name.ends_with(suffix) {
                        return None;
                    }
                    let url = asset.get("browser_download_url")?.as_str()?;
                    Some((name.to_string(), url.to_string()))
                })
        })
        .ok_or_else(|| format!("No suitable asset in release {tag}."))?;
    Ok(UpdateInfo {
        tag: tag.to_string(),
        asset_name: asset.0,
        asset_url: asset.1,
    })
}

/// Asset name tails this platform's packages use, most preferred first.
#[cfg(target_os = "macos")]
fn asset_suffixes() -> &'static [&'static str] {
    &["macos-arm64.dmg"]
}
/// Windows prefers the per-user NSIS installer; the portable zip remains
/// as the fallback for releases that predate it.
#[cfg(windows)]
fn asset_suffixes() -> &'static [&'static str] {
    &["windows-amd64-setup.exe", "windows-amd64.zip"]
}
/// Linux picks the deb matching the distro: the 22.04 compat package for
/// Ubuntu 22.04 (no VTE-GTK4 there), the full package elsewhere.
#[cfg(all(unix, not(target_os = "macos")))]
fn asset_suffixes() -> &'static [&'static str] {
    if is_ubuntu_2204() {
        &["ubuntu22.04-amd64.deb"]
    } else {
        &["ubuntu24.04-amd64.deb"]
    }
}

#[cfg(all(unix, not(target_os = "macos")))]
fn is_ubuntu_2204() -> bool {
    let Ok(text) = std::fs::read_to_string("/etc/os-release") else {
        return false;
    };
    let id = text
        .lines()
        .find_map(|l| l.strip_prefix("ID="))
        .map(|v| v.trim_matches('"'))
        .unwrap_or_default();
    let version = text
        .lines()
        .find_map(|l| l.strip_prefix("VERSION_ID="))
        .map(|v| v.trim_matches('"'))
        .unwrap_or_default();
    id == "ubuntu" && version.starts_with("22.04")
}

/// `v1.0.3` vs `1.0.2` — numeric component compare, suffix-free tags only.
fn version_newer(tag: &str, current: &str) -> bool {
    fn parts(s: &str) -> Vec<u64> {
        s.trim_start_matches('v')
            .split('.')
            .map(|p| p.chars().take_while(|c| c.is_ascii_digit()).collect::<String>().parse().unwrap_or(0))
            .collect()
    }
    let (new, old) = (parts(tag), parts(current));
    for i in 0..new.len().max(old.len()) {
        let (n, o) = (new.get(i).copied().unwrap_or(0), old.get(i).copied().unwrap_or(0));
        if n != o {
            return n > o;
        }
    }
    false
}

/// Check whether the latest release is newer than `current` (e.g. `1.0.2`).
pub fn check_for_update(current: &str) -> Result<Option<UpdateInfo>, String> {
    let info = latest_release()?;
    if version_newer(&info.tag, current) {
        Ok(Some(info))
    } else {
        Ok(None)
    }
}

/// Download the asset to the app's cache directory. The file name is kept
/// so installers see the versioned name.
pub fn download(info: &UpdateInfo) -> Result<PathBuf, String> {
    let dir = download_directory()?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join(&info.asset_name);
    curl(
        &["-fSL", "-o", &path.to_string_lossy(), &info.asset_url],
        600,
    )?;
    Ok(path)
}

fn download_directory() -> Result<PathBuf, String> {
    dirs::cache_dir()
        .map(|d| d.join("dev.pitex.app").join("updates"))
        .ok_or_else(|| "Could not resolve the cache directory.".to_string())
}

/// Install a downloaded asset — per-platform mechanics.
pub fn install(downloaded: &Path, info: &UpdateInfo) -> Result<InstallOutcome, String> {
    install_impl(downloaded, info)
}

/// Debian package: `pkexec apt install` shows the polkit prompt and swaps
/// the binary in place; the running app keeps its mapped pages so a manual
/// restart finishes the update. Without polkit, the same command goes to an
/// interactive terminal (`sudo`).
#[cfg(all(unix, not(target_os = "macos")))]
fn install_impl(downloaded: &Path, info: &UpdateInfo) -> Result<InstallOutcome, String> {
    // apt's `_apt` sandbox user can't read files under $HOME (0700 on
    // Ubuntu 24.04), which makes `apt install ./file.deb` warn "Download is
    // performed unsandboxed as root" — and fail outright on some setups.
    // Stage the package in /tmp, world-readable, before elevating.
    let staging = std::env::temp_dir().join(format!("pitex-update-{}", crate::model::uuid_v4()));
    std::fs::create_dir(&staging).map_err(|e| e.to_string())?;
    let staged = staging.join("pitex.deb");
    std::fs::copy(downloaded, &staged)
        .map_err(|e| format!("Could not stage the update package: {e}"))?;
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&staging, std::fs::Permissions::from_mode(0o755))
        .and_then(|_| std::fs::set_permissions(&staged, std::fs::Permissions::from_mode(0o644)))
        .map_err(|e| e.to_string())?;
    let deb = staged.to_string_lossy().into_owned();
    let elevated = Command::new("pkexec")
        .args(["apt", "install", "-y", &deb])
        .stdin(std::process::Stdio::null())
        .status();
    match elevated {
        Ok(status) if status.success() => {
            let installed = Command::new("dpkg-query")
                .args(["-W", "-f=${Status} ${Version}", "pitex"])
                .output().map_err(|e| e.to_string())?;
            let text = String::from_utf8_lossy(&installed.stdout);
            let _ = std::fs::remove_dir_all(&staging);
            if installed.status.success() && text.strip_prefix("install ok installed ")
                .is_some_and(|version| !version_newer(&info.tag, version.trim())) {
                return Ok(InstallOutcome::AwaitingRestart);
            }
            return Err(format!("{} was not installed: {}", info.tag, text.trim()));
        }
        _ => {}
    }
    // No polkit agent (or it failed): hand the install to the user.
    if crate::compat::run_in_external_terminal(
        &format!("sudo apt install -y {}", shell_quote(&deb)),
        None,
    ) {
        return Ok(InstallOutcome::HandedToTerminal);
    }
    reveal(&downloaded.to_path_buf());
    Ok(InstallOutcome::ManualFallback)
}

#[cfg(windows)]
fn install_impl(downloaded: &Path, info: &UpdateInfo) -> Result<InstallOutcome, String> {
    if downloaded
        .extension()
        .map(|e| e.eq_ignore_ascii_case("exe"))
        .unwrap_or(false)
    {
        install_setup(downloaded)
    } else {
        install_zip(downloaded, info)
    }
}

/// NSIS setup.exe: the installer does all file work — a helper script
/// waits for this process to exit, runs it silently (/S), and relaunches
/// the installed exe. A portable-zip copy migrates to the installed
/// location (%LOCALAPPDATA%\Programs\Pitex) the same way.
#[cfg(windows)]
fn install_setup(setup: &Path) -> Result<InstallOutcome, String> {
    let staging = setup
        .parent()
        .ok_or_else(|| "The download path is invalid.".to_string())?;
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let install_root = exe.parent().and_then(|bin| bin.parent())
        .filter(|root| root.join("uninstall.exe").is_file())
        .map(Path::to_path_buf)
        .or_else(|| dirs::data_local_dir().map(|d| d.join("Programs").join("Pitex")))
        .ok_or_else(|| "Could not resolve the install directory.".to_string())?;
    let action = format!(
        "$installer = Start-Process -FilePath {} -ArgumentList {} -Wait -PassThru\nif ($installer.ExitCode -ne 0) {{ throw \"Installer failed: $($installer.ExitCode)\" }}",
        powershell_quote(&setup.to_string_lossy()),
        powershell_quote(&format!("/S /D={}", install_root.display()))
    );
    launch_windows_update(staging, &install_root.join("bin/pitex.exe"), &action)
}

/// Portable zip: a running exe cannot overwrite itself, so a helper script
/// waits for the process to exit, mirrors the new tree over the install
/// root, and relaunches it.
#[cfg(windows)]
fn install_zip(downloaded: &Path, info: &UpdateInfo) -> Result<InstallOutcome, String> {
    let staging = downloaded
        .parent()
        .map(|p| p.join(format!("extract-{}", crate::model::uuid_v4())))
        .ok_or_else(|| "The download path is invalid.".to_string())?;
    std::fs::create_dir_all(&staging).map_err(|e| e.to_string())?;
    // bsdtar (System32, Win10 1803+) reads zips natively; Expand-Archive is
    // the guaranteed fallback.
    let extracted = Command::new("tar")
        .args(["-xf", &downloaded.to_string_lossy()])
        .current_dir(&staging)
        .stdin(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
        || Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                &format!(
                    "Expand-Archive -Force -LiteralPath {} -DestinationPath {}",
                    powershell_quote(&downloaded.to_string_lossy()),
                    powershell_quote(&staging.to_string_lossy())
                ),
            ])
            .stdin(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
    if !extracted || !staging.join("pitex").is_dir() {
        return Err(format!("Could not extract {}", info.asset_name));
    }
    // The bundle layout is <root>/bin/pitex.exe — the install root is the
    // exe's grandparent. Anything else (a cargo target dir) is not a
    // self-installed bundle; open the download instead.
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let install_root = exe
        .parent()
        .filter(|bin| {
            bin.file_name()
                .map(|n| n.to_string_lossy().eq_ignore_ascii_case("bin"))
                .unwrap_or(false)
        })
        .and_then(|bin| bin.parent())
        .map(|p| p.to_path_buf())
        .ok_or_else(|| "The app is not running from an installed bundle.".to_string())
        .or_else(|e| {
            reveal(downloaded);
            Err(e)
        })?;
    let action = format!(
        "& robocopy {} {} /E /IS /IT /NFL /NDL /NJH /NJS /NP /R:10 /W:1\nif ($LASTEXITCODE -ge 8) {{ throw \"Bundle copy failed: $LASTEXITCODE\" }}",
        powershell_quote(&staging.join("pitex").to_string_lossy()),
        powershell_quote(&install_root.to_string_lossy())
    );
    launch_windows_update(&staging, &install_root.join("bin/pitex.exe"), &action)
}

#[cfg(any(windows, test))]
fn powershell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

#[cfg(any(windows, test))]
fn windows_update_script(pid: u32, executable: &Path, action: &str) -> String {
    format!(r#"$ErrorActionPreference = 'Stop'
try {{
    $app = Get-Process -Id {pid} -ErrorAction SilentlyContinue
    if ($app) {{ Wait-Process -InputObject $app -Timeout 120 }}
    {action}
    Start-Process -FilePath {executable}
}} catch {{
    $_ | Out-File -LiteralPath "$PSCommandPath.log"
    Invoke-Item -LiteralPath "$PSCommandPath.log"
    exit 1
}}
Remove-Item -LiteralPath $PSCommandPath
"#, executable = powershell_quote(&executable.to_string_lossy()))
}

#[cfg(windows)]
fn launch_windows_update(staging: &Path, executable: &Path, action: &str) -> Result<InstallOutcome, String> {
    use std::os::windows::process::CommandExt;
    let script_path = staging.join(format!("pitex-update-{}.ps1", crate::model::uuid_v4()));
    let script = windows_update_script(std::process::id(), executable, action);
    // Windows PowerShell 5.1 needs the BOM for non-ASCII installation paths.
    std::fs::write(&script_path, format!("\u{feff}{script}")).map_err(|e| e.to_string())?;
    Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-File"])
        .arg(&script_path)
        .creation_flags(0x08000000) // CREATE_NO_WINDOW; survives the app exiting.
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(InstallOutcome::ExitRequested)
}

#[cfg(target_os = "macos")]
fn install_impl(downloaded: &Path, _info: &UpdateInfo) -> Result<InstallOutcome, String> {
    let _ = downloaded;
    Err("macOS updates are installed by the app itself.".to_string())
}

/// Open the downloaded package's folder so the user can install manually.
fn reveal(path: &Path) {
    #[cfg(unix)]
    {
        let _ = Command::new("xdg-open")
            .arg(path.parent().unwrap_or(path))
            .stdin(std::process::Stdio::null())
            .spawn();
    }
    #[cfg(windows)]
    {
        let _ = Command::new("explorer")
            .arg(format!("/select,{}", path.to_string_lossy()))
            .stdin(std::process::Stdio::null())
            .spawn();
    }
}

#[cfg(unix)]
fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

/// The whole auto-update pass for launch: check, download, install.
/// Returns the outcome so the caller can toast or exit.
pub fn auto_update(current: &str) -> Result<Option<(UpdateInfo, InstallOutcome)>, String> {
    let Some(info) = check_for_update(current)? else {
        return Ok(None);
    };
    let path = download(&info)?;
    let outcome = install(&path, &info)?;
    Ok(Some((info, outcome)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn newer_tag_wins() {
        assert!(version_newer("v1.0.3", "1.0.2"));
        assert!(version_newer("v2.0.0", "1.9.9"));
        assert!(version_newer("v1.1.0", "1.0.9"));
    }

    #[test]
    fn equal_or_older_is_not_an_update() {
        assert!(!version_newer("v1.0.2", "1.0.2"));
        assert!(!version_newer("v1.0.1", "1.0.2"));
        assert!(!version_newer("v0.9.9", "1.0.0"));
    }

    #[test]
    fn version_lengths_may_differ() {
        assert!(version_newer("v1.0.2.1", "1.0.2"));
        assert!(!version_newer("v1.0", "1.0.1"));
        assert!(!version_newer("v1.1.4", "1.1.4-1"));
    }

    #[test]
    fn windows_script_quotes_paths_and_waits_before_installing() {
        let script = windows_update_script(123, Path::new("C:\\User's 한글\\bin\\pitex.exe"), "INSTALL_HERE");
        assert!(script.contains("'C:\\User''s 한글\\bin\\pitex.exe'"));
        assert!(script.find("Wait-Process").unwrap() < script.find("INSTALL_HERE").unwrap());
        assert!(script.find("INSTALL_HERE").unwrap() < script.find("Start-Process").unwrap());
        assert!(script.contains("catch") && script.contains("exit 1"));
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    #[test]
    fn linux_checks_installed_version_after_successful_apt() {
        use std::os::unix::fs::PermissionsExt;
        let root = std::env::temp_dir().join(format!("pitex-install-check-{}", crate::model::uuid_v4()));
        std::fs::create_dir(&root).unwrap();
        let old_path = std::env::var_os("PATH");
        struct Cleanup(PathBuf, Option<std::ffi::OsString>);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                if let Some(path) = &self.1 { std::env::set_var("PATH", path); }
                else { std::env::remove_var("PATH"); }
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
        let _cleanup = Cleanup(root.clone(), old_path);
        let pkexec = root.join("pkexec");
        std::fs::write(&pkexec, "#!/bin/sh\n[ \"$1 $2 $3\" = 'apt install -y' ] && [ -r \"$4\" ]\n").unwrap();
        std::fs::set_permissions(&pkexec, std::fs::Permissions::from_mode(0o755)).unwrap();
        let query = root.join("dpkg-query");
        let package = root.join("update.deb");
        std::fs::write(&package, "test package").unwrap();
        std::env::set_var("PATH", &root);
        let info = UpdateInfo { tag: "v1.1.4".into(), asset_name: "update.deb".into(), asset_url: String::new() };
        for (version, succeeds) in [("install ok installed 1.1.3-1", false), ("install ok installed 1.1.4-1", true), ("deinstall ok config-files 1.1.4-1", false)] {
            std::fs::write(&query, format!("#!/bin/sh\nprintf '%s' '{version}'\n")).unwrap();
            std::fs::set_permissions(&query, std::fs::Permissions::from_mode(0o755)).unwrap();
            let result = install(&package, &info);
            assert_eq!(result.is_ok(), succeeds, "{version}: {result:?}");
        }
    }

    #[cfg(windows)]
    #[test]
    fn windows_helper_waits_and_does_not_relaunch_after_failure() {
        let root = std::env::temp_dir().join(format!("pitex update's 한글 {}", crate::model::uuid_v4()));
        std::fs::create_dir(&root).unwrap();
        let launched = root.join("launched");
        let installed = root.join("installed");
        for fail in [false, true] {
            let mut app = Command::new("powershell.exe").args(["-NoProfile", "-Command", "Start-Sleep -Seconds 3"]).spawn().unwrap();
            let script_path = root.join("update.ps1");
            let action = if fail { "throw 'install failed'".to_string() } else {
                format!("Set-Content -LiteralPath {} -Value done", powershell_quote(&installed.to_string_lossy()))
            };
            // Exercise the real wait/catch flow, replacing only app launch
            // and the error viewer to avoid opening UI during the test.
            let mocks = format!("function Start-Process {{ Set-Content -LiteralPath {} -Value launched }}\nfunction Invoke-Item {{}}\n", powershell_quote(&launched.to_string_lossy()));
            let script = windows_update_script(app.id(), Path::new("test.exe"), &action);
            std::fs::write(&script_path, format!("\u{feff}{mocks}{script}")).unwrap();
            let mut helper = Command::new("powershell.exe")
                .args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-File"])
                .arg(&script_path).spawn().unwrap();
            std::thread::sleep(std::time::Duration::from_millis(500));
            assert!(!installed.exists() && !launched.exists(), "Installer ran before app exit");
            let status = helper.wait().unwrap();
            app.wait().unwrap();
            assert_eq!(status.success(), !fail);
            assert_eq!(launched.exists(), !fail);
            if !fail {
                std::fs::remove_file(&installed).unwrap();
                std::fs::remove_file(&launched).unwrap();
            } else { assert!(root.join("update.ps1.log").is_file()); }
        }
        std::fs::remove_dir_all(root).unwrap();
    }
}
