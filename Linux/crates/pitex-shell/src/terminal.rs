//! Embedded terminal for builds without VTE (Ubuntu 22.04, Windows).
//!
//! A `gtk4::DrawingArea` rendering `terminal_core::Session`'s grid with
//! Pango. PTY output reaches the GTK thread through a coalesced
//! `idle_add_once` hop — at most one queued redraw per output burst.
//! Input goes through an `IMMulticontext` (IME/CJK work) and then
//! `terminal_core::encode_key` for the keys a terminal owns.

use std::cell::{Cell, RefCell};
use std::path::Path;
use std::rc::{Rc, Weak};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use gtk4::gdk;
use gtk4::glib;
use gtk4::pango;
use gtk4::prelude::*;
use libadwaita as adw;

use terminal_core::alacritty_terminal::event::WindowSize;
use terminal_core::alacritty_terminal::grid::{Dimensions, Scroll};
use terminal_core::alacritty_terminal::index::{Column, Line, Point, Side};
use terminal_core::alacritty_terminal::selection::{Selection, SelectionType};
use terminal_core::alacritty_terminal::term::cell::Flags;
use terminal_core::alacritty_terminal::term::{self, TermMode};
use terminal_core::alacritty_terminal::vte::ansi::{Color, CursorShape, NamedColor, Rgb};
use terminal_core::{encode_key, encode_mouse, encode_paste, Key, KeyModes, Modifiers, Session, SessionEvent};

const SCROLLBACK: usize = 10_000;
const BLINK_MS: u64 = 530;
/// Fallback cell metrics until the widget is realized and Pango can
/// measure the real font.
const DEFAULT_CELL: (f64, f64) = (9.0, 17.0);

/// Theme-dependent 16-color palette plus defaults.
#[derive(Clone, Copy)]
struct Palette {
    fg: (f64, f64, f64),
    bg: (f64, f64, f64),
    cursor: (f64, f64, f64),
    selection: (f64, f64, f64, f64),
    ansi: [(f64, f64, f64); 16],
}

fn palette(dark: bool) -> Palette {
    if dark {
        Palette {
            fg: (0.92, 0.93, 0.94),
            bg: (0.13, 0.13, 0.15),
            cursor: (0.92, 0.93, 0.94),
            selection: (0.38, 0.50, 0.71, 0.45),
            ansi: [
                (0.13, 0.13, 0.15), // black
                (0.90, 0.29, 0.32), // red
                (0.44, 0.72, 0.42), // green
                (0.85, 0.66, 0.27), // yellow
                (0.36, 0.60, 0.84), // blue
                (0.74, 0.48, 0.79), // magenta
                (0.35, 0.72, 0.75), // cyan
                (0.85, 0.87, 0.88), // white
                (0.42, 0.44, 0.47), // bright black
                (0.95, 0.45, 0.45), // bright red
                (0.57, 0.80, 0.53), // bright green
                (0.93, 0.78, 0.42), // bright yellow
                (0.51, 0.71, 0.90), // bright blue
                (0.81, 0.60, 0.85), // bright magenta
                (0.49, 0.80, 0.82), // bright cyan
                (0.97, 0.97, 0.97), // bright white
            ],
        }
    } else {
        Palette {
            fg: (0.15, 0.16, 0.18),
            bg: (0.99, 0.99, 0.99),
            cursor: (0.15, 0.16, 0.18),
            selection: (0.22, 0.50, 0.95, 0.30),
            ansi: [
                (0.15, 0.16, 0.18),
                (0.75, 0.16, 0.19),
                (0.21, 0.55, 0.24),
                (0.63, 0.46, 0.08),
                (0.11, 0.40, 0.72),
                (0.56, 0.27, 0.62),
                (0.12, 0.52, 0.56),
                (0.71, 0.72, 0.74),
                (0.45, 0.47, 0.50),
                (0.88, 0.30, 0.32),
                (0.32, 0.66, 0.35),
                (0.77, 0.58, 0.16),
                (0.24, 0.52, 0.82),
                (0.68, 0.40, 0.73),
                (0.22, 0.64, 0.68),
                (0.95, 0.95, 0.95),
            ],
        }
    }
}

fn dim(c: (f64, f64, f64), bg: (f64, f64, f64)) -> (f64, f64, f64) {
    (c.0 * 0.5 + bg.0 * 0.5, c.1 * 0.5 + bg.1 * 0.5, c.2 * 0.5 + bg.2 * 0.5)
}

/// 256-color indexed entries: palette for 0-15, 6×6×6 cube, then gray ramp.
fn indexed_rgb(i: u8, pal: &Palette) -> (f64, f64, f64) {
    match i {
        0..=15 => pal.ansi[i as usize],
        16..=231 => {
            let i = i as usize - 16;
            let level = |v: usize| [0.0, 95.0, 135.0, 175.0, 215.0, 255.0][v] / 255.0;
            (level(i / 36), level((i / 6) % 6), level(i % 6))
        }
        _ => {
            let v = (8.0 + (i as f64 - 232.0) * 10.0) / 255.0;
            (v, v, v)
        }
    }
}

struct Inner {
    session: RefCell<Session>,
    /// Self-reference so `Exited` handlers can receive `&EmbeddedTerminal`.
    me: RefCell<Weak<Inner>>,
    area: gtk4::DrawingArea,
    overlay: gtk4::Overlay,
    scrollbar: gtk4::Scrollbar,
    adjustment: gtk4::Adjustment,
    im: gtk4::IMMulticontext,
    font: RefCell<pango::FontDescription>,
    /// (cell width, cell height) in pixels.
    cell: Cell<(f64, f64)>,
    /// Shell process running — `send` falls back to an external terminal
    /// when false.
    alive: Cell<bool>,
    preedit: RefCell<String>,
    /// Cursor blink phase.
    cursor_on: Cell<bool>,
    /// Anchor of the in-progress drag selection (grid point).
    sel_anchor: Cell<Point>,
    /// Widget-relative pointer position for scroll reports, and the last
    /// reported cell so motion within one cell does not flood the PTY.
    pointer: Cell<(f64, f64)>,
    last_mouse_cell: Cell<Option<(usize, usize)>>,
    wheel_remainder: Cell<(f64, f64)>,
    /// Scrollbar writes re-enter via value-changed — guard the feedback.
    syncing_scroll: Cell<bool>,
    exited: RefCell<Vec<Box<dyn Fn(&EmbeddedTerminal) + Send>>>,
}

/// The non-VTE embedded terminal behind `compat::ShellTerminal`. Cheap to
/// clone — `connect_exited` hands clones to its listeners.
#[derive(Clone)]
pub struct EmbeddedTerminal {
    inner: Rc<Inner>,
}

impl EmbeddedTerminal {
    pub fn new() -> Self {
        let area = gtk4::DrawingArea::new();
        area.set_focusable(true);
        area.set_hexpand(true);
        area.set_vexpand(true);
        area.set_content_width((80.0 * DEFAULT_CELL.0) as i32);
        area.set_content_height((24.0 * DEFAULT_CELL.1) as i32);

        let overlay = gtk4::Overlay::new();
        overlay.set_child(Some(&area));

        let adjustment = gtk4::Adjustment::new(0.0, 0.0, 0.0, 1.0, 4.0, 1.0);
        let scrollbar = gtk4::Scrollbar::new(gtk4::Orientation::Vertical, Some(&adjustment));
        scrollbar.set_halign(gtk4::Align::End);

        let im = gtk4::IMMulticontext::new();
        im.set_client_widget(Some(&area));
        // We render the preedit ourselves — the IME must not pop its own.
        im.set_use_preedit(true);

        let inner = Rc::new(Inner {
            session: RefCell::new(Session::new(80, 24, SCROLLBACK)),
            me: RefCell::new(Weak::new()),
            area: area.clone(),
            overlay,
            scrollbar,
            adjustment,
            im,
            font: RefCell::new(pango::FontDescription::from_string("Monospace 11")),
            cell: Cell::new(DEFAULT_CELL),
            alive: Cell::new(false),
            preedit: RefCell::new(String::new()),
            cursor_on: Cell::new(true),
            sel_anchor: Cell::new(Point::new(Line(0), Column(0))),
            pointer: Cell::new((0.0, 0.0)),
            last_mouse_cell: Cell::new(None),
            wheel_remainder: Cell::new((0.0, 0.0)),
            syncing_scroll: Cell::new(false),
            exited: RefCell::new(Vec::new()),
        });
        *inner.me.borrow_mut() = Rc::downgrade(&inner);

        Self::wire_input(&inner);
        Self::wire_redraw(&inner);
        Self::wire_blink(&inner);
        Self::wire_draw(&inner);

        Self { inner }
    }

    /// The widget embedded in the console pane: the grid plus an overlay
    /// scrollbar (attached on first scrollback so GTK doesn't snapshot an
    /// unallocated child).
    pub fn widget(&self) -> gtk4::Widget {
        self.inner.overlay.clone().upcast()
    }

    /// App status text — rendered into the grid, never sent to the shell.
    pub fn feed(&self, text: &str) {
        self.inner.session.borrow().feed(text.as_bytes());
        self.inner.area.queue_draw();
    }

    /// A command line to the shell's stdin. With no live shell (spawn
    /// failed or exited), falls back to an external terminal — the same
    /// contract the VTE-less transcript used to have for everything.
    pub fn send(&self, command: &str, dir: Option<&Path>) {
        if self.inner.alive.get() {
            self.inner.session.borrow().write(format!("{command}\r").into_bytes());
            self.inner.scroll_to_bottom();
        } else if crate::compat::run_in_external_terminal(command, dir) {
            self.feed(&format!("\r\n\u{1b}[90m→ ran in external terminal: {command}\u{1b}[0m\r\n"));
        } else {
            self.feed(&format!("\r\nNo terminal emulator found to run: {command}\r\n"));
        }
    }

    /// Spawn the shell on the PTY. Setup doesn't block, so `on_result`
    /// fires synchronously — same outcome as the VTE path's callback.
    pub fn spawn_shell(&self, dir: Option<&Path>, on_result: impl Fn(bool) + 'static) {
        let shell = Self::shell();
        let (cell_w, cell_h) = self.inner.cell.get();
        // Before the first allocation there is no measured size — spawn at
        // the classic 80×24; the draw callback resizes (SIGWINCH) on show.
        let (cols, rows) = if self.inner.area.width() > 0 {
            self.inner.grid_size()
        } else {
            (80, 24)
        };
        let result = self.inner.session.borrow_mut().spawn(terminal_core::SpawnOptions {
            shell: Some(shell),
            working_directory: dir.map(|d| d.to_path_buf()),
            env: vec![
                ("TERM".into(), "xterm-256color".into()),
                ("COLORTERM".into(), "truecolor".into()),
                ("TERM_PROGRAM".into(), "Pitex".into()),
            ],
            cols,
            rows,
            cell_width: cell_w as u16,
            cell_height: cell_h as u16,
        });
        match result {
            Ok(()) => {
                self.inner.alive.set(true);
                on_result(true);
            }
            Err(err) => {
                self.feed(&format!(
                    "\u{1b}[31mCouldn't spawn a shell ({err}) — interactive commands run in an external terminal.\u{1b}[0m\r\n"
                ));
                on_result(false);
            }
        }
    }

    /// `$SHELL -l` on Unix (24.04 parity); on Windows the configured shell
    /// (`customShellExecutable`, cmd.exe by default — powershell.exe if the
    /// user picks it) or `%COMSPEC%`.
    fn shell() -> terminal_core::Shell {
        #[cfg(unix)]
        {
            terminal_core::default_shell()
        }
        #[cfg(windows)]
        {
            match crate::settings::Preferences::standard().string("customShellExecutable") {
                Some(prog) if !prog.trim().is_empty() => {
                    terminal_core::Shell::new(prog, Vec::new())
                }
                _ => terminal_core::default_shell(),
            }
        }
    }

    /// Follows the terminal-font setting; `None` resets to the default.
    pub fn set_font(&self, font: Option<&pango::FontDescription>) {
        *self.inner.font.borrow_mut() =
            font.cloned().unwrap_or_else(|| pango::FontDescription::from_string("Monospace 11"));
        if self.inner.measure() {
            self.inner.resize_to_fit();
        }
        self.inner.area.queue_draw();
    }

    /// Shell exit → UI listeners (same shape as VTE's `child-exited`).
    pub fn connect_exited(&self, f: impl Fn(&Self) + Send + 'static) {
        self.inner.exited.borrow_mut().push(Box::new(move |t| f(t)));
    }

    // ── wiring ─────────────────────────────────────────────────────────
    // All widget-held callbacks capture `Weak<Inner>` — strong captures
    // would cycle (Inner → area → closure → Inner) and `Session::drop`,
    // which shuts the PTY down, would never run.

    fn wire_input(inner: &Rc<Inner>) {
        let keys = gtk4::EventControllerKey::new();
        keys.connect_key_pressed({
            let weak = Rc::downgrade(inner);
            move |ctl, keyval, _keycode, state| {
                let Some(inner) = weak.upgrade() else {
                    return glib::Propagation::Proceed;
                };
                inner.key_pressed(ctl, keyval, state)
            }
        });
        keys.connect_key_released({
            let weak = Rc::downgrade(inner);
            move |ctl, _keyval, _keycode, _state| {
                // IMEs consume releases too (jamo flush, compose state).
                let (Some(inner), Some(ev)) = (weak.upgrade(), ctl.current_event()) else {
                    return;
                };
                inner.im.filter_keypress(&ev);
            }
        });
        inner.area.add_controller(keys);

        inner.im.connect_commit({
            let weak = Rc::downgrade(inner);
            move |_, text| {
                let Some(inner) = weak.upgrade() else { return };
                inner.session.borrow().write(text.as_bytes().to_vec());
                inner.scroll_to_bottom();
                inner.area.queue_draw();
            }
        });
        inner.im.connect_preedit_changed({
            let weak = Rc::downgrade(inner);
            move |im| {
                let Some(inner) = weak.upgrade() else { return };
                *inner.preedit.borrow_mut() = im.preedit_string().0.to_string();
                inner.area.queue_draw();
            }
        });
        inner.im.connect_preedit_end({
            let weak = Rc::downgrade(inner);
            move |_| {
                let Some(inner) = weak.upgrade() else { return };
                inner.preedit.borrow_mut().clear();
                inner.area.queue_draw();
            }
        });

        let focus = gtk4::EventControllerFocus::new();
        focus.connect_enter({
            let weak = Rc::downgrade(inner);
            move |_| {
                let Some(inner) = weak.upgrade() else { return };
                inner.im.focus_in();
                inner.report_focus(true);
                inner.cursor_on.set(true);
                inner.area.queue_draw();
            }
        });
        focus.connect_leave({
            let weak = Rc::downgrade(inner);
            move |_| {
                let Some(inner) = weak.upgrade() else { return };
                inner.im.focus_out();
                inner.report_focus(false);
                inner.area.queue_draw();
            }
        });
        inner.area.add_controller(focus);

        let click = gtk4::GestureClick::new();
        click.set_button(0);
        click.connect_pressed({
            let weak = Rc::downgrade(inner);
            move |gesture, n_press, x, y| {
                let Some(inner) = weak.upgrade() else { return };
                inner.area.grab_focus();
                inner.pointer.set((x, y));
                let button = gesture.current_button();
                if (1..=3).contains(&button)
                    && inner.report_mouse((button - 1) as u8, x, y, gesture.current_event_state(), false)
                {
                    return;
                }
                match button {
                    1 => inner.selection_pressed(n_press, x, y),
                    #[cfg(unix)]
                    2 => inner.paste_primary(),
                    #[cfg(windows)]
                    2 => inner.paste_clipboard(),
                    _ => {}
                }
            }
        });
        click.connect_released({
            let weak = Rc::downgrade(inner);
            move |gesture, _, x, y| {
                let Some(inner) = weak.upgrade() else { return };
                let button = gesture.current_button();
                if (1..=3).contains(&button) {
                    inner.report_mouse((button - 1) as u8, x, y, gesture.current_event_state(), true);
                }
            }
        });
        let drag = gtk4::GestureDrag::new();
        // Without grouping only one gesture could claim the button-1
        // sequence — click and drag must both see it.
        drag.group_with(&click);
        inner.area.add_controller(click);
        drag.connect_drag_update({
            let weak = Rc::downgrade(inner);
            move |gesture, dx, dy| {
                let (Some(inner), Some((sx, sy))) =
                    (weak.upgrade(), gesture.start_point())
                else {
                    return;
                };
                if !inner.mouse_tracking(gesture.current_event_state()) {
                    inner.selection_dragged(sx + dx, sy + dy);
                }
            }
        });
        drag.connect_drag_end({
            let weak = Rc::downgrade(inner);
            move |gesture, _, _| {
                if let Some(inner) = weak.upgrade() {
                    if !inner.mouse_tracking(gesture.current_event_state()) {
                        inner.selection_finished()
                    }
                }
            }
        });
        inner.area.add_controller(drag);

        let motion = gtk4::EventControllerMotion::new();
        motion.connect_enter({
            let weak = Rc::downgrade(inner);
            move |_, x, y| {
                if let Some(inner) = weak.upgrade() {
                    inner.pointer.set((x, y));
                    inner.last_mouse_cell.set(None);
                }
            }
        });
        motion.connect_motion({
            let weak = Rc::downgrade(inner);
            move |controller, x, y| {
                if let Some(inner) = weak.upgrade() {
                    inner.pointer.set((x, y));
                    inner.report_motion(x, y, controller.current_event_state());
                }
            }
        });
        inner.area.add_controller(motion);

        let scroll =
            gtk4::EventControllerScroll::new(gtk4::EventControllerScrollFlags::BOTH_AXES);
        scroll.connect_scroll({
            let weak = Rc::downgrade(inner);
            move |controller, dx, dy| {
                if let Some(inner) = weak.upgrade() {
                    inner.wheel(dx, dy, controller.current_event_state());
                }
                glib::Propagation::Stop
            }
        });
        inner.area.add_controller(scroll);

        inner.adjustment.connect_value_changed({
            let weak = Rc::downgrade(inner);
            move |adj| {
                if let Some(inner) = weak.upgrade() {
                    inner.scrollbar_moved(adj);
                }
            }
        });
    }

    /// Coalesced PTY→GTK hop: the session fires this from its reader
    /// thread; we post one `idle_add_once` (Send captures only) that asks
    /// for a redraw. The draw callback drains the event queue.
    fn wire_redraw(inner: &Rc<Inner>) {
        let pending = Arc::new(AtomicBool::new(false));
        let weak = glib::SendWeakRef::from(inner.area.downgrade());
        inner.session.borrow().set_wakeup(move || {
            if pending.swap(true, Ordering::SeqCst) {
                return;
            }
            let pending = pending.clone();
            let weak = weak.clone();
            glib::idle_add_once(move || {
                pending.store(false, Ordering::SeqCst);
                if let Some(area) = weak.upgrade() {
                    area.queue_draw();
                }
            });
        });
    }

    /// Cursor blink (~530ms half-period, like VTE). Skips repaints when the
    /// cursor doesn't blink.
    fn wire_blink(inner: &Rc<Inner>) {
        let weak = Rc::downgrade(inner);
        glib::timeout_add_local(std::time::Duration::from_millis(BLINK_MS), move || {
            let Some(inner) = weak.upgrade() else {
                return glib::ControlFlow::Break;
            };
            let blinking = inner.area.has_focus()
                && inner.session.borrow().term.lock().cursor_style().blinking;
            if blinking {
                inner.cursor_on.set(!inner.cursor_on.get());
            } else if !inner.cursor_on.get() {
                inner.cursor_on.set(true);
            } else {
                return glib::ControlFlow::Continue;
            }
            inner.area.queue_draw();
            glib::ControlFlow::Continue
        });
    }

    /// The draw callback: drain session events first (exit notices,
    /// clipboard requests), then paint the grid.
    fn wire_draw(inner: &Rc<Inner>) {
        let weak = Rc::downgrade(inner);
        inner.area.set_draw_func(move |_area, cr, width, height| {
            let Some(inner) = weak.upgrade() else { return };
            inner.handle_events();
            inner.update_size(width, height);
            inner.sync_scrollbar();
            inner.paint(cr);
        });
    }
}

impl Inner {
    fn upgrade(&self) -> EmbeddedTerminal {
        EmbeddedTerminal {
            inner: self.me.borrow().upgrade().expect("Inner outlives its cells"),
        }
    }

    // ── events from the emulator ───────────────────────────────────────

    fn handle_events(&self) {
        let events: Vec<SessionEvent> = self.session.borrow().drain_events().collect();
        for event in events {
            match event {
                SessionEvent::Wakeup | SessionEvent::Title(_) => {}
                SessionEvent::Bell => self.area.error_bell(),
                SessionEvent::Exited(_) => {
                    self.alive.set(false);
                    let terminal = self.upgrade();
                    for handler in self.exited.borrow().iter() {
                        handler(&terminal);
                    }
                }
                SessionEvent::SetClipboard(ty, text) => {
                    self.clipboard(ty).set_text(&text);
                }
                SessionEvent::ClipboardQuery(ty, format) => {
                    let me = self.me.borrow().clone();
                    self.clipboard(ty).read_text_async(
                        gtk4::gio::Cancellable::NONE,
                        move |result| {
                            if let Some(inner) = me.upgrade() {
                                // Empty or unavailable clipboards still
                                // need an OSC 52 reply; waiting applications
                                // must not hang indefinitely on a query.
                                let text = result.ok().flatten().unwrap_or_default();
                                inner.session.borrow().write(format(&text).into_bytes());
                            }
                        },
                    );
                }
                SessionEvent::ColorQuery(index, format) => {
                    let rgb = self.query_rgb(index);
                    self.session.borrow().write(format(rgb).into_bytes());
                }
                SessionEvent::SizeQuery(format) => {
                    let (cols, rows) = self.grid_size();
                    let (cw, ch) = self.cell.get();
                    self.session.borrow().write(
                        format(WindowSize {
                            num_lines: rows as u16,
                            num_cols: cols as u16,
                            cell_width: cw as u16,
                            cell_height: ch as u16,
                        })
                        .into_bytes(),
                    );
                }
            }
        }
    }

    fn clipboard(&self, ty: terminal_core::ClipboardType) -> gdk::Clipboard {
        let display = self.area.display();
        match ty {
            terminal_core::ClipboardType::Clipboard => display.clipboard(),
            terminal_core::ClipboardType::Selection => {
                #[cfg(unix)]
                { display.primary_clipboard() }
                // Windows has one system clipboard and no primary
                // selection. OSC 52's selection target uses that clipboard.
                #[cfg(windows)]
                { display.clipboard() }
            }
        }
    }

    /// Current value of a palette slot for OSC 4/10/11/12 queries: the
    /// emulator's override table first, then our theme palette.
    fn query_rgb(&self, index: usize) -> Rgb {
        let session = self.session.borrow();
        let term = session.term.lock();
        let set = term.colors()[index];
        drop(term);
        drop(session);
        if let Some(rgb) = set {
            return rgb;
        }
        let pal = palette(adw::StyleManager::default().is_dark());
        let (r, g, b) = match index {
            0..=15 => pal.ansi[index],
            16..=255 => indexed_rgb(index as u8, &pal),
            256 => pal.fg,
            257 => pal.bg,
            _ => pal.cursor,
        };
        Rgb { r: (r * 255.0) as u8, g: (g * 255.0) as u8, b: (b * 255.0) as u8 }
    }

    // ── keys ───────────────────────────────────────────────────────────

    fn key_pressed(
        &self,
        ctl: &gtk4::EventControllerKey,
        keyval: gdk::Key,
        state: gdk::ModifierType,
    ) -> glib::Propagation {
        if let Some(ev) = ctl.current_event() {
            if self.im.filter_keypress(&ev) {
                return glib::Propagation::Stop;
            }
        }
        let mods = Modifiers {
            shift: state.contains(gdk::ModifierType::SHIFT_MASK),
            ctrl: state.contains(gdk::ModifierType::CONTROL_MASK),
            alt: state.contains(gdk::ModifierType::ALT_MASK),
        };
        // Terminal-local chords before encoding.
        if mods.ctrl && !mods.shift && !mods.alt
            && matches!(keyval, gdk::Key::Insert | gdk::Key::KP_Insert)
        {
            self.copy_selection();
            return glib::Propagation::Stop;
        }
        if mods.ctrl && mods.shift {
            match keyval {
                gdk::Key::C | gdk::Key::c => {
                    self.copy_selection();
                    return glib::Propagation::Stop;
                }
                gdk::Key::V | gdk::Key::v => {
                    self.paste_clipboard();
                    return glib::Propagation::Stop;
                }
                _ => {}
            }
        }
        if mods.shift && !mods.ctrl && !mods.alt {
            match keyval {
                gdk::Key::Page_Up | gdk::Key::KP_Page_Up => {
                    self.session.borrow().scroll(Scroll::PageUp);
                    self.area.queue_draw();
                    return glib::Propagation::Stop;
                }
                gdk::Key::Page_Down | gdk::Key::KP_Page_Down => {
                    self.session.borrow().scroll(Scroll::PageDown);
                    self.area.queue_draw();
                    return glib::Propagation::Stop;
                }
                gdk::Key::Insert | gdk::Key::KP_Insert => {
                    self.paste_clipboard();
                    return glib::Propagation::Stop;
                }
                gdk::Key::Home | gdk::Key::KP_Home => {
                    self.session.borrow().scroll(Scroll::Top);
                    self.area.queue_draw();
                    return glib::Propagation::Stop;
                }
                gdk::Key::End | gdk::Key::KP_End => {
                    self.scroll_to_bottom();
                    return glib::Propagation::Stop;
                }
                _ => {}
            }
        }
        let modes = self.key_modes();
        let encoded = match gdk_key(keyval) {
            Some(key) => encode_key(key, mods, modes),
            None => keyval
                .to_unicode()
                .and_then(|c| encode_key(Key::Char(c), mods, modes)),
        };
        if let Some(bytes) = encoded {
            self.session.borrow().write(bytes);
            self.scroll_to_bottom();
            return glib::Propagation::Stop;
        }
        // Printable text without Ctrl/Alt — the IM filter already passed;
        // feed it to the shell like a text widget would.
        if !mods.ctrl && !mods.alt {
            if let Some(c) = keyval.to_unicode().filter(|c| !c.is_control()) {
                let mut buf = [0u8; 4];
                self.session.borrow().write(c.encode_utf8(&mut buf).as_bytes().to_vec());
                self.scroll_to_bottom();
                return glib::Propagation::Stop;
            }
        }
        glib::Propagation::Proceed
    }

    fn key_modes(&self) -> KeyModes {
        let session = self.session.borrow();
        let mode = *session.term.lock().mode();
        KeyModes {
            app_cursor_keys: mode.contains(TermMode::APP_CURSOR),
            app_keypad: mode.contains(TermMode::APP_KEYPAD),
            bracketed_paste: mode.contains(TermMode::BRACKETED_PASTE),
        }
    }

    // ── application mouse / focus reports ──────────────────────────────

    fn mouse_tracking(&self, state: gdk::ModifierType) -> bool {
        !state.contains(gdk::ModifierType::SHIFT_MASK)
            && self.session.borrow().term.lock().mode().intersects(TermMode::MOUSE_MODE)
    }

    fn mouse_cell(&self, x: f64, y: f64) -> (usize, usize) {
        let (cw, ch) = self.cell.get();
        let session = self.session.borrow();
        let term = session.term.lock();
        (
            (x / cw).clamp(0.0, term.columns().saturating_sub(1) as f64) as usize,
            (y / ch).clamp(0.0, term.screen_lines().saturating_sub(1) as f64) as usize,
        )
    }

    /// Shift keeps local selection/paste/scrolling available, just as in VTE.
    /// An active protocol consumes even an unencodable legacy coordinate.
    fn report_mouse(&self, button: u8, x: f64, y: f64, state: gdk::ModifierType, released: bool) -> bool {
        if !self.mouse_tracking(state) {
            return false;
        }
        let mode = *self.session.borrow().term.lock().mode();
        let code = button
            | if state.contains(gdk::ModifierType::ALT_MASK) { 8 } else { 0 }
            | if state.contains(gdk::ModifierType::CONTROL_MASK) { 16 } else { 0 };
        let (column, row) = self.mouse_cell(x, y);
        self.last_mouse_cell.set(Some((column, row)));
        if let Some(bytes) = encode_mouse(code, column, row, released,
            mode.contains(TermMode::SGR_MOUSE), mode.contains(TermMode::UTF8_MOUSE))
        {
            self.session.borrow().write(bytes);
        }
        true
    }

    fn report_motion(&self, x: f64, y: f64, state: gdk::ModifierType) {
        let mode = *self.session.borrow().term.lock().mode();
        let button = if state.contains(gdk::ModifierType::BUTTON1_MASK) { 0 }
            else if state.contains(gdk::ModifierType::BUTTON2_MASK) { 1 }
            else if state.contains(gdk::ModifierType::BUTTON3_MASK) { 2 }
            else { 3 };
        if (mode.contains(TermMode::MOUSE_MOTION)
            || (mode.contains(TermMode::MOUSE_DRAG) && button != 3))
            && self.last_mouse_cell.get() != Some(self.mouse_cell(x, y))
        {
            self.report_mouse(button | 32, x, y, state, false);
        }
    }

    fn report_focus(&self, focused: bool) {
        if self.session.borrow().term.lock().mode().contains(TermMode::FOCUS_IN_OUT) {
            self.session.borrow().write(if focused { b"\x1b[I".to_vec() } else { b"\x1b[O".to_vec() });
        }
    }

    // ── clipboard / selection ──────────────────────────────────────────

    fn copy_selection(&self) {
        let text = self.session.borrow().term.lock().selection_to_string();
        if let Some(text) = text {
            self.area.clipboard().set_text(&text);
        }
    }

    fn paste_clipboard(&self) {
        let modes = self.key_modes();
        let me = self.me.borrow().clone();
        self.area.clipboard().read_text_async(
            gtk4::gio::Cancellable::NONE,
            move |result| {
                if let (Ok(Some(text)), Some(inner)) = (result, me.upgrade()) {
                    inner.session.borrow().write(encode_paste(&text, modes));
                    inner.scroll_to_bottom();
                }
            },
        );
    }

    /// Middle-click paste (X11/Wayland primary selection).
    #[cfg(unix)]
    fn paste_primary(&self) {
        let modes = self.key_modes();
        let me = self.me.borrow().clone();
        self.area.display().primary_clipboard().read_text_async(
            gtk4::gio::Cancellable::NONE,
            move |result| {
                if let (Ok(Some(text)), Some(inner)) = (result, me.upgrade()) {
                    inner.session.borrow().write(encode_paste(&text, modes));
                }
            },
        );
    }

    fn selection_pressed(&self, n_press: i32, x: f64, y: f64) {
        let point = self.cell_point(x, y);
        let session = self.session.borrow();
        let mut term = session.term.lock();
        match n_press {
            // Double/triple click: word/line selection.
            2 => term.selection = Some(Selection::new(SelectionType::Semantic, point, Side::Right)),
            3 => term.selection = Some(Selection::new(SelectionType::Lines, point, Side::Right)),
            _ => {
                // Plain click clears the selection; the drag creates it.
                term.selection = None;
                self.sel_anchor.set(point);
            }
        }
        drop(term);
        drop(session);
        self.area.queue_draw();
    }

    fn selection_dragged(&self, x: f64, y: f64) {
        let point = self.cell_point(x, y);
        let session = self.session.borrow();
        let mut term = session.term.lock();
        if term.selection.is_none() {
            term.selection = Some(Selection::new(
                SelectionType::Simple,
                self.sel_anchor.get(),
                Side::Right,
            ));
        }
        if let Some(selection) = term.selection.as_mut() {
            let side = if point < self.sel_anchor.get() { Side::Left } else { Side::Right };
            selection.update(point, side);
        }
        drop(term);
        drop(session);
        self.area.queue_draw();
    }

    /// On release the selection goes to the primary clipboard (Unix), like
    /// other terminal emulators.
    fn selection_finished(&self) {
        #[cfg(unix)]
        {
            let text = self.session.borrow().term.lock().selection_to_string();
            if let Some(text) = text {
                self.area.display().primary_clipboard().set_text(&text);
            }
        }
    }

    /// Pixel → grid point (scrollback-aware via `display_offset`).
    fn cell_point(&self, x: f64, y: f64) -> Point {
        let (cw, ch) = self.cell.get();
        let session = self.session.borrow();
        let term = session.term.lock();
        let cols = term.grid().columns().max(1) as f64;
        let rows = term.grid().screen_lines().max(1) as f64;
        let offset = term.grid().display_offset();
        let col = (x / cw).clamp(0.0, cols - 1.0) as usize;
        let row = (y / ch).clamp(0.0, rows - 1.0) as usize;
        term::viewport_to_point(offset, Point::new(row, Column(col)))
    }

    // ── scrolling ──────────────────────────────────────────────────────

    fn wheel(&self, dx: f64, dy: f64, state: gdk::ModifierType) {
        if self.mouse_tracking(state) {
            let (x, y) = self.pointer.get();
            let (rx, ry) = self.wheel_remainder.get();
            let deltas = (rx + dx * 3.0, ry + dy * 3.0);
            for (delta, negative, positive) in [(deltas.0, 66, 67), (deltas.1, 64, 65)] {
                for _ in 0..delta.abs().floor() as usize {
                    self.report_mouse(if delta < 0.0 { negative } else { positive }, x, y, state, false);
                }
            }
            self.wheel_remainder.set((deltas.0.fract(), deltas.1.fract()));
            return;
        }
        self.wheel_remainder.set((0.0, 0.0));
        if dy == 0.0 {
            return;
        }
        let session = self.session.borrow();
        let mut term = session.term.lock();
        let mode = *term.mode();
        if mode.contains(TermMode::ALT_SCREEN) && mode.contains(TermMode::ALTERNATE_SCROLL)
            && !state.contains(gdk::ModifierType::SHIFT_MASK)
        {
            // VTE sends arrow keys to full-screen apps instead of scrolling.
            let key: &[u8] = if dy < 0.0 { b"\x1bOA" } else { b"\x1bOB" };
            drop(term);
            drop(session);
            for _ in 0..3 {
                self.session.borrow().write(key.to_vec());
            }
            return;
        }
        term.scroll_display(Scroll::Delta(-(dy * 3.0) as i32));
        drop(term);
        drop(session);
        self.area.queue_draw();
    }

    fn scrollbar_moved(&self, adj: &gtk4::Adjustment) {
        if self.syncing_scroll.get() {
            return;
        }
        let session = self.session.borrow();
        let mut term = session.term.lock();
        let history = term.grid().history_size() as f64;
        let target = (history - adj.value()) as i32;
        let delta = term.grid().display_offset() as i32 - target;
        term.scroll_display(Scroll::Delta(delta));
        drop(term);
        drop(session);
        self.area.queue_draw();
    }

    fn scroll_to_bottom(&self) {
        self.session.borrow().term.lock().scroll_display(Scroll::Bottom);
        self.area.queue_draw();
    }

    /// Map `value = history - display_offset` (bottom = live view).
    fn sync_scrollbar(&self) {
        let session = self.session.borrow();
        let (history, rows, offset) = {
            let term = session.term.lock();
            (
                term.grid().history_size() as f64,
                term.grid().screen_lines() as f64,
                term.grid().display_offset() as f64,
            )
        };
        drop(session);
        self.syncing_scroll.set(true);
        self.adjustment.set_upper(history + rows);
        self.adjustment.set_page_size(rows);
        self.adjustment.set_value(history - offset);
        self.syncing_scroll.set(false);
        if history > 0.0 && self.scrollbar.parent().is_none() {
            self.overlay.add_overlay(&self.scrollbar);
        }
        self.scrollbar.set_visible(history > 0.0);
    }

    // ── metrics / resize ───────────────────────────────────────────────

    /// Measure the real cell: monospace advance via "M" run, height via
    /// font metrics. Returns whether the metrics changed.
    fn measure(&self) -> bool {
        if self.area.width() <= 0 {
            return false;
        }
        let font = self.font.borrow().clone();
        let layout = self.area.create_pango_layout(Some("MMMMMMMMMM"));
        layout.set_font_description(Some(&font));
        let (w, _) = layout.pixel_size();
        let metrics = self.area.create_pango_context().metrics(Some(&font), None);
        let height = (metrics.ascent() + metrics.descent()) as f64 / pango::SCALE as f64;
        let cell = ((w as f64 / 10.0).max(1.0), height.max(1.0));
        let changed = cell != self.cell.get();
        self.cell.set(cell);
        changed
    }

    /// Re-measure + resize when the allocation or font changes. Called from
    /// the draw callback so it runs on every size change.
    fn update_size(&self, width: i32, height: i32) {
        let metrics_changed = self.measure();
        let (cw, ch) = self.cell.get();
        let (cols, rows) = self.grid_size_for(width as f64, height as f64, cw, ch);
        let session = self.session.borrow();
        let (cur_cols, cur_rows) = {
            let term = session.term.lock();
            (term.grid().columns(), term.grid().screen_lines())
        };
        if metrics_changed || cols != cur_cols || rows != cur_rows {
            session.resize(cols, rows, cw as u16, ch as u16);
        }
    }

    fn resize_to_fit(&self) {
        self.update_size(self.area.width(), self.area.height());
    }

    fn grid_size(&self) -> (usize, usize) {
        let (cw, ch) = self.cell.get();
        self.grid_size_for(self.area.width() as f64, self.area.height() as f64, cw, ch)
    }

    fn grid_size_for(&self, w: f64, h: f64, cw: f64, ch: f64) -> (usize, usize) {
        ((w / cw).floor().max(2.0) as usize, (h / ch).floor().max(1.0) as usize)
    }

    // ── rendering ──────────────────────────────────────────────────────

    fn paint(&self, cr: &gtk4::cairo::Context) {
        let pal = palette(adw::StyleManager::default().is_dark());
        let session = self.session.borrow();
        let term = session.term.lock();
        let content = term.renderable_content();
        let (cw, ch) = self.cell.get();
        let offset = content.display_offset as i32;
        let cols = term.grid().columns();
        let rows = term.grid().screen_lines();

        cr.set_source_rgb(pal.bg.0, pal.bg.1, pal.bg.2);
        cr.paint().ok();

        // Group each row's cells into runs of equal style.
        struct Run {
            col_start: usize,
            span: usize,
            text: String,
            fg: (f64, f64, f64),
            bg: (f64, f64, f64),
            flags: Flags,
        }
        let mut line_runs: Vec<Vec<Run>> = (0..rows).map(|_| Vec::new()).collect();
        for indexed in content.display_iter {
            let cell = indexed.cell;
            let row = indexed.point.line.0 + offset;
            if !(0..rows as i32).contains(&row) {
                continue;
            }
            let row = row as usize;
            // Wide-char spacers are covered by the wide char's run; extend
            // the span so backgrounds/selection stay contiguous.
            if cell
                .flags
                .intersects(Flags::WIDE_CHAR_SPACER | Flags::LEADING_WIDE_CHAR_SPACER)
            {
                if let Some(run) = line_runs[row].last_mut() {
                    run.span += 1;
                }
                continue;
            }
            let (mut fg, mut bg) = cell_colors(cell.fg, cell.bg, cell.flags, &pal, content.colors);
            if cell.flags.contains(Flags::INVERSE) {
                std::mem::swap(&mut fg, &mut bg);
            }
            if cell.flags.contains(Flags::HIDDEN) {
                fg = bg;
            }
            let mut text = String::new();
            text.push(cell.c);
            if let Some(extra) = cell.zerowidth() {
                text.extend(extra.iter());
            }
            let col = indexed.point.column.0;
            match line_runs[row].last_mut() {
                Some(run)
                    if run.fg == fg
                        && run.bg == bg
                        && run.flags == cell.flags
                        && run.col_start + run.span == col =>
                {
                    run.text.push_str(&text);
                    run.span += 1;
                }
                _ => line_runs[row].push(Run { col_start: col, span: 1, text, fg, bg, flags: cell.flags }),
            }
        }

        // Cell backgrounds.
        for (row, runs) in line_runs.iter().enumerate() {
            for run in runs {
                if run.bg != pal.bg {
                    cr.set_source_rgb(run.bg.0, run.bg.1, run.bg.2);
                    cr.rectangle(run.col_start as f64 * cw, row as f64 * ch, run.span as f64 * cw, ch);
                    cr.fill().ok();
                }
            }
        }

        // Selection overlay.
        if let Some(selection) = content.selection {
            cr.set_source_rgba(pal.selection.0, pal.selection.1, pal.selection.2, pal.selection.3);
            for row in 0..rows {
                for col in 0..cols {
                    let point = Point::new(Line(row as i32 - offset), Column(col));
                    if selection.contains(point) {
                        cr.rectangle(col as f64 * cw, row as f64 * ch, cw, ch);
                    }
                }
            }
            cr.fill().ok();
        }

        // Text runs.
        let layout = self.area.create_pango_layout(None);
        layout.set_font_description(Some(&self.font.borrow()));
        for (row, runs) in line_runs.iter().enumerate() {
            for run in runs {
                if run.text.trim().is_empty() {
                    continue;
                }
                layout.set_text(&run.text);
                let attrs = pango::AttrList::new();
                attrs.insert(pango::AttrColor::new_foreground(
                    (run.fg.0 * 65535.0) as u16,
                    (run.fg.1 * 65535.0) as u16,
                    (run.fg.2 * 65535.0) as u16,
                ));
                if run.flags.intersects(Flags::BOLD | Flags::DIM_BOLD) {
                    attrs.insert(pango::AttrInt::new_weight(pango::Weight::Bold));
                }
                if run.flags.contains(Flags::ITALIC) {
                    attrs.insert(pango::AttrInt::new_style(pango::Style::Italic));
                }
                if run.flags.contains(Flags::DOUBLE_UNDERLINE) {
                    attrs.insert(pango::AttrInt::new_underline(pango::Underline::Double));
                } else if run.flags.intersects(Flags::ALL_UNDERLINES) {
                    attrs.insert(pango::AttrInt::new_underline(pango::Underline::Single));
                }
                if run.flags.contains(Flags::STRIKEOUT) {
                    attrs.insert(pango::AttrInt::new_strikethrough(true));
                }
                layout.set_attributes(Some(&attrs));
                cr.move_to(run.col_start as f64 * cw, row as f64 * ch);
                pangocairo::functions::show_layout(cr, &layout);
            }
        }

        let cursor = content.cursor;
        let crow = cursor.point.line.0 + offset;

        // IME preedit at the cursor.
        let preedit = self.preedit.borrow().clone();
        if !preedit.is_empty() && (0..rows as i32).contains(&crow) {
            let x = cursor.point.column.0 as f64 * cw;
            let y = crow as f64 * ch;
            cr.set_source_rgba(pal.fg.0, pal.fg.1, pal.fg.2, 0.25);
            cr.rectangle(x, y, preedit.chars().count() as f64 * cw, ch);
            cr.fill().ok();
            layout.set_text(&preedit);
            let attrs = pango::AttrList::new();
            attrs.insert(pango::AttrInt::new_underline(pango::Underline::Single));
            attrs.insert(pango::AttrColor::new_foreground(
                (pal.fg.0 * 65535.0) as u16,
                (pal.fg.1 * 65535.0) as u16,
                (pal.fg.2 * 65535.0) as u16,
            ));
            layout.set_attributes(Some(&attrs));
            cr.move_to(x, y);
            pangocairo::functions::show_layout(cr, &layout);
            self.im.set_cursor_location(&gdk::Rectangle::new(
                x as i32,
                (y + ch) as i32,
                cw as i32,
                ch as i32,
            ));
        }

        // Cursor.
        if cursor.shape != CursorShape::Hidden
            && self.cursor_on.get()
            && (0..rows as i32).contains(&crow)
        {
            let x = cursor.point.column.0 as f64 * cw;
            let y = crow as f64 * ch;
            cr.set_source_rgb(pal.cursor.0, pal.cursor.1, pal.cursor.2);
            let shape = if self.area.has_focus() { cursor.shape } else { CursorShape::HollowBlock };
            match shape {
                CursorShape::Block => {
                    cr.set_source_rgba(pal.cursor.0, pal.cursor.1, pal.cursor.2, 0.55);
                    cr.rectangle(x, y, cw, ch);
                    cr.fill().ok();
                }
                CursorShape::Underline => {
                    cr.rectangle(x, y + ch - 2.0, cw, 2.0);
                    cr.fill().ok();
                }
                CursorShape::Beam => {
                    cr.rectangle(x, y, 2.0, ch);
                    cr.fill().ok();
                }
                CursorShape::HollowBlock | CursorShape::Hidden => {
                    cr.rectangle(x, y, cw, ch);
                    cr.set_line_width(1.0);
                    cr.stroke().ok();
                }
            }
        }
    }
}

/// Resolve a cell's fg/bg: OSC palette overrides first, then the theme
/// palette; bold selects the bright slot like VTE, DIM fades toward bg.
fn cell_colors(
    fg: Color,
    bg: Color,
    flags: Flags,
    pal: &Palette,
    overrides: &term::color::Colors,
) -> ((f64, f64, f64), (f64, f64, f64)) {
    (
        resolve_color(fg, pal, overrides, flags.contains(Flags::BOLD), pal.fg),
        resolve_color(bg, pal, overrides, false, pal.bg),
    )
}

fn resolve_color(
    color: Color,
    pal: &Palette,
    overrides: &term::color::Colors,
    bold: bool,
    default: (f64, f64, f64),
) -> (f64, f64, f64) {
    let rgb = |r: Rgb| (r.r as f64 / 255.0, r.g as f64 / 255.0, r.b as f64 / 255.0);
    match color {
        Color::Spec(rgb) => (rgb.r as f64 / 255.0, rgb.g as f64 / 255.0, rgb.b as f64 / 255.0),
        Color::Named(named) => {
            if let Some(c) = overrides[named] {
                return rgb(c);
            }
            let named = if bold { named.to_bright() } else { named };
            match named {
                NamedColor::Foreground | NamedColor::BrightForeground => pal.fg,
                NamedColor::Background => pal.bg,
                NamedColor::Cursor => pal.cursor,
                NamedColor::DimForeground => dim(pal.fg, pal.bg),
                // Black..BrightWhite are indices 0-15; Dim* follow the
                // named slots (259+).
                n if (n as usize) < 16 => pal.ansi[n as usize],
                n => dim(pal.ansi[n as usize - NamedColor::DimBlack as usize], pal.bg),
            }
        }
        Color::Indexed(i) => {
            if let Some(c) = overrides[i as usize] {
                return rgb(c);
            }
            let i = if bold && i < 8 { i + 8 } else { i };
            let _ = default;
            indexed_rgb(i, pal)
        }
    }
}

/// `gdk::Key` → portable key; `None` means "let the text path handle it".
fn gdk_key(key: gdk::Key) -> Option<Key> {
    Some(match key {
        gdk::Key::Up | gdk::Key::KP_Up => Key::Up,
        gdk::Key::Down | gdk::Key::KP_Down => Key::Down,
        gdk::Key::Left | gdk::Key::KP_Left => Key::Left,
        gdk::Key::Right | gdk::Key::KP_Right => Key::Right,
        gdk::Key::Home | gdk::Key::KP_Home => Key::Home,
        gdk::Key::End | gdk::Key::KP_End => Key::End,
        gdk::Key::KP_Begin => Key::Begin,
        gdk::Key::Page_Up | gdk::Key::KP_Page_Up => Key::PageUp,
        gdk::Key::Page_Down | gdk::Key::KP_Page_Down => Key::PageDown,
        gdk::Key::Insert | gdk::Key::KP_Insert => Key::Insert,
        gdk::Key::Delete | gdk::Key::KP_Delete => Key::Delete,
        gdk::Key::BackSpace => Key::Backspace,
        gdk::Key::Tab | gdk::Key::ISO_Left_Tab => Key::Tab,
        gdk::Key::KP_Space => Key::Keypad(' '),
        gdk::Key::KP_Tab => Key::Keypad('\t'),
        gdk::Key::KP_Separator => Key::Keypad(','),
        gdk::Key::Return | gdk::Key::KP_Enter | gdk::Key::ISO_Enter => Key::Enter,
        gdk::Key::Escape => Key::Escape,
        gdk::Key::KP_F1 => Key::KeypadF(1),
        gdk::Key::KP_F2 => Key::KeypadF(2),
        gdk::Key::KP_F3 => Key::KeypadF(3),
        gdk::Key::KP_F4 => Key::KeypadF(4),
        gdk::Key::F1 => Key::F(1),
        gdk::Key::F2 => Key::F(2),
        gdk::Key::F3 => Key::F(3),
        gdk::Key::F4 => Key::F(4),
        gdk::Key::F5 => Key::F(5),
        gdk::Key::F6 => Key::F(6),
        gdk::Key::F7 => Key::F(7),
        gdk::Key::F8 => Key::F(8),
        gdk::Key::F9 => Key::F(9),
        gdk::Key::F10 => Key::F(10),
        gdk::Key::F11 => Key::F(11),
        gdk::Key::F12 => Key::F(12),
        _ => return None,
    })
}

#[cfg(all(test, unix))]
mod application_input_tests {
    use super::*;
    use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

    #[test]
    #[ignore = "requires a GTK display; run alone"]
    fn application_mouse_focus_and_shift_selection_reach_the_pty() {
        gtk4::init().unwrap();
        let root = std::env::temp_dir().join(format!("pitex-terminal-input-{}-{}",
            std::process::id(), SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()));
        std::fs::create_dir(&root).unwrap();
        struct Cleanup(std::path::PathBuf);
        impl Drop for Cleanup {
            fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.0); }
        }
        let _cleanup = Cleanup(root.clone());
        let capture = root.join("input");
        let terminal = EmbeddedTerminal::new();
        terminal.inner.session.borrow_mut().spawn(terminal_core::SpawnOptions {
            shell: Some(terminal_core::Shell::new("/bin/bash".into(), vec!["-c".into(),
                format!("stty raw -echo; printf '\\033[?1000h\\033[?1006h\\033[?1004h'; cat > '{}'", capture.display())])),
            working_directory: Some(root.clone()), env: Vec::new(),
            cols: 80, rows: 24, cell_width: 9, cell_height: 17,
        }).unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        while !capture.exists() || !terminal.inner.mouse_tracking(gdk::ModifierType::empty()) {
            assert!(Instant::now() < deadline, "PTY child did not enable mouse reporting");
            std::thread::sleep(Duration::from_millis(5));
        }
        let inner = &terminal.inner;
        let none = gdk::ModifierType::empty();
        let shift = gdk::ModifierType::SHIFT_MASK;
        inner.report_focus(false);
        inner.report_focus(true);
        assert!(inner.report_mouse(0, 36.0, 34.0, none, false));
        assert!(inner.report_mouse(0, 36.0, 34.0, none, true));
        inner.report_motion(54.0, 51.0, gdk::ModifierType::BUTTON1_MASK);
        assert!(!inner.report_mouse(0, 36.0, 34.0, shift, false));
        inner.selection_pressed(1, 0.0, 0.0);
        inner.selection_dragged(36.0, 0.0);
        assert!(inner.session.borrow().term.lock().selection.is_some());
        inner.pointer.set((36.0, 34.0));
        inner.wheel(0.0, -1.0, none);
        inner.report_mouse(2, 36.0, 34.0,
            gdk::ModifierType::CONTROL_MASK | gdk::ModifierType::ALT_MASK, false);
        inner.session.borrow().feed(b"\x1b[?1000l\x1b[?1002h");
        inner.report_motion(54.0, 51.0, none); // Drag mode requires a held button.
        inner.report_motion(54.0, 51.0, gdk::ModifierType::BUTTON1_MASK);
        inner.report_motion(55.0, 52.0, gdk::ModifierType::BUTTON1_MASK); // Same cell.
        inner.session.borrow().feed(b"\x1b[?1002l\x1b[?1003h");
        inner.report_motion(63.0, 51.0, none);
        inner.session.borrow().feed(b"\x1b[?1004l\x1b[?1003l");
        inner.report_focus(false);
        assert!(!inner.report_mouse(0, 36.0, 34.0, none, false));
        let controller = gtk4::EventControllerKey::new();
        inner.session.borrow().feed(b"\x1b=\x1b[?1l");
        for key in [gdk::Key::KP_Begin, gdk::Key::KP_F1, gdk::Key::KP_F2,
            gdk::Key::KP_F3, gdk::Key::KP_F4, gdk::Key::KP_Space,
            gdk::Key::KP_Tab, gdk::Key::KP_Separator, gdk::Key::KP_Enter]
        {
            assert_eq!(inner.key_pressed(&controller, key, none), glib::Propagation::Stop);
        }
        inner.session.borrow().feed(b"\x1b>\x1b[?1h");
        for key in [gdk::Key::KP_Begin, gdk::Key::KP_Space, gdk::Key::KP_Tab,
            gdk::Key::KP_Separator, gdk::Key::KP_Enter]
        {
            assert_eq!(inner.key_pressed(&controller, key, none), glib::Propagation::Stop);
        }
        let expected = b"\x1b[O\x1b[I\x1b[<0;5;3M\x1b[<0;5;3m\x1b[<64;5;3M\x1b[<64;5;3M\x1b[<64;5;3M\x1b[<26;5;3M\x1b[<32;7;4M\x1b[<35;8;4M\x1b[E\x1bOP\x1bOQ\x1bOR\x1bOS\x1bO \x1bOI\x1bOl\r\x1bOE \t,\r";
        while std::fs::read(&capture).unwrap() != expected {
            assert!(Instant::now() < deadline, "Unexpected PTY input: {:?}", std::fs::read(&capture).unwrap());
            std::thread::sleep(Duration::from_millis(5));
        }
        // Local navigation/copy must not send application key sequences.
        for _ in 0..40 { inner.session.borrow().feed(b"history\r\n"); }
        assert_eq!(inner.key_pressed(&controller, gdk::Key::Home, shift), glib::Propagation::Stop);
        {
            let session = inner.session.borrow();
            let term = session.term.lock();
            assert!(term.grid().history_size() > 0);
            assert_eq!(term.grid().display_offset(), term.grid().history_size());
        }
        assert_eq!(inner.key_pressed(&controller, gdk::Key::End, shift), glib::Propagation::Stop);
        assert_eq!(inner.session.borrow().term.lock().grid().display_offset(), 0);
        inner.session.borrow().feed(b"\x1b[2J\x1b[Hcopy-me");
        inner.selection_pressed(2, 0.0, 0.0);
        let selection = inner.session.borrow().term.lock().selection_to_string().unwrap();
        assert_eq!(selection, "copy-me");
        assert_eq!(inner.key_pressed(&controller, gdk::Key::Insert, gdk::ModifierType::CONTROL_MASK),
            glib::Propagation::Stop);
        assert_eq!(inner.key_pressed(&controller, gdk::Key::Insert, shift), glib::Propagation::Stop);
        let deadline = Instant::now() + Duration::from_secs(5);
        let expected = [expected.as_slice(), b"copy-me"].concat();
        while std::fs::read(&capture).unwrap() != expected {
            while glib::MainContext::default().pending() { glib::MainContext::default().iteration(false); }
            assert!(Instant::now() < deadline, "Clipboard copy/paste did not reach PTY: {:?}", std::fs::read(&capture).unwrap());
            std::thread::sleep(Duration::from_millis(5));
        }
        inner.area.clipboard().set_text("한글\r\nsecond\nthird");
        inner.session.borrow().feed(b"\x1b[?2004h");
        assert_eq!(inner.key_pressed(&controller, gdk::Key::Insert, shift), glib::Propagation::Stop);
        let expected = [expected, "\x1b[200~한글\rsecond\rthird\x1b[201~".as_bytes().to_vec()].concat();
        while std::fs::read(&capture).unwrap() != expected {
            while glib::MainContext::default().pending() { glib::MainContext::default().iteration(false); }
            assert!(Instant::now() < deadline, "Bracketed CRLF paste did not reach PTY: {:?}", std::fs::read(&capture).unwrap());
            std::thread::sleep(Duration::from_millis(5));
        }
        // TUI applications query the same extended palette we draw. Cube
        // and grayscale entries must not fall back to the cursor color.
        inner.session.borrow().feed(b"\x1b]4;16;?\x07\x1b]4;231;?\x07\x1b]4;232;?\x07\x1b]4;255;?\x07");
        inner.handle_events();
        let expected = [expected, b"\x1b]4;16;rgb:0000/0000/0000\x07\x1b]4;231;rgb:ffff/ffff/ffff\x07\x1b]4;232;rgb:0808/0808/0808\x07\x1b]4;255;rgb:eeee/eeee/eeee\x07".to_vec()].concat();
        let deadline = Instant::now() + Duration::from_secs(5);
        while std::fs::read(&capture).unwrap() != expected {
            assert!(Instant::now() < deadline, "Extended palette replies did not reach PTY: {:?}", std::fs::read(&capture).unwrap());
            std::thread::sleep(Duration::from_millis(5));
        }
        inner.session.borrow().feed(b"\x1b]4;16;rgb:12/34/56\x07\x1b]4;16;?\x07");
        inner.handle_events();
        let expected = [expected, b"\x1b]4;16;rgb:1212/3434/5656\x07".to_vec()].concat();
        while std::fs::read(&capture).unwrap() != expected {
            assert!(Instant::now() < deadline, "Palette override reply did not reach PTY: {:?}", std::fs::read(&capture).unwrap());
            std::thread::sleep(Duration::from_millis(5));
        }
        eprintln!("PASS terminal mouse/focus/keypad, scroll/copy, UTF-8 CRLF paste and extended palette queries through GTK/PTY");
    }
}
