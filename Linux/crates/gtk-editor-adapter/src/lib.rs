//! Rust port of `Packages/TexApp/Sources/EditorMacAdapter` for GTK.
//!
//! `SessionTextView : NSTextView` maps to a `sourceview5::View` whose
//! `GtkTextBuffer` is the authoritative editing state — the same contract the
//! macOS adapter-test asserts (`textView.string`, `selectedRange()`,
//! `hasMarkedText()`, `markedRange()`, session-owned undo manager).
//! All AppPorts/editor ranges are UTF-16 code units, like `NSRange`; GTK iters
//! are unichar offsets, so every crossing is converted here.

use app_ports::{DocumentMutation, DocumentMutationResult, DocumentSnapshot, DocumentTextRange};
use editor_feature::{EditorDecorationSnapshot, EditorTextRange};
use gtk4::prelude::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// UTF-16 code units in `text` before `char_offset` unichars.
pub fn char_offset_to_utf16(text: &str, char_offset: usize) -> usize {
    text.chars()
        .take(char_offset)
        .map(|c| c.len_utf16())
        .sum()
}

/// Unichars in `text` spanning `utf16_offset` code units. `utf16_offset` is
/// clamped to a char boundary, matching `NSRange` clamping semantics.
pub fn utf16_offset_to_char_offset(text: &str, utf16_offset: usize) -> usize {
    let mut units = 0usize;
    let mut count = 0usize;
    for c in text.chars() {
        if units >= utf16_offset {
            break;
        }
        units += c.len_utf16();
        count += 1;
    }
    count
}

struct Shared {
    committed: RefCell<DocumentSnapshot>,
    desired_text: RefCell<String>,
    is_submitting: Cell<bool>,
    /// Suppresses the next buffer-changed callback during programmatic apply.
    suppress_change: Cell<bool>,
    /// Input-method preedit (marked text) presence — updated from the view's
    /// `preedit-changed` signal, which carries the current preedit string.
    has_preedit: Cell<bool>,
    /// Generation counter invalidating a pending find-indicator removal when
    /// a newer indicator arrives first (Swift cancels its scheduled task).
    find_indicator_generation: Cell<u64>,
}

/// The GTK editor adapter. Constructed on the main thread; all GTK objects are
/// `Rc`-bound and never cross threads, exactly like the `@MainActor` Swift
/// adapter.
pub struct GtkEditorAdapter {
    view: sourceview5::View,
    buffer: sourceview5::Buffer,
    session: Rc<dyn SessionClient>,
    shared: Rc<Shared>,
}

/// The adapter-facing half of `DocumentSessionPort`. GTK cannot block on a
/// session actor, so the port contract is split: the adapter issues mutations
/// and the shell delivers results back through `apply`.
pub trait SessionClient {
    fn snapshot(&self) -> DocumentSnapshot;
    fn submit(&self, mutation: &DocumentMutation) -> DocumentMutationResult;
}

/// Bridge so the shell can wire a real `DocumentSessionPort` behind a closure.
pub struct FnSessionClient<F, G>
where
    F: Fn() -> DocumentSnapshot,
    G: Fn(&DocumentMutation) -> DocumentMutationResult,
{
    snapshot: F,
    submit: G,
}
impl<F, G> FnSessionClient<F, G>
where
    F: Fn() -> DocumentSnapshot,
    G: Fn(&DocumentMutation) -> DocumentMutationResult,
{
    pub fn new(snapshot: F, submit: G) -> Self {
        Self { snapshot, submit }
    }
}
impl<F, G> SessionClient for FnSessionClient<F, G>
where
    F: Fn() -> DocumentSnapshot,
    G: Fn(&DocumentMutation) -> DocumentMutationResult,
{
    fn snapshot(&self) -> DocumentSnapshot {
        (self.snapshot)()
    }
    fn submit(&self, mutation: &DocumentMutation) -> DocumentMutationResult {
        (self.submit)(mutation)
    }
}

impl GtkEditorAdapter {
    /// `EditorMacAdapter.make(session:)` — loads the snapshot, configures the
    /// native text view (plain text, undo enabled) and installs the delegates.
    pub fn make(session: Rc<dyn SessionClient>) -> Self {
        let buffer = sourceview5::Buffer::new(None);
        let view = sourceview5::View::with_buffer(&buffer);
        let snapshot = session.snapshot();
        let shared = Rc::new(Shared {
            committed: RefCell::new(snapshot.clone()),
            desired_text: RefCell::new(snapshot.text.clone()),
            is_submitting: Cell::new(false),
            suppress_change: Cell::new(false),
            has_preedit: Cell::new(false),
            find_indicator_generation: Cell::new(0),
        });

        view.set_monospace(true);
        view.set_wrap_mode(gtk4::WrapMode::Word);
        buffer.set_text(&snapshot.text);

        // Track IM preedit so `has_marked_text` mirrors `hasMarkedText()`:
        // `preedit-changed` fires with the current preedit string, empty
        // included (commit/cancel), on the view's IM context.
        {
            let shared = shared.clone();
            view.connect_preedit_changed(move |_, preedit| {
                shared.has_preedit.set(!preedit.is_empty());
            });
        }

        let adapter = Self {
            view,
            buffer,
            session,
            shared: shared.clone(),
        };
        adapter.install_change_hook();
        adapter
    }

    pub fn view(&self) -> &sourceview5::View {
        &self.view
    }
    pub fn buffer(&self) -> &sourceview5::Buffer {
        &self.buffer
    }

    /// `textView.string` — authoritative text.
    pub fn text(&self) -> String {
        let (start, end) = self.buffer.bounds();
        self.buffer.text(&start, &end, false).to_string()
    }

    /// `textView.selectedRange()` in UTF-16 units.
    pub fn selected_range(&self) -> EditorTextRange {
        let text = self.text();
        let (start, end) = self
            .buffer
            .selection_bounds()
            .unwrap_or_else(|| {
                let cursor = self.buffer.iter_at_mark(&self.buffer.get_insert());
                (cursor, cursor)
            });
        let location = char_offset_to_utf16(&text, start.offset() as usize);
        let end_utf16 = char_offset_to_utf16(&text, end.offset() as usize);
        EditorTextRange {
            location: location as i64,
            length: (end_utf16 - location) as i64,
        }
    }

    /// `textView.hasMarkedText()` — true while the input method has
    /// uncommitted preedit text (tracked from `preedit-changed`).
    pub fn has_marked_text(&self) -> bool {
        self.shared.has_preedit.get()
    }

    /// `revealSelection(_:highlight:)` — clamp like the Swift adapter, scroll
    /// and focus, then run the find indicator: a flash over the target line
    /// when `highlight` is on, or clearing an earlier indicator when off.
    pub fn reveal_selection(&self, range: EditorTextRange, highlight: bool) {
        let text = self.text();
        let count: usize = text.encode_utf16().count();
        let location = (range.location.max(0) as usize).min(count);
        let length = (range.length.max(0) as usize).min(count - location);
        let start_char = utf16_offset_to_char_offset(&text, location);
        let end_char = utf16_offset_to_char_offset(&text, location + length);

        let mut start = self.buffer.iter_at_offset(start_char as i32);
        let end = self.buffer.iter_at_offset(end_char as i32);
        self.buffer.select_range(&start, &end);
        self.view.scroll_to_iter(&mut start, 0.0, false, 0.0, 0.0);
        self.view.grab_focus();
        self.show_find_indicator(&start, highlight);
    }

    /// `showFindIndicator(for:)` — the native indicator flashes the target
    /// line's contents briefly; a disabled highlight passes a zero-length
    /// range that only clears the previous indicator.
    fn show_find_indicator(&self, at: &gtk4::TextIter, enabled: bool) {
        let tag = self.find_indicator_tag();
        let generation = self.shared.find_indicator_generation.get() + 1;
        self.shared.find_indicator_generation.set(generation);
        self.buffer
            .remove_tag(&tag, &self.buffer.start_iter(), &self.buffer.end_iter());
        if !enabled {
            return;
        }
        let mut start = at.clone();
        start.set_line_offset(0);
        let mut end = start.clone();
        if !end.ends_line() {
            end.forward_to_line_end();
        }
        self.buffer.apply_tag(&tag, &start, &end);
        let (buffer, shared) = (self.buffer.clone(), self.shared.clone());
        gtk4::glib::timeout_add_local_once(std::time::Duration::from_millis(1500), move || {
            if shared.find_indicator_generation.get() != generation {
                return;
            }
            buffer.remove_tag(&tag, &buffer.start_iter(), &buffer.end_iter());
        });
    }

    /// Lazily created tag — the yellow marker the PDF side draws at 35%
    /// alpha; text tags take an opaque color so it is pre-blended on white.
    fn find_indicator_tag(&self) -> gtk4::TextTag {
        const NAME: &str = "pitex-find-indicator";
        if let Some(tag) = self.buffer.tag_table().lookup(NAME) {
            return tag;
        }
        let tag = gtk4::TextTag::new(Some(NAME));
        tag.set_background(Some("#fbe9a6"));
        self.buffer.tag_table().add(&tag);
        tag
    }

    /// `refreshFromSession()` — pull the latest snapshot and apply it.
    pub fn refresh_from_session(&self) {
        let snapshot = self.session.snapshot();
        self.apply(snapshot);
    }

    fn install_change_hook(&self) {
        let session = self.session.clone();
        let shared = self.shared.clone();
        self.buffer.connect_changed(move |buffer| {
            if shared.suppress_change.get() {
                return;
            }
            let (start, end) = buffer.bounds();
            let text = buffer.text(&start, &end, false).to_string();
            *shared.desired_text.borrow_mut() = text;
            Self::submit_pending_change(session.as_ref(), shared.clone(), buffer);
        });
    }

    /// `submitPendingChangeIfNeeded` — same full-text replace mutation the
    /// Swift adapter issues, guarded by `isSubmitting` and the desired/committed
    /// text comparison.
    fn submit_pending_change(
        session: &dyn SessionClient,
        shared: Rc<Shared>,
        buffer: &sourceview5::Buffer,
    ) {
        if shared.is_submitting.get()
            || *shared.desired_text.borrow() == shared.committed.borrow().text
        {
            return;
        }
        shared.is_submitting.set(true);
        let submitted_text = shared.desired_text.borrow().clone();
        let mutation = DocumentMutation {
            base_revision: shared.committed.borrow().revision,
            range: DocumentTextRange {
                location: 0,
                length: shared.committed.borrow().text.encode_utf16().count(),
            },
            replacement: submitted_text.clone(),
        };
        let result = session.submit(&mutation);
        match result {
            DocumentMutationResult::Applied(snapshot) => {
                let was_same = *shared.desired_text.borrow() == submitted_text;
                shared.committed.replace(snapshot.clone());
                if was_same && snapshot.text != submitted_text {
                    Self::apply_snapshot(shared.clone(), buffer, snapshot);
                }
            }
            DocumentMutationResult::Rejected { current } => {
                Self::apply_snapshot(shared.clone(), buffer, current);
            }
        }
        shared.is_submitting.set(false);
        Self::submit_pending_change(session, shared, buffer);
    }

    /// `apply(_:)` for the submit path — pushes the session snapshot into the
    /// buffer with the change hook suppressed, preserving a clamped selection.
    fn apply_snapshot(
        shared: Rc<Shared>,
        buffer: &sourceview5::Buffer,
        snapshot: DocumentSnapshot,
    ) {
        let (s, e) = buffer.bounds();
        let current = buffer.text(&s, &e, false).to_string();
        shared.committed.replace(snapshot.clone());
        *shared.desired_text.borrow_mut() = snapshot.text.clone();
        if current == snapshot.text {
            return;
        }
        shared.suppress_change.set(true);
        buffer.set_text(&snapshot.text);
        shared.suppress_change.set(false);
    }

    /// `apply(_:)` — push a snapshot into the buffer, preserving a clamped
    /// selection like the Swift implementation.
    pub fn apply(&self, snapshot: DocumentSnapshot) {
        let text_before = self.text();
        self.shared.committed.replace(snapshot.clone());
        *self.shared.desired_text.borrow_mut() = snapshot.text.clone();
        if text_before == snapshot.text {
            return;
        }
        let previous = self.selected_range();
        self.shared.suppress_change.set(true);
        self.buffer.set_text(&snapshot.text);
        let maximum = snapshot.text.encode_utf16().count() as i64;
        let location = previous.location.min(maximum);
        let length = previous.length.min(maximum - location);
        let start_char = utf16_offset_to_char_offset(&snapshot.text, location as usize);
        let end_char =
            utf16_offset_to_char_offset(&snapshot.text, (location + length) as usize);
        let start = self.buffer.iter_at_offset(start_char as i32);
        let end = self.buffer.iter_at_offset(end_char as i32);
        self.buffer.select_range(&start, &end);
        self.shared.suppress_change.set(false);
    }

    /// Session-owned undo, matching `sessionUndoManager` + `allowsUndo`.
    pub fn undo(&self) {
        if self.buffer.can_undo() {
            self.buffer.undo();
        }
    }
    pub fn redo(&self) {
        if self.buffer.can_redo() {
            self.buffer.redo();
        }
    }

    /// Applies syntax decorations as buffer `TextTag`s keyed by token kind.
    /// Range inputs are UTF-8 (language-core `SourceRange`), converted to char
    /// offsets through the buffer text.
    pub fn apply_decorations(&self, snapshot: &EditorDecorationSnapshot) {
        if snapshot.document_revision != self.shared.committed.borrow().revision {
            return; // stale decoration revision — dropped, matching checked()
        }
        let table = self.buffer.tag_table();
        let text = self.text();
        // Clear previously applied decoration tags so stale ranges don't
        // keep their color after edits shift the token boundaries.
        let mut stale = Vec::new();
        table.foreach(|tag| {
            if let Some(name) = tag.name() {
                if name.starts_with("pitex.decoration.") {
                    stale.push(tag.clone());
                }
            }
        });
        let (buf_start, buf_end) = self.buffer.bounds();
        for tag in stale {
            self.buffer.remove_tag(&tag, &buf_start, &buf_end);
        }
        for decoration in &snapshot.decorations {
            let name = editor_feature::decoration_tag_name(&decoration.token_kind);
            if table.lookup(&name).is_none() {
                let tag = gtk4::TextTag::new(Some(&name));
                table.add(&tag);
            }
            let tag = table.lookup(&name).unwrap();
            let start_byte = decoration.range.utf8_offset.max(0) as usize;
            let end_byte = ((decoration.range.utf8_offset + decoration.range.utf8_length)
                .max(0) as usize)
                .min(text.len());
            if start_byte >= end_byte {
                continue;
            }
            // `String.Index(_, within:)` parity: a range that doesn't land on
            // scalar boundaries is dropped, never sliced mid-character.
            if !text.is_char_boundary(start_byte) || !text.is_char_boundary(end_byte) {
                continue;
            }
            let start_char = text[..start_byte].chars().count();
            let end_char = text[..end_byte].chars().count();
            let start = self.buffer.iter_at_offset(start_char as i32);
            let end = self.buffer.iter_at_offset(end_char as i32);
            self.buffer.apply_tag(&tag, &start, &end);
        }
    }
}
