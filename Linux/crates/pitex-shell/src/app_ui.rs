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
use crate::ghost_completion::{CompletionContext, GhostCompletionCoordinator};
use crate::l10n::{resolve_language, tr, tr1};
use crate::model::{
    ConsoleSection, DocumentTodoItem, GitRefresh, SidebarSection, TodoLineEdit,
    WorkspaceBuildState, WorkspaceMessage, WorkspaceModel, WorkspacePhase, WorkspaceSyncTeXState,
};
use crate::pdf::{PdfInfo, PdfRenderer};
use crate::settings::{AppearanceColorRole, AppearanceSettings, Preferences, SettingsStore, Theme};
use git_core::GitChange;

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
    /// Overlay wrapping `editor_scroller`; hosts the fold chip layer and
    /// the ghost-completion label.
    pub editor_overlay: RefCell<Option<gtk4::Overlay>>,
    /// The translucent ghost-text label the completion coordinator
    /// repositions at the caret — created once in `build_editor_column`.
    pub ghost_label: RefCell<Option<gtk4::Label>>,
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
    pub todo_list: RefCell<Option<gtk4::ListBox>>,
    pub todo_add_button: RefCell<Option<gtk4::Button>>,
    pub project_list: RefCell<Option<gtk4::ListBox>>,
    /// Compiled-artifact strip under the project list — the dashed divider
    /// plus the same-stem PDF rows `extract_output_pdfs` pulled out.
    pub project_output_sep: RefCell<Option<gtk4::Widget>>,
    pub project_output_list: RefCell<Option<gtk4::ListBox>>,
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
    // Git Integration pane (`GitIntegrationView`).
    pub git_stack: RefCell<Option<gtk4::Stack>>,
    pub git_repo_name: RefCell<Option<gtk4::Label>>,
    pub git_branch_dropdown: RefCell<Option<gtk4::DropDown>>,
    pub git_branch_model: RefCell<Option<gtk4::StringList>>,
    pub git_ahead_behind: RefCell<Option<gtk4::Label>>,
    pub git_busy_spinner: RefCell<Option<gtk4::Spinner>>,
    pub git_error_label: RefCell<Option<gtk4::Label>>,
    pub git_changes_list: RefCell<Option<gtk4::ListView>>,
    pub(crate) git_changes_model: RefCell<Option<crate::git_list::GitList>>,
    pub(crate) git_rendered_commits: RefCell<Vec<git_core::GitCommit>>,
    pub git_graph_list: RefCell<Option<gtk4::ListBox>>,
    pub git_commit_view: RefCell<Option<gtk4::TextView>>,
    pub git_commit_button: RefCell<Option<gtk4::Button>>,
    /// The "Suggest" half of the commit box — child swaps to a Spinner
    /// while `git_suggest_busy`.
    pub git_suggest_button: RefCell<Option<gtk4::Button>>,
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
    /// Copilot-style inline completion — a dedicated pi subprocess that
    /// never touches the chat transcript. Rebound per session in
    /// `attach_session`, shut down with the workspace.
    pub completion: Rc<GhostCompletionCoordinator>,
    /// The fold chip layer currently overlaid on the editor scroller —
    /// tracked so `rebind_editor_widget` can remove the previous one.
    pub fold_chip: RefCell<Option<gtk4::DrawingArea>>,
    /// Last `AgentCoordinator::ui_revision` rendered by `refresh_assistant`
    /// — the transcript/picker rebuild is skipped while it matches.
    pub rendered_agent_revision: Cell<u64>,
    /// Last AI font size applied to the transcript CSS — changes force a
    /// rebuild even when `ui_revision` is unchanged.
    pub rendered_ai_font_size: Cell<f64>,
    rendered_transcript_keys: RefCell<Vec<u64>>,
    rendered_picker_key: Cell<u64>,
    rendered_sidebar_keys: Cell<[u64; 4]>,
    rendered_log: RefCell<String>,
    rendered_issues_key: Cell<u64>,
    build_ui_pending: Cell<bool>,
    git_panel_was_visible: Cell<bool>,
    pub(crate) git_refresh_pending: Cell<bool>,
    pub(crate) git_refresh_root: RefCell<Option<PathBuf>>,
    /// Last `agent_context_key` pushed into `context_cell` — event dispatch
    /// skips the clone-heavy rebuild while the key is unchanged.
    pub last_context_key: Cell<u64>,
    /// `AppEnvironment` bundle — the Linux platform ports (`files` feeds the
    /// capability lease like `capabilityBroker`, `workspace` opens externals).
    pub env: PlatformEnvironment,
    /// `CFBundleShortVersionString` equivalent — compared against release
    /// tags by the updater.
    pub app_version: String,
    pub active_session: Option<DocumentSession>,
    /// Debounce source for post-edit re-highlighting (120ms like Swift).
    pub highlight_pending: Cell<bool>,
    /// Debounce for the per-keystroke structure parse (outline/labels) —
    /// same 120ms cadence as `highlight_pending`.
    pub structure_pending: Cell<bool>,
    /// Last `structure_revision`/`files_revision` rendered by
    /// `refresh_sidebar` — the list/tree rebuilds are skipped while they
    /// match (this ran on every keystroke).
    pub rendered_structure_revision: Cell<u64>,
    pub rendered_files_revision: Cell<u64>,
    /// Last `refresh_tabs` identity key — tab chips rebuild only on a bump.
    pub rendered_tabs_key: Cell<u64>,
    /// Last `render_pdf_page` key — the raster skips while doc/page/scale match.
    pub rendered_pdf_key: Cell<u64>,
    displayed_pdf_key: Cell<u64>,
    pending_pdf_highlight: Option<(i64, f64, f64, f64, f64)>,
    /// Debounce for external-change coalescing (0.35s).
    pub disk_pending: RefCell<HashMap<PathBuf, glib::SourceId>>,
    pub watchers: Vec<gio::FileMonitor>,
    pub pdf: Option<PdfInfo>,
    pdf_renderer: PdfRenderer,
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
    /// Change awaiting the discard confirmation dialog (`discardTarget`).
    pub git_pending_discard: Option<GitChange>,
    /// Guards the branch dropdown's `selected` notify while a refresh
    /// re-splices the model — otherwise the programmatic selection would
    /// read as a user branch switch.
    pub git_branch_updating: Cell<bool>,
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
            completion: GhostCompletionCoordinator::new(),
            fold_chip: RefCell::new(None),
            rendered_agent_revision: Cell::new(0),
            rendered_ai_font_size: Cell::new(0.0),
            rendered_transcript_keys: RefCell::new(Vec::new()),
            rendered_picker_key: Cell::new(0),
            rendered_sidebar_keys: Cell::new([0; 4]),
            rendered_log: RefCell::new(String::new()),
            rendered_issues_key: Cell::new(0),
            build_ui_pending: Cell::new(false),
            git_panel_was_visible: Cell::new(false),
            git_refresh_pending: Cell::new(false),
            git_refresh_root: RefCell::new(None),
            last_context_key: Cell::new(0),
            env: PlatformEnvironment::make("dev.pitex.app"),
            app_version,
            active_session: None,
            highlight_pending: Cell::new(false),
            structure_pending: Cell::new(false),
            rendered_structure_revision: Cell::new(0),
            rendered_pdf_key: Cell::new(0),
            displayed_pdf_key: Cell::new(0),
            pending_pdf_highlight: None,
            rendered_files_revision: Cell::new(0),
            rendered_tabs_key: Cell::new(0),
            disk_pending: RefCell::new(HashMap::new()),
            watchers: Vec::new(),
            pdf: None,
            pdf_renderer: PdfRenderer::new(tx.clone()),
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
            git_pending_discard: None,
            git_branch_updating: Cell::new(false),
        };
        state.wire_model_callbacks();
        // `completion.contextProvider` — reads live workspace state
        // through `STATE` at fire time, so the setting toggle and document
        // switches apply immediately without re-attaching.
        state.completion.set_context_provider(Box::new(|| {
            STATE.with(|s| {
                s.borrow()
                    .as_ref()
                    .and_then(|state| {
                        state
                            .try_borrow()
                            .ok()
                            .map(|st| st.completion_context())
                    })
                    .unwrap_or_default()
            })
        }));
        state
    }

    /// Re-read imported preferences into the live stores after a settings
    /// backup import — `store`'s encoded blob plus every appearance value.
    /// (On `self`, not the call site: `RefMut`'s deref_mut holds `s` while
    /// `store`/`appearance` split cleanly only through `&mut self`.)
    pub fn reload_imported_settings(&mut self) {
        self.store.reload();
        self.appearance.reload(self.store.prefs());
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

    /// `completionContext()` — the fire-time gates for inline completion:
    /// setting toggle, .tex extension, project root for the subprocess
    /// cwd, and the file name that lands in the prompt header.
    fn completion_context(&self) -> CompletionContext {
        let mut context = CompletionContext {
            project_root: self.model.project_url.clone(),
            enabled: self.store.ai_autocompletion(),
            is_tex: self
                .model
                .active_document_url
                .as_ref()
                .and_then(|u| u.extension())
                .map(|e| e.eq_ignore_ascii_case("tex"))
                .unwrap_or(false),
            editable: self
                .editor
                .as_ref()
                .map(|e| e.view().is_editable())
                .unwrap_or(false),
            ..Default::default()
        };
        context.file_name = self
            .model
            .document_snapshot
            .as_ref()
            .map(|s| s.path.to_string())
            .or_else(|| {
                self.model
                    .active_document_url
                    .as_ref()
                    .and_then(|u| u.file_name())
                    .map(|n| n.to_string_lossy().into_owned())
            })
            .unwrap_or_else(|| "document.tex".into());
        context
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
        // Unset role colors default to the effective mode's palette, so the
        // scheme must be in place before `stored_hex` reads below.
        self.appearance.resolved_dark.set(manager.is_dark());
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
        // The ghost label tracks the editor font so suggestion text and
        // document text share family and size.
        self.completion.refresh_font(&self.editor_font_desc());
    }

    /// `rehighlight()` — re-tokenize and push decorations + structure.
    pub fn rehighlight(&self) {
        let Some(editor) = &self.editor else { return };
        let Some(snapshot) = &self.model.document_snapshot else { return };
        let dialect = dialect_for(self.model.active_document_url.as_deref());
        let tokens = DeterministicTeXLexer::tokenize(&snapshot.text, dialect);
        if let Some(fold) = &self.fold { fold.recompute_with_tokens(&snapshot.text, &tokens); }
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
    /// through `schedule_rehighlight`/`schedule_autosave` so the
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
                    DocumentMutation::ReplaceRange {
                        utf16_offset: mutation.range.location,
                        utf16_length: mutation.range.length,
                        text: mutation.replacement.clone(),
                    },
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
        // Structure parse is debounced — it re-parses the whole document
        // and ran on every keystroke.
        self.schedule_structure_refresh();
        self.refresh_after_document_change();
        self.model.sync_selection_attachment();
        self.schedule_autosave();
        self.schedule_rehighlight();
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
                    st.refresh_git();
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
        compat::unhide_pointer_on_typing(adapter.view());
        // Completion provider — the GTK counterpart of the mac adapter's
        // `completionSource`: candidates come from the model through STATE
        // so the provider itself stays data-agnostic.
        let provider = crate::completion::TexCompletionProvider::new(
            &tr(self.language, "editor.completion"),
            Rc::new(|context: &language_core::CompletionContext| {
                STATE.with(|s| {
                    s.borrow()
                        .as_ref()
                        .and_then(|state| {
                            state
                                .try_borrow()
                                .ok()
                                .map(|st| st.model.editor_completions(context))
                        })
                        .unwrap_or_default()
                })
            }),
        );
        adapter.view().completion().add_provider(&provider);
        // FoldEngine attaches before the widget rebind so its chip layer
        // lands on the editor overlay, mirroring FoldEngine.attach.
        let dialect = dialect_for(self.model.active_document_url.as_deref());
        let fold = crate::fold::FoldEngine::attach(
            adapter.view(),
            dialect,
            self.store.code_folding(),
        );
        self.editor = Some(adapter.clone());
        self.fold = Some(fold);
        // Ghost completion rebinds to the fresh view/buffer like
        // FoldEngine — `completion.attach(to:)` in the Swift workspace.
        let ghost_label = UI.with(|ui| ui.ghost_label.borrow().clone());
        if let Some(ghost_label) = ghost_label {
            self.completion
                .attach(adapter.view(), adapter.buffer(), &ghost_label);
        }
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

    /// Coalesce typing bursts through the already-borrowed AppState. Trying
    /// to borrow STATE again here always failed during snapshot submission,
    /// leaving the pending flag unset and queuing one full pass per edit.
    fn schedule_rehighlight(&self) {
        if self.highlight_pending.replace(true) {
            return;
        }
        glib::timeout_add_local_once(Duration::from_millis(120), || {
            STATE.with(|s| {
                if let Some(state) = s.borrow().as_ref() {
                    if let Ok(st) = state.try_borrow() {
                        st.highlight_pending.set(false);
                        st.rehighlight();
                    }
                }
            });
        });
    }

    /// Debounced structure refresh — coalesces per-keystroke calls into one
    /// parse+sidebar rebuild 120ms after the last edit.
    fn schedule_structure_refresh(&self) {
        if self.structure_pending.replace(true) {
            return;
        }
        glib::timeout_add_local_once(Duration::from_millis(120), || {
            STATE.with(|s| {
                if let Some(state) = s.borrow().as_ref() {
                    if let Ok(mut st) = state.try_borrow_mut() {
                        st.structure_pending.set(false);
                        st.model.refresh_structure();
                        st.refresh_sidebar();
                        st.refresh_footer();
                    }
                }
            });
        });
    }

    // ── command actions ────────────────────────────────────────────────────

    /// `close()`'s agent half — terminate the coordinator and its subprocess
    /// with the project; a fresh one is built on the next open.
    fn shutdown_agent(&mut self) {
        // `completion?.shutdown()` — the dedicated subprocess dies with the
        // workspace and `attach` respawns it on the next document.
        self.completion.shutdown();
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
        self.refresh_git();
    }

    pub fn save_all_action(&mut self) {
        if let Some(err) = self.model.persist_dirty_sessions() {
            self.toast(&err);
        }
        self.refresh_after_document_change();
        self.refresh_git();
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

    /// `openTodo(_:)` — the model parks cross-file jumps in `pending_jump`
    /// (replayed by `apply_activate`); a same-file jump publishes through
    /// `on_jump_to`, which `drain_side_effects` lands immediately here.
    pub fn open_todo(&mut self, item: &DocumentTodoItem) {
        if let Some(tx) = self.tx.clone() {
            self.model.open_todo(item, tx);
        }
        self.drain_side_effects();
    }

    /// TODOs `+` — `addTodo()`: inserts `% TODO: ` at the caret when the
    /// active document is .tex (the buffer's change hook submits the edit
    /// through the session, like `insertText`), otherwise appends it to the
    /// build target on disk — guarded against dirty sessions by the model.
    pub fn add_todo_action(&mut self) {
        let active_is_tex = self
            .model
            .active_document_url
            .as_ref()
            .and_then(|u| u.extension())
            .map(|e| e.eq_ignore_ascii_case("tex"))
            .unwrap_or(false);
        if active_is_tex {
            if let Some(editor) = &self.editor {
                let buffer = editor.buffer().clone();
                buffer.begin_user_action();
                buffer.insert_at_cursor("% TODO: ");
                buffer.end_user_action();
            }
            self.refresh_after_document_change();
        } else {
            self.model.append_todo_to_target();
            self.model.refresh_structure();
            self.refresh_sidebar();
        }
    }

    /// `renameTodo`/`removeTodo`/`toggleTodo` — active-document edits come
    /// back as a (UTF-16 range, replacement) applied through the buffer as
    /// a user action (`toggle_comment_action` path); other files are written
    /// by the model on disk unless a dirty session owns them.
    pub fn todo_edit_action(&mut self, item: &DocumentTodoItem, edit: TodoLineEdit) {
        if let Some((range, replacement)) = self.model.edit_todo(item, edit) {
            if let Some(editor) = self.editor.clone() {
                let text = editor.text();
                let start_char =
                    gtk_editor_adapter::utf16_offset_to_char_offset(&text, range.start);
                let end_char =
                    gtk_editor_adapter::utf16_offset_to_char_offset(&text, range.end);
                let buffer = editor.buffer().clone();
                buffer.begin_user_action();
                let mut start = buffer.iter_at_offset(start_char as i32);
                let mut end = buffer.iter_at_offset(end_char as i32);
                buffer.delete(&mut start, &mut end);
                if !replacement.is_empty() {
                    let mut at = buffer.iter_at_offset(start_char as i32);
                    buffer.insert(&mut at, &replacement);
                }
                buffer.end_user_action();
            }
            self.refresh_after_document_change();
        } else {
            // Disk write (or a guarded no-op) — rescan so the row repaints.
            self.model.refresh_structure();
            self.refresh_sidebar();
        }
    }

    /// Figure rows open in the system viewer — the GTK half of
    /// `NSWorkspace.shared.open(url)`. GTK 4.6 has no `FileLauncher`, so the
    /// launch goes through `gio::AppInfo`; failures get a window-modal
    /// error dialog (`MessageDialog` — `AlertDialog` needs GTK 4.10).
    #[allow(deprecated)]
    pub fn open_external(&self, url: &Path) {
        let uri = gio::File::for_path(url).uri();
        if let Err(e) =
            gio::AppInfo::launch_default_for_uri(&uri, gio::AppLaunchContext::NONE)
        {
            let window = UI.with(|ui| ui.window.borrow().clone());
            let dialog = gtk4::MessageDialog::new(
                window.as_ref().map(|w| w.upcast_ref::<gtk4::Window>()),
                gtk4::DialogFlags::MODAL,
                gtk4::MessageType::Error,
                gtk4::ButtonsType::Close,
                &e.to_string(),
            );
            dialog.connect_response(|d, _| d.close());
            dialog.present();
        }
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
        if self.displayed_pdf_key.get() != self.rendered_pdf_key.get() { return; }
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
        // Skip the raster when nothing changed — `refresh_pdf_ui` calls
        // this on every keystroke.
        let key = {
            use std::hash::{Hash, Hasher};
            let mut h = std::collections::hash_map::DefaultHasher::new();
            self.pdf_hash.hash(&mut h);
            self.pdf_page.hash(&mut h);
            scale.to_bits().hash(&mut h);
            h.finish()
        };
        if key == self.rendered_pdf_key.get() {
            return;
        }
        self.rendered_pdf_key.set(key);
        self.pdf_renderer.render(self.pdf_hash, key, self.pdf_page, scale);
        UI.with(|ui| {
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
        self.pending_pdf_highlight = None;
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
        if self.displayed_pdf_key.get() != self.rendered_pdf_key.get() {
            self.pending_pdf_highlight = Some((page, x, y, w, h));
            return;
        }
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
        let key = self.model.agent_context_key();
        if self.agent.is_none() || self.last_context_key.get() != key {
            *self.context_cell.borrow_mut() = self.model.agent_context();
            self.last_context_key.set(key);
            self.model.sync_selection_attachment();
        }
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
                    ConsoleSection::Git => "git",
                    ConsoleSection::Terminal => "terminal",
                    ConsoleSection::Issues => "issues",
                    ConsoleSection::Log => "log",
                };
                stack.set_visible_child_name(name);
            }
            if let Some(dd) = ui.console_section_dropdown.borrow().as_ref() {
                let idx = match self.model.console_section {
                    ConsoleSection::Assistant => 0,
                    ConsoleSection::Git => 1,
                    ConsoleSection::Issues => 2,
                    ConsoleSection::Terminal => 3,
                    ConsoleSection::Log => 4,
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
        let visible = self.model.console_section == ConsoleSection::Git && self.model.bottom_panel_visible;
        if visible && !self.git_panel_was_visible.replace(visible) { self.refresh_git(); }
        if !visible { self.git_panel_was_visible.set(false); }
        self.refresh_git_panel();
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
        // Cheap identity key — rebuilding every tab chip per keystroke was
        // O(tabs) widget churn on each `refresh_after_document_change`.
        let key = {
            use std::hash::{Hash, Hasher};
            let mut h = std::collections::hash_map::DefaultHasher::new();
            self.model.open_documents.hash(&mut h);
            self.model.active_document_url.hash(&mut h);
            self.model
                .document_snapshot
                .as_ref()
                .map(|s| s.save_state != DocumentSaveState::Clean)
                .hash(&mut h);
            h.finish()
        };
        if key == self.rendered_tabs_key.get() {
            return;
        }
        self.rendered_tabs_key.set(key);
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
                compat::initial_tooltip(&close, &tr(self.language, "editor.close"));
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
            let structure_dirty =
                self.model.structure_revision != self.rendered_structure_revision.get();
            let files_dirty =
                self.model.files_revision != self.rendered_files_revision.get();
            if structure_dirty {
                self.rendered_structure_revision.set(self.model.structure_revision);
            }
            let mut changed = [false; 4];
            if structure_dirty {
                let keys = [
                    view_key(&(lang, &self.model.outline_items)),
                    view_key(&(lang, &self.model.label_items)),
                    view_key(&(lang, &self.model.bibliography_items)),
                    view_key(&(lang, &self.model.todo_items)),
                ];
                let previous = self.rendered_sidebar_keys.replace(keys);
                for i in 0..4 { changed[i] = keys[i] != previous[i]; }
            }
            if files_dirty {
                self.rendered_files_revision.set(self.model.files_revision);
            }
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
            if changed[0] {
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
            }
            if changed[1] {
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
            }
            if changed[2] {
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
            }
            if changed[3] {
            if let Some(list) = ui.todo_list.borrow().as_ref() {
                clear_list(list);
                for (i, item) in self.model.todo_items.iter().enumerate() {
                    append_todo_row(list, i, item, lang);
                }
                if self.model.todo_items.is_empty() {
                    let row = gtk4::Label::new(Some(&tr(lang, "sidebar.no_todos")));
                    row.add_css_class("dim-label");
                    row.set_margin_top(20);
                    list.append(&row);
                }
            }
            }
            if let Some(add) = ui.todo_add_button.borrow().as_ref() {
                add.set_sensitive(self.model.can_add_todo());
            }
            if files_dirty {
            // Project file tree — always visible at the bottom. Built from
            // relative paths through the shared `project-feature` builder
            // so directories nest like the macOS project tree.
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
                    &main_rel.clone().unwrap_or_default(),
                    &child_rels,
                );
                // Same-stem PDFs are build artifacts — pinned under the
                // tree behind the dashed divider (`outputPDFs` in the
                // SwiftUI sidebar).
                let main_stem = main_rel
                    .as_deref()
                    .and_then(|p| Path::new(p).file_stem())
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_default();
                let (tree, outputs) = project_feature::extract_output_pdfs(tree, &main_stem);
                for node in &tree {
                    append_project_node(list, node, 0, &root, &self.model);
                }
                if let (Some(sep), Some(output_list)) = (
                    ui.project_output_sep.borrow().as_ref(),
                    ui.project_output_list.borrow().as_ref(),
                ) {
                    clear_list(output_list);
                    let has_outputs = !outputs.is_empty();
                    sep.set_visible(has_outputs);
                    output_list.set_visible(has_outputs);
                    for node in &outputs {
                        append_project_node(output_list, node, 0, &root, &self.model);
                    }
                }
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
        let key = view_key(&(self.language, &self.model.build_issues));
        if self.rendered_issues_key.replace(key) == key { return; }
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

    /// `GitIntegrationView` body — repopulates the pane from `model.git_*`.
    pub fn refresh_git_panel(&self) {
        if self.model.console_section != ConsoleSection::Git || !self.model.bottom_panel_visible { return; }
        let lang = self.language;
        let busy = self.model.git_busy;
        UI.with(|ui| {
            if let Some(stack) = ui.git_stack.borrow().as_ref() {
                stack.set_visible_child_name(if self.model.git_status.is_some() {
                    "repo"
                } else {
                    "empty"
                });
            }
            if let Some(spinner) = ui.git_busy_spinner.borrow().as_ref() {
                spinner.set_visible(busy);
                if busy {
                    spinner.start();
                } else {
                    spinner.stop();
                }
            }
            if let Some(label) = ui.git_error_label.borrow().as_ref() {
                if let Some(error) = &self.model.git_error {
                    label.set_label(error);
                    label.set_visible(true);
                } else {
                    label.set_visible(false);
                }
            }
            if let Some(model) = ui.git_changes_model.borrow().as_ref() {
                model.set_status(self.model.git_status.clone());
            }
            let Some(status) = self.model.git_status.as_ref() else { return; };
            if let Some(label) = ui.git_repo_name.borrow().as_ref() {
                label.set_label(&status.repo_name);
            }
            if let Some(label) = ui.git_ahead_behind.borrow().as_ref() {
                label.set_visible(status.ahead > 0 || status.behind > 0);
                label.set_label(&format!("↑{} ↓{}", status.ahead, status.behind));
            }
            // Branch dropdown — re-splice under the guard so the selected
            // notify does not read as a user branch switch.
            if let (Some(dd), Some(model)) = (
                ui.git_branch_dropdown.borrow().as_ref().cloned(),
                ui.git_branch_model.borrow().as_ref().cloned(),
            ) {
                self.git_branch_updating.set(true);
                let items = &self.model.git_branches;
                let strings: Vec<&str> = items.iter().map(String::as_str).collect();
                if model.n_items() as usize != items.len() || items.iter().enumerate().any(|(i, branch)| model.string(i as u32).as_deref() != Some(branch.as_str())) {
                    model.splice(0, model.n_items(), &strings);
                }
                if let Some(idx) = items.iter().position(|b| *b == status.branch) {
                    dd.set_selected(idx as u32);
                }
                self.git_branch_updating.set(false);
            }
            if let Some(list) = ui.git_graph_list.borrow().as_ref() {
                if *ui.git_rendered_commits.borrow() == self.model.git_commits && list.first_child().is_some() { return; }
                *ui.git_rendered_commits.borrow_mut() = self.model.git_commits.clone();
                clear_list(list);
                if self.model.git_commits.is_empty() {
                    let row = gtk4::Label::new(Some(&tr(lang, "git.no_commits")));
                    row.add_css_class("dim-label");
                    row.set_margin_top(12);
                    list.append(&row);
                } else {
                    for commit in &self.model.git_commits {
                        list.append(&crate::panes::git_commit_row(commit, lang));
                    }
                }
            }
        });
        self.refresh_git_commit_button();
    }

    /// Commit button sensitivity — separate so the buffer `changed` hook
    /// does not rebuild the lists on every keystroke.
    pub(crate) fn refresh_git_commit_button(&self) {
        UI.with(|ui| {
            let no_changes = self
                .model
                .git_status
                .as_ref()
                .map(|s| s.staged.is_empty() && s.unstaged.is_empty())
                .unwrap_or(true);
            if let Some(button) = ui.git_commit_button.borrow().as_ref() {
                let empty_message = self.model.git_commit_message.trim().is_empty();
                button.set_sensitive(!empty_message && !self.model.git_busy && !no_changes);
            }
            if let Some(button) = ui.git_suggest_button.borrow().as_ref() {
                button.set_sensitive(
                    !self.model.git_busy && !self.model.git_suggest_busy && !no_changes,
                );
                if self.model.git_suggest_busy {
                    let spinner = gtk4::Spinner::new();
                    spinner.start();
                    button.set_child(Some(&spinner));
                } else {
                    button.set_label(&tr(LANG.get(), "git.suggest"));
                }
            }
        });
    }

    /// VSCode discard confirmation — `MessageDialog` (AlertDialog needs
    /// GTK 4.10); destructive styling on the Discard button.
    #[allow(deprecated)]
    pub fn git_discard_dialog(&mut self) {
        let Some(change) = self.git_pending_discard.clone() else {
            return;
        };
        let lang = self.language;
        let window = UI.with(|ui| ui.window.borrow().clone());
        let key = if change.kind == git_core::GitChangeKind::Untracked {
            "git.discard_confirm_untracked"
        } else {
            "git.discard_confirm"
        };
        let dialog = gtk4::MessageDialog::new(
            window.as_ref().map(|w| w.upcast_ref::<gtk4::Window>()),
            gtk4::DialogFlags::MODAL,
            gtk4::MessageType::Warning,
            gtk4::ButtonsType::None,
            &tr1(lang, key, &change.path),
        );
        dialog.add_button(&tr(lang, "git.cancel"), gtk4::ResponseType::Cancel);
        dialog
            .add_button(&tr(lang, "git.discard"), gtk4::ResponseType::Accept)
            .add_css_class("destructive-action");
        dialog.connect_response(|d, response| {
            if response == gtk4::ResponseType::Accept {
                STATE.with(|s| {
                    if let Some(state) = s.borrow().as_ref() {
                        if let Ok(mut st) = state.try_borrow_mut() {
                            if let Some(change) = st.git_pending_discard.take() {
                                st.git_discard(&change);
                            }
                        }
                    }
                });
            }
            d.close();
        });
        dialog.present();
    }

    /// New-branch prompt — `MessageDialog` with an entry in its message
    /// area (the compat story for `AlertDialog` + text field). `at` pins
    /// the branch's start point — the graph context menu's "New Branch…"
    /// branches off the selected commit (VSCode parity).
    #[allow(deprecated)]
    pub fn git_new_branch_dialog(&mut self, at: Option<String>) {
        let lang = self.language;
        let window = UI.with(|ui| ui.window.borrow().clone());
        let dialog = gtk4::MessageDialog::new(
            window.as_ref().map(|w| w.upcast_ref::<gtk4::Window>()),
            gtk4::DialogFlags::MODAL,
            gtk4::MessageType::Question,
            gtk4::ButtonsType::None,
            &tr(lang, "git.branch_new"),
        );
        let entry = gtk4::Entry::new();
        entry.set_placeholder_text(Some(&tr(lang, "git.branch_name")));
        entry.set_activates_default(true);
        dialog
            .message_area()
            .downcast::<gtk4::Box>()
            .unwrap()
            .append(&entry);
        dialog.add_button(&tr(lang, "git.cancel"), gtk4::ResponseType::Cancel);
        dialog
            .add_button(&tr(lang, "git.create"), gtk4::ResponseType::Accept)
            .add_css_class("suggested-action");
        dialog.set_default_response(gtk4::ResponseType::Accept);
        dialog.connect_response(move |d, response| {
            if response == gtk4::ResponseType::Accept {
                let name = entry.text().to_string();
                let at = at.clone();
                STATE.with(|s| {
                    if let Some(state) = s.borrow().as_ref() {
                        if let Ok(mut st) = state.try_borrow_mut() {
                            st.git_create_branch(&name, at.as_deref());
                        }
                    }
                });
            }
            d.close();
        });
        dialog.present();
    }

    /// The graph context menu's "Open Changes" — `git show` on a worker,
    /// then a read-only scrolled window with the raw patch (the Linux
    /// panel has no side-by-side diff surface like macOS).
    pub fn git_open_commit_diff(&self, commit: &git_core::GitCommit) {
        let Some(root) = self.model.git_status.as_ref().map(|s| s.root.clone()) else {
            return;
        };
        let title = format!("{} — {}", commit.hash, commit.subject);
        let hash = commit.full_hash.clone();
        std::thread::spawn(move || {
            let result = crate::git::run_git(
                Path::new(&root),
                git_core::show_commit_args(&hash),
            );
            glib::idle_add_once(move || {
                let text = result.unwrap_or_else(|e| e);
                let window = UI.with(|ui| ui.window.borrow().clone());
                let dialog = gtk4::Window::new();
                dialog.set_title(Some(&title));
                dialog.set_transient_for(window.as_ref().map(|w| w.upcast_ref::<gtk4::Window>()));
                dialog.set_default_size(880, 560);
                dialog.set_titlebar(Some(&gtk4::HeaderBar::new()));
                let scroll = gtk4::ScrolledWindow::new();
                let view = gtk4::TextView::new();
                view.set_editable(false);
                view.set_monospace(true);
                view.set_top_margin(8);
                view.set_bottom_margin(8);
                view.set_left_margin(10);
                view.set_right_margin(10);
                view.buffer().set_text(&text);
                scroll.set_child(Some(&view));
                dialog.set_child(Some(&scroll));
                dialog.present();
            });
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
                let mut previous = self.rendered_log.borrow_mut();
                if *previous != text {
                    if let Some(added) = text.strip_prefix(previous.as_str()) {
                        buffer.insert(&mut buffer.end_iter(), added);
                    } else {
                        buffer.set_text(&text);
                    }
                    *previous = text;
                    let mut end = buffer.end_iter();
                    view.scroll_to_iter(&mut end, 0.0, false, 0.0, 1.0);
                }
            }
        });
        self.refresh_issues();
    }

    // ── refresh: PDF / synctex ─────────────────────────────────────────────

    pub fn refresh_pdf_ui(&mut self) {
        // Hash computed once at `Succeeded` construction — hashing the PDF
        // bytes per refresh (every keystroke) was an O(pdf) pass.
        let pdf_hash = match &self.model.build_state {
            WorkspaceBuildState::Succeeded { hash, .. } => Some(*hash),
            _ => None,
        };
        match (pdf_hash, &self.pdf) {
            (Some(hash), _) => {
                let WorkspaceBuildState::Succeeded { pdf: data, .. } =
                    &self.model.build_state
                else {
                    return;
                };
                if self.pdf_hash != hash {
                    self.pdf = None;
                    self.pdf_hash = hash;
                    self.rendered_pdf_key.set(0);
                    self.pdf_renderer.load(hash, data.clone());
                    UI.with(|ui| {
                        if let Some(picture) = ui.pdf_picture.borrow().as_ref() {
                            picture.set_paintable(None::<&gtk4::gdk::Paintable>);
                        }
                    });
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
                if self.pdf_hash != 0 {
                    self.pdf = None;
                    self.pdf_hash = 0;
                    self.rendered_pdf_key.set(0);
                    self.pdf_renderer.load(0, std::sync::Arc::from([]));
                }
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
            // Rebuild the transcript + pickers only when agent state or the
            // font size changed — the 40ms poll used to rebuild the whole
            // widget tree every tick.
            let font_size = self.store.ai_font_size();
            let font_changed = font_size != self.rendered_ai_font_size.get();
            let dirty = agent.ui_revision != self.rendered_agent_revision.get() || font_changed;
            if dirty {
                self.rendered_agent_revision.set(agent.ui_revision);
                self.rendered_ai_font_size.set(font_size);
            }
            if dirty {
            if let Some(box_) = ui.transcript_box.borrow().as_ref() {
                if agent.transcript.is_empty() {
                    while let Some(child) = box_.first_child() { box_.remove(&child); }
                    self.rendered_transcript_keys.borrow_mut().clear();
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
                    let keys: Vec<u64> = agent.transcript.iter()
                        .map(|entry| view_key(&(entry, font_size.to_bits())))
                        .collect();
                    let mut previous = self.rendered_transcript_keys.borrow_mut();
                    if previous.is_empty() {
                        while let Some(child) = box_.first_child() { box_.remove(&child); }
                    }
                    let mut child = box_.first_child();
                    for (i, entry) in agent.transcript.iter().enumerate() {
                        let next = child.as_ref().and_then(|row| row.next_sibling());
                        if previous.get(i) != keys.get(i) {
                            let row = transcript_row(entry, font_size);
                            if let Some(old) = child.as_ref() {
                                box_.insert_child_after(&row, old.prev_sibling().as_ref());
                                box_.remove(old);
                            } else {
                                box_.append(&row);
                            }
                        }
                        child = next;
                    }
                    while let Some(row) = child {
                        child = row.next_sibling();
                        box_.remove(&row);
                    }
                    *previous = keys;
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
            let picker_key = view_key(&(&agent.models, &agent.current_model,
                &agent.thinking_levels, &agent.thinking_level,
                agent.is_updating_model_settings, agent.connection == crate::agent::Connection::Ready, lang));
            if self.rendered_picker_key.replace(picker_key) != picker_key {
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
            }
            if font_changed {
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
            } // if dirty
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

    /// The completion coordinator's event handler — toolchain await, event
    /// drain and exit detection, all interior-mutated so `&self` suffices.
    fn poll_completion(&self) {
        let completion = self.completion.clone();
        completion.poll_toolchain();
        while let Some(event) = completion.poll_event() {
            completion.handle(&event);
        }
        if completion.poll_exit() {
            completion.process_did_exit();
        }
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
                    self.refresh_git();
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
            WorkspaceMessage::PdfLoaded { hash, info } => {
                if self.pdf_hash == hash {
                    self.pdf = info;
                    self.rendered_pdf_key.set(0);
                    self.render_pdf_page();
                }
            }
            WorkspaceMessage::PdfRendered { key, raster } => {
                if key == self.rendered_pdf_key.get() {
                    self.displayed_pdf_key.set(key);
                    UI.with(|ui| {
                        if let Some(picture) = ui.pdf_picture.borrow().as_ref() {
                            let texture = crate::pdf::texture_from_pixels(raster.pixels, raster.width, raster.height, raster.stride);
                            picture.set_paintable(Some(&texture));
                            picture.set_size_request(raster.width, raster.height);
                        }
                    });
                    if let Some((page, x, y, w, h)) = self.pending_pdf_highlight.take() {
                        glib::idle_add_local_once(move || STATE.with(|slot| {
                            if let Some(state) = slot.borrow().as_ref() {
                                let mut state = state.borrow_mut();
                                if state.rendered_pdf_key.get() == key {
                                    state.show_synctex_highlight(page, x, y, w, h);
                                }
                            }
                        }));
                    }
                }
            }
            WorkspaceMessage::BuildEvent(event) => {
                self.model.apply_build_event(event);
                if !self.build_ui_pending.replace(true) {
                    glib::timeout_add_local_once(Duration::from_millis(50), || {
                        STATE.with(|slot| {
                            if let Some(state) = slot.borrow().as_ref() {
                                let mut s = state.borrow_mut();
                                s.build_ui_pending.set(false);
                                s.refresh_build_ui();
                            }
                        });
                    });
                }
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
                // External edits also move git status — refresh keeps the
                // panel live like VSCode's filesystem watcher.
                self.refresh_git();
            }
            WorkspaceMessage::AgentActivityFinished => {
                let confirm = self.store.confirm_overwrite();
                self.model.refresh_after_agent_activity(confirm);
                if let Some(editor) = &self.editor {
                    editor.refresh_from_session();
                }
                self.refresh_after_document_change();
            }
            WorkspaceMessage::GitRefreshed(result) => {
                self.git_refresh_pending.set(false);
                if self.git_refresh_root.borrow_mut().take() != self.model.project_url {
                    self.refresh_git();
                    return;
                }
                match result {
                    Ok(Some(GitRefresh {
                        status,
                        commits,
                        branches,
                    })) => {
                        let old = self.model.git_status.replace(status);
                        if old.is_some() { std::thread::spawn(move || drop(old)); }
                        self.model.git_commits = commits;
                        self.model.git_branches = branches;
                    }
                    // `Ok(None)` = not a repository; `Err` = git unusable —
                    // both land on the empty page with its init button.
                    _ => {
                        self.model.git_status = None;
                        self.model.git_commits.clear();
                        self.model.git_branches.clear();
                    }
                }
                self.refresh_git_panel();
            }
            WorkspaceMessage::GitOpFinished {
                error,
                clear_commit,
            } => {
                self.model.git_busy = false;
                self.model.git_error = error;
                if clear_commit && self.model.git_error.is_none() {
                    self.model.git_commit_message.clear();
                    UI.with(|ui| {
                        if let Some(view) = ui.git_commit_view.borrow().as_ref() {
                            view.buffer().set_text("");
                        }
                    });
                }
                // Post-op refresh — the panel rebuilds when GitRefreshed
                // lands; update the busy spinner immediately.
                self.refresh_git();
                self.refresh_git_panel();
            }
            WorkspaceMessage::GitSuggestFinished(result) => {
                self.model.git_suggest_busy = false;
                match result {
                    Ok(message) => {
                        self.model.git_commit_message = message.clone();
                        UI.with(|ui| {
                            if let Some(view) = ui.git_commit_view.borrow().as_ref() {
                                view.buffer().set_text(&message);
                            }
                        });
                    }
                    Err(error) => self.model.git_error = Some(error),
                }
                self.refresh_git_panel();
                self.refresh_git_commit_button();
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

pub(crate) fn wake_agent() {
    use std::sync::atomic::{AtomicBool, Ordering};
    static PENDING: AtomicBool = AtomicBool::new(false);
    if PENDING.swap(true, Ordering::AcqRel) { return; }
    glib::idle_add_once(|| {
        PENDING.store(false, Ordering::Release);
        STATE.with(|slot| {
            if let Some(state) = slot.borrow().as_ref() {
                let mut s = state.borrow_mut();
                if s.agent.is_some() { s.poll_agent(); }
                s.poll_completion();
                s.drain_side_effects();
            }
        });
    });
}

fn view_key(value: &impl std::hash::Hash) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    value.hash(&mut hash);
    hash.finish()
}

fn clear_list(list: &gtk4::ListBox) {
    while let Some(row) = list.row_at_index(0) {
        list.remove(&row);
    }
}

/// One TODOs-pane row: done checkbox, text + `file:line`, and the
/// rename/delete affordances of the SwiftUI context menu. Activation
/// (jump) is wired on the ListBox; the child carries the `todo-{i}` index.
fn append_todo_row(list: &gtk4::ListBox, index: usize, item: &DocumentTodoItem, lang: &str) {
    let row = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
    row.set_widget_name(&format!("todo-{index}"));
    row.set_margin_start(8);
    row.set_margin_end(8);
    row.set_margin_top(3);
    row.set_margin_bottom(3);

    let check = gtk4::CheckButton::new();
    check.set_active(item.done);
    compat::initial_tooltip(&check, &tr(lang, "todos.toggle_help"));
    check.connect_toggled(move |_| {
        STATE.with(|s| {
            if let Some(state) = s.borrow().as_ref() {
                if let Ok(mut s) = state.try_borrow_mut() {
                    if let Some(item) = s.model.todo_items.get(index).cloned() {
                        s.todo_edit_action(&item, TodoLineEdit::ToggleDone);
                    }
                }
            }
        });
    });
    row.append(&check);

    let body = gtk4::Box::new(gtk4::Orientation::Vertical, 1);
    body.set_hexpand(true);
    let text = gtk4::Label::new(Some(&item.text));
    text.set_xalign(0.0);
    text.set_ellipsize(gtk4::pango::EllipsizeMode::End);
    if item.done {
        // `.strikethrough + .secondary` — done items dim and strike through.
        let attrs = gtk4::pango::AttrList::new();
        attrs.insert(gtk4::pango::AttrInt::new_strikethrough(true));
        text.set_attributes(Some(&attrs));
        text.add_css_class("dim-label");
    }
    let meta = gtk4::Label::new(Some(&format!("{}:{}", item.file, item.line)));
    meta.set_xalign(0.0);
    meta.add_css_class("dim-label");
    meta.add_css_class("caption");
    body.append(&text);
    body.append(&meta);
    row.append(&body);

    // Inline rename: swap the labels for an Entry; Return applies.
    let rename = gtk4::Button::from_icon_name("document-edit-symbolic");
    rename.add_css_class("flat");
    compat::initial_tooltip(&rename, &tr(lang, "todos.rename"));
    {
        let body = body.clone();
        rename.connect_clicked(move |_| {
            let entry = gtk4::Entry::new();
            STATE.with(|s| {
                if let Some(state) = s.borrow().as_ref() {
                    if let Ok(s) = state.try_borrow() {
                        if let Some(item) = s.model.todo_items.get(index) {
                            entry.set_text(&item.text);
                        }
                    }
                }
            });
            entry.set_hexpand(true);
            entry.connect_activate(move |e| {
                let new_text = e.text().to_string();
                STATE.with(|s| {
                    if let Some(state) = s.borrow().as_ref() {
                        if let Ok(mut s) = state.try_borrow_mut() {
                            if let Some(item) = s.model.todo_items.get(index).cloned() {
                                s.todo_edit_action(&item, TodoLineEdit::Rename(new_text));
                            }
                        }
                    }
                });
            });
            while let Some(child) = body.first_child() {
                body.remove(&child);
            }
            body.append(&entry);
            entry.grab_focus();
        });
    }
    row.append(&rename);

    let delete = gtk4::Button::from_icon_name("user-trash-symbolic");
    delete.add_css_class("flat");
    compat::initial_tooltip(&delete, &tr(lang, "todos.delete"));
    delete.connect_clicked(move |_| {
        STATE.with(|s| {
            if let Some(state) = s.borrow().as_ref() {
                if let Ok(mut s) = state.try_borrow_mut() {
                    if let Some(item) = s.model.todo_items.get(index).cloned() {
                        s.todo_edit_action(&item, TodoLineEdit::Delete);
                    }
                }
            }
        });
    });
    row.append(&delete);
    list.append(&row);
}

/// Recursive renderer for the project tree — mirrors `ProjectTreeRows` in
/// `ProjectSidebarView.swift`. Directories toggle collapse state (tracked
/// in `WorkspaceModel::collapsed_project_dirs`); files activate documents.
fn append_project_node(
    list: &gtk4::ListBox,
    node: &project_feature::ProjectFileNode,
    depth: u32,
    root: &Option<PathBuf>,
    model: &crate::model::WorkspaceModel,
) {
    let indent = 8 + (depth as i32) * 24;
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
    // Sources activate in the editor; every other listed extension is a
    // figure that opens in the system viewer.
    let is_source = WorkspaceModel::is_source_file(&url);
    let icon = gtk4::Image::from_icon_name(if !is_source {
        "image-x-generic-symbolic"
    } else if url.extension().map(|e| e == "bib").unwrap_or(false) {
        "accessories-dictionary-symbolic"
    } else {
        "x-office-document-symbolic"
    });
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
    if is_source && model.pinned_build_target.as_ref() == Some(&url) {
        row.append(&gtk4::Image::from_icon_name("emblem-important-symbolic"));
    } else if is_source && model.automatic_build_target.as_ref() == Some(&url) {
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
                    if WorkspaceModel::is_source_file(&url) {
                        s.activate_document(url.clone());
                    } else {
                        s.open_external(&url);
                    }
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
    // blocks on its worker and dispatches only arriving messages onto GTK.
    // The Rc<RefCell<AppState>> remains main-thread-only.
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
    compat::initial_tooltip(&sidebar_toggle, &tr(lang, "editor.show_sidebar"));
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
    compat::initial_tooltip(&open_btn, &tr(lang, "workspace.open"));
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
    compat::initial_tooltip(&pdf_toggle, &tr(lang, "preview.title"));
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
    compat::initial_tooltip(&assistant_toggle, &tr(lang, "assistant.title"));
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
    compat::initial_tooltip(&settings_btn, &tr(lang, "command.settings"));
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
    compat::initial_tooltip(&primary_btn, &tr(lang, "command.view"));
    a11y(&primary_btn, "pitex.menu.primary", "command.view");
    header.pack_end(&primary_btn);

    let find_btn = gtk4::Button::from_icon_name("edit-find-symbolic");
    compat::initial_tooltip(&find_btn, &tr(lang, "editor.find"));
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

    // Wait off-thread; idle applications no longer poll empty channels.
    std::thread::spawn(move || {
        while let Ok(message) = model_rx.recv() {
            glib::idle_add_once(move || {
                STATE.with(|slot| {
                    if let Some(state) = slot.borrow().as_ref() {
                        state.borrow_mut().dispatch(message);
                    }
                });
            });
        }
    });

    // Git Integration — refresh while the pane is visible. VSCode watches
    // the worktree; a light 4s poll covers external `git` CLI changes too.
    {
        let state = state.clone();
        glib::timeout_add_local(Duration::from_secs(4), move || {
            let Ok(s) = state.try_borrow_mut() else {
                return glib::ControlFlow::Continue;
            };
            if s.model.console_section == ConsoleSection::Git
                && s.model.bottom_panel_visible
                && !s.model.git_busy
            {
                s.refresh_git();
            }
            glib::ControlFlow::Continue
        });
    }

    state.borrow_mut().apply_theme();
    // System mode: when the OS flips light/dark the effective palette
    // changes — re-resolve colors like the Swift `effectiveAppearance`
    // observation does. Under a pinned Force* scheme `dark` never fires.
    {
        let state = state.clone();
        adw::StyleManager::default().connect_dark_notify(move |_| {
            let Ok(s) = state.try_borrow() else { return };
            s.apply_theme();
            s.rehighlight();
        });
    }
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

/// Symbols popover — the GTK counterpart of `SymbolsPaletteView`: a
/// category picker over a glyph grid from the shared `TEX_SYMBOLS`
/// catalogue. A click runs `insert_at_cursor`, which flows through the
/// buffer's `changed` hook like typed text (session submit, undo,
/// autosave all see it) and never replaces surrounding text.
fn build_symbols_popover() -> gtk4::Popover {
    let lang = LANG.with(|l| l.get());
    let popover = gtk4::Popover::new();
    let root = gtk4::Box::new(gtk4::Orientation::Vertical, 6);
    root.set_margin_top(8);
    root.set_margin_bottom(8);
    root.set_margin_start(8);
    root.set_margin_end(8);
    a11y(&root, "pitex.symbols.palette", "editor.symbols");

    let titles: Vec<String> = language_core::SymbolCategory::ALL
        .iter()
        .map(|category| tr(lang, category.title_key()))
        .collect();
    let title_strs: Vec<&str> = titles.iter().map(String::as_str).collect();
    let picker = gtk4::DropDown::from_strings(&title_strs);
    a11y(&picker, "pitex.symbols.category", "editor.symbols");
    root.append(&picker);

    let flow = gtk4::FlowBox::new();
    flow.set_selection_mode(gtk4::SelectionMode::None);
    flow.set_min_children_per_line(6);
    flow.set_max_children_per_line(12);
    flow.set_homogeneous(true);
    let scroller = gtk4::ScrolledWindow::new();
    scroller.set_min_content_width(360);
    scroller.set_min_content_height(240);
    scroller.set_child(Some(&flow));
    root.append(&scroller);
    popover.set_child(Some(&root));

    let rebuild = {
        let popover = popover.clone();
        move |flow: &gtk4::FlowBox, index: u32| {
            while let Some(child) = flow.first_child() {
                flow.remove(&child);
            }
            let Some(&category) = language_core::SymbolCategory::ALL.get(index as usize)
            else {
                return;
            };
            for symbol in language_core::symbols_in(category) {
                let button = gtk4::Button::with_label(symbol.glyph);
                compat::initial_tooltip(&button, symbol.command);
                let command = symbol.command;
                let popover = popover.clone();
                button.connect_clicked(move |_| {
                    STATE.with(|s| {
                        if let Some(state) = s.borrow().as_ref() {
                            if let Ok(st) = state.try_borrow() {
                                if let Some(editor) = &st.editor {
                                    editor.buffer().insert_at_cursor(command);
                                    editor.view().grab_focus();
                                }
                            }
                        }
                    });
                    popover.popdown();
                });
                // `insert(-1)` appends — `FlowBox::append` needs gtk4 v4_6,
                // absent from the Ubuntu 22.04 no-default-features build.
                flow.insert(&button, -1);
            }
        }
    };
    rebuild(&flow, 0);
    {
        let flow = flow.clone();
        picker.connect_selected_notify(move |picker| rebuild(&flow, picker.selected()));
    }
    popover
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

    // TODOs shares the lower pane with Project; task actions stay unchanged.
    let todos_page = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    let todos_header = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
    todos_header.set_margin_start(10);
    todos_header.set_margin_end(6);
    todos_header.set_margin_top(4);
    let todos_title = gtk4::Label::new(Some(&tr(lang, "sidebar.todos")));
    todos_title.set_xalign(0.0);
    todos_title.set_hexpand(true);
    todos_title.add_css_class("caption");
    todos_title.add_css_class("dim-label");
    todos_header.append(&todos_title);
    let todo_add = gtk4::Button::from_icon_name("list-add-symbolic");
    todo_add.add_css_class("flat");
    compat::initial_tooltip(&todo_add, &tr(lang, "todos.add_help"));
    a11y(&todo_add, "pitex.sidebar.todos.add", "todos.add_help");
    {
        let state = state.clone();
        todo_add.connect_clicked(move |_| {
            let Ok(mut s) = state.try_borrow_mut() else { return };
            s.add_todo_action();
        });
    }
    ui.todo_add_button.replace(Some(todo_add.clone()));
    todos_header.append(&todo_add);
    todos_page.append(&todos_header);
    let todos_scroll = gtk4::ScrolledWindow::new();
    todos_scroll.set_vexpand(true);
    let todo_list = gtk4::ListBox::new();
    todo_list.set_selection_mode(gtk4::SelectionMode::None);
    todo_list.add_css_class("navigation-sidebar");
    a11y(&todo_list, "pitex.sidebar.todos", "sidebar.todos");
    {
        let state = state.clone();
        todo_list.connect_row_activated(move |_, row| {
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
            if let Some(item) = s.model.todo_items.get(idx).cloned() {
                s.open_todo(&item);
            }
        });
    }
    todos_scroll.set_child(Some(&todo_list));
    todos_page.append(&todos_scroll);
    ui.todo_list.replace(Some(todo_list));

    stack.set_visible_child_name("outline");
    ui.sidebar_stack.replace(Some(stack.clone()));
    root.append(&stack);
    root.append(&gtk4::Separator::new(gtk4::Orientation::Horizontal));

    // Project and TODOs switch independently of the document structure above.
    let project_page = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
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
    compat::initial_tooltip(&rescan, &tr(lang, "sidebar.rescan_help"));
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
    compat::initial_tooltip(&pin, &tr(lang, "sidebar.pin_help"));
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
    project_page.append(&header);

    let project_scroll = gtk4::ScrolledWindow::new();
    project_scroll.set_vexpand(true);
    project_scroll.set_min_content_height(140);
    let project_list = gtk4::ListBox::new();
    project_list.set_selection_mode(gtk4::SelectionMode::None);
    project_list.add_css_class("navigation-sidebar");
    project_scroll.set_child(Some(&project_list));
    ui.project_list.replace(Some(project_list));
    project_page.append(&project_scroll);

    // Compiled artifacts — PDFs sharing the main document's stem sit pinned
    // under the tree behind a dashed divider (the SwiftUI `outputPDFs`
    // section in `ProjectSidebarView`). Hidden until `refresh_sidebar`
    // finds matching outputs.
    let output_sep = gtk4::Separator::new(gtk4::Orientation::Horizontal);
    output_sep.add_css_class("pitex-dash-sep");
    output_sep.set_margin_start(10);
    output_sep.set_margin_end(10);
    output_sep.set_margin_top(4);
    output_sep.set_visible(false);
    let output_list = gtk4::ListBox::new();
    output_list.set_selection_mode(gtk4::SelectionMode::None);
    output_list.add_css_class("navigation-sidebar");
    output_list.set_visible(false);
    a11y(&output_list, "pitex.sidebar.outputs", "sidebar.project");
    ui.project_output_sep.replace(Some(output_sep.clone().upcast()));
    ui.project_output_list.replace(Some(output_list.clone()));
    project_page.append(&output_sep);
    project_page.append(&output_list);
    if let Some(display) = gdk::Display::default() {
        let provider = gtk4::CssProvider::new();
        provider.load_from_data(
            ".pitex-dash-sep { border-top: 1px dashed alpha(currentColor, 0.35); min-height: 0; }",
        );
        gtk4::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }

    let project_stack = gtk4::Stack::new();
    project_stack.set_vexpand(true);
    project_stack.add_titled(&project_page, Some("project"), &tr(lang, "sidebar.project"));
    project_stack.add_titled(&todos_page, Some("todos"), &tr(lang, "sidebar.todos"));
    project_stack.set_visible_child_name("project");
    let project_switcher = gtk4::StackSwitcher::new();
    project_switcher.set_stack(Some(&project_stack));
    project_switcher.set_margin_start(6);
    project_switcher.set_margin_end(6);
    project_switcher.set_margin_top(6);
    project_switcher.set_margin_bottom(6);
    a11y(&project_switcher, "pitex.sidebar.projectSection", "sidebar.project");
    root.append(&project_switcher);
    root.append(&project_stack);
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

    // TeXifier-style symbols palette — the same catalogue + ordering the
    // macOS `SymbolsPaletteView` renders; a glyph click inserts the LaTeX
    // command at the caret. Sits immediately left of the "+" menu like
    // the macOS tab strip.
    let symbols_btn = gtk4::MenuButton::new();
    symbols_btn.set_icon_name("accessories-character-map-symbolic");
    compat::initial_tooltip(&symbols_btn, &tr(lang, "editor.symbols"));
    a11y(&symbols_btn, "pitex.toolbar.symbols", "editor.symbols");
    symbols_btn.set_popover(Some(&build_symbols_popover()));
    strip.append(&symbols_btn);

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
    compat::initial_tooltip(&build_btn, &tr(lang, "build.start"));
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
    let tooltip_state = Rc::downgrade(state);
    compat::dynamic_tooltip(&disk, move || {
        let state = tooltip_state.upgrade()?;
        let state = state.try_borrow().ok()?;
        let conflicted = state.model.document_snapshot.as_ref()
            .map(|s| s.save_state == DocumentSaveState::Conflicted).unwrap_or(false);
        Some(tr(state.language, if conflicted { "editor.disk_changed" } else { "editor.disk_unchanged" }))
    });
    a11y(&disk, "pitex.editor.diskStatus", "editor.disk_unchanged");
    {
        let state = state.clone();
        disk.connect_clicked(move |_| state.borrow_mut().reload_active());
    }
    ui.disk_status.replace(Some(disk.clone()));
    editor_header.append(&disk);
    let reload = gtk4::Button::from_icon_name("view-refresh-symbolic");
    reload.add_css_class("flat");
    compat::initial_tooltip(&reload, &tr(lang, "editor.reload"));
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
    // ibus-hangul Backspace fix — capture on the scroller so it runs before
    // the view's own IM filter; the resolver re-reads the current adapter.
    compat::fix_ime_backspace(&scroller, || {
        STATE.with(|s| {
            s.borrow()
                .as_ref()
                .and_then(|state| state.try_borrow().ok())
                .and_then(|s| s.editor.clone())
        })
    });
    // The fold chip layer overlays the scroller from outside — putting it
    // inside would break the view's scroll adjustments (minimap, jump-to).
    let editor_overlay = gtk4::Overlay::new();
    editor_overlay.set_child(Some(&scroller));
    ui.editor_overlay.replace(Some(editor_overlay.clone()));
    // The ghost-completion label: translucent, non-interactive and
    // hidden until a suggestion lands — the coordinator moves it to the
    // caret's window position via margins. Decorative chrome like the
    // fold chip layer, so it carries no a11y identifier.
    let ghost = gtk4::Label::new(None);
    ghost.set_opacity(crate::ghost_completion::GHOST_OPACITY);
    ghost.set_halign(gtk4::Align::Start);
    ghost.set_valign(gtk4::Align::Start);
    ghost.set_can_target(false);
    ghost.set_visible(false);
    editor_overlay.add_overlay(&ghost);
    ui.ghost_label.replace(Some(ghost));
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
    compat::initial_tooltip(&flip_btn, &tr(lang, "editor.flip_panels"));
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

#[cfg(all(test, target_os = "linux"))]
mod startup_tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    /// Run alone under Xvfb: this exercises the real window and async open path.
    #[test]
    #[ignore = "requires a GTK display and isolated process"]
    fn small_project_opens_without_blocking_the_ui() {
        let root = std::env::temp_dir().join(format!("pitex-startup-{}", std::process::id()));
        let project = root.join("project");
        std::fs::create_dir_all(&project).unwrap();
        // Keep preferences, agent files and installer work inside this test.
        std::env::set_var("XDG_CONFIG_HOME", root.join("config"));
        std::env::set_var("XDG_CACHE_HOME", root.join("cache"));
        std::env::set_var("PI_CODING_AGENT_DIR", root.join("pi"));
        let launcher = crate::agent::pi_paths::runtime_executable();
        std::fs::create_dir_all(launcher.parent().unwrap()).unwrap();
        std::fs::write(&launcher, r#"#!/usr/bin/python3
import json, sys, time
time.sleep(2)
model = {"id": "fixture", "provider": "fixture", "name": "Fixture model"}
for line in sys.stdin:
    request = json.loads(line)
    command = request["type"]
    data = {"get_state": {"model": model, "thinkingLevel": "off"},
            "get_available_models": {"models": [model]},
            "get_available_thinking_levels": {"levels": ["off"]},
            "get_commands": {"commands": []}}.get(command, {})
    print(json.dumps({"type": "response", "command": command, "id": request.get("id"), "success": True, "data": data}), flush=True)
"#).unwrap();
        std::fs::set_permissions(&launcher, std::fs::Permissions::from_mode(0o700)).unwrap();
        std::fs::write(crate::agent::pi_paths::runtime_directory().join("package.json"),
            format!(r#"{{"name":"{}","version":"{}"}}"#,
                crate::agent::pi_installer::PACKAGE_NAME, crate::agent::pi_installer::DESIRED_VERSION)).unwrap();
        let source = "\\documentclass{article}\n\\begin{document}\n\\section{Hello}\n한글 $x$ test.\n\\label{sec:hello}\n\\end{document}\n";
        let file = project.join("main.tex");
        std::fs::write(&file, source).unwrap();
        std::fs::write(project.join("main.pdf"), include_bytes!("../../../../Fixtures/projects/startup-preview/main.pdf")).unwrap();
        // An independent watchdog catches a GTK callback that never returns.
        let finished = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let watchdog = finished.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_secs(20));
            if !watchdog.load(std::sync::atomic::Ordering::Acquire) {
                eprintln!("FAIL: application startup/file-open stopped processing GTK events");
                std::process::abort();
            }
        });
        adw::init().unwrap();
        // GTK queries X11 from the tooltip setter even before a widget is
        // parented, unless it is hidden. Check the real property notification.
        for visible in [true, false] {
            let button = gtk4::Button::new();
            button.set_visible(visible);
            button.connect_tooltip_text_notify(|button| assert!(!button.is_visible()));
            compat::initial_tooltip(&button, "help");
            assert_eq!(button.tooltip_text().as_deref(), Some("help"));
            assert_eq!(button.is_visible(), visible);
        }
        let app = adw::Application::builder().application_id("app.pitex.StartupTest").build();
        app.register(None::<&gio::Cancellable>).unwrap();
        eprintln!("startup: building real window");
        build_window(&app, "startup-test");
        let state = STATE.with(|slot| slot.borrow().as_ref().unwrap().clone());
        let tooltip_changes = Rc::new(Cell::new(0));
        UI.with(|ui| {
            for button in [ui.disk_status.borrow().as_ref(), ui.build_button.borrow().as_ref()].into_iter().flatten() {
                let changes = tooltip_changes.clone();
                button.connect_tooltip_text_notify(move |_| changes.set(changes.get() + 1));
            }
        });
        eprintln!("startup: opening small TeX file");
        state.borrow_mut().open_selected(file);
        let context = glib::MainContext::default();
        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        let mut ready_ticks = 0;
        while std::time::Instant::now() < deadline {
            while context.pending() { context.iteration(false); }
            if matches!(state.borrow().model.phase, WorkspacePhase::Ready) {
                assert_eq!(state.borrow().editor.as_ref().unwrap().text(), source);
                ready_ticks += 1;
                if ready_ticks >= 500 && state.borrow().agent.as_ref().map(|a| !a.models.is_empty()).unwrap_or(false)
                    && state.borrow().displayed_pdf_key.get() != 0 { break; }
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        if ready_ticks < 500 || !state.borrow().agent.as_ref().map(|a| !a.models.is_empty()).unwrap_or(false)
            || state.borrow().displayed_pdf_key.get() == 0 {
            eprintln!("FAIL: startup phase={:?}, ticks={}, PDF={}, agent={:?}", state.borrow().model.phase,
                ready_ticks, state.borrow().displayed_pdf_key.get(), state.borrow().agent.as_ref().map(|a| &a.connection));
            std::process::abort();
        }
        assert_eq!(tooltip_changes.get(), 0, "refreshing a mapped widget must not trigger X11 tooltip pointer queries");
        {
            let count: usize = std::env::var("PITEX_AUDIT_GIT_ROWS").unwrap_or_else(|_| "1012101".into()).parse().unwrap();
            let status = std::sync::Arc::new(git_core::GitStatus {
                root: project.to_string_lossy().into_owned(), repo_name: "large-status".into(),
                branch: "main".into(), upstream: None, ahead: 0, behind: 0, staged: Vec::new(),
                unstaged: (0..count).map(|i| git_core::GitChange {
                    path: format!("files/{i}.txt"), original_path: None,
                    kind: git_core::GitChangeKind::Untracked, staged: false,
                }).collect(),
            });
            let started = std::time::Instant::now();
            state.borrow_mut().model.git_status = Some(status);
            state.borrow().refresh_git_panel();
            eprintln!("GIT_AUDIT hidden {count} entries: {:?}", started.elapsed());
            assert!(started.elapsed() < Duration::from_secs(1), "hidden Git panel blocked the main thread");
            let list = UI.with(|ui| ui.git_changes_list.borrow().as_ref().unwrap().clone());
            let model = UI.with(|ui| ui.git_changes_model.borrow().as_ref().unwrap().clone());
            assert_eq!(model.n_items(), 0, "hidden panel must not create rows");
            // Keep the synthetic snapshot stable while exercising the real view.
            state.borrow().git_refresh_pending.set(true);
            let started = std::time::Instant::now();
            {
                let mut s = state.borrow_mut();
                s.model.console_section = ConsoleSection::Git;
                s.model.bottom_panel_visible = true;
                s.refresh_console_visibility();
            }
            for _ in 0..20 {
                while context.pending() { context.iteration(false); }
                std::thread::sleep(Duration::from_millis(5));
            }
            eprintln!("GIT_AUDIT visible {count} entries: {:?}", started.elapsed());
            assert!(started.elapsed() < Duration::from_secs(1), "showing Git blocked the main thread");
            assert_eq!(model.n_items() as usize, count + 1);
            let item = model.item(1).unwrap();
            let started = std::time::Instant::now();
            for _ in 0..100 { state.borrow().refresh_git_panel(); }
            assert_eq!(model.item(1).unwrap(), item, "unchanged refresh rebuilt the list");
            eprintln!("GIT_AUDIT 100 unchanged refreshes: {:?}", started.elapsed());
            assert!(started.elapsed() < Duration::from_secs(1));
            let adjustment = list.vadjustment().unwrap();
            let started = std::time::Instant::now();
            adjustment.set_value(adjustment.upper() - adjustment.page_size());
            for _ in 0..20 {
                while context.pending() { context.iteration(false); }
                std::thread::sleep(Duration::from_millis(5));
            }
            let mut children = 0;
            let mut child = list.first_child();
            while let Some(widget) = child { children += 1; child = widget.next_sibling(); }
            eprintln!("GIT_AUDIT scroll to end: {:?}, {children} live row widgets", started.elapsed());
            assert!(started.elapsed() < Duration::from_secs(1));
            assert!(list.is_mapped() && list.height() > 0, "Git list must actually be visible");
            assert!(children > 0 && children < 512, "list allocated offscreen rows");
            fn has_name(widget: &gtk4::Widget, name: &str) -> bool {
                if widget.widget_name() == name { return true; }
                let mut child = widget.first_child();
                while let Some(node) = child {
                    if has_name(&node, name) { return true; }
                    child = node.next_sibling();
                }
                false
            }
            assert!(has_name(list.upcast_ref(), &format!("gitc:u:files/{}.txt", count - 1)), "last file was not rendered after scrolling");
            let Some(crate::git_list::Row::Change(last)) = model.row(count as u32) else { panic!("last row missing") };
            assert_eq!(last.path, format!("files/{}.txt", count - 1));
            // Editor events still run after displaying and scrolling the full list.
            state.borrow().editor.as_ref().unwrap().view().grab_focus();
            while context.pending() { context.iteration(false); }
            assert_eq!(state.borrow().editor.as_ref().unwrap().text(), source);
        }
        state.borrow_mut().shutdown_agent();
        UI.with(|ui| ui.window.borrow().as_ref().unwrap().close());
        finished.store(true, std::sync::atomic::Ordering::Release);
        eprintln!("PASS: small project opened and GTK continued processing events");
    }
}
