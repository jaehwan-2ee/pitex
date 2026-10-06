//! Rust port of `EditorFolding.swift` for GtkSourceView — the GTK half.
//! Region discovery lives in `editor_feature::fold` (verbatim `findRegions`);
//! this engine binds the results to a `sourceview5::View`. Where the macOS
//! engine drives the layout manager to zero hidden lines' heights, GTK
//! collapses the same character ranges with an `invisible` `TextTag` — hidden
//! lines are truly gone from layout, so line numbers and the minimap skip
//! them identically. The gutter disclosure triangles map to a
//! `GutterRendererText`, and the "…" chip overlay to a draw-only
//! `DrawingArea` over the view.

use gtk4::prelude::*;
use gtk4::glib;
use language_core::{DeterministicTeXLexer, LanguageToken, TeXDialect};
use sourceview5::prelude::*;
use std::cell::{Cell, RefCell};
use std::collections::HashSet;
use std::rc::{Rc, Weak};

use editor_feature::fold::{compute_line_starts, find_regions_with_tokens, FoldRegion};

const FOLD_TAG: &str = "pitex.fold.hidden";
/// A beat after the highlight pass (`ANALYSIS_DEBOUNCE`), whose
/// `recompute_with_tokens` normally lands first and cancels this one — the
/// document is then lexed once per typing pause.
const RECOMPUTE_DEBOUNCE: std::time::Duration =
    crate::app_ui::ANALYSIS_DEBOUNCE.saturating_add(std::time::Duration::from_millis(100));

/// The fold engine bound to one `sourceview5::View`/`Buffer` pair —
/// `FoldEngine` + `FoldChipOverlayView` in one GTK-native unit. Recomputes
/// on buffer changes (debounced behind `scheduleHighlight`), carries
/// folded state across recomputes by signature, and drives the gutter
/// renderer + chip overlay. `Rc`-bound; never crosses threads.
pub struct FoldEngine {
    view: sourceview5::View,
    buffer: sourceview5::Buffer,
    regions: RefCell<Vec<FoldRegion>>,
    /// (byte offset, char offset) per line — rebuilt with `regions`.
    line_starts: RefCell<Vec<(usize, usize)>>,
    hidden_lines: RefCell<HashSet<usize>>,
    header_lines: RefCell<HashSet<usize>>,
    enabled: Cell<bool>,
    recompute_source: RefCell<Option<glib::SourceId>>,
    dialect: TeXDialect,
    /// Chip rects in overlay coordinates: (x, y, w, h, header_line) rebuilt
    /// each draw for hit-testing — `FoldChipOverlayView.chipRects`.
    chip_rects: RefCell<Vec<(f64, f64, f64, f64, usize)>>,
    chip_area: gtk4::DrawingArea,
    renderer: sourceview5::GutterRendererText,
    self_weak: RefCell<Weak<FoldEngine>>,
}

impl FoldEngine {
    /// `attach(to:)` — builds the tag, gutter renderer, chip layer and hooks.
    /// `chip_area()` is overlaid over the scroller by the caller.
    pub fn attach(view: &sourceview5::View, dialect: TeXDialect, enabled: bool) -> Rc<Self> {
        let buffer = view
            .buffer()
            .downcast::<sourceview5::Buffer>()
            .expect("pitex editor buffer");

        // Collapse tag — `invisible` removes the range from layout entirely.
        let table = buffer.tag_table();
        if table.lookup(FOLD_TAG).is_none() {
            let tag = gtk4::TextTag::new(Some(FOLD_TAG));
            tag.set_invisible(true);
            table.add(&tag);
        }

        // Chip layer: a draw-only overlay the caller hosts over the
        // ScrolledWindow (never inside it — a non-Scrollable child would
        // disconnect the view's adjustments and kill minimap/jump-to).
        let chip_area = gtk4::DrawingArea::new();
        chip_area.set_can_target(false);

        // Gutter disclosure triangles, inserted leftmost in the gutter.
        let renderer = sourceview5::GutterRendererText::new();
        renderer.set_xalign(1.0);
        let gutter = sourceview5::prelude::ViewExt::gutter(view, gtk4::TextWindowType::Left);
        gutter.insert(&renderer, 0);

        let engine = Rc::new(Self {
            view: view.clone(),
            buffer,
            regions: RefCell::new(Vec::new()),
            line_starts: RefCell::new(Vec::new()),
            hidden_lines: RefCell::new(HashSet::new()),
            header_lines: RefCell::new(HashSet::new()),
            enabled: Cell::new(enabled),
            recompute_source: RefCell::new(None),
            dialect,
            chip_rects: RefCell::new(Vec::new()),
            chip_area,
            renderer,
            self_weak: RefCell::new(Weak::new()),
        });
        *engine.self_weak.borrow_mut() = Rc::downgrade(&engine);

        engine.install_hooks();
        engine.recompute();
        engine
    }

    /// The draw-only chip layer — the caller adds it as an overlay over
    /// the ScrolledWindow (see `attach` for why it can't live inside).
    pub fn chip_area(&self) -> &gtk4::DrawingArea {
        &self.chip_area
    }

    /// `isEnabled` — disabling reports every region unfolded and unhides all
    /// lines; enabling re-discovers regions fresh.
    pub fn set_enabled(&self, enabled: bool) {
        if self.enabled.get() == enabled {
            return;
        }
        self.enabled.set(enabled);
        if enabled {
            self.recompute();
        } else {
            for region in self.regions.borrow_mut().iter_mut() {
                region.folded = false;
            }
            self.apply_folds();
            self.on_change();
        }
    }

    /// `isLineHidden` — exposed for parity; the gutter and line-number
    /// columns skip hidden rows automatically through the collapse tag.
    pub fn is_line_hidden(&self, line: usize) -> bool {
        self.hidden_lines.borrow().contains(&line)
    }

    fn install_hooks(self: &Rc<Self>) {
        // Re-scan on edits — the NSText.didChange observer equivalent.
        let weak = Rc::downgrade(self);
        self.buffer.connect_changed(move |_| {
            if let Some(engine) = weak.upgrade() {
                engine.schedule_recompute();
            }
        });

        // Gutter text per line: ▾ on foldable headers, ▸ when folded.
        let weak = Rc::downgrade(self);
        self.renderer.connect_query_data(move |renderer, _line_obj, line| {
            let Some(engine) = weak.upgrade() else { return };
            let line = line as usize;
            let markup = if !engine.enabled.get() {
                ""
            } else if engine.is_folded(line) {
                "<span size=\"x-small\" foreground=\"#888888\">▸</span>"
            } else if engine.header_lines.borrow().contains(&line) {
                "<span size=\"x-small\" foreground=\"#888888\">▾</span>"
            } else {
                ""
            };
            renderer.set_markup(markup);
        });
        let weak = Rc::downgrade(self);
        self.renderer.connect_query_activatable(move |_, iter, _area| {
            let Some(engine) = weak.upgrade() else { return false };
            engine.enabled.get()
                && engine.header_lines.borrow().contains(&(iter.line() as usize))
        });
        let weak = Rc::downgrade(self);
        self.renderer.connect_activate(move |_, iter, _area, _button, _state, _n| {
            if let Some(engine) = weak.upgrade() {
                engine.toggle(iter.line() as usize);
            }
        });

        // Chip layer: redraws "…" markers at folded line ends each frame.
        let weak = Rc::downgrade(self);
        self.chip_area.set_draw_func(move |_, cr, width, height| {
            if let Some(engine) = weak.upgrade() {
                engine.draw_chips(cr, width, height);
            }
        });

        // Chip hit-testing: a capture-phase click on the view claims the
        // sequence only inside a chip rect, unfolding immediately.
        let weak = Rc::downgrade(self);
        let click = gtk4::GestureClick::new();
        click.set_button(1);
        click.set_propagation_phase(gtk4::PropagationPhase::Capture);
        click.connect_pressed(move |gesture, _n, x, y| {
            let Some(engine) = weak.upgrade() else { return };
            if let Some(line) = engine.chip_at(x, y) {
                gesture.set_state(gtk4::EventSequenceState::Claimed);
                engine.unfold(line);
            }
        });
        self.view.add_controller(click);

        // The chip layer overlays the scroller (fixed to the visible area),
        // so scroll/zoom must redraw it — rects are recomputed per draw.
        // Attached before the view enters the scroller, hence the helper.
        let weak = Rc::downgrade(self);
        crate::app_ui::on_view_scroll(&self.view, move || {
            if let Some(engine) = weak.upgrade() {
                engine.chip_area.queue_draw();
            }
        });
    }

    fn schedule_recompute(&self) {
        if !self.enabled.get() {
            return;
        }
        self.cancel_scheduled_recompute();
        let weak = self.self_weak.borrow().clone();
        let id = glib::timeout_add_local_once(RECOMPUTE_DEBOUNCE, move || {
            if let Some(engine) = weak.upgrade() {
                // Fired: forget the id before anything could remove it.
                engine.recompute_source.borrow_mut().take();
                engine.recompute();
            }
        });
        *self.recompute_source.borrow_mut() = Some(id);
    }

    fn cancel_scheduled_recompute(&self) {
        if let Some(id) = self.recompute_source.borrow_mut().take() {
            id.remove();
        }
    }

    /// `recompute()` — rescan regions, restore folded state by signature.
    pub fn recompute(&self) {
        if !self.enabled.get() { return; }
        let text = self.buffer_text();
        let tokens = DeterministicTeXLexer::tokenize(&text, self.dialect);
        self.recompute_with_tokens(&text, &tokens);
    }

    pub fn recompute_with_tokens(&self, text: &str, tokens: &[LanguageToken]) {
        self.cancel_scheduled_recompute();
        if !self.enabled.get() { return; }
        let starts = compute_line_starts(&text);
        let previous: Vec<String> = self
            .regions
            .borrow()
            .iter()
            .filter(|r| r.folded)
            .map(|r| r.signature.clone())
            .collect();
        let mut regions = find_regions_with_tokens(text, &starts, tokens);
        for region in regions.iter_mut() {
            if previous.iter().any(|s| *s == region.signature) {
                region.folded = true;
            }
        }
        *self.line_starts.borrow_mut() = starts;
        *self.regions.borrow_mut() = regions;
        self.apply_folds();
        self.on_change();
    }

    /// Re-derives the hidden line set and applies the collapse tag — the
    /// `applyFolds` equivalent (`hiddenCharRanges` → tag ranges).
    fn apply_folds(&self) {
        let (buf_start, buf_end) = self.buffer.bounds();
        let tag = self.buffer.tag_table().lookup(FOLD_TAG);
        if let Some(tag) = &tag {
            self.buffer.remove_tag(tag, &buf_start, &buf_end);
        }
        let mut lines = HashSet::new();
        let mut header_lines = HashSet::new();
        // Every fold tag was just removed, so the buffer's own count is the
        // full text's — no document copy needed.
        let total_chars = self.buffer.char_count();
        {
            let line_starts = self.line_starts.borrow();
            for region in self.regions.borrow().iter() {
                header_lines.insert(region.header_line);
                if !self.enabled.get() || !region.folded {
                    continue;
                }
                let (lo, hi) = region.hidden_line_range;
                if region.header_line + 1 >= line_starts.len() {
                    continue;
                }
                let last_line = hi.min(line_starts.len() - 1);
                if lo >= line_starts.len() {
                    continue;
                }
                let first_char = line_starts[lo].1 as i32;
                let end_char = if last_line + 1 < line_starts.len() {
                    line_starts[last_line + 1].1 as i32
                } else {
                    total_chars
                };
                if end_char <= first_char {
                    continue;
                }
                if let Some(tag) = &tag {
                    let start = self.buffer.iter_at_offset(first_char);
                    let end = self.buffer.iter_at_offset(end_char);
                    self.buffer.apply_tag(tag, &start, &end);
                }
                for line in lo..=last_line {
                    lines.insert(line);
                }
            }
        }
        *self.hidden_lines.borrow_mut() = lines;
        *self.header_lines.borrow_mut() = header_lines;
    }

    fn on_change(&self) {
        self.view.queue_draw();
        self.chip_area.queue_draw();
        self.renderer.queue_draw();
    }

    /// The whole document, folded (invisible) ranges included — excluding
    /// them made `recompute` lex a text whose lines no longer matched the
    /// buffer, misplacing every fold after the first hidden range.
    fn buffer_text(&self) -> String {
        let (s, e) = self.buffer.bounds();
        self.buffer.text(&s, &e, true).to_string()
    }

    fn is_folded(&self, line: usize) -> bool {
        self.regions
            .borrow()
            .iter()
            .any(|r| r.header_line == line && r.folded)
    }

    /// `toggle(atLine:)` — the disclosure-triangle handler.
    pub fn toggle(&self, line: usize) {
        let found = {
            let mut regions = self.regions.borrow_mut();
            if let Some(region) = regions.iter_mut().find(|r| r.header_line == line) {
                region.folded = !region.folded;
                true
            } else {
                false
            }
        };
        if !found {
            return;
        }
        self.apply_folds();
        self.on_change();
    }

    /// `unfold(atLine:)` — the chip-click handler.
    pub fn unfold(&self, line: usize) {
        let found = {
            let mut regions = self.regions.borrow_mut();
            if let Some(region) = regions
                .iter_mut()
                .find(|r| r.header_line == line && r.folded)
            {
                region.folded = false;
                true
            } else {
                false
            }
        };
        if !found {
            return;
        }
        self.apply_folds();
        self.on_change();
    }

    /// Chip rects for `draw_chips` + `chip_at` — the macOS overlay computes
    /// them per-draw in view coordinates; GTK does the same so scrolling
    /// and layout stay exact.
    fn compute_chip_rects(&self) -> Vec<(f64, f64, f64, f64, usize)> {
        let mut rects = Vec::new();
        if !self.enabled.get() {
            return rects;
        }
        let buffer = self.view.buffer();
        for region in self.regions.borrow().iter() {
            if !region.folded {
                continue;
            }
            let Some(mut iter) = buffer.iter_at_line(region.header_line as i32) else {
                continue;
            };
            if !iter.ends_line() && !iter.forward_to_line_end() {
                continue;
            }
            let rect = self.view.iter_location(&iter);
            let (x, y) = self.view.buffer_to_window_coords(
                gtk4::TextWindowType::Widget,
                rect.x() + rect.width(),
                rect.y(),
            );
            let chip_h = (rect.height() as f64 - 4.0).max(8.0);
            rects.push((
                x as f64 + 3.0,
                y as f64 + 2.0,
                chip_h * 2.2,
                chip_h,
                region.header_line,
            ));
        }
        rects
    }

    fn draw_chips(&self, cr: &gtk4::cairo::Context, _width: i32, _height: i32) {
        let rects = self.compute_chip_rects();
        *self.chip_rects.borrow_mut() = rects.clone();
        for (x, y, w, h, _line) in rects {
            // Rounded "…" chip: background pill + three dots — the
            // reference editor's folded-region affordance.
            let r = (h / 2.0).min(4.0);
            cr.new_sub_path();
            cr.arc(
                x + w - r,
                y + r,
                r,
                -0.5 * std::f64::consts::PI,
                0.5 * std::f64::consts::PI,
            );
            cr.arc(
                x + r,
                y + r,
                r,
                0.5 * std::f64::consts::PI,
                1.5 * std::f64::consts::PI,
            );
            cr.close_path();
            cr.set_source_rgba(0.5, 0.5, 0.5, 0.18);
            let _ = cr.fill_preserve();
            cr.set_source_rgba(0.5, 0.5, 0.5, 0.85);
            cr.set_line_width(1.0);
            let _ = cr.stroke();
            cr.set_source_rgba(0.45, 0.45, 0.45, 0.9);
            let cy = y + h / 2.0;
            let dot_r = (h / 7.0).max(0.8);
            for k in 0..3 {
                let cx = x + w * (0.25 + 0.25 * k as f64);
                cr.arc(cx, cy, dot_r, 0.0, 2.0 * std::f64::consts::PI);
                let _ = cr.fill();
            }
        }
    }

    fn chip_at(&self, x: f64, y: f64) -> Option<usize> {
        self.chip_rects
            .borrow()
            .iter()
            .find(|(cx, cy, w, h, _)| x >= *cx && x <= *cx + *w && y >= *cy && y <= *cy + *h)
            .map(|(_, _, _, _, line)| *line)
    }
}
