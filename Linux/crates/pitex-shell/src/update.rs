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
            .map(|p| p.trim_end_matches(|c: char| !c.is_ascii_digit()).parse().unwrap_or(0))
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
fn install_impl(downloaded: &Path, _info: &UpdateInfo) -> Result<InstallOutcome, String> {
    // apt's `_apt` sandbox user can't read files under $HOME (0700 on
    // Ubuntu 24.04), which makes `apt install ./file.deb` warn "Download is
    // performed unsandboxed as root" — and fail outright on some setups.
    // Stage the package in /tmp, world-readable, before elevating.
    let staged = std::env::temp_dir().join(
        downloaded
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "pitex-update.deb".into()),
    );
    std::fs::copy(downloaded, &staged)
        .map_err(|e| format!("Could not stage the update package: {e}"))?;
    use std::os::unix::fs::PermissionsExt;
    let _ = std::fs::set_permissions(&staged, std::fs::Permissions::from_mode(0o644));
    let deb = staged.to_string_lossy().into_owned();
    let elevated = Command::new("pkexec")
        .args(["apt", "install", "-y", &deb])
        .stdin(std::process::Stdio::null())
        .status();
    match elevated {
        Ok(status) if status.success() => return Ok(InstallOutcome::AwaitingRestart),
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

/// NSIS setup.exe: the installer does all file work — an updater .cmd just
/// waits for this process to exit, runs it silently (/S), and relaunches
/// the installed exe. A portable-zip copy migrates to the installed
/// location (%LOCALAPPDATA%\Programs\Pitex) the same way.
#[cfg(windows)]
fn install_setup(setup: &Path) -> Result<InstallOutcome, String> {
    let staging = setup
        .parent()
        .ok_or_else(|| "The download path is invalid.".to_string())?;
    let installed_exe = dirs::data_local_dir()
        .map(|d| d.join("Programs").join("Pitex").join("bin").join("pitex.exe"))
        .ok_or_else(|| "Could not resolve the install directory.".to_string())?;
    let script_path = staging.join("pitex-update.cmd");
    let script = format!(
        "@echo off\r\ntimeout /t 2 /nobreak >nul\r\n\"{}\" /S\r\nstart \"\" \"{}\"\r\ndel \"%~f0\"\r\n",
        setup.to_string_lossy(),
        installed_exe.to_string_lossy()
    );
    std::fs::write(&script_path, script).map_err(|e| e.to_string())?;
    Command::new("cmd")
        .args(["/c", "start", "/min", "PitexUpdate"])
        .arg(&script_path)
        .stdin(std::process::Stdio::null())
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(InstallOutcome::ExitRequested)
}

/// Portable zip: a running exe cannot overwrite itself, so an updater .cmd
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
                    "Expand-Archive -Force '{}' '{}'",
                    downloaded.to_string_lossy(),
                    staging.to_string_lossy()
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
    let script_path = staging.join("pitex-update.cmd");
    let script = format!(
        "@echo off\r\ntimeout /t 2 /nobreak >nul\r\nrobocopy \"{}\" \"{}\" /E /IS /IT /NFL /NDL /NJH /NJS /NP /R:10 /W:1 >nul\r\nstart \"\" \"{}\"\r\ndel \"%~f0\"\r\n",
        staging.join("pitex").to_string_lossy(),
        install_root.to_string_lossy(),
        install_root.join("bin").join("pitex.exe").to_string_lossy()
    );
    std::fs::write(&script_path, script).map_err(|e| e.to_string())?;
    Command::new("cmd")
        .args(["/c", "start", "/min", "PitexUpdate"])
        .arg(&script_path)
        .stdin(std::process::Stdio::null())
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
    use super::version_newer;

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
    }
}
