//! Pitex — native Ubuntu 24.04 port of the macOS LaTeX environment.
//!
//! GUI entry point: `pitex_shell::run()`. The workspace model, SyncTeX
//! runner, agent coordinator, settings store, PDF renderer and localization
//! tables are platform-neutral; `app_ui` binds them to GTK4/libadwaita.

pub mod agent;
pub mod app_ui;
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
