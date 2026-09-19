//! Pane builders: the bottom console (Assistant | Terminal | Issues | Log),
//! the right-hand PDF preview column, and the settings sheet. Mirrors
//! `BottomConsoleView.swift`, `Preview.swift`, `AgentPanel.swift`, and
//! `SettingsView.swift`.

use std::cell::RefCell;
use std::rc::Rc;

use gtk4::prelude::*;
use libadwaita as adw;
use adw::prelude::*;

#[cfg(unix)]
use linux_platform::{
    LinuxDefaultEditorRegistration as PlatformDefaultEditorRegistration,
    LinuxWorkspaceOpener as PlatformWorkspaceOpener,
};
#[cfg(windows)]
use windows_platform::{
    WindowsDefaultEditorRegistration as PlatformDefaultEditorRegistration,
    WindowsWorkspaceOpener as PlatformWorkspaceOpener,
};

use crate::app_ui::{a11y, AppState, UiHandles};
use crate::compat;
use crate::l10n::tr;
use crate::model::{ConsoleSection, WorkspaceBuildState};
use settings_feature::ShellExecutionPreference;

// ─── Bottom console ─────────────────────────────────────────────────────────

/// `BottomConsoleView`: segmented picker + assistant/terminal/issues/log panes.
pub fn build_console(state: &Rc<RefCell<AppState>>, ui: &UiHandles) -> gtk4::Widget {
    let lang = state.borrow().language;
    let root = gtk4::Box::new(gtk4::Orientation::Vertical, 0);

    // Header: section picker + build/custom command entries + run buttons.
    let header = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    header.set_margin_start(8);
    header.set_margin_end(8);
    header.set_margin_top(6);
    header.set_margin_bottom(6);

    let section_items = [
        tr(lang, "assistant.title"),
        tr(lang, "build.issues"),
        tr(lang, "console.terminal"),
        tr(lang, "build.log"),
    ];
    let section_strs: Vec<&str> = section_items.iter().map(String::as_str).collect();
    let section = gtk4::DropDown::from_strings(&section_strs);
    section.set_tooltip_text(Some(&tr(lang, "console.terminal")));
    {
        let state = state.clone();
        section.connect_selected_notify(move |dd| {
            let section = match dd.selected() {
                0 => ConsoleSection::Assistant,
                1 => ConsoleSection::Issues,
                2 => ConsoleSection::Terminal,
                _ => ConsoleSection::Log,
            };
            if let Ok(mut s) = state.try_borrow_mut() {
                s.set_console_section(section);
            }
        });
    }
    ui.console_section_dropdown.replace(Some(section.clone()));
    header.append(&section);

    // Build command entry — `{file}` placeholder, persisted per project.
    let build_entry = gtk4::Entry::new();
    build_entry.set_hexpand(true);
    build_entry.set_tooltip_text(Some(&tr(lang, "console.build_command_help")));
    a11y(&build_entry, "pitex.console.buildCommand", "console.build_command_help");
    {
        let state2 = state.clone();
        let state3 = state.clone();
        build_entry.connect_changed(move |e| {
            let Ok(mut s) = state2.try_borrow_mut() else { return };
            s.model.build_command_text = e.text().to_string();
            s.persist_commands();
        });
        build_entry.connect_activate(move |_| {
            if let Ok(mut s) = state3.try_borrow_mut() { s.start_build_action(); }
        });
    }
    ui.build_command_entry.replace(Some(build_entry.clone()));
    header.append(&build_entry);

    let run_build = gtk4::Button::from_icon_name("media-playback-start-symbolic");
    run_build.set_tooltip_text(Some(&tr(lang, "console.build_run_help")));
    a11y(&run_build, "pitex.console.build", "build.start");
    {
        let state = state.clone();
        run_build.connect_clicked(move |_| state.borrow_mut().start_build_action());
    }
    ui.build_button.replace(Some(run_build.clone()));
    header.append(&run_build);

    // `pitex.buildStatus` — mirrors the toolbar's building/succeeded/failed
    // indicator (`BuildStatusView`).
    let build_status = gtk4::Label::new(None);
    build_status.add_css_class("caption");
    a11y(&build_status, "pitex.buildStatus", "build.title");
    ui.build_status.replace(Some(build_status.clone()));
    header.append(&build_status);

    let custom_entry = gtk4::Entry::new();
    custom_entry.set_width_chars(18);
    custom_entry.set_placeholder_text(Some(&tr(lang, "console.custom_command_help")));
    custom_entry.set_tooltip_text(Some(&tr(lang, "console.custom_command_help")));
    a11y(&custom_entry, "pitex.console.customCommand", "console.custom_command_help");
    {
        let state = state.clone();
        custom_entry.connect_changed(move |e| {
            let Ok(mut s) = state.try_borrow_mut() else { return };
            s.model.custom_command_text = e.text().to_string();
            s.persist_commands();
        });
    }
    ui.custom_command_entry.replace(Some(custom_entry.clone()));
    header.append(&custom_entry);

    let run_custom = gtk4::Button::from_icon_name("utilities-terminal-symbolic");
    run_custom.set_tooltip_text(Some(&tr(lang, "console.custom_run_help")));
    a11y(&run_custom, "pitex.console.customRun", "command.run_custom");
    {
        let state = state.clone();
        run_custom.connect_clicked(move |_| state.borrow_mut().run_custom_command_action());
    }
    header.append(&run_custom);
    root.append(&header);
    root.append(&gtk4::Separator::new(gtk4::Orientation::Horizontal));

    // Panes.
    let stack = gtk4::Stack::new();
    stack.set_vexpand(true);
    stack.add_named(&build_assistant_pane(state, ui), Some("assistant"));
    stack.add_named(&build_terminal_pane(state, ui), Some("terminal"));
    stack.add_named(&build_issues_pane(state, ui), Some("issues"));
    stack.add_named(&build_log_pane(state, ui), Some("log"));
    stack.set_visible_child_name("terminal");
    ui.console_stack.replace(Some(stack.clone()));
    root.append(&stack);
    root.upcast()
}

fn build_terminal_pane(state: &Rc<RefCell<AppState>>, ui: &UiHandles) -> gtk4::Widget {
    let terminal = compat::ShellTerminal::new();
    let widget = terminal.widget();
    widget.set_hexpand(true);
    widget.set_vexpand(true);
    a11y(&widget, "pitex.console.terminal", "console.terminal");
    let (font, dir) = {
        let s = state.borrow();
        (s.terminal_font_desc(), s.model.project_url.clone())
    };
    terminal.set_font(Some(&font));
    // Spawn a login shell rooted at the project — `TerminalShellView` parity.
    // The Ubuntu 22.04 build has no VTE: `spawn_shell` reports failure and
    // the transcript explains that commands run in an external terminal.
    {
        let state = state.clone();
        terminal.spawn_shell(dir.as_deref(), move |ok| {
            if let Ok(mut s) = state.try_borrow_mut() { s.terminal_running = ok; }
        });
    }
    terminal.connect_exited(|term| {
        term.feed("\r\n\x1b[90m(shell exited)\x1b[0m\r\n");
    });
    ui.terminal.replace(Some(terminal));
    widget
}

fn build_issues_pane(state: &Rc<RefCell<AppState>>, ui: &UiHandles) -> gtk4::Widget {
    let lang = state.borrow().language;
    let root = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    // Severity filter chips: All | Errors | Warnings.
    let filter_items = [
        tr(lang, "issues.filter.all"),
        tr(lang, "issues.filter.errors"),
        tr(lang, "issues.filter.warnings"),
    ];
    let filter_strs: Vec<&str> = filter_items.iter().map(String::as_str).collect();
    let filter = gtk4::DropDown::from_strings(&filter_strs);
    filter.set_margin_start(8);
    filter.set_margin_top(6);
    filter.set_margin_bottom(6);
    filter.set_halign(gtk4::Align::Start);
    {
        let state = state.clone();
        filter.connect_selected_notify(move |_| {
            if let Ok(mut s) = state.try_borrow_mut() { s.refresh_issues(); }
        });
    }
    ui.issue_filter.replace(Some(filter.clone()));
    root.append(&filter);

    let scroll = gtk4::ScrolledWindow::new();
    scroll.set_vexpand(true);
    let list = gtk4::ListBox::new();
    list.set_selection_mode(gtk4::SelectionMode::None);
    list.add_css_class("boxed-list");
    list.set_margin_start(8);
    list.set_margin_end(8);
    list.set_margin_bottom(8);
    a11y(&list, "pitex.issues", "build.issues");
    {
        let state = state.clone();
        list.connect_row_activated(move |_, row| {
            let Ok(mut s) = state.try_borrow_mut() else { return };
            let index = row
                .child()
                .map(|c| c.widget_name().to_string())
                .and_then(|n| n.strip_prefix("issue-").map(str::to_string))
                .and_then(|n| n.parse::<usize>().ok());
            if let Some(index) = index {
                s.jump_to_issue(index);
            }
        });
    }
    scroll.set_child(Some(&list));
    ui.issues_list.replace(Some(list.clone()));
    root.append(&scroll);
    root.upcast()
}

fn build_log_pane(state: &Rc<RefCell<AppState>>, ui: &UiHandles) -> gtk4::Widget {
    let scroll = gtk4::ScrolledWindow::new();
    scroll.set_vexpand(true);
    let view = gtk4::TextView::new();
    view.set_editable(false);
    view.set_monospace(true);
    view.set_cursor_visible(false);
    view.set_wrap_mode(gtk4::WrapMode::Word);
    a11y(&view, "pitex.buildLog", "build.log");
    scroll.set_child(Some(&view));
    ui.build_log_view.replace(Some(view.clone()));
    view.buffer().set_text(&tr(state.borrow().language, "build.log_empty"));
    scroll.upcast()
}

// ─── Assistant pane ─────────────────────────────────────────────────────────

fn build_assistant_pane(state: &Rc<RefCell<AppState>>, ui: &UiHandles) -> gtk4::Widget {
    let lang = state.borrow().language;
    let root = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    a11y(&root, "pitex.assistant", "assistant.title");

    // Control row: model picker, reasoning picker, attach toggle, clear.
    let controls = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    controls.set_margin_start(10);
    controls.set_margin_end(10);
    controls.set_margin_top(6);
    controls.set_margin_bottom(2);
    let model_picker = gtk4::DropDown::from_strings(&[tr(lang, "assistant.model_placeholder").as_str()]);
    model_picker.set_size_request(180, -1);
    a11y(&model_picker, "pitex.assistantModel", "assistant.model");
    {
        let state = state.clone();
        model_picker.connect_selected_notify(move |dd| {
            let Ok(mut s) = state.try_borrow_mut() else { return };
            let i = dd.selected() as usize;
            if let Some(agent) = s.agent.as_mut() {
                if let Some(model) = agent.models.get(i).cloned() {
                    agent.select_model(&model);
                }
            }
        });
    }
    ui.agent_model_picker.replace(Some(model_picker.clone()));
    controls.append(&model_picker);

    let reasoning = gtk4::DropDown::from_strings(&["off"]);
    reasoning.set_tooltip_text(Some(&tr(lang, "assistant.reasoning_help")));
    a11y(&reasoning, "pitex.assistantReasoning", "assistant.reasoning_help");
    {
        let state = state.clone();
        reasoning.connect_selected_notify(move |dd| {
            let Ok(mut s) = state.try_borrow_mut() else { return };
            let Some(agent) = s.agent.as_mut() else { return };
            let level = agent
                .thinking_levels
                .get(dd.selected() as usize)
                .map(|l| l.to_string());
            if let Some(level) = level {
                agent.select_thinking_level(&level);
            }
        });
    }
    ui.agent_reasoning_picker.replace(Some(reasoning.clone()));
    controls.append(&reasoning);

    let attach_label = gtk4::Label::new(Some(&tr(lang, "assistant.attach_document")));
    controls.append(&attach_label);
    let attach_toggle = gtk4::Switch::new();
    attach_toggle.set_valign(gtk4::Align::Center);
    a11y(&attach_toggle, "pitex.assistantContext", "assistant.attach_document");
    {
        let state = state.clone();
        attach_toggle.connect_state_set(move |_, on| {
            let Ok(mut s) = state.try_borrow_mut() else {
                return gtk4::glib::Propagation::Proceed;
            };
            let crate::app_ui::AppState { agent, store, .. } = &mut *s;
            if let Some(agent) = agent.as_mut() {
                agent.set_attach_active_document(store, on);
            }
            gtk4::glib::Propagation::Proceed
        });
    }
    ui.agent_attach_toggle.replace(Some(attach_toggle.clone()));
    controls.append(&attach_toggle);

    let clear = gtk4::Button::from_icon_name("user-trash-symbolic");
    clear.set_tooltip_text(Some(&tr(lang, "assistant.clear")));
    a11y(&clear, "pitex.agent.newSession", "assistant.clear");
    {
        let state = state.clone();
        clear.connect_clicked(move |_| {
            let Ok(mut s) = state.try_borrow_mut() else { return };
            if let Some(agent) = s.agent.as_mut() {
                agent.new_session();
            }
            s.refresh_assistant();
        });
    }
    controls.append(&clear);
    root.append(&controls);

    // Transcript.
    let scroll = gtk4::ScrolledWindow::new();
    scroll.set_vexpand(true);
    scroll.set_propagate_natural_height(true);
    let transcript = gtk4::Box::new(gtk4::Orientation::Vertical, 10);
    transcript.set_margin_start(10);
    transcript.set_margin_end(10);
    transcript.set_margin_top(6);
    a11y(&transcript, "pitex.ai", "assistant.title");
    scroll.set_child(Some(&transcript));
    ui.transcript_box.replace(Some(transcript.clone()));
    ui.transcript_scroll.replace(Some(scroll.clone()));
    root.append(&scroll);

    let status = gtk4::Label::new(None);
    status.set_xalign(0.0);
    status.set_margin_start(10);
    status.add_css_class("dim-label");
    a11y(&status, "pitex.assistant.status", "assistant.title");
    ui.agent_status_label.replace(Some(status.clone()));
    root.append(&status);

    // Selection attachment chip.
    let chip = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
    chip.set_margin_start(10);
    chip.set_margin_end(10);
    chip.set_margin_bottom(4);
    chip.set_visible(false);
    chip.add_css_class("card");
    let chip_label = gtk4::Label::new(None);
    chip_label.set_ellipsize(gtk4::pango::EllipsizeMode::Middle);
    chip_label.set_hexpand(true);
    chip_label.set_xalign(0.0);
    let chip_close = gtk4::Button::from_icon_name("window-close-symbolic");
    chip_close.add_css_class("flat");
    a11y(&chip_close, "pitex.assistant.selectionRemove", "assistant.selection_remove");
    {
        let state = state.clone();
        chip_close.connect_clicked(move |_| {
            if let Some(agent) = state.borrow_mut().agent.as_mut() {
                agent.clear_selection_attachment();
            }
            if let Ok(mut s) = state.try_borrow_mut() { s.refresh_selection_chip(); }
        });
    }
    chip.append(&chip_label);
    chip.append(&chip_close);
    a11y(&chip, "pitex.assistant.selection", "assistant.title");
    ui.selection_chip.replace(Some(chip.clone()));
    ui.selection_chip_label.replace(Some(chip_label.clone()));
    root.append(&chip);

    // Composer: attach, entry, send/stop.
    let composer = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    composer.set_margin_start(10);
    composer.set_margin_end(10);
    composer.set_margin_top(6);
    composer.set_margin_bottom(10);
    let attach = gtk4::Button::from_icon_name("mail-attachment-symbolic");
    attach.set_tooltip_text(Some(&tr(lang, "assistant.attach_files_help")));
    a11y(&attach, "pitex.assistant.attach", "assistant.attach_files_help");
    {
        let state = state.clone();
        attach.connect_clicked(move |b| {
            let state2 = state.clone();
            attach_files_dialog(b, move |paths| {
                if let Ok(mut s) = state2.try_borrow_mut() { s.attach_files_action(paths); }
            });
        });
    }
    composer.append(&attach);

    let entry = gtk4::Entry::new();
    entry.set_hexpand(true);
    entry.add_css_class("pitex-conv-font");
    entry.set_placeholder_text(Some(&tr(lang, "assistant.message_placeholder")));
    a11y(&entry, "pitex.assistant.message", "assistant.message_placeholder");
    {
        let state = state.clone();
        entry.connect_activate(move |_| {
            if let Ok(mut s) = state.try_borrow_mut() { s.send_agent_draft(); }
        });
    }
    ui.agent_composer.replace(Some(entry.clone()));
    composer.append(&entry);

    let send = gtk4::Button::from_icon_name("go-up-symbolic");
    send.set_tooltip_text(Some(&tr(lang, "assistant.send_help")));
    a11y(&send, "pitex.assistantSend", "assistant.send");
    {
        let state = state.clone();
        send.connect_clicked(move |_| {
            if let Ok(mut s) = state.try_borrow_mut() { s.send_agent_draft(); }
        });
    }
    ui.agent_send_button.replace(Some(send.clone()));
    composer.append(&send);

    let stop = gtk4::Button::from_icon_name("media-playback-stop-symbolic");
    stop.set_tooltip_text(Some(&tr(lang, "assistant.stop_help")));
    stop.set_visible(false);
    a11y(&stop, "pitex.assistantCancel", "assistant.stop");
    {
        let state = state.clone();
        stop.connect_clicked(move |_| {
            if let Some(agent) = state.borrow_mut().agent.as_mut() {
                agent.stop();
            }
        });
    }
    ui.agent_stop_button.replace(Some(stop.clone()));
    composer.append(&stop);
    root.append(&composer);
    root.upcast()
}

/// Multi-select file dialog whose result paths are composer-inserted
/// (project-relative when possible, matching `attachFiles()`).
fn attach_files_dialog(button: &gtk4::Button, on_paths: impl Fn(Vec<String>) + 'static) {
    let window = button.root().and_then(|r| r.downcast::<gtk4::Window>().ok());
    compat::pick_files(window.as_ref(), "Attach files", move |paths| {
        on_paths(
            paths
                .iter()
                .map(|p| p.to_string_lossy().into_owned())
                .collect(),
        );
    });
}

// ─── PDF preview column ─────────────────────────────────────────────────────

/// `Preview.preview`: SyncTeX status row, toolbar, page view.
pub fn build_preview_pane(state: &Rc<RefCell<AppState>>, ui: &UiHandles) -> gtk4::Widget {
    let lang = state.borrow().language;
    let root = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    a11y(&root, "pitex.inspector", "preview.title");

    // SyncTeX status header.
    let status = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    status.set_margin_start(10);
    status.set_margin_end(10);
    status.set_margin_top(10);
    status.set_margin_bottom(10);
    let status_icon = gtk4::Image::from_icon_name("emblem-synchronizing-symbolic");
    let status_text = gtk4::Box::new(gtk4::Orientation::Vertical, 2);
    status_text.set_hexpand(true);
    let title = gtk4::Label::new(Some(&tr(lang, "preview.synctex")));
    title.set_xalign(0.0);
    title.add_css_class("caption");
    let detail = gtk4::Label::new(None);
    detail.set_xalign(0.0);
    detail.add_css_class("dim-label");
    detail.set_wrap(true);
    status_text.append(&title);
    status_text.append(&detail);
    status.append(&status_icon);
    status.append(&status_text);
    a11y(&status, "pitex.synctexStatus", "preview.synctex");
    ui.synctex_status_icon.replace(Some(status_icon.clone()));
    ui.synctex_status_label.replace(Some(detail.clone()));
    root.append(&status);
    root.append(&gtk4::Separator::new(gtk4::Orientation::Horizontal));

    // PDF toolbar: name, external open, save copy.
    let toolbar = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    toolbar.set_margin_start(10);
    toolbar.set_margin_end(10);
    toolbar.set_margin_top(6);
    toolbar.set_margin_bottom(6);
    let name = gtk4::Label::new(None);
    name.set_hexpand(true);
    name.set_xalign(0.0);
    name.set_ellipsize(gtk4::pango::EllipsizeMode::Middle);
    name.add_css_class("caption");
    ui.pdf_name_label.replace(Some(name.clone()));
    toolbar.append(&name);
    let external = gtk4::Button::from_icon_name("document-send-symbolic");
    external.set_tooltip_text(Some(&tr(lang, "preview.open_external")));
    a11y(&external, "pitex.pdf.openExternal", "preview.open_external");
    {
        let state = state.clone();
        external.connect_clicked(move |_| {
            let Ok(s) = state.try_borrow() else { return };
            if let Some(binding) = &s.model.synctex_binding {
                // `workspace.openDocument` — the Linux port opens through
                // xdg-open; a raw "file://{path}" URI breaks on spaces/%s.
                let _ = app_ports::WorkspaceOpening::open_document(
                    &s.env.workspace,
                    &binding.pdf_url,
                );
            }
        });
    }
    toolbar.append(&external);
    let download = gtk4::Button::from_icon_name("document-save-symbolic");
    download.set_tooltip_text(Some(&tr(lang, "preview.download")));
    a11y(&download, "pitex.pdf.download", "preview.download");
    {
        let state = state.clone();
        download.connect_clicked(move |b| {
            let pdf = {
                let Ok(s) = state.try_borrow() else { return };
                match &s.model.build_state {
                    WorkspaceBuildState::Succeeded { pdf, .. } => Some(pdf.clone()),
                    _ => None,
                }
            };
            if let Some(data) = pdf {
                let name = {
                    let Ok(s) = state.try_borrow() else { return };
                    s.pdf_display_name()
                };
                let window = b.root().and_then(|r| r.downcast::<gtk4::Window>().ok());
                compat::save_file(window.as_ref(), "Save PDF", Some(name.as_str()), move |path| {
                    let _ = std::fs::write(path, &data);
                });
            }
        });
    }
    toolbar.append(&download);
    ui.pdf_toolbar.replace(Some(toolbar.clone()));
    root.append(&toolbar);
    root.append(&gtk4::Separator::new(gtk4::Orientation::Horizontal));

    // Page navigation + zoom.
    let nav = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
    nav.set_margin_start(10);
    nav.set_margin_top(4);
    nav.set_margin_bottom(4);
    let prev = gtk4::Button::from_icon_name("go-previous-symbolic");
    prev.set_tooltip_text(Some(&tr(lang, "preview.previous_page")));
    let next = gtk4::Button::from_icon_name("go-next-symbolic");
    next.set_tooltip_text(Some(&tr(lang, "preview.next_page")));
    let page_label = gtk4::Label::new(Some("—"));
    let zoom_out = gtk4::Button::from_icon_name("zoom-out-symbolic");
    zoom_out.set_tooltip_text(Some(&tr(lang, "preview.zoom_out")));
    let zoom_in = gtk4::Button::from_icon_name("zoom-in-symbolic");
    zoom_in.set_tooltip_text(Some(&tr(lang, "preview.zoom_in")));
    {
        let s = state.clone();
        prev.connect_clicked(move |_| s.borrow_mut().pdf_prev_page());
    }
    {
        let s = state.clone();
        next.connect_clicked(move |_| s.borrow_mut().pdf_next_page());
    }
    {
        let s = state.clone();
        zoom_out.connect_clicked(move |_| s.borrow_mut().pdf_zoom(1.0 / 1.15));
    }
    {
        let s = state.clone();
        zoom_in.connect_clicked(move |_| s.borrow_mut().pdf_zoom(1.15));
    }
    nav.append(&prev);
    nav.append(&next);
    nav.append(&page_label);
    let spacer = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
    spacer.set_hexpand(true);
    nav.append(&spacer);
    nav.append(&zoom_out);
    nav.append(&zoom_in);
    ui.pdf_prev_button.replace(Some(prev.clone()));
    ui.pdf_next_button.replace(Some(next.clone()));
    ui.pdf_page_label.replace(Some(page_label.clone()));
    root.append(&nav);

    // Page surface: picture inside a scrolled window; click = inverse sync.
    let scroll = gtk4::ScrolledWindow::new();
    scroll.set_vexpand(true);
    a11y(&scroll, "pitex.preview", "preview.title");
    {
        let state = state.clone();
        let last_w = std::cell::Cell::new(0);
        scroll.add_tick_callback(move |widget, _| {
            let w = widget.width();
            if w != last_w.replace(w) && w > 0 {
                if let Ok(mut s) = state.try_borrow_mut() {
                    s.pdf_viewport_resized();
                }
            }
            gtk4::glib::ControlFlow::Continue
        });
    }
    let overlay = gtk4::Overlay::new();
    let picture = gtk4::Picture::new();
    compat::fit_picture(&picture);
    picture.set_can_shrink(true);
    a11y(&picture, "pitex.pdf", "preview.title");
    overlay.set_child(Some(&picture));
    // Highlight rectangle drawn on top after forward sync.
    let highlight = gtk4::DrawingArea::new();
    highlight.set_draw_func(|_, cr, w, h| {
        cr.set_source_rgba(1.0, 0.8, 0.0, 0.35);
        cr.rectangle(0.0, 0.0, w as f64, h as f64);
        cr.fill().expect("highlight fill");
    });
    highlight.set_visible(false);
    highlight.set_can_target(false);
    overlay.add_overlay(&highlight);
    scroll.set_child(Some(&overlay));
    ui.pdf_picture.replace(Some(picture.clone()));
    ui.pdf_highlight.replace(Some(highlight.clone()));
    ui.pdf_scroll.replace(Some(scroll.clone()));

    // Ctrl+click on the PDF → inverse SyncTeX (widget coords → PDF points),
    // matching the editor's Ctrl+click forward sync and Preview.swift's
    // Command-click inverse sync on macOS.
    {
        let state = state.clone();
        let gesture = gtk4::GestureClick::new();
        gesture.set_button(1);
        let picture2 = picture.clone();
        gesture.connect_released(move |gesture, _, x, y| {
            let modifiers = gesture.current_event_state();
            if !modifiers.contains(gtk4::gdk::ModifierType::CONTROL_MASK) {
                return;
            }
            if let Ok(mut s) = state.try_borrow_mut() { s.pdf_click(&picture2, x, y); }
        });
        overlay.add_controller(gesture);
    }
    root.append(&scroll);

    // Empty state shown when no PDF is available.
    let empty = adw::StatusPage::new();
    empty.set_title(&tr(lang, "preview.no_pdf"));
    empty.set_description(Some(&tr(lang, "preview.no_pdf_detail")));
    empty.set_icon_name(Some("x-office-document-symbolic"));
    empty.set_vexpand(true);
    a11y(&empty, "pitex.preview.empty", "preview.no_pdf");
    ui.pdf_empty.replace(Some(empty.clone()));
    root.append(&empty);
    scroll.set_visible(false);
    nav.set_visible(false);
    toolbar.set_visible(false);
    root.upcast()
}

// ─── Settings ────────────────────────────────────────────────────────────────

/// `SettingsView` — an `adw::PreferencesWindow` with the four panes
/// (Compile / Editor / Appearance / AI) replacing the capsule tab strip.
pub fn show_settings(state: &Rc<RefCell<AppState>>, parent: &gtk4::Window) {
    let (lang, store_settings) = {
        let s = state.borrow();
        (s.language, s.store.settings.clone())
    };
    let window = adw::PreferencesWindow::new();
    window.set_transient_for(Some(parent));
    window.set_modal(true);
    window.set_title(Some(&tr(lang, "settings.title")));
    window.set_default_size(720, 560);

    // ── Compile ──
    let compile = adw::PreferencesPage::new();
    compile.set_title(&tr(lang, "settings.tab.compile"));
    compile.set_icon_name(Some("tools-check-spelling-symbolic"));
    let presets_group = adw::PreferencesGroup::new();
    presets_group.set_title(&tr(lang, "settings.compile.presets"));
    // Bound widgets sharing the command text — the preset buttons and the
    // commands entry both track the same live value in SwiftUI.
    let build_cmd_cell: Rc<RefCell<Option<gtk4::Entry>>> = Rc::new(RefCell::new(None));
    for (label, command) in [
        ("preset.pdflatex", "pdflatex -interaction=nonstopmode -synctex=1 {file}"),
        ("preset.xelatex", "xelatex -interaction=nonstopmode -synctex=1 {file}"),
        ("preset.lualatex", "lualatex -interaction=nonstopmode -synctex=1 {file}"),
        ("preset.latexmk", "latexmk -pdf -interaction=nonstopmode -synctex=1 {file}"),
        ("preset.tectonic", "tectonic --synctex {file}"),
    ] {
        let row = adw::ActionRow::new();
        row.set_title(&tr(lang, label));
        row.set_subtitle(command);
        let apply = gtk4::Button::with_label(&tr(lang, "assistant.apply"));
        let state = state.clone();
        let cmd = command.to_string();
        let build_cmd_cell = build_cmd_cell.clone();
        // `applyPreset` — the workspace command while a project is open,
        // `defaultBuildCommand` otherwise.
        apply.connect_clicked(move |_| {
            let Ok(mut s) = state.try_borrow_mut() else { return };
            if s.model.has_project() {
                s.model.build_command_text = cmd.clone();
                s.persist_commands();
                s.sync_build_entries();
            } else {
                s.store.set_default_build_command(&cmd);
            }
            if let Some(e) = build_cmd_cell.borrow().as_ref() {
                e.set_text(&cmd);
            }
        });
        row.add_suffix(&apply);
        presets_group.add(&row);
    }
    compile.add(&presets_group);

    let commands = adw::PreferencesGroup::new();
    commands.set_title(&tr(lang, "settings.compile.commands"));
    commands.set_description(Some(&tr(lang, "settings.compile.commands_note")));
    // `buildCommandTextBinding` — the live workspace command when a project
    // is open, the persisted default otherwise.
    let (build_cmd_row, build_cmd) = compat::entry_row(&tr(lang, "settings.compile.build_command"));
    {
        let s = state.borrow();
        let text = if s.model.has_project() {
            s.model.build_command_text.clone()
        } else {
            s.store.default_build_command()
        };
        build_cmd.set_text(&text);
    }
    *build_cmd_cell.borrow_mut() = Some(build_cmd.clone());
    {
        let state = state.clone();
        build_cmd.connect_changed(move |row| {
            let Ok(mut s) = state.try_borrow_mut() else { return };
            if s.model.has_project() {
                s.model.build_command_text = row.text().to_string();
                s.persist_commands();
                s.sync_build_entries();
            } else {
                s.store.set_default_build_command(&row.text());
            }
        });
    }
    commands.add(&build_cmd_row);
    // `customCommandTextBinding` — same workspace/default split.
    let (custom_cmd_row, custom_cmd) = compat::entry_row(&tr(lang, "settings.compile.custom_command"));
    {
        let s = state.borrow();
        let text = if s.model.has_project() {
            s.model.custom_command_text.clone()
        } else {
            s.store.default_custom_command()
        };
        custom_cmd.set_text(&text);
    }
    {
        let state = state.clone();
        custom_cmd.connect_changed(move |row| {
            let Ok(mut s) = state.try_borrow_mut() else { return };
            if s.model.has_project() {
                s.model.custom_command_text = row.text().to_string();
                s.persist_commands();
                s.sync_build_entries();
            } else {
                s.store.set_default_custom_command(&row.text());
            }
        });
    }
    commands.add(&custom_cmd_row);
    compile.add(&commands);

    let behavior = adw::PreferencesGroup::new();
    behavior.set_title(&tr(lang, "settings.compile.behavior"));
    let (switch_pdf_row, switch_pdf) = compat::switch_row(&tr(lang, "settings.compile.switch_pdf"));
    switch_pdf.set_active(state.borrow().store.switch_to_pdf_on_build());
    {
        let state = state.clone();
        switch_pdf.connect_active_notify(move |r| {
            if let Ok(mut s) = state.try_borrow_mut() { s.store.set_switch_to_pdf_on_build(r.is_active()); }
        });
    }
    behavior.add(&switch_pdf_row);
    let (jump_row, jump) = compat::switch_row(&tr(lang, "settings.compile.jump_cursor"));
    jump.set_active(state.borrow().store.jump_to_cursor_after_build());
    {
        let state = state.clone();
        jump.connect_active_notify(move |r| {
            if let Ok(mut s) = state.try_borrow_mut() { s.store.set_jump_to_cursor_after_build(r.is_active()); }
        });
    }
    behavior.add(&jump_row);
    let (stop_err_row, stop_err) = compat::switch_row(&tr(lang, "settings.compile.stop_error"));
    stop_err.set_active(store_settings.build.stops_after_first_error);
    {
        let state = state.clone();
        stop_err.connect_active_notify(move |r| {
            if let Ok(mut s) = state.try_borrow_mut() {
                s.update_build_prefs(|b| {
                    b.stops_after_first_error = r.is_active();
                });
            }
        });
    }
    behavior.add(&stop_err_row);
    let (passes_row, passes) = compat::spin_row(&tr1(lang, "settings.compile.max_passes", &store_settings.build.maximum_passes.to_string()), 1.0, 10.0, 1.0);
    passes.set_value(store_settings.build.maximum_passes as f64);
    {
        let state = state.clone();
        passes.connect_value_notify(move |r| {
            if let Ok(mut s) = state.try_borrow_mut() {
                s.update_build_prefs(|b| {
                    b.maximum_passes = r.value().clamp(1.0, 10.0) as u8;
                });
            }
        });
    }
    behavior.add(&passes_row);
    compile.add(&behavior);

    let shell_group = adw::PreferencesGroup::new();
    shell_group.set_title(&tr(lang, "settings.compile.shell"));
    let (shell_path_row, shell_path) = compat::entry_row(&tr(lang, "settings.compile.shell_path"));
    shell_path.set_text(&state.borrow().store.custom_shell_executable());
    {
        let state = state.clone();
        shell_path.connect_changed(move |row| {
            if let Ok(mut s) = state.try_borrow_mut() { s.store.set_custom_shell_executable(&row.text()); }
        });
    }
    shell_group.add(&shell_path_row);
    let (ack_row, ack) = compat::switch_row(&tr(lang, "settings.compile.shell_ack"));
    ack.set_active(store_settings.build.custom_shell_acknowledged);
    {
        let state = state.clone();
        ack.connect_active_notify(move |r| {
            if let Ok(mut s) = state.try_borrow_mut() { s.update_shell_acknowledgement(r.is_active()); }
        });
    }
    shell_group.add(&ack_row);
    // `TextField("settings.compile.fallback_command", text: customCommandBinding)`
    let (fallback_row, fallback) = compat::entry_row(&tr(lang, "settings.compile.fallback_command"));
    if let ShellExecutionPreference::Custom { command } =
        &store_settings.build.shell_execution
    {
        fallback.set_text(command);
    }
    {
        let state = state.clone();
        fallback.connect_changed(move |row| {
            if let Ok(mut s) = state.try_borrow_mut() {
                s.set_shell_execution(&row.text());
            }
        });
    }
    shell_group.add(&fallback_row);
    compile.add(&shell_group);

    // `DefaultEditorRegistration.registerAsDefault()` — claim the declared
    // types via xdg-mime, the LaunchServices counterpart.
    let default_group = adw::PreferencesGroup::new();
    let make_default = adw::ActionRow::new();
    make_default.set_title(&tr(lang, "settings.compile.make_default_editor"));
    make_default.set_subtitle(&tr(lang, "settings.compile.make_default_note"));
    make_default.set_activatable(true);
    make_default.connect_activated(|_| {
        let _ = PlatformDefaultEditorRegistration::register_as_default();
    });
    default_group.add(&make_default);
    compile.add(&default_group);
    window.add(&compile);

    // ── Editor ──
    let editor = adw::PreferencesPage::new();
    editor.set_title(&tr(lang, "settings.tab.editor"));
    editor.set_icon_name(Some("document-edit-symbolic"));
    let autosave_group = adw::PreferencesGroup::new();
    autosave_group.set_title(&tr(lang, "settings.editor.autosave"));
    let (auto_save_row, auto_save) = compat::switch_row(&tr(lang, "settings.editor.autosave"));
    auto_save.set_active(state.borrow().store.auto_save());
    autosave_group.add(&auto_save_row);
    // `.disabled(!store.autoSave)` — the delay picker greys out while
    // autosave is off.
    let delay = adw::ComboRow::new();
    delay.set_title(&tr(lang, "settings.editor.autosave_delay"));
    delay.set_sensitive(state.borrow().store.auto_save());
    {
        let state = state.clone();
        let delay = delay.clone();
        auto_save.connect_active_notify(move |r| {
            delay.set_sensitive(r.is_active());
            if let Ok(mut s) = state.try_borrow_mut() {
                s.store.set_auto_save(r.is_active());
            }
        });
    }
    let delay_items = [
        tr(lang, "settings.editor.delay_2"),
        tr(lang, "settings.editor.delay_5"),
        tr(lang, "settings.editor.delay_10"),
    ];
    let delay_strs: Vec<&str> = delay_items.iter().map(String::as_str).collect();
    let delays = gtk4::StringList::new(&delay_strs);
    delay.set_model(Some(&delays));
    let current_delay = state.borrow().store.auto_save_delay();
    delay.set_selected(match current_delay {
        0..=3 => 0,
        4..=7 => 1,
        _ => 2,
    });
    {
        let state = state.clone();
        delay.connect_selected_notify(move |r| {
            let v = match r.selected() {
                0 => 2,
                1 => 5,
                _ => 10,
            };
            if let Ok(mut s) = state.try_borrow_mut() { s.store.set_auto_save_delay(v); }
        });
    }
    autosave_group.add(&delay);
    editor.add(&autosave_group);

    let editing = adw::PreferencesGroup::new();
    editing.set_title(&tr(lang, "settings.editor.editing"));
    let (confirm_row, confirm) = compat::switch_row(&tr(lang, "settings.editor.confirm_overwrite"));
    confirm.set_active(state.borrow().store.confirm_overwrite());
    {
        let state = state.clone();
        confirm.connect_active_notify(move |r| {
            if let Ok(mut s) = state.try_borrow_mut() { s.store.set_confirm_overwrite(r.is_active()); }
        });
    }
    editing.add(&confirm_row);
    // `Toggle("settings.editor.autocomplete", isOn: completesDelimitersBinding)`
    let (autocomplete_row, autocomplete) = compat::switch_row(&tr(lang, "settings.editor.autocomplete"));
    autocomplete.set_active(store_settings.editor.completes_delimiters);
    {
        let state = state.clone();
        autocomplete.connect_active_notify(move |r| {
            let Ok(mut s) = state.try_borrow_mut() else { return };
            s.update_editor_prefs(|e| {
                e.completes_delimiters = r.is_active();
            });
        });
    }
    editing.add(&autocomplete_row);
    let (folding_row, folding) = compat::switch_row(&tr(lang, "settings.editor.folding"));
    folding.set_active(state.borrow().store.code_folding());
    {
        let state = state.clone();
        folding.connect_active_notify(move |r| {
            let Ok(mut s) = state.try_borrow_mut() else { return };
            s.store.set_code_folding(r.is_active());
            s.apply_editor_preferences();
        });
    }
    editing.add(&folding_row);
    let (minimap_row, minimap) = compat::switch_row(&tr(lang, "settings.editor.minimap"));
    minimap.set_active(state.borrow().store.minimap());
    {
        let state = state.clone();
        minimap.connect_active_notify(move |r| {
            let Ok(mut s) = state.try_borrow_mut() else { return };
            s.store.set_minimap(r.is_active());
            s.apply_editor_preferences();
        });
    }
    editing.add(&minimap_row);
    editor.add(&editing);

    let session_group = adw::PreferencesGroup::new();
    session_group.set_title(&tr(lang, "settings.editor.session"));
    let (restore_row, restore) = compat::switch_row(&tr(lang, "settings.editor.restore"));
    restore.set_active(state.borrow().store.restore_session());
    {
        let state = state.clone();
        restore.connect_active_notify(move |r| {
            if let Ok(mut s) = state.try_borrow_mut() { s.store.set_restore_session(r.is_active()); }
        });
    }
    session_group.add(&restore_row);
    // `Stepper(..., value: tabWidthBinding, in: 1...16)`
    let (tab_width_row, tab_width) = compat::spin_row(&tr1(
        lang,
        "settings.editor.tab_width",
        &store_settings.editor.tab_width.to_string(),
    ), 1.0, 16.0, 1.0);
    tab_width.set_value(store_settings.editor.tab_width as f64);
    {
        let state = state.clone();
        tab_width.connect_value_notify(move |r| {
            let Ok(mut s) = state.try_borrow_mut() else { return };
            s.update_editor_prefs(|e| {
                e.tab_width = r.value().clamp(1.0, 16.0) as u8;
            });
            s.apply_editor_preferences();
        });
    }
    session_group.add(&tab_width_row);
    let (wrap_row, wrap) = compat::switch_row(&tr(lang, "settings.editor.wrap"));
    wrap.set_active(store_settings.editor.wraps_lines);
    {
        let state = state.clone();
        wrap.connect_active_notify(move |r| {
            let Ok(mut s) = state.try_borrow_mut() else { return };
            s.update_editor_prefs(|e| {
                e.wraps_lines = r.is_active();
            });
            s.apply_editor_preferences();
        });
    }
    session_group.add(&wrap_row);
    editor.add(&session_group);

    // `Section("SyncTeX")` — independent highlight toggles, both off by
    // default; navigation works regardless.
    let synctex_group = adw::PreferencesGroup::new();
    synctex_group.set_title("SyncTeX");
    let (inverse_highlight_row, inverse_highlight) = compat::switch_row(&tr(lang, "settings.synctex.inverse_highlight"));
    inverse_highlight.set_active(state.borrow().store.inverse_sync_highlight());
    a11y(&inverse_highlight, "pitex.settings.synctex.inverseHighlight", "settings.synctex.inverse_highlight");
    {
        let state = state.clone();
        inverse_highlight.connect_active_notify(move |r| {
            if let Ok(mut s) = state.try_borrow_mut() { s.store.set_inverse_sync_highlight(r.is_active()); }
        });
    }
    synctex_group.add(&inverse_highlight_row);
    let (forward_highlight_row, forward_highlight) = compat::switch_row(&tr(lang, "settings.synctex.forward_highlight"));
    forward_highlight.set_active(state.borrow().store.forward_sync_highlight());
    a11y(&forward_highlight, "pitex.settings.synctex.forwardHighlight", "settings.synctex.forward_highlight");
    {
        let state = state.clone();
        forward_highlight.connect_active_notify(move |r| {
            if let Ok(mut s) = state.try_borrow_mut() {
                s.store.set_forward_sync_highlight(r.is_active());
                // `clearSyncHighlight()` when the preference turns off — the
                // generation bump also cancels a pending auto-hide.
                if !r.is_active() {
                    s.clear_synctex_highlight();
                }
            }
        });
    }
    synctex_group.add(&forward_highlight_row);
    // `Text("settings.synctex.highlight_note").font(.caption)` — a subtitle
    // row carries the same secondary-text styling.
    let note = adw::ActionRow::new();
    note.set_subtitle(&tr(lang, "settings.synctex.highlight_note"));
    note.set_sensitive(false);
    synctex_group.add(&note);
    editor.add(&synctex_group);
    window.add(&editor);

    // ── Appearance ──
    let appearance = adw::PreferencesPage::new();
    appearance.set_title(&tr(lang, "settings.tab.appearance"));
    appearance.set_icon_name(Some("applications-graphics-symbolic"));
    let theme_group = adw::PreferencesGroup::new();
    theme_group.set_title(&tr(lang, "settings.appearance.theme"));
    let mode = adw::ComboRow::new();
    mode.set_title(&tr(lang, "settings.appearance.mode"));
    let mode_items = [
        tr(lang, "settings.appearance.system"),
        tr(lang, "settings.appearance.light"),
        tr(lang, "settings.appearance.dark"),
    ];
    let mode_strs: Vec<&str> = mode_items.iter().map(String::as_str).collect();
    let modes = gtk4::StringList::new(&mode_strs);
    mode.set_model(Some(&modes));
    mode.set_selected(match state.borrow().appearance.theme {
        crate::settings::Theme::System => 0,
        crate::settings::Theme::Light => 1,
        crate::settings::Theme::Dark => 2,
    });
    {
        let state = state.clone();
        mode.connect_selected_notify(move |r| {
            let Ok(mut s) = state.try_borrow_mut() else { return };
            let theme = match r.selected() {
                1 => crate::settings::Theme::Light,
                2 => crate::settings::Theme::Dark,
                _ => crate::settings::Theme::System,
            };
            let crate::app_ui::AppState { appearance, store, .. } = &mut *s;
            appearance.set_theme(store.prefs_mut(), theme);
            s.apply_theme();
        });
    }
    theme_group.add(&mode);
    let preset = adw::ComboRow::new();
    preset.set_title(&tr(lang, "settings.appearance.theme"));
    let preset_names: Vec<String> = crate::settings::AppearanceThemePreset::ALL
        .iter()
        .map(|p| p.title().to_string())
        .collect();
    let preset_strs: Vec<&str> = preset_names.iter().map(String::as_str).collect();
    let preset_list = gtk4::StringList::new(&preset_strs);
    preset.set_model(Some(&preset_list));
    let matching = {
        let s = state.borrow();
        s.appearance.matching_preset(s.store.prefs())
    };
    if let Some(match_idx) = matching {
        preset.set_selected(
            crate::settings::AppearanceThemePreset::ALL
                .iter()
                .position(|p| *p == match_idx)
                .unwrap_or(0) as u32,
        );
    }
    // Widgets bound to the palette — populated by the colors/preview
    // sections below so a preset pick re-paints them like SwiftUI's
    // `@ObservedObject appearance` does.
    let color_wells: Rc<RefCell<Vec<compat::ColorWell>>> =
        Rc::new(RefCell::new(Vec::new()));
    let preview_label_cell: Rc<RefCell<Option<gtk4::Label>>> = Rc::new(RefCell::new(None));
    let palette_refreshing = Rc::new(std::cell::Cell::new(false));
    {
        let state = state.clone();
        let color_wells = color_wells.clone();
        let preview_label_cell = preview_label_cell.clone();
        let palette_refreshing = palette_refreshing.clone();
        preset.connect_selected_notify(move |r| {
            if let Some(p) = crate::settings::AppearanceThemePreset::ALL.get(r.selected() as usize) {
                let Ok(mut s) = state.try_borrow_mut() else { return };
                {
                    let crate::app_ui::AppState { appearance, store, .. } = &mut *s;
                    appearance.apply_preset(store.prefs_mut(), *p);
                }
                s.apply_theme();
                s.rehighlight();
                palette_refreshing.set(true);
                for (i, well) in color_wells.borrow().iter().enumerate() {
                    let (r2, g, b, a) = s
                        .appearance
                        .color(s.store.prefs(), crate::settings::AppearanceColorRole::ALL[i]);
                    well.set_rgba(&gtk4::gdk::RGBA::new(
                        r2 as f32, g as f32, b as f32, a as f32,
                    ));
                }
                palette_refreshing.set(false);
                if let Some(l) = preview_label_cell.borrow().as_ref() {
                    apply_syntax_preview(&s, l);
                }
            }
        });
    }
    theme_group.add(&preset);
    let language_row = adw::ComboRow::new();
    language_row.set_title(&tr(lang, "settings.appearance.language"));
    let language_items = [
        tr(lang, "settings.appearance.language_system"),
        "English".to_string(),
        "한국어".to_string(),
        "日本語".to_string(),
        "Tiếng Việt".to_string(),
        "Русский".to_string(),
        "简体中文".to_string(),
        "Español".to_string(),
    ];
    let language_strs: Vec<&str> = language_items.iter().map(String::as_str).collect();
    let languages = gtk4::StringList::new(&language_strs);
    language_row.set_model(Some(&languages));
    language_row.set_selected(match state.borrow().appearance.language {
        crate::settings::AppLanguage::En => 1,
        crate::settings::AppLanguage::Ko => 2,
        crate::settings::AppLanguage::Ja => 3,
        crate::settings::AppLanguage::Vi => 4,
        crate::settings::AppLanguage::Ru => 5,
        crate::settings::AppLanguage::ZhHans => 6,
        crate::settings::AppLanguage::Es => 7,
        _ => 0,
    });
    // `if appearance.languageRestartPending { Text("settings.appearance.language_restart") }`
    let restart_row = adw::ActionRow::new();
    restart_row.set_subtitle(&tr(lang, "settings.appearance.language_restart"));
    restart_row.set_sensitive(false);
    restart_row.set_visible(state.borrow().appearance.language_restart_pending());
    {
        let state = state.clone();
        let restart_row = restart_row.clone();
        language_row.connect_selected_notify(move |r| {
            let Ok(mut s) = state.try_borrow_mut() else { return };
            let lang = match r.selected() {
                1 => crate::settings::AppLanguage::En,
                2 => crate::settings::AppLanguage::Ko,
                3 => crate::settings::AppLanguage::Ja,
                4 => crate::settings::AppLanguage::Vi,
                5 => crate::settings::AppLanguage::Ru,
                6 => crate::settings::AppLanguage::ZhHans,
                7 => crate::settings::AppLanguage::Es,
                _ => crate::settings::AppLanguage::System,
            };
            let crate::app_ui::AppState { appearance, store, .. } = &mut *s;
            appearance.set_language(store.prefs_mut(), lang);
            restart_row.set_visible(s.appearance.language_restart_pending());
        });
    }
    theme_group.add(&language_row);
    theme_group.add(&restart_row);
    appearance.add(&theme_group);

    let font_group = adw::PreferencesGroup::new();
    font_group.set_title(&tr(lang, "settings.appearance.font"));
    let font_button = compat::FontPicker::new();
    font_button.set_font_desc(&state.borrow().editor_font_desc());
    // `SyntaxPreview()` — styled sample lines in the palette + editor font;
    // refreshed whenever the font or a role color changes.
    let preview = gtk4::Box::new(gtk4::Orientation::Vertical, 2);
    preview.add_css_class("pitex-syntax-preview");
    let preview_label = gtk4::Label::new(None);
    preview_label.set_halign(gtk4::Align::Start);
    preview_label.set_xalign(0.0);
    preview.append(&preview_label);
    *preview_label_cell.borrow_mut() = Some(preview_label.clone());
    {
        let s = state.borrow();
        apply_syntax_preview(&s, &preview_label);
    }
    {
        let state = state.clone();
        let preview_label = preview_label.clone();
        font_button.connect_changed(move |b| {
            if let Some(desc) = b.font_desc() {
                let Ok(mut s) = state.try_borrow_mut() else { return };
                let family = desc.family().map(|f| f.to_string()).unwrap_or_default();
                // `Slider(value: $appearance.fontSize, in: 9...24, step: 1)`
                let size = (desc.size() as f64 / gtk4::pango::SCALE as f64).clamp(9.0, 24.0);
                let crate::app_ui::AppState { appearance, store, .. } = &mut *s;
                appearance.set_font(store.prefs_mut(), &family, size);
                // The editor's font rides in the `.pitex-editor` display CSS —
                // `apply_theme` regenerates it; `apply_editor_preferences`
                // covers the terminal font.
                s.apply_theme();
                s.apply_editor_preferences();
                apply_syntax_preview(&s, &preview_label);
            }
        });
    }
    let font_row = adw::ActionRow::new();
    font_row.set_title(&tr(lang, "settings.appearance.family"));
    font_row.add_suffix(font_button.widget());
    font_group.add(&font_row);
    font_group.add(&preview);
    // `Toggle("settings.appearance.terminal_font_custom")` — off means the
    // terminal follows the editor font (empty family).
    let (term_font_row, term_font_switch) =
        compat::switch_row(&tr(lang, "settings.appearance.terminal_font_custom"));
    term_font_switch.set_active(!state.borrow().appearance.terminal_font_family.is_empty());
    let term_font_button = compat::FontPicker::new();
    term_font_button.set_font_desc(&state.borrow().terminal_font_desc());
    let term_font_picker_row = adw::ActionRow::new();
    term_font_picker_row.set_title(&tr(lang, "settings.appearance.terminal_font_family"));
    term_font_picker_row.add_suffix(term_font_button.widget());
    term_font_picker_row.set_sensitive(term_font_switch.is_active());
    {
        let state = state.clone();
        let term_font_picker_row = term_font_picker_row.clone();
        let term_font_button = term_font_button.clone_ref();
        term_font_switch.connect_active_notify(move |sw| {
            let Ok(mut s) = state.try_borrow_mut() else { return };
            if sw.is_active() {
                // Seed with the editor font so the picker shows a real font.
                let (family, size) = {
                    let a = &s.appearance;
                    (
                        if a.font_family.is_empty() { "Monospace".to_string() } else { a.font_family.clone() },
                        a.font_size,
                    )
                };
                {
                    let crate::app_ui::AppState { appearance, store, .. } = &mut *s;
                    appearance.set_terminal_font(store.prefs_mut(), &family, size);
                }
                term_font_button.set_font_desc(&s.terminal_font_desc());
            } else {
                {
                    let crate::app_ui::AppState { appearance, store, .. } = &mut *s;
                    appearance.set_terminal_font(store.prefs_mut(), "", 13.0);
                }
            }
            term_font_picker_row.set_sensitive(sw.is_active());
            s.apply_editor_preferences();
        });
    }
    {
        let state = state.clone();
        term_font_button.connect_changed(move |b| {
            if let Some(desc) = b.font_desc() {
                let Ok(mut s) = state.try_borrow_mut() else { return };
                let family = desc.family().map(|f| f.to_string()).unwrap_or_default();
                let size = (desc.size() as f64 / gtk4::pango::SCALE as f64).clamp(9.0, 24.0);
                {
                    let crate::app_ui::AppState { appearance, store, .. } = &mut *s;
                    appearance.set_terminal_font(store.prefs_mut(), &family, size);
                }
                s.apply_editor_preferences();
            }
        });
    }
    font_group.add(&term_font_row);
    font_group.add(&term_font_picker_row);
    appearance.add(&font_group);

    // `Section("settings.appearance.colors")` — one ColorWellButton per role.
    let colors_group = adw::PreferencesGroup::new();
    colors_group.set_title(&tr(lang, "settings.appearance.colors"));
    for role in crate::settings::AppearanceColorRole::ALL {
        let row = adw::ActionRow::new();
        row.set_title(&tr(lang, role.title_key()));
        row.set_subtitle(&tr(lang, role.detail_key()));
        let well = compat::ColorWell::new();
        {
            let s = state.borrow();
            let (r, g, b, a) = s.appearance.color(s.store.prefs(), role);
            well.set_rgba(&gtk4::gdk::RGBA::new(r as f32, g as f32, b as f32, a as f32));
        }
        {
            let state = state.clone();
            let preview_label = preview_label.clone();
            let palette_refreshing = palette_refreshing.clone();
            well.connect_changed(move |w| {
                // Preset picks re-set every well — don't re-store the same
                // hexes eleven times over.
                if palette_refreshing.get() {
                    return;
                }
                let Ok(mut s) = state.try_borrow_mut() else { return };
                let rgba = w.rgba();
                let hex = crate::settings::rgba_to_hex_string(
                    rgba.red() as f64,
                    rgba.green() as f64,
                    rgba.blue() as f64,
                    rgba.alpha() as f64,
                );
                {
                    let crate::app_ui::AppState { appearance, store, .. } = &mut *s;
                    appearance.set_color(store.prefs_mut(), role, &hex);
                }
                s.apply_theme();
                s.rehighlight();
                apply_syntax_preview(&s, &preview_label);
            });
        }
        row.add_suffix(well.widget());
        color_wells.borrow_mut().push(well);
        colors_group.add(&row);
    }
    appearance.add(&colors_group);
    window.add(&appearance);

    // ── AI ──
    let ai = adw::PreferencesPage::new();
    ai.set_title(&tr(lang, "settings.tab.ai"));
    ai.set_icon_name(Some("starred-symbolic"));
    let agent_group = adw::PreferencesGroup::new();
    agent_group.set_title(&tr(lang, "settings.ai.agent"));
    let location = adw::ActionRow::new();
    location.set_title(&tr(lang, "settings.ai.location"));
    location.set_subtitle(&crate::agent::pi_paths::runtime_directory().to_string_lossy());
    agent_group.add(&location);
    let (attach_default_row, attach_default) = compat::switch_row(&tr(lang, "settings.ai.attach_default"));
    attach_default.set_active(state.borrow().store.ai_attach_default());
    {
        let state = state.clone();
        attach_default.connect_active_notify(move |r| {
            if let Ok(mut s) = state.try_borrow_mut() { s.store.set_ai_attach_default(r.is_active()); }
        });
    }
    agent_group.add(&attach_default_row);
    // `LabeledContent("settings.ai.font_size")` — 10…24pt slider + pt readout
    // + a live preview line, like the Swift settings page.
    let font_row = adw::ActionRow::new();
    font_row.set_title(&tr(lang, "settings.ai.font_size"));
    let font_scale = gtk4::Scale::with_range(gtk4::Orientation::Horizontal, 10.0, 24.0, 1.0);
    font_scale.set_size_request(180, -1);
    font_scale.set_valign(gtk4::Align::Center);
    font_scale.set_draw_value(false);
    font_scale.set_value(state.borrow().store.ai_font_size());
    a11y(&font_scale, "pitex.settings.ai.fontSize", "settings.ai.font_size");
    let font_value = gtk4::Label::new(Some(&format!(
        "{} pt",
        state.borrow().store.ai_font_size() as i64
    )));
    font_value.add_css_class("dim-label");
    font_value.set_width_chars(5);
    font_row.add_suffix(&font_scale);
    font_row.add_suffix(&font_value);
    agent_group.add(&font_row);
    let preview_label = gtk4::Label::new(Some(&tr(lang, "settings.ai.font_preview")));
    preview_label.set_xalign(0.0);
    preview_label.set_wrap(true);
    preview_label.set_margin_start(12);
    preview_label.set_margin_end(12);
    preview_label.set_margin_bottom(8);
    preview_label.set_attributes(Some(&crate::app_ui::font_attrs(
        state.borrow().store.ai_font_size(),
    )));
    {
        let state = state.clone();
        let preview_label = preview_label.clone();
        font_scale.connect_value_changed(move |scale| {
            let size = scale.value().round().clamp(10.0, 24.0);
            font_value.set_text(&format!("{} pt", size as i64));
            preview_label.set_attributes(Some(&crate::app_ui::font_attrs(size)));
            if let Ok(mut s) = state.try_borrow_mut() {
                s.store.set_ai_font_size(size);
                s.refresh_assistant();
            }
        });
    }
    agent_group.add(&preview_label);
    let login = adw::ActionRow::new();
    login.set_title(&tr(lang, "settings.ai.setup_providers"));
    login.set_activatable(true);
    {
        let state = state.clone();
        login.connect_activated(move |_| {
            if let Ok(mut s) = state.try_borrow_mut() { s.run_agent_auth_script(false); }
        });
    }
    agent_group.add(&login);
    let logout = adw::ActionRow::new();
    logout.set_title(&tr(lang, "settings.ai.logout"));
    logout.set_activatable(true);
    {
        let state = state.clone();
        logout.connect_activated(move |_| {
            if let Ok(mut s) = state.try_borrow_mut() { s.run_agent_auth_script(true); }
        });
    }
    agent_group.add(&logout);
    // `Button("settings.ai.custom_provider")` — opens models.json.
    let custom_provider = adw::ActionRow::new();
    custom_provider.set_title(&tr(lang, "settings.ai.custom_provider"));
    custom_provider.set_activatable(true);
    custom_provider.connect_activated(|_| {
        if let Ok(path) = crate::agent::pi_paths::configuration_file(true) {
            let _ = app_ports::WorkspaceOpening::open_document(
                &PlatformWorkspaceOpener,
                &path,
            );
        }
    });
    agent_group.add(&custom_provider);
    let config = adw::ActionRow::new();
    config.set_title(&tr(lang, "settings.ai.open_config"));
    config.set_activatable(true);
    config.connect_activated(|_| {
        if let Ok(path) = crate::agent::pi_paths::configuration_file(false) {
            let _ = app_ports::WorkspaceOpening::open_document(
                &PlatformWorkspaceOpener,
                &path,
            );
        }
    });
    agent_group.add(&config);
    let install = adw::ActionRow::new();
    install.set_title(&tr(lang, if crate::agent::app_local_runtime_installed() {
        "settings.ai.reinstall_runtime"
    } else {
        "settings.ai.install_runtime"
    }));
    install.set_subtitle(&tr(lang, "settings.ai.runtime_note"));
    install.set_activatable(true);
    {
        install.connect_activated(move |_| {
            std::thread::spawn(move || {
                let result = crate::agent::pi_installer::ensure_installed();
                let msg = match result {
                    Ok(_) => "Pitex Agent installed.".to_string(),
                    Err(e) => format!("Install failed: {e}"),
                };
                // `invoke` needs `Send` — reach the state through the
                // main-thread-local `STATE` inside the callback instead of
                // capturing the `Rc` here.
                gtk4::glib::MainContext::default().invoke(move || {
                    crate::app_ui::STATE.with(|s| {
                        if let Some(state) = s.borrow().as_ref() {
                            if let Ok(s) = state.try_borrow_mut() { s.toast(&msg); }
                        }
                    });
                });
            });
        });
    }
    agent_group.add(&install);
    ai.add(&agent_group);
    window.add(&ai);

    // ── Updates ──
    // Check GitHub Releases on a worker thread; widgets are touched back on
    // the main loop through `SendWrapper` (they are !Send like every GTK
    // object). The pending UpdateInfo is main-thread `Rc` state shared by
    // the check and install buttons.
    let updates_page = adw::PreferencesPage::new();
    updates_page.set_title(&tr(lang, "settings.tab.updates"));
    updates_page.set_icon_name(Some("software-update-available-symbolic"));
    let update_group = adw::PreferencesGroup::new();
    update_group.set_title(&tr(lang, "settings.updates.section"));

    let version_row = adw::ActionRow::new();
    version_row.set_title(&tr(lang, "settings.updates.current_version"));
    version_row.set_subtitle(&state.borrow().app_version);
    update_group.add(&version_row);

    let check_row = adw::ActionRow::new();
    check_row.set_title(&tr(lang, "settings.updates.check"));
    check_row.set_activatable(true);
    a11y(&check_row, "pitex.settings.updates.check", "settings.updates.check");

    let install_row = adw::ActionRow::new();
    install_row.set_activatable(true);
    install_row.set_visible(false);
    a11y(&install_row, "pitex.settings.updates.install", "settings.updates.install");

    let pending_update: Rc<RefCell<Option<crate::update::UpdateInfo>>> =
        Rc::new(RefCell::new(None));
    UPDATE_ROWS.with(|t| {
        *t.borrow_mut() = Some(UpdateRows {
            check: check_row.downgrade(),
            install: install_row.downgrade(),
            pending: pending_update.clone(),
        });
    });

    {
        let state = state.clone();
        check_row.connect_activated(move |row| {
            row.set_subtitle(&tr(lang, "settings.updates.checking"));
            let version = state.borrow().app_version.clone();
            std::thread::spawn(move || {
                let result = crate::update::check_for_update(&version);
                // `invoke` needs `Send` — only the result crosses; the rows
                // are reached through the main-thread-local weak refs.
                gtk4::glib::MainContext::default().invoke(move || {
                    UPDATE_ROWS.with(|t| {
                        let rows = t.borrow();
                        let Some(rows) = rows.as_ref() else { return };
                        let Some(row) = rows.check.upgrade() else { return };
                        match result {
                            Ok(Some(info)) => {
                                if let Some(install) = rows.install.upgrade() {
                                    install.set_title(&tr1(
                                        lang,
                                        "settings.updates.install_version",
                                        &info.tag,
                                    ));
                                    install.set_visible(true);
                                }
                                row.set_subtitle(&tr1(
                                    lang,
                                    "settings.updates.available",
                                    &info.tag,
                                ));
                                *rows.pending.borrow_mut() = Some(info);
                            }
                            Ok(None) => {
                                row.set_subtitle(&tr(lang, "settings.updates.up_to_date"));
                                *rows.pending.borrow_mut() = None;
                                if let Some(install) = rows.install.upgrade() {
                                    install.set_visible(false);
                                }
                            }
                            Err(e) => {
                                row.set_subtitle(&tr1(lang, "settings.updates.failed", &e))
                            }
                        }
                    });
                });
            });
        });
    }
    update_group.add(&check_row);

    {
        install_row.connect_activated(move |row| {
            let Some(info) = pending_update.borrow().clone() else { return };
            row.set_subtitle(&tr(lang, "settings.updates.downloading"));
            std::thread::spawn(move || {
                let outcome = crate::update::download(&info)
                    .and_then(|path| crate::update::install(&path, &info));
                gtk4::glib::MainContext::default().invoke(move || {
                    UPDATE_ROWS.with(|t| {
                        let rows = t.borrow();
                        let Some(rows) = rows.as_ref() else { return };
                        let Some(row) = rows.install.upgrade() else { return };
                        match outcome {
                            Ok(crate::update::InstallOutcome::ExitRequested) => {
                                std::process::exit(0);
                            }
                            Ok(crate::update::InstallOutcome::AwaitingRestart) => {
                                row.set_subtitle(&tr(lang, "settings.updates.installed"));
                            }
                            Ok(crate::update::InstallOutcome::HandedToTerminal) => {
                                row.set_subtitle(&tr(
                                    lang,
                                    "settings.updates.install_terminal",
                                ));
                            }
                            Ok(crate::update::InstallOutcome::ManualFallback) => {
                                row.set_subtitle(&tr(lang, "settings.updates.manual"));
                            }
                            Err(e) => {
                                row.set_subtitle(&tr1(lang, "settings.updates.failed", &e))
                            }
                        }
                    });
                });
            });
        });
    }
    update_group.add(&install_row);

    let (auto_row, auto_switch) = compat::switch_row(&tr(lang, "settings.updates.auto_install"));
    auto_switch.set_active(state.borrow().store.auto_install_updates());
    a11y(&auto_switch, "pitex.settings.updates.autoInstall", "settings.updates.auto_install");
    {
        let state = state.clone();
        auto_switch.connect_active_notify(move |r| {
            if let Ok(mut s) = state.try_borrow_mut() {
                s.store.set_auto_install_updates(r.is_active());
            }
        });
    }
    update_group.add(&auto_row);
    updates_page.add(&update_group);
    window.add(&updates_page);

    window.present();
}

fn tr1(lang: &str, key: &str, arg: &str) -> String {
    crate::l10n::tr1(lang, key, arg)
}

/// `SyntaxPreview` — the six styled sample lines rendered in the palette and
/// the editor font, over the editor-background role color.
fn apply_syntax_preview(
    state: &AppState,
    label: &gtk4::Label,
) {
    use crate::settings::AppearanceColorRole as R;
    let prefs = state.store.prefs();
    // Pango `foreground` takes #RRGGBB; translucency rides in `alpha`.
    let span_attrs = |role: R| {
        let (r, g, b, a) = state.appearance.color(prefs, role);
        let c = |v: f64| (v.clamp(0.0, 1.0) * 255.0).round() as u32;
        let mut attrs = format!("foreground=\"#{:02X}{:02X}{:02X}\"", c(r), c(g), c(b));
        if a < 1.0 {
            attrs += &format!(" alpha=\"{}\"", (a.clamp(0.0, 1.0) * 65535.0).round() as u32);
        }
        attrs
    };
    // `styled(_:_:)` — words take roles[min(index, count-1)].
    let styled = |text: &str, roles: &[R]| -> String {
        text.split(' ')
            .enumerate()
            .map(|(i, word)| {
                let role = roles[i.min(roles.len() - 1)];
                format!(
                    "<span {}>{}</span>",
                    span_attrs(role),
                    gtk4::glib::markup_escape_text(word)
                )
            })
            .collect::<Vec<_>>()
            .join(" ")
    };
    let lines = [
        styled("\\documentclass{article}", &[R::Commands, R::BodyText]),
        styled("% A comment", &[R::Comments]),
        styled("\\begin{equation}", &[R::Environments]),
        styled("  E = mc^2", &[R::Math]),
        styled("\\end{equation}", &[R::Environments]),
        styled(
            "Body text with \\textbf{braces}.",
            &[R::BodyText, R::Commands, R::Braces],
        ),
    ];
    label.set_markup(&format!(
        "<span font_desc=\"{}\">{}</span>",
        gtk4::glib::markup_escape_text(&state.appearance.font_description()),
        lines.join("\n")
    ));
    // `.background(Color(appearance.color(for: .editorBackground)))` with the
    // rounded, lightly-stroked border.
    let (r, g, b, a) = state
        .appearance
        .color(prefs, R::EditorBackground);
    PREVIEW_CSS.with(|cell| {
        let provider = cell.get_or_init(|| {
            let p = gtk4::CssProvider::new();
            if let Some(display) = gtk4::gdk::Display::default() {
                gtk4::style_context_add_provider_for_display(
                    &display,
                    &p,
                    gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
                );
            }
            p
        });
        provider.load_from_data(&format!(
            "box.pitex-syntax-preview {{ background: rgba({},{},{},{}); border-radius: 6px; \
             border: 1px solid alpha(currentColor, 0.3); padding: 10px; }}",
            (r * 255.0).round() as u32,
            (g * 255.0).round() as u32,
            (b * 255.0).round() as u32,
            a
        ));
    });
}

thread_local! {
    static PREVIEW_CSS: std::cell::OnceCell<gtk4::CssProvider> = std::cell::OnceCell::new();
    /// Weak handles to the settings Updates rows — worker threads post their
    /// results through `MainContext::invoke` and paint them here, since the
    /// widgets themselves are !Send and can't cross the boundary.
    static UPDATE_ROWS: RefCell<Option<UpdateRows>> = const { RefCell::new(None) };
}

struct UpdateRows {
    check: gtk4::glib::WeakRef<adw::ActionRow>,
    install: gtk4::glib::WeakRef<adw::ActionRow>,
    pending: Rc<RefCell<Option<crate::update::UpdateInfo>>>,
}
