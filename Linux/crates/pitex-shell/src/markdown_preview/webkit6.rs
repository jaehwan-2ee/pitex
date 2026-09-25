//! WebKitGTK 6.0 engine — the original backend. The WebView is a real GTK
//! widget composited by the toolkit, so there are no bounds/airspace
//! concerns; `set_covered` is a no-op here. See `markdown_preview.rs` for
//! the shared contract (`pitexRender`, `pitexScrollToLine`, `pitexScroll`,
//! `NavigationPolicy`).

use std::cell::RefCell;
use std::path::Path;

use gtk4::prelude::*;
use webkit6::prelude::*;

use super::{EngineHooks, NavVerdict, NavigationPolicy, PREVIEW_HTML};

/// The widget half of the preview — the WebView itself plus the hooks its
/// print failure handler still needs.
pub(super) struct Engine {
    /// `pub(super)` for `MarkdownPreview::webview_for_test`.
    pub(super) view: webkit6::WebView,
    hooks: EngineHooks,
}

/// WebKitGTK is a hard dependency of the feature — the engine always
/// exists, so `runtime_available` is unconditionally true.
pub(super) fn runtime_available() -> bool {
    true
}

impl Engine {
    /// Build the WebView under `host` and kick off the shared page load.
    /// Always succeeds — the `Option` return is for the Windows engine.
    pub(super) fn create(host: &gtk4::Box, hooks: EngineHooks) -> Option<Self> {
        let manager = webkit6::UserContentManager::new();
        manager.register_script_message_handler("pitexScroll", None);
        let scroll = hooks.scroll.clone();
        manager.connect_script_message_received(Some("pitexScroll"), move |_, value| {
            if !value.is_number() {
                return;
            }
            scroll(value.to_double());
        });
        let view = webkit6::WebView::builder()
            .user_content_manager(&manager)
            .build();
        // Strict navigation: only the initial `load_html` is allowed. Every
        // later navigation (link clicks, meta refresh, location=…) is
        // cancelled — clicked links go through `preview_link_action`.
        let policy = RefCell::new(NavigationPolicy::new());
        let open_link = hooks.open_link.clone();
        view.connect_decide_policy(move |_, decision, kind| {
            use webkit6::{NavigationType, PolicyDecisionType};
            let (uri, is_link) = match kind {
                PolicyDecisionType::NavigationAction | PolicyDecisionType::NewWindowAction => {
                    decision
                        .downcast_ref::<webkit6::NavigationPolicyDecision>()
                        .and_then(|d| d.navigation_action())
                        .map(|mut a| {
                            (
                                a.request().and_then(|r| r.uri()).map(|u| u.to_string()),
                                a.navigation_type() == NavigationType::LinkClicked,
                            )
                        })
                        .unwrap_or((None, false))
                }
                PolicyDecisionType::Response => {
                    decision.use_();
                    return true;
                }
                _ => {
                    decision.ignore();
                    return true;
                }
            };
            let verdict = if kind == PolicyDecisionType::NewWindowAction {
                policy.borrow_mut().new_window(uri.as_deref())
            } else {
                policy.borrow_mut().navigate(uri.as_deref(), is_link)
            };
            match verdict {
                NavVerdict::Allow => decision.use_(),
                NavVerdict::Cancel => decision.ignore(),
                NavVerdict::Route(uri) => {
                    decision.ignore();
                    open_link(uri);
                }
            }
            true
        });
        let loaded = hooks.loaded.clone();
        view.connect_load_changed(move |_, event| {
            if event != webkit6::LoadEvent::Finished {
                return;
            }
            loaded();
        });
        // file:/// origin so relative file:// images resolve.
        view.load_html(PREVIEW_HTML, Some("file:///"));
        view.set_vexpand(true);
        host.append(&view);
        Some(Self { view, hooks })
    }

    /// `pitexRender`/`pitexScrollToLine` — fire-and-forget page JavaScript.
    pub(super) fn eval(&self, script: &str) {
        self.view
            .evaluate_javascript(script, None, None, gtk4::gio::Cancellable::NONE, |_| {});
    }

    /// GTK's "Print to File" — paginated PDF, no dialog. `failed_message`
    /// is unused: the print operation reports its own error text.
    pub(super) fn print_to_pdf(&self, path: &Path, failed_message: &str) {
        let _ = failed_message;
        let settings = gtk4::PrintSettings::new();
        settings.set_printer("Print to File");
        settings.set(&gtk4::PRINT_SETTINGS_OUTPUT_FILE_FORMAT, Some("pdf"));
        settings.set(
            &gtk4::PRINT_SETTINGS_OUTPUT_URI,
            Some(&gtk4::gio::File::for_path(path).uri()),
        );
        let operation = webkit6::PrintOperation::new(&self.view);
        operation.set_print_settings(&settings);
        let toast = self.hooks.toast.clone();
        operation.connect_failed(move |_, error| toast(error.message().to_string()));
        operation.print();
    }

    /// GTK composites the WebView itself — overlays just draw over it.
    pub(super) fn set_covered(&self, _covered: bool) {}
}
