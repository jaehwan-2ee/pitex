//! Native GTK4/libadwaita application shell — the Linux port of the SwiftUI
//! `WorkspaceView`/`EditorContainerView` layer. All widgets are `Rc`-bound on
//! the main thread; `WorkspaceModel` async completions arrive through a
//! channel and are dispatched here.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::mpsc::Sender;
use std::time::Duration;

use app_ports::DocumentMutationResult;
#[cfg(unix)]
use linux_platform::LinuxEnvironment as PlatformEnvironment;
#[cfg(windows)]
use windows_platform::WindowsEnvironment as PlatformEnvironment;
use document_session_core::{DocumentMutation, DocumentSaveState, DocumentSession};
use gtk4::prelude::*;
use gtk4::{gdk, gio, glib};
use libadwaita as adw;
use adw::prelude::*;
use sourceview5::prelude::*;

use crate::compat;
use settings_feature::{BuildPreferences, EditorPreferences, PersistedSettings, ShellExecutionPreference};

use gtk_editor_adapter::{FnSessionClient, GtkEditorAdapter, SessionClient};
use language_core::{DeterministicTeXLexer, TeXDialect};

use crate::agent::{AgentCoordinator, AgentSelectionAttachment};
use crate::l10n::{resolve_language, tr};
use crate::model::{
    ConsoleSection, SidebarSection, WorkspaceBuildState, WorkspaceMessage, WorkspaceModel,
    WorkspacePhase, WorkspaceSyncTeXState,
};
use crate::pdf::PdfDocument;
use crate::settings::{AppearanceColorRole, AppearanceSettings, Preferences, SettingsStore, Theme};

pub const APP_ID: &str = "dev.pitex.app";

/// Widget name + accessible label — the GTK equivalent of SwiftUI's
/// `accessibilityIdentifier`/`accessibilityLabel`. Labels resolve through
/// `LANG` so the helper is safe to call while `AppState` is mutably borrowed.
pub fn a11y<T>(widget: &T, id: &str, label_key: &str)
where
    T: IsA<gtk4::Widget> + IsA<gtk4::Accessible>,
{
    widget.set_widget_name(id);
    let lang = LANG.with(|l| l.get());
    widget.update_property(&[gtk4::accessible::Property::Label(&tr(lang, label_key))]);
}

/// Every widget the refresh logic touches, in one place (SwiftUI's
/// `@Published` surface replaced by explicit handle updates).
#[derive(Default)]
pub struct UiHandles {
    pub root_stack: RefCell<Option<gtk4::Stack>>,
    pub status_page: RefCell<Option<adw::StatusPage>>,
    pub window: RefCell<Option<adw::ApplicationWindow>>,
    pub toast_overlay: RefCell<Option<adw::ToastOverlay>>,
    pub tab_row: RefCell<Option<gtk4::Box>>,
    pub outer_paned: RefCell<Option<gtk4::Paned>>,
    pub inner_paned: RefCell<Option<gtk4::Paned>>,
    pub console_paned: RefCell<Option<gtk4::Paned>>,
    pub editor_scroller: RefCell<Option<gtk4::ScrolledWindow>>,
    /// Overlay wrapping `editor_scroller`; hosts the fold chip layer.
    pub editor_overlay: RefCell<Option<gtk4::Overlay>>,
    pub editor_stack: RefCell<Option<gtk4::Stack>>,
    pub minimap: RefCell<Option<sourceview5::Map>>,
    pub footer: RefCell<Option<gtk4::Box>>,
    pub footer_path: RefCell<Option<gtk4::Label>>,
    pub footer_words: RefCell<Option<gtk4::Label>>,
    pub disk_status: RefCell<Option<gtk4::Button>>,
    pub conflict_banner: RefCell<Option<gtk4::Box>>,
    pub conflict_label: RefCell<Option<gtk4::Label>>,
    pub sidebar_stack: RefCell<Option<gtk4::Stack>>,
    pub sidebar_section_dropdown: RefCell<Option<gtk4::DropDown>>,
    pub outline_list: RefCell<Option<gtk4::ListBox>>,
    pub labels_list: RefCell<Option<gtk4::ListBox>>,
    pub bib_list: RefCell<Option<gtk4::ListBox>>,
    pub project_list: RefCell<Option<gtk4::ListBox>>,
    pub pin_button: RefCell<Option<gtk4::Button>>,
    /// `win.save` / `win.saveas` / `win.saveall` — enabled state mirrors the
    /// macOS File menu's `.disabled(...)` conditions.
    pub save_action: RefCell<Option<gio::SimpleAction>>,
    pub save_as_action: RefCell<Option<gio::SimpleAction>>,
    pub save_all_action: RefCell<Option<gio::SimpleAction>>,
    pub build_button: RefCell<Option<gtk4::Button>>,
    pub build_status: RefCell<Option<gtk4::Label>>,
    pub header_build_button: RefCell<Option<gtk4::Button>>,
    pub build_command_entry: RefCell<Option<gtk4::Entry>>,
    pub custom_command_entry: RefCell<Option<gtk4::Entry>>,
    pub console_section_dropdown: RefCell<Option<gtk4::DropDown>>,
    pub console_stack: RefCell<Option<gtk4::Stack>>,
    pub terminal: RefCell<Option<compat::ShellTerminal>>,
    pub issues_list: RefCell<Option<gtk4::ListBox>>,
    pub issue_filter: RefCell<Option<gtk4::DropDown>>,
    pub build_log_view: RefCell<Option<gtk4::TextView>>,
    pub pdf_toolbar: RefCell<Option<gtk4::Box>>,
    pub pdf_name_label: RefCell<Option<gtk4::Label>>,
    pub pdf_picture: RefCell<Option<gtk4::Picture>>,
    pub pdf_highlight: RefCell<Option<gtk4::DrawingArea>>,
    pub pdf_scroll: RefCell<Option<gtk4::ScrolledWindow>>,
    pub pdf_empty: RefCell<Option<adw::StatusPage>>,
    pub pdf_page_label: RefCell<Option<gtk4::Label>>,
    pub pdf_prev_button: RefCell<Option<gtk4::Button>>,
    pub pdf_next_button: RefCell<Option<gtk4::Button>>,
    pub synctex_status_icon: RefCell<Option<gtk4::Image>>,
    pub synctex_status_label: RefCell<Option<gtk4::Label>>,
    pub transcript_box: RefCell<Option<gtk4::Box>>,
    pub transcript_scroll: RefCell<Option<gtk4::ScrolledWindow>>,
    pub agent_model_picker: RefCell<Option<gtk4::DropDown>>,
    pub agent_reasoning_picker: RefCell<Option<gtk4::DropDown>>,
    pub agent_attach_toggle: RefCell<Option<gtk4::Switch>>,
    pub agent_composer: RefCell<Option<gtk4::Entry>>,
    /// Reloaded (not re-added) whenever `ai.fontSize` changes so the composer
    /// entry tracks the conversation font.
    pub agent_composer_font: RefCell<Option<gtk4::CssProvider>>,
    pub agent_send_button: RefCell<Option<gtk4::Button>>,
    pub agent_stop_button: RefCell<Option<gtk4::Button>>,
    pub agent_status_label: RefCell<Option<gtk4::Label>>,
    /// Right-aligned usage/context readout in the assistant controls row —
    /// populated from `get_session_stats`, hidden until stats arrive.
    pub agent_usage_label: RefCell<Option<gtk4::Label>>,
    pub selection_chip: RefCell<Option<gtk4::Box>>,
    pub selection_chip_label: RefCell<Option<gtk4::Label>>,
    pub search_bar: RefCell<Option<gtk4::SearchBar>>,
    pub search_entry: RefCell<Option<gtk4::SearchEntry>>,
    pub shell_warning: RefCell<Option<gtk4::Box>>,
    /// "+" menu button — its model is rebuilt on refresh so the Open Recent
    /// section mirrors `recent_documents` (macOS File → Open Recent).
    pub add_menu_button: RefCell<Option<gtk4::MenuButton>>,
    /// Header open button — the popover's anchor when `win.open` fires from
    /// a shortcut or menu item rather than a button click.
    pub open_button: RefCell<Option<gtk4::Button>>,
}

/// Shared application state — replaces SwiftUI's `@Published` propagation
/// with explicit refreshes after every mutation. Model `on_*` callbacks fire
/// while `AppState` is mutably borrowed, so they write into `Rc` cells which
/// `drain_side_effects` consumes on the main context.
pub struct AppState {
    pub model: WorkspaceModel,
    pub store: SettingsStore,
    pub appearance: AppearanceSettings,
    pub agent: Option<AgentCoordinator>,
    pub language: &'static str,
    pub terminal_running: bool,
    pub editor: Option<Rc<GtkEditorAdapter>>,
    /// The `FoldEngine` bound to the current adapter — rebuilt per session
    /// like the macOS environment's `adapter`.
    pub fold: Option<Rc<crate::fold::FoldEngine>>,
    /// The fold chip layer currently overlaid on the editor scroller —
    /// tracked so `rebind_editor_widget` can remove the previous one.
    pub fold_chip: RefCell<Option<gtk4::DrawingArea>>,
    /// `AppEnvironment` bundle — the Linux platform ports (`files` feeds the
    /// capability lease like `capabilityBroker`, `workspace` opens externals).
    pub env: PlatformEnvironment,
    /// `CFBundleShortVersionString` equivalent — compared against release
    /// tags by the updater.
    pub app_version: String,
    pub active_session: Option<DocumentSession>,
    /// Debounce source for post-edit re-highlighting (120ms like Swift).
    pub highlight_pending: Cell<bool>,
    /// Debounce for external-change coalescing (0.35s).
    pub disk_pending: RefCell<HashMap<PathBuf, glib::SourceId>>,
    pub watchers: Vec<gio::FileMonitor>,
    pub pdf: Option<PdfDocument>,
    pub pdf_page: usize,
    pub pdf_scale: f64,
    /// `pi` config dir monitor — restarts the agent when auth/models/
    /// settings change so provider edits apply without an app restart.
    pub agent_config_monitor: Option<gio::FileMonitor>,
    /// Debounce generation for the agent-config monitor (~500ms).
    pub agent_config_generation: Cell<u64>,
    /// PDFView `autoScales` — fit page width to the pane until the user zooms.
    pub pdf_auto_fit: bool,
    /// Hash of the PDF byte buffer currently loaded in `pdf`.
    pub pdf_hash: u64,
    /// Set by the coordinator's activity callback; consumed after `handle`.
    pub agent_activity_pending: Rc<Cell<bool>>,
    /// Staged for the coordinator's persist-dirty-sessions callback.
    pub persist_result: Rc<RefCell<Option<Option<String>>>>,
    /// Cached context the coordinator's provider closure reads.
    pub context_cell: Rc<RefCell<crate::agent::AgentContextSnapshot>>,
    /// `on_selection_attachment` → pending attachment update (Option = clear).
    pub attachment_cell: Rc<RefCell<Option<Option<AgentSelectionAttachment>>>>,
    /// `on_synctex_highlight` → (page, x, y, w, h) to draw.
    pub highlight_cell: Rc<RefCell<Option<(i64, f64, f64, f64, f64)>>>,
    /// Invalidates a pending forward-marker removal when a newer navigation
    /// arrives first (the Swift coordinator's highlightTask cancellation).
    pub highlight_generation: Rc<Cell<u64>>,
    /// `on_jump_to` → (line, column) to reveal.
    pub jump_cell: Rc<RefCell<Option<(usize, usize)>>>,
    /// `on_terminal_feed` / `on_terminal_send` queues.
    pub feed_queue: Rc<RefCell<Vec<String>>>,
    pub send_queue: Rc<RefCell<Vec<String>>>,
    /// One-shot flags for refresh scheduling from model callbacks.
    pub snapshot_flag: Rc<Cell<bool>>,
    pub structure_flag: Rc<Cell<bool>>,
    pub active_doc_flag: Rc<Cell<bool>>,
    pub autosave_flag: Rc<Cell<bool>>,
    pub tx: Option<Sender<WorkspaceMessage>>,
}

impl AppState {
    fn new(store: SettingsStore, tx: Sender<WorkspaceMessage>, app_version: String) -> Self {
        let mut model = WorkspaceModel::new();
        model.set_event_sink(tx.clone());
        model.load_recents(&store);
        let appearance = AppearanceSettings::new(store.prefs());
        let mut state = Self {
            model,
            store,
            appearance,
            agent: None,
            language: "en",
            terminal_running: false,
            editor: None,
            fold: None,
            fold_chip: RefCell::new(None),
            env: PlatformEnvironment::make("dev.pitex.app"),
            app_version,
            active_session: None,
            highlight_pending: Cell::new(false),
            disk_pending: RefCell::new(HashMap::new()),
            watchers: Vec::new(),
            pdf: None,
            pdf_page: 0,
            pdf_scale: 1.5,
            agent_config_monitor: None,
            agent_config_generation: Cell::new(0),
            pdf_auto_fit: true,
            pdf_hash: 0,
            agent_activity_pending: Rc::new(Cell::new(false)),
            persist_result: Rc::new(RefCell::new(None)),
            context_cell: Rc::new(RefCell::new(Default::default())),
            attachment_cell: Rc::new(RefCell::new(None)),
            highlight_cell: Rc::new(RefCell::new(None)),
            highlight_generation: Rc::new(Cell::new(0)),
            jump_cell: Rc::new(RefCell::new(None)),
            feed_queue: Rc::new(RefCell::new(Vec::new())),
            send_queue: Rc::new(RefCell::new(Vec::new())),
            snapshot_flag: Rc::new(Cell::new(false)),
            structure_flag: Rc::new(Cell::new(false)),
            active_doc_flag: Rc::new(Cell::new(false)),
            autosave_flag: Rc::new(Cell::new(false)),
            tx: Some(tx),
        };
        state.wire_model_callbacks();
        state
    }

    /// Wire `WorkspaceModel.on_*` callbacks into the `Rc` side-effect cells.
    fn wire_model_callbacks(&mut self) {
        let highlight = self.highlight_cell.clone();
        self.model.on_synctex_highlight = Some(Box::new(move |p, x, y, w, h| {
            *highlight.borrow_mut() = Some((p, x, y, w, h));
        }));
        let jump = self.jump_cell.clone();
        self.model.on_jump_to = Some(Box::new(move |line, col| {
            *jump.borrow_mut() = Some((line, col));
        }));
        let attachment = self.attachment_cell.clone();
        self.model.on_selection_attachment = Some(Box::new(move |a| {
            *attachment.borrow_mut() = Some(a.map(|(path, start, end, text)| {
                AgentSelectionAttachment {
                    path,
                    start_line: start,
                    end_line: end,
                    text,
                }
            }));
        }));
        let feed = self.feed_queue.clone();
        self.model.on_terminal_feed = Some(Box::new(move |text| {
            if let Ok(mut s) = feed.try_borrow_mut() { s.push(text); }
        }));
        let send = self.send_queue.clone();
        self.model.on_terminal_send = Some(Box::new(move |cmd| {
            if let Ok(mut s) = send.try_borrow_mut() { s.push(cmd); }
        }));
        let flag = self.snapshot_flag.clone();
        self.model.on_snapshot_changed = Some(Box::new(move |_| flag.set(true)));
        let flag = self.structure_flag.clone();
        self.model.on_structure_changed = Some(Box::new(move || flag.set(true)));
        let flag = self.active_doc_flag.clone();
        self.model.on_active_document_changed = Some(Box::new(move || flag.set(true)));
        let flag = self.autosave_flag.clone();
        self.model.on_autosave_schedule = Some(Box::new(move || flag.set(true)));
    }

    /// Consume queued side effects written by model callbacks. Runs at the
    /// end of every `dispatch` and on the agent poll tick.
    fn drain_side_effects(&mut self) {
        let feeds: Vec<String> = self.feed_queue.borrow_mut().drain(..).collect();
        for text in feeds {
            self.terminal_feed(&text);
        }
        let sends: Vec<String> = self.send_queue.borrow_mut().drain(..).collect();
        for cmd in sends {
            self.terminal_send(&cmd);
        }
        // Deferred inverse-SyncTeX jump: `apply_inverse_result` parks the
        // target in `pending_jump` while the owning document activates, then
        // `apply_activate` republishes it through `on_jump_to`. Draining here
        // is what lets a cross-file navigation land its caret at all.
        let jump = self.jump_cell.borrow_mut().take();
        if let Some((line, col)) = jump {
            let highlight = self.store.inverse_sync_highlight();
            self.jump_to(line, col, highlight);
        }
        let attachment = self.attachment_cell.borrow_mut().take();
        if let Some(attachment) = attachment {
            self.ensure_agent();
            if let Some(agent) = self.agent.as_mut() {
                agent.update_selection_attachment(attachment);
            }
            self.refresh_selection_chip();
        }
        if self.structure_flag.replace(false) {
            self.refresh_sidebar();
        }
        let active_changed = self.active_doc_flag.replace(false);
        if self.snapshot_flag.replace(false) || active_changed {
            self.refresh_after_document_change();
        }
        if active_changed && self.model.has_project() {
            // `agent = coordinator` + `coordinator.prepare()` — the Swift
            // workspace creates and prepares the coordinator inside `open()`.
            self.ensure_agent();
        }
        if self.autosave_flag.replace(false) {
            self.schedule_autosave();
        }
    }

    // ── helpers used by panes.rs ────────────────────────────────────────────

    pub fn editor_font_desc(&self) -> gtk4::pango::FontDescription {
        let mut desc = gtk4::pango::FontDescription::from_string(&self.appearance.font_description());
        if desc.size() <= 0 {
            desc.set_size(12 * gtk4::pango::SCALE);
        }
        desc
    }

    /// Terminal font: custom family when set, otherwise the editor font.
    pub fn terminal_font_desc(&self) -> gtk4::pango::FontDescription {
        if self.appearance.terminal_font_family.is_empty() {
            return self.editor_font_desc();
        }
        let mut desc = gtk4::pango::FontDescription::from_string(
            &self.appearance.terminal_font_description(),
        );
        if desc.size() <= 0 {
            desc.set_size(13 * gtk4::pango::SCALE);
        }
        desc
    }

    /// Name of the rendered PDF, matching the built target (or active doc).
    pub fn pdf_display_name(&self) -> String {
        self.model
            .latest_built_pdf_name
            .clone()
            .unwrap_or_else(|| "document.pdf".into())
    }

    pub fn toast(&self, message: &str) {
        if let Some(overlay) = UI.with(|ui| ui.toast_overlay.borrow().clone()) {
            overlay.add_toast(adw::Toast::new(message));
        }
    }

    pub fn set_console_section(&mut self, section: ConsoleSection) {
        self.model.console_section = section;
        self.refresh_console_visibility();
    }

    pub fn persist_commands(&mut self) {
        self.model.persist_commands(&mut self.store);
    }

    pub fn sync_build_entries(&self) {
        UI.with(|ui| {
            if let Some(e) = ui.build_command_entry.borrow().as_ref() {
                e.set_text(&self.model.build_command_text);
            }
            if let Some(e) = ui.custom_command_entry.borrow().as_ref() {
                e.set_text(&self.model.custom_command_text);
            }
        });
    }

    // ── settings mutations ─────────────────────────────────────────────────

    pub fn update_build_prefs(&mut self, f: impl FnOnce(&mut BuildPreferences)) {
        let mut next: PersistedSettings = self.store.settings.clone();
        f(&mut next.build);
        self.store.update_settings(next);
    }

    pub fn update_editor_prefs(&mut self, f: impl FnOnce(&mut EditorPreferences)) {
        let mut next: PersistedSettings = self.store.settings.clone();
        f(&mut next.editor);
        self.store.update_settings(next);
    }

    /// `shellAckBinding` — the acknowledgement only latches while a custom
    /// shell command is configured (`acknowledgingCustomShell`).
    pub fn update_shell_acknowledgement(&mut self, acknowledged: bool) {
        let mut next: PersistedSettings = self.store.settings.clone();
        next.build.custom_shell_acknowledged = acknowledged
            && matches!(
                next.build.shell_execution,
                ShellExecutionPreference::Custom { .. }
            );
        self.store.update_settings(next);
        self.refresh_shell_warning();
    }

    /// `customCommandBinding` — an empty field disables the login-shell
    /// fallback and clears the acknowledgement; a non-empty command keeps
    /// whatever acknowledgement state was already stored.
    pub fn set_shell_execution(&mut self, command: &str) {
        let mut next: PersistedSettings = self.store.settings.clone();
        if command.is_empty() {
            next.build.shell_execution = ShellExecutionPreference::Disabled;
            next.build.custom_shell_acknowledged = false;
        } else {
            next.build.shell_execution = ShellExecutionPreference::Custom {
                command: command.to_string(),
            };
        }
        self.store.update_settings(next);
        self.refresh_shell_warning();
    }

    // ── appearance ─────────────────────────────────────────────────────────

    pub fn apply_theme(&self) {
        let manager = adw::StyleManager::default();
        manager.set_color_scheme(match self.appearance.theme {
            Theme::Dark => adw::ColorScheme::ForceDark,
            Theme::Light => adw::ColorScheme::ForceLight,
            Theme::System => adw::ColorScheme::Default,
        });
        // Generate a GtkSourceView style scheme from the palette — the
        // native mechanism for editor/gutter/line-number colors.
        let prefs = self.store.prefs();
        let scheme_xml = style_scheme_xml(&self.appearance, prefs);
        if let Some(display) = gdk::Display::default() {
            let provider = gtk4::CssProvider::new();
            let bg = self.appearance.stored_hex(prefs, AppearanceColorRole::EditorBackground);
            let fg = self.appearance.stored_hex(prefs, AppearanceColorRole::BodyText);
            let family = if self.appearance.font_family.is_empty() {
                "Monospace".to_string()
            } else {
                self.appearance.font_family.clone()
            };
            let size = self.appearance.font_size.max(1.0);
            provider.load_from_data(&format!(
                ".pitex-editor, .pitex-editor text {{ background-color: {bg}; color: {fg}; caret-color: {fg}; font-family: {family}; font-size: {size}pt; }}"
            ));
            gtk4::style_context_add_provider_for_display(
                &display,
                &provider,
                gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        }
        let scheme_dir = dirs::cache_dir()
            .unwrap_or_else(std::env::temp_dir)
            .join("pitex/schemes");
        let _ = std::fs::create_dir_all(&scheme_dir);
        let scheme_path = scheme_dir.join("pitex-theme.xml");
        if std::fs::write(&scheme_path, scheme_xml).is_ok() {
            let manager = sourceview5::StyleSchemeManager::default();
            manager.append_search_path(&scheme_dir.to_string_lossy());
            manager.force_rescan();
            if let (Some(scheme), Some(editor)) =
                (manager.scheme("pitex-dynamic"), &self.editor)
            {
                editor.buffer().set_style_scheme(Some(&scheme));
            }
        }
    }

    /// GtkSourceView style schemes don't cover all eleven roles, so token
    /// colors also go through TextTags created lazily by the adapter —
    /// this (re)colors them after the palette changes.
    fn apply_tag_colors(&self) {
        let Some(editor) = &self.editor else { return };
        let table = editor.buffer().tag_table();
        let prefs = self.store.prefs();
        for kind in token_kinds() {
            let name = editor_feature::decoration_tag_name(&kind);
            if let Some(tag) = table.lookup(&name) {
                let hex = self
                    .appearance
                    .stored_hex(prefs, color_role_for(&kind));
                if let Some((r, g, b, _a)) = crate::settings::parse_hex_color(&hex) {
                    tag.set_foreground_rgba(Some(&gdk::RGBA::new(r as f32, g as f32, b as f32, 1.0)));
                }
            }
        }
    }

    pub fn apply_editor_preferences(&self) {
        let Some(editor) = &self.editor else { return };
        let settings = &self.store.settings.editor;
        editor.view().set_tab_width(settings.tab_width as u32);
        editor.view().set_wrap_mode(if settings.wraps_lines {
            gtk4::WrapMode::Word
        } else {
            gtk4::WrapMode::None
        });
        editor.view().set_insert_spaces_instead_of_tabs(true);
        editor.view().set_auto_indent(true);
        editor.view().set_highlight_current_line(true);
        editor.view().set_show_line_numbers(true);
        editor.buffer().set_highlight_matching_brackets(true);
        if let Some(fold) = &self.fold {
            fold.set_enabled(self.store.code_folding());
        }
        let font = self.terminal_font_desc();
        UI.with(|ui| {
            if let Some(map) = ui.minimap.borrow().as_ref() {
                map.set_visible(self.store.minimap());
            }
            if let Some(term) = ui.terminal.borrow().as_ref() {
                term.set_font(Some(&font));
            }
        });
    }

    /// `rehighlight()` — re-tokenize and push decorations + structure.
    pub fn rehighlight(&self) {
        let Some(editor) = &self.editor else { return };
        let Some(snapshot) = &self.model.document_snapshot else { return };
        let dialect = dialect_for(self.model.active_document_url.as_deref());
        let tokens = DeterministicTeXLexer::tokenize(&snapshot.text, dialect);
        let decorations = tokens
            .into_iter()
            .map(|t| editor_feature::EditorDecoration {
                range: t.range,
                token_kind: t.kind,
            })
            .collect();
        editor.apply_decorations(&editor_feature::EditorDecorationSnapshot::new(
            snapshot.revision,
            decorations,
        ));
        self.apply_tag_colors();
    }

    /// Debounced variant — Swift's `scheduleHighlight` (~120ms), delivered
    /// through `schedule_rehighlight_static`/`schedule_autosave` so the
    /// timer callbacks re-borrow `AppState` lazily.

    // ── document/session wiring ────────────────────────────────────────────

    /// Build a `SessionClient` bridging the adapter to the canonical
    /// `DocumentSession`, mirroring `EditorMacAdapter.make(session:)`.
    /// `on_applied` is invoked on the main context after the session accepts
    /// a mutation so the model can resynchronize.
    fn session_client(
        session: document_session_core::DocumentSession,
        on_applied: impl Fn() + 'static,
    ) -> Rc<dyn SessionClient> {
        let snap_session = session.clone();
        let submit_session = session.clone();
        Rc::new(FnSessionClient::new(
            move || {
                let s = snap_session.snapshot();
                app_ports::DocumentSnapshot {
                    revision: s.revision,
                    text: s.text,
                }
            },
            move |mutation| {
                let result = submit_session.apply(
                    DocumentMutation::ReplaceText(mutation.replacement.clone()),
                    mutation.base_revision,
                );
                match result {
                    Ok(snapshot) => {
                        on_applied();
                        DocumentMutationResult::Applied(app_ports::DocumentSnapshot {
                            revision: snapshot.revision,
                            text: snapshot.text,
                        })
                    }
                    Err(document_session_core::DocumentSessionError::StaleRevision { .. }) => {
                        let s = submit_session.snapshot();
                        DocumentMutationResult::Rejected {
                            current: app_ports::DocumentSnapshot {
                                revision: s.revision,
                                text: s.text,
                            },
                        }
                    }
                    Err(_) => {
                        let s = submit_session.snapshot();
                        DocumentMutationResult::Rejected {
                            current: app_ports::DocumentSnapshot {
                                revision: s.revision,
                                text: s.text,
                            },
                        }
                    }
                }
            },
        ))
    }

    /// Pull the adapter-committed snapshot into the model + refresh derived
    /// UI (structure, decorations, footer). Called via the session client.
    pub fn sync_snapshot_from_session(&mut self) {
        if let Some(session) = self.active_session.clone() {
            self.model.document_snapshot = Some(session.snapshot());
        }
        self.model.refresh_structure();
        self.refresh_after_document_change();
        self.model.sync_selection_attachment();
        self.schedule_autosave();
        Self::schedule_rehighlight_static();
    }

    /// `scheduleAutosave` — every edit cancels the pending timer and re-arms
    /// a fresh one, so the save lands `autoSaveDelay` seconds after the last
    /// change. The timer is armed only while autosave is enabled and the
    /// document is dirty with read-write access; `save()` re-checks `canSave`
    /// at fire time exactly like the Swift task.
    fn schedule_autosave(&self) {
        // `autosaveTask?.cancel()` — the pending source is destroyed so a new
        // edit always restarts the full delay.
        AUTOSAVE_SOURCE.with(|s| {
            if let Some(id) = s.borrow_mut().take() {
                id.remove();
            }
        });
        // guard settings.autoSave && saveState == .dirty && lease == .readWrite
        if !(self.store.auto_save() && self.model.can_save()) {
            return;
        }
        let delay = self.store.auto_save_delay().max(1) as u64;
        let id = glib::timeout_add_local_once(Duration::from_secs(delay), move || {
            AUTOSAVE_SOURCE.with(|s| s.borrow_mut().take());
            STATE.with(|s| {
                if let Some(state) = s.borrow().as_ref() {
                    let Ok(mut st) = state.try_borrow_mut() else { return };
                    st.model.save();
                    st.refresh_after_document_change();
                }
            });
        });
        AUTOSAVE_SOURCE.with(|s| *s.borrow_mut() = Some(id));
    }

    /// Attach a session to the (single) editor adapter — mirrors the macOS
    /// environment rebuilding `adapter` when the active document changes.
    pub fn attach_session(&mut self, session: DocumentSession) {
        self.active_session = Some(session.clone());
        let client = Self::session_client(session, || {
            // Defer: the submit callback runs inside a GTK signal; doing the
            // model update through idle avoids RefCell re-entrancy.
            glib::idle_add_local_once(|| {
                STATE.with(|s| {
                    if let Some(state) = s.borrow().as_ref() {
                        state.borrow_mut().sync_snapshot_from_session();
                    }
                });
            });
        });
        let adapter = Rc::new(GtkEditorAdapter::make(client));
        adapter.view().add_css_class("pitex-editor");
        a11y(adapter.view(), "pitex.editor.text", "editor.title");
        // FoldEngine attaches before the widget rebind so its chip layer
        // lands on the editor overlay, mirroring FoldEngine.attach.
        let dialect = dialect_for(self.model.active_document_url.as_deref());
        let fold = crate::fold::FoldEngine::attach(
            adapter.view(),
            dialect,
            self.store.code_folding(),
        );
        self.editor = Some(adapter);
        self.fold = Some(fold);
        self.rebind_editor_widget();
        self.apply_editor_preferences();
        self.apply_theme();
        self.rehighlight();
        self.install_editor_controllers();
    }

    /// Swap the editor child inside the scroller and (re)wire the minimap.
    /// The view stays the scroller's direct child — a non-Scrollable child
    /// would disconnect its adjustments and kill minimap/jump-to — while
    /// the fold chip layer overlays the scroller from outside.
    fn rebind_editor_widget(&self) {
        let Some(editor) = &self.editor else { return };
        UI.with(|ui| {
            if let Some(scroller) = ui.editor_scroller.borrow().as_ref() {
                scroller.set_child(Some(editor.view()));
            }
            if let Some(overlay) = ui.editor_overlay.borrow().as_ref() {
                if let Some(old) = self.fold_chip.borrow_mut().take() {
                    overlay.remove_overlay(&old);
                }
                if let Some(fold) = &self.fold {
                    overlay.add_overlay(fold.chip_area());
                    *self.fold_chip.borrow_mut() = Some(fold.chip_area().clone());
                }
            }
            if let Some(map) = ui.minimap.borrow().as_ref() {
                map.set_view(editor.view());
            }
        });
    }

    /// Ctrl+click → forward SyncTeX; mark-set → selection attachment.
    fn install_editor_controllers(&self) {
        let Some(editor) = &self.editor else { return };
        let click = gtk4::GestureClick::new();
        click.set_button(1);
        let view = editor.view().clone();
        click.connect_pressed(move |gesture, _, x, y| {
            let state = gesture.current_event_state();
            if !state.contains(gdk::ModifierType::CONTROL_MASK) {
                return;
            }
            let (bx, by) = view.window_to_buffer_coords(
                gtk4::TextWindowType::Widget,
                x as i32,
                y as i32,
            );
            let iter = view.iter_at_location(bx, by).or_else(|| {
                let (mut iter, _top) = view.line_at_y(by);
                if !iter.ends_line() {
                    iter.forward_to_line_end();
                }
                Some(iter)
            });
            if let Some(iter) = iter {
                let line = iter.line() as usize + 1;
                let mut start = iter;
                start.set_line_offset(0);
                let col_text = view.buffer().text(&start, &iter, false);
                let column = crate::model::utf16_len(col_text.as_str());
                STATE.with(|s| {
                    if let Some(state) = s.borrow().as_ref() {
                        if let Ok(mut s) = state.try_borrow_mut() {
                            s.sync_forward_at(line, column);
                        }
                    }
                });
            }
        });
        editor.view().add_controller(click);

        // Track selection → editor_selection + agent attachment chip.
        let buffer = editor.buffer().clone();
        buffer.connect_mark_set(|buffer, _iter, mark| {
            if mark != &buffer.get_insert() && mark != &buffer.selection_bound() {
                return;
            }
            glib::idle_add_local_once(|| {
                STATE.with(|s| {
                    if let Some(state) = s.borrow().as_ref() {
                        let Ok(mut st) = state.try_borrow_mut() else { return };
                        if let Some(range) =
                            st.editor.as_ref().map(|e| e.selected_range())
                        {
                            st.model.editor_selection = (
                                range.location.max(0) as usize,
                                range.length.max(0) as usize,
                            );
                        }
                        st.model.sync_selection_attachment();
                        st.drain_side_effects();
                        st.refresh_footer();
                    }
                });
            });
        });
    }

    /// After any text-affecting change: footer, save button, structure lists.
    pub fn refresh_after_document_change(&mut self) {
        self.refresh_footer();
        self.refresh_tabs();
        self.refresh_sidebar();
        self.refresh_conflict_banner();
        self.refresh_shell_warning();
        self.refresh_save_sensitivity();
        UI.with(|ui| {
            if let Some(stack) = ui.editor_stack.borrow().as_ref() {
                stack.set_visible_child_name(if self.model.document_snapshot.is_some() {
                    "editor"
                } else {
                    "empty"
                });
            }
        });
        // `restoreBuiltPreview` may have swapped the bound PDF alongside the
        // document change (main document vs. chapter/bibliography target).
        self.refresh_pdf_ui();
    }

    /// `STATE`-free rehighlight scheduling used by the submit path.
    fn schedule_rehighlight_static() {
        glib::timeout_add_local_once(Duration::from_millis(120), || {
            STATE.with(|s| {
                if let Some(state) = s.borrow().as_ref() {
                    state.borrow().rehighlight();
                }
            });
        });
    }

    // ── command actions ────────────────────────────────────────────────────

    /// `close()`'s agent half — terminate the coordinator and its subprocess
    /// with the project; a fresh one is built on the next open.
    fn shutdown_agent(&mut self) {
        if let Some(agent) = self.agent.as_mut() {
            agent.shutdown();
        }
        self.agent = None;
    }

    pub fn open_selected(&mut self, path: PathBuf) {
        // Swift `open()` runs `await close()` first — including
        // `agent?.shutdown()` — before loading the new project.
        self.shutdown_agent();
        if let Some(tx) = &self.tx {
            self.model.open(path, tx.clone());
        }
        self.refresh_phase();
    }

    /// File → Open Recent — `open(url)` for a recents entry (menu target is
    /// the index into `recent_documents`).
    pub fn open_recent_action(&mut self, index: i32) {
        let Some(url) = self
            .model
            .recent_documents
            .get(index.max(0) as usize)
            .cloned()
        else {
            return;
        };
        self.open_selected(url);
    }

    /// File → Open Recent → Clear — `clearRecents()`.
    pub fn clear_recents_action(&mut self) {
        self.model.clear_recents(&mut self.store);
        self.refresh_phase();
    }

    /// `anchor` is the button that triggered the chooser — the popover
    /// points at it. `win.open` (shortcut/menu) passes `None` and falls
    /// back to the header open button, then the window.
    pub fn present_open(&self, anchor: Option<&gtk4::Widget>) {
        let anchor = anchor.cloned().or_else(|| {
            UI.with(|ui| {
                ui.open_button
                    .borrow()
                    .as_ref()
                    .filter(|b| b.is_mapped())
                    .map(|b| b.clone().upcast::<gtk4::Widget>())
                    .or_else(|| {
                        ui.window
                            .borrow()
                            .as_ref()
                            .map(|w| w.clone().upcast::<gtk4::Widget>())
                    })
            })
        });
        let Some(anchor) = anchor else { return };
        let menu = gtk4::Popover::new();
        let vbox = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        let file_btn = gtk4::Button::with_label(&tr(self.language, "workspace.open"));
        let folder_btn = gtk4::Button::with_label(&tr(self.language, "workspace.open_project"));
        file_btn.set_has_frame(false);
        folder_btn.set_has_frame(false);
        let menu_state = menu.clone();
        file_btn.connect_clicked(move |b| {
            menu_state.popdown();
            let window = b.root().and_then(|r| r.downcast::<gtk4::Window>().ok());
            compat::pick_source_file(window.as_ref(), "Open", |path| {
                STATE.with(|s| {
                    if let Some(state) = s.borrow().as_ref() {
                        if let Ok(mut s) = state.try_borrow_mut() { s.open_selected(path); }
                    }
                });
            });
        });
        let menu_state = menu.clone();
        folder_btn.connect_clicked(move |b| {
            menu_state.popdown();
            let window = b.root().and_then(|r| r.downcast::<gtk4::Window>().ok());
            compat::pick_folder(window.as_ref(), "Open Project Folder", |path| {
                STATE.with(|s| {
                    if let Some(state) = s.borrow().as_ref() {
                        if let Ok(mut s) = state.try_borrow_mut() { s.open_selected(path); }
                    }
                });
            });
        });
        vbox.append(&file_btn);
        vbox.append(&folder_btn);
        menu.set_child(Some(&vbox));
        menu.set_parent(&anchor);
        menu.popup();
    }

    pub fn save_action(&mut self) {
        self.model.save();
        self.refresh_after_document_change();
    }

    pub fn save_all_action(&mut self) {
        if let Some(err) = self.model.persist_dirty_sessions() {
            self.toast(&err);
        }
        self.refresh_after_document_change();
    }

    pub fn save_as_action(&self) {
        let window = UI.with(|ui| ui.window.borrow().clone());
        let Some(window) = window else { return };
        let name = self
            .model
            .active_document_url
            .as_ref()
            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()));
        compat::save_file(Some(window.upcast_ref()), "Save As", name.as_deref(), |path| {
            STATE.with(|s| {
                if let Some(state) = s.borrow().as_ref() {
                    let Ok(mut st) = state.try_borrow_mut() else { return };
                    if let Some(tx) = st.tx.clone() {
                        st.model.save_as(path, tx);
                    }
                    st.refresh_after_document_change();
                }
            });
        });
    }

    /// File → New Document: creates `untitled.tex` at the project root and
    /// activates it — matches `createDocument()` (no save panel).
    pub fn create_document_action(&mut self) {
        if let Some(tx) = self.tx.clone() {
            self.model.create_document(tx);
        }
        self.refresh_after_document_change();
    }

    pub fn activate_document(&mut self, url: PathBuf) {
        if let Some(tx) = self.tx.clone() {
            self.model.activate_document(url, tx);
        }
    }

    pub fn close_document(&mut self, url: PathBuf) {
        self.model.close_document(&url);
        self.refresh_after_document_change();
    }

    pub fn reload_active(&mut self) {
        let confirm = self.store.confirm_overwrite();
        self.model.reload_active_document_from_disk(confirm);
        if let Some(editor) = &self.editor {
            editor.refresh_from_session();
        }
        self.refresh_after_document_change();
    }

    pub fn resolve_conflict(&mut self, use_disk: bool) {
        self.model.resolve_conflict(use_disk);
        if let Some(editor) = &self.editor {
            editor.refresh_from_session();
        }
        self.refresh_after_document_change();
    }

    /// `jumpTo(line:column:highlight:)` — reveal the position in the editor
    /// and update the tracked selection (UTF-16 units, like `NSTextView`).
    /// The find indicator only flashes when `highlight` is set.
    pub fn jump_to(&mut self, line: usize, column: usize, highlight: bool) {
        if let Some(editor) = &self.editor {
            let text = editor.text();
            let utf16 = line_col_to_utf16(&text, line.saturating_sub(1), column);
            editor.reveal_selection(
                editor_feature::EditorTextRange {
                    location: utf16 as i64,
                    length: 0,
                },
                highlight,
            );
            self.model.editor_selection = (utf16, 0);
        }
    }

    pub fn jump_to_issue(&mut self, index: usize) {
        let issue = self.filtered_issues().get(index).cloned();
        let Some(issue) = issue else { return };
        let (Some(file), Some(line)) = (issue.file.clone(), issue.line) else {
            return;
        };
        let path = self
            .model
            .project_url
            .as_ref()
            .map(|root| root.join(&file))
            .unwrap_or_else(|| PathBuf::from(&file));
        self.activate_document(path);
        let column = issue.column.unwrap_or(0) as usize;
        self.jump_to(line.max(1) as usize, column, false);
    }

    pub fn filtered_issues(&self) -> Vec<build_core::BuildIssueRecord> {
        let mode = UI.with(|ui| {
            ui.issue_filter
                .borrow()
                .as_ref()
                .map(|d| d.selected())
                .unwrap_or(0)
        });
        self.model
            .build_issues
            .iter()
            .filter(|i| match mode {
                1 => i.severity == build_core::BuildIssueSeverity::Error,
                2 => i.severity == build_core::BuildIssueSeverity::Warning,
                _ => true,
            })
            .cloned()
            .collect()
    }

    // ── build / terminal actions ───────────────────────────────────────────

    pub fn start_build_action(&mut self) {
        self.model.start_build(&self.store);
        self.refresh_build_ui();
    }

    pub fn cancel_build_action(&mut self) {
        self.model.cancel_build();
        self.refresh_build_ui();
    }

    pub fn toggle_build_action(&mut self) {
        if self.model.is_building() {
            self.cancel_build_action();
        } else {
            self.start_build_action();
        }
    }

    pub fn run_custom_command_action(&mut self) {
        let acknowledged = self.store.settings.build.custom_shell_acknowledged;
        self.model.run_custom_command(acknowledged);
        self.model.console_section = ConsoleSection::Terminal;
        self.refresh_console_visibility();
    }

    pub fn clean_artifacts_action(&mut self) {
        self.model.clean_build_artifacts();
        self.toast(&tr(self.language, "command.clean"));
    }

    /// Edit → Toggle Line Comment — `toggleLineComment()`: the model computes
    /// the transformed line range (UTF-16), the buffer applies it as a user
    /// action so undo groups it and the adapter's change hook submits it.
    pub fn toggle_comment_action(&mut self) {
        let Some(editor) = self.editor.clone() else { return };
        let Some((range, transformed)) = self.model.toggle_line_comment_transform()
        else {
            return;
        };
        let text = editor.text();
        let start_char =
            gtk_editor_adapter::utf16_offset_to_char_offset(&text, range.start);
        let end_char = gtk_editor_adapter::utf16_offset_to_char_offset(&text, range.end);
        let buffer = editor.buffer().clone();
        buffer.begin_user_action();
        let mut start = buffer.iter_at_offset(start_char as i32);
        let mut end = buffer.iter_at_offset(end_char as i32);
        buffer.delete(&mut start, &mut end);
        let mut at = buffer.iter_at_offset(start_char as i32);
        buffer.insert(&mut at, &transformed);
        buffer.end_user_action();
        self.refresh_after_document_change();
    }

    /// Edit → Send Selection to AI — `sendSelectionToAssistant()`: reveal the
    /// assistant and insert the fenced selection into the composer.
    pub fn send_selection_action(&mut self) {
        let Some(payload) = self.model.selection_for_assistant() else { return };
        self.model.reveal_assistant();
        self.refresh_console_visibility();
        self.refresh_assistant();
        if let Some(agent) = &mut self.agent {
            agent.insert_into_composer(&payload);
        }
        self.apply_pending_composer();
    }

    /// File → Close — drop the project (sessions, watchers, lease) and return
    /// to the status page.
    pub fn close_action(&mut self) {
        self.watchers.clear();
        self.editor = None;
        self.fold = None;
        // `agent?.shutdown(); agent = nil` — terminate the agent process with
        // the project; a reopen builds a fresh coordinator and subprocess.
        self.shutdown_agent();
        self.model.close();
        self.refresh_phase();
    }

    /// File → Pin Build Target — `togglePinnedBuildTarget()`.
    pub fn pin_target_action(&mut self) {
        self.model.toggle_pinned_build_target();
        self.refresh_sidebar();
        self.refresh_pdf_ui();
    }

    /// View toggles — the paned visibility bindings the macOS commands flip.
    pub fn toggle_sidebar_action(&mut self) {
        self.model.sidebar_visible = !self.model.sidebar_visible;
        self.refresh_console_visibility();
    }
    pub fn toggle_inspector_action(&mut self) {
        self.model.inspector_visible = !self.model.inspector_visible;
        self.refresh_console_visibility();
    }
    pub fn toggle_bottom_panel_action(&mut self) {
        self.model.bottom_panel_visible = !self.model.bottom_panel_visible;
        self.refresh_console_visibility();
    }
    /// `toggleAssistant()` — reveal/hide the assistant console section.
    pub fn toggle_assistant_action(&mut self) {
        self.model.toggle_assistant();
        self.refresh_console_visibility();
        self.refresh_assistant();
    }

    /// `on_terminal_feed` — ANSI status text into the terminal pane.
    pub fn terminal_feed(&self, text: &str) {
        UI.with(|ui| {
            if let Some(term) = ui.terminal.borrow().as_ref() {
                term.feed(text);
            }
        });
    }

    /// `on_terminal_send` — a full command line to the shell's stdin (or an
    /// external terminal on the Ubuntu 22.04 build, which has no VTE).
    pub fn terminal_send(&self, command: &str) {
        UI.with(|ui| {
            if let Some(term) = ui.terminal.borrow().as_ref() {
                term.send(command, self.model.project_url.as_deref());
            }
        });
    }

    // ── SyncTeX / PDF actions ─────────────────────────────────────────────

    pub fn sync_forward_action(&mut self) {
        if let Some(editor) = &self.editor {
            let range = editor.selected_range();
            let (line, col) = utf16_to_line_col(&editor.text(), range.location.max(0) as usize);
            self.model.sync_forward_at(line + 1, col);
        } else {
            self.model.sync_forward();
        }
    }

    pub fn sync_forward_at(&mut self, line: usize, column: usize) {
        self.model.sync_forward_at(line, column);
    }

    pub fn pdf_prev_page(&mut self) {
        if self.pdf_page > 0 {
            self.pdf_page -= 1;
            self.render_pdf_page();
        }
    }

    pub fn pdf_next_page(&mut self) {
        if let Some(doc) = &self.pdf {
            if self.pdf_page + 1 < doc.page_count() {
                self.pdf_page += 1;
                self.render_pdf_page();
            }
        }
    }

    pub fn pdf_zoom(&mut self, factor: f64) {
        self.pdf_auto_fit = false;
        self.pdf_scale = (self.pdf_scale * factor).clamp(0.4, 6.0);
        self.render_pdf_page();
    }

    /// Scale that fits the page width inside the preview scroller.
    fn pdf_fit_scale(&self) -> f64 {
        let Some(doc) = &self.pdf else { return self.pdf_scale };
        let Some((w_pt, _)) = doc.page_size(self.pdf_page) else { return self.pdf_scale };
        let viewport = UI.with(|ui| {
            ui.pdf_scroll.borrow().as_ref().map(|s| s.width()).unwrap_or(0)
        });
        if viewport > 0 {
            ((viewport as f64 - 16.0) / w_pt).clamp(0.2, 6.0)
        } else {
            self.pdf_scale
        }
    }

    /// Called when the preview scroller changes size — refits while
    /// `pdf_auto_fit` is on, like PDFView `autoScales`.
    pub fn pdf_viewport_resized(&mut self) {
        if !self.pdf_auto_fit || self.pdf.is_none() {
            return;
        }
        let fit = self.pdf_fit_scale();
        if (fit - self.pdf_scale).abs() > 0.01 {
            self.pdf_scale = fit;
            self.render_pdf_page();
        }
    }

    /// Click on the PDF → inverse SyncTeX (widget px → PDF points).
    pub fn pdf_click(&mut self, picture: &gtk4::Picture, x: f64, y: f64) {
        let Some(doc) = &self.pdf else { return };
        let Some((w_pt, h_pt)) = doc.page_size(self.pdf_page) else { return };
        // The Picture uses ContentFit::Contain inside the scroller; compute
        // the displayed rect from the texture's natural aspect.
        let alloc_w = picture.width() as f64;
        let alloc_h = picture.height() as f64;
        if alloc_w <= 0.0 || alloc_h <= 0.0 {
            return;
        }
        let scale = (alloc_w / w_pt).min(alloc_h / h_pt);
        let disp_w = w_pt * scale;
        let disp_h = h_pt * scale;
        let ox = (alloc_w - disp_w) / 2.0;
        let oy = (alloc_h - disp_h) / 2.0;
        let px = (x - ox) / scale;
        let py = (y - oy) / scale;
        if px < 0.0 || py < 0.0 || px > w_pt || py > h_pt {
            return;
        }
        // `synctex edit` wants top-left origin points — widget coords are
        // already top-left, so pass them through unflipped.
        if let Ok(point) = synctex_core::PDFPoint::new(px, py) {
            self.model.sync_inverse(self.pdf_page as i64 + 1, point);
        }
    }

    /// Re-render the current page into the picture widget.
    fn render_pdf_page(&self) {
        let Some(doc) = &self.pdf else { return };
        let scale = if self.pdf_auto_fit { self.pdf_fit_scale() } else { self.pdf_scale };
        UI.with(|ui| {
            if let Some(picture) = ui.pdf_picture.borrow().as_ref() {
                if let Some((pixels, w, h, stride)) = doc.render_page(self.pdf_page, scale) {
                    let texture = crate::pdf::texture_from_pixels(pixels, w, h, stride);
                    picture.set_paintable(Some(&texture));
                    picture.set_size_request(w, h);
                }
            }
            if let Some(label) = ui.pdf_page_label.borrow().as_ref() {
                label.set_text(&format!(
                    "{} / {}",
                    self.pdf_page + 1,
                    doc.page_count()
                ));
            }
            if let Some(b) = ui.pdf_prev_button.borrow().as_ref() {
                b.set_sensitive(self.pdf_page > 0);
            }
            if let Some(b) = ui.pdf_next_button.borrow().as_ref() {
                b.set_sensitive(self.pdf_page + 1 < doc.page_count());
            }
        });
    }

    /// `clearSyncHighlight()` — hide the marker and bump the generation so a
    /// pending auto-hide for a previous marker cannot fire later.
    pub fn clear_synctex_highlight(&mut self) {
        self.highlight_generation.set(self.highlight_generation.get() + 1);
        UI.with(|ui| {
            if let Some(hl) = ui.pdf_highlight.borrow().as_ref() {
                hl.set_visible(false);
            }
        });
    }

    /// `apply_forward_result` UI half — show the highlight box on the page.
    /// Forward-sync destination: the page scroll always runs — the yellow
    /// marker is the optional part gated by `forwardSyncHighlight`. The
    /// existing marker clears first like `clearSyncHighlight()` at the top
    /// of the Swift handler, so a newer navigation supersedes the old one.
    fn show_synctex_highlight(&mut self, page: i64, x: f64, y: f64, w: f64, h: f64) {
        let target = (page.max(1) - 1) as usize;
        if self.pdf_page != target {
            self.pdf_page = target;
            self.render_pdf_page();
        }
        self.clear_synctex_highlight();
        if !self.store.forward_sync_highlight() {
            return;
        }
        let generation = self.highlight_generation.get();
        UI.with(|ui| {
            let (Some(picture), Some(hl), Some(doc)) = (
                ui.pdf_picture.borrow().as_ref().cloned(),
                ui.pdf_highlight.borrow().as_ref().cloned(),
                self.pdf.as_ref(),
            ) else {
                return;
            };
            let Some((w_pt, h_pt)) = doc.page_size(self.pdf_page) else {
                return;
            };
            let alloc_w = picture.width() as f64;
            let alloc_h = picture.height() as f64;
            if alloc_w <= 0.0 || alloc_h <= 0.0 {
                return;
            }
            let scale = (alloc_w / w_pt).min(alloc_h / h_pt);
            let ox = (alloc_w - w_pt * scale) / 2.0;
            let oy = (alloc_h - h_pt * scale) / 2.0;
            // SyncTeX h/v are in PostScript points from the top-left? The
            // CLI reports h/v in TeX points from the page's top-left with y
            // growing downward — convert like PDFDocumentView does.
            hl.set_size_request((w * scale).max(2.0) as i32, (h * scale).max(2.0) as i32);
            hl.set_margin_start((x * scale + ox).max(0.0) as i32);
            hl.set_margin_top((y * scale + oy).max(0.0) as i32);
            hl.set_halign(gtk4::Align::Start);
            hl.set_valign(gtk4::Align::Start);
            hl.set_visible(true);
            let generation_cell = self.highlight_generation.clone();
            glib::timeout_add_local_once(Duration::from_millis(1500), move || {
                // A newer navigation cancels the earlier removal, like the
                // coordinator's highlightTask cancellation.
                if generation_cell.get() == generation {
                    hl.set_visible(false);
                }
            });
        });
    }

    // ── agent actions ─────────────────────────────────────────────────────

    pub fn ensure_agent(&mut self) {
        // Refresh cached context before any agent call — the coordinator's
        // provider reads this cell, so it must be current before `prepare()`.
        *self.context_cell.borrow_mut() = self.model.agent_context();
        self.model.sync_selection_attachment();
        if self.agent.is_none() {
            let mut coordinator = AgentCoordinator::new(&self.store);
            let cell = self.context_cell.clone();
            coordinator.context_provider = Some(Box::new(move || cell.borrow().clone()));
            let persist = self.persist_result.clone();
            coordinator.persist_dirty_sessions = Some(Box::new(move || {
                persist.borrow_mut().take().flatten()
            }));
            let flag = self.agent_activity_pending.clone();
            coordinator.on_agent_activity_finished = Some(Box::new(move || flag.set(true)));
            // PitexApp.swift:407-409 — the workspace injects the AI settings
            // into the new coordinator: attach-default wins over the persisted
            // session key (the didSet mirrors it back), the default model seeds
            // the picker, and the history cap bounds the transcript.
            let attach_default = self.store.ai_attach_default();
            coordinator.set_attach_active_document(&mut self.store, attach_default);
            let default_model = self.store.ai_default_model();
            coordinator.preferred_model_id = if default_model.is_empty() {
                None
            } else {
                Some(default_model)
            };
            coordinator.history_limit = self.store.chat_history_limit().max(0) as usize;
            self.agent = Some(coordinator);
            // `coordinator.prepare()` (PitexApp.swift:413) — spawn the agent
            // process eagerly when the coordinator is created, like `open()`
            // does; idempotent and a no-op while a project root is absent.
            if let Some(agent) = self.agent.as_mut() {
                agent.prepare();
            }
        }
    }

    pub fn send_agent_draft(&mut self) {
        let draft = UI.with(|ui| {
            ui.agent_composer
                .borrow()
                .as_ref()
                .map(|e| e.text().to_string())
                .unwrap_or_default()
        });
        if draft.trim().is_empty() {
            return;
        }
        self.ensure_agent();
        *self.context_cell.borrow_mut() = self.model.agent_context();
        *self.persist_result.borrow_mut() = Some(self.model.persist_dirty_sessions());
        if let Some(agent) = self.agent.as_mut() {
            agent.send(&draft);
        }
        UI.with(|ui| {
            if let Some(e) = ui.agent_composer.borrow().as_ref() {
                e.set_text("");
            }
        });
        self.refresh_assistant();
    }

    /// Apply queued composer insertion text (attach-files flow).
    pub fn apply_pending_composer(&mut self) {
        let insertion = self
            .agent
            .as_mut()
            .and_then(|a| a.pending_composer_insertion.take());
        if let Some(text) = insertion {
            UI.with(|ui| {
                if let Some(e) = ui.agent_composer.borrow().as_ref() {
                    let mut pos = e.position();
                    e.insert_text(&text, &mut pos);
                }
            });
        }
    }

    /// `attachFiles()` — convert absolute paths to project-relative where
    /// possible, then insert them into the composer.
    pub fn attach_files_action(&mut self, paths: Vec<String>) {
        self.ensure_agent();
        let root = self.model.project_url.clone();
        let converted: Vec<String> = paths
            .iter()
            .map(|p| {
                let url = PathBuf::from(p);
                root.as_ref()
                    .and_then(|r| WorkspaceModel::relative_path(&url, r).ok())
                    .map(|rel| rel.raw_value().to_string())
                    .unwrap_or_else(|| p.clone())
            })
            .collect();
        if let Some(agent) = self.agent.as_mut() {
            agent.insert_into_composer(&format!("{} ", converted.join(" ")));
        }
        self.apply_pending_composer();
    }

    /// `openAuthenticationInTerminal` — toolchain discovery + script
    /// generation block on shell probes, so they run off the UI thread and
    /// the terminal hand-off hops back through idle. `terminal_send` picks
    /// the embedded PTY or an external emulator per build variant.
    pub fn run_agent_auth_script(&mut self, logout: bool) {
        std::thread::spawn(move || {
            let result = crate::agent::pi_installer::authentication_script(logout);
            glib::idle_add_once(move || {
                STATE.with(|s| {
                    if let Some(state) = s.borrow().as_ref() {
                        if let Ok(mut st) = state.try_borrow_mut() {
                            match result {
                                Ok(script) => {
                                    // Shell-quote the path — `{:?}` debug
                                    // format is NOT quoting: `$`, backticks
                                    // and `\` stay live inside bash quotes.
                                    #[cfg(unix)]
                                    let command = format!(
                                        "bash {} && exit",
                                        crate::agent::pi_installer::shell_quote(
                                            &script.display().to_string()
                                        )
                                    );
                                    // `cmd /k` (spawn_external_terminal) runs
                                    // the .bat directly — quoting the path
                                    // is the whole command line.
                                    #[cfg(windows)]
                                    let command = format!("\"{}\"", script.display());
                                    st.model.console_section = ConsoleSection::Terminal;
                                    st.model.bottom_panel_visible = true;
                                    st.refresh_console_visibility();
                                    st.terminal_send(&command);
                                }
                                Err(e) => st.toast(&e),
                            }
                        }
                    }
                });
            });
        });
    }

    // ── refresh: phase & layout ────────────────────────────────────────────

    pub fn refresh_phase(&mut self) {
        self.refresh_save_sensitivity();
        UI.with(|ui| {
            if let Some(stack) = ui.root_stack.borrow().as_ref() {
                let (name, title, desc, icon) = match &self.model.phase {
                    WorkspacePhase::NoProject => (
                        "empty",
                        tr(self.language, "workspace.no_project"),
                        tr(self.language, "workspace.no_project_detail"),
                        "folder-open-symbolic",
                    ),
                    WorkspacePhase::Loading(_) => (
                        "loading",
                        tr(self.language, "state.loading"),
                        String::new(),
                        "folder-open-symbolic",
                    ),
                    WorkspacePhase::Ready => ("ready", String::new(), String::new(), ""),
                    WorkspacePhase::Failed(msg) => (
                        "error",
                        tr(self.language, "error.title"),
                        msg.clone(),
                        "dialog-error-symbolic",
                    ),
                };
                stack.set_visible_child_name(name);
                if let Some(page) = stack
                    .visible_child()
                    .and_then(|w| w.downcast::<adw::StatusPage>().ok())
                {
                    if !title.is_empty() {
                        page.set_title(&title);
                    }
                    if name != "loading" {
                        page.set_description(if desc.is_empty() {
                            None
                        } else {
                            Some(desc.as_str())
                        });
                    }
                    if !icon.is_empty() {
                        page.set_icon_name(Some(icon));
                    }
                }
            }
            if let Some(window) = ui.window.borrow().as_ref() {
                let title = self
                    .model
                    .project_url
                    .as_ref()
                    .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
                    .unwrap_or_else(|| tr(self.language, "app.name"));
                window.set_title(Some(&title));
            }
            // Rebuild the "+" menu — the Open Recent section follows the
            // model's `recent_documents` like the macOS File menu submenu.
            if let Some(add) = ui.add_menu_button.borrow().as_ref() {
                let menu = gio::Menu::new();
                menu.append(Some(&tr(self.language, "command.new")), Some("win.newdoc"));
                menu.append(Some(&tr(self.language, "command.open")), Some("win.open"));
                if !self.model.recent_documents.is_empty() {
                    let recents = gio::Menu::new();
                    for (i, url) in self.model.recent_documents.iter().enumerate() {
                        let label = url
                            .file_name()
                            .map(|n| n.to_string_lossy().into_owned())
                            .unwrap_or_else(|| url.display().to_string());
                        recents.append(
                            Some(&label),
                            Some(&format!("win.openrecent({i})")),
                        );
                    }
                    recents.append(
                        Some(&tr(self.language, "command.clear_recents")),
                        Some("win.clearrecents"),
                    );
                    menu.append_submenu(
                        Some(&tr(self.language, "command.open_recent")),
                        &recents,
                    );
                }
                let tail = gio::Menu::new();
                tail.append(
                    Some(&tr(self.language, "command.pin_build_target")),
                    Some("win.pintarget"),
                );
                tail.append(Some(&tr(self.language, "command.close")), Some("win.close"));
                // `command.clear_session` — `await workspace.close()` again.
                tail.append(
                    Some(&tr(self.language, "command.clear_session")),
                    Some("win.close"),
                );
                menu.append_section(None, &tail);
                add.set_menu_model(Some(&menu));
            }
        });
        self.refresh_console_visibility();
        self.refresh_after_document_change();
        self.refresh_build_ui();
        self.refresh_pdf_ui();
    }

    fn refresh_console_visibility(&self) {
        UI.with(|ui| {
            if let Some(stack) = ui.console_stack.borrow().as_ref() {
                let name = match self.model.console_section {
                    ConsoleSection::Assistant => "assistant",
                    ConsoleSection::Terminal => "terminal",
                    ConsoleSection::Issues => "issues",
                    ConsoleSection::Log => "log",
                };
                stack.set_visible_child_name(name);
            }
            if let Some(dd) = ui.console_section_dropdown.borrow().as_ref() {
                let idx = match self.model.console_section {
                    ConsoleSection::Assistant => 0,
                    ConsoleSection::Issues => 1,
                    ConsoleSection::Terminal => 2,
                    ConsoleSection::Log => 3,
                };
                if dd.selected() != idx {
                    dd.set_selected(idx);
                }
            }
            if let Some(paned) = ui.console_paned.borrow().as_ref() {
                if let Some(console) = paned.end_child() {
                    console.set_visible(self.model.bottom_panel_visible);
                }
            }
            if let Some(paned) = ui.outer_paned.borrow().as_ref() {
                if let Some(sidebar) = paned.start_child() {
                    sidebar.set_visible(self.model.sidebar_visible);
                }
            }
            if let Some(paned) = ui.inner_paned.borrow().as_ref() {
                if let Some(inspector) = paned.end_child() {
                    inspector.set_visible(self.model.inspector_visible);
                }
            }
        });
    }

    fn refresh_save_sensitivity(&self) {
        // Save button sensitivity follows `canSave`.
        UI.with(|ui| {
            if let Some(action) = ui.save_action.borrow().as_ref() {
                action.set_enabled(self.model.can_save());
            }
            if let Some(action) = ui.save_as_action.borrow().as_ref() {
                action.set_enabled(self.model.document_snapshot.is_some());
            }
            if let Some(action) = ui.save_all_action.borrow().as_ref() {
                action.set_enabled(self.model.project_url.is_some());
            }
        });
    }

    pub fn refresh_shell_warning(&self) {
        let show = {
            let settings = &self.store.settings.build;
            matches!(settings.shell_execution, ShellExecutionPreference::Custom { .. })
                && !settings.custom_shell_acknowledged
        };
        UI.with(|ui| {
            if let Some(b) = ui.shell_warning.borrow().as_ref() {
                b.set_visible(show);
            }
        });
    }

    // ── refresh: tabs / sidebar / footer ───────────────────────────────────

    pub fn refresh_tabs(&mut self) {
        UI.with(|ui| {
            let Some(row) = ui.tab_row.borrow().as_ref().cloned() else { return };
            while let Some(child) = row.first_child() {
                row.remove(&child);
            }
            for url in &self.model.open_documents {
                let active = self.model.active_document_url.as_ref() == Some(url);
                let is_dirty = self
                    .model
                    .document_snapshot
                    .as_ref()
                    .map(|s| active && s.save_state != DocumentSaveState::Clean)
                    .unwrap_or(false);
                let chip = gtk4::Box::new(gtk4::Orientation::Horizontal, 5);
                chip.add_css_class("pitex-tab");
                if active {
                    chip.add_css_class("pitex-tab-active");
                }
                chip.set_margin_start(2);
                chip.set_margin_end(2);
                let title = gtk4::Label::new(Some(
                    &url.file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_default(),
                ));
                if is_dirty {
                    title.add_css_class("dim-label");
                }
                let url2 = url.clone();
                let gesture = gtk4::GestureClick::new();
                gesture.connect_released(move |_, _, _, _| {
                    STATE.with(|s| {
                        if let Some(state) = s.borrow().as_ref() {
                            if let Ok(mut s) = state.try_borrow_mut() { s.activate_document(url2.clone()); }
                        }
                    });
                });
                chip.add_controller(gesture);
                let close = gtk4::Button::from_icon_name("window-close-symbolic");
                close.add_css_class("flat");
                close.set_tooltip_text(Some(&tr(self.language, "editor.close")));
                a11y(&close, "pitex.editor.tabClose", "editor.close");
                let url3 = url.clone();
                close.connect_clicked(move |_| {
                    STATE.with(|s| {
                        if let Some(state) = s.borrow().as_ref() {
                            if let Ok(mut s) = state.try_borrow_mut() { s.close_document(url3.clone()); }
                        }
                    });
                });
                chip.append(&title);
                chip.append(&close);
                a11y(&chip, "pitex.editor.tab", "editor.title");
                row.append(&chip);
            }
        });
    }

    pub fn refresh_sidebar(&mut self) {
        let lang = self.language;
        UI.with(|ui| {
            if let Some(stack) = ui.sidebar_stack.borrow().as_ref() {
                let name = match self.model.sidebar_section {
                    SidebarSection::Outline => "outline",
                    SidebarSection::Labels => "labels",
                    SidebarSection::BibTeX => "bibtex",
                };
                stack.set_visible_child_name(name);
            }
            if let Some(dd) = ui.sidebar_section_dropdown.borrow().as_ref() {
                let idx = match self.model.sidebar_section {
                    SidebarSection::Outline => 0,
                    SidebarSection::Labels => 1,
                    SidebarSection::BibTeX => 2,
                };
                if dd.selected() != idx {
                    dd.set_selected(idx);
                }
            }
            if let Some(list) = ui.outline_list.borrow().as_ref() {
                clear_list(list);
                for (i, item) in self.model.outline_items.iter().enumerate() {
                    let row = gtk4::Label::new(Some(&item.title));
                    row.set_xalign(0.0);
                    row.set_margin_start(8 + item.level as i32 * 10);
                    row.set_margin_top(3);
                    row.set_margin_bottom(3);
                    row.set_ellipsize(gtk4::pango::EllipsizeMode::End);
                    row.set_widget_name(&format!("outline-{i}"));
                    list.append(&row);
                }
                if self.model.outline_items.is_empty() {
                    let row = gtk4::Label::new(Some(&tr(lang, "sidebar.no_outline")));
                    row.add_css_class("dim-label");
                    row.set_margin_top(20);
                    list.append(&row);
                }
            }
            if let Some(list) = ui.labels_list.borrow().as_ref() {
                clear_list(list);
                for (i, item) in self.model.label_items.iter().enumerate() {
                    let row = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
                    let name = gtk4::Label::new(Some(&item.name));
                    name.set_xalign(0.0);
                    name.set_hexpand(true);
                    let line = gtk4::Label::new(Some(&item.line.to_string()));
                    line.add_css_class("dim-label");
                    row.append(&name);
                    row.append(&line);
                    row.set_margin_start(8);
                    row.set_margin_end(8);
                    row.set_margin_top(3);
                    row.set_margin_bottom(3);
                    row.set_widget_name(&format!("label-{i}"));
                    list.append(&row);
                }
                if self.model.label_items.is_empty() {
                    let row = gtk4::Label::new(Some(&tr(lang, "sidebar.no_labels")));
                    row.add_css_class("dim-label");
                    row.set_margin_top(20);
                    list.append(&row);
                }
            }
            if let Some(list) = ui.bib_list.borrow().as_ref() {
                clear_list(list);
                for item in &self.model.bibliography_items {
                    let row = gtk4::Box::new(gtk4::Orientation::Vertical, 1);
                    let key = gtk4::Label::new(Some(&item.key));
                    key.set_xalign(0.0);
                    let meta = gtk4::Label::new(Some(&format!("{} · {}", item.kind, item.file)));
                    meta.set_xalign(0.0);
                    meta.add_css_class("dim-label");
                    row.append(&key);
                    row.append(&meta);
                    row.set_margin_start(8);
                    row.set_margin_top(3);
                    row.set_margin_bottom(3);
                    list.append(&row);
                }
                if self.model.bibliography_items.is_empty() {
                    let row = gtk4::Label::new(Some(&tr(lang, "sidebar.no_bib")));
                    row.add_css_class("dim-label");
                    row.set_margin_top(20);
                    list.append(&row);
                }
            }
            // Project file tree — always visible at the bottom. Built from
            // relative paths through the shared `project-feature` builder
            // so directories nest like SwiftUI's OutlineGroup.
            if let Some(list) = ui.project_list.borrow().as_ref() {
                clear_list(list);
                let root = self.model.project_url.clone();
                let rel_paths: Vec<String> = self
                    .model
                    .project_files
                    .iter()
                    .map(|url| {
                        root.as_ref()
                            .and_then(|r| url.strip_prefix(r).ok().map(|p| p.to_path_buf()))
                            .unwrap_or_else(|| url.clone())
                            .to_string_lossy()
                            .into_owned()
                    })
                    .collect();
                let rel_of = |url: &PathBuf| {
                    root.as_ref()
                        .and_then(|r| url.strip_prefix(r).ok().map(|p| p.to_path_buf()))
                        .unwrap_or_else(|| url.clone())
                        .to_string_lossy()
                        .into_owned()
                };
                let main_rel = self.model.build_source_url().map(|u| rel_of(&u));
                let child_rels: Vec<String> = self
                    .model
                    .project_children
                    .iter()
                    .map(rel_of)
                    .collect();
                let tree = project_feature::nest_project_children(
                    project_feature::build_project_file_tree(&rel_paths),
                    &main_rel.unwrap_or_default(),
                    &child_rels,
                );
                for node in &tree {
                    append_project_node(list, node, 0, &root, &self.model);
                }
            }
            if let Some(pin) = ui.pin_button.borrow().as_ref() {
                pin.set_sensitive(
                    self.model
                        .active_document_url
                        .as_ref()
                        .and_then(|u| u.extension())
                        .map(|e| e.eq_ignore_ascii_case("tex"))
                        .unwrap_or(false),
                );
                pin.set_icon_name(if self.model.pinned_build_target.is_some() {
                    "emblem-favorite-symbolic"
                } else {
                    "emblem-important-symbolic"
                });
            }
        });
    }

    pub fn refresh_footer(&self) {
        UI.with(|ui| {
            if let Some(path) = ui.footer_path.borrow().as_ref() {
                path.set_text(
                    &self
                        .model
                        .active_document_relative_path()
                        .unwrap_or_default(),
                );
            }
            if let Some(words) = ui.footer_words.borrow().as_ref() {
                words.set_text(&format!(
                    "{} {}",
                    if self.model.document_snapshot.is_some() {
                        self.model.word_count().to_string()
                    } else {
                        "—".into()
                    },
                    tr(self.language, "editor.words_unit")
                ));
            }
            if let Some(disk) = ui.disk_status.borrow().as_ref() {
                let conflicted = self
                    .model
                    .document_snapshot
                    .as_ref()
                    .map(|s| s.save_state == DocumentSaveState::Conflicted)
                    .unwrap_or(false);
                disk.set_tooltip_text(Some(&tr(
                    self.language,
                    if conflicted {
                        "editor.disk_changed"
                    } else {
                        "editor.disk_unchanged"
                    },
                )));
            }
        });
    }

    fn refresh_conflict_banner(&self) {
        let conflicted = self
            .model
            .document_snapshot
            .as_ref()
            .map(|s| s.save_state == DocumentSaveState::Conflicted)
            .unwrap_or(false);
        UI.with(|ui| {
            if let Some(banner) = ui.conflict_banner.borrow().as_ref() {
                banner.set_visible(conflicted);
            }
            if conflicted {
                if let Some(label) = ui.conflict_label.borrow().as_ref() {
                    let name = self
                        .model
                        .active_document_url
                        .as_ref()
                        .and_then(|u| u.file_name().map(|n| n.to_string_lossy().into_owned()))
                        .unwrap_or_default();
                    label.set_text(&crate::l10n::tr1(
                        self.language,
                        "conflict.message",
                        &name,
                    ));
                }
            }
        });
    }

    // ── refresh: issues / log / build ──────────────────────────────────────

    pub fn refresh_issues(&mut self) {
        let issues = self.filtered_issues();
        let lang = self.language;
        UI.with(|ui| {
            if let Some(list) = ui.issues_list.borrow().as_ref() {
                clear_list(list);
                if issues.is_empty() {
                    let row = gtk4::Label::new(Some(&tr(lang, "console.no_issues")));
                    row.add_css_class("dim-label");
                    row.set_margin_top(20);
                    list.append(&row);
                    return;
                }
                for (i, issue) in issues.iter().enumerate() {
                    let row = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
                    let icon = gtk4::Image::from_icon_name(match issue.severity {
                        build_core::BuildIssueSeverity::Error => "dialog-error-symbolic",
                        build_core::BuildIssueSeverity::Warning => "dialog-warning-symbolic",
                    });
                    let text = gtk4::Label::new(Some(&issue.message));
                    text.set_xalign(0.0);
                    text.set_hexpand(true);
                    text.set_wrap(true);
                    let loc = gtk4::Label::new(Some(&match (&issue.file, issue.line) {
                        (Some(f), Some(l)) => format!("{f}:{l}"),
                        (Some(f), None) => f.clone(),
                        _ => String::new(),
                    }));
                    loc.add_css_class("dim-label");
                    row.append(&icon);
                    row.append(&text);
                    row.append(&loc);
                    row.set_margin_start(8);
                    row.set_margin_end(8);
                    row.set_margin_top(4);
                    row.set_margin_bottom(4);
                    row.set_widget_name(&format!("issue-{i}"));
                    if !issue.is_clickable() {
                        row.set_sensitive(false);
                    }
                    list.append(&row);
                }
            }
        });
    }

    pub fn refresh_build_ui(&mut self) {
        let building = self.model.is_building();
        UI.with(|ui| {
            if let Some(b) = ui.build_button.borrow().as_ref() {
                b.set_icon_name(if building {
                    "media-playback-stop-symbolic"
                } else {
                    "media-playback-start-symbolic"
                });
                b.set_sensitive(building || self.model.build_unavailable_reason().is_none());
                b.set_tooltip_text(Some(
                    &self
                        .model
                        .build_unavailable_reason()
                        .unwrap_or_else(|| tr(self.language, "build.start")),
                ));
            }
            if let Some(b) = ui.header_build_button.borrow().as_ref() {
                b.set_icon_name(if building {
                    "media-playback-stop-symbolic"
                } else {
                    "media-playback-start-symbolic"
                });
                b.set_sensitive(building || self.model.build_unavailable_reason().is_none());
            }
            if let Some(label) = ui.build_status.borrow().as_ref() {
                label.set_text(&match &self.model.build_state {
                    WorkspaceBuildState::Building => tr(self.language, "build.running"),
                    WorkspaceBuildState::Succeeded { .. } => tr(self.language, "build.succeeded"),
                    WorkspaceBuildState::Failed(reason) => reason.clone(),
                    WorkspaceBuildState::Unavailable(reason) => reason.clone(),
                });
            }
            if let Some(view) = ui.build_log_view.borrow().as_ref() {
                let text = if self.model.build_log_text.is_empty() {
                    tr(self.language, "build.log_empty")
                } else {
                    self.model.build_log_text.clone()
                };
                let buffer = view.buffer();
                buffer.set_text(&text);
                let mut end = buffer.end_iter();
                view.scroll_to_iter(&mut end, 0.0, false, 0.0, 1.0);
            }
        });
        self.refresh_issues();
    }

    // ── refresh: PDF / synctex ─────────────────────────────────────────────

    pub fn refresh_pdf_ui(&mut self) {
        let pdf_data = match &self.model.build_state {
            WorkspaceBuildState::Succeeded { pdf, .. } => Some(pdf.clone()),
            _ => None,
        };
        match (pdf_data, &self.pdf) {
            (Some(data), _) => {
                use std::hash::{Hash, Hasher};
                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                data.hash(&mut hasher);
                let hash = hasher.finish();
                if self.pdf.is_none() || self.pdf_hash != hash {
                    self.pdf = PdfDocument::from_data(&data);
                    self.pdf_hash = hash;
                    self.pdf_page = 0;
                    self.pdf_auto_fit = true;
                    // `clearSyncHighlight()` on document replacement — the old
                    // page's marker must not linger over the new document.
                    self.clear_synctex_highlight();
                }
                UI.with(|ui| {
                    if let Some(p) = ui.pdf_empty.borrow().as_ref() {
                        p.set_visible(false);
                    }
                    for w in [
                        ui.pdf_scroll.borrow().as_ref().cloned().map(|w| w.upcast::<gtk4::Widget>()),
                        ui.pdf_toolbar.borrow().as_ref().cloned().map(|w| w.upcast::<gtk4::Widget>()),
                    ]
                    .into_iter()
                    .flatten()
                    {
                        w.set_visible(true);
                    }
                    if let Some(nav) = ui
                        .pdf_page_label
                        .borrow()
                        .as_ref()
                        .and_then(|l| l.parent())
                    {
                        nav.set_visible(true);
                    }
                    if let Some(name) = ui.pdf_name_label.borrow().as_ref() {
                        name.set_text(&self.pdf_display_name());
                    }
                });
                if self.store.switch_to_pdf_on_build() && !self.model.inspector_visible {
                    self.model.inspector_visible = true;
                    self.refresh_console_visibility();
                }
                self.render_pdf_page();
            }
            (None, _) => {
                UI.with(|ui| {
                    if let Some(p) = ui.pdf_empty.borrow().as_ref() {
                        p.set_visible(true);
                    }
                    if let Some(s) = ui.pdf_scroll.borrow().as_ref() {
                        s.set_visible(false);
                    }
                    if let Some(t) = ui.pdf_toolbar.borrow().as_ref() {
                        t.set_visible(false);
                    }
                    if let Some(nav) = ui
                        .pdf_page_label
                        .borrow()
                        .as_ref()
                        .and_then(|l| l.parent())
                    {
                        nav.set_visible(false);
                    }
                });
            }
        }
        self.refresh_synctex_status();
    }

    fn refresh_synctex_status(&self) {
        UI.with(|ui| {
            let (icon_name, detail) = match &self.model.synctex_state {
                WorkspaceSyncTeXState::Current => ("emblem-ok-symbolic", tr(self.language, "preview.synctex")),
                WorkspaceSyncTeXState::Stale(msg) => ("dialog-warning-symbolic", msg.clone()),
                WorkspaceSyncTeXState::Ambiguous(msg) => ("dialog-question-symbolic", msg.clone()),
                WorkspaceSyncTeXState::Unavailable(msg) => ("process-stop-symbolic", msg.clone()),
            };
            if let Some(icon) = ui.synctex_status_icon.borrow().as_ref() {
                icon.set_icon_name(Some(icon_name));
            }
            if let Some(label) = ui.synctex_status_label.borrow().as_ref() {
                label.set_text(&detail);
            }
        });
    }

    // ── refresh: assistant ─────────────────────────────────────────────────

    pub fn refresh_assistant(&mut self) {
        self.ensure_agent();
        let lang = self.language;
        let agent = self.agent.as_mut().unwrap();
        UI.with(|ui| {
            // Transcript rebuild (small transcripts; SwiftUI redraws anyway).
            let font_size = self.store.ai_font_size();
            if let Some(box_) = ui.transcript_box.borrow().as_ref() {
                while let Some(child) = box_.first_child() {
                    box_.remove(&child);
                }
                if agent.transcript.is_empty() {
                    let empty = adw::StatusPage::new();
                    empty.set_title(&tr(lang, "assistant.empty_headline"));
                    empty.set_icon_name(Some("starred-symbolic"));
                    empty.add_css_class("pitex-conv-title");
                    let detail = match &agent.connection {
                        crate::agent::Connection::Idle => {
                            tr(lang, "assistant.configure_hint")
                        }
                        crate::agent::Connection::PiMissing => tr(lang, "assistant.pi_missing"),
                        crate::agent::Connection::Connecting => tr(lang, "state.loading"),
                        _ => String::new(),
                    };
                    if !detail.is_empty() {
                        empty.set_description(Some(&detail));
                    }
                    box_.append(&empty);
                } else {
                    for entry in &agent.transcript {
                        box_.append(&transcript_row(entry, font_size));
                    }
                }
                if let Some(scroll) = ui.transcript_scroll.borrow().as_ref() {
                    let adj = scroll.vadjustment();
                    adj.set_value(adj.upper());
                }
            }
            if let Some(label) = ui.agent_status_label.borrow().as_ref() {
                label.set_text(agent.status_message.as_deref().unwrap_or(""));
                label.set_visible(agent.status_message.is_some());
            }
            // Model picker — when models span multiple providers the
            // provider name disambiguates same-named entries.
            if let Some(picker) = ui.agent_model_picker.borrow().as_ref() {
                let providers: std::collections::HashSet<&str> =
                    agent.models.iter().map(|m| m.provider.as_str()).collect();
                let disambiguate = providers.len() > 1;
                let titles: Vec<String> = if agent.models.is_empty() {
                    vec![tr(lang, "assistant.model_placeholder")]
                } else {
                    agent
                        .models
                        .iter()
                        .map(|m| {
                            if disambiguate {
                                format!("{} · {}", m.picker_title(), m.provider)
                            } else {
                                m.picker_title()
                            }
                        })
                        .collect()
                };
                let list = gtk4::StringList::new(&titles.iter().map(String::as_str).collect::<Vec<_>>());
                picker.set_model(Some(&list));
                picker.set_sensitive(
                    !agent.models.is_empty() && !agent.is_updating_model_settings,
                );
                if let Some(current) = agent.current_model.as_ref() {
                    if let Some(idx) = agent.models.iter().position(|m| m == current) {
                        picker.set_selected(idx as u32);
                    }
                } else {
                    picker.set_selected(gtk4::INVALID_LIST_POSITION);
                }
            }
            if let Some(picker) = ui.agent_reasoning_picker.borrow().as_ref() {
                let levels = &agent.thinking_levels;
                picker.set_visible(
                    agent.connection == crate::agent::Connection::Ready && !levels.is_empty(),
                );
                let list =
                    gtk4::StringList::new(&levels.iter().map(String::as_str).collect::<Vec<_>>());
                picker.set_model(Some(&list));
                picker.set_sensitive(levels.len() >= 2 && !agent.is_updating_model_settings);
                if let Some(idx) = levels.iter().position(|l| *l == agent.thinking_level) {
                    picker.set_selected(idx as u32);
                }
            }
            {
                let mut slot = ui.agent_composer_font.borrow_mut();
                if slot.is_none() {
                    let p = gtk4::CssProvider::new();
                    if let Some(display) = gtk4::gdk::Display::default() {
                        gtk4::style_context_add_provider_for_display(
                            &display,
                            &p,
                            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
                        );
                    }
                    *slot = Some(p);
                }
                if let Some(provider) = slot.as_ref() {
                    provider.load_from_data(&format!(
                        ".pitex-conv-font {{ font-size: {font_size}pt; }} \
                         .pitex-conv-title .title-1 {{ font-size: {font_size}pt; }}"
                    ));
                }
            }
            if let Some(t) = ui.agent_attach_toggle.borrow().as_ref() {
                if t.is_active() != agent.attach_active_document {
                    t.set_active(agent.attach_active_document);
                }
            }
            // Usage/context readout — `get_session_stats` payload formatted
            // compactly ("12.3k/200k ctx (6%) · 45.2k tok · $0.12").
            if let Some(label) = ui.agent_usage_label.borrow().as_ref() {
                if let Some(stats) = &agent.session_stats {
                    label.set_text(&crate::l10n::tr1(
                        lang,
                        "assistant.usage",
                        &format_session_stats(stats),
                    ));
                    label.set_visible(true);
                } else {
                    label.set_visible(false);
                }
            }
            if let Some(b) = ui.agent_send_button.borrow().as_ref() {
                b.set_visible(!agent.is_running);
            }
            if let Some(b) = ui.agent_stop_button.borrow().as_ref() {
                b.set_visible(agent.is_running);
            }
        });
        self.refresh_selection_chip();
    }

    pub fn refresh_selection_chip(&mut self) {
        let attachment = self
            .agent
            .as_ref()
            .and_then(|a| a.selection_attachment.clone());
        UI.with(|ui| {
            if let (Some(chip), Some(label)) = (
                ui.selection_chip.borrow().as_ref(),
                ui.selection_chip_label.borrow().as_ref(),
            ) {
                if let Some(a) = attachment {
                    let file = a.path.rsplit('/').next().unwrap_or(&a.path);
                    let text = if a.start_line == a.end_line {
                        format!("{file}:{}", a.start_line)
                    } else {
                        format!("{file}:{}–{}", a.start_line, a.end_line)
                    };
                    label.set_text(&text);
                    chip.set_visible(true);
                } else {
                    chip.set_visible(false);
                }
            }
        });
    }

    /// Drain the agent's event receiver + check for unexpected exit.
    fn poll_agent(&mut self) {
        self.ensure_agent();
        *self.context_cell.borrow_mut() = self.model.agent_context();
        let agent = self.agent.as_mut().unwrap();
        agent.poll_toolchain();
        while let Some(event) = agent.poll_event() {
            agent.handle(&event);
        }
        if agent.poll_exit() {
            agent.process_did_exit();
        }
        if self.agent_activity_pending.replace(false) {
            let confirm = self.store.confirm_overwrite();
            self.model.refresh_after_agent_activity(confirm);
            if let Some(editor) = &self.editor {
                editor.refresh_from_session();
            }
            self.refresh_after_document_change();
        }
        self.refresh_assistant();
    }

    // ── watchers ───────────────────────────────────────────────────────────

    /// `FileSystemWatcher` — monitor every project file, coalesced 0.35s.
    fn install_watchers(&mut self) {
        self.watchers.clear();
        for url in &self.model.project_files {
            let file = gio::File::for_path(url);
            let Ok(monitor) = file.monitor_file(gio::FileMonitorFlags::NONE, gio::Cancellable::NONE)
            else {
                continue;
            };
            let path = url.clone();
            monitor.connect_changed(move |_, _, _, event| {
                if !matches!(
                    event,
                    gio::FileMonitorEvent::Changed
                        | gio::FileMonitorEvent::ChangesDoneHint
                        | gio::FileMonitorEvent::Created
                        | gio::FileMonitorEvent::Deleted
                ) {
                    return;
                }
                let path = path.clone();
                STATE.with(|s| {
                    if let Some(state) = s.borrow().as_ref() {
                        // Coalesce: one process_disk_change per file per 350ms.
                        let Ok(st) = state.try_borrow_mut() else { return };
                        if let Some(old) = st.disk_pending.borrow_mut().remove(&path) {
                            old.remove();
                        }
                        let p = path.clone();
                        let id = glib::timeout_add_local_once(
                            Duration::from_millis(350),
                            move || {
                                STATE.with(|s| {
                                    if let Some(state) = s.borrow().as_ref() {
                                        let Ok(mut st) = state.try_borrow_mut() else { return };
                                        st.disk_pending.borrow_mut().remove(&p);
                                        let confirm = st.store.confirm_overwrite();
                                        st.model.process_disk_change(&p, confirm);
                                        if p == *st.model.active_document_url.as_ref().unwrap_or(&PathBuf::new()) {
                                            if let Some(editor) = &st.editor {
                                                editor.refresh_from_session();
                                            }
                                        }
                                        st.refresh_after_document_change();
                                    }
                                });
                            },
                        );
                        st.disk_pending.borrow_mut().insert(path, id);
                    }
                });
            });
            self.watchers.push(monitor);
        }
    }

    /// Watch the pi agent dir: edits to `auth.json`, `models.json`, or
    /// `settings.json` (Settings → AI, `pi /login`, a text editor) restart
    /// the agent so the new provider config applies without an app restart.
    /// Coalesced ~500ms like the project-file watchers.
    fn install_agent_config_watch(&mut self) {
        let dir = crate::agent::pi_paths::agent_directory();
        if std::fs::create_dir_all(&dir).is_err() {
            return;
        }
        let file = gio::File::for_path(&dir);
        let Ok(monitor) =
            file.monitor_directory(gio::FileMonitorFlags::NONE, gio::Cancellable::NONE)
        else {
            return;
        };
        monitor.connect_changed(move |_, file, _, event| {
            if !matches!(
                event,
                gio::FileMonitorEvent::Changed
                    | gio::FileMonitorEvent::ChangesDoneHint
                    | gio::FileMonitorEvent::Created
                    | gio::FileMonitorEvent::Deleted
                    | gio::FileMonitorEvent::Moved
            ) {
                return;
            }
            let name = file
                .basename()
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_default();
            if !matches!(name.as_str(), "auth.json" | "models.json" | "settings.json") {
                return;
            }
            STATE.with(|s| {
                if let Some(state) = s.borrow().as_ref() {
                    let Ok(st) = state.try_borrow_mut() else { return };
                    let generation = st.agent_config_generation.get() + 1;
                    st.agent_config_generation.set(generation);
                    glib::timeout_add_local_once(Duration::from_millis(500), move || {
                        STATE.with(|s| {
                            if let Some(state) = s.borrow().as_ref() {
                                let Ok(mut st) = state.try_borrow_mut() else { return };
                                if st.agent_config_generation.get() != generation {
                                    return;
                                }
                                st.ensure_agent();
                                if let Some(agent) = st.agent.as_mut() {
                                    agent.restart();
                                }
                                st.refresh_assistant();
                            }
                        });
                    });
                }
            });
        });
        self.agent_config_monitor = Some(monitor);
    }

    // ── workspace message dispatch ─────────────────────────────────────────

    fn dispatch(&mut self, message: WorkspaceMessage) {
        match message {
            WorkspaceMessage::OpenFinished(result) => match result {
                Ok(opened) => {
                    let session = opened.session.clone();
                    self.model.apply_open(&mut self.store, opened);
                    self.attach_session(session);
                    self.install_watchers();
                    self.refresh_phase();
                }
                Err(e) => {
                    // `open()`'s catch runs `await close()` — including the
                    // agent shutdown — before surfacing the failure.
                    self.shutdown_agent();
                    self.model.open_failed(e);
                    self.refresh_phase();
                }
            },
            WorkspaceMessage::ActivateFinished(result) => match result {
                Ok(activated) => {
                    let session = activated.session.clone();
                    self.model.apply_activate(activated);
                    self.attach_session(session);
                    self.refresh_after_document_change();
                }
                // `activateDocument`'s catch publishes `.failed` — the
                // workspace never went through a loading phase first.
                Err(e) => {
                    self.model.phase = crate::model::WorkspacePhase::Failed(e);
                    self.refresh_phase();
                }
            },
            WorkspaceMessage::BuildEvent(event) => {
                self.model.apply_build_event(event);
                self.refresh_build_ui();
            }
            WorkspaceMessage::BuildFinished { outcome, output_pdf } => {
                self.model.apply_build_finished(
                    outcome,
                    &output_pdf,
                    self.store.switch_to_pdf_on_build(),
                    self.store.jump_to_cursor_after_build(),
                );
                self.refresh_build_ui();
                self.refresh_pdf_ui();
            }
            WorkspaceMessage::ForwardResult(result) => {
                self.model.apply_forward_result(result);
                let highlight = self.highlight_cell.borrow_mut().take();
                if let Some((page, x, y, w, h)) = highlight {
                    self.show_synctex_highlight(page, x, y, w, h);
                }
                self.refresh_synctex_status();
            }
            WorkspaceMessage::InverseResult(result) => {
                if let Some(tx) = self.tx.clone() {
                    self.model.apply_inverse_result(result, tx);
                }
                // `jumpTo(..., highlight: settings.inverseSyncHighlight)` —
                // same-file inverse navigation lands here; a cross-file jump
                // arrives through `drain_side_effects` after activation.
                let jump = self.jump_cell.borrow_mut().take();
                if let Some((line, col)) = jump {
                    let highlight = self.store.inverse_sync_highlight();
                    self.jump_to(line, col, highlight);
                }
                self.refresh_synctex_status();
            }
            WorkspaceMessage::BindingRefreshed(result) => {
                self.model.apply_binding_refreshed(result);
                self.refresh_synctex_status();
            }
            WorkspaceMessage::DiskChanged(path) => {
                let confirm = self.store.confirm_overwrite();
                self.model.process_disk_change(&path, confirm);
                self.refresh_after_document_change();
            }
            WorkspaceMessage::AgentActivityFinished => {
                let confirm = self.store.confirm_overwrite();
                self.model.refresh_after_agent_activity(confirm);
                if let Some(editor) = &self.editor {
                    editor.refresh_from_session();
                }
                self.refresh_after_document_change();
            }
        }
        self.drain_side_effects();
    }
}

// ─── module-level shared handles ─────────────────────────────────────────────

thread_local! {
    static UI: UiHandles = UiHandles::default();
    pub(crate) static STATE: RefCell<Option<Rc<RefCell<AppState>>>> = const { RefCell::new(None) };
    /// The pending autosave `glib` source — removed and re-armed on every
    /// edit, which is the `autosaveTask?.cancel()` half of Swift's debounce.
    static AUTOSAVE_SOURCE: RefCell<Option<glib::SourceId>> = const { RefCell::new(None) };
    /// Resolved UI language for `a11y` — readable without borrowing state.
    static LANG: Cell<&'static str> = const { Cell::new("en") };
}

fn clear_list(list: &gtk4::ListBox) {
    while let Some(row) = list.row_at_index(0) {
        list.remove(&row);
    }
}

/// Recursive renderer for the project tree — mirrors `OutlineGroup` in
/// `ProjectSidebarView.swift`. Directories toggle collapse state (tracked
/// in `WorkspaceModel::collapsed_project_dirs`); files activate documents.
fn append_project_node(
    list: &gtk4::ListBox,
    node: &project_feature::ProjectFileNode,
    depth: u32,
    root: &Option<PathBuf>,
    model: &crate::model::WorkspaceModel,
) {
    let indent = 8 + (depth as i32) * 14;
    let row = gtk4::Box::new(gtk4::Orientation::Horizontal, 7);
    row.set_margin_start(indent);
    row.set_margin_end(8);
    row.set_margin_top(3);
    row.set_margin_bottom(3);

    if node.is_directory {
        let collapsed = model.collapsed_project_dirs.contains(&node.path);
        let disclosure = gtk4::Image::from_icon_name(if collapsed {
            "pan-end-symbolic"
        } else {
            "pan-down-symbolic"
        });
        let icon = gtk4::Image::from_icon_name("folder-symbolic");
        let name = gtk4::Label::new(Some(&node.name));
        name.set_xalign(0.0);
        name.set_hexpand(true);
        name.set_ellipsize(gtk4::pango::EllipsizeMode::Middle);
        row.append(&disclosure);
        row.append(&icon);
        row.append(&name);
        let dir_path = node.path.clone();
        let gesture = gtk4::GestureClick::new();
        gesture.connect_released(move |_, _, _, _| {
            STATE.with(|s| {
                if let Some(state) = s.borrow().as_ref() {
                    if let Ok(mut s) = state.try_borrow_mut() {
                        if !s.model.collapsed_project_dirs.remove(&dir_path) {
                            s.model.collapsed_project_dirs.insert(dir_path.clone());
                        }
                        s.refresh_sidebar();
                    }
                }
            });
        });
        row.add_controller(gesture);
        list.append(&row);
        if !collapsed {
            for child in node.children.as_deref().unwrap_or(&[]) {
                append_project_node(list, child, depth + 1, root, model);
            }
        }
        return;
    }

    let url = root
        .as_ref()
        .map(|r| r.join(&node.path))
        .unwrap_or_else(|| PathBuf::from(&node.path));
    let icon = gtk4::Image::from_icon_name(
        if url.extension().map(|e| e == "bib").unwrap_or(false) {
            "accessories-dictionary-symbolic"
        } else {
            "x-office-document-symbolic"
        },
    );
    // Spacer keeps file labels aligned under the directory labels' names —
    // files have no disclosure triangle.
    let spacer = gtk4::Label::new(None);
    spacer.set_width_chars(1);
    let name = gtk4::Label::new(Some(&node.name));
    name.set_xalign(0.0);
    name.set_hexpand(true);
    name.set_ellipsize(gtk4::pango::EllipsizeMode::Middle);
    row.append(&spacer);
    row.append(&icon);
    row.append(&name);
    if model.pinned_build_target.as_ref() == Some(&url) {
        row.append(&gtk4::Image::from_icon_name("emblem-important-symbolic"));
    } else if model.automatic_build_target.as_ref() == Some(&url) {
        // `hammer` — Adwaita's engineering icon marks the inferred main
        // document like the SF Symbol does.
        row.append(&gtk4::Image::from_icon_name(
            "applications-engineering-symbolic",
        ));
    }
    if model.active_document_url.as_ref() == Some(&url) {
        row.add_css_class("pitex-tab-active");
    }
    let gesture = gtk4::GestureClick::new();
    gesture.connect_released(move |_, _, _, _| {
        STATE.with(|s| {
            if let Some(state) = s.borrow().as_ref() {
                if let Ok(mut s) = state.try_borrow_mut() {
                    s.activate_document(url.clone());
                }
            }
        });
    });
    row.add_controller(gesture);
    list.append(&row);
    // Dependency children nested by `nest_project_children` render one level
    // deeper; files have no disclosure so they are always visible.
    for child in node.children.as_deref().unwrap_or(&[]) {
        append_project_node(list, child, depth + 1, root, model);
    }
}

fn dialect_for(url: Option<&Path>) -> TeXDialect {
    match url.and_then(|u| u.extension()).and_then(|e| e.to_str()) {
        Some("bib") => TeXDialect::Bibtex,
        _ => TeXDialect::Latex,
    }
}

fn token_kinds() -> Vec<language_core::LanguageTokenKind> {
    use language_core::LanguageTokenKind as K;
    vec![
        K::ControlSequence(String::new()),
        K::Comment(String::new()),
        K::LeftBrace,
        K::RightBrace,
        K::Whitespace(String::new()),
        K::Text(String::new()),
        K::BibEntryMarker,
        K::Punctuation(String::new()),
        K::EnvironmentName(String::new()),
        K::Math(String::new()),
    ]
}

fn color_role_for(kind: &language_core::LanguageTokenKind) -> AppearanceColorRole {
    use language_core::LanguageTokenKind as K;
    match kind {
        K::ControlSequence(_) => AppearanceColorRole::Commands,
        K::Comment(_) => AppearanceColorRole::Comments,
        K::LeftBrace | K::RightBrace | K::Punctuation(_) => AppearanceColorRole::Braces,
        K::EnvironmentName(_) => AppearanceColorRole::Environments,
        K::Math(_) => AppearanceColorRole::Math,
        K::BibEntryMarker => AppearanceColorRole::Commands,
        _ => AppearanceColorRole::BodyText,
    }
}

/// Generate a GtkSourceView style scheme from the appearance palette — the
/// native mechanism covering gutter, line numbers, selection and tokens.
fn style_scheme_xml(
    appearance: &AppearanceSettings,
    prefs: &crate::settings::Preferences,
) -> String {
    let hex = |role: AppearanceColorRole| appearance.stored_hex(prefs, role);
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<style-scheme id="pitex-dynamic" _name="Pitex" version="1.0">
  <author>Pitex</author>
  <color name="bg" value="{bg}"/>
  <color name="fg" value="{fg}"/>
  <color name="gutter" value="{gutter}"/>
  <color name="linenr" value="{linenr}"/>
  <color name="command" value="{command}"/>
  <color name="env" value="{env}"/>
  <color name="math" value="{math}"/>
  <color name="brace" value="{brace}"/>
  <color name="comment" value="{comment}"/>
  <color name="bracket" value="{bracket}"/>
  <style name="text" foreground="fg" background="bg"/>
  <style name="background" background="bg"/>
  <style name="selection" background="bracket"/>
  <style name="current-line" background="gutter"/>
  <style name="line-numbers" foreground="linenr" background="gutter"/>
  <style name="right-margin" foreground="linenr"/>
  <style name="bracket-match" foreground="bracket" bold="true"/>
  <style name="bracket-mismatch" foreground="comment"/>
  <style name="search-match" background="math"/>
</style-scheme>"#,
        bg = hex(AppearanceColorRole::EditorBackground),
        fg = hex(AppearanceColorRole::BodyText),
        gutter = hex(AppearanceColorRole::GutterBackground),
        linenr = hex(AppearanceColorRole::LineNumbers),
        command = hex(AppearanceColorRole::Commands),
        env = hex(AppearanceColorRole::Environments),
        math = hex(AppearanceColorRole::Math),
        brace = hex(AppearanceColorRole::Braces),
        comment = hex(AppearanceColorRole::Comments),
        bracket = hex(AppearanceColorRole::BracketMatch),
    )
}

/// UTF-16 offset → (0-based line, byte column) used for forward SyncTeX.
fn utf16_to_line_col(text: &str, utf16_offset: usize) -> (usize, usize) {
    let mut units = 0usize;
    let mut line = 0usize;
    let mut line_start_units = 0usize;
    for c in text.chars() {
        if units >= utf16_offset {
            break;
        }
        if c == '\n' {
            line += 1;
            line_start_units = units + 1;
        }
        units += c.len_utf16();
    }
    (line, utf16_offset.saturating_sub(line_start_units))
}

/// (0-based line, 0-based column in characters) → UTF-16 offset.
fn line_col_to_utf16(text: &str, line: usize, column: usize) -> usize {
    let mut units = 0usize;
    for (i, l) in text.split('\n').enumerate() {
        if i == line {
            units += l.chars().take(column).map(|c| c.len_utf16()).sum::<usize>();
            return units;
        }
        units += l.chars().map(|c| c.len_utf16()).sum::<usize>() + 1;
    }
    units
}

/// `.font(.system(size:))` for labels — pt size via Pango attributes.
pub(crate) fn font_attrs(size: f64) -> gtk4::pango::AttrList {
    let attrs = gtk4::pango::AttrList::new();
    attrs.insert(gtk4::pango::AttrSize::new(
        (size * gtk4::pango::SCALE as f64) as i32,
    ));
    attrs
}

/// Compact token count — "45.2k" past 999, raw below.
fn compact_tokens(n: u64) -> String {
    if n >= 1000 {
        format!("{:.1}k", n as f64 / 1000.0)
    } else {
        n.to_string()
    }
}

/// `12.3k/200k ctx (6%) · 45.2k tok · $0.12` — the assistant usage readout.
fn format_session_stats(stats: &crate::agent::PiSessionStats) -> String {
    let mut parts: Vec<String> = Vec::new();
    if let (Some(tokens), Some(window)) = (stats.context_tokens, stats.context_window) {
        let mut ctx = format!("{}/{} ctx", compact_tokens(tokens), compact_tokens(window));
        if let Some(percent) = stats.context_percent {
            ctx.push_str(&format!(" ({:.0}%)", percent));
        }
        parts.push(ctx);
    }
    parts.push(format!("{} tok", compact_tokens(stats.total_tokens)));
    parts.push(format!("${:.2}", stats.cost));
    parts.join(" · ")
}

fn transcript_row(entry: &crate::agent::AgentTranscriptEntry, font_size: f64) -> gtk4::Widget {
    let caption_size = (font_size - 2.0).max(9.0);
    let row = gtk4::Box::new(gtk4::Orientation::Vertical, 3);
    let (role, icon) = match entry.role {
        crate::agent::TranscriptRole::User => ("You".to_string(), "avatar-default-symbolic"),
        crate::agent::TranscriptRole::Assistant => {
            ("Assistant".to_string(), "starred-symbolic")
        }
        crate::agent::TranscriptRole::Thinking => {
            ("Thinking".to_string(), "weather-fog-symbolic")
        }
        crate::agent::TranscriptRole::Tool => {
            ("Tool".to_string(), "emblem-system-symbolic")
        }
        crate::agent::TranscriptRole::Notice => {
            ("Status".to_string(), "dialog-information-symbolic")
        }
    };
    let header = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
    header.append(&gtk4::Image::from_icon_name(icon));
    let title_text = if entry.title.is_empty() { role } else { entry.title.clone() };
    let role_label = gtk4::Label::new(Some(&title_text));
    role_label.add_css_class("dim-label");
    role_label.set_xalign(0.0);
    role_label.set_ellipsize(gtk4::pango::EllipsizeMode::End);
    role_label.set_attributes(Some(&font_attrs(caption_size)));
    header.append(&role_label);
    row.append(&header);
    if !entry.text.is_empty() {
        let body = gtk4::Label::new(Some(&entry.text));
        body.set_xalign(0.0);
        body.set_wrap(true);
        body.set_selectable(true);
        // Assistant/user text at `fontSize`; thinking/tool captions scale down.
        let size = match entry.role {
            crate::agent::TranscriptRole::User | crate::agent::TranscriptRole::Assistant => {
                font_size
            }
            _ => caption_size,
        };
        body.set_attributes(Some(&font_attrs(size)));
        match entry.status {
            crate::agent::TranscriptStatus::Streaming | crate::agent::TranscriptStatus::Running => {
                body.add_css_class("dim-label")
            }
            crate::agent::TranscriptStatus::Failed => body.add_css_class("error"),
            _ => {}
        }
        row.append(&body);
    }
    if !entry.detail.is_empty() {
        let detail = gtk4::Label::new(Some(&entry.detail));
        detail.set_xalign(0.0);
        detail.set_wrap(true);
        detail.set_selectable(true);
        detail.add_css_class("dim-label");
        detail.add_css_class("monospace");
        detail.set_attributes(Some(&font_attrs(caption_size)));
        row.append(&detail);
    }
    row.upcast()
}

// ─── window assembly ─────────────────────────────────────────────────────────

pub fn run(app_version: &str) -> i32 {
    let app = adw::Application::builder()
        .application_id(APP_ID)
        .flags(gio::ApplicationFlags::HANDLES_OPEN)
        .build();
    let version = app_version.to_string();
    {
        let version = version.clone();
        app.connect_activate(move |app| {
            build_window(app, &version);
            // Session restore: `open` launches skip `activate`, so this
            // only fires when the app starts without a file argument.
            STATE.with(|s| {
                if let Some(state) = s.borrow().as_ref() {
                    if let Ok(mut st) = state.try_borrow_mut() {
                        let st = &mut *st;
                        if let Some(tx) = st.tx.clone() {
                            st.model.restore_session_if_needed(&st.store, tx);
                        }
                    }
                }
            });
        });
    }
    // `application(_:open:)` — files passed on the command line (or via the
    // desktop file) open as projects, mirroring the macOS entry point.
    app.connect_open(move |app, files, _hint| {
        let running = STATE.with(|s| s.borrow().is_some());
        if !running {
            build_window(app, &version);
        }
        for file in files {
            if let Some(path) = file.path() {
                STATE.with(|s| {
                    if let Some(state) = s.borrow().as_ref() {
                        if let Ok(mut st) = state.try_borrow_mut() {
                            st.open_selected(path.to_path_buf());
                        }
                    }
                });
            }
        }
    });
    app.run().value()
}

fn build_window(app: &adw::Application, app_version: &str) {
    // Channel: worker threads → main context dispatch. The std mpsc receiver
    // is drained by a 10ms tick on the GTK main loop (glib 0.19 removed
    // `MainContext::channel`; a poll keeps `Rc<RefCell<AppState>>` main-only).
    let (model_tx, model_rx) = std::sync::mpsc::channel::<WorkspaceMessage>();

    let state = Rc::new(RefCell::new(AppState::new(
        SettingsStore::new(Preferences::standard()),
        model_tx,
        app_version.to_string(),
    )));
    let lang = resolve_language(state.borrow().appearance.language);
    state.borrow_mut().language = lang;
    LANG.with(|l| l.set(lang));
    STATE.with(|s| *s.borrow_mut() = Some(state.clone()));
    state.borrow_mut().install_agent_config_watch();

    UI.with(|ui| build_chrome(app, &state, ui, model_rx));

    // Auto-update: check+download+install on a worker thread when the
    // preference allows it. `ExitRequested` means the Windows updater is
    // staged and the process must leave so it can swap binaries.
    if state.borrow().store.auto_install_updates() {
        let version = app_version.to_string();
        std::thread::spawn(move || {
            let result = crate::update::auto_update(&version);
            // `invoke` needs `Send` — only the result crosses; the state is
            // reached through the main-thread-local `STATE` like elsewhere.
            gtk4::glib::MainContext::default().invoke(move || {
                let Some(state) = STATE.with(|s| s.borrow().clone()) else { return };
                let Ok(st) = state.try_borrow() else { return };
                match result {
                    Ok(Some((info, outcome))) => match outcome {
                        crate::update::InstallOutcome::ExitRequested => {
                            std::process::exit(0);
                        }
                        crate::update::InstallOutcome::AwaitingRestart => {
                            st.toast(&crate::l10n::tr1(
                                st.language,
                                "settings.updates.installed_version",
                                &info.tag,
                            ));
                        }
                        crate::update::InstallOutcome::HandedToTerminal => {
                            st.toast(&crate::l10n::tr(
                                st.language,
                                "settings.updates.install_terminal",
                            ));
                        }
                        crate::update::InstallOutcome::ManualFallback => {
                            st.toast(&crate::l10n::tr(st.language, "settings.updates.manual"));
                        }
                    },
                    Ok(None) => {}
                    Err(e) => st.toast(&crate::l10n::tr1(
                        st.language,
                        "settings.updates.failed",
                        &e,
                    )),
                }
            });
        });
    }
}

fn build_chrome(
    app: &adw::Application,
    state: &Rc<RefCell<AppState>>,
    ui: &UiHandles,
    model_rx: std::sync::mpsc::Receiver<WorkspaceMessage>,
) {
    let lang = state.borrow().language;

    // ── window chrome ──
    let window = adw::ApplicationWindow::new(app);
    window.set_default_size(1280, 800);
    a11y(&window, "pitex.workspace", "app.name");
    ui.window.replace(Some(window.clone()));

    // Vertical box mirrors ToolbarView's header+content layout; the plain
    // Box keeps the same visuals on both GTK variants. On `modern-gtk` the
    // header moves into a real `AdwToolbarView` wrapping the whole window —
    // the only way libadwaita 1.4+ draws window controls (AdwWindow rejects
    // `set_titlebar`), which is what gives Windows its min/max/close buttons.
    let toolbar_view = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    let header = adw::HeaderBar::new();

    // Navigation side: sidebar toggle.
    let sidebar_toggle = gtk4::Button::from_icon_name("sidebar-show-symbolic");
    sidebar_toggle.set_tooltip_text(Some(&tr(lang, "editor.show_sidebar")));
    a11y(&sidebar_toggle, "pitex.toolbar.sidebar", "editor.show_sidebar");
    {
        let state = state.clone();
        sidebar_toggle.connect_clicked(move |_| {
            let Ok(mut s) = state.try_borrow_mut() else { return };
            s.model.sidebar_visible = !s.model.sidebar_visible;
            s.refresh_console_visibility();
        });
    }
    header.pack_start(&sidebar_toggle);

    // Trailing controls: open, PDF toggle, assistant, settings, find.
    let open_btn = gtk4::Button::from_icon_name("folder-open-symbolic");
    open_btn.set_tooltip_text(Some(&tr(lang, "workspace.open")));
    a11y(&open_btn, "pitex.toolbar.open", "workspace.open");
    ui.open_button.replace(Some(open_btn.clone()));
    {
        let state = state.clone();
        open_btn.connect_clicked(move |b| {
            state.borrow().present_open(Some(b.upcast_ref()));
        });
    }
    header.pack_end(&open_btn);

    let pdf_toggle = gtk4::Button::from_icon_name("x-office-document-symbolic");
    pdf_toggle.set_tooltip_text(Some(&tr(lang, "preview.title")));
    a11y(&pdf_toggle, "pitex.toolbar.pdf", "preview.title");
    {
        let state = state.clone();
        pdf_toggle.connect_clicked(move |_| {
            let Ok(mut s) = state.try_borrow_mut() else { return };
            s.model.inspector_visible = !s.model.inspector_visible;
            s.refresh_console_visibility();
        });
    }
    header.pack_end(&pdf_toggle);

    let assistant_toggle = gtk4::Button::from_icon_name("starred-symbolic");
    assistant_toggle.set_tooltip_text(Some(&tr(lang, "assistant.title")));
    a11y(&assistant_toggle, "pitex.toolbar.assistant", "assistant.title");
    {
        let state = state.clone();
        assistant_toggle.connect_clicked(move |_| {
            let Ok(mut s) = state.try_borrow_mut() else { return };
            s.model.toggle_assistant();
            s.refresh_console_visibility();
            s.refresh_assistant();
        });
    }
    header.pack_end(&assistant_toggle);

    let settings_btn = gtk4::Button::from_icon_name("emblem-system-symbolic");
    settings_btn.set_tooltip_text(Some(&tr(lang, "command.settings")));
    a11y(&settings_btn, "pitex.settings", "command.settings");
    {
        let state = state.clone();
        let win = window.clone();
        settings_btn.connect_clicked(move |_| {
            crate::panes::show_settings(&state, &win.clone().upcast());
        });
    }
    header.pack_end(&settings_btn);

    // Primary menu — the macOS menubar's Edit/Build/View command surface.
    let primary = gio::Menu::new();
    let edit_section = gio::Menu::new();
    edit_section.append(
        Some(&tr(lang, "command.toggle_comment")),
        Some("win.comment"),
    );
    edit_section.append(
        Some(&tr(lang, "command.send_selection_ai")),
        Some("win.sendsel"),
    );
    primary.append_section(Some(&tr(lang, "command.project")), &edit_section);
    let build_section = gio::Menu::new();
    build_section.append(Some(&tr(lang, "command.build")), Some("win.build"));
    build_section.append(
        Some(&tr(lang, "command.cancel_build")),
        Some("win.cancelbuild"),
    );
    build_section.append(Some(&tr(lang, "command.clean")), Some("win.clean"));
    build_section.append(
        Some(&tr(lang, "command.run_custom")),
        Some("win.runcustom"),
    );
    build_section.append(
        Some(&tr(lang, "command.sync_forward")),
        Some("win.syncforward"),
    );
    primary.append_section(Some(&tr(lang, "command.build_menu")), &build_section);
    let view_section = gio::Menu::new();
    view_section.append(
        Some(&tr(lang, "command.toggle_assistant")),
        Some("win.assistant"),
    );
    view_section.append(
        Some(&tr(lang, "command.toggle_inspector")),
        Some("win.inspector"),
    );
    view_section.append(
        Some(&tr(lang, "command.toggle_bottom")),
        Some("win.bottompanel"),
    );
    view_section.append(
        Some(&tr(lang, "command.toggle_sidebar")),
        Some("win.sidebar"),
    );
    primary.append_section(Some(&tr(lang, "command.view")), &view_section);
    let tail_section = gio::Menu::new();
    tail_section.append(
        Some(&tr(lang, "command.settings")),
        Some("win.settings"),
    );
    primary.append_section(None, &tail_section);
    let primary_btn = gtk4::MenuButton::new();
    primary_btn.set_icon_name("open-menu-symbolic");
    primary_btn.set_menu_model(Some(&primary));
    primary_btn.set_tooltip_text(Some(&tr(lang, "command.view")));
    a11y(&primary_btn, "pitex.menu.primary", "command.view");
    header.pack_end(&primary_btn);

    let find_btn = gtk4::Button::from_icon_name("edit-find-symbolic");
    find_btn.set_tooltip_text(Some(&tr(lang, "editor.find")));
    a11y(&find_btn, "pitex.toolbar.find", "editor.find");
    {
        find_btn.connect_clicked(move |_| {
            UI.with(|ui| {
                if let Some(bar) = ui.search_bar.borrow().as_ref() {
                    bar.set_search_mode(!bar.is_search_mode());
                }
            });
        });
    }
    header.pack_end(&find_btn);
    #[cfg(not(feature = "modern-gtk"))]
    toolbar_view.append(&header);

    // ── central three-column layout ──
    let outer = gtk4::Paned::new(gtk4::Orientation::Horizontal);
    outer.set_resize_start_child(false);
    // Fixed minimum like the macOS sidebar (minWidth 170): the divider
    // stops at the sidebar's min width — shrinking further would allocate
    // below the min and GTK would shift+clip the content. The editor is
    // protected by inner.shrink_start_child(false) below.
    outer.set_shrink_start_child(false);
    ui.outer_paned.replace(Some(outer.clone()));

    let inner = gtk4::Paned::new(gtk4::Orientation::Horizontal);
    inner.set_resize_end_child(false);
    inner.set_shrink_end_child(false);
    // The editor column must never be squeezed below its minimum — the
    // preview absorbs the deficit instead (clipped, not overlapped).
    inner.set_shrink_start_child(false);
    ui.inner_paned.replace(Some(inner.clone()));

    // Sidebar.
    let sidebar = build_sidebar(state, ui);
    outer.set_start_child(Some(&sidebar));
    outer.set_end_child(Some(&inner));

    // Center: editor column over the console in a vertical pane.
    let console_paned = gtk4::Paned::new(gtk4::Orientation::Vertical);
    console_paned.set_resize_start_child(true);
    console_paned.set_shrink_end_child(false);
    ui.console_paned.replace(Some(console_paned.clone()));
    let editor_column = build_editor_column(state, ui);
    console_paned.set_start_child(Some(&editor_column));
    let console = crate::panes::build_console(state, ui);
    console_paned.set_end_child(Some(&console));
    console_paned.set_position(520);
    inner.set_start_child(Some(&console_paned));

    // Right inspector: PDF preview.
    let preview = crate::panes::build_preview_pane(state, ui);
    inner.set_end_child(Some(&preview));
    inner.set_position(900);
    outer.set_position(240);

    outer.set_vexpand(true);
    toolbar_view.append(&outer);

    // ── phase stack overlays the whole window ──
    let root_stack = gtk4::Stack::new();
    root_stack.add_named(&empty_page(state, ui), Some("empty"));
    root_stack.add_named(&loading_page(state), Some("loading"));
    root_stack.add_named(&error_page(state, ui), Some("error"));
    root_stack.add_named(&toolbar_view, Some("ready"));
    root_stack.set_visible_child_name("empty");
    ui.root_stack.replace(Some(root_stack.clone()));

    // Toast overlay wraps everything.
    let toast = adw::ToastOverlay::new();
    toast.set_child(Some(&root_stack));
    ui.toast_overlay.replace(Some(toast.clone()));
    #[cfg(feature = "modern-gtk")]
    {
        // The header is the window's top bar — window controls (min/max/
        // close on Windows, close/min/max per decoration layout on Linux)
        // appear on every stack page. Legacy GTK keeps the header inside
        // the ready page's box.
        let chrome = adw::ToolbarView::new();
        chrome.add_top_bar(&header);
        chrome.set_content(Some(&toast));
        window.set_content(Some(&chrome));
    }
    #[cfg(not(feature = "modern-gtk"))]
    window.set_content(Some(&toast));

    // ── keyboard shortcuts — the macOS `AppCommands` accelerator surface:
    // ⌘N new, ⌘O open, ⌘P pin, ⌘W close, ⌘S save, ⌘⇧S save-as, ⌘⌥S save-all,
    // ⌘/ comment, ⌘⇧A send-selection, ⌘B build, ⌘. cancel, ⌘⌃B custom,
    // ⌘⇧J sync, ⌘T assistant, ⌘⌥P inspector, ⌘⇧Y bottom panel. (⌘ maps to
    // Ctrl; macOS Control+Command pairs map to Ctrl+Alt here.)
    app.set_accels_for_action("win.newdoc", &["<Control>n"]);
    app.set_accels_for_action("win.open", &["<Control>o"]);
    app.set_accels_for_action("win.pintarget", &["<Control>p"]);
    app.set_accels_for_action("win.close", &["<Control>w"]);
    app.set_accels_for_action("win.save", &["<Control>s"]);
    app.set_accels_for_action("win.saveas", &["<Control><Shift>s"]);
    app.set_accels_for_action("win.saveall", &["<Control><Alt>s"]);
    app.set_accels_for_action("win.comment", &["<Control>slash"]);
    app.set_accels_for_action("win.sendsel", &["<Control><Shift>a"]);
    app.set_accels_for_action("win.build", &["<Control>b"]);
    app.set_accels_for_action("win.cancelbuild", &["<Control>period"]);
    app.set_accels_for_action("win.runcustom", &["<Control><Alt>b"]);
    app.set_accels_for_action("win.syncforward", &["<Control><Shift>j"]);
    app.set_accels_for_action("win.assistant", &["<Control>t"]);
    app.set_accels_for_action("win.inspector", &["<Control><Alt>p"]);
    app.set_accels_for_action("win.bottompanel", &["<Control><Shift>y"]);
    app.set_accels_for_action("win.find", &["<Control>f"]);
    app.set_accels_for_action("win.settings", &["<Control>comma"]);

    let save_action = gio::SimpleAction::new("save", None);
    {
        let state = state.clone();
        save_action.connect_activate(move |_, _| state.borrow_mut().save_action());
    }
    window.add_action(&save_action);
    ui.save_action.replace(Some(save_action.clone()));
    let build_action = gio::SimpleAction::new("build", None);
    {
        let state = state.clone();
        build_action.connect_activate(move |_, _| state.borrow_mut().toggle_build_action());
    }
    window.add_action(&build_action);
    let cancel_action = gio::SimpleAction::new("cancelbuild", None);
    {
        let state = state.clone();
        cancel_action.connect_activate(move |_, _| state.borrow_mut().cancel_build_action());
    }
    window.add_action(&cancel_action);
    let custom_action = gio::SimpleAction::new("runcustom", None);
    {
        let state = state.clone();
        custom_action.connect_activate(move |_, _| state.borrow_mut().run_custom_command_action());
    }
    window.add_action(&custom_action);
    let clean_action = gio::SimpleAction::new("clean", None);
    {
        let state = state.clone();
        clean_action.connect_activate(move |_, _| state.borrow_mut().clean_artifacts_action());
    }
    window.add_action(&clean_action);
    let comment_action = gio::SimpleAction::new("comment", None);
    {
        let state = state.clone();
        comment_action.connect_activate(move |_, _| state.borrow_mut().toggle_comment_action());
    }
    window.add_action(&comment_action);
    let sendsel_action = gio::SimpleAction::new("sendsel", None);
    {
        let state = state.clone();
        sendsel_action.connect_activate(move |_, _| state.borrow_mut().send_selection_action());
    }
    window.add_action(&sendsel_action);
    let pin_action = gio::SimpleAction::new("pintarget", None);
    {
        let state = state.clone();
        pin_action.connect_activate(move |_, _| state.borrow_mut().pin_target_action());
    }
    window.add_action(&pin_action);
    let close_action = gio::SimpleAction::new("close", None);
    {
        let state = state.clone();
        close_action.connect_activate(move |_, _| state.borrow_mut().close_action());
    }
    window.add_action(&close_action);
    let assistant_action = gio::SimpleAction::new("assistant", None);
    {
        let state = state.clone();
        assistant_action.connect_activate(move |_, _| state.borrow_mut().toggle_assistant_action());
    }
    window.add_action(&assistant_action);
    let inspector_action = gio::SimpleAction::new("inspector", None);
    {
        let state = state.clone();
        inspector_action.connect_activate(move |_, _| state.borrow_mut().toggle_inspector_action());
    }
    window.add_action(&inspector_action);
    let bottom_action = gio::SimpleAction::new("bottompanel", None);
    {
        let state = state.clone();
        bottom_action.connect_activate(move |_, _| state.borrow_mut().toggle_bottom_panel_action());
    }
    window.add_action(&bottom_action);
    let sidebar_action = gio::SimpleAction::new("sidebar", None);
    {
        let state = state.clone();
        sidebar_action.connect_activate(move |_, _| state.borrow_mut().toggle_sidebar_action());
    }
    window.add_action(&sidebar_action);
    let settings_action = gio::SimpleAction::new("settings", None);
    {
        let state = state.clone();
        let win = window.clone();
        settings_action.connect_activate(move |_, _| {
            crate::panes::show_settings(&state, &win.clone().upcast());
        });
    }
    window.add_action(&settings_action);
    let find_action = gio::SimpleAction::new("find", None);
    find_action.connect_activate(move |_, _| {
        UI.with(|ui| {
            if let Some(bar) = ui.search_bar.borrow().as_ref() {
                bar.set_search_mode(true);
            }
        });
    });
    window.add_action(&find_action);
    let sync_action = gio::SimpleAction::new("syncforward", None);
    {
        let state = state.clone();
        sync_action.connect_activate(move |_, _| state.borrow_mut().sync_forward_action());
    }
    window.add_action(&sync_action);
    // Open Recent — the menu item carries the recents index as an int target.
    // `win.openrecent(5)` parses `5` via g_variant_parse → int64 ("x").
    let openrecent_action = gio::SimpleAction::new(
        "openrecent",
        Some(glib::VariantTy::new("x").expect("variant ty")),
    );
    {
        let state = state.clone();
        openrecent_action.connect_activate(move |_, param| {
            if let Some(i) = param.and_then(|p| p.get::<i64>()) {
                state.borrow_mut().open_recent_action(i as i32);
            }
        });
    }
    window.add_action(&openrecent_action);
    let clearrecents_action = gio::SimpleAction::new("clearrecents", None);
    {
        let state = state.clone();
        clearrecents_action
            .connect_activate(move |_, _| state.borrow_mut().clear_recents_action());
    }
    window.add_action(&clearrecents_action);

    // ── dispatch loop ──
    {
        let state = state.clone();
        glib::timeout_add_local(Duration::from_millis(10), move || {
            while let Ok(msg) = model_rx.try_recv() {
                if let Ok(mut s) = state.try_borrow_mut() { s.dispatch(msg); }
            }
            glib::ControlFlow::Continue
        });
    }

    // Agent event polling — the Swift `AsyncStream` drain becomes a 40ms idle
    // tick plus an on-demand poll after each send.
    {
        let state = state.clone();
        glib::timeout_add_local(Duration::from_millis(40), move || {
            let Ok(mut s) = state.try_borrow_mut() else {
                return glib::ControlFlow::Continue;
            };
            if s.agent.is_some() {
                s.poll_agent();
            }
            glib::ControlFlow::Continue
        });
    }

    state.borrow_mut().apply_theme();
    state.borrow_mut().refresh_phase();
    window.present();

    // `applicationDidFinishLaunching` — the agent runtime self-installs on
    // first launch and refreshes its bundled skills every launch, then the
    // agent is prepared. The install runs off the main thread; the prepare
    // hop returns through an idle on the main context.
    std::thread::spawn(|| {
        let _ = crate::agent::pi_installer::ensure_installed();
        glib::idle_add_once(|| {
            STATE.with(|s| {
                if let Some(state) = s.borrow().as_ref() {
                    if let Ok(mut st) = state.try_borrow_mut() {
                        if st.agent.is_some() {
                            if let Some(agent) = st.agent.as_mut() {
                                agent.prepare();
                            }
                            st.refresh_assistant();
                        }
                    }
                }
            });
        });
    });
}

fn empty_page(state: &Rc<RefCell<AppState>>, ui: &UiHandles) -> gtk4::Widget {
    let lang = state.borrow().language;
    let page = adw::StatusPage::new();
    page.set_title(&tr(lang, "workspace.no_project"));
    page.set_description(Some(&tr(lang, "workspace.no_project_detail")));
    page.set_icon_name(Some("folder-open-symbolic"));
    a11y(&page, "pitex.noProject", "workspace.no_project");
    let button = gtk4::Button::with_label(&tr(lang, "workspace.open"));
    button.set_halign(gtk4::Align::Center);
    button.add_css_class("suggested-action");
    button.add_css_class("pill");
    a11y(&button, "pitex.open", "workspace.open");
    {
        let state = state.clone();
        button.connect_clicked(move |b| {
            state.borrow().present_open(Some(b.upcast_ref()));
        });
    }
    page.set_child(Some(&button));
    ui.status_page.replace(Some(page.clone()));
    page.upcast()
}

fn loading_page(state: &Rc<RefCell<AppState>>) -> gtk4::Widget {
    let lang = state.borrow().language;
    let page = adw::StatusPage::new();
    page.set_title(&tr(lang, "state.loading"));
    page.set_icon_name(Some("folder-open-symbolic"));
    let spinner = gtk4::Spinner::new();
    spinner.start();
    page.set_child(Some(&spinner));
    a11y(&page, "pitex.loading", "state.loading");
    page.upcast()
}

fn error_page(state: &Rc<RefCell<AppState>>, ui: &UiHandles) -> gtk4::Widget {
    let lang = state.borrow().language;
    let page = adw::StatusPage::new();
    page.set_title(&tr(lang, "error.title"));
    page.set_icon_name(Some("dialog-error-symbolic"));
    a11y(&page, "pitex.error", "error.title");
    let button = gtk4::Button::with_label(&tr(lang, "command.open"));
    button.set_halign(gtk4::Align::Center);
    {
        let state = state.clone();
        button.connect_clicked(move |b| {
            state.borrow().present_open(Some(b.upcast_ref()));
        });
    }
    page.set_child(Some(&button));
    let _ = ui; // status_page shared with empty state
    page.upcast()
}

fn build_sidebar(state: &Rc<RefCell<AppState>>, ui: &UiHandles) -> gtk4::Widget {
    let lang = state.borrow().language;
    let root = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    a11y(&root, "pitex.projectOutline", "sidebar.outline");
    root.set_size_request(170, -1); // macOS sidebar minWidth parity

    // Section picker (segmented → DropDown is the compact GTK equivalent).
    let section_items = [
        tr(lang, "sidebar.outline"),
        tr(lang, "sidebar.labels"),
        tr(lang, "sidebar.bibtex"),
    ];
    let section_strs: Vec<&str> = section_items.iter().map(String::as_str).collect();
    let sections = gtk4::DropDown::from_strings(&section_strs);
    sections.set_margin_start(6);
    sections.set_margin_end(6);
    sections.set_margin_top(6);
    sections.set_margin_bottom(6);
    a11y(&sections, "pitex.sidebar.section", "sidebar.outline");
    {
        let state = state.clone();
        sections.connect_selected_notify(move |dd| {
            let Ok(mut s) = state.try_borrow_mut() else { return };
            s.model.sidebar_section = match dd.selected() {
                1 => SidebarSection::Labels,
                2 => SidebarSection::BibTeX,
                _ => SidebarSection::Outline,
            };
            s.refresh_sidebar();
        });
    }
    ui.sidebar_section_dropdown.replace(Some(sections.clone()));
    root.append(&sections);
    root.append(&gtk4::Separator::new(gtk4::Orientation::Horizontal));

    // Section content stack.
    let stack = gtk4::Stack::new();
    stack.set_vexpand(true);
    for (name, handle) in [
        ("outline", &ui.outline_list),
        ("labels", &ui.labels_list),
        ("bibtex", &ui.bib_list),
    ] {
        let scroll = gtk4::ScrolledWindow::new();
        scroll.set_vexpand(true);
        let list = gtk4::ListBox::new();
        list.set_selection_mode(gtk4::SelectionMode::None);
        list.add_css_class("navigation-sidebar");
        a11y(
            &list,
            match name {
                "outline" => "pitex.sidebar.outline",
                "labels" => "pitex.sidebar.labels",
                _ => "pitex.sidebar.bibtex",
            },
            "sidebar.outline",
        );
        {
            let state = state.clone();
            let section = name.to_string();
            list.connect_row_activated(move |_, row| {
                let Ok(mut s) = state.try_borrow_mut() else { return };
                let name = row
                    .child()
                    .map(|c| c.widget_name().to_string())
                    .unwrap_or_default();
                let idx = name
                    .rsplit('-')
                    .next()
                    .and_then(|n| n.parse::<usize>().ok())
                    .unwrap_or(0);
                let target = match section.as_str() {
                    "outline" => s.model.outline_items.get(idx).map(|i| (i.line, 0usize)),
                    "labels" => s.model.label_items.get(idx).map(|i| (i.line, 0usize)),
                    _ => None,
                };
                if let Some((line, col)) = target {
                    s.jump_to(line.max(1), col, false);
                }
            });
        }
        scroll.set_child(Some(&list));
        handle.replace(Some(list));
        stack.add_named(&scroll, Some(name));
    }
    stack.set_visible_child_name("outline");
    ui.sidebar_stack.replace(Some(stack.clone()));
    root.append(&stack);
    root.append(&gtk4::Separator::new(gtk4::Orientation::Horizontal));

    // Project section — pinned at the bottom.
    let header = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
    header.set_margin_start(10);
    header.set_margin_end(6);
    header.set_margin_top(8);
    let title = gtk4::Label::new(Some(&tr(lang, "sidebar.project")));
    title.set_xalign(0.0);
    title.set_hexpand(true);
    title.add_css_class("caption");
    title.add_css_class("dim-label");
    header.append(&title);
    let rescan = gtk4::Button::from_icon_name("view-refresh-symbolic");
    rescan.add_css_class("flat");
    rescan.set_tooltip_text(Some(&tr(lang, "sidebar.rescan_help")));
    {
        let state = state.clone();
        rescan.connect_clicked(move |_| {
            let Ok(mut s) = state.try_borrow_mut() else { return };
            s.model.rescan_project();
            s.refresh_sidebar();
            s.install_watchers();
        });
    }
    header.append(&rescan);
    let pin = gtk4::Button::from_icon_name("emblem-important-symbolic");
    pin.add_css_class("flat");
    pin.set_tooltip_text(Some(&tr(lang, "sidebar.pin_help")));
    a11y(&pin, "pitex.sidebar.pin", "sidebar.pin_help");
    {
        let state = state.clone();
        pin.connect_clicked(move |_| {
            let Ok(mut s) = state.try_borrow_mut() else { return };
            s.model.toggle_pinned_build_target();
            s.refresh_sidebar();
            s.refresh_pdf_ui();
        });
    }
    ui.pin_button.replace(Some(pin.clone()));
    header.append(&pin);
    root.append(&header);

    let project_scroll = gtk4::ScrolledWindow::new();
    project_scroll.set_vexpand(true);
    project_scroll.set_min_content_height(140);
    let project_list = gtk4::ListBox::new();
    project_list.set_selection_mode(gtk4::SelectionMode::None);
    project_list.add_css_class("navigation-sidebar");
    project_scroll.set_child(Some(&project_list));
    ui.project_list.replace(Some(project_list));
    root.append(&project_scroll);
    root.upcast()
}

fn build_editor_column(state: &Rc<RefCell<AppState>>, ui: &UiHandles) -> gtk4::Widget {
    let lang = state.borrow().language;
    let root = gtk4::Box::new(gtk4::Orientation::Vertical, 0);

    // Tab strip + add/save/build/sync cluster.
    let strip = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
    strip.set_margin_start(6);
    strip.set_margin_end(6);
    let scroll = gtk4::ScrolledWindow::new();
    scroll.set_policy(gtk4::PolicyType::Automatic, gtk4::PolicyType::Never);
    scroll.set_hexpand(true);
    let tab_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 4);
    a11y(&tab_row, "pitex.tabs", "editor.title");
    scroll.set_child(Some(&tab_row));
    ui.tab_row.replace(Some(tab_row));
    strip.append(&scroll);

    // "+" menu: the File-menu equivalent — new / open / pin / close; the
    // Open Recent section is rebuilt in `refresh_phase` from the model.
    let add_menu = gio::Menu::new();
    add_menu.append(Some(&tr(lang, "command.new")), Some("win.newdoc"));
    add_menu.append(Some(&tr(lang, "command.open")), Some("win.open"));
    let add = gtk4::MenuButton::new();
    add.set_icon_name("list-add-symbolic");
    add.set_menu_model(Some(&add_menu));
    a11y(&add, "pitex.editor.add", "command.new");
    ui.add_menu_button.replace(Some(add.clone()));
    strip.append(&add);

    // Save menu: save / save as / save all.
    let save_menu = gio::Menu::new();
    save_menu.append(Some(&tr(lang, "editor.save")), Some("win.save"));
    save_menu.append(Some(&tr(lang, "command.save_as")), Some("win.saveas"));
    save_menu.append(Some(&tr(lang, "command.save_all")), Some("win.saveall"));
    let save = gtk4::MenuButton::new();
    save.set_icon_name("document-save-symbolic");
    save.set_menu_model(Some(&save_menu));
    a11y(&save, "pitex.toolbar.save", "editor.save");
    strip.append(&save);

    let build_btn = gtk4::Button::from_icon_name("media-playback-start-symbolic");
    build_btn.set_tooltip_text(Some(&tr(lang, "build.start")));
    a11y(&build_btn, "pitex.build", "build.start");
    {
        let state = state.clone();
        build_btn.connect_clicked(move |_| state.borrow_mut().toggle_build_action());
    }
    ui.header_build_button.replace(Some(build_btn.clone()));
    strip.append(&build_btn);

    root.append(&strip);

    // "Editor" header: caption + disk status + reload.
    let editor_header = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    editor_header.set_margin_start(10);
    editor_header.set_margin_end(10);
    let caption = gtk4::Label::new(Some(&tr(lang, "editor.title")));
    caption.set_xalign(0.0);
    caption.set_hexpand(true);
    caption.add_css_class("caption");
    caption.add_css_class("dim-label");
    editor_header.append(&caption);
    let disk = gtk4::Button::from_icon_name("emblem-synchronizing-symbolic");
    disk.add_css_class("flat");
    disk.set_tooltip_text(Some(&tr(lang, "editor.disk_unchanged")));
    a11y(&disk, "pitex.editor.diskStatus", "editor.disk_unchanged");
    {
        let state = state.clone();
        disk.connect_clicked(move |_| state.borrow_mut().reload_active());
    }
    ui.disk_status.replace(Some(disk.clone()));
    editor_header.append(&disk);
    let reload = gtk4::Button::from_icon_name("view-refresh-symbolic");
    reload.add_css_class("flat");
    reload.set_tooltip_text(Some(&tr(lang, "editor.reload")));
    a11y(&reload, "pitex.editor.reload", "editor.reload");
    {
        let state = state.clone();
        reload.connect_clicked(move |_| state.borrow_mut().reload_active());
    }
    editor_header.append(&reload);
    root.append(&editor_header);

    // Conflict banner.
    let banner = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    banner.set_margin_start(9);
    banner.set_margin_end(9);
    banner.set_margin_top(4);
    banner.set_margin_bottom(4);
    banner.add_css_class("warning");
    banner.set_visible(false);
    a11y(&banner, "pitex.conflict", "conflict.title");
    banner.append(&gtk4::Image::from_icon_name("dialog-warning-symbolic"));
    let banner_text = gtk4::Box::new(gtk4::Orientation::Vertical, 2);
    banner_text.set_hexpand(true);
    let banner_title = gtk4::Label::new(Some(&tr(lang, "conflict.title")));
    banner_title.set_xalign(0.0);
    let banner_msg = gtk4::Label::new(None);
    banner_msg.set_xalign(0.0);
    banner_msg.add_css_class("caption");
    banner_text.append(&banner_title);
    banner_text.append(&banner_msg);
    ui.conflict_label.replace(Some(banner_msg));
    banner.append(&banner_text);
    let use_disk = gtk4::Button::with_label(&tr(lang, "conflict.use_external"));
    a11y(&use_disk, "pitex.conflict.useExternal", "conflict.use_external");
    {
        let state = state.clone();
        use_disk.connect_clicked(move |_| state.borrow_mut().resolve_conflict(true));
    }
    banner.append(&use_disk);
    let keep_mine = gtk4::Button::with_label(&tr(lang, "conflict.keep_mine"));
    a11y(&keep_mine, "pitex.conflict.keepMine", "conflict.keep_mine");
    {
        let state = state.clone();
        keep_mine.connect_clicked(move |_| state.borrow_mut().resolve_conflict(false));
    }
    banner.append(&keep_mine);
    ui.conflict_banner.replace(Some(banner.clone()));
    root.append(&banner);

    // Custom-shell authority warning.
    let shell_warn = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    shell_warn.set_margin_start(10);
    shell_warn.set_margin_top(4);
    shell_warn.set_margin_bottom(4);
    shell_warn.add_css_class("warning");
    shell_warn.set_visible(false);
    a11y(&shell_warn, "pitex.shellWarning", "warning.custom_shell_authority");
    shell_warn.append(&gtk4::Image::from_icon_name("dialog-warning-symbolic"));
    let warn_label = gtk4::Label::new(Some(&tr(lang, "warning.custom_shell_authority")));
    warn_label.set_xalign(0.0);
    shell_warn.append(&warn_label);
    ui.shell_warning.replace(Some(shell_warn.clone()));
    root.append(&shell_warn);

    // Search bar (GtkSourceView SearchContext behind it). NSTextView's
    // find panel ships find + replace; the second row mirrors that.
    let search_bar = gtk4::SearchBar::new();
    let search_box = gtk4::Box::new(gtk4::Orientation::Vertical, 4);
    let find_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
    let entry = gtk4::SearchEntry::new();
    entry.set_hexpand(true);
    let prev = gtk4::Button::from_icon_name("go-up-symbolic");
    let next = gtk4::Button::from_icon_name("go-down-symbolic");
    find_row.append(&entry);
    find_row.append(&prev);
    find_row.append(&next);
    search_box.append(&find_row);
    let replace_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
    let replace_entry = gtk4::Entry::new();
    replace_entry.set_hexpand(true);
    replace_entry.set_placeholder_text(Some(&tr(lang, "editor.replace")));
    let replace_btn = gtk4::Button::with_label(&tr(lang, "editor.replace"));
    let replace_all_btn = gtk4::Button::with_label(&tr(lang, "editor.replace_all"));
    replace_row.append(&replace_entry);
    replace_row.append(&replace_btn);
    replace_row.append(&replace_all_btn);
    search_box.append(&replace_row);
    search_bar.set_child(Some(&search_box));
    search_bar.connect_entry(&entry);
    {
        let state = state.clone();
        entry.connect_search_changed(move |e| {
            if let Some(state) = STATE.with(|s| s.borrow().clone()) {
                search_step(&state, &e.text(), true);
            }
        });
        let e2 = entry.clone();
        prev.connect_clicked(move |_| {
            if let Some(state) = STATE.with(|s| s.borrow().clone()) {
                search_step(&state, &e2.text(), false);
            }
        });
        let e3 = entry.clone();
        next.connect_clicked(move |_| {
            if let Some(state) = STATE.with(|s| s.borrow().clone()) {
                search_step(&state, &e3.text(), true);
            }
        });
        let e4 = entry.clone();
        let r1 = replace_entry.clone();
        replace_btn.connect_clicked(move |_| {
            if let Some(state) = STATE.with(|s| s.borrow().clone()) {
                search_replace(&state, &e4.text(), &r1.text());
            }
        });
        let e5 = entry.clone();
        let r2 = replace_entry.clone();
        replace_all_btn.connect_clicked(move |_| {
            if let Some(state) = STATE.with(|s| s.borrow().clone()) {
                search_replace_all(&state, &e5.text(), &r2.text());
            }
        });
        let r3 = replace_entry.clone();
        entry.connect_stop_search(move |_| {
            r3.set_text("");
        });
        let _ = state;
    }
    ui.search_bar.replace(Some(search_bar.clone()));
    ui.search_entry.replace(Some(entry));
    root.append(&search_bar);

    // Editor stack: empty placeholder vs scroller(view) + minimap.
    let editor_stack = gtk4::Stack::new();
    editor_stack.set_vexpand(true);
    a11y(&editor_stack, "pitex.editor", "editor.title");
    let no_doc = adw::StatusPage::new();
    no_doc.set_title(&tr(lang, "editor.no_document"));
    no_doc.set_icon_name(Some("x-office-document-symbolic"));
    a11y(&no_doc, "pitex.editor.empty", "editor.no_document");
    editor_stack.add_named(&no_doc, Some("empty"));

    let editor_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
    let scroller = gtk4::ScrolledWindow::new();
    scroller.set_hexpand(true);
    scroller.set_vexpand(true);
    a11y(&scroller, "pitex.editor.scroll", "editor.title");
    ui.editor_scroller.replace(Some(scroller.clone()));
    // The fold chip layer overlays the scroller from outside — putting it
    // inside would break the view's scroll adjustments (minimap, jump-to).
    let editor_overlay = gtk4::Overlay::new();
    editor_overlay.set_child(Some(&scroller));
    ui.editor_overlay.replace(Some(editor_overlay.clone()));
    editor_row.append(&editor_overlay);
    let minimap = sourceview5::Map::new();
    minimap.set_visible(state.borrow().store.minimap());
    a11y(&minimap, "pitex.minimap", "editor.title");
    ui.minimap.replace(Some(minimap.clone()));
    editor_row.append(&minimap);
    editor_stack.add_named(&editor_row, Some("editor"));
    editor_stack.set_visible_child_name("empty");
    ui.editor_stack.replace(Some(editor_stack.clone()));
    root.append(&editor_stack);

    // Footer: panel toggles + path + word count.
    let footer = gtk4::Box::new(gtk4::Orientation::Horizontal, 10);
    footer.set_margin_start(10);
    footer.set_margin_end(10);
    footer.set_margin_top(2);
    footer.set_margin_bottom(2);
    a11y(&footer, "pitex.editor.status", "editor.title");
    let sidebar_btn = gtk4::Button::from_icon_name("sidebar-show-symbolic");
    sidebar_btn.add_css_class("flat");
    {
        let state = state.clone();
        sidebar_btn.connect_clicked(move |_| {
            let Ok(mut s) = state.try_borrow_mut() else { return };
            s.model.sidebar_visible = !s.model.sidebar_visible;
            s.refresh_console_visibility();
        });
    }
    footer.append(&sidebar_btn);
    let bottom_btn = gtk4::Button::from_icon_name("pan-down-symbolic");
    bottom_btn.add_css_class("flat");
    {
        let state = state.clone();
        bottom_btn.connect_clicked(move |_| {
            let Ok(mut s) = state.try_borrow_mut() else { return };
            s.model.bottom_panel_visible = !s.model.bottom_panel_visible;
            s.refresh_console_visibility();
        });
    }
    footer.append(&bottom_btn);
    let flip_btn = gtk4::Button::from_icon_name("object-flip-horizontal-symbolic");
    flip_btn.add_css_class("flat");
    flip_btn.set_tooltip_text(Some(&tr(lang, "editor.flip_panels")));
    {
        let state = state.clone();
        flip_btn.connect_clicked(move |_| {
            UI.with(|ui| {
                if let Some(paned) = ui.inner_paned.borrow().as_ref() {
                    let start = paned.start_child();
                    let end = paned.end_child();
                    paned.set_start_child(end.as_ref());
                    paned.set_end_child(start.as_ref());
                }
            });
            let _ = &state;
        });
    }
    footer.append(&flip_btn);
    let inspector_btn = gtk4::Button::from_icon_name("sidebar-show-right-symbolic");
    inspector_btn.add_css_class("flat");
    {
        let state = state.clone();
        inspector_btn.connect_clicked(move |_| {
            let Ok(mut s) = state.try_borrow_mut() else { return };
            s.model.inspector_visible = !s.model.inspector_visible;
            s.refresh_console_visibility();
        });
    }
    footer.append(&inspector_btn);
    let path_label = gtk4::Label::new(None);
    path_label.set_xalign(0.0);
    path_label.set_hexpand(true);
    path_label.set_ellipsize(gtk4::pango::EllipsizeMode::Start);
    path_label.add_css_class("caption");
    path_label.add_css_class("dim-label");
    ui.footer_path.replace(Some(path_label.clone()));
    footer.append(&path_label);
    let words = gtk4::Label::new(None);
    words.add_css_class("caption");
    words.add_css_class("dim-label");
    ui.footer_words.replace(Some(words.clone()));
    footer.append(&words);
    ui.footer.replace(Some(footer.clone()));
    root.append(&footer);

    // Window actions for menus.
    let state2 = state.clone();
    let newdoc = gio::SimpleAction::new("newdoc", None);
    newdoc.connect_activate(move |_, _| state2.borrow_mut().create_document_action());
    let state3 = state.clone();
    let open_action = gio::SimpleAction::new("open", None);
    open_action.connect_activate(move |_, _| state3.borrow().present_open(None));
    let state4 = state.clone();
    let saveas = gio::SimpleAction::new("saveas", None);
    saveas.connect_activate(move |_, _| state4.borrow().save_as_action());
    let state5 = state.clone();
    let saveall = gio::SimpleAction::new("saveall", None);
    saveall.connect_activate(move |_, _| state5.borrow_mut().save_all_action());
    UI.with(|ui| {
        if let Some(w) = ui.window.borrow().as_ref() {
            w.add_action(&newdoc);
            w.add_action(&open_action);
            w.add_action(&saveas);
            w.add_action(&saveall);
        }
        ui.save_as_action.replace(Some(saveas.clone()));
        ui.save_all_action.replace(Some(saveall.clone()));
    });

    root.upcast()
}

/// GtkSourceView `SearchContext` — parity with `performFindPanelAction`.
fn search_step(state: &Rc<RefCell<AppState>>, query: &str, forward: bool) {
    let s = state.borrow();
    let Some(editor) = &s.editor else { return };
    let settings = sourceview5::SearchSettings::new();
    settings.set_search_text(if query.is_empty() {
        None
    } else {
        Some(query)
    });
    settings.set_case_sensitive(false);
    settings.set_wrap_around(true);
    let context = sourceview5::SearchContext::new(editor.buffer(), Some(&settings));
    context.set_highlight(true);
    let (start, end) = editor.buffer().bounds();
    let _ = (start, end);
    let cursor = editor.buffer().iter_at_mark(&editor.buffer().get_insert());
    let found = if forward {
        context.forward(&cursor)
    } else {
        context.backward(&cursor)
    };
    if let Some((m_start, m_end, _wrapped)) = found {
        editor.buffer().select_range(&m_start, &m_end);
        let mut iter = m_start;
        editor.view().scroll_to_iter(&mut iter, 0.1, false, 0.0, 0.0);
    }
}

/// `SearchContext` helpers shared by the replace actions — NSTextView's
/// find panel behaviour: `Replace` swaps the current match then advances;
/// `Replace All` swaps every match in the buffer.
fn search_context(editor: &GtkEditorAdapter, query: &str) -> sourceview5::SearchContext {
    let settings = sourceview5::SearchSettings::new();
    settings.set_search_text(if query.is_empty() {
        None
    } else {
        Some(query)
    });
    settings.set_case_sensitive(false);
    settings.set_wrap_around(true);
    let context = sourceview5::SearchContext::new(editor.buffer(), Some(&settings));
    context.set_highlight(true);
    context
}

fn search_replace(state: &Rc<RefCell<AppState>>, query: &str, replacement: &str) {
    {
        let s = state.borrow();
        let Some(editor) = &s.editor else { return };
        let context = search_context(editor, query);
        let buffer = editor.buffer();
        if let Some((sel_start, sel_end)) = buffer.selection_bounds() {
            if let Some((mut m_start, mut m_end, _)) = context.forward(&sel_start) {
                if m_start.offset() == sel_start.offset() && m_end.offset() == sel_end.offset() {
                    let _ = context.replace(&mut m_start, &mut m_end, replacement);
                }
            }
        }
    }
    search_step(state, query, true);
}

fn search_replace_all(state: &Rc<RefCell<AppState>>, query: &str, replacement: &str) {
    let s = state.borrow();
    let Some(editor) = &s.editor else { return };
    let context = search_context(editor, query);
    let _ = context.replace_all(replacement);
}
