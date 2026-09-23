//! `CommitDiffView` port — the side-by-side diff surface that covers the
//! editor area while `model.git_diff` is set. `DiffList` flattens
//! `display_items` output into a lazy `GListModel` so a file of
//! thousands of rows still binds only what scrolls into view; `build`
//! returns the `"diff"` child of `editor_stack` (header + content).

use crate::app_ui::{a11y, AppState, UiHandles};
use crate::compat;
use crate::l10n::{tr, tr1};
use git_core::{GitChangeKind, GitCommitFile, GitDiffFileSection, GitDiffItem, GitDiffLine, GitDiffLineKind};
use glib::subclass::prelude::*;
use gtk4::{gio, glib, prelude::*};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

/// The Swift `displayItems` default — context lines kept around a fold.
const EDGE_CONTEXT: usize = 3;

/// One flattened display row of the diff list.
#[derive(Clone)]
pub enum DiffRow {
    /// Multi-file section banner (click collapses the section).
    FileHeader {
        section: usize,
        file: GitCommitFile,
        additions: usize,
        deletions: usize,
        collapsed: bool,
    },
    Pair {
        left: Option<GitDiffLine>,
        right: Option<GitDiffLine>,
    },
    /// Collapsed unchanged region — activating it expands the pairs.
    Fold { section: usize, id: u32, count: usize },
    /// Hunk-boundary marker — same look as `Fold`, not expandable.
    Gap { count: usize },
    /// `\ No newline at end of file` — dimmed full-width line.
    Note(String),
    /// `section.binary` placeholder.
    Binary,
}

mod imp {
    use super::*;
    use gio::subclass::prelude::*;
    use gtk4::glib;

    #[derive(Default)]
    pub struct DiffList {
        /// `GitDiff.id` of the rendered session — re-setting the same
        /// session is a no-op so fold/expand state survives refreshes.
        pub session: std::cell::Cell<u64>,
        pub sections: RefCell<Vec<GitDiffFileSection>>,
        /// `session.file != nil` → the file banner rows are skipped (the
        /// outer header already names the file).
        pub single_file: std::cell::Cell<bool>,
        /// `(section, fold id)` keys the user expanded.
        pub expanded_folds: RefCell<HashSet<(usize, u32)>>,
        pub collapsed_files: RefCell<HashSet<usize>>,
        /// Flattened `DiffRow`s, rebuilt by `rebuild()`.
        pub flat: RefCell<Vec<DiffRow>>,
        pub items: RefCell<HashMap<u32, glib::WeakRef<glib::BoxedAnyObject>>>,
        /// `(dark, editor bg rgb)` for intra-line markup — set by
        /// `refresh_git_diff`, which can reach AppState; binds may run
        /// while the state is already mutably borrowed.
        pub dark: std::cell::Cell<bool>,
        pub editor_bg: std::cell::Cell<(f64, f64, f64)>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for DiffList {
        const NAME: &'static str = "PitexDiffList";
        type Type = super::DiffList;
        type Interfaces = (gio::ListModel,);
    }
    impl ObjectImpl for DiffList {}
    impl ListModelImpl for DiffList {
        fn item_type(&self) -> glib::Type {
            glib::BoxedAnyObject::static_type()
        }
        fn n_items(&self) -> u32 {
            self.flat.borrow().len() as u32
        }
        fn item(&self, position: u32) -> Option<glib::Object> {
            let mut items = self.items.borrow_mut();
            if let Some(item) = items.get(&position).and_then(glib::WeakRef::upgrade) {
                return Some(item.upcast());
            }
            let item = glib::BoxedAnyObject::new(self.obj().row(position)?);
            if items.len() >= 1024 {
                items.retain(|_, item| item.upgrade().is_some());
            }
            items.insert(position, item.downgrade());
            Some(item.upcast())
        }
    }
}

glib::wrapper! {
    pub struct DiffList(ObjectSubclass<imp::DiffList>) @implements gio::ListModel;
}

impl DiffList {
    pub fn new() -> Self {
        glib::Object::new()
    }

    /// Intra-line markup colors from the appearance settings — read at
    /// bind time, so keep them current on every `refresh_git_diff`.
    pub fn set_theme_colors(&self, dark: bool, editor_bg: (f64, f64, f64)) {
        self.imp().dark.set(dark);
        self.imp().editor_bg.set(editor_bg);
    }

    /// The rendered session's id — callers can skip cloning `sections`
    /// when it already matches (`set_sections` is a no-op for it anyway).
    pub fn session(&self) -> u64 {
        self.imp().session.get()
    }

    /// Swap the rendered session — expansion state is per session, like
    /// SwiftUI's fresh `@State` for a new `gitDiff`.
    pub fn set_sections(&self, session: u64, single_file: bool, sections: Vec<GitDiffFileSection>) {
        if self.imp().session.replace(session) == session {
            return;
        }
        let old_count = self.n_items();
        self.imp().single_file.set(single_file);
        self.imp().sections.replace(sections);
        self.imp().expanded_folds.borrow_mut().clear();
        self.imp().collapsed_files.borrow_mut().clear();
        self.rebuild();
        self.imp().items.borrow_mut().clear();
        self.items_changed(0, old_count, self.n_items());
    }

    /// The "⋯ N unchanged lines" bar expanded — re-run the fold map.
    pub fn expand_fold(&self, section: usize, id: u32) {
        if self.imp().expanded_folds.borrow_mut().insert((section, id)) {
            self.refresh();
        }
    }

    /// Multi-file section banner toggled — collapse or re-show a file.
    pub fn toggle_file(&self, section: usize) {
        let collapsed = &self.imp().collapsed_files;
        if !collapsed.borrow_mut().insert(section) {
            collapsed.borrow_mut().remove(&section);
        }
        self.refresh();
    }

    pub fn row(&self, position: u32) -> Option<DiffRow> {
        self.imp().flat.borrow().get(position as usize).cloned()
    }

    fn refresh(&self) {
        let old_count = self.n_items();
        self.rebuild();
        self.imp().items.borrow_mut().clear();
        self.items_changed(0, old_count, self.n_items());
    }

    fn rebuild(&self) {
        let imp = self.imp();
        let sections = imp.sections.borrow();
        let expanded = imp.expanded_folds.borrow();
        let collapsed = imp.collapsed_files.borrow();
        let single = imp.single_file.get();
        let mut flat = Vec::new();
        for (index, section) in sections.iter().enumerate() {
            if !single {
                flat.push(DiffRow::FileHeader {
                    section: index,
                    file: section.file.clone(),
                    additions: section.additions(),
                    deletions: section.deletions(),
                    collapsed: collapsed.contains(&index),
                });
                if collapsed.contains(&index) {
                    continue;
                }
            }
            if section.binary {
                flat.push(DiffRow::Binary);
            }
            for item in git_core::display_items(&section.rows, EDGE_CONTEXT) {
                match item {
                    GitDiffItem::Pair { left, right } => flat.push(DiffRow::Pair { left, right }),
                    GitDiffItem::Fold { id, pairs } => {
                        if expanded.contains(&(index, id)) {
                            for p in pairs {
                                flat.push(DiffRow::Pair {
                                    left: p.left,
                                    right: p.right,
                                });
                            }
                        } else {
                            flat.push(DiffRow::Fold {
                                section: index,
                                id,
                                count: pairs.len(),
                            });
                        }
                    }
                    GitDiffItem::Gap { old_lines } => flat.push(DiffRow::Gap {
                        count: old_lines as usize,
                    }),
                    GitDiffItem::Note(text) => flat.push(DiffRow::Note(text)),
                }
            }
        }
        *imp.flat.borrow_mut() = flat;
    }
}

// ── The `CommitDiffView` surface ─────────────────────────────────────────────

/// Badge letter color, matching the Changes rows (`badgeColor` on macOS).
fn badge_css_class(kind: GitChangeKind) -> &'static str {
    match kind {
        GitChangeKind::Modified | GitChangeKind::TypeChanged => "warning",
        GitChangeKind::Added | GitChangeKind::Untracked => "success",
        GitChangeKind::Deleted | GitChangeKind::Conflicted => "error",
        GitChangeKind::Renamed | GitChangeKind::Copied => "accent",
    }
}

/// `monoSmall` — the small monospaced header text (badges, stats, chips).
fn mono_small(label: &gtk4::Label) {
    label.add_css_class("monospace");
    label.add_css_class("caption");
}

/// The `"diff"` child of `editor_stack`: header (badge/path/stats/commit
/// or working-tree chip + close) over a loading/error/empty/rows stack
/// whose rows page is a virtualized `ListView`.
pub fn build(state: &Rc<RefCell<AppState>>, ui: &UiHandles, lang: &'static str) -> gtk4::Widget {
    let container = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    container.add_css_class("pitex-editor");
    container.set_widget_name("pitex.git.diff");

    let header = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    header.set_margin_start(10);
    header.set_margin_end(10);
    header.set_margin_top(6);
    header.set_margin_bottom(6);

    let badge = gtk4::Label::new(None);
    mono_small(&badge);
    ui.git_diff_badge.replace(Some(badge.clone()));
    header.append(&badge);
    let path = gtk4::Label::new(None);
    path.add_css_class("monospace");
    path.set_ellipsize(gtk4::pango::EllipsizeMode::Middle);
    path.set_xalign(0.0);
    ui.git_diff_path.replace(Some(path.clone()));
    header.append(&path);
    let adds = gtk4::Label::new(None);
    mono_small(&adds);
    adds.add_css_class("success");
    ui.git_diff_adds.replace(Some(adds.clone()));
    header.append(&adds);
    let dels = gtk4::Label::new(None);
    mono_small(&dels);
    dels.add_css_class("error");
    ui.git_diff_dels.replace(Some(dels.clone()));
    header.append(&dels);
    let files = gtk4::Label::new(None);
    mono_small(&files);
    files.add_css_class("dim-label");
    ui.git_diff_files.replace(Some(files.clone()));
    header.append(&files);

    let chip = gtk4::Label::new(None);
    mono_small(&chip);
    chip.add_css_class("pitex-diff-chip");
    ui.git_diff_chip.replace(Some(chip.clone()));
    header.append(&chip);
    let subject = gtk4::Label::new(None);
    subject.add_css_class("dim-label");
    subject.set_ellipsize(gtk4::pango::EllipsizeMode::End);
    ui.git_diff_subject.replace(Some(subject.clone()));
    header.append(&subject);

    let spacer = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
    spacer.set_hexpand(true);
    header.append(&spacer);

    let close = gtk4::Button::from_icon_name("window-close-symbolic");
    close.add_css_class("flat");
    compat::initial_tooltip(&close, &tr(lang, "command.close"));
    a11y(&close, "pitex.git.diff.close", "command.close");
    {
        let state = state.clone();
        close.connect_clicked(move |_| {
            let Ok(mut s) = state.try_borrow_mut() else {
                return;
            };
            s.git_close_diff();
        });
    }
    header.append(&close);
    container.append(&header);
    container.append(&gtk4::Separator::new(gtk4::Orientation::Horizontal));

    let stack = gtk4::Stack::new();
    stack.set_vexpand(true);
    let loading = gtk4::Spinner::new();
    loading.set_halign(gtk4::Align::Center);
    loading.set_valign(gtk4::Align::Center);
    loading.start();
    stack.add_named(&loading, Some("loading"));
    let error = gtk4::Label::new(None);
    error.add_css_class("monospace");
    error.add_css_class("error");
    error.set_wrap(true);
    error.set_margin_top(12);
    error.set_valign(gtk4::Align::Start);
    ui.git_diff_error.replace(Some(error.clone()));
    stack.add_named(&error, Some("error"));
    let empty = gtk4::Label::new(Some(&tr(lang, "git.no_changes")));
    empty.add_css_class("dim-label");
    stack.add_named(&empty, Some("empty"));

    let model = DiffList::new();
    let selection = gtk4::NoSelection::new(Some(model.clone()));
    let factory = gtk4::SignalListItemFactory::new();
    let factory_model = model.clone();
    factory.connect_bind(move |_, object| {
        let item = object.downcast_ref::<gtk4::ListItem>().unwrap();
        let row = item.item().unwrap().downcast::<gtk4::glib::BoxedAnyObject>().unwrap();
        let row = row.borrow::<DiffRow>();
        item.set_selectable(false);
        item.set_activatable(false);
        item.set_child(Some(&diff_row_widget(&row, &factory_model, lang)));
    });
    factory.connect_unbind(|_, object| {
        object.downcast_ref::<gtk4::ListItem>().unwrap().set_child(None::<&gtk4::Widget>);
    });
    let list = gtk4::ListView::new(Some(selection), Some(factory));
    let scroll = gtk4::ScrolledWindow::new();
    scroll.set_vexpand(true);
    scroll.set_child(Some(&list));
    stack.add_named(&scroll, Some("rows"));
    stack.set_visible_child_name("loading");
    ui.git_diff_stack.replace(Some(stack.clone()));
    ui.git_diff_model.replace(Some(model));
    container.append(&stack);
    container.upcast()
}

/// One `DiffRow` → its row widget (`diffItem` on macOS).
fn diff_row_widget(row: &DiffRow, model: &DiffList, lang: &'static str) -> gtk4::Widget {
    let dark = model.imp().dark.get();
    let bg = model.imp().editor_bg.get();
    match row {
        DiffRow::Pair { left, right } => pair_row(left.as_ref(), right.as_ref(), dark, bg),
        DiffRow::Fold { section, id, count } => fold_row(*section, *id, *count, model, lang),
        DiffRow::Gap { count } => folded_bar(*count, lang).upcast(),
        DiffRow::Note(text) => {
            let label = gtk4::Label::new(Some(text));
            mono_small(&label);
            label.add_css_class("dim-label");
            label.set_xalign(0.0);
            label.set_margin_start(10);
            label.set_margin_end(10);
            label.set_margin_top(1);
            label.set_margin_bottom(1);
            label.upcast()
        }
        DiffRow::Binary => {
            let label = gtk4::Label::new(Some(&tr(lang, "git.diff.binary")));
            mono_small(&label);
            label.add_css_class("dim-label");
            label.set_margin_top(4);
            label.set_margin_bottom(4);
            label.upcast()
        }
        DiffRow::FileHeader {
            section,
            file,
            additions,
            deletions,
            collapsed,
        } => file_header(*section, file, *additions, *deletions, *collapsed, model),
    }
}

/// Collapsible file banner between sections (VSCode's multi-diff header).
fn file_header(
    section: usize,
    file: &GitCommitFile,
    additions: usize,
    deletions: usize,
    collapsed: bool,
    model: &DiffList,
) -> gtk4::Widget {
    let button = gtk4::Button::new();
    button.add_css_class("flat");
    button.add_css_class("pitex-diff-filehdr");
    button.set_widget_name(&format!("pitex.git.diff.file.{}", file.path));
    let hbox = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    hbox.set_margin_start(10);
    hbox.set_margin_end(10);
    hbox.set_margin_top(5);
    hbox.set_margin_bottom(5);
    let chevron = gtk4::Image::from_icon_name(if collapsed {
        "pan-end-symbolic"
    } else {
        "pan-down-symbolic"
    });
    chevron.add_css_class("dim-label");
    hbox.append(&chevron);
    let badge = gtk4::Label::new(Some(file.kind.badge()));
    mono_small(&badge);
    badge.add_css_class(badge_css_class(file.kind));
    badge.set_width_chars(2);
    hbox.append(&badge);
    let path = gtk4::Label::new(Some(&file.path));
    path.add_css_class("monospace");
    path.set_ellipsize(gtk4::pango::EllipsizeMode::Middle);
    path.set_xalign(0.0);
    hbox.append(&path);
    let adds = gtk4::Label::new(Some(&format!("+{additions}")));
    mono_small(&adds);
    adds.add_css_class("success");
    hbox.append(&adds);
    let dels = gtk4::Label::new(Some(&format!("−{deletions}")));
    mono_small(&dels);
    dels.add_css_class("error");
    hbox.append(&dels);
    let spacer = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
    spacer.set_hexpand(true);
    hbox.append(&spacer);
    button.set_child(Some(&hbox));
    {
        let model = model.clone();
        button.connect_clicked(move |_| model.toggle_file(section));
    }
    button.upcast()
}

/// One aligned row: original line on the left, new line on the right,
/// with a 1px divider between the columns like VSCode's diff editor.
fn pair_row(
    left: Option<&GitDiffLine>,
    right: Option<&GitDiffLine>,
    dark: bool,
    bg: (f64, f64, f64),
) -> gtk4::Widget {
    let hbox = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
    hbox.append(&diff_cell(left, right, dark, bg));
    hbox.append(&gtk4::Separator::new(gtk4::Orientation::Vertical));
    hbox.append(&diff_cell(right, left, dark, bg));
    hbox.upcast()
}

/// One side of a pair row: fixed-width gutter number + wrapping text,
/// tinted by kind (`diffCell`/`diffCellBackground` on macOS).
fn diff_cell(
    line: Option<&GitDiffLine>,
    other: Option<&GitDiffLine>,
    dark: bool,
    bg: (f64, f64, f64),
) -> gtk4::Widget {
    let cell = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
    cell.set_hexpand(true);
    cell.set_valign(gtk4::Align::Start);
    let number = gtk4::Label::new(line.map(|l| l.number.to_string()).as_deref());
    number.add_css_class("dim-label");
    number.set_width_chars(5);
    number.set_xalign(1.0);
    number.set_valign(gtk4::Align::Start);
    number.set_margin_end(8);
    cell.append(&number);
    if let Some(line) = line {
        let text = gtk4::Label::new(None);
        // Long lines wrap inside their cell instead of scrolling — a
        // long paragraph can never push the new side off-screen.
        text.set_wrap(true);
        text.set_wrap_mode(gtk4::pango::WrapMode::WordChar);
        text.set_xalign(0.0);
        text.set_hexpand(true);
        text.set_valign(gtk4::Align::Start);
        if let Some(markup) = intra_line_markup(line, other, dark, bg) {
            text.set_markup(&markup);
        } else {
            text.set_text(&line.text);
        }
        cell.append(&text);
    }
    cell.add_css_class(match line.map(|l| l.kind) {
        None => "pitex-diff-empty",
        Some(GitDiffLineKind::Removed) => "pitex-diff-removed",
        Some(GitDiffLineKind::Added) => "pitex-diff-added",
        Some(GitDiffLineKind::Context) => "pitex-diff-context",
    });
    cell.upcast()
}

/// The "⋯ N unchanged lines" bar spanning both panes — VSCode's
/// collapsed-region widget look.
fn folded_bar(count: usize, lang: &'static str) -> gtk4::Box {
    let hbox = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
    hbox.add_css_class("pitex-diff-fold");
    hbox.set_halign(gtk4::Align::Fill);
    let icon = gtk4::Image::from_icon_name("view-more-symbolic");
    icon.add_css_class("dim-label");
    hbox.append(&icon);
    let label = gtk4::Label::new(Some(&tr1(lang, "git.folded_lines", &count.to_string())));
    mono_small(&label);
    label.add_css_class("dim-label");
    hbox.append(&label);
    hbox
}

/// A collapsed fold — same bar, but activating expands the region.
fn fold_row(section: usize, id: u32, count: usize, model: &DiffList, lang: &'static str) -> gtk4::Widget {
    let button = gtk4::Button::new();
    button.add_css_class("flat");
    button.set_widget_name("pitex.git.diff.fold");
    button.set_child(Some(&folded_bar(count, lang)));
    {
        let model = model.clone();
        button.connect_clicked(move |_| model.expand_fold(section, id));
    }
    button.upcast()
}

/// `highlighted(_:other:)` — when a removed line is paired with an added
/// one, `common_affixes` trims the shared head/tail and the middle gets
/// the darker change-range shading. Returns Pango markup with the span
/// background pre-blended against the theme colors.
fn intra_line_markup(
    line: &GitDiffLine,
    other: Option<&GitDiffLine>,
    dark: bool,
    bg: (f64, f64, f64),
) -> Option<String> {
    let other = other?;
    if !matches!(
        (line.kind, other.kind),
        (GitDiffLineKind::Removed, GitDiffLineKind::Added)
            | (GitDiffLineKind::Added, GitDiffLineKind::Removed)
    ) {
        return None;
    }
    let (prefix, suffix) = git_core::common_affixes(&line.text, &other.text);
    let offsets: Vec<usize> = line.text.char_indices().map(|(i, _)| i).collect();
    if offsets.len() <= prefix + suffix {
        return None;
    }
    let start = offsets[prefix];
    let end = offsets
        .get(offsets.len() - suffix)
        .copied()
        .unwrap_or(line.text.len());
    let color = intra_line_hex(line.kind == GitDiffLineKind::Removed, dark, bg);
    let esc = glib::markup_escape_text;
    Some(format!(
        "{}<span background=\"{}\">{}</span>{}",
        esc(&line.text[..start]),
        color,
        esc(&line.text[start..end]),
        esc(&line.text[end..]),
    ))
}

/// The intra-line span color: `NSColor.systemRed`/`systemGreen` at
/// macOS's alphas (0.38/0.34), pre-blended over the row's 16% tint on
/// the editor background — Pango markup backgrounds take solid colors.
fn intra_line_hex(removed: bool, dark: bool, bg: (f64, f64, f64)) -> String {
    // NSColor.systemRed / systemGreen — light then dark.
    let tint = match (removed, dark) {
        (true, false) => (1.0, 0.231, 0.188),
        (true, true) => (1.0, 0.271, 0.227),
        (false, false) => (0.157, 0.804, 0.255),
        (false, true) => (0.188, 0.820, 0.345),
    };
    let alpha = if removed { 0.38 } else { 0.34 };
    // Composite tint over the cell's own 16% tint on the editor bg.
    let cell = blend(tint, bg, 0.16);
    to_hex(blend(tint, cell, alpha))
}

fn blend(fg: (f64, f64, f64), bg: (f64, f64, f64), a: f64) -> (f64, f64, f64) {
    (
        fg.0 * a + bg.0 * (1.0 - a),
        fg.1 * a + bg.1 * (1.0 - a),
        fg.2 * a + bg.2 * (1.0 - a),
    )
}

fn to_hex((r, g, b): (f64, f64, f64)) -> String {
    format!(
        "#{:02X}{:02X}{:02X}",
        (r.clamp(0.0, 1.0) * 255.0).round() as u8,
        (g.clamp(0.0, 1.0) * 255.0).round() as u8,
        (b.clamp(0.0, 1.0) * 255.0).round() as u8,
    )
}
