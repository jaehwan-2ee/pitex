//! SourceView completion provider — the GTK half of the macOS
//! `completionSource` wiring. Candidate data comes from a model-supplied
//! closure so the provider stays data-agnostic like the mac adapter; the
//! shared `CompletionContextDetector` decides when a popup should exist.

use gtk4::gio;
use gtk4::glib;
use gtk4::glib::subclass::prelude::*;
use gtk4::prelude::*;
use language_core::{
    CompletionContext, CompletionContextDetector, CompletionKind, LanguageCompletion,
};
use sourceview5::subclass::prelude::*;
use std::cell::{Cell, RefCell};
use std::pin::Pin;
use std::rc::Rc;

mod proposal_imp {
    use super::*;

    /// One popup row — the full candidate string (`\subsection`, `kim2026`)
    /// the activation path inserts, replacing only the detected prefix.
    pub struct TexProposal {
        pub text: RefCell<String>,
        pub kind: Cell<CompletionKind>,
    }
    impl Default for TexProposal {
        fn default() -> Self {
            Self {
                text: RefCell::new(String::new()),
                kind: Cell::new(CompletionKind::Command),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for TexProposal {
        const NAME: &'static str = "PitexTexCompletionProposal";
        type Type = super::TexProposal;
        type Interfaces = (sourceview5::CompletionProposal,);
    }
    impl ObjectImpl for TexProposal {}
    impl CompletionProposalImpl for TexProposal {}
}

glib::wrapper! {
    pub struct TexProposal(ObjectSubclass<proposal_imp::TexProposal>)
        @implements sourceview5::CompletionProposal;
}

impl TexProposal {
    fn new(text: &str, kind: CompletionKind) -> Self {
        let obj: Self = glib::Object::new();
        *obj.imp().text.borrow_mut() = text.to_string();
        obj.imp().kind.set(kind);
        obj
    }

    fn text(&self) -> String {
        self.imp().text.borrow().clone()
    }
}

mod provider_imp {
    use super::*;

    /// The data-source closure — supplied by the shell so the provider
    /// never touches the model directly (the macOS adapter's
    /// `completionSource` contract).
    pub type Candidates = Rc<dyn Fn(&CompletionContext) -> Vec<LanguageCompletion>>;

    pub struct TexCompletionProvider {
        pub candidates: RefCell<Option<Candidates>>,
        pub title: RefCell<String>,
    }
    impl Default for TexCompletionProvider {
        fn default() -> Self {
            Self {
                candidates: RefCell::new(None),
                title: RefCell::new(String::new()),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for TexCompletionProvider {
        const NAME: &'static str = "PitexTexCompletionProvider";
        type Type = super::TexCompletionProvider;
        type Interfaces = (sourceview5::CompletionProvider,);
    }
    impl ObjectImpl for TexCompletionProvider {}

    impl TexCompletionProvider {
        /// Shared context detection → candidates. `context.bounds()` end is
        /// the caret; the detector only scans backwards, so the text before
        /// it is all that is needed (and all that is copied).
        fn candidates_for(
            &self,
            context: &sourceview5::CompletionContext,
        ) -> Option<Vec<LanguageCompletion>> {
            let (_begin, end) = context.bounds()?;
            let buffer = context.buffer()?;
            let start = buffer.start_iter();
            let before = buffer.text(&start, &end, false);
            let caret_utf16 = before.encode_utf16().count();
            let ctx = CompletionContextDetector::context(&before, caret_utf16)?;
            let candidates = self.candidates.borrow();
            let items = candidates.as_ref()?(&ctx);
            if items.is_empty() {
                None
            } else {
                Some(items)
            }
        }

        fn fill(&self, store: &gio::ListStore, items: &[LanguageCompletion]) {
            for item in items {
                store.append(
                    TexProposal::new(&item.text, item.kind)
                        .upcast_ref::<sourceview5::CompletionProposal>(),
                );
            }
        }
    }

    /// Every vfunc is overridden — the generated interface registers
    /// trampolines for all of them and the default parent implementations
    /// are NULL on the base interface.
    impl CompletionProviderImpl for TexCompletionProvider {
        fn title(&self) -> Option<glib::GString> {
            let title = self.title.borrow();
            if title.is_empty() {
                None
            } else {
                Some(glib::GString::from(title.as_str()))
            }
        }

        fn priority(&self, _context: &sourceview5::CompletionContext) -> i32 {
            1
        }

        /// Interactive triggers: `\` starts a command prefix, `{` enters a
        /// group argument, `,` starts the next citation key.
        fn is_trigger(&self, _iter: &gtk4::TextIter, ch: char) -> bool {
            matches!(ch, '\\' | '{' | ',')
        }

        /// Only Return/Tab/Escape keep their defaults — every other key is
        /// text (and re-filters), never activation.
        fn key_activates(
            &self,
            _context: &sourceview5::CompletionContext,
            _proposal: &sourceview5::CompletionProposal,
            _keyval: gtk4::gdk::Key,
            _state: gtk4::gdk::ModifierType,
        ) -> bool {
            false
        }

        fn list_alternates(
            &self,
            _context: &sourceview5::CompletionContext,
            _proposal: &sourceview5::CompletionProposal,
        ) -> Vec<sourceview5::CompletionProposal> {
            Vec::new()
        }

        fn display(
            &self,
            _context: &sourceview5::CompletionContext,
            proposal: &sourceview5::CompletionProposal,
            cell: &sourceview5::CompletionCell,
        ) {
            if cell.column() != sourceview5::CompletionColumn::TypedText {
                return;
            }
            if let Some(proposal) = proposal.downcast_ref::<TexProposal>() {
                cell.set_text(Some(proposal.text().as_str()));
            }
        }

        /// The context's word grew or shrank — recompute candidates and
        /// swap the store in place, the contract `refilter` carries.
        fn refilter(&self, context: &sourceview5::CompletionContext, model: &gio::ListModel) {
            let Ok(store) = model.clone().downcast::<gio::ListStore>() else {
                return;
            };
            store.remove_all();
            if let Some(items) = self.candidates_for(context) {
                self.fill(&store, &items);
            }
        }

        /// Insert the proposal, replacing only the detected prefix range —
        /// `context.bounds()` stops at word characters, which would leave
        /// the leading `\` behind for command completions, so the shared
        /// detector's prefix range wins like `rangeForUserCompletion`.
        fn activate(
            &self,
            context: &sourceview5::CompletionContext,
            proposal: &sourceview5::CompletionProposal,
        ) {
            let Some((mut begin, mut end)) = context.bounds() else { return };
            let Some(buffer) = context.buffer() else { return };
            let Some(proposal) = proposal.downcast_ref::<TexProposal>() else { return };
            let start = buffer.start_iter();
            let before = buffer.text(&start, &end, false);
            let caret_utf16 = before.encode_utf16().count();
            if let Some(ctx) = CompletionContextDetector::context(&before, caret_utf16) {
                let char_offset = gtk_editor_adapter::utf16_offset_to_char_offset(
                    &before,
                    ctx.prefix_utf16_offset,
                );
                begin = buffer.iter_at_offset(char_offset as i32);
            }
            let text = proposal.text();
            buffer.begin_user_action();
            buffer.delete(&mut begin, &mut end);
            buffer.insert(&mut begin, &text);
            buffer.end_user_action();
        }

        fn populate(
            &self,
            context: &sourceview5::CompletionContext,
        ) -> Result<gio::ListModel, glib::Error> {
            let model = gio::ListStore::new::<sourceview5::CompletionProposal>();
            if let Some(items) = self.candidates_for(context) {
                self.fill(&model, &items);
            }
            Ok(model.upcast())
        }

        /// The base interface's async slot is NULL — answer immediately
        /// with the synchronous populate result instead of delegating up.
        fn populate_future(
            &self,
            context: &sourceview5::CompletionContext,
        ) -> Pin<Box<dyn std::future::Future<Output = Result<gio::ListModel, glib::Error>>>>
        {
            let result = self.populate(context);
            Box::pin(async move { result })
        }
    }
}

glib::wrapper! {
    pub struct TexCompletionProvider(ObjectSubclass<provider_imp::TexCompletionProvider>)
        @implements sourceview5::CompletionProvider;
}

impl TexCompletionProvider {
    /// `title` is the localized popup header; `candidates` maps a detected
    /// context to the project-wide candidate list.
    pub fn new(title: &str, candidates: provider_imp::Candidates) -> Self {
        let obj: Self = glib::Object::new();
        *obj.imp().candidates.borrow_mut() = Some(candidates);
        *obj.imp().title.borrow_mut() = title.to_string();
        obj
    }
}
