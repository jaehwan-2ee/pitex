//! Pitex — native Linux/Windows port of the macOS LaTeX environment
//! (Ubuntu 24.04 full build; 22.04 via `--no-default-features` and Windows
//! via `modern-gtk` without `vte`, see `compat`).
//!
//! GUI entry point: `pitex_shell::run()`. The workspace model, SyncTeX
//! runner, agent coordinator, settings store, PDF renderer and localization
//! tables are platform-neutral; `app_ui` binds them to GTK4/libadwaita.

pub mod agent;
pub mod app_ui;
pub mod compat;
pub mod completion;
pub mod fold;
pub mod ghost_completion;
pub mod git;
mod git_list;
pub mod l10n;
pub mod model;
pub mod panes;
pub mod pdf;
pub mod settings;
pub mod synctex;
pub mod update;

// Unit tests that mutate/read the process PATH must not overlap.
#[cfg(test)]
pub(crate) static TEST_ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Launch the native application (GTK4 main loop). `app_version` is the
/// semantic version the updater compares against release tags.
pub fn run(app_version: &str) -> i32 {
    app_ui::run(app_version)
}
