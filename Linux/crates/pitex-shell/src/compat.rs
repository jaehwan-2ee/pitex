//! Build-variant compatibility shims.
//!
//! Two orthogonal features select the surface:
//!
//! - `modern-gtk` — GTK 4.10+ / libadwaita 1.4+ APIs (file `add_suffix`,
//!   `ContentFit`, `FontDialogButton`, `ColorDialogButton`). Off for the
//!   Ubuntu 22.04 build (GTK 4.6 / libadwaita 1.1), on everywhere else.
//! - `vte` — the embedded VTE terminal. On for Unix desktop builds; off on
//!   Ubuntu 22.04 and Windows, where the console terminal degrades to a
//!   read-only transcript plus an external terminal emulator (kgx /
//!   gnome-terminal / konsole / xterm on Linux, `wt` / conhost on Windows).

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
/// running a login shell; Ubuntu 22.04 and Windows have no VTE-GTK4, so the
/// pane degrades to a read-only transcript — status feeds still land here
/// and `send` hands interactive commands to an external terminal emulator.
pub enum ShellTerminal {
    #[cfg(feature = "vte")]
    Embedded(vte4::Terminal),
    /// Only constructed when `vte` is off — kept visible in both builds so
    /// the match arms below stay uniform.
    #[allow(dead_code)]
    External(gtk4::TextView),
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
            let view = gtk4::TextView::new();
            view.set_editable(false);
            view.set_cursor_visible(false);
            view.set_monospace(true);
            view.set_wrap_mode(gtk4::WrapMode::Char);
            view.set_top_margin(8);
            view.set_bottom_margin(8);
            view.set_left_margin(8);
            view.set_right_margin(8);
            view.buffer()
                .set_text("Interactive commands run in an external terminal on this build.\n");
            Self::External(view)
        }
    }

    /// The widget placed inside the console pane (already wrapped in a
    /// `ScrolledWindow` for the legacy transcript).
    pub fn widget(&self) -> gtk4::Widget {
        match self {
            #[cfg(feature = "vte")]
            Self::Embedded(t) => t.clone().upcast(),
            Self::External(v) => {
                let scroll = gtk4::ScrolledWindow::new();
                scroll.set_vexpand(true);
                scroll.set_hexpand(true);
                scroll.set_child(Some(v));
                scroll.upcast()
            }
        }
    }

    /// ANSI status text. The legacy transcript strips escape sequences —
    /// it only ever carries the app's own colored status lines.
    pub fn feed(&self, text: &str) {
        match self {
            #[cfg(feature = "vte")]
            Self::Embedded(t) => t.feed(text.as_bytes()),
            Self::External(v) => {
                let buffer = v.buffer();
                let mut end = buffer.end_iter();
                buffer.insert(&mut end, &strip_ansi(text));
            }
        }
    }

    /// A command line to a shell's stdin. Embedded builds write to the PTY;
    /// builds without VTE spawn an external terminal emulator so interactive
    /// flows (Pi sign-in, custom commands) still work.
    pub fn send(&self, command: &str, dir: Option<&Path>) {
        match self {
            #[cfg(feature = "vte")]
            Self::Embedded(t) => t.feed_child(format!("{command}\r").as_bytes()),
            Self::External(_) => {
                if spawn_external_terminal(command, dir) {
                    self.feed(&format!("\n→ ran in external terminal: {command}\n"));
                } else {
                    self.feed(&format!("\nNo terminal emulator found to run: {command}\n"));
                }
            }
        }
    }

    /// Spawn the login shell — embedded only. Transcript builds have no
    /// embedded shell; the callback reports failure so `terminal_running`
    /// stays honest.
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
            Self::External(_) => {
                let _ = dir;
                on_result(false);
            }
        }
    }

    pub fn set_font(&self, font: Option<&gtk4::pango::FontDescription>) {
        match self {
            #[cfg(feature = "vte")]
            Self::Embedded(t) => t.set_font(font),
            Self::External(_) => {
                let _ = font;
            }
        }
    }

    /// Embedded shells report exit so the transcript can note it; external
    /// terminals are fire-and-forget so there is nothing to connect.
    pub fn connect_exited(&self, f: impl Fn(&Self) + 'static) {
        match self {
            #[cfg(feature = "vte")]
            Self::Embedded(t) => {
                let this = self.clone_ref();
                t.connect_child_exited(move |_, _| f(&this));
            }
            Self::External(_) => {
                let _ = f;
            }
        }
    }

    #[cfg(feature = "vte")]
    fn clone_ref(&self) -> Self {
        match self {
            Self::Embedded(t) => Self::Embedded(t.clone()),
            Self::External(v) => Self::External(v.clone()),
        }
    }
}

/// Strip ANSI CSI/OSC sequences for the read-only transcript.
fn strip_ansi(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            match chars.peek() {
                Some(&'[') => {
                    // CSI: consume until a final byte in 0x40..=0x7e.
                    chars.next();
                    for n in chars.by_ref() {
                        if ('\u{40}'..='\u{7e}').contains(&n) {
                            break;
                        }
                    }
                }
                Some(&']') => {
                    // OSC: consume until BEL or ST (ESC \).
                    chars.next();
                    let mut prev = '\0';
                    for n in chars.by_ref() {
                        if n == '\u{7}' || (prev == '\u{1b}' && n == '\\') {
                            break;
                        }
                        prev = n;
                    }
                }
                _ => {}
            }
            continue;
        }
        out.push(c);
    }
    out
}

/// Non-VTE builds hand interactive commands to a real terminal emulator —
/// the updater uses the same path for `sudo apt install`.
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

/// Editor-font picker. GTK 4.10's `FontDialogButton` on modern builds; a
/// "Family Size" text entry on 22.04 (no font chooser exists below 4.10).
pub enum FontPicker {
    #[cfg(feature = "modern-gtk")]
    Button(gtk4::FontDialogButton),
    /// Only constructed when `modern-gtk` is off.
    #[allow(dead_code)]
    Entry(gtk4::Entry),
}

impl FontPicker {
    pub fn new() -> Self {
        #[cfg(feature = "modern-gtk")]
        {
            Self::Button(gtk4::FontDialogButton::new(Some(gtk4::FontDialog::new())))
        }
        #[cfg(not(feature = "modern-gtk"))]
        {
            let entry = gtk4::Entry::new();
            entry.set_placeholder_text(Some("Family Size"));
            entry.set_width_chars(18);
            Self::Entry(entry)
        }
    }

    pub fn widget(&self) -> &gtk4::Widget {
        match self {
            #[cfg(feature = "modern-gtk")]
            Self::Button(b) => b.upcast_ref(),
            Self::Entry(e) => e.upcast_ref(),
        }
    }

    pub fn set_font_desc(&self, desc: &gtk4::pango::FontDescription) {
        match self {
            #[cfg(feature = "modern-gtk")]
            Self::Button(b) => b.set_font_desc(desc),
            Self::Entry(e) => e.set_text(&desc.to_string()),
        }
    }

    pub fn font_desc(&self) -> Option<gtk4::pango::FontDescription> {
        match self {
            #[cfg(feature = "modern-gtk")]
            Self::Button(b) => b.font_desc(),
            Self::Entry(e) => {
                let text = e.text().to_string();
                if text.trim().is_empty() {
                    None
                } else {
                    Some(gtk4::pango::FontDescription::from_string(&text))
                }
            }
        }
    }

    /// Fires when the picked font changes (notify::font-desc on modern,
    /// entry changes on legacy).
    pub fn connect_changed(&self, f: impl Fn(&Self) + 'static) {
        match self {
            #[cfg(feature = "modern-gtk")]
            Self::Button(b) => {
                let this = self.clone_ref();
                b.connect_font_desc_notify(move |_| f(&this));
            }
            Self::Entry(e) => {
                let this = self.clone_ref();
                e.connect_changed(move |_| f(&this));
            }
        }
    }

    pub fn clone_ref(&self) -> Self {
        match self {
            #[cfg(feature = "modern-gtk")]
            Self::Button(b) => Self::Button(b.clone()),
            Self::Entry(e) => Self::Entry(e.clone()),
        }
    }
}

/// Palette color well. GTK 4.10's `ColorDialogButton` on modern builds; a
/// hex entry (`#rrggbb[aa]`) on 22.04 — GTK4 has no color chooser below
/// 4.10, and hex is the palette's storage format anyway.
pub enum ColorWell {
    #[cfg(feature = "modern-gtk")]
    Button(gtk4::ColorDialogButton),
    /// Only constructed when `modern-gtk` is off.
    #[allow(dead_code)]
    Entry(gtk4::Entry),
}

impl ColorWell {
    pub fn new() -> Self {
        #[cfg(feature = "modern-gtk")]
        {
            let dialog = gtk4::ColorDialog::new();
            dialog.set_with_alpha(true);
            Self::Button(gtk4::ColorDialogButton::new(Some(dialog)))
        }
        #[cfg(not(feature = "modern-gtk"))]
        {
            let entry = gtk4::Entry::new();
            entry.set_placeholder_text(Some("#rrggbb"));
            entry.set_width_chars(10);
            Self::Entry(entry)
        }
    }

    pub fn widget(&self) -> &gtk4::Widget {
        match self {
            #[cfg(feature = "modern-gtk")]
            Self::Button(b) => b.upcast_ref(),
            Self::Entry(e) => e.upcast_ref(),
        }
    }

    pub fn set_rgba(&self, rgba: &gtk4::gdk::RGBA) {
        match self {
            #[cfg(feature = "modern-gtk")]
            Self::Button(b) => b.set_rgba(rgba),
            Self::Entry(e) => e.set_text(&crate::settings::rgba_to_hex_string(
                rgba.red() as f64,
                rgba.green() as f64,
                rgba.blue() as f64,
                rgba.alpha() as f64,
            )),
        }
    }

    pub fn rgba(&self) -> gtk4::gdk::RGBA {
        match self {
            #[cfg(feature = "modern-gtk")]
            Self::Button(b) => b.rgba(),
            Self::Entry(e) => {
                let fallback = gtk4::gdk::RGBA::new(0.0, 0.0, 0.0, 1.0);
                crate::settings::parse_hex_color(&e.text())
                    .map(|(r, g, b, a)| {
                        gtk4::gdk::RGBA::new(r as f32, g as f32, b as f32, a as f32)
                    })
                    .unwrap_or(fallback)
            }
        }
    }

    /// notify::rgba on modern; text commits on legacy.
    pub fn connect_changed(&self, f: impl Fn(&Self) + 'static) {
        match self {
            #[cfg(feature = "modern-gtk")]
            Self::Button(b) => {
                let this = self.clone_ref();
                b.connect_rgba_notify(move |_| f(&this));
            }
            Self::Entry(e) => {
                let this = self.clone_ref();
                e.connect_changed(move |_| f(&this));
            }
        }
    }

    fn clone_ref(&self) -> Self {
        match self {
            #[cfg(feature = "modern-gtk")]
            Self::Button(b) => Self::Button(b.clone()),
            Self::Entry(e) => Self::Entry(e.clone()),
        }
    }
}
