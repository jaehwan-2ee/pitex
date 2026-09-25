//! Build-variant compatibility shims.
//!
//! Two orthogonal features select the surface:
//!
//! - `modern-gtk` — GTK 4.10+ / libadwaita 1.4+ APIs (file `add_suffix`,
//!   `ContentFit`, `FontDialogButton`, `ColorDialogButton`). Off for the
//!   Ubuntu 22.04 build (GTK 4.6 / libadwaita 1.1), on everywhere else.
//! - `vte` — the embedded VTE terminal. On for Unix desktop builds; off on
//!   Ubuntu 22.04 and Windows, which embed the terminal-core DrawingArea
//!   instead (same API). An external emulator (kgx / gnome-terminal / konsole
//!   / xterm on Linux, `wt` / conhost on Windows) is only the fallback for a
//!   shell that failed to spawn or already exited.

use gtk4::prelude::*;
use libadwaita as adw;
use adw::prelude::*;
#[cfg(feature = "vte")]
use vte4::prelude::*;
use std::path::{Path, PathBuf};

// ─── Settings rows (identical UX on every libadwaita ≥ 1.0) ─────────────

/// `adw::SwitchRow` equivalent — ActionRow + Switch suffix works on
/// libadwaita 1.1, where SwitchRow (1.4) doesn't exist yet.
pub fn switch_row(title: &str) -> (adw::ActionRow, gtk4::Switch) {
    let row = adw::ActionRow::new();
    row.set_title(title);
    let switch = gtk4::Switch::new();
    switch.set_valign(gtk4::Align::Center);
    row.add_suffix(&switch);
    row.set_activatable_widget(Some(&switch));
    (row, switch)
}

/// `adw::EntryRow` equivalent — the suffix entry takes the row's trailing
/// space and expands so long commands stay editable.
pub fn entry_row(title: &str) -> (adw::ActionRow, gtk4::Entry) {
    let row = adw::ActionRow::new();
    row.set_title(title);
    let entry = gtk4::Entry::new();
    entry.set_valign(gtk4::Align::Center);
    entry.set_hexpand(true);
    row.add_suffix(&entry);
    row.set_activatable_widget(Some(&entry));
    (row, entry)
}

/// `adw::SpinRow` equivalent — ActionRow + SpinButton suffix.
pub fn spin_row(title: &str, min: f64, max: f64, step: f64) -> (adw::ActionRow, gtk4::SpinButton) {
    let row = adw::ActionRow::new();
    row.set_title(title);
    let spin = gtk4::SpinButton::with_range(min, max, step);
    spin.set_valign(gtk4::Align::Center);
    row.add_suffix(&spin);
    row.set_activatable_widget(Some(&spin));
    (row, spin)
}

/// Set fixed help text before mounting. GTK 4.14 queries the X11 pointer for
/// any visible widget, even an unparented one. Keep it hidden during the setter
/// so the native tooltip/accessibility properties are stored without that query.
pub fn initial_tooltip(widget: &impl IsA<gtk4::Widget>, text: &str) {
    debug_assert!(widget.root().is_none());
    let visible = widget.is_visible();
    widget.set_visible(false);
    widget.set_tooltip_text(Some(text));
    widget.set_visible(visible);
}

/// Resolve changing help text only when GTK requests a tooltip, avoiding the
/// synchronous X11 pointer query triggered by updating tooltip-text.
pub fn dynamic_tooltip(widget: &impl IsA<gtk4::Widget>, text: impl Fn() -> Option<String> + 'static) {
    widget.connect_query_tooltip(move |_, _, _, _, tooltip| {
        let Some(text) = text() else { return false };
        tooltip.set_text(Some(&text));
        true
    });
    widget.set_has_tooltip(true);
}

// ─── File dialogs ────────────────────────────────────────────────────────
// Newer GTK (24.04+) uses the portal-backed `FileDialog`, which doesn't exist
// on Ubuntu 22.04 (GTK 4.6). The compat path uses `FileChooserDialog` rather
// than `FileChooserNative`: FileChooserNative's fallback never maps a real
// window when no usable xdg-desktop-portal is around, so Open/Save appeared
// to do nothing.

/// Single file picker filtered to TeX/BibTeX sources.
pub fn pick_source_file(
    window: Option<&gtk4::Window>,
    title: &str,
    on_path: impl Fn(PathBuf) + 'static,
) {
    #[cfg(feature = "modern-gtk")]
    {
        let dialog = gtk4::FileDialog::new();
        dialog.set_title(title);
        dialog.set_modal(true);
        let filter = gtk4::FileFilter::new();
        filter.add_suffix("tex");
        filter.add_suffix("bib");
        filter.add_suffix("md");
        filter.add_suffix("markdown");
        let filters = gtk4::gio::ListStore::new::<gtk4::FileFilter>();
        filters.append(&filter);
        dialog.set_filters(Some(&filters));
        dialog.set_default_filter(Some(&filter));
        dialog.open(window, gtk4::gio::Cancellable::NONE, move |result| {
            if let Ok(file) = result {
                if let Some(path) = file.path() {
                    on_path(path);
                }
            }
        });
        return;
    }
    #[cfg(not(feature = "modern-gtk"))]
    {
        let dialog = chooser_dialog(window, title, gtk4::FileChooserAction::Open, "Open");
        let filter = gtk4::FileFilter::new();
        filter.add_pattern("*.tex");
        filter.add_pattern("*.bib");
        filter.add_pattern("*.md");
        filter.add_pattern("*.markdown");
        dialog.add_filter(&filter);
        dialog.connect_response(move |d, response| {
            if response == gtk4::ResponseType::Accept {
                if let Some(path) = d.file().and_then(|f| f.path()) {
                    on_path(path);
                }
            }
            d.destroy();
        });
        dialog.present();
    }
}

/// Multi-select picker (attachment insertion) — returns every selection.
pub fn pick_files(
    window: Option<&gtk4::Window>,
    title: &str,
    on_paths: impl Fn(Vec<PathBuf>) + 'static,
) {
    #[cfg(feature = "modern-gtk")]
    {
        let dialog = gtk4::FileDialog::new();
        dialog.set_title(title);
        dialog.set_modal(true);
        dialog.open_multiple(window, gtk4::gio::Cancellable::NONE, move |result| {
            let Ok(files) = result else { return };
            let paths: Vec<PathBuf> = (0..files.n_items())
                .filter_map(|i| files.item(i).and_then(|o| o.downcast::<gtk4::gio::File>().ok()))
                .filter_map(|f| f.path())
                .collect();
            on_paths(paths);
        });
        return;
    }
    #[cfg(not(feature = "modern-gtk"))]
    {
        let dialog = chooser_dialog(window, title, gtk4::FileChooserAction::Open, "Open");
        dialog.set_select_multiple(true);
        dialog.connect_response(move |d, response| {
            if response == gtk4::ResponseType::Accept {
                let files = d.files();
                let paths: Vec<PathBuf> = (0..files.n_items())
                    .filter_map(|i| {
                        files
                            .item(i)
                            .and_then(|o| o.downcast::<gtk4::gio::File>().ok())
                    })
                    .filter_map(|f| f.path())
                    .collect();
                on_paths(paths);
            }
            d.destroy();
        });
        dialog.present();
    }
}

/// Single `.json` picker (settings import) — same dual-path pattern as
/// `pick_source_file` but filtered to JSON with an all-files fallback.
pub fn pick_settings_file(
    window: Option<&gtk4::Window>,
    title: &str,
    on_path: impl Fn(PathBuf) + 'static,
) {
    #[cfg(feature = "modern-gtk")]
    {
        let dialog = gtk4::FileDialog::new();
        dialog.set_title(title);
        dialog.set_modal(true);
        let filter = gtk4::FileFilter::new();
        filter.add_suffix("json");
        let filters = gtk4::gio::ListStore::new::<gtk4::FileFilter>();
        filters.append(&filter);
        dialog.set_filters(Some(&filters));
        dialog.open(window, gtk4::gio::Cancellable::NONE, move |result| {
            if let Ok(file) = result {
                if let Some(path) = file.path() {
                    on_path(path);
                }
            }
        });
        return;
    }
    #[cfg(not(feature = "modern-gtk"))]
    {
        let dialog = chooser_dialog(window, title, gtk4::FileChooserAction::Open, "Open");
        let filter = gtk4::FileFilter::new();
        filter.add_pattern("*.json");
        dialog.add_filter(&filter);
        dialog.connect_response(move |d, response| {
            if response == gtk4::ResponseType::Accept {
                if let Some(path) = d.file().and_then(|f| f.path()) {
                    on_path(path);
                }
            }
            d.destroy();
        });
        dialog.present();
    }
}

/// Folder picker (project open).
pub fn pick_folder(
    window: Option<&gtk4::Window>,
    title: &str,
    on_path: impl Fn(PathBuf) + 'static,
) {
    #[cfg(feature = "modern-gtk")]
    {
        let dialog = gtk4::FileDialog::new();
        dialog.set_title(title);
        dialog.set_modal(true);
        dialog.select_folder(window, gtk4::gio::Cancellable::NONE, move |result| {
            if let Ok(file) = result {
                if let Some(path) = file.path() {
                    on_path(path);
                }
            }
        });
        return;
    }
    #[cfg(not(feature = "modern-gtk"))]
    {
        let dialog = chooser_dialog(window, title, gtk4::FileChooserAction::SelectFolder, "Open");
        dialog.connect_response(move |d, response| {
            if response == gtk4::ResponseType::Accept {
                if let Some(path) = d.file().and_then(|f| f.path()) {
                    on_path(path);
                }
            }
            d.destroy();
        });
        dialog.present();
    }
}

/// Save-as dialog with an optional suggested name.
pub fn save_file(
    window: Option<&gtk4::Window>,
    title: &str,
    initial_name: Option<&str>,
    on_path: impl Fn(PathBuf) + 'static,
) {
    #[cfg(feature = "modern-gtk")]
    {
        let dialog = gtk4::FileDialog::new();
        dialog.set_title(title);
        dialog.set_modal(true);
        if let Some(name) = initial_name {
            dialog.set_initial_name(Some(name));
        }
        dialog.save(window, gtk4::gio::Cancellable::NONE, move |result| {
            if let Ok(file) = result {
                if let Some(path) = file.path() {
                    on_path(path);
                }
            }
        });
        return;
    }
    #[cfg(not(feature = "modern-gtk"))]
    {
        let dialog = chooser_dialog(window, title, gtk4::FileChooserAction::Save, "Save");
        if let Some(name) = initial_name {
            dialog.set_current_name(name);
        }
        dialog.connect_response(move |d, response| {
            if response == gtk4::ResponseType::Accept {
                if let Some(path) = d.file().and_then(|f| f.path()) {
                    on_path(path);
                }
            }
            d.destroy();
        });
        dialog.present();
    }
}

// FileChooserNative routes through xdg-desktop-portal; without a working
// portal its fallback dialog never maps a real window, so the picker appears
// to do nothing (reported on Ubuntu 22.04). FileChooserDialog renders
// in-process and works everywhere GTK 4.6 runs.
#[allow(deprecated)]
#[cfg(not(feature = "modern-gtk"))]
fn chooser_dialog(
    window: Option<&gtk4::Window>,
    title: &str,
    action: gtk4::FileChooserAction,
    accept_label: &str,
) -> gtk4::FileChooserDialog {
    let dialog = gtk4::FileChooserDialog::new(
        Some(title),
        window,
        action,
        &[("Cancel", gtk4::ResponseType::Cancel), (accept_label, gtk4::ResponseType::Accept)],
    );
    dialog.set_modal(true);
    dialog
}

/// `GtkTextView` hides the mouse pointer while typing via a widget-level
/// "none" cursor and restores it on motion — but the restore is gated on a
/// device-timestamp check that fails on GTK 4.6 (Ubuntu 22.04), leaving the
/// pointer invisible. Re-assert the widget's "text" cursor at idle after
/// each commit — lands after GTK's hide and goes through GTK's normal
/// widget cursor tracking, so other widgets keep their own cursors.
/// (Never set the surface cursor here: while obscured, `view.cursor()` IS
/// the "none" cursor, and a raw surface set bypasses widget tracking.)
pub fn unhide_pointer_on_typing(view: &sourceview5::View) {
    let weak = view.downgrade();
    view.buffer().connect_changed(move |_| {
        let Some(view) = weak.upgrade() else { return };
        gtk4::glib::idle_add_local_once(move || {
            view.set_cursor_from_name(Some("text"));
        });
    });
}

/// `set_content_fit(Contain)` is GTK 4.8+; on 4.6 the equivalent is
/// `can_shrink` — the widget preserves aspect by default either way.
pub fn fit_picture(picture: &gtk4::Picture) {
    #[cfg(feature = "modern-gtk")]
    picture.set_content_fit(gtk4::ContentFit::Contain);
    #[cfg(not(feature = "modern-gtk"))]
    picture.set_can_shrink(true);
}

// ─── Embedded terminal ──────────────────────────────────────────────────

/// The bottom-console terminal. `vte` builds embed a real VTE widget
/// running a login shell; Ubuntu 22.04 and Windows have no VTE-GTK4, so
/// they embed the `terminal-core` + DrawingArea terminal instead — same
/// API, same behavior. An external emulator is only the fallback for a
/// shell that failed to spawn or already exited.
pub enum ShellTerminal {
    #[cfg(feature = "vte")]
    Embedded(vte4::Terminal),
    #[cfg(not(feature = "vte"))]
    Embedded(crate::terminal::EmbeddedTerminal),
}

impl ShellTerminal {
    pub fn new() -> Self {
        #[cfg(feature = "vte")]
        {
            let terminal = vte4::Terminal::new();
            terminal.set_scrollback_lines(10_000);
            terminal.set_scroll_on_output(false);
            terminal.set_scroll_on_keystroke(true);
            Self::Embedded(terminal)
        }
        #[cfg(not(feature = "vte"))]
        {
            Self::Embedded(crate::terminal::EmbeddedTerminal::new())
        }
    }

    /// The widget placed inside the console pane.
    pub fn widget(&self) -> gtk4::Widget {
        match self {
            #[cfg(feature = "vte")]
            Self::Embedded(t) => t.clone().upcast(),
            #[cfg(not(feature = "vte"))]
            Self::Embedded(t) => t.widget(),
        }
    }

    /// ANSI status text — rendered into the grid like child output.
    pub fn feed(&self, text: &str) {
        match self {
            #[cfg(feature = "vte")]
            Self::Embedded(t) => t.feed(text.as_bytes()),
            #[cfg(not(feature = "vte"))]
            Self::Embedded(t) => t.feed(text),
        }
    }

    /// A command line to the shell's stdin. If the embedded shell never
    /// spawned or already exited, `send` falls back to an external terminal
    /// emulator so interactive flows (Pi sign-in, custom commands) still work.
    pub fn send(&self, command: &str, dir: Option<&Path>) {
        match self {
            #[cfg(feature = "vte")]
            Self::Embedded(t) => {
                let _ = dir;
                t.feed_child(format!("{command}\r").as_bytes())
            }
            #[cfg(not(feature = "vte"))]
            Self::Embedded(t) => t.send(command, dir),
        }
    }

    /// Spawn the login shell on the embedded PTY. The non-VTE spawn is
    /// synchronous — the callback still reports success/failure so
    /// `terminal_running` stays honest.
    pub fn spawn_shell(&self, dir: Option<&Path>, on_result: impl Fn(bool) + 'static) {
        match self {
            #[cfg(feature = "vte")]
            Self::Embedded(t) => {
                let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".into());
                let mut env: Vec<String> =
                    std::env::vars().map(|(k, v)| format!("{k}={v}")).collect();
                env.push("TERM=xterm-256color".into());
                env.push("COLORTERM=truecolor".into());
                env.push("TERM_PROGRAM=Pitex".into());
                let argv = [shell.as_str(), "-l"];
                let envv: Vec<&str> = env.iter().map(|s| s.as_str()).collect();
                let dir_str = dir.map(|d| d.to_string_lossy().into_owned());
                t.spawn_async(
                    vte4::PtyFlags::DEFAULT,
                    dir_str.as_deref(),
                    &argv,
                    &envv,
                    gtk4::glib::SpawnFlags::DEFAULT,
                    || {},
                    -1,
                    gtk4::gio::Cancellable::NONE,
                    move |result| on_result(result.is_ok()),
                );
            }
            #[cfg(not(feature = "vte"))]
            Self::Embedded(t) => t.spawn_shell(dir, on_result),
        }
    }

    pub fn set_font(&self, font: Option<&gtk4::pango::FontDescription>) {
        match self {
            #[cfg(feature = "vte")]
            Self::Embedded(t) => t.set_font(font),
            #[cfg(not(feature = "vte"))]
            Self::Embedded(t) => t.set_font(font),
        }
    }

    /// Embedded shells report exit so the transcript can note it.
    pub fn connect_exited(&self, f: impl Fn(&Self) + Send + 'static) {
        match self {
            #[cfg(feature = "vte")]
            Self::Embedded(t) => {
                let this = self.clone_ref();
                t.connect_child_exited(move |_, _| f(&this));
            }
            #[cfg(not(feature = "vte"))]
            Self::Embedded(t) => {
                t.connect_exited(move |inner| f(&Self::Embedded(inner.clone())));
            }
        }
    }

    #[cfg(feature = "vte")]
    fn clone_ref(&self) -> Self {
        match self {
            Self::Embedded(t) => Self::Embedded(t.clone()),
        }
    }
}

/// Fallback for a dead/unspawned embedded shell — the updater uses the
/// same path for `sudo apt install`.
pub fn run_in_external_terminal(command: &str, dir: Option<&Path>) -> bool {
    spawn_external_terminal(command, dir)
}

/// Try common terminal emulators in order; each stays open after the
/// command finishes so sign-in output remains readable.
#[cfg(unix)]
fn spawn_external_terminal(command: &str, dir: Option<&Path>) -> bool {
    let dir_arg = dir.map(|d| d.to_string_lossy().into_owned());
    let hold = format!("{command}; exec bash");
    let candidates: [(&str, Vec<String>); 4] = [
        (
            "kgx",
            vec!["--".into(), "bash".into(), "-c".into(), hold.clone()],
        ),
        (
            "gnome-terminal",
            vec!["--".into(), "bash".into(), "-c".into(), hold.clone()],
        ),
        (
            "konsole",
            vec!["-e".into(), "bash".into(), "-c".into(), hold.clone()],
        ),
        ("xterm", vec!["-e".into(), "bash".into(), "-c".into(), hold]),
    ];
    for (bin, args) in candidates {
        let mut cmd = std::process::Command::new(bin);
        // Only the GNOME emulators take --working-directory; the rest
        // inherit the app's cwd.
        if let Some(d) = &dir_arg {
            if matches!(bin, "kgx" | "gnome-terminal") {
                cmd.arg(format!("--working-directory={d}"));
            }
        }
        if cmd.args(&args).spawn().is_ok() {
            return true;
        }
    }
    false
}

/// Windows counterpart: `cmd /k` keeps the console open after the command —
/// the same "stay readable" contract the Unix `; exec bash` tail gives.
/// Windows Terminal (`wt`) is preferred; the conhost `start` fallback works
/// on every Windows install. `current_dir` carries the working directory
/// into the new console (avoids quoting `start /D` edge cases).
#[cfg(windows)]
fn spawn_external_terminal(command: &str, dir: Option<&Path>) -> bool {
    let mut wt = std::process::Command::new("wt");
    wt.args(["cmd", "/k", command]);
    if let Some(d) = dir {
        wt.current_dir(d);
    }
    if wt.spawn().is_ok() {
        return true;
    }
    let mut conhost = std::process::Command::new("cmd");
    conhost.args(["/c", "start", "Pitex", "cmd", "/k", command]);
    if let Some(d) = dir {
        conhost.current_dir(d);
    }
    conhost.spawn().is_ok()
}

// ─── Font / color pickers ────────────────────────────────────────────────

/// Editor-font picker. GTK 4.10's `FontDialogButton` on modern builds;
/// `GtkFontButton` on 22.04 — deprecated in 4.10, but the only stock font
/// picker GTK 4.6 ships.
pub enum FontPicker {
    #[cfg(feature = "modern-gtk")]
    Dialog(gtk4::FontDialogButton),
    /// Only constructed when `modern-gtk` is off.
    #[allow(dead_code)]
    #[allow(deprecated)]
    Button(gtk4::FontButton),
}

impl FontPicker {
    pub fn new() -> Self {
        #[cfg(feature = "modern-gtk")]
        {
            Self::Dialog(gtk4::FontDialogButton::new(Some(gtk4::FontDialog::new())))
        }
        #[cfg(not(feature = "modern-gtk"))]
        #[allow(deprecated)]
        {
            Self::Button(gtk4::FontButton::new())
        }
    }

    pub fn widget(&self) -> &gtk4::Widget {
        match self {
            #[cfg(feature = "modern-gtk")]
            Self::Dialog(b) => b.upcast_ref(),
            Self::Button(b) => b.upcast_ref(),
        }
    }

    pub fn set_font_desc(&self, desc: &gtk4::pango::FontDescription) {
        match self {
            #[cfg(feature = "modern-gtk")]
            Self::Dialog(b) => b.set_font_desc(desc),
            #[allow(deprecated)]
            Self::Button(b) => b.set_font_desc(desc),
        }
    }

    pub fn font_desc(&self) -> Option<gtk4::pango::FontDescription> {
        match self {
            #[cfg(feature = "modern-gtk")]
            Self::Dialog(b) => b.font_desc(),
            #[allow(deprecated)]
            Self::Button(b) => b.font_desc(),
        }
    }

    /// Fires on notify::font-desc — same signal on both widget families.
    pub fn connect_changed(&self, f: impl Fn(&Self) + 'static) {
        match self {
            #[cfg(feature = "modern-gtk")]
            Self::Dialog(b) => {
                let this = self.clone_ref();
                b.connect_font_desc_notify(move |_| f(&this));
            }
            #[allow(deprecated)]
            Self::Button(b) => {
                let this = self.clone_ref();
                b.connect_font_desc_notify(move |_| f(&this));
            }
        }
    }

    pub fn clone_ref(&self) -> Self {
        match self {
            #[cfg(feature = "modern-gtk")]
            Self::Dialog(b) => Self::Dialog(b.clone()),
            #[allow(deprecated)]
            Self::Button(b) => Self::Button(b.clone()),
        }
    }
}

/// Palette color well. GTK 4.10's `ColorDialogButton` on modern builds;
/// `GtkColorButton` on 22.04 — deprecated in 4.10, but the only stock
/// color picker GTK 4.6 ships. Alpha stays on, matching the modern
/// dialog's `with_alpha`.
pub enum ColorWell {
    #[cfg(feature = "modern-gtk")]
    Dialog(gtk4::ColorDialogButton),
    /// Only constructed when `modern-gtk` is off.
    #[allow(dead_code)]
    #[allow(deprecated)]
    Button(gtk4::ColorButton),
}

impl ColorWell {
    pub fn new() -> Self {
        #[cfg(feature = "modern-gtk")]
        {
            let dialog = gtk4::ColorDialog::new();
            dialog.set_with_alpha(true);
            Self::Dialog(gtk4::ColorDialogButton::new(Some(dialog)))
        }
        #[cfg(not(feature = "modern-gtk"))]
        #[allow(deprecated)]
        {
            let button = gtk4::ColorButton::new();
            button.set_use_alpha(true);
            Self::Button(button)
        }
    }

    pub fn widget(&self) -> &gtk4::Widget {
        match self {
            #[cfg(feature = "modern-gtk")]
            Self::Dialog(b) => b.upcast_ref(),
            Self::Button(b) => b.upcast_ref(),
        }
    }

    pub fn set_rgba(&self, rgba: &gtk4::gdk::RGBA) {
        match self {
            #[cfg(feature = "modern-gtk")]
            Self::Dialog(b) => b.set_rgba(rgba),
            #[allow(deprecated)]
            Self::Button(b) => b.set_rgba(rgba),
        }
    }

    pub fn rgba(&self) -> gtk4::gdk::RGBA {
        match self {
            #[cfg(feature = "modern-gtk")]
            Self::Dialog(b) => b.rgba(),
            #[allow(deprecated)]
            Self::Button(b) => b.rgba(),
        }
    }

    /// notify::rgba on both widget families.
    pub fn connect_changed(&self, f: impl Fn(&Self) + 'static) {
        match self {
            #[cfg(feature = "modern-gtk")]
            Self::Dialog(b) => {
                let this = self.clone_ref();
                b.connect_rgba_notify(move |_| f(&this));
            }
            #[allow(deprecated)]
            Self::Button(b) => {
                let this = self.clone_ref();
                b.connect_rgba_notify(move |_| f(&this));
            }
        }
    }

    fn clone_ref(&self) -> Self {
        match self {
            #[cfg(feature = "modern-gtk")]
            Self::Dialog(b) => Self::Dialog(b.clone()),
            #[allow(deprecated)]
            Self::Button(b) => Self::Button(b.clone()),
        }
    }
}
