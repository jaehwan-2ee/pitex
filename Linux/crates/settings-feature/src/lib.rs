//! Rust port of `Packages/TexApp/Sources/SettingsFeature`.

use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingsValidationError {
    EditorFontSizeOutOfRange,
    EditorTabWidthOutOfRange,
    BuildPassLimitOutOfRange,
    EmptyCustomShell,
    InvalidCustomShell,
}
impl fmt::Display for SettingsValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for SettingsValidationError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SettingsKey {
    Editor,
    Build,
    Pdf,
    DiagnosticsConsent,
    Project,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EditorPreferences {
    pub font_size: u8,
    pub tab_width: u8,
    pub wraps_lines: bool,
    pub completes_delimiters: bool,
}
impl EditorPreferences {
    pub fn new(
        font_size: u8,
        tab_width: u8,
        wraps_lines: bool,
        completes_delimiters: bool,
    ) -> Result<Self, SettingsValidationError> {
        if !(8..=72).contains(&font_size) {
            return Err(SettingsValidationError::EditorFontSizeOutOfRange);
        }
        if !(1..=16).contains(&tab_width) {
            return Err(SettingsValidationError::EditorTabWidthOutOfRange);
        }
        Ok(Self { font_size, tab_width, wraps_lines, completes_delimiters })
    }
    pub fn safe_defaults() -> Self {
        Self::new(14, 4, true, true).unwrap()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TeXEnginePreference {
    PdfLaTeX,
    XeLaTeX,
    LuaLaTeX,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ShellExecutionPreference {
    Disabled,
    Custom { command: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildPreferences {
    pub engine: TeXEnginePreference,
    pub maximum_passes: u8,
    pub stops_after_first_error: bool,
    pub shell_execution: ShellExecutionPreference,
    pub custom_shell_acknowledged: bool,
}
impl BuildPreferences {
    pub fn new(
        engine: TeXEnginePreference,
        maximum_passes: u8,
        stops_after_first_error: bool,
        shell_execution: ShellExecutionPreference,
        custom_shell_acknowledged: bool,
    ) -> Result<Self, SettingsValidationError> {
        if !(1..=10).contains(&maximum_passes) {
            return Err(SettingsValidationError::BuildPassLimitOutOfRange);
        }
        if let ShellExecutionPreference::Custom { command } = &shell_execution {
            if command.is_empty() {
                return Err(SettingsValidationError::EmptyCustomShell);
            }
            if command.contains('\0') {
                return Err(SettingsValidationError::InvalidCustomShell);
            }
        }
        Ok(Self {
            engine,
            maximum_passes,
            stops_after_first_error,
            shell_execution,
            custom_shell_acknowledged,
        })
    }
    pub fn safe_defaults() -> Self {
        Self::new(
            TeXEnginePreference::PdfLaTeX,
            3,
            false,
            ShellExecutionPreference::Disabled,
            false,
        )
        .unwrap()
    }
    pub fn selecting_shell_execution(
        &self,
        selection: ShellExecutionPreference,
    ) -> Result<Self, SettingsValidationError> {
        Self::new(
            self.engine,
            self.maximum_passes,
            self.stops_after_first_error,
            selection,
            false,
        )
    }
    pub fn acknowledging_custom_shell(&self) -> Result<Self, SettingsValidationError> {
        let acknowledged = matches!(self.shell_execution, ShellExecutionPreference::Custom { .. });
        Self::new(
            self.engine,
            self.maximum_passes,
            self.stops_after_first_error,
            self.shell_execution.clone(),
            acknowledged,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PDFPageLayout {
    SinglePage,
    Continuous,
    TwoUp,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PDFPreferences {
    pub auto_reload_after_build: bool,
    pub highlights_sync_location: bool,
    pub page_layout: PDFPageLayout,
}
impl PDFPreferences {
    pub fn new(
        auto_reload_after_build: bool,
        highlights_sync_location: bool,
        page_layout: PDFPageLayout,
    ) -> Self {
        Self { auto_reload_after_build, highlights_sync_location, page_layout }
    }
    pub fn safe_defaults() -> Self {
        Self::new(true, true, PDFPageLayout::Continuous)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ConsentPreference {
    NotGranted,
    Granted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticsConsentPreferences {
    pub evidence_logging: ConsentPreference,
    pub debug_logging: ConsentPreference,
}
impl DiagnosticsConsentPreferences {
    pub fn new(evidence_logging: ConsentPreference, debug_logging: ConsentPreference) -> Self {
        Self { evidence_logging, debug_logging }
    }
    pub fn safe_defaults() -> Self {
        Self::new(ConsentPreference::NotGranted, ConsentPreference::NotGranted)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RootFileBehavior {
    AskEveryTime,
    UseLastSuccessful,
    DiscoverFromDocument,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectPreferences {
    pub restores_last_project: bool,
    pub autosaves_documents: bool,
    pub root_file_behavior: RootFileBehavior,
}
impl ProjectPreferences {
    pub fn new(
        restores_last_project: bool,
        autosaves_documents: bool,
        root_file_behavior: RootFileBehavior,
    ) -> Self {
        Self { restores_last_project, autosaves_documents, root_file_behavior }
    }
    pub fn safe_defaults() -> Self {
        Self::new(false, true, RootFileBehavior::AskEveryTime)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistedSettings {
    pub editor: EditorPreferences,
    pub build: BuildPreferences,
    pub pdf: PDFPreferences,
    pub diagnostics_consent: DiagnosticsConsentPreferences,
    pub project: ProjectPreferences,
}
impl PersistedSettings {
    pub fn safe_defaults() -> Self {
        Self {
            editor: EditorPreferences::safe_defaults(),
            build: BuildPreferences::safe_defaults(),
            pdf: PDFPreferences::safe_defaults(),
            diagnostics_consent: DiagnosticsConsentPreferences::safe_defaults(),
            project: ProjectPreferences::safe_defaults(),
        }
    }
}
impl Default for PersistedSettings {
    fn default() -> Self {
        Self::safe_defaults()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CapabilityAvailability {
    Available,
    Unavailable { reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsRuntimeStatus {
    pub build_capability: CapabilityAvailability,
    pub pdf_capability: CapabilityAvailability,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsState {
    pub preferences: PersistedSettings,
    pub runtime: SettingsRuntimeStatus,
}
