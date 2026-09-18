//! Pitex — native Ubuntu port of the macOS LaTeX environment (24.04 full
//! build; 22.04 via `--no-default-features`, see `compat`).
//!
//! GUI entry point: `pitex_shell::run()`. The workspace model, SyncTeX
//! runner, agent coordinator, settings store, PDF renderer and localization
//! tables are platform-neutral; `app_ui` binds them to GTK4/libadwaita.

pub mod agent;
pub mod app_ui;
pub mod compat;
pub mod fold;
pub mod l10n;
pub mod model;
pub mod panes;
pub mod pdf;
pub mod settings;
pub mod synctex;

/// Launch the native application (GTK4 main loop).
pub fn run() -> i32 {
    app_ui::run()
}
