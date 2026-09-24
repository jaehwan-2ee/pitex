//! Port of `SettingsStore` + `AppearanceSettings` from
//! `Mac/Sources/Features/SettingsView.swift` and `AppearanceTheme.swift`.
//! `UserDefaults` maps to a JSON plist at
//! `~/.config/pitex/preferences.json` — same keys, same defaults.

use settings_feature::PersistedSettings;
use std::collections::BTreeMap;
use std::path::PathBuf;

/// Flat key→JSON store, mirroring `UserDefaults.standard` keys verbatim so a
/// user's macOS-era expectations carry over.
#[derive(Debug, Default)]
pub struct Preferences {
    values: BTreeMap<String, serde_json::Value>,
    path: PathBuf,
}
impl Preferences {
    pub fn standard() -> Self {
        let dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("pitex");
        let path = dir.join("preferences.json");
        let values = std::fs::read(&path)
            .ok()
            .and_then(|d| serde_json::from_slice(&d).ok())
            .unwrap_or_default();
        Self { values, path }
    }

    fn persist(&self) {
        if let Some(parent) = self.path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(data) = serde_json::to_vec_pretty(&self.values) {
            let tmp = self.path.with_extension("tmp");
            if std::fs::write(&tmp, &data).is_ok() {
                let _ = std::fs::rename(&tmp, &self.path);
            }
        }
    }

    pub fn get(&self, key: &str) -> Option<&serde_json::Value> {
        self.values.get(key)
    }
    pub fn string(&self, key: &str) -> Option<String> {
        self.values.get(key)?.as_str().map(|s| s.to_string())
    }
    pub fn bool(&self, key: &str) -> Option<bool> {
        self.values.get(key)?.as_bool()
    }
    pub fn int(&self, key: &str) -> Option<i64> {
        self.values.get(key)?.as_i64()
    }
    pub fn double(&self, key: &str) -> Option<f64> {
        self.values.get(key)?.as_f64()
    }
    pub fn string_array(&self, key: &str) -> Option<Vec<String>> {
        self.values.get(key)?.as_array().map(|a| {
            a.iter().filter_map(|v| v.as_str().map(String::from)).collect()
        })
    }
    pub fn dictionary(&self, key: &str) -> Option<&serde_json::Map<String, serde_json::Value>> {
        self.values.get(key)?.as_object()
    }
    pub fn contains(&self, key: &str) -> bool {
        self.values.contains_key(key)
    }
    pub fn set(&mut self, key: &str, value: impl Into<serde_json::Value>) {
        self.values.insert(key.to_string(), value.into());
        self.persist();
    }
    pub fn remove(&mut self, key: &str) {
        self.values.remove(key);
        self.persist();
    }

    /// Settings backup — the same key filter the macOS `SettingsBackup`
    /// applies. The `ai.*` Pitex Agent section and session state
    /// (`pitex.pref.workspace.*`, `commands.<path>`) are per-machine and
    /// never exported.
    const BACKUP_PREFIXES: &'static [&'static str] = &[
        "dev.pitex.",
        "pitex.pref.",
        "appearance.",
        "project.",
        "customShellExecutable",
    ];
    const BACKUP_EXCLUDES: &'static [&'static str] = &["ai.", "pitex.pref.workspace."];

    fn backup_key(key: &str) -> bool {
        Self::BACKUP_PREFIXES.iter().any(|p| key.starts_with(p))
            && !Self::BACKUP_EXCLUDES.iter().any(|p| key.starts_with(p))
    }

    /// `{format, version, preferences:{key: value}}` — the envelope the
    /// macOS export writes, so one file moves between platforms.
    pub fn export_backup(&self) -> serde_json::Value {
        let prefs: serde_json::Map<String, serde_json::Value> = self
            .values
            .iter()
            .filter(|(k, _)| Self::backup_key(k))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        serde_json::json!({
            "format": "pitex-settings",
            "version": 1,
            "preferences": prefs,
        })
    }

    /// Merge a backup file (the envelope above, or a bare key→value map)
    /// into the store; returns the number of applied keys.
    pub fn import_backup(&mut self, data: &[u8]) -> Result<usize, String> {
        let parsed: serde_json::Value =
            serde_json::from_slice(data).map_err(|e| e.to_string())?;
        let root = parsed.as_object().ok_or("Not a settings file")?;
        let prefs = parsed
            .get("preferences")
            .and_then(|v| v.as_object())
            .unwrap_or(root);
        let mut applied = 0;
        for (key, value) in prefs {
            if Self::backup_key(key) {
                self.values.insert(key.clone(), value.clone());
                applied += 1;
            }
        }
        self.persist();
        Ok(applied)
    }
}

/// Port of `SettingsStore` — every @Published field with its UserDefaults key
/// and default.
pub struct SettingsStore {
    prefs: Preferences,
    pub settings: PersistedSettings,
}
impl SettingsStore {
    const SETTINGS_KEY: &'static str = "dev.pitex.settings";

    pub fn new(prefs: Preferences) -> Self {
        let settings = prefs
            .get(Self::SETTINGS_KEY)
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_else(PersistedSettings::safe_defaults);
        Self { prefs, settings }
    }

    fn persist_settings(&mut self) {
        if let Ok(v) = serde_json::to_value(&self.settings) {
            self.prefs.set(Self::SETTINGS_KEY, v);
        }
    }

    // Field accessors with the exact UserDefaults key + default.
    pub fn custom_shell_executable(&self) -> String {
        // The Windows default is cmd — `LoginShellCommandPlan` pairs it with
        // `/c`, the counterpart of bash's `-l -c`.
        let default = if cfg!(windows) { "cmd.exe" } else { "/bin/bash" };
        self.prefs.string("customShellExecutable").unwrap_or_else(|| default.into())
    }
    pub fn set_custom_shell_executable(&mut self, v: &str) {
        self.prefs.set("customShellExecutable", v);
    }
    pub fn switch_to_pdf_on_build(&self) -> bool {
        self.prefs.bool("pitex.pref.build.switchToPDFOnBuild").unwrap_or(true)
    }
    pub fn set_switch_to_pdf_on_build(&mut self, v: bool) {
        self.prefs.set("pitex.pref.build.switchToPDFOnBuild", v);
    }
    pub fn jump_to_cursor_after_build(&self) -> bool {
        self.prefs.bool("pitex.pref.build.jumpToCursorAfterBuild").unwrap_or(true)
    }
    pub fn set_jump_to_cursor_after_build(&mut self, v: bool) {
        self.prefs.set("pitex.pref.build.jumpToCursorAfterBuild", v);
    }
    /// `pitex.pref.update.autoInstall` — on launch, check GitHub Releases
    /// and install a newer package without asking.
    pub fn auto_install_updates(&self) -> bool {
        self.prefs.bool("pitex.pref.update.autoInstall").unwrap_or(false)
    }
    pub fn set_auto_install_updates(&mut self, v: bool) {
        self.prefs.set("pitex.pref.update.autoInstall", v);
    }
    pub fn restore_session(&self) -> bool {
        self.prefs.bool("pitex.pref.editor.restoreSession").unwrap_or(true)
    }
    pub fn set_restore_session(&mut self, v: bool) {
        self.prefs.set("pitex.pref.editor.restoreSession", v);
    }
    pub fn code_folding(&self) -> bool {
        self.prefs.bool("pitex.pref.editor.codeFolding").unwrap_or(true)
    }
    pub fn set_code_folding(&mut self, v: bool) {
        self.prefs.set("pitex.pref.editor.codeFolding", v);
    }
    pub fn minimap(&self) -> bool {
        self.prefs.bool("pitex.pref.editor.minimap").unwrap_or(true)
    }
    pub fn set_minimap(&mut self, v: bool) {
        self.prefs.set("pitex.pref.editor.minimap", v);
    }
    /// `inverseSyncHighlight`/`forwardSyncHighlight` — persisted separately,
    /// default off; navigation still works with highlights disabled.
    pub fn inverse_sync_highlight(&self) -> bool {
        self.prefs
            .bool("pitex.pref.synctex.inverseHighlight")
            .unwrap_or(false)
    }
    pub fn set_inverse_sync_highlight(&mut self, v: bool) {
        self.prefs.set("pitex.pref.synctex.inverseHighlight", v);
    }
    pub fn forward_sync_highlight(&self) -> bool {
        self.prefs
            .bool("pitex.pref.synctex.forwardHighlight")
            .unwrap_or(false)
    }
    pub fn set_forward_sync_highlight(&mut self, v: bool) {
        self.prefs.set("pitex.pref.synctex.forwardHighlight", v);
    }
    pub fn auto_save(&self) -> bool {
        self.prefs.bool("pitex.pref.editor.autoSave").unwrap_or(false)
    }
    pub fn set_auto_save(&mut self, v: bool) {
        self.prefs.set("pitex.pref.editor.autoSave", v);
    }
    pub fn auto_save_delay(&self) -> i64 {
        self.prefs.int("pitex.pref.editor.autoSaveDelay").unwrap_or(5)
    }
    pub fn set_auto_save_delay(&mut self, v: i64) {
        self.prefs.set("pitex.pref.editor.autoSaveDelay", v);
    }
    pub fn confirm_overwrite(&self) -> bool {
        self.prefs.bool("pitex.pref.editor.confirmOverwrite").unwrap_or(true)
    }
    pub fn set_confirm_overwrite(&mut self, v: bool) {
        self.prefs.set("pitex.pref.editor.confirmOverwrite", v);
    }
    pub fn chat_history_limit(&self) -> i64 {
        self.prefs.int("ai.chatHistoryLimit").unwrap_or(50)
    }
    pub fn set_chat_history_limit(&mut self, v: i64) {
        self.prefs.set("ai.chatHistoryLimit", v);
    }
    pub fn ai_default_model(&self) -> String {
        self.prefs.string("ai.defaultModel").unwrap_or_default()
    }
    pub fn set_ai_default_model(&mut self, v: &str) {
        self.prefs.set("ai.defaultModel", v);
    }
    pub fn ai_attach_default(&self) -> bool {
        self.prefs.bool("ai.attachDefault").unwrap_or(true)
    }
    pub fn set_ai_attach_default(&mut self, v: bool) {
        self.prefs.set("ai.attachDefault", v);
    }
    /// `ai.autocompletion` — Copilot-style ghost text while typing LaTeX.
    /// Same key and default-OFF as the Mac `SettingsStore`: the feature
    /// spawns a pi subprocess, so it is opt-in, and silently no-ops until
    /// a `.tex` document is open and pi resolves.
    pub fn ai_autocompletion(&self) -> bool {
        self.prefs.bool("ai.autocompletion").unwrap_or(false)
    }
    pub fn set_ai_autocompletion(&mut self, v: bool) {
        self.prefs.set("ai.autocompletion", v);
    }
    /// `ai.fontSize` — conversation text size, clamped to 10…24, default 13.
    pub fn ai_font_size(&self) -> f64 {
        self.prefs.double("ai.fontSize").unwrap_or(13.0).clamp(10.0, 24.0)
    }
    pub fn set_ai_font_size(&mut self, v: f64) {
        self.prefs.set("ai.fontSize", v);
    }
    pub fn default_build_command(&self) -> String {
        self.prefs
            .string("project.defaultBuildCommand")
            .unwrap_or_else(|| "xelatex -interaction=nonstopmode -synctex=1 {file}".into())
    }
    pub fn set_default_build_command(&mut self, v: &str) {
        self.prefs.set("project.defaultBuildCommand", v);
    }
    pub fn default_custom_command(&self) -> String {
        self.prefs.string("project.defaultCustomCommand").unwrap_or_default()
    }
    pub fn set_default_custom_command(&mut self, v: &str) {
        self.prefs.set("project.defaultCustomCommand", v);
    }
    /// `pitex.pref.markdown.livePreview` — render on every edit, default on.
    /// Off renders on activation and after save instead.
    pub fn markdown_live_preview(&self) -> bool {
        self.prefs.bool("pitex.pref.markdown.livePreview").unwrap_or(true)
    }
    pub fn set_markdown_live_preview(&mut self, v: bool) {
        self.prefs.set("pitex.pref.markdown.livePreview", v);
    }
    /// `pitex.pref.markdown.syncScroll` — editor↔preview scroll sync,
    /// default on.
    pub fn markdown_sync_scroll(&self) -> bool {
        self.prefs.bool("pitex.pref.markdown.syncScroll").unwrap_or(true)
    }
    pub fn set_markdown_sync_scroll(&mut self, v: bool) {
        self.prefs.set("pitex.pref.markdown.syncScroll", v);
    }
    /// `pitex.pref.markdown.theme` — "system" | "light" | "dark"; missing
    /// or unknown values resolve to "system".
    pub fn markdown_theme(&self) -> &'static str {
        match self.prefs.string("pitex.pref.markdown.theme").as_deref() {
            Some("light") => "light",
            Some("dark") => "dark",
            _ => "system",
        }
    }
    pub fn set_markdown_theme(&mut self, v: &str) {
        self.prefs.set("pitex.pref.markdown.theme", v);
    }
    /// `pitex.pref.markdown.fontSize` — preview root size, 10…28, default 16.
    pub fn markdown_font_size(&self) -> f64 {
        self.prefs
            .double("pitex.pref.markdown.fontSize")
            .unwrap_or(16.0)
            .clamp(10.0, 28.0)
    }
    pub fn set_markdown_font_size(&mut self, v: f64) {
        self.prefs.set("pitex.pref.markdown.fontSize", v);
    }

    pub fn update_settings(&mut self, next: PersistedSettings) {
        self.settings = next;
        self.persist_settings();
    }
    /// Re-read the `PersistedSettings` blob after a settings import wrote
    /// new values behind the store's back (flat keys read live already).
    pub fn reload(&mut self) {
        if let Some(v) = self.prefs.get(Self::SETTINGS_KEY) {
            if let Ok(decoded) = serde_json::from_value(v.clone()) {
                self.settings = decoded;
            }
        }
    }
    pub fn prefs(&self) -> &Preferences {
        &self.prefs
    }
    pub fn prefs_mut(&mut self) -> &mut Preferences {
        &mut self.prefs
    }

    /// `pitex.pref.ssh.connections` — the devices "Open via SSH" offers;
    /// the JSON schema matches what the macOS SettingsStore writes.
    #[cfg(unix)]
    pub fn ssh_connections(&self) -> Vec<remote_core::SshConnection> {
        self.prefs
            .get("pitex.pref.ssh.connections")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default()
    }

    #[cfg(unix)]
    pub fn set_ssh_connections(&mut self, connections: &[remote_core::SshConnection]) {
        if let Ok(value) = serde_json::to_value(connections) {
            self.prefs.set("pitex.pref.ssh.connections", value);
        }
    }

    /// `pitex.pref.ssh.lastConnection` — the device "Open via SSH"
    /// preselects (a connection `id`, a UUID string like macOS stores).
    #[cfg(unix)]
    pub fn last_ssh_connection(&self) -> Option<String> {
        self.prefs.string("pitex.pref.ssh.lastConnection")
    }

    #[cfg(unix)]
    pub fn set_last_ssh_connection(&mut self, id: &str) {
        self.prefs.set("pitex.pref.ssh.lastConnection", id);
    }
}

// ─── Appearance ────────────────────────────────────────────────────────────

/// `AppearanceColorRole` — the eleven highlight roles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AppearanceColorRole {
    EditorBackground,
    GutterBackground,
    LineNumbers,
    BodyText,
    Commands,
    Environments,
    Math,
    Braces,
    Comments,
    FoldAccent,
    BracketMatch,
}
impl AppearanceColorRole {
    pub const ALL: [Self; 11] = [
        Self::EditorBackground,
        Self::GutterBackground,
        Self::LineNumbers,
        Self::BodyText,
        Self::Commands,
        Self::Environments,
        Self::Math,
        Self::Braces,
        Self::Comments,
        Self::FoldAccent,
        Self::BracketMatch,
    ];
    pub fn raw_value(self) -> &'static str {
        match self {
            Self::EditorBackground => "editorBackground",
            Self::GutterBackground => "gutterBackground",
            Self::LineNumbers => "lineNumbers",
            Self::BodyText => "bodyText",
            Self::Commands => "commands",
            Self::Environments => "environments",
            Self::Math => "math",
            Self::Braces => "braces",
            Self::Comments => "comments",
            Self::FoldAccent => "foldAccent",
            Self::BracketMatch => "bracketMatch",
        }
    }
    /// Palette default for an unset role — resolved against the effective
    /// appearance so the editor follows light/dark mode (`defaultHex(dark:)`).
    pub fn default_hex(self, dark: bool) -> &'static str {
        if dark {
            AppearanceThemePreset::Dark
        } else {
            AppearanceThemePreset::Light
        }
        .hex_for(&self)
    }
    /// `titleKey` — localization key for the row title.
    pub fn title_key(self) -> &'static str {
        match self {
            Self::EditorBackground => "appearance.color.editor_background",
            Self::GutterBackground => "appearance.color.gutter_background",
            Self::LineNumbers => "appearance.color.line_numbers",
            Self::BodyText => "appearance.color.body_text",
            Self::Commands => "appearance.color.commands",
            Self::Environments => "appearance.color.environments",
            Self::Math => "appearance.color.math",
            Self::Braces => "appearance.color.braces",
            Self::Comments => "appearance.color.comments",
            Self::FoldAccent => "appearance.color.fold_accent",
            Self::BracketMatch => "appearance.color.bracket_match",
        }
    }
    /// `detailKey` — localization key for the row's secondary text.
    pub fn detail_key(self) -> &'static str {
        match self {
            Self::EditorBackground => "appearance.color.editor_background_d",
            Self::GutterBackground => "appearance.color.gutter_background_d",
            Self::LineNumbers => "appearance.color.line_numbers_d",
            Self::BodyText => "appearance.color.body_text_d",
            Self::Commands => "appearance.color.commands_d",
            Self::Environments => "appearance.color.environments_d",
            Self::Math => "appearance.color.math_d",
            Self::Braces => "appearance.color.braces_d",
            Self::Comments => "appearance.color.comments_d",
            Self::FoldAccent => "appearance.color.fold_accent_d",
            Self::BracketMatch => "appearance.color.bracket_match_d",
        }
    }
}

/// `AppearanceThemePreset` — twelve palettes in the Swift role order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AppearanceThemePreset {
    Light,
    Dark,
    Monokai,
    Dracula,
    Nord,
    OneDark,
    SolarizedDark,
    SolarizedLight,
    Gruvbox,
    CatppuccinLatte,
    CatppuccinFrappe,
    CatppuccinMacchiato,
    CatppuccinMocha,
}
impl AppearanceThemePreset {
    pub const ALL: [Self; 13] = [
        Self::Light,
        Self::Dark,
        Self::Monokai,
        Self::Dracula,
        Self::Nord,
        Self::OneDark,
        Self::SolarizedDark,
        Self::SolarizedLight,
        Self::Gruvbox,
        Self::CatppuccinLatte,
        Self::CatppuccinFrappe,
        Self::CatppuccinMacchiato,
        Self::CatppuccinMocha,
    ];
    pub fn title(self) -> &'static str {
        match self {
            Self::Light => "Light",
            Self::Dark => "Dark",
            Self::Monokai => "Monokai",
            Self::Dracula => "Dracula",
            Self::Nord => "Nord",
            Self::OneDark => "One Dark",
            Self::SolarizedDark => "Solarized Dark",
            Self::SolarizedLight => "Solarized Light",
            Self::Gruvbox => "Gruvbox",
            Self::CatppuccinLatte => "Catppuccin Latte",
            Self::CatppuccinFrappe => "Catppuccin Frappé",
            Self::CatppuccinMacchiato => "Catppuccin Macchiato",
            Self::CatppuccinMocha => "Catppuccin Mocha",
        }
    }
    pub fn is_dark(self) -> bool {
        !matches!(self, Self::Light | Self::SolarizedLight | Self::CatppuccinLatte)
    }
    fn palette(self) -> [&'static str; 11] {
        match self {
            Self::Light => [
                "#FFFFFF", "#F6F8FA", "#BABBBD", "#24292E", "#D73A49", "#6F42C1",
                "#005CC5", "#24292E", "#6A737D", "#0366D6", "#0366D622",
            ],
            Self::Dark => [
                "#2C2C33", "#3C3D42", "#9E9E9E", "#E0E0DA", "#77C3F3", "#9FD790",
                "#D0A5E3", "#9E9D92", "#929D92", "#FF902F", "#6DB6E566",
            ],
            Self::Monokai => [
                "#272822", "#2E2F28", "#90908A", "#F8F8F2", "#F92672", "#A6E22E",
                "#AE81FF", "#F8F8F2", "#75715E", "#66D9EF", "#66D9EF33",
            ],
            Self::Dracula => [
                "#282A36", "#21222C", "#6272A4", "#F8F8F2", "#FF79C6", "#8BE9FD",
                "#BD93F9", "#F8F8F2", "#6272A4", "#BD93F9", "#BD93F933",
            ],
            Self::Nord => [
                "#2E3440", "#3B4252", "#4C566A", "#D8DEE9", "#81A1C1", "#8FBCBB",
                "#B48EAD", "#ECEFF4", "#616E88", "#88C0D0", "#88C0D033",
            ],
            Self::OneDark => [
                "#282C34", "#2C313A", "#4B5263", "#ABB2BF", "#C678DD", "#E5C07B",
                "#D19A66", "#ABB2BF", "#5C6370", "#61AFEF", "#61AFEF33",
            ],
            Self::SolarizedDark => [
                "#002B36", "#073642", "#586E75", "#839496", "#268BD2", "#2AA198",
                "#D33682", "#93A1A1", "#586E75", "#B58900", "#268BD233",
            ],
            Self::SolarizedLight => [
                "#FDF6E3", "#EEE8D5", "#93A1A1", "#657B83", "#268BD2", "#2AA198",
                "#D33682", "#586E75", "#93A1A1", "#B58900", "#268BD226",
            ],
            Self::Gruvbox => [
                "#282828", "#3C3836", "#7C6F64", "#EBDBB2", "#FB4934", "#8EC07C",
                "#D3869B", "#EBDBB2", "#928374", "#FE8019", "#83A59833",
            ],
            Self::CatppuccinLatte => [
                "#EFF1F5", "#E6E9EF", "#8C8FA1", "#4C4F69", "#1E66F5", "#40A02B",
                "#8839EF", "#DF8E1D", "#7C7F93", "#FE640B", "#1E66F533",
            ],
            Self::CatppuccinFrappe => [
                "#303446", "#292C3C", "#737994", "#C6D0F5", "#8CAAEE", "#A6D189",
                "#CA9EE6", "#E5C890", "#949CBB", "#EF9F76", "#8CAAEE33",
            ],
            Self::CatppuccinMacchiato => [
                "#24273A", "#1E2030", "#6E738D", "#CAD3F5", "#8AADF4", "#A6DA95",
                "#C6A0F6", "#EED49F", "#939AB7", "#F5A97F", "#8AADF433",
            ],
            Self::CatppuccinMocha => [
                "#1E1E2E", "#181825", "#6C7086", "#CDD6F4", "#89B4FA", "#A6E3A1",
                "#CBA6F7", "#F9E2AF", "#9399B2", "#FAB387", "#89B4FA33",
            ],
        }
    }
    pub fn hex_for(&self, role: &AppearanceColorRole) -> &'static str {
        let palette = self.palette();
        palette[AppearanceColorRole::ALL.iter().position(|r| r == role).unwrap()]
    }
}

/// Parse `#RRGGBB` / `#RRGGBBAA` into (r,g,b,a) 0–1 floats.
pub fn parse_hex_color(hex: &str) -> Option<(f64, f64, f64, f64)> {
    let h = hex.trim().trim_start_matches('#');
    if h.len() != 6 && h.len() != 8 {
        return None;
    }
    let mut value = u64::from_str_radix(h, 16).ok()?;
    let mut alpha = 255u64;
    if h.len() == 8 {
        alpha = value & 0xFF;
        value >>= 8;
    }
    Some((
        ((value >> 16) & 0xFF) as f64 / 255.0,
        ((value >> 8) & 0xFF) as f64 / 255.0,
        (value & 0xFF) as f64 / 255.0,
        alpha as f64 / 255.0,
    ))
}

/// `NSColor.hexString` — "#RRGGBB", or "#RRGGBBAA" when alpha < 255.
pub fn rgba_to_hex_string(r: f64, g: f64, b: f64, a: f64) -> String {
    let clamp = |v: f64| (v.clamp(0.0, 1.0) * 255.0).round() as u32;
    let (r, g, b, a) = (clamp(r), clamp(g), clamp(b), clamp(a));
    if a < 255 {
        format!("#{r:02X}{g:02X}{b:02X}{a:02X}")
    } else {
        format!("#{r:02X}{g:02X}{b:02X}")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Theme {
    System,
    Light,
    Dark,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppLanguage {
    System,
    En,
    Ko,
    Ja,
    Vi,
    Ru,
    ZhHans,
    Es,
}
impl AppLanguage {
    pub fn raw_value(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::En => "en",
            Self::Ko => "ko",
            Self::Ja => "ja",
            Self::Vi => "vi",
            Self::Ru => "ru",
            Self::ZhHans => "zh-Hans",
            Self::Es => "es",
        }
    }
    pub fn from_raw(v: &str) -> Self {
        match v {
            "en" => Self::En,
            "ko" => Self::Ko,
            "ja" => Self::Ja,
            "vi" => Self::Vi,
            "ru" => Self::Ru,
            "zh-Hans" => Self::ZhHans,
            "es" => Self::Es,
            _ => Self::System,
        }
    }
}

/// Port of `AppearanceSettings` — same keys and behavior.
pub struct AppearanceSettings {
    pub theme: Theme,
    pub language: AppLanguage,
    pub launched_language: AppLanguage,
    pub font_family: String,
    pub font_size: f64,
    pub terminal_font_family: String,
    pub terminal_font_size: f64,
    pub color_revision: u64,
    /// Last effective dark state observed for `Theme::System`, refreshed by
    /// `apply_theme` from `adw::StyleManager`. A `Cell` so the GTK-free
    /// model stays `&self`-readable while the shell updates it in place.
    pub resolved_dark: std::cell::Cell<bool>,
}
impl AppearanceSettings {
    pub fn new(prefs: &Preferences) -> Self {
        let language = AppLanguage::from_raw(
            &prefs.string("appearance.language").unwrap_or_else(|| "system".into()),
        );
        let theme = match prefs.string("appearance.theme").as_deref() {
            Some("light") => Theme::Light,
            Some("dark") => Theme::Dark,
            _ => Theme::System,
        };
        Self {
            theme,
            language,
            launched_language: language,
            font_family: prefs.string("appearance.fontFamily").unwrap_or_default(),
            font_size: prefs.double("appearance.fontSize").unwrap_or(13.0),
            terminal_font_family: prefs
                .string("appearance.terminalFontFamily")
                .unwrap_or_default(),
            terminal_font_size: prefs
                .double("appearance.terminalFontSize")
                .unwrap_or(13.0),
            color_revision: 0,
            resolved_dark: std::cell::Cell::new(true),
        }
    }
    /// `effectiveDark` — `Light`/`Dark` pin the answer; `System` resolves to
    /// the last value `apply_theme` read from the style manager.
    pub fn effective_dark(&self) -> bool {
        match self.theme {
            Theme::Light => false,
            Theme::Dark => true,
            Theme::System => self.resolved_dark.get(),
        }
    }
    pub fn language_restart_pending(&self) -> bool {
        self.language != self.launched_language
    }
    pub fn stored_hex(&self, prefs: &Preferences, role: AppearanceColorRole) -> String {
        prefs
            .string(&format!("appearance.color.{}", role.raw_value()))
            .unwrap_or_else(|| role.default_hex(self.effective_dark()).to_string())
    }
    pub fn color(&self, prefs: &Preferences, role: AppearanceColorRole) -> (f64, f64, f64, f64) {
        prefs
            .string(&format!("appearance.color.{}", role.raw_value()))
            .and_then(|h| parse_hex_color(&h))
            .or_else(|| parse_hex_color(role.default_hex(self.effective_dark())))
            .unwrap_or((0.0, 0.0, 0.0, 1.0))
    }
    pub fn matching_preset(&self, prefs: &Preferences) -> Option<AppearanceThemePreset> {
        AppearanceThemePreset::ALL.iter().copied().find(|preset| {
            AppearanceColorRole::ALL.iter().all(|role| {
                self.stored_hex(prefs, *role).eq_ignore_ascii_case(preset.hex_for(role))
            })
        })
    }
    pub fn apply_preset(&mut self, prefs: &mut Preferences, preset: AppearanceThemePreset) {
        for role in AppearanceColorRole::ALL {
            prefs.set(
                &format!("appearance.color.{}", role.raw_value()),
                preset.hex_for(&role),
            );
        }
        self.theme = if preset.is_dark() { Theme::Dark } else { Theme::Light };
        prefs.set("appearance.theme", if preset.is_dark() { "dark" } else { "light" });
        self.color_revision += 1;
    }
    pub fn set_color(&mut self, prefs: &mut Preferences, role: AppearanceColorRole, hex: &str) {
        prefs.set(&format!("appearance.color.{}", role.raw_value()), hex);
        self.color_revision += 1;
    }
    pub fn set_theme(&mut self, prefs: &mut Preferences, theme: Theme) {
        self.theme = theme;
        prefs.set(
            "appearance.theme",
            match theme {
                Theme::System => "system",
                Theme::Light => "light",
                Theme::Dark => "dark",
            },
        );
    }
    pub fn set_language(&mut self, prefs: &mut Preferences, language: AppLanguage) {
        self.language = language;
        prefs.set("appearance.language", language.raw_value());
    }
    pub fn set_font(&mut self, prefs: &mut Preferences, family: &str, size: f64) {
        self.font_family = family.to_string();
        self.font_size = size;
        prefs.set("appearance.fontFamily", family);
        prefs.set("appearance.fontSize", size);
    }
    pub fn set_terminal_font(&mut self, prefs: &mut Preferences, family: &str, size: f64) {
        self.terminal_font_family = family.to_string();
        self.terminal_font_size = size;
        prefs.set("appearance.terminalFontFamily", family);
        prefs.set("appearance.terminalFontSize", size);
    }
    /// Re-read after a settings import — mirrors `AppearanceSettings.reload`
    /// on macOS: `launched_language` stays so the restart hint still works.
    pub fn reload(&mut self, prefs: &Preferences) {
        self.language = AppLanguage::from_raw(
            &prefs.string("appearance.language").unwrap_or_else(|| "system".into()),
        );
        self.theme = match prefs.string("appearance.theme").as_deref() {
            Some("light") => Theme::Light,
            Some("dark") => Theme::Dark,
            _ => Theme::System,
        };
        self.font_family = prefs.string("appearance.fontFamily").unwrap_or_default();
        self.font_size = prefs.double("appearance.fontSize").unwrap_or(13.0);
        self.terminal_font_family = prefs
            .string("appearance.terminalFontFamily")
            .unwrap_or_default();
        self.terminal_font_size = prefs
            .double("appearance.terminalFontSize")
            .unwrap_or(13.0);
        self.color_revision += 1;
    }
    /// Font description for Pango; empty family = system monospace.
    pub fn font_description(&self) -> String {
        if self.font_family.is_empty() {
            format!("Monospace {}", self.font_size.max(1.0) as i64)
        } else {
            format!("{} {}", self.font_family, self.font_size.max(1.0) as i64)
        }
    }
    /// Terminal font description; empty family = system monospace (the
    /// "follow editor" fallback lives in `AppState::terminal_font_desc`).
    pub fn terminal_font_description(&self) -> String {
        if self.terminal_font_family.is_empty() {
            format!("Monospace {}", self.terminal_font_size.max(1.0) as i64)
        } else {
            format!(
                "{} {}",
                self.terminal_font_family,
                self.terminal_font_size.max(1.0) as i64
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use settings_feature::{BuildPreferences, ShellExecutionPreference};

    /// `NSColor.hexString` — opaque colors serialize as #RRGGBB, translucent
    /// ones carry the alpha suffix like the reference editor stores them.
    #[test]
    fn rgba_to_hex_string_matches_swift_format() {
        assert_eq!(rgba_to_hex_string(0.4, 0.85, 0.94, 1.0), "#66D9F0");
        assert_eq!(rgba_to_hex_string(0.4, 0.85, 0.94, 0.2), "#66D9F033");
        assert_eq!(rgba_to_hex_string(0.0, 0.0, 0.0, 1.0), "#000000");
        assert_eq!(rgba_to_hex_string(1.0, 1.0, 1.0, 1.0), "#FFFFFF");
    }

    /// Round-trip through `parse_hex_color` for both storage formats.
    #[test]
    fn hex_color_round_trip() {
        for hex in ["#66D9F0", "#66D9F033", "#1E1E2E", "#FFFFFF"] {
            let (r, g, b, a) = parse_hex_color(hex).expect("parses");
            assert_eq!(rgba_to_hex_string(r, g, b, a), hex);
        }
        assert!(parse_hex_color("#FFF").is_none());
        assert!(parse_hex_color("not-a-color").is_none());
    }

    /// Every role exposes both localization keys, matching `titleKey`/`detailKey`.
    #[test]
    fn color_role_localization_keys() {
        let titles: std::collections::HashSet<_> = AppearanceColorRole::ALL
            .iter()
            .map(|r| r.title_key())
            .collect();
        let details: std::collections::HashSet<_> = AppearanceColorRole::ALL
            .iter()
            .map(|r| r.detail_key())
            .collect();
        assert_eq!(titles.len(), AppearanceColorRole::ALL.len());
        assert_eq!(details.len(), AppearanceColorRole::ALL.len());
        assert!(titles.contains("appearance.color.editor_background"));
        assert!(details.contains("appearance.color.bracket_match_d"));
    }

    /// Unset role colors default to the effective mode's palette — light
    /// mode gets Light, dark gets Dark, System resolves `resolved_dark`.
    /// An explicit stored value always wins.
    #[test]
    fn default_palette_follows_effective_mode() {
        let prefs = Preferences {
            values: BTreeMap::new(),
            path: std::env::temp_dir().join("pitex-test-palette-defaults.json"),
        };
        let mut a = AppearanceSettings::new(&prefs);
        assert_eq!(a.theme, Theme::System);
        // System starts resolved-dark until apply_theme observes the OS.
        assert_eq!(
            a.stored_hex(&prefs, AppearanceColorRole::EditorBackground),
            "#2C2C33"
        );
        a.resolved_dark.set(false);
        assert_eq!(
            a.stored_hex(&prefs, AppearanceColorRole::EditorBackground),
            "#FFFFFF"
        );
        let mut prefs = prefs;
        a.set_theme(&mut prefs, Theme::Light);
        assert_eq!(
            a.stored_hex(&prefs, AppearanceColorRole::EditorBackground),
            "#FFFFFF"
        );
        a.set_theme(&mut prefs, Theme::Dark);
        assert_eq!(
            a.stored_hex(&prefs, AppearanceColorRole::EditorBackground),
            "#2C2C33"
        );
        // Explicit colors survive any mode.
        a.set_color(&mut prefs, AppearanceColorRole::EditorBackground, "#123456");
        a.set_theme(&mut prefs, Theme::Light);
        assert_eq!(
            a.stored_hex(&prefs, AppearanceColorRole::EditorBackground),
            "#123456"
        );
        let _ = std::fs::remove_file(&prefs.path);
    }

    /// `languageRestartPending` — set_language marks the delta against the
    /// launched language.
    #[test]
    fn language_restart_pending_tracks_launched_language() {
        let mut prefs = Preferences {
            values: BTreeMap::new(),
            path: std::env::temp_dir().join("pitex-test-preferences.json"),
        };
        let mut appearance = AppearanceSettings::new(&prefs);
        assert!(!appearance.language_restart_pending());
        appearance.set_language(&mut prefs, AppLanguage::Ko);
        assert!(appearance.language_restart_pending());
        let launched = appearance.launched_language;
        appearance.set_language(&mut prefs, launched);
        assert!(!appearance.language_restart_pending());
        let _ = std::fs::remove_file(&prefs.path);
    }

    /// `inverseSyncHighlight`/`forwardSyncHighlight` — independent keys,
    /// both defaulting off, persisted through the JSON store.
    #[test]
    fn sync_highlight_preferences_default_off_and_persist() {
        let path = std::env::temp_dir().join(format!(
            "pitex-prefs-sync-{}.json",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        let mut store = SettingsStore::new(Preferences {
            values: BTreeMap::new(),
            path: path.clone(),
        });
        assert!(!store.inverse_sync_highlight());
        assert!(!store.forward_sync_highlight());
        store.set_inverse_sync_highlight(true);
        store.set_forward_sync_highlight(true);
        // A fresh store over the same file sees the persisted values.
        let values: BTreeMap<String, serde_json::Value> =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        let store = SettingsStore::new(Preferences {
            values,
            path: path.clone(),
        });
        assert!(store.inverse_sync_highlight());
        assert!(store.forward_sync_highlight());
        let _ = std::fs::remove_file(&path);
    }

    /// `pitex.pref.markdown.*` — live preview + sync scroll default on,
    /// font size clamps to 10…28, and an unknown theme falls back to
    /// "system".
    #[test]
    fn markdown_preferences_defaults_persist_and_fallback() {
        let path = std::env::temp_dir().join(format!(
            "pitex-prefs-markdown-{}.json",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        let mut store = SettingsStore::new(Preferences {
            values: BTreeMap::new(),
            path: path.clone(),
        });
        assert!(store.markdown_live_preview());
        assert!(store.markdown_sync_scroll());
        assert_eq!(store.markdown_font_size(), 16.0);
        assert_eq!(store.markdown_theme(), "system");
        store.set_markdown_live_preview(false);
        store.set_markdown_sync_scroll(false);
        store.set_markdown_font_size(99.0);
        store.set_markdown_theme("dark");
        // A fresh store over the same file sees the persisted values.
        let values: BTreeMap<String, serde_json::Value> =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        let mut store = SettingsStore::new(Preferences {
            values,
            path: path.clone(),
        });
        assert!(!store.markdown_live_preview());
        assert!(!store.markdown_sync_scroll());
        assert_eq!(store.markdown_font_size(), 28.0);
        assert_eq!(store.markdown_theme(), "dark");
        // An unknown stored theme resolves to "system".
        store.set_markdown_theme("sepia");
        assert_eq!(store.markdown_theme(), "system");
        let _ = std::fs::remove_file(&path);
    }

    /// `customCommandBinding` semantics — an empty field disables the
    /// execution and clears the acknowledgement; a command preserves it.
    #[test]
    fn shell_execution_binding_semantics() {
        // `selecting_shell_execution` clears the acknowledgement on every
        // selection change — the settings layer's own guard.
        let custom = ShellExecutionPreference::Custom {
            command: "make".into(),
        };
        let build = BuildPreferences::safe_defaults()
            .selecting_shell_execution(custom.clone())
            .unwrap();
        assert!(!build.custom_shell_acknowledged);
        // `acknowledging_custom_shell` latches only while `.custom` is set.
        let mut build = build.acknowledging_custom_shell().unwrap();
        assert!(build.custom_shell_acknowledged);
        build.shell_execution = ShellExecutionPreference::Disabled;
        let build = BuildPreferences::new(
            build.engine,
            build.maximum_passes,
            build.stops_after_first_error,
            build.shell_execution.clone(),
            false,
        )
        .unwrap();
        assert!(!build.custom_shell_acknowledged);
    }
}
