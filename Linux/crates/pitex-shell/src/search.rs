//! Keep one native search context per document: GTK counts asynchronously.
use gtk4::prelude::*;
use sourceview5::prelude::*;

pub(crate) struct EditorSearch {
    context: sourceview5::SearchContext,
    view: sourceview5::View,
}

fn update_count(context: &sourceview5::SearchContext, label: &gtk4::Label) {
    let total = context.occurrences_count();
    if total < 0 {
        label.set_text("…");
        return;
    }
    let position = context.buffer().selection_bounds()
        .map(|(start, end)| context.occurrence_position(&start, &end).max(0))
        .unwrap_or(0);
    label.set_text(&format!("{position} / {total}"));
}

impl EditorSearch {
    pub(crate) fn new(view: &sourceview5::View, buffer: &sourceview5::Buffer, label: &gtk4::Label) -> Self {
        let settings = sourceview5::SearchSettings::new();
        settings.set_case_sensitive(false);
        settings.set_wrap_around(true);
        let context = sourceview5::SearchContext::new(buffer, Some(&settings));
        context.set_highlight(true);
        let weak_label = label.downgrade();
        context.connect_occurrences_count_notify(move |context| {
            if let Some(label) = weak_label.upgrade() { update_count(context, &label); }
        });
        let weak_context = context.downgrade();
        let weak_label = label.downgrade();
        buffer.connect_mark_set(move |_, _, _| {
            if let (Some(context), Some(label)) = (weak_context.upgrade(), weak_label.upgrade()) {
                update_count(&context, &label);
            }
        });
        update_count(&context, label);
        Self { context, view: view.clone() }
    }

    pub(crate) fn set_query(&self, query: &str) {
        self.context.settings().set_search_text(if query.is_empty() { None } else { Some(query) });
    }

    pub(crate) fn step(&self, query: &str, forward: bool) {
        let changed = self.context.settings().search_text().as_deref() != Some(query);
        self.set_query(query);
        let buffer = self.context.buffer();
        // select_range places the insertion mark at either end depending on
        // selection direction. Always leave the current match before searching.
        let cursor = buffer.selection_bounds()
            .map(|(start, end)| if forward && !changed { end } else { start })
            .unwrap_or_else(|| buffer.iter_at_mark(&buffer.get_insert()));
        let found = if forward { self.context.forward(&cursor) } else { self.context.backward(&cursor) };
        if let Some((mut start, end, _)) = found {
            buffer.select_range(&end, &start);
            self.view.scroll_to_iter(&mut start, 0.1, false, 0.0, 0.0);
        }
    }

    pub(crate) fn replace(&self, query: &str, replacement: &str) {
        self.set_query(query);
        if let Some((start, end)) = self.context.buffer().selection_bounds() {
            if let Some((mut a, mut b, _)) = self.context.forward(&start) {
                if (a.offset(), b.offset()) == (start.offset(), end.offset()) {
                    let _ = self.context.replace(&mut a, &mut b, replacement);
                }
            }
        }
        self.step(query, true);
    }

    pub(crate) fn replace_all(&self, query: &str, replacement: &str) {
        self.set_query(query);
        let _ = self.context.replace_all(replacement);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn settle() {
        let context = gtk4::glib::MainContext::default();
        for _ in 0..20 {
            while context.pending() { context.iteration(false); }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    }
    #[test]
    #[ignore = "requires GTK display; run alone"]
    fn navigation_counts_edits_and_unicode() {
        gtk4::init().unwrap();
        let buffer = sourceview5::Buffer::new(None);
        let view = sourceview5::View::with_buffer(&buffer);
        let label = gtk4::Label::new(None);
        let search = EditorSearch::new(&view, &buffer, &label);
        buffer.set_text("한 😀 cat CAT cat");
        buffer.place_cursor(&buffer.start_iter());
        for expected in ["1 / 3", "2 / 3", "3 / 3", "1 / 3"] {
            search.step("cat", true); settle(); assert_eq!(label.text(), expected);
        }
        search.step("cat", false); settle(); assert_eq!(label.text(), "3 / 3");
        // A reversed selection must behave identically.
        let (start, end) = buffer.selection_bounds().unwrap();
        buffer.select_range(&start, &end);
        search.step("cat", false); settle(); assert_eq!(label.text(), "2 / 3");
        search.replace("cat", "dog"); settle(); assert_eq!(label.text(), "2 / 2");
        buffer.insert(&mut buffer.end_iter(), " cat"); settle(); assert_eq!(label.text(), "2 / 3");
        search.replace_all("cat", "dog"); settle(); assert_eq!(label.text(), "0 / 0");
        search.step("한", true); settle(); assert_eq!(label.text(), "1 / 1");
        search.step("missing", true); settle(); assert_eq!(label.text(), "0 / 0");
        search.set_query(""); settle(); assert_eq!(label.text(), "0 / 0");
    }
}
