//! Compile-time application identity selected by PITEX_CHANNEL and PITEX_VERSION.
//!
//! Stable and nightly share one source of truth: `Identity`.  `current()` picks
//! the value for the compiled channel, `STABLE` exposes the existing literals,
//! `NIGHTLY` exposes the nightly literals, and `app_version()` applies the
//! optional PITEX_VERSION override.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Identity {
    pub channel: &'static str,
    pub display_name: &'static str,
    pub app_id: &'static str,
    pub xdg_dir_name: &'static str,
    pub cache_dir_name: &'static str,
    pub temp_prefix: &'static str,
    pub linux_package_name: &'static str,
    pub linux_desktop_id: &'static str,
    pub linux_desktop_name: &'static str,
    pub linux_icon_name: &'static str,
    pub linux_share_dir_name: &'static str,
    pub windows_exe_name: &'static str,
    pub windows_zip_root: &'static str,
    pub windows_install_dir_name: &'static str,
    pub windows_progid_prefix: &'static str,
    pub release_api: &'static str,
    pub is_nightly: bool,
}

pub const STABLE: Identity = Identity {
    channel: "stable",
    display_name: "Pitex",
    app_id: "dev.pitex.app",
    xdg_dir_name: "pitex",
    cache_dir_name: "dev.pitex.app",
    temp_prefix: "pitex",
    linux_package_name: "pitex",
    linux_desktop_id: "dev.pitex.app.desktop",
    linux_desktop_name: "Pitex",
    linux_icon_name: "dev.pitex.app",
    linux_share_dir_name: "pitex",
    windows_exe_name: "pitex.exe",
    windows_zip_root: "pitex",
    windows_install_dir_name: "Pitex",
    windows_progid_prefix: "Pitex",
    release_api: "https://api.github.com/repos/jaehwan-2ee/pitex/releases/latest",
    is_nightly: false,
};

pub const NIGHTLY: Identity = Identity {
    channel: "nightly",
    display_name: "Pitex Nightly",
    app_id: "dev.pitex.app.nightly",
    xdg_dir_name: "pitex-nightly",
    cache_dir_name: "dev.pitex.app.nightly",
    temp_prefix: "pitex-nightly",
    linux_package_name: "pitex-nightly",
    linux_desktop_id: "dev.pitex.app.nightly.desktop",
    linux_desktop_name: "Pitex Nightly",
    linux_icon_name: "dev.pitex.app.nightly",
    linux_share_dir_name: "pitex-nightly",
    windows_exe_name: "pitex-nightly.exe",
    windows_zip_root: "pitex-nightly",
    windows_install_dir_name: "Pitex Nightly",
    windows_progid_prefix: "PitexNightly",
    release_api: "https://api.github.com/repos/jaehwan-2ee/pitex/releases/tags/nightly",
    is_nightly: true,
};

pub fn current() -> &'static Identity {
    if option_env!("PITEX_CHANNEL") == Some("nightly") {
        &NIGHTLY
    } else {
        &STABLE
    }
}

pub fn app_version(default: &'static str) -> &'static str {
    match option_env!("PITEX_VERSION") {
        Some(v) if !v.is_empty() => v,
        _ => default,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stable_matches_existing_literals() {
        assert_eq!(STABLE.channel, "stable");
        assert_eq!(STABLE.display_name, "Pitex");
        assert_eq!(STABLE.app_id, "dev.pitex.app");
        assert_eq!(STABLE.xdg_dir_name, "pitex");
        assert_eq!(STABLE.cache_dir_name, "dev.pitex.app");
        assert_eq!(STABLE.temp_prefix, "pitex");
        assert_eq!(STABLE.linux_package_name, "pitex");
        assert_eq!(STABLE.linux_desktop_id, "dev.pitex.app.desktop");
        assert_eq!(STABLE.windows_exe_name, "pitex.exe");
        assert_eq!(STABLE.windows_progid_prefix, "Pitex");
        assert_eq!(STABLE.windows_install_dir_name, "Pitex");
        assert!(!STABLE.is_nightly);
    }

    #[test]
    fn nightly_differs_from_stable_everywhere() {
        assert_ne!(NIGHTLY.channel, STABLE.channel);
        assert_ne!(NIGHTLY.display_name, STABLE.display_name);
        assert_ne!(NIGHTLY.app_id, STABLE.app_id);
        assert_ne!(NIGHTLY.xdg_dir_name, STABLE.xdg_dir_name);
        assert_ne!(NIGHTLY.cache_dir_name, STABLE.cache_dir_name);
        assert_ne!(NIGHTLY.temp_prefix, STABLE.temp_prefix);
        assert_ne!(NIGHTLY.linux_package_name, STABLE.linux_package_name);
        assert_ne!(NIGHTLY.linux_desktop_id, STABLE.linux_desktop_id);
        assert_ne!(NIGHTLY.windows_exe_name, STABLE.windows_exe_name);
        assert_ne!(NIGHTLY.windows_progid_prefix, STABLE.windows_progid_prefix);
        assert_ne!(NIGHTLY.windows_install_dir_name, STABLE.windows_install_dir_name);
        assert_ne!(NIGHTLY.release_api, STABLE.release_api);
        assert!(NIGHTLY.is_nightly);
    }

    #[test]
    fn current_matches_compile_time_channel() {
        if option_env!("PITEX_CHANNEL") == Some("nightly") {
            assert!(current().is_nightly);
            assert_eq!(current().display_name, "Pitex Nightly");
        } else {
            assert!(!current().is_nightly);
            assert_eq!(current().display_name, "Pitex");
        }
    }
}
