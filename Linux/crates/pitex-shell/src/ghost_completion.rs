//! Port of the `GhostCompletionCoordinator`/`GhostCompletionOverlayView`
//! half of `EditorContainerView.swift` — Copilot-style inline LaTeX
//! completion. A dedicated `pi --mode rpc` subprocess (separate from the
//! chat `AgentCoordinator`, so prompts never touch the transcript or
//! contend with a running chat request) answers ~50/20-line continuation
//! prompts; the reply renders as translucent ghost text at the caret.
//! Tab accepts, Esc/typing/caret moves dismiss. The whole feature is a
//! silent no-op while the setting is off, the document is not `.tex`, or
//! pi cannot be resolved.

use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::rc::{Rc, Weak};
use std::time::Duration;

use gtk4::prelude::*;
use gtk4::{gdk, glib};

use crate::agent::{
    locate_pi_executable_in, PiAgentProcess, PiRPCCommand, PiRPCEvent, PiToolchain,
};

/// The 600ms debounce matches the Swift `Task.sleep(for: .milliseconds(600))`.
const DEBOUNCE_MS: u64 = 600;
/// `~50 lines before / ~20 lines after`, char-bounded — see
/// `GhostCompletionCoordinator.prompt` in Swift.
const LINES_BEFORE: usize = 50;
const LINES_AFTER: usize = 20;
const MAX_BEFORE: usize = 6_000;
const MAX_AFTER: usize = 2_000;
/// `withAlphaComponent(0.38)` — the ghost's translucency, applied to the
/// overlay label where it's created in `build_editor_column`.
pub(crate) const GHOST_OPACITY: f64 = 0.38;

/// The `(toolchain, executable)` pair `PiToolchain::discover` resolves
/// off-thread — the awaited half of `start_process`.
type ToolchainResult = std::sync::mpsc::Receiver<(PiToolchain, Option<PathBuf>)>;

/// `GhostCompletionCoordinator.Context` — workspace state read at fire
/// time so the setting toggle and document switches apply immediately
/// without re-attaching.
#[derive(Default)]
pub struct CompletionContext {
    pub project_root: Option<PathBuf>,
    pub file_name: String,
    pub is_tex: bool,
    pub enabled: bool,
    /// `textView.isEditable && !textView.hasMarkedText()` — the Swift
    /// editable/IME guards folded into one provider-side flag.
    pub editable: bool,
}

/// `GhostCompletionCoordinator` — owns the dedicated pi subprocess, the
/// debounce/supersede state machine, and the ghost label's position. The
/// label itself is created once in `build_editor_column` and hosted by the
/// editor overlay like the fold chip layer.
pub struct GhostCompletionCoordinator {
    /// Rebound per session in `attach` — like the Swift observers rebuilt
    /// against each new `EditorMacAdapter`.
    view: RefCell<Option<sourceview5::View>>,
    buffer: RefCell<Option<sourceview5::Buffer>>,
    label: RefCell<Option<gtk4::Label>>,
    /// `contextProvider` — reads the workspace through `STATE` so no Rc
    /// cycle forms between the coordinator and `AppState`.
    context_provider: RefCell<Box<dyn Fn() -> CompletionContext>>,
    self_weak: RefCell<Weak<GhostCompletionCoordinator>>,

    process: RefCell<Option<PiAgentProcess>>,
    /// The awaited half of `PiToolchain.discover` — the completion-side
    /// `AgentCoordinator::toolchain_rx`.
    toolchain_rx: RefCell<Option<ToolchainResult>>,
    pending_root: RefCell<Option<PathBuf>>,
    /// Spawn/resolution failures latch here until the next `attach`, so a
    /// missing pi costs one probe per document — not one per idle pause.
    pi_unavailable: Cell<bool>,
    /// The prompt staged while the subprocess is still starting.
    staged_prompt: RefCell<Option<String>>,
    /// The prompt id whose reply may produce a suggestion; a new request
    /// supersedes it so stale replies are dropped.
    pending_request_id: RefCell<Option<String>>,
    /// True while a request that may answer is in flight — armed at send
    /// time rather than on the prompt `response`, whose position relative
    /// to `message_end`/`agent_end` is not guaranteed by the wire order.
    armed: Cell<bool>,
    /// Set by the first `agent_start` after arming — it marks this
    /// request's own run. Leftover events from a superseded, aborted run
    /// can land after the prompt was sent but always before its run
    /// starts, so `message_end` is only honoured while this is set.
    run_started: Cell<bool>,
    /// Debounce generation — bumped on every edit/caret move so stale
    /// timers no-op; the `debounceTask?.cancel()` equivalent.
    generation: Cell<u64>,
    /// The visible ghost plus the char offset it was generated for —
    /// `reposition` anchors here so a scroll never drifts to a moved caret.
    suggestion: RefCell<Option<String>>,
    anchor: Cell<i32>,
    has_preedit: Cell<bool>,
}

impl GhostCompletionCoordinator {
    pub fn new() -> Rc<Self> {
        let engine = Rc::new(Self {
            view: RefCell::new(None),
            buffer: RefCell::new(None),
            label: RefCell::new(None),
            context_provider: RefCell::new(Box::new(CompletionContext::default)),
            self_weak: RefCell::new(Weak::new()),
            process: RefCell::new(None),
            toolchain_rx: RefCell::new(None),
            pending_root: RefCell::new(None),
            pi_unavailable: Cell::new(false),
            staged_prompt: RefCell::new(None),
            pending_request_id: RefCell::new(None),
            armed: Cell::new(false),
            run_started: Cell::new(false),
            generation: Cell::new(0),
            suggestion: RefCell::new(None),
            anchor: Cell::new(0),
            has_preedit: Cell::new(false),
        });
        *engine.self_weak.borrow_mut() = Rc::downgrade(&engine);
        engine
    }

    /// `completion.contextProvider = { … }` — set once in `AppState::new`.
    pub fn set_context_provider(&self, provider: Box<dyn Fn() -> CompletionContext>) {
        *self.context_provider.borrow_mut() = provider;
    }

    /// `attach(to:)` — rebind the text/selection/key hooks against the
    /// session's fresh adapter, mirroring
    /// `completion.attach(to: appEnvironment.editor)`.
    pub fn attach(
        self: &Rc<Self>,
        view: &sourceview5::View,
        buffer: &sourceview5::Buffer,
        label: &gtk4::Label,
    ) {
        self.has_preedit.set(false);
        *self.view.borrow_mut() = Some(view.clone());
        *self.buffer.borrow_mut() = Some(buffer.clone());
        *self.label.borrow_mut() = Some(label.clone());
        self.generation.set(self.generation.get() + 1);
        self.supersede();
        self.dismiss();
        // A fresh document gets one more chance to find pi.
        self.pi_unavailable.set(false);
        self.install_hooks();
    }

    /// `apply_editor_preferences`'s font half — the label tracks the editor
    /// font so ghost text and document text share family and size.
    pub fn refresh_font(&self, desc: &gtk4::pango::FontDescription) {
        if let Some(label) = self.label.borrow().as_ref() {
            let attrs = gtk4::pango::AttrList::new();
            attrs.insert(gtk4::pango::AttrFontDesc::new(desc));
            label.set_attributes(Some(&attrs));
        }
    }

    fn install_hooks(self: &Rc<Self>) {
        let Some(buffer) = self.buffer.borrow().clone() else {
            return;
        };
        let Some(view) = self.view.borrow().clone() else {
            return;
        };

        let weak = Rc::downgrade(self);
        view.connect_preedit_changed(move |_, text| {
            if let Some(engine) = weak.upgrade() {
                engine.has_preedit.set(!text.is_empty());
                engine.dismiss();
                engine.supersede();
            }
        });

        // NSText.didChangeNotification — every edit supersedes the ghost.
        let weak = Rc::downgrade(self);
        buffer.connect_changed(move |_| {
            if let Some(engine) = weak.upgrade() {
                engine.editor_activity();
            }
        });
        // NSTextView.didChangeSelectionNotification — caret moves too.
        let weak = Rc::downgrade(self);
        buffer.connect_mark_set(move |buffer, _iter, mark| {
            if mark != &buffer.get_insert() && mark != &buffer.selection_bound() {
                return;
            }
            if let Some(engine) = weak.upgrade() {
                engine.editor_activity();
            }
        });

        // Tab/Esc in the Capture phase, before the source view consumes
        // them — the `textView(_:doCommandBy:)` insertTab/cancelOperation
        // equivalent. Only claimed while a ghost is visible.
        let weak = Rc::downgrade(self);
        let keys = gtk4::EventControllerKey::new();
        keys.set_propagation_phase(gtk4::PropagationPhase::Capture);
        keys.connect_key_pressed(move |_, key, _code, state| {
            let Some(engine) = weak.upgrade() else {
                return glib::Propagation::Proceed;
            };
            if engine.has_preedit.get() || engine.suggestion.borrow().is_none() {
                return glib::Propagation::Proceed;
            }
            let mods = state
                & (gdk::ModifierType::SHIFT_MASK
                    | gdk::ModifierType::CONTROL_MASK
                    | gdk::ModifierType::ALT_MASK
                    | gdk::ModifierType::SUPER_MASK
                    | gdk::ModifierType::HYPER_MASK
                    | gdk::ModifierType::META_MASK);
            if !mods.is_empty() {
                return glib::Propagation::Proceed;
            }
            if key == gdk::Key::Tab {
                if engine.accept() {
                    return glib::Propagation::Stop;
                }
                return glib::Propagation::Proceed;
            }
            if key == gdk::Key::Escape {
                engine.dismiss();
                return glib::Propagation::Stop;
            }
            glib::Propagation::Proceed
        });
        view.add_controller(keys);

        // The label is fixed to the overlay, not the document — scrolling
        // re-anchors it to the caret's window position, like the fold chip
        // layer's `queue_draw` on adjustment changes.
        for adjustment in [view.hadjustment(), view.vadjustment()]
            .into_iter()
            .flatten()
        {
            let weak = Rc::downgrade(self);
            adjustment.connect_value_changed(move |_| {
                if let Some(engine) = weak.upgrade() {
                    engine.reposition();
                }
            });
        }
    }

    /// Edits and caret moves both count as "the user kept typing": drop the
    /// visible ghost, supersede the in-flight request and re-arm the 600ms
    /// debounce. O(1) here — context gathering waits for the timer.
    fn editor_activity(&self) {
        self.dismiss();
        self.supersede();
        self.schedule_request();
    }

    /// Tag-and-abort: a new request id makes late replies stale, and the
    /// abort stops the old run early instead of burning provider tokens.
    fn supersede(&self) {
        self.staged_prompt.borrow_mut().take();
        let in_flight = self.pending_request_id.borrow().is_some() || self.armed.get();
        self.pending_request_id.borrow_mut().take();
        self.armed.set(false);
        self.run_started.set(false);
        if in_flight {
            if let Some(process) = self.process.borrow().as_ref() {
                let _ = process.send(&PiRPCCommand::Abort, None);
            }
        }
    }

    fn schedule_request(&self) {
        self.generation.set(self.generation.get() + 1);
        let generation = self.generation.get();
        let weak = self.self_weak.borrow().clone();
        glib::timeout_add_local_once(Duration::from_millis(DEBOUNCE_MS), move || {
            let Some(engine) = weak.upgrade() else { return };
            if engine.generation.get() != generation {
                return;
            }
            engine.fire();
        });
    }

    /// The debounced half: gates re-checked at fire time (the setting may
    /// have flipped mid-burst), then the ~50/20-line window is staged for
    /// the subprocess.
    fn fire(&self) {
        let context = (self.context_provider.borrow())();
        if !context.enabled || !context.is_tex || !context.editable || self.pi_unavailable.get() {
            return;
        }
        let Some(root) = context.project_root.clone() else {
            return;
        };
        let Some(buffer) = self.buffer.borrow().clone() else {
            return;
        };
        // A collapsed caret only — a dragged selection never completes.
        if let Some((start, end)) = buffer.selection_bounds() {
            if start.offset() != end.offset() {
                return;
            }
        }
        let iter = buffer.iter_at_mark(&buffer.get_insert());
        let (buf_start, buf_end) = buffer.bounds();
        let text = buffer.text(&buf_start, &buf_end, false).to_string();
        let caret = iter.offset().max(0) as usize;
        *self.staged_prompt.borrow_mut() = Some(Self::prompt(&context.file_name, &text, caret));
        if self
            .process
            .borrow()
            .as_ref()
            .map(|p| p.is_running())
            .unwrap_or(false)
        {
            self.send_staged();
        } else {
            self.start_process(&root);
        }
    }

    /// `prepareAgent`'s lazy counterpart — discovery runs off the UI
    /// thread; resolution/spawn failures latch `pi_unavailable` silently.
    fn start_process(&self, root: &std::path::Path) {
        if self.toolchain_rx.borrow().is_some() {
            return;
        }
        let (tx, rx) = std::sync::mpsc::channel();
        *self.toolchain_rx.borrow_mut() = Some(rx);
        *self.pending_root.borrow_mut() = Some(root.to_path_buf());
        std::thread::spawn(move || {
            let tools = PiToolchain::discover(
                std::env::vars().collect(),
                &dirs::home_dir().unwrap_or_default(),
                &PiToolchain::SYSTEM_DIRECTORIES,
            );
            let executable = locate_pi_executable_in(&tools.environment);
            let _ = tx.send((tools, executable));
            crate::app_ui::wake_agent();
        });
    }

    /// `poll_toolchain` — the event-driven completion of `start_process`.
    pub fn poll_toolchain(&self) {
        let rx = self.toolchain_rx.borrow_mut().take();
        let Some(rx) = rx else { return };
        let Ok((tools, executable)) = rx.try_recv() else {
            *self.toolchain_rx.borrow_mut() = Some(rx);
            return;
        };
        let Some(root) = self.pending_root.borrow_mut().take() else {
            return;
        };
        match executable {
            Some(executable) => self.spawn_with_tools(&executable, &root, &tools),
            None => self.pi_unavailable.set(true),
        }
    }

    fn spawn_with_tools(
        &self,
        executable: &std::path::Path,
        root: &std::path::Path,
        tools: &PiToolchain,
    ) {
        // The completion session skips `--append-system-prompt`: the
        // agentic chat supplement is wasted context for a fill-in engine.
        let arguments = vec![
            "--mode".to_string(),
            "rpc".to_string(),
            "--no-session".to_string(),
        ];
        let launch = match tools.launch(executable, &arguments) {
            Ok(launch) => launch,
            Err(_) => {
                self.pi_unavailable.set(true);
                return;
            }
        };
        let environment =
            crate::agent::AgentCoordinator::child_environment(&launch.0, &tools.environment);
        match PiAgentProcess::start(&launch.0, root, &launch.1, &environment) {
            Ok(process) => {
                *self.process.borrow_mut() = Some(process);
                self.send_staged();
            }
            Err(_) => self.pi_unavailable.set(true),
        }
    }

    /// abort + new_session + tagged prompt: the abort ends a superseded
    /// run, `new_session` keeps every request's context cheap so earlier
    /// completion prompts never accumulate, and the id lets `handle` drop
    /// replies to superseded asks.
    fn send_staged(&self) {
        let binding = self.process.borrow();
        let Some(process) = binding.as_ref() else {
            return;
        };
        if !process.is_running() {
            return;
        }
        let Some(prompt) = self.staged_prompt.borrow_mut().take() else {
            return;
        };
        let id = crate::model::uuid_v4();
        *self.pending_request_id.borrow_mut() = Some(id.clone());
        self.run_started.set(false);
        if process.send(&PiRPCCommand::Abort, None).is_err()
            || process.send(&PiRPCCommand::NewSession, None).is_err()
            || process
                .send(
                    &PiRPCCommand::Prompt {
                        message: prompt,
                        streaming_behavior: None,
                    },
                    Some(&id),
                )
                .is_err()
        {
            self.pending_request_id.borrow_mut().take();
            return;
        }
        self.armed.set(true);
    }

    /// The `response`/`agent_start`/`message_end`/`agent_end` state
    /// machine — everything else (abort/new_session acks, thinking
    /// deltas, tool chatter) is ignored. Arming happens at send time and
    /// `run_started` marks the request's own run, so the machine is
    /// correct whether the prompt `response` is a dispatch-time ack or
    /// the run's final result, and stale events from a just-aborted run
    /// can never produce a suggestion.
    pub fn handle(&self, event: &PiRPCEvent) {
        match event.event_type.as_str() {
            "response" => {
                if event.response_command().as_deref() != Some("prompt") {
                    return;
                }
                let is_current = self
                    .pending_request_id
                    .borrow()
                    .as_ref()
                    .is_some_and(|id| Some(id.as_str()) == event.string("id").as_deref());
                if !is_current {
                    return;
                }
                self.pending_request_id.borrow_mut().take();
                if !event.response_succeeded() {
                    self.armed.set(false);
                    self.run_started.set(false);
                }
            }
            "agent_start" => {
                if self.armed.get() {
                    self.run_started.set(true);
                }
            }
            "message_end" => {
                if !self.armed.get() || !self.run_started.get() {
                    return;
                }
                let Some(message) = event.nested("message") else {
                    return;
                };
                if message.get("role").and_then(|r| r.as_str()) != Some("assistant") {
                    return;
                }
                let text = message
                    .get("content")
                    .and_then(|c| c.as_array())
                    .map(|content| {
                        content
                            .iter()
                            .filter(|c| c.get("type").and_then(|t| t.as_str()) == Some("text"))
                            .filter_map(|c| c.get("text").and_then(|t| t.as_str()))
                            .collect::<String>()
                    })
                    .unwrap_or_default();
                self.apply_suggestion(&text);
            }
            "agent_end" if self.run_started.get() => {
                self.run_started.set(false);
                self.armed.set(false);
            }
            _ => {}
        }
    }

    /// The model's reply becomes the ghost only after cleanup — a stray
    /// code-fence wrapper is dropped, one trailing newline is stripped, and
    /// blank replies show nothing.
    fn apply_suggestion(&self, raw: &str) {
        if self.has_preedit.get() { return; }
        let mut suggestion = raw.to_string();
        if suggestion.starts_with("```") {
            let mut lines: Vec<&str> = suggestion.lines().collect();
            lines.remove(0);
            if lines.last().map(|l| l.trim() == "```").unwrap_or(false) {
                lines.pop();
            }
            suggestion = lines.join("\n");
        }
        if suggestion.ends_with('\n') {
            suggestion.pop();
        }
        if suggestion.trim().is_empty() {
            return;
        }
        *self.suggestion.borrow_mut() = Some(suggestion.clone());
        if let Some(buffer) = self.buffer.borrow().as_ref() {
            self.anchor
                .set(buffer.iter_at_mark(&buffer.get_insert()).offset());
        }
        if let Some(label) = self.label.borrow().as_ref() {
            label.set_text(&suggestion);
        }
        self.reposition();
    }

    /// `draw` — re-anchor the label to the caret's viewport position using
    /// the stored anchor; scrolls and resizes repaint through the
    /// adjustment hook. Off-screen carets hide the label while keeping the
    /// suggestion, so it reappears on scroll-back.
    pub fn reposition(&self) {
        let Some(label) = self.label.borrow().clone() else {
            return;
        };
        let (Some(view), Some(buffer)) = (self.view.borrow().clone(), self.buffer.borrow().clone())
        else {
            label.set_visible(false);
            return;
        };
        let Some(suggestion) = self.suggestion.borrow().clone() else {
            label.set_visible(false);
            return;
        };
        let iter = buffer.iter_at_offset(self.anchor.get());
        let rect = view.iter_location(&iter);
        // `buffer_to_window_coords(Widget)` already lands in viewport
        // coordinates — the same space the fold chip layer draws in.
        let (x, y) = view.buffer_to_window_coords(gtk4::TextWindowType::Widget, rect.x(), rect.y());
        let top = y + if suggestion.contains('\n') {
            rect.height() + 2
        } else {
            0
        };
        let scroller = view.ancestor(gtk4::ScrolledWindow::static_type());
        let visible_height = scroller.map(|s| s.height()).unwrap_or_default();
        if top < -rect.height() || (visible_height > 0 && top > visible_height) {
            label.set_visible(false);
            return;
        }
        label.set_margin_start(x.max(0));
        label.set_margin_top(top.max(0));
        label.set_visible(true);
    }

    /// Tab → `buffer.insert_at_cursor`, the normal edit path: the buffer's
    /// changed hook submits the session mutation, so the insert stays
    /// undo-safe and reschedules the next completion.
    pub fn accept(&self) -> bool {
        if self.has_preedit.get() { self.dismiss(); return false; }
        let Some(suggestion) = self.suggestion.borrow_mut().take() else {
            return false;
        };
        let Some(buffer) = self.buffer.borrow().clone() else {
            return false;
        };
        if let Some(label) = self.label.borrow().as_ref() {
            label.set_visible(false);
        }
        buffer.insert_at_cursor(&suggestion);
        true
    }

    /// Esc/typing/caret move/setting-off — hide the ghost only; superseding
    /// the in-flight request is `editor_activity`'s job.
    pub fn dismiss(&self) {
        self.suggestion.borrow_mut().take();
        if let Some(label) = self.label.borrow().as_ref() {
            label.set_visible(false);
        }
    }

    /// Poll one event (non-blocking) — the UI drains on its idle tick.
    pub fn poll_event(&self) -> Option<PiRPCEvent> {
        self.process
            .borrow()
            .as_ref()
            .and_then(|p| p.events.lock().unwrap().try_recv().ok())
    }

    /// True once when the process exit signal fires.
    pub fn poll_exit(&self) -> bool {
        self.process
            .borrow()
            .as_ref()
            .map(|p| p.exited.lock().unwrap().try_recv().is_ok())
            .unwrap_or(false)
    }

    /// `processDidExit` — drop the dead handle so the next `fire` respawns.
    pub fn process_did_exit(&self) {
        self.process.borrow_mut().take();
        self.armed.set(false);
        self.run_started.set(false);
        self.pending_request_id.borrow_mut().take();
    }

    /// `shutdown` — terminate the subprocess with the workspace and drop
    /// every pending request; the debounce generation bump retires timers.
    pub fn shutdown(&self) {
        if let Some(process) = self.process.borrow_mut().take() {
            process.terminate();
        }
        self.toolchain_rx.borrow_mut().take();
        self.pending_root.borrow_mut().take();
        self.staged_prompt.borrow_mut().take();
        self.pending_request_id.borrow_mut().take();
        self.armed.set(false);
        self.run_started.set(false);
        self.generation.set(self.generation.get() + 1);
        self.dismiss();
    }

    /// `~50 lines before / ~20 lines after, char-bounded` — the completion
    /// envelope. `caret` is a char offset; the `<CURSOR>` marker sits
    /// exactly where the insert happens.
    fn prompt(file_name: &str, text: &str, caret: usize) -> String {
        let caret_byte = text
            .char_indices()
            .nth(caret)
            .map(|(i, _)| i)
            .unwrap_or(text.len());
        // `window_start` is the answer; `probe` walks one newline further
        // up per iteration — searching from `window_start` again would
        // find the same newline every time.
        let mut window_start = caret_byte;
        let mut probe = caret_byte;
        let mut lines = 0;
        while probe > 0 && lines < LINES_BEFORE {
            let Some(previous) = text[..probe].rfind('\n') else {
                window_start = 0;
                break;
            };
            let candidate = previous + 1;
            if caret_byte - candidate > MAX_BEFORE {
                break;
            }
            window_start = candidate;
            probe = previous;
            lines += 1;
        }
        let mut end = caret_byte;
        let mut forward = 0;
        while end < text.len() && forward < LINES_AFTER {
            let Some(next) = text[end..].find('\n').map(|i| end + i) else {
                end = text.len();
                break;
            };
            let candidate = next + 1;
            if candidate - caret_byte > MAX_AFTER {
                break;
            }
            end = candidate;
            forward += 1;
        }
        // Hard cap: a giant line still leaves the window ~6000/~2000
        // chars wide rather than exceeding the line-granular bound.
        let mut window_start = window_start.max(caret_byte.saturating_sub(MAX_BEFORE));
        let mut end = end.min(caret_byte.saturating_add(MAX_AFTER));
        while !text.is_char_boundary(window_start) { window_start += 1; }
        while !text.is_char_boundary(end) { end -= 1; }
        let before = &text[window_start..caret_byte];
        let after = &text[caret_byte..end];
        format!(
            "You are the inline autocompletion engine of the Pitex LaTeX editor. \
             Reply with ONLY the exact text to insert at <CURSOR>: no markdown, \
             no code fences, no explanation. Prefer completing the current \
             command, environment, or line in at most three lines. Reply with \
             nothing when there is no useful continuation.\n\n\
             File: {file_name}\n---\n{before}<CURSOR>{after}"
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompt_caps_preserve_unicode_boundaries() {
        for text in ["한".repeat(3000), "👩🏽‍💻".repeat(1500), "e\u{301}".repeat(4000)] {
            let prompt = GhostCompletionCoordinator::prompt("main.tex", &text, 1500);
            let body = prompt.split("---\n").nth(1).unwrap();
            let (before, after) = body.split_once("<CURSOR>").unwrap();
            assert!(before.len() <= MAX_BEFORE);
            assert!(after.len() <= MAX_AFTER);
            assert!(!body.contains('\u{fffd}'));
        }
    }

    #[test]
    fn prompt_windows_around_cursor_marker() {
        let text = (1..=100)
            .map(|i| format!("line{i}"))
            .collect::<Vec<_>>()
            .join("\n");
        // Caret mid-document (start of "line51").
        let caret = text.match_indices("line51").next().unwrap().0;
        let caret_char = text[..caret].chars().count();
        let prompt = GhostCompletionCoordinator::prompt("main.tex", &text, caret_char);
        assert!(prompt.contains("<CURSOR>"));
        assert!(prompt.contains("File: main.tex"));
        let body = prompt.split("---\n").nth(1).unwrap();
        let (before, after) = body.split_once("<CURSOR>").unwrap();
        // 50 line-starts back = 49 full lines plus the caret line's prefix.
        assert_eq!(before.lines().count(), 49);
        assert!(before.starts_with("line2\n"));
        assert_eq!(after.lines().count(), 20);
        assert!(after.starts_with("line51\n"));
        assert!(after.ends_with("line70\n"));
    }

    #[test]
    fn prompt_bounds_chars() {
        // Giant lines are clamped to the ~6000-char window bound.
        let long_line = "x".repeat(5_000);
        let text = format!("{long_line}\n{long_line}\ncursor");
        let caret = text.len() - "cursor".len();
        let caret_char = text[..caret].chars().count();
        let prompt = GhostCompletionCoordinator::prompt("a.tex", &text, caret_char);
        let body = prompt.split("---\n").nth(1).unwrap();
        let before = body.split("<CURSOR>").next().unwrap();
        assert!(before.len() <= 6_000);
        // The window ends on the caret line's prefix — here just the
        // newline terminating the second giant line.
        assert!(before.ends_with(&format!("{long_line}\n")));
    }

    #[test]
    fn prompt_at_document_start() {
        let prompt = GhostCompletionCoordinator::prompt("a.tex", "abc\ndef", 0);
        assert!(prompt.contains("<CURSOR>abc"));
    }
}
