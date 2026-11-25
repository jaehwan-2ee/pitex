//! GTK drawing of assistant Markdown (`chat_markdown` blocks) in the agent
//! transcript: Pango-markup labels, code boxes with Copy, tables, and TeX
//! math typeset by the exact equation renderer (the project's TeX engine
//! path the equation preview uses), with the TeX source as the placeholder
//! and the fallback when no TeX is installed.

use crate::chat_markdown::{self, Block, Inline, LinkTarget};
use build_core::equation_exact::{ExactEquationEngine, ExactEquationProfile, ExactEquationRenderer, ExactEquationShellEscape};
use gtk4::prelude::*;
use gtk4::{gdk, gio, glib};
use language_core::equation_preview::ExactEquationDocument;
use language_core::{DeterministicTeXLexer, LanguageTokenKind, TeXDialect};
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;

/// Colors the transcript takes from the app palette; part of the row key,
/// so a theme change redraws the messages.
#[derive(Clone, Debug, Default, Hash, PartialEq, Eq)]
pub struct ChatTheme {
    pub dark: bool,
    /// LaTeX token colors (`#rrggbb`) for ```latex blocks.
    pub commands: String,
    pub comments: String,
    pub braces: String,
    pub environments: String,
    pub math: String,
}

struct Context {
    size: f64,
    root: Option<PathBuf>,
    theme: ChatTheme,
}

/// One assistant message: its blocks, plus a note when it was cut.
pub fn markdown_view(text: &str, streaming: bool, size: f64, root: Option<&Path>, theme: &ChatTheme) -> gtk4::Widget {
    let document = chat_markdown::parse(text, streaming);
    let context = Context { size, root: root.map(Path::to_path_buf), theme: theme.clone() };
    let column = gtk4::Box::new(gtk4::Orientation::Vertical, 6);
    for block in &document.blocks {
        column.append(&block_widget(block, &context));
    }
    if document.truncated {
        let note = gtk4::Label::new(Some(&crate::l10n::tr(crate::app_ui::language(), "assistant.markdown.truncated")));
        note.add_css_class("dim-label");
        note.set_xalign(0.0);
        note.set_wrap(true);
        note.set_attributes(Some(&crate::app_ui::font_attrs((size - 2.0).max(9.0))));
        column.append(&note);
    }
    column.upcast()
}

fn block_widget(block: &Block, context: &Context) -> gtk4::Widget {
    match block {
        Block::Paragraph(inlines) => inline_widget(inlines, context.size, false, context),
        Block::Heading(level, inlines) => {
            let scale = match level { 1 => 1.4, 2 => 1.25, 3 => 1.1, _ => 1.0 };
            inline_widget(inlines, context.size * scale, true, context)
        }
        Block::List { ordered, start, items } => {
            let list = gtk4::Box::new(gtk4::Orientation::Vertical, 3);
            let last = if *ordered { format!("{}.", start + items.len().saturating_sub(1) as u64) } else { "•".into() };
            // Room for the widest marker: digits are ~0.6 em, plus a gap.
            let indent = (context.size * (0.6 * last.chars().count() as f64 + 0.6)).ceil() as i32;
            for (index, item) in items.iter().enumerate() {
                // The marker floats over the item's left margin: a horizontal
                // box here multiplied height-for-width passes per nesting level
                // (seconds for a long item four levels deep).
                let content = gtk4::Box::new(gtk4::Orientation::Vertical, 4);
                content.set_margin_start(indent);
                for block in item {
                    content.append(&block_widget(block, context));
                }
                let row = gtk4::Overlay::new();
                row.set_child(Some(&content));
                let marker = if *ordered { format!("{}.", start + index as u64) } else { "•".to_string() };
                let marker = gtk4::Label::new(Some(&marker));
                marker.set_halign(gtk4::Align::Start);
                marker.set_valign(gtk4::Align::Start);
                marker.set_attributes(Some(&crate::app_ui::font_attrs(context.size)));
                row.add_overlay(&marker);
                list.append(&row);
            }
            list.upcast()
        }
        Block::Quote(blocks) => {
            let quote = gtk4::Box::new(gtk4::Orientation::Vertical, 4);
            quote.add_css_class("pitex-md-quote");
            for block in blocks {
                let widget = block_widget(block, context);
                // A nested quote dims its own text; dimming it here too would compound.
                if !matches!(block, Block::Quote(_)) {
                    widget.add_css_class("dim-label");
                }
                quote.append(&widget);
            }
            quote.upcast()
        }
        Block::Code { language, text } => code_block(language, text, context),
        Block::Math(tex) => {
            let math = math_widget(tex, true, context);
            math.set_halign(gtk4::Align::Center);
            math
        }
        Block::Table(rows) => {
            let grid = gtk4::Grid::new();
            grid.set_column_spacing(14);
            grid.set_row_spacing(4);
            for (r, row) in rows.iter().enumerate() {
                for (c, cell) in row.iter().enumerate() {
                    let widget = inline_widget(cell, context.size, r == 0, context);
                    widget.add_css_class("pitex-md-cell");
                    // Wrapped cells would be measured at their narrowest inside the
                    // scroller (one word per line); the table scrolls sideways instead.
                    if let Some(label) = widget.downcast_ref::<gtk4::Label>() {
                        label.set_wrap(false);
                    } else if let Some(view) = widget.downcast_ref::<gtk4::TextView>() {
                        view.set_wrap_mode(gtk4::WrapMode::None);
                    }
                    grid.attach(&widget, c as i32, r as i32, 1, 1);
                }
            }
            let scroller = gtk4::ScrolledWindow::new();
            scroller.set_policy(gtk4::PolicyType::Automatic, gtk4::PolicyType::Never);
            scroller.set_propagate_natural_height(true);
            scroller.set_child(Some(&grid));
            scroller.upcast()
        }
        Block::Rule => gtk4::Separator::new(gtk4::Orientation::Horizontal).upcast(),
    }
}

fn has_math(inlines: &[Inline]) -> bool {
    inlines.iter().any(|inline| match inline {
        Inline::Math(_) => true,
        Inline::Strong(children) | Inline::Emphasis(children) | Inline::Link { children, .. } => has_math(children),
        _ => false,
    })
}

/// A selectable label, or a read-only text view when inline math needs
/// pictures in the text flow.
fn inline_widget(inlines: &[Inline], size: f64, bold: bool, context: &Context) -> gtk4::Widget {
    if has_math(inlines) {
        return rich_text(inlines, size, bold, context);
    }
    let mut markup = String::new();
    inline_markup(inlines, context.root.as_deref(), true, &mut markup);
    if bold {
        markup = format!("<b>{markup}</b>");
    }
    let label = gtk4::Label::new(None);
    label.set_markup(&markup);
    label.set_xalign(0.0);
    label.set_wrap(true);
    label.set_wrap_mode(gtk4::pango::WrapMode::WordChar);
    label.set_selectable(true);
    label.set_attributes(Some(&crate::app_ui::font_attrs(size)));
    let root = context.root.clone();
    label.connect_activate_link(move |_, uri| {
        crate::app_ui::open_chat_link(uri, root.as_deref());
        glib::Propagation::Stop
    });
    label.upcast()
}

/// Pango markup for inline content. `links` emits `<a href>` (GtkLabel
/// only); links that go nowhere render as their text.
fn inline_markup(inlines: &[Inline], root: Option<&Path>, links: bool, out: &mut String) {
    for inline in inlines {
        match inline {
            Inline::Text(text) => out.push_str(&glib::markup_escape_text(text)),
            Inline::Strong(children) => {
                out.push_str("<b>");
                inline_markup(children, root, links, out);
                out.push_str("</b>");
            }
            Inline::Emphasis(children) => {
                out.push_str("<i>");
                inline_markup(children, root, links, out);
                out.push_str("</i>");
            }
            Inline::Code(code) => {
                out.push_str("<span font_family=\"monospace\" background=\"#808080\" bgalpha=\"22%\">");
                out.push_str(&glib::markup_escape_text(code));
                out.push_str("</span>");
            }
            Inline::Link { url, children } => {
                let live = links && chat_markdown::classify_link(url, root) != LinkTarget::Unsupported;
                if live {
                    out.push_str(&format!("<a href=\"{}\">", glib::markup_escape_text(url)));
                }
                inline_markup(children, root, links, out);
                if live {
                    out.push_str("</a>");
                }
            }
            // Math nested in other inlines, or a label without pictures: the source.
            Inline::Math(tex) => {
                out.push_str("<span font_family=\"monospace\">");
                out.push_str(&glib::markup_escape_text(&format!("${tex}$")));
                out.push_str("</span>");
            }
        }
    }
}

/// Inline content with math pictures anchored in the text flow, at any
/// depth (`**bold $x$**`). Links are tagged ranges opened by click or Enter
/// (a text view has no `<a>` markup).
fn rich_text(inlines: &[Inline], size: f64, bold: bool, context: &Context) -> gtk4::Widget {
    let view = gtk4::TextView::new();
    view.set_editable(false);
    view.set_wrap_mode(gtk4::WrapMode::WordChar);
    view.add_css_class("pitex-md-text");
    let buffer = view.buffer();
    // Created first, so the bold/italic/markup tags inserted later win over it.
    let weight = if bold { 700 } else { 400 };
    let base = buffer.create_tag(None, &[("size-points", &size), ("weight", &weight)]);
    let link_color = if context.theme.dark { "#78aeed" } else { "#1c71d8" }; // Adwaita's link colors
    let tags = RichTags {
        bold: buffer.create_tag(None, &[("weight", &700)]),
        italic: buffer.create_tag(None, &[("style", &gtk4::pango::Style::Italic)]),
        link: buffer.create_tag(None, &[("underline", &gtk4::pango::Underline::Single), ("foreground", &link_color)]),
    };
    let links: Rc<RefCell<Vec<(i32, i32, String)>>> = Rc::default();
    append_rich(&view, inlines, &tags, &mut links.borrow_mut(), context);
    if let Some(tag) = &base {
        buffer.apply_tag(tag, &buffer.start_iter(), &buffer.end_iter());
    }
    if links.borrow().is_empty() {
        view.set_cursor_visible(false);
        return view.upcast();
    }
    let link_at = {
        let links = links.clone();
        move |offset: i32| links.borrow().iter().find(|(s, e, _)| (*s..*e).contains(&offset)).map(|(_, _, url)| url.clone())
    };
    let at_point = |view: &gtk4::TextView, x: f64, y: f64| {
        let (bx, by) = view.window_to_buffer_coords(gtk4::TextWindowType::Widget, x as i32, y as i32);
        view.iter_at_location(bx, by).map(|iter| iter.offset())
    };
    let root = context.root.clone();
    let click = gtk4::GestureClick::new();
    click.connect_released({
        let (view, root, link_at) = (view.clone(), root.clone(), link_at.clone());
        move |_, _, x, y| {
            if view.buffer().has_selection() {
                return;
            }
            if let Some(url) = at_point(&view, x, y).and_then(&link_at) {
                crate::app_ui::open_chat_link(&url, root.as_deref());
            }
        }
    });
    view.add_controller(click);
    let motion = gtk4::EventControllerMotion::new();
    motion.connect_motion({
        let (view, link_at) = (view.clone(), link_at.clone());
        move |_, x, y| {
            let over = at_point(&view, x, y).and_then(&link_at).is_some();
            view.set_cursor_from_name(Some(if over { "pointer" } else { "text" }));
        }
    });
    view.add_controller(motion);
    // Keyboard: the caret (shown while focused) on a link, then Enter.
    let keys = gtk4::EventControllerKey::new();
    keys.connect_key_pressed({
        let view = view.clone();
        move |_, key, _, _| {
            if !matches!(key, gdk::Key::Return | gdk::Key::KP_Enter) {
                return glib::Propagation::Proceed;
            }
            let buffer = view.buffer();
            match link_at(buffer.iter_at_mark(&buffer.get_insert()).offset()) {
                Some(url) => {
                    crate::app_ui::open_chat_link(&url, root.as_deref());
                    glib::Propagation::Stop
                }
                None => glib::Propagation::Proceed,
            }
        }
    });
    view.add_controller(keys);
    view.upcast()
}

struct RichTags {
    bold: Option<gtk4::TextTag>,
    italic: Option<gtk4::TextTag>,
    link: Option<gtk4::TextTag>,
}

fn append_rich(view: &gtk4::TextView, inlines: &[Inline], tags: &RichTags, links: &mut Vec<(i32, i32, String)>, context: &Context) {
    let buffer = view.buffer();
    for inline in inlines {
        let start = buffer.end_iter().offset();
        let tag = match inline {
            Inline::Math(tex) => {
                let anchor = buffer.create_child_anchor(&mut buffer.end_iter());
                view.add_child_at_anchor(&math_widget(tex, false, context), &anchor);
                None
            }
            Inline::Strong(children) => {
                append_rich(view, children, tags, links, context);
                tags.bold.as_ref()
            }
            Inline::Emphasis(children) => {
                append_rich(view, children, tags, links, context);
                tags.italic.as_ref()
            }
            Inline::Link { url, children } => {
                append_rich(view, children, tags, links, context);
                let live = chat_markdown::classify_link(url, context.root.as_deref()) != LinkTarget::Unsupported;
                if live {
                    links.push((start, buffer.end_iter().offset(), url.clone()));
                }
                tags.link.as_ref().filter(|_| live)
            }
            Inline::Text(_) | Inline::Code(_) => {
                let mut markup = String::new();
                inline_markup(std::slice::from_ref(inline), None, false, &mut markup);
                buffer.insert_markup(&mut buffer.end_iter(), &markup);
                None
            }
        };
        if let Some(tag) = tag {
            buffer.apply_tag(tag, &buffer.iter_at_offset(start), &buffer.end_iter());
        }
    }
}

fn code_block(language: &str, code: &str, context: &Context) -> gtk4::Widget {
    let lang = crate::app_ui::language();
    let caption = (context.size - 2.0).max(9.0);
    let frame = gtk4::Box::new(gtk4::Orientation::Vertical, 2);
    frame.add_css_class("pitex-md-code");
    let header = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
    let name = gtk4::Label::new(Some(language));
    name.add_css_class("dim-label");
    name.set_xalign(0.0);
    name.set_attributes(Some(&crate::app_ui::font_attrs(caption)));
    header.append(&name);
    header.append(&copy_button(code, "assistant.markdown.copy", lang));
    frame.append(&header);
    let body = gtk4::Label::new(None);
    match language.to_ascii_lowercase().as_str() {
        "latex" | "tex" => body.set_markup(&latex_markup(code, TeXDialect::Latex, &context.theme)),
        "bibtex" | "bib" => body.set_markup(&latex_markup(code, TeXDialect::Bibtex, &context.theme)),
        _ => body.set_text(code),
    }
    body.set_xalign(0.0);
    body.set_selectable(true);
    body.add_css_class("monospace");
    body.set_attributes(Some(&crate::app_ui::font_attrs(caption)));
    let scroller = gtk4::ScrolledWindow::new();
    scroller.set_policy(gtk4::PolicyType::Automatic, gtk4::PolicyType::Never);
    scroller.set_propagate_natural_height(true);
    scroller.set_child(Some(&body));
    frame.append(&scroller);
    frame.upcast()
}

/// A flat Copy button putting `text` on the clipboard; it reads "Copied"
/// for 1.5 s after (Paseo's `COPIED_RESET_MS`).
fn copy_button(text: &str, label_key: &'static str, lang: &'static str) -> gtk4::Button {
    let button = gtk4::Button::with_label(&crate::l10n::tr(lang, label_key));
    button.add_css_class("flat");
    button.set_valign(gtk4::Align::Center);
    crate::app_ui::a11y(&button, "pitex.assistant.markdown.copy", label_key);
    let text = text.to_string();
    button.connect_clicked(move |button| {
        button.clipboard().set_text(&text);
        let show = move |button: &gtk4::Button, key: &str| {
            button.set_label(&crate::l10n::tr(lang, key));
            button.update_property(&[gtk4::accessible::Property::Label(&crate::l10n::tr(lang, key))]);
        };
        show(button, "assistant.markdown.copied");
        let weak = button.downgrade();
        glib::timeout_add_local_once(std::time::Duration::from_millis(1500), move || {
            if let Some(button) = weak.upgrade() {
                show(&button, label_key);
            }
        });
    });
    button
}

/// The editor's LaTeX lexer and palette on a plain string.
fn latex_markup(code: &str, dialect: TeXDialect, theme: &ChatTheme) -> String {
    let mut out = String::new();
    let mut at = 0;
    for token in DeterministicTeXLexer::tokenize(code, dialect) {
        let (start, end) = (token.range.utf8_offset as usize, token.range.end_utf8_offset() as usize);
        if start < at || end > code.len() || !code.is_char_boundary(start) || !code.is_char_boundary(end) {
            continue;
        }
        let color = match token.kind {
            LanguageTokenKind::ControlSequence(_) | LanguageTokenKind::BibEntryMarker => &theme.commands,
            LanguageTokenKind::Comment(_) => &theme.comments,
            LanguageTokenKind::LeftBrace | LanguageTokenKind::RightBrace | LanguageTokenKind::Punctuation(_) => &theme.braces,
            LanguageTokenKind::EnvironmentName(_) => &theme.environments,
            LanguageTokenKind::Math(_) => &theme.math,
            _ => continue,
        };
        out.push_str(&glib::markup_escape_text(&code[at..start]));
        let piece = glib::markup_escape_text(&code[start..end]);
        if color.is_empty() {
            out.push_str(&piece);
        } else {
            out.push_str(&format!("<span foreground=\"{color}\">{piece}</span>"));
        }
        at = end;
    }
    out.push_str(&glib::markup_escape_text(&code[at..]));
    out
}

// ─── math ────────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
struct MathKey {
    tex: String,
    display: bool,
    /// Pixels per TeX point × 1000.
    scale: u32,
    dark: bool,
}

// shortcut: the cache and the pdflatex jobs are unbounded (one per distinct
// formula, all at once); add an LRU cap and a job queue if long math-heavy
// chats show memory or CPU spikes.
#[derive(Default)]
struct MathCache {
    /// Typeset pictures; `None` = TeX failed or is missing (keep the source).
    done: HashMap<MathKey, Option<gdk::Texture>>,
    /// Holders waiting on a running job.
    waiting: HashMap<MathKey, Vec<glib::WeakRef<gtk4::Box>>>,
}

thread_local! {
    static MATH: RefCell<MathCache> = RefCell::default();
}

/// The TeX source in monospace, replaced by the typeset picture when the
/// job finishes (or at once when cached).
fn math_widget(tex: &str, display: bool, context: &Context) -> gtk4::Widget {
    let holder = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
    holder.set_tooltip_text(Some(tex));
    let source = gtk4::Label::new(Some(&if display { tex.to_string() } else { format!("${tex}$") }));
    source.add_css_class("monospace");
    source.set_selectable(true);
    source.set_wrap(display);
    source.set_attributes(Some(&crate::app_ui::font_attrs((context.size - 1.0).max(9.0))));
    holder.append(&source);
    let window_scale = crate::app_ui::main_window().map_or(1, |w| w.scale_factor()).max(1) as f64;
    // TeX sets 10pt text; match the transcript's point size at 96 dpi.
    let scale = context.size / 10.0 * (96.0 / 72.0) * window_scale;
    let key = MathKey { tex: tex.to_string(), display, scale: (scale * 1000.0) as u32, dark: context.theme.dark };
    let start = MATH.with(|cache| {
        let mut cache = cache.borrow_mut();
        if let Some(done) = cache.done.get(&key) {
            if let Some(texture) = done {
                show_math(&holder, texture, window_scale, tex);
            }
            return false;
        }
        let waiters = cache.waiting.entry(key.clone()).or_default();
        waiters.push(holder.downgrade());
        waiters.len() == 1
    });
    if start {
        glib::spawn_future_local(async move {
            let job = key.clone();
            let image = gio::spawn_blocking(move || typeset(&job.tex, job.display, scale, job.dark)).await.ok().flatten();
            let texture = image.map(|(pixels, width, height, stride)| crate::pdf::texture_from_pixels(pixels.into(), width, height, stride));
            let waiters = MATH.with(|cache| {
                let mut cache = cache.borrow_mut();
                cache.done.insert(key.clone(), texture.clone());
                cache.waiting.remove(&key).unwrap_or_default()
            });
            if let Some(texture) = texture {
                for holder in waiters.iter().filter_map(|w| w.upgrade()) {
                    show_math(&holder, &texture, window_scale, &key.tex);
                }
            }
        });
    }
    holder.upcast()
}

fn show_math(holder: &gtk4::Box, texture: &gdk::Texture, window_scale: f64, tex: &str) {
    while let Some(child) = holder.first_child() {
        holder.remove(&child);
    }
    let picture = gtk4::Picture::for_paintable(texture);
    picture.set_can_shrink(true);
    picture.set_size_request(
        (texture.width() as f64 / window_scale).ceil() as i32,
        (texture.height() as f64 / window_scale).ceil() as i32,
    );
    picture.update_property(&[gtk4::accessible::Property::Label(tex)]);
    holder.append(&picture);
}

/// pdflatex (no shell escape) on a one-formula document, cropped to the ink
/// and turned into an alpha mask in the text color. Runs off the main thread.
fn typeset(tex: &str, display: bool, scale: f64, dark: bool) -> Option<(Vec<u8>, i32, i32, i32)> {
    let region = if display { format!("\\[{tex}\\]") } else { format!("${tex}$") };
    let document = ExactEquationDocument::make(None, &[], &[], &region);
    let prefix = crate::identity::current().temp_prefix;
    let token = "assistant".to_string();
    let renderer = ExactEquationRenderer::new(
        ExactEquationProfile { engine: ExactEquationEngine::PdfLaTeX, shell_escape: ExactEquationShellEscape::Disabled },
        // Private per-user directory: nothing in a shared /tmp joins TEXINPUTS.
        ExactEquationRenderer::workspace_root(prefix, &token),
        build_core::augmented_environment(&HashMap::new()),
        prefix,
        token,
    );
    let pdf = renderer.render(&document, None).ok()?;
    let (mut pixels, width, height, stride) = crate::pdf::cropped_first_page(&pdf, scale, if display { 3.0 } else { 1.0 })?;
    ink_mask(&mut pixels, dark);
    Some((pixels, width, height, stride))
}

/// Black-on-white BGRA → premultiplied text-colored ink on transparency.
fn ink_mask(pixels: &mut [u8], dark: bool) {
    let ink: u32 = if dark { 0xEE } else { 0x1E };
    for pixel in pixels.chunks_exact_mut(4) {
        let gray = (pixel[0] as u32 + pixel[1] as u32 + pixel[2] as u32) / 3;
        let alpha = 255 - gray;
        let value = (ink * alpha / 255) as u8;
        pixel[..3].fill(value);
        pixel[3] = alpha as u8;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "needs a display"]
    fn nested_lists_with_long_items_measure_in_linear_time() {
        gtk4::init().unwrap();
        let text = format!("{}{}", "+ ".repeat(4), ">".repeat(1000));
        let view = markdown_view(&text, false, 13.0, None, &ChatTheme::default());
        let started = std::time::Instant::now();
        let _ = view.measure(gtk4::Orientation::Vertical, 500);
        let _ = view.measure(gtk4::Orientation::Horizontal, -1);
        // Row boxes took ~1.6 s here; the overlay rows ~25 ms.
        assert!(started.elapsed() < std::time::Duration::from_millis(400), "{:?}", started.elapsed());
    }

    #[test]
    fn ink_mask_makes_paper_transparent_and_ink_opaque() {
        let mut pixels = vec![255, 255, 255, 255, 0, 0, 0, 255];
        ink_mask(&mut pixels, true);
        assert_eq!(pixels, vec![0, 0, 0, 0, 0xEE, 0xEE, 0xEE, 255]);
    }

    #[test]
    fn broken_tex_or_no_tex_keeps_the_source() {
        // None with or without a TeX install: the view keeps the monospace source.
        assert!(typeset("\\undefinedcontrolsequence", false, 1.0, false).is_none());
    }

    #[test]
    fn latex_blocks_take_the_palette() {
        let theme = ChatTheme { commands: "#112233".into(), comments: "#445566".into(), ..Default::default() };
        let markup = latex_markup("\\section{A} % note <x>", TeXDialect::Latex, &theme);
        assert!(markup.starts_with("<span foreground=\"#112233\">\\section</span>"), "{markup}");
        assert!(markup.contains("<span foreground=\"#445566\">% note &lt;x&gt;</span>"), "{markup}");
    }

    #[test]
    fn link_markup_only_for_reachable_targets() {
        let mut out = String::new();
        let inlines = chat_markdown::parse_inlines("[a](https://x.org?q=1&b=2) [b](../out.tex) [c](main.tex:3)");
        inline_markup(&inlines, Some(Path::new("/proj")), true, &mut out);
        assert_eq!(out, "<a href=\"https://x.org?q=1&amp;b=2\">a</a> b <a href=\"main.tex:3\">c</a>");
    }
}

