//! Port of `EquationPreviewTypes.swift` + `EquationPreviewEngine.swift` —
//! the pure, deterministic state machine behind the Overleaf-style equation
//! preview. No threads, timers or I/O: the host feeds editor events with
//! millisecond timestamps, arms one timer for [`EquationPreviewEngine::next_deadline`]
//! and calls [`EquationPreviewEngine::poll`] when it fires. Every entry
//! point returns the [`EquationPreviewCommand`]s the host performs.
//!
//! Host units are Unicode scalar offsets (GTK TextIter counts chars), where
//! the Swift original uses UTF-16 NSRange — `Snapshot` converts.

use language_core::equation_preview::{
    ExactEquationDocument, MathIncludeKey, MathIncludeRequest, MathPreviewHash, MathProjectContext,
    MathRegion, MathSourceScan, MathSourceScanner,
};
use language_core::SourceRange;
use std::collections::{HashMap, HashSet};
use std::ops::Range;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EquationPreviewPlacement {
    Above,
    Below,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EquationPreviewRendererMode {
    Fast,
    FastWithTexFallback,
}

/// `pitex.pref.equationPreview.*` as the engine consumes them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EquationPreviewSettings {
    pub enabled: bool,
    pub while_typing: bool,
    pub placement: EquationPreviewPlacement,
    pub renderer: EquationPreviewRendererMode,
    pub delay_milliseconds: u64,
}

impl EquationPreviewSettings {
    pub const ALLOWED_DELAYS: [u64; 3] = [0, 80, 150];

    pub fn new(
        enabled: bool,
        while_typing: bool,
        placement: EquationPreviewPlacement,
        renderer: EquationPreviewRendererMode,
        delay_milliseconds: u64,
    ) -> Self {
        Self {
            enabled,
            while_typing,
            placement,
            renderer,
            delay_milliseconds: if Self::ALLOWED_DELAYS.contains(&delay_milliseconds) {
                delay_milliseconds
            } else {
                80
            },
        }
    }

    /// From the persisted strings; unknown values fall back to defaults.
    pub fn from_persisted(
        enabled: bool,
        while_typing: bool,
        placement: &str,
        renderer: &str,
        delay_milliseconds: i64,
    ) -> Self {
        Self::new(
            enabled,
            while_typing,
            match placement {
                "below" => EquationPreviewPlacement::Below,
                _ => EquationPreviewPlacement::Above,
            },
            match renderer {
                "fastWithTeXFallback" => EquationPreviewRendererMode::FastWithTexFallback,
                _ => EquationPreviewRendererMode::Fast,
            },
            // Negative persisted values are invalid, not "instant": the
            // conservative default is 80, while a stored 0 stays 0 (C13).
            if delay_milliseconds < 0 { 80 } else { delay_milliseconds as u64 },
        )
    }
}

impl Default for EquationPreviewSettings {
    fn default() -> Self {
        Self::new(true, true, EquationPreviewPlacement::Above, EquationPreviewRendererMode::Fast, 80)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EquationPreviewScheme {
    Light,
    Dark,
    HighContrastLight,
    HighContrastDark,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EquationPreviewAppearance {
    pub scheme: EquationPreviewScheme,
    /// Editor font size in points; the renderer derives em/ex metrics.
    pub font_size: f64,
}

impl Eq for EquationPreviewAppearance {}

impl std::hash::Hash for EquationPreviewAppearance {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.scheme.hash(state);
        self.font_size.to_bits().hash(state);
    }
}

impl Default for EquationPreviewAppearance {
    fn default() -> Self {
        Self { scheme: EquationPreviewScheme::Light, font_size: 13.0 }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EquationPreviewTrigger {
    Caret,
    Hover,
}

/// Fast-preview cache identity — never the caret position.
#[derive(Debug, Clone, PartialEq)]
pub struct EquationPreviewKey {
    pub source: String,
    pub display_mode: bool,
    pub context_key: String,
    pub renderer_version: String,
    /// Only appearance inputs that alter the rendered SVG belong here:
    /// font_size changes metrics; the color scheme is applied by the host
    /// when presenting, so a theme change must re-present, not re-typeset.
    pub font_size: f64,
}

// f64 is !Eq/!Hash: identity by bits, same convention as
// EquationPreviewAppearance's manual impl.
impl Eq for EquationPreviewKey {}
impl std::hash::Hash for EquationPreviewKey {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.source.hash(state);
        self.display_mode.hash(state);
        self.context_key.hash(state);
        self.renderer_version.hash(state);
        self.font_size.to_bits().hash(state);
    }
}

impl EquationPreviewKey {
    /// Runs of spaces/tabs are one TeX space; line breaks stay because a
    /// `%` comment ends at them. Byte-wise so a UTF-8 `"\r\n"` pair is one
    /// line break like Swift's grapheme-cluster iteration.
    pub fn normalize(source: &str) -> String {
        let mut result = String::with_capacity(source.len());
        let mut pending_space = false;
        let bytes = source.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            let b = bytes[i];
            if b == b' ' || b == b'\t' {
                pending_space = true;
                i += 1;
                continue;
            }
            let is_break = b == b'\n' || (b == b'\r' && bytes.get(i + 1) == Some(&b'\n'));
            if pending_space && !result.is_empty() && !is_break {
                result.push(' ');
            }
            pending_space = false;
            if is_break && b == b'\r' {
                result.push('\r');
                result.push('\n');
                i += 2;
            } else {
                result.push_str(&source[i..i + utf8_len(b)]);
                i += utf8_len(b);
            }
        }
        result.trim_matches(|c| matches!(c, '\n' | '\r' | ' ')).to_string()
    }
}

fn utf8_len(first: u8) -> usize {
    if first < 0x80 {
        1
    } else if first >> 5 == 0b110 {
        2
    } else if first >> 4 == 0b1110 {
        3
    } else {
        4
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EquationRenderRequest {
    pub generation: u64,
    pub key: EquationPreviewKey,
    /// Structured JSON arguments for the renderer page — never spliced into
    /// script text.
    pub source: String,
    pub display_mode: bool,
    pub definitions: Vec<String>,
    pub font_size: f64,
}

impl EquationRenderRequest {
    pub fn context_key(&self) -> &str {
        &self.key.context_key
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EquationRenderFailure {
    /// Unknown macro or environment — fast preview cannot know it.
    UndefinedCommand,
    /// Syntax error (typically an unfinished edit).
    Invalid,
    /// Exceeds the source/expansion bounds.
    TooLarge,
    /// The page failed (timeout, crashed process); not cached.
    RendererFailed,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EquationRenderOutcome {
    Svg(String),
    Failed(EquationRenderFailure),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ExactEquationRequest {
    pub generation: u64,
    pub key: String,
    /// Complete TeX document; the host compiles it with the project's
    /// engine in an isolated temporary directory.
    pub document: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExactEquationOutcome {
    Pdf(Vec<u8>),
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EquationPreviewUnavailableReason {
    /// A command/environment the fast renderer does not know.
    Unsupported,
    TooLarge,
    /// Exact fallback requested but the build command names no supported engine.
    ExactUnavailable,
    ExactFailed,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EquationPreviewContent {
    Fast { svg: String },
    Exact { pdf: Vec<u8> },
    Unavailable(EquationPreviewUnavailableReason),
}

#[derive(Debug, Clone, PartialEq)]
pub struct EquationPreviewPresentation {
    pub generation: u64,
    pub content: EquationPreviewContent,
    /// Region in host text units (char offsets on GTK), for anchoring.
    pub anchor: Range<usize>,
    pub anchor_utf8: SourceRange,
    pub display_mode: bool,
    pub placement: EquationPreviewPlacement,
    pub trigger: EquationPreviewTrigger,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EquationPreviewCommand {
    Render(EquationRenderRequest),
    RenderExact(ExactEquationRequest),
    /// Kill the exact job for this key; its completion is still reported.
    CancelExact { key: String },
    Show(EquationPreviewPresentation),
    Hide,
    ResolveIncludes(Vec<MathIncludeRequest>),
}

/// Bounded LRU by entry count and total cost.
struct Lru<K: Eq + std::hash::Hash, V> {
    entries: HashMap<K, (V, usize, u64)>, // value, cost, stamp
    clock: u64,
    total_cost: usize,
    capacity: usize,
    byte_budget: usize,
}

impl<K: Eq + std::hash::Hash + Clone, V> Lru<K, V> {
    fn new(capacity: usize, byte_budget: usize) -> Self {
        Self { entries: HashMap::new(), clock: 0, total_cost: 0, capacity, byte_budget }
    }

    fn contains(&self, key: &K) -> bool {
        self.entries.contains_key(key)
    }

    fn value(&mut self, key: &K) -> Option<&V> {
        let entry = self.entries.get_mut(key)?;
        self.clock += 1;
        entry.2 = self.clock;
        Some(&entry.0)
    }

    fn insert(&mut self, key: K, value: V, cost: usize) {
        if cost > self.byte_budget {
            return;
        }
        if let Some(old) = self.entries.get(&key) {
            self.total_cost -= old.1;
        }
        self.clock += 1;
        self.entries.insert(key, (value, cost, self.clock));
        self.total_cost += cost;
        while self.entries.len() > self.capacity || self.total_cost > self.byte_budget {
            // ponytail: O(n) eviction scan over <=256 entries; a linked list if capacity grows.
            let Some((oldest_key, _)) = self.entries.iter().min_by_key(|(_, e)| e.2) else {
                break;
            };
            let oldest_key = oldest_key.clone();
            let (_, cost, _) = self.entries.remove(&oldest_key).unwrap();
            self.total_cost -= cost;
        }
    }

    fn remove_all(&mut self) {
        self.entries.clear();
        self.total_cost = 0;
    }
}

struct Snapshot {
    revision: u64,
    file_id: String,
    text: String,
    scan: MathSourceScan,
    /// Char boundaries for host-unit conversion (Unicode scalars on GTK).
    char_boundaries: Vec<usize>,
}

impl Snapshot {
    /// Host char offset -> UTF-8 byte offset (clamped).
    fn utf8_for_char(&self, offset: usize) -> usize {
        self.char_boundaries
            .get(offset.min(self.char_boundaries.len().saturating_sub(1)))
            .copied()
            .unwrap_or(self.text.len())
    }

    /// UTF-8 byte offset -> host char offset; a mid-scalar byte rounds down
    /// to its scalar's index (Swift drops a dangling surrogate half).
    fn char_for_utf8(&self, offset: usize) -> usize {
        match self.char_boundaries.binary_search(&offset) {
            Ok(i) => i,
            Err(i) => i - 1,
        }
    }
}

#[derive(Clone)]
struct Target {
    generation: u64,
    region: MathRegion,
    anchor: Range<usize>,
    trigger: EquationPreviewTrigger,
    key: Option<EquationPreviewKey>,
    /// Arc so a cloned Target (the failure path) does not deep-copy the
    /// generated .tex document (C11).
    exact: Option<Arc<ExactPlan>>,
    failure: Option<EquationRenderFailure>,
}

#[derive(Clone)]
struct ExactPlan {
    key: String,
    document: String,
}

struct Visible {
    region_start: i64,
    #[allow(dead_code)]
    content: EquationPreviewContent,
}

pub struct EquationPreviewEngine {
    settings: EquationPreviewSettings,
    appearance: EquationPreviewAppearance,
    /// Renderer identity (MathJax + font + page revision) once the page is
    /// ready; renders wait for it, and a change empties the cache.
    renderer_version: Option<String>,
    renderer_failed: bool,
    /// Exact renderer identity (engine + shell-escape policy); None when the
    /// project build command names no supported TeX engine.
    exact_profile: Option<String>,

    file_id: Option<String>,
    root_file_id: Option<String>,
    text_revision: u64,
    snapshot: Option<Arc<Snapshot>>,
    external_scans: HashMap<String, MathSourceScan>,
    resolutions: HashMap<MathIncludeKey, Option<String>>,
    pending_resolutions: HashSet<MathIncludeKey>,
    context_version: u64,
    project_context: Option<(u64, u64, Arc<MathProjectContext>)>,

    focused: bool,
    composing: bool,
    selection: (usize, usize),
    selection_revision: Option<u64>,
    suppressed_by_typing: bool,
    hover_location: Option<usize>,
    pointer_in_popover: bool,
    dismissed_region_start: Option<i64>,
    explicit_exact_region_start: Option<i64>,

    generation: u64,
    fire_deadline: Option<u64>,
    keep_deadline: Option<u64>,
    hover_grace_deadline: Option<u64>,
    exact_deadline: Option<u64>,

    target: Option<Target>,
    visible: Option<Visible>,
    in_flight: Option<(u64, EquationPreviewKey)>,
    queued: Option<EquationRenderRequest>,
    exact_in_flight: Option<(u64, String)>,
    cache: Lru<EquationPreviewKey, String>,
    failures: Lru<EquationPreviewKey, EquationRenderFailure>,
    exact_cache: Lru<String, Vec<u8>>,
}

impl EquationPreviewEngine {
    pub const KEEP_LAST_GOOD_MS: u64 = 300;
    pub const HOVER_EXIT_GRACE_MS: u64 = 250;
    pub const EXACT_SETTLE_MS: u64 = 600;
    pub const CARET_DELAY_CAP_MS: u64 = 30;
    pub const MAXIMUM_SOURCE_BYTES: usize = 32 * 1024;
    pub const CACHE_CAPACITY: usize = 256;
    pub const CACHE_BYTE_BUDGET: usize = 16 * 1024 * 1024;
    pub const EXACT_CACHE_CAPACITY: usize = 16;
    pub const FAILURE_CACHE_CAPACITY: usize = 128;

    pub fn new(settings: EquationPreviewSettings, appearance: EquationPreviewAppearance) -> Self {
        Self {
            settings,
            appearance,
            renderer_version: None,
            renderer_failed: false,
            exact_profile: None,
            file_id: None,
            root_file_id: None,
            text_revision: 0,
            snapshot: None,
            external_scans: HashMap::new(),
            resolutions: HashMap::new(),
            pending_resolutions: HashSet::new(),
            context_version: 0,
            project_context: None,
            focused: false,
            composing: false,
            selection: (0, 0),
            selection_revision: None,
            suppressed_by_typing: false,
            hover_location: None,
            pointer_in_popover: false,
            dismissed_region_start: None,
            explicit_exact_region_start: None,
            generation: 0,
            fire_deadline: None,
            keep_deadline: None,
            hover_grace_deadline: None,
            exact_deadline: None,
            target: None,
            visible: None,
            in_flight: None,
            queued: None,
            exact_in_flight: None,
            cache: Lru::new(Self::CACHE_CAPACITY, Self::CACHE_BYTE_BUDGET),
            failures: Lru::new(Self::FAILURE_CACHE_CAPACITY, usize::MAX),
            exact_cache: Lru::new(Self::EXACT_CACHE_CAPACITY, 8 * 1024 * 1024),
        }
    }

    pub fn settings(&self) -> &EquationPreviewSettings {
        &self.settings
    }

    pub fn appearance(&self) -> &EquationPreviewAppearance {
        &self.appearance
    }

    pub fn renderer_version(&self) -> Option<&str> {
        self.renderer_version.as_deref()
    }

    pub fn renderer_failed(&self) -> bool {
        self.renderer_failed
    }

    pub fn exact_profile(&self) -> Option<&str> {
        self.exact_profile.as_deref()
    }

    /// Earliest instant `poll` must run.
    pub fn next_deadline(&self) -> Option<u64> {
        [self.fire_deadline, self.keep_deadline, self.hover_grace_deadline, self.exact_deadline]
            .into_iter()
            .flatten()
            .min()
    }

    /// Something is on screen (the host consumes Escape only then).
    pub fn is_visible(&self) -> bool {
        self.visible.is_some()
    }

    /// Files the context depends on besides the active one — the host
    /// re-checks them (off the main thread) after saves or app activation.
    pub fn external_file_ids(&self) -> Vec<String> {
        let mut ids: Vec<String> = self.external_scans.keys().cloned().collect();
        ids.sort();
        ids
    }

    // MARK: Configuration

    pub fn set_settings(
        &mut self,
        settings: EquationPreviewSettings,
        now_ms: u64,
    ) -> Vec<EquationPreviewCommand> {
        if settings == self.settings {
            return Vec::new();
        }
        let mode_changed = settings.renderer != self.settings.renderer;
        self.settings = settings;
        if !settings.enabled {
            return self.retire();
        }
        if mode_changed {
            self.failures.remove_all();
            // Only the transition TO Fast kills exact work (C8):
            // Fast+TeX->Fast leaves a running job whose result could still
            // present; Fast->Fast+TeX keeps it useful.
            if settings.renderer == EquationPreviewRendererMode::Fast {
                self.explicit_exact_region_start = None;
                self.exact_deadline = None;
                let commands = self.cancel_exact();
                self.schedule(now_ms);
                return commands;
            }
        }
        self.schedule(now_ms);
        Vec::new()
    }

    pub fn set_appearance(
        &mut self,
        appearance: EquationPreviewAppearance,
        now_ms: u64,
    ) -> Vec<EquationPreviewCommand> {
        if appearance == self.appearance {
            return Vec::new();
        }
        self.appearance = appearance;
        self.schedule(now_ms);
        Vec::new()
    }

    pub fn renderer_ready(&mut self, version: &str, now_ms: u64) -> Vec<EquationPreviewCommand> {
        if self.renderer_version.as_deref() != Some(version) {
            self.cache.remove_all();
            self.failures.remove_all();
        }
        self.renderer_version = Some(version.to_string());
        self.renderer_failed = false;
        self.schedule(now_ms);
        Vec::new()
    }

    /// The renderer page could not load (or its process died); fast previews
    /// stop until the host reports a fresh `renderer_ready`.
    pub fn renderer_unavailable(&mut self) -> Vec<EquationPreviewCommand> {
        self.renderer_version = None;
        self.renderer_failed = true;
        self.in_flight = None;
        self.queued = None;
        self.retire()
    }

    pub fn set_exact_profile(
        &mut self,
        profile: Option<String>,
        now_ms: u64,
    ) -> Vec<EquationPreviewCommand> {
        if profile == self.exact_profile {
            return Vec::new();
        }
        self.exact_profile = profile;
        self.exact_cache.remove_all();
        self.failures.remove_all();
        let commands = self.cancel_exact();
        self.schedule(now_ms);
        commands
    }

    // MARK: Document

    /// Document switch (or first open). The outgoing file's last scan keeps
    /// serving as its context until the host reports a fresher one.
    pub fn open_document(
        &mut self,
        file_id: Option<String>,
        revision: u64,
        now_ms: u64,
    ) -> Vec<EquationPreviewCommand> {
        if let Some(snapshot) = &self.snapshot {
            if Some(snapshot.file_id.as_str()) != file_id.as_deref() {
                self.external_scans.insert(snapshot.file_id.clone(), snapshot.scan.clone());
            }
        }
        if let Some(id) = &file_id {
            self.external_scans.remove(id.as_str());
        }
        self.context_version += 1;
        self.file_id = file_id;
        self.text_revision = revision;
        self.snapshot = None;
        self.hover_location = None;
        self.selection_revision = None;
        self.suppressed_by_typing = false;
        self.dismissed_region_start = None;
        self.explicit_exact_region_start = None;
        let commands = self.retire();
        self.schedule(now_ms);
        commands
    }

    /// The main document the context and exact preamble start from; None
    /// uses the active file as its own root.
    pub fn set_root_file(
        &mut self,
        root_file_id: Option<String>,
        scan: Option<MathSourceScan>,
        now_ms: u64,
    ) -> Vec<EquationPreviewCommand> {
        if let Some(root) = &root_file_id {
            if Some(root.as_str()) != self.file_id.as_deref() {
                match scan.clone() {
                    Some(scan) => {
                        self.external_scans.insert(root.clone(), scan);
                    }
                    None => {
                        self.external_scans.remove(root.as_str());
                    }
                }
            }
        }
        if root_file_id == self.root_file_id && scan.is_none() {
            return Vec::new();
        }
        self.root_file_id = root_file_id;
        self.context_version += 1;
        // A changed root/preamble changes exact-render semantics without
        // changing any document text or cache key: stale PDFs must go.
        self.exact_cache.remove_all();
        self.schedule(now_ms);
        Vec::new()
    }

    pub fn text_changed(&mut self, revision: u64, now_ms: u64) -> Vec<EquationPreviewCommand> {
        self.text_revision = revision;
        self.dismissed_region_start = None;
        self.explicit_exact_region_start = None;
        self.exact_deadline = None;
        // A stale hover anchor must not survive the edit: the character
        // index now points at different text, and a hover-target preview
        // would render a region the pointer no longer selects.
        self.hover_location = None;
        self.hover_grace_deadline = None;
        if self.target.as_ref().map(|t| t.trigger) == Some(EquationPreviewTrigger::Hover) {
            self.target = None;
        }
        let mut commands = Vec::new();
        if !self.settings.while_typing {
            self.suppressed_by_typing = true;
            commands.append(&mut self.retire());
        }
        self.schedule(now_ms + self.settings.delay_milliseconds);
        commands
    }

    /// Hosts report the post-edit caret with the edit's new revision (before
    /// or after `text_changed`); only a selection change at an unchanged
    /// revision counts as navigation and ends typing suppression.
    pub fn selection_changed(
        &mut self,
        location: usize,
        length: usize,
        revision: u64,
        now_ms: u64,
    ) -> Vec<EquationPreviewCommand> {
        let navigation = self.selection_revision == Some(revision) && revision == self.text_revision;
        self.selection_revision = Some(revision);
        let resumed = navigation && self.suppressed_by_typing;
        if navigation {
            self.suppressed_by_typing = false;
        }
        if (location, length) == self.selection && !resumed {
            return Vec::new();
        }
        self.selection = (location, length);
        // Never shorten a pending typing debounce.
        let pending = self.fire_deadline.unwrap_or(0);
        self.schedule(
            pending.max(now_ms + self.settings.delay_milliseconds.min(Self::CARET_DELAY_CAP_MS)),
        );
        Vec::new()
    }

    // MARK: Pointer / focus

    /// The character under the pointer, or None when the pointer is outside
    /// the text (or a button is held). Hover never moves the caret.
    pub fn hover(&mut self, location: Option<usize>, now_ms: u64) -> Vec<EquationPreviewCommand> {
        if let Some(location) = location {
            // Grace is NOT cleared here: it counts down from the actual
            // exit so motions over plain text cannot stretch it
            // indefinitely (C6). fire() clears it on a math region hit.
            if Some(location) == self.hover_location {
                return Vec::new();
            }
            self.hover_location = Some(location);
            self.schedule(now_ms + self.settings.delay_milliseconds);
        } else if self.target.as_ref().map(|t| t.trigger) == Some(EquationPreviewTrigger::Hover) {
            // Pointer left the text entirely: the position is gone, so the
            // stale index must not be consulted at re-fire — the grace
            // deadline alone protects the visible popover (C6 r2).
            self.hover_location = None;
            if self.hover_grace_deadline.is_none() {
                self.hover_grace_deadline = Some(now_ms + Self::HOVER_EXIT_GRACE_MS);
            }
        } else if self.hover_location.is_some() {
            self.hover_location = None;
            self.schedule(now_ms);
        }
        Vec::new()
    }

    pub fn pointer_in_popover(&mut self, inside: bool, now_ms: u64) -> Vec<EquationPreviewCommand> {
        self.pointer_in_popover = inside;
        if inside {
            self.hover_grace_deadline = None;
        } else if self.target.as_ref().map(|t| t.trigger) == Some(EquationPreviewTrigger::Hover) {
            self.hover_grace_deadline = Some(now_ms + Self::HOVER_EXIT_GRACE_MS);
        } else {
            // Leaving the popover for a non-editor area must re-evaluate:
            // a caret-triggered target with no hover_location has no other
            // event that would ever hide it (C7).
            self.schedule(now_ms);
        }
        Vec::new()
    }

    pub fn focus_changed(&mut self, focused: bool, now_ms: u64) -> Vec<EquationPreviewCommand> {
        if focused == self.focused {
            return Vec::new();
        }
        self.focused = focused;
        if !focused {
            self.hover_location = None;
            self.pointer_in_popover = false;
            self.hover_grace_deadline = None;
            return self.retire();
        }
        self.schedule(now_ms);
        Vec::new()
    }

    /// IME marked text: previews stay down until composition ends.
    pub fn composition_changed(&mut self, composing: bool, now_ms: u64) -> Vec<EquationPreviewCommand> {
        if composing == self.composing {
            return Vec::new();
        }
        self.composing = composing;
        if composing {
            return self.retire();
        }
        self.schedule(now_ms + self.settings.delay_milliseconds);
        Vec::new()
    }

    /// Escape: dismisses a visible preview, or a pending-but-eligible one
    /// (scheduled fire resolving to math, or a live render request).
    /// `consumed` tells the host whether to swallow the key; it is false
    /// for dormant/failed targets, pending fires on plain text, and any
    /// stale work — none of which may be poisoned by a dismissal (N5).
    pub fn escape(&mut self, _now_ms: u64) -> (bool, Vec<EquationPreviewCommand>) {
        // Pending fire is eligible only when it would resolve to a math
        // region. The answer needs the *current* scan; with a stale or
        // absent snapshot we decline rather than poison a dormant target.
        let pending_region: Option<i64> = if self.fire_deadline.is_some()
            && self.settings.enabled
            && !self.composing
            && !(self.renderer_version.is_none() && self.renderer_failed)
        {
            self.snapshot
                .as_ref()
                .filter(|s| s.revision == self.text_revision && Some(s.file_id.as_str()) == self.file_id.as_deref())
                .and_then(|s| {
                    let mut region: Option<&MathRegion> = None;
                    if let Some(hover_location) = self.hover_location {
                        region = s.scan.region_at_character(s.utf8_for_char(hover_location));
                    }
                    if region.is_none()
                        && self.focused
                        && self.selection.1 == 0
                        && !self.suppressed_by_typing
                    {
                        region = s.scan.region_at_caret(s.utf8_for_char(self.selection.0));
                    }
                    region.map(|r| r.range.utf8_offset)
                })
        } else {
            None
        };
        // A pending fire already dismissed once resolves to nothing: the
        // dismissal check in fire() runs before any render, so Escape
        // must fall through rather than consume with no effect.
        let pending_region =
            pending_region.filter(|start| Some(*start) != self.dismissed_region_start);
        // Deadline gone but work pending: an in-flight job can still
        // publish at this generation (completion steals the reply only
        // when generations diverge).
        let live_work = self
            .in_flight
            .as_ref()
            .map_or(false, |(g, _)| *g == self.generation)
            || self.exact_in_flight.as_ref().map_or(false, |(g, _)| *g == self.generation);
        let consumed = self.visible.is_some() || pending_region.is_some() || live_work;
        if !consumed {
            return (false, Vec::new());
        }
        // Suppress the region the pending fire would actually hit — not a
        // stale target left over from an earlier caret position.
        self.dismissed_region_start = pending_region
            .or_else(|| self.target.as_ref().map(|t| t.region.range.utf8_offset))
            .or_else(|| self.visible.as_ref().map(|v| v.region_start));
        (true, self.retire())
    }

    /// Explicit "exact TeX preview" for the current equation — works in
    /// either renderer mode, once per request.
    pub fn request_exact(&mut self, now_ms: u64) -> Vec<EquationPreviewCommand> {
        let Some(region_start) = self.target.as_ref().map(|t| t.region.range.utf8_offset) else {
            return Vec::new();
        };
        self.explicit_exact_region_start = Some(region_start);
        self.schedule(now_ms);
        Vec::new()
    }

    // MARK: Project context

    /// Answers `.resolve_includes`: `file_id == None` marks a missing target.
    pub fn includes_resolved(
        &mut self,
        results: Vec<(MathIncludeKey, Option<String>)>,
        scans: HashMap<String, MathSourceScan>,
        now_ms: u64,
    ) -> Vec<EquationPreviewCommand> {
        let mut accepted = false;
        for (key, file_id) in results {
            // Only keys we asked for land in resolutions: a stale reply
            // from a previous context must not satisfy this one (B7).
            if self.pending_resolutions.remove(&key) {
                self.resolutions.insert(key, file_id);
                accepted = true;
            }
        }
        for (id, scan) in scans {
            if Some(id.as_str()) != self.file_id.as_deref() {
                self.external_scans.insert(id, scan);
                accepted = true;
            }
        }
        // A wholly stale reply (every key already pruned) changes nothing:
        // no context bump, no cache flush, no refire (B7).
        if !accepted {
            return Vec::new();
        }
        self.context_version += 1;
        // A changed include (new preamble/definitions, or a removed file)
        // changes exact-render semantics without changing cache keys.
        self.exact_cache.remove_all();
        self.schedule(now_ms);
        Vec::new()
    }

    /// A project file changed on disk (or an unsaved buffer was left);
    /// None drops it. Resolutions are re-checked lazily on the next miss.
    pub fn external_file_changed(
        &mut self,
        changed_file_id: &str,
        scan: Option<MathSourceScan>,
        now_ms: u64,
    ) -> Vec<EquationPreviewCommand> {
        if Some(changed_file_id) == self.file_id.as_deref()
            || self.external_scans.get(changed_file_id) == scan.as_ref()
        {
            return Vec::new();
        }
        if scan.is_none() {
            self.external_scans.remove(changed_file_id);
            let stale: Vec<MathIncludeKey> = self
                .resolutions
                .iter()
                .filter(|(_, v)| v.as_deref() == Some(changed_file_id))
                .map(|(k, _)| k.clone())
                .collect();
            for key in stale {
                self.resolutions.remove(&key);
            }
        } else {
            self.external_scans.insert(changed_file_id.to_string(), scan.unwrap());
        }
        self.exact_cache.remove_all();
        self.context_version += 1;
        self.schedule(now_ms);
        Vec::new()
    }

    // MARK: Renderer results

    pub fn fast_render_completed(
        &mut self,
        key: &EquationPreviewKey,
        outcome: EquationRenderOutcome,
        now_ms: u64,
    ) -> Vec<EquationPreviewCommand> {
        let Some((active_generation, ref active_key)) = self.in_flight else { return Vec::new() };
        if active_key != key {
            return Vec::new();
        }
        self.in_flight = None;
        match &outcome {
            EquationRenderOutcome::Svg(svg) => self.cache.insert(key.clone(), svg.clone(), svg.len()),
            EquationRenderOutcome::Failed(failure) => {
                if *failure != EquationRenderFailure::RendererFailed {
                    self.failures.insert(key.clone(), *failure, 1);
                }
            }
        }
        let mut commands = Vec::new();
        if active_generation == self.generation {
            if let Some(mut target) = self.target.take() {
                if target.generation == self.generation && target.key.as_ref() == Some(key) {
                    match outcome {
                        EquationRenderOutcome::Svg(svg) => {
                            commands.append(&mut self.present(EquationPreviewContent::Fast { svg }, &target));
                            self.target = Some(target);
                        }
                        EquationRenderOutcome::Failed(failure) => {
                            target.failure = Some(failure);
                            self.target = Some(target);
                            commands.append(&mut self.handle_failure(failure, now_ms));
                        }
                    }
                } else {
                    self.target = Some(target);
                }
            }
        }
        if let Some(next) = self.queued.take() {
            if next.generation == self.generation
                && self.target.as_ref().and_then(|t| t.key.as_ref()) == Some(&next.key)
            {
                if self.cache.contains(&next.key) || self.failures.contains(&next.key) {
                    self.schedule(now_ms);
                } else {
                    self.in_flight = Some((next.generation, next.key.clone()));
                    commands.push(EquationPreviewCommand::Render(next));
                }
            }
        }
        commands
    }

    pub fn exact_render_completed(
        &mut self,
        key: &str,
        outcome: ExactEquationOutcome,
        _now_ms: u64,
    ) -> Vec<EquationPreviewCommand> {
        let Some((active_generation, ref active_key)) = self.exact_in_flight else {
            return Vec::new();
        };
        if active_key.as_str() != key {
            return Vec::new();
        }
        self.exact_in_flight = None;
        if let ExactEquationOutcome::Pdf(bytes) = &outcome {
            self.exact_cache.insert(key.to_string(), bytes.clone(), bytes.len());
        }
        let Some(target) = self.target.take() else { return Vec::new() };
        let matched = active_generation == self.generation
            && target.generation == self.generation
            && target.exact.as_ref().map(|e| e.key.as_str()) == Some(key);
        let commands = if matched {
            match outcome {
                ExactEquationOutcome::Pdf(bytes) => {
                    self.present(EquationPreviewContent::Exact { pdf: bytes }, &target)
                }
                ExactEquationOutcome::Failed => self.present(
                    EquationPreviewContent::Unavailable(EquationPreviewUnavailableReason::ExactFailed),
                    &target,
                ),
            }
        } else {
            Vec::new()
        };
        self.target = Some(target);
        commands
    }

    // MARK: Clock

    pub fn poll(&mut self, now_ms: u64, text: &dyn Fn() -> String) -> Vec<EquationPreviewCommand> {
        let mut commands = Vec::new();
        if let Some(deadline) = self.hover_grace_deadline {
            if deadline <= now_ms {
                self.hover_grace_deadline = None;
                if !self.pointer_in_popover {
                    // Clear the hover TARGET, not hover_location: the
                    // pointer may already rest on a new equation C whose
                    // region must resolve on this re-fire (C6).
                    if self.target.as_ref().map(|t| t.trigger) == Some(EquationPreviewTrigger::Hover) {
                        self.target = None;
                    }
                    self.schedule(now_ms);
                }
            }
        }
        if let Some(deadline) = self.keep_deadline {
            if deadline <= now_ms {
                self.keep_deadline = None;
                // Still showing an older result for an equation that stayed invalid.
                let stale = self
                    .target
                    .as_ref()
                    .map_or(false, |t| t.failure.is_some() || !t.region.is_renderable());
                if stale && self.visible.is_some() {
                    self.visible = None;
                    commands.push(EquationPreviewCommand::Hide);
                }
            }
        }
        if let Some(deadline) = self.fire_deadline {
            if deadline <= now_ms {
                self.fire_deadline = None;
                commands.append(&mut self.fire(now_ms, &text()));
            }
        }
        if let Some(deadline) = self.exact_deadline {
            if deadline <= now_ms {
                self.exact_deadline = None;
                let start = self.target.as_ref().map_or(false, |t| {
                    t.generation == self.generation
                        && t.failure == Some(EquationRenderFailure::UndefinedCommand)
                });
                if start {
                    let target = self.target.take().unwrap();
                    commands.append(&mut self.start_exact(&target));
                    self.target = Some(target);
                }
            }
        }
        commands
    }

    // MARK: Internals

    fn schedule(&mut self, deadline: u64) {
        self.generation += 1;
        self.fire_deadline = Some(deadline);
    }

    /// Hide everything and invalidate in-flight publication.
    fn retire(&mut self) -> Vec<EquationPreviewCommand> {
        self.generation += 1;
        self.fire_deadline = None;
        self.keep_deadline = None;
        self.exact_deadline = None;
        self.target = None;
        self.queued = None;
        let mut commands = self.cancel_exact();
        if self.visible.is_some() {
            self.visible = None;
            commands.push(EquationPreviewCommand::Hide);
        }
        commands
    }

    fn cancel_exact(&mut self) -> Vec<EquationPreviewCommand> {
        match self.exact_in_flight.take() {
            Some((_, key)) => vec![EquationPreviewCommand::CancelExact { key }],
            None => Vec::new(),
        }
    }

    fn hide_now(&mut self) -> Vec<EquationPreviewCommand> {
        self.target = None;
        self.keep_deadline = None;
        self.exact_deadline = None;
        let mut commands = self.cancel_exact();
        if self.visible.is_some() {
            self.visible = None;
            commands.push(EquationPreviewCommand::Hide);
        }
        commands
    }

    fn current_snapshot(&mut self, text: &str) -> Option<Arc<Snapshot>> {
        let file_id = self.file_id.clone()?;
        if let Some(snapshot) = &self.snapshot {
            if snapshot.revision == self.text_revision && snapshot.file_id == file_id {
                return self.snapshot.clone();
            }
        }
        let char_boundaries: Vec<usize> = text
            .char_indices()
            .map(|(i, _)| i)
            .chain(std::iter::once(text.len()))
            .collect();
        self.snapshot = Some(Arc::new(Snapshot {
            revision: self.text_revision,
            file_id,
            text: text.to_string(),
            scan: MathSourceScanner::scan(text),
            char_boundaries,
        }));
        self.snapshot.clone()
    }

    fn context_for(&mut self, snapshot: &Snapshot) -> (Arc<MathProjectContext>, Vec<EquationPreviewCommand>) {
        if let Some((version, revision, context)) = &self.project_context {
            if *version == self.context_version && *revision == snapshot.revision {
                return (context.clone(), Vec::new());
            }
        }
        // The active file's scan is passed as an overlay so the whole
        // external_scans map is not cloned on every rebuild (C11).
        let root = self.root_file_id.clone().unwrap_or_else(|| snapshot.file_id.clone());
        let context = Arc::new(MathProjectContext::new(
            &root,
            Some(&snapshot.file_id),
            Some(&snapshot.scan),
            &self.external_scans,
            &self.resolutions,
        ));
        // A context switch can orphan keys from the previous context: the
        // host may never answer them, and fire() waits on every pending key.
        // Keep only what the fresh context still needs answered.
        let still_unresolved: HashSet<MathIncludeKey> =
            context.unresolved.iter().map(|r| r.key.clone()).collect();
        self.pending_resolutions
            .retain(|k| still_unresolved.contains(k));
        let requests: Vec<MathIncludeRequest> = context
            .unresolved
            .iter()
            .filter(|r| self.pending_resolutions.insert(r.key.clone()))
            .cloned()
            .collect();
        self.project_context = Some((self.context_version, snapshot.revision, context.clone()));
        if requests.is_empty() {
            (context, Vec::new())
        } else {
            (context, vec![EquationPreviewCommand::ResolveIncludes(requests)])
        }
    }

    fn fire(&mut self, now_ms: u64, text: &str) -> Vec<EquationPreviewCommand> {
        if !self.settings.enabled
            || self.composing
            || (self.renderer_version.is_none() && self.renderer_failed)
        {
            return self.hide_now();
        }
        let Some(snapshot) = self.current_snapshot(text) else { return self.hide_now() };

        let mut region: Option<MathRegion> = None;
        let mut trigger = EquationPreviewTrigger::Caret;
        if let Some(hover_location) = self.hover_location {
            region = snapshot
                .scan
                .region_at_character(snapshot.utf8_for_char(hover_location))
                .cloned();
            trigger = EquationPreviewTrigger::Hover;
            // Crossing plain text toward the popover must not drop it at once.
            if region.is_none()
                && self.target.as_ref().map(|t| t.trigger) == Some(EquationPreviewTrigger::Hover)
                && self.visible.is_some()
                && !self.pointer_in_popover
            {
                if self.hover_grace_deadline.is_none() {
                    self.hover_grace_deadline = Some(now_ms + Self::HOVER_EXIT_GRACE_MS);
                }
                return Vec::new();
            }
            // Reaching a math region ends the exit-grace window (C6).
            if region.is_some() {
                self.hover_grace_deadline = None;
            }
        }
        // B3: this early return protects only a HOVER-target preview; a
        // caret target keeps refreshing while the pointer rests on the
        // popover.
        if region.is_none()
            && self.pointer_in_popover
            && self.visible.is_some()
            && self.target.as_ref().map(|t| t.trigger) == Some(EquationPreviewTrigger::Hover)
        {
            return Vec::new();
        }
        if region.is_none() && self.focused && self.selection.1 == 0 && !self.suppressed_by_typing {
            region = snapshot
                .scan
                .region_at_caret(snapshot.utf8_for_char(self.selection.0))
                .cloned();
            trigger = EquationPreviewTrigger::Caret;
        }
        let Some(region) = region else { return self.hide_now() };
        let region_start = region.range.utf8_offset;
        if let Some(dismissed) = self.dismissed_region_start {
            if dismissed == region_start {
                return self.hide_now();
            }
            self.dismissed_region_start = None;
        }
        let anchor = snapshot.char_for_utf8(region.range.utf8_offset.max(0) as usize)
            ..snapshot.char_for_utf8(region.range.end_utf8_offset().max(0) as usize);
        let same_equation = self.visible.as_ref().map(|v| v.region_start) == Some(region_start)
            || self.target.as_ref().map(|t| t.region.range.utf8_offset) == Some(region_start);
        if !same_equation {
            self.keep_deadline = None;
        }

        if !region.is_renderable() {
            self.target = Some(Target {
                generation: self.generation,
                region,
                anchor,
                trigger,
                key: None,
                exact: None,
                failure: Some(EquationRenderFailure::Invalid),
            });
            self.exact_deadline = None;
            let mut commands = self.cancel_exact();
            commands.append(&mut self.keep_or_hide(now_ms));
            return commands;
        }

        let source = region.render_source(&snapshot.text);
        let (project_context, mut commands) = self.context_for(&snapshot);
        if source.len() > Self::MAXIMUM_SOURCE_BYTES {
            let target = Target {
                generation: self.generation,
                region,
                anchor,
                trigger,
                key: None,
                exact: None,
                failure: Some(EquationRenderFailure::TooLarge),
            };
            self.target = Some(target.clone());
            commands.append(&mut self.present(
                EquationPreviewContent::Unavailable(EquationPreviewUnavailableReason::TooLarge),
                &target,
            ));
            return commands;
        }
        let math_context = project_context.context(&snapshot.file_id, region_start as usize);
        let key = EquationPreviewKey {
            source: EquationPreviewKey::normalize(&source),
            display_mode: region.display_mode,
            context_key: math_context.key.clone(),
            renderer_version: self.renderer_version.clone().unwrap_or_default(),
            font_size: self.appearance.font_size,
        };
        let region_text = region.source_text(&snapshot.text);
        let display_mode = region.display_mode;
        let exact = self.exact_profile.clone().map(|profile| {
            Arc::new(
                self.exact_plan(&profile, &region_text, &project_context, &snapshot, region_start as usize),
            )
        });
        let mut target = Target {
            generation: self.generation,
            region,
            anchor,
            trigger,
            key: Some(key),
            exact,
            failure: None,
        };

        if let Some((_, active_key)) = self.exact_in_flight.clone() {
            if Some(&active_key) == target.exact.as_ref().map(|e| &e.key) {
                self.exact_in_flight = Some((self.generation, active_key));
            } else {
                commands.append(&mut self.cancel_exact());
            }
        }
        if self.explicit_exact_region_start == Some(region_start) {
            commands.append(&mut self.start_exact(&target));
            self.target = Some(target);
            return commands;
        }
        self.exact_deadline = None;
        // Definitions from unresolved includes are still missing: rendering
        // now would flash "unavailable". The host's answer fires again.
        if !self.pending_resolutions.is_empty() {
            self.target = Some(target);
            return commands;
        }

        let key = target.key.clone().unwrap();
        if let Some(svg) = self.cache.value(&key).cloned() {
            commands.append(&mut self.present(EquationPreviewContent::Fast { svg }, &target));
            self.target = Some(target);
            return commands;
        }
        if let Some(failure) = self.failures.value(&key).copied() {
            target.failure = Some(failure);
            self.target = Some(target);
            commands.append(&mut self.handle_failure(failure, now_ms));
            return commands;
        }
        self.target = Some(target);
        if self.renderer_version.is_none() {
            return commands; // page still loading: ready re-fires
        }
        let request = EquationRenderRequest {
            generation: self.generation,
            key: key.clone(),
            source,
            display_mode,
            definitions: math_context.definitions,
            font_size: self.appearance.font_size,
        };
        if let Some((_, active_key)) = &self.in_flight {
            if *active_key == key {
                self.in_flight = Some((self.generation, key));
            } else {
                self.queued = Some(request);
            }
        } else {
            self.in_flight = Some((self.generation, key));
            commands.push(EquationPreviewCommand::Render(request));
        }
        commands
    }

    fn exact_plan(
        &self,
        profile: &str,
        region: &str,
        context: &MathProjectContext,
        snapshot: &Snapshot,
        region_start: usize,
    ) -> ExactPlan {
        let root = context.root_file_id.clone();
        let root_scan = if root == snapshot.file_id {
            Some(&snapshot.scan)
        } else {
            self.external_scans.get(&root)
        };
        let preamble = root_scan.and_then(|scan| scan.preamble.as_deref());
        let definitions = context.body_definitions(
            &snapshot.file_id,
            region_start,
            if preamble.is_none() { None } else { root_scan.and_then(|s| s.document_begin_offset) },
        );
        let document = if preamble.is_none() {
            ExactEquationDocument::make(None, &definitions, &[], region)
        } else {
            ExactEquationDocument::make(preamble, &[], &definitions, region)
        };
        let key = MathPreviewHash::hex(MathPreviewHash::combine(
            MathPreviewHash::combine(MathPreviewHash::BASIS, profile),
            &document,
        ));
        ExactPlan { key, document }
    }

    fn start_exact(&mut self, target: &Target) -> Vec<EquationPreviewCommand> {
        let Some(exact) = &target.exact else {
            return self.present(
                EquationPreviewContent::Unavailable(EquationPreviewUnavailableReason::ExactUnavailable),
                target,
            );
        };
        if let Some(pdf) = self.exact_cache.value(&exact.key).cloned() {
            return self.present(EquationPreviewContent::Exact { pdf }, target);
        }
        if let Some((_, active_key)) = &self.exact_in_flight {
            if *active_key == exact.key {
                self.exact_in_flight = Some((self.generation, exact.key.clone()));
                return Vec::new();
            }
        }
        let mut commands = self.cancel_exact();
        self.exact_in_flight = Some((self.generation, exact.key.clone()));
        commands.push(EquationPreviewCommand::RenderExact(ExactEquationRequest {
            generation: self.generation,
            key: exact.key.clone(),
            document: exact.document.clone(),
        }));
        commands
    }

    fn handle_failure(
        &mut self,
        failure: EquationRenderFailure,
        now_ms: u64,
    ) -> Vec<EquationPreviewCommand> {
        let Some(target) = self.target.clone() else { return Vec::new() };
        match failure {
            EquationRenderFailure::UndefinedCommand => {
                if self.settings.renderer != EquationPreviewRendererMode::FastWithTexFallback {
                    return self.present(
                        EquationPreviewContent::Unavailable(EquationPreviewUnavailableReason::Unsupported),
                        &target,
                    );
                }
                let Some(exact) = target.exact.clone() else {
                    return self.present(
                        EquationPreviewContent::Unavailable(EquationPreviewUnavailableReason::ExactUnavailable),
                        &target,
                    );
                };
                if let Some(pdf) = self.exact_cache.value(&exact.key).cloned() {
                    return self.present(EquationPreviewContent::Exact { pdf }, &target);
                }
                // TeX runs only once the equation has settled — never per keystroke.
                let commands = self.keep_or_hide(now_ms);
                self.exact_deadline = Some(now_ms + Self::EXACT_SETTLE_MS);
                commands
            }
            EquationRenderFailure::Invalid => self.keep_or_hide(now_ms),
            EquationRenderFailure::TooLarge => self.present(
                EquationPreviewContent::Unavailable(EquationPreviewUnavailableReason::TooLarge),
                &target,
            ),
            EquationRenderFailure::RendererFailed => self.hide_now(),
        }
    }

    /// An invalid intermediate state: the previous preview of the same
    /// equation stays briefly, anything else hides now. The target (and an
    /// adopted exact job) survive for the next result.
    fn keep_or_hide(&mut self, now_ms: u64) -> Vec<EquationPreviewCommand> {
        let target_start = self.target.as_ref().map(|t| t.region.range.utf8_offset);
        let same = self
            .visible
            .as_ref()
            .map_or(false, |v| Some(v.region_start) == target_start);
        if !same {
            self.keep_deadline = None;
            if self.visible.is_none() {
                return Vec::new();
            }
            self.visible = None;
            return vec![EquationPreviewCommand::Hide];
        }
        if self.keep_deadline.is_none() {
            self.keep_deadline = Some(now_ms + Self::KEEP_LAST_GOOD_MS);
        }
        Vec::new()
    }

    fn present(&mut self, content: EquationPreviewContent, target: &Target) -> Vec<EquationPreviewCommand> {
        self.keep_deadline = None;
        self.visible =
            Some(Visible { region_start: target.region.range.utf8_offset, content: content.clone() });
        vec![EquationPreviewCommand::Show(EquationPreviewPresentation {
            generation: target.generation,
            content,
            anchor: target.anchor.clone(),
            anchor_utf8: target.region.range,
            display_mode: target.region.display_mode,
            placement: self.settings.placement,
            trigger: target.trigger,
        })]
    }
}
