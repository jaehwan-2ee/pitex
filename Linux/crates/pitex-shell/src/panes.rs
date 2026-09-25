//! Pane builders: the bottom console (Assistant | Terminal | Issues | Log),
//! the right-hand PDF preview column, and the settings sheet. Mirrors
//! `BottomConsoleView.swift`, `Preview.swift`, `AgentPanel.swift`, and
//! `SettingsView.swift`.

use std::cell::RefCell;
use std::rc::Rc;
use std::path::PathBuf;

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

use crate::app_ui::{a11y, AppState, UiHandles, STATE};
use crate::compat;
use crate::l10n::tr;
use crate::model::{ConsoleSection, WorkspaceBuildState};
use git_core::{GitChange, GitChangeKind, GitCommit};
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
        tr(lang, "git.integration"),
        tr(lang, "build.issues"),
        tr(lang, "console.terminal"),
        tr(lang, "build.log"),
    ];
    let section_strs: Vec<&str> = section_items.iter().map(String::as_str).collect();
    let section = gtk4::DropDown::from_strings(&section_strs);
    compat::initial_tooltip(&section, &tr(lang, "console.terminal"));
    {
        let state = state.clone();
        section.connect_selected_notify(move |dd| {
            let section = match dd.selected() {
                0 => ConsoleSection::Assistant,
                1 => ConsoleSection::Git,
                2 => ConsoleSection::Issues,
                3 => ConsoleSection::Terminal,
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
    compat::initial_tooltip(&build_entry, &tr(lang, "console.build_command_help"));
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
    let tooltip_state = Rc::downgrade(state);
    compat::dynamic_tooltip(&run_build, move || {
        let state = tooltip_state.upgrade()?;
        let state = state.try_borrow().ok()?;
        Some(state.model.build_unavailable_reason().unwrap_or_else(|| tr(state.language, "build.start")))
    });
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
    compat::initial_tooltip(&custom_entry, &tr(lang, "console.custom_command_help"));
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
    compat::initial_tooltip(&run_custom, &tr(lang, "console.custom_run_help"));
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
    stack.add_named(&build_git_pane(state, ui), Some("git"));
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
    // Builds without VTE embed the terminal-core engine instead; an external
    // emulator is only the fallback if the shell fails to spawn (reported via
    // `terminal_running`) or exits.
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

/// `GitIntegrationView` — repository header (name, branch picker, sync
/// buttons), staged/unstaged change lists, the commit box, and the commit
/// graph. `AppState::refresh_git_panel` repopulates it from the model.
fn build_git_pane(state: &Rc<RefCell<AppState>>, ui: &UiHandles) -> gtk4::Widget {
    let lang = state.borrow().language;
    let stack = gtk4::Stack::new();
    stack.set_vexpand(true);
    a11y(&stack, "pitex.git", "git.integration");
    ui.git_stack.replace(Some(stack.clone()));

    // ── Repository page ─────────────────────────────────────────────────
    let repo = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    let header = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    header.set_margin_start(8);
    header.set_margin_end(8);
    header.set_margin_top(6);
    header.set_margin_bottom(6);

    let repo_name = gtk4::Label::new(None);
    repo_name.add_css_class("heading");
    ui.git_repo_name.replace(Some(repo_name.clone()));
    header.append(&repo_name);

    // Branch picker + new-branch — the VSCode branch chip.
    let branch_model = gtk4::StringList::new(&[]);
    let branch_dd = gtk4::DropDown::new(Some(branch_model.clone()), gtk4::Expression::NONE);
    a11y(&branch_dd, "pitex.git.branch", "git.branch");
    compat::initial_tooltip(&branch_dd, &tr(lang, "git.branch"));
    {
        let state = state.clone();
        branch_dd.connect_selected_notify(move |dd| {
            let Ok(mut s) = state.try_borrow_mut() else {
                return;
            };
            if s.git_branch_updating.get() {
                return;
            }
            let Some(branch) = dd
                .selected_item()
                .and_then(|o| o.downcast::<gtk4::StringObject>().ok())
                .map(|o| o.string().to_string())
            else {
                return;
            };
            let current = s
                .model
                .git_status
                .as_ref()
                .map(|g| g.branch.clone())
                .unwrap_or_default();
            if !branch.is_empty() && branch != current {
                s.git_switch(&branch);
            }
        });
    }
    ui.git_branch_model.replace(Some(branch_model));
    ui.git_branch_dropdown.replace(Some(branch_dd.clone()));
    header.append(&branch_dd);

    let new_branch = gtk4::Button::from_icon_name("list-add-symbolic");
    compat::initial_tooltip(&new_branch, &tr(lang, "git.branch_new"));
    a11y(&new_branch, "pitex.git.branchNew", "git.branch_new");
    {
        let state = state.clone();
        new_branch.connect_clicked(move |_| {
            let Ok(mut s) = state.try_borrow_mut() else {
                return;
            };
            s.git_new_branch_dialog(None);
        });
    }
    header.append(&new_branch);

    let ahead_behind = gtk4::Label::new(None);
    ahead_behind.add_css_class("caption");
    ahead_behind.add_css_class("dim-label");
    ahead_behind.set_visible(false);
    ui.git_ahead_behind.replace(Some(ahead_behind.clone()));
    header.append(&ahead_behind);

    let spacer = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
    spacer.set_hexpand(true);
    header.append(&spacer);

    let busy = gtk4::Spinner::new();
    busy.set_visible(false);
    ui.git_busy_spinner.replace(Some(busy.clone()));
    header.append(&busy);

    let pull = git_tool_button(lang, "go-down-symbolic", "git.pull", state, git_pull);
    a11y(&pull, "pitex.git.pull", "git.pull");
    header.append(&pull);
    let push = git_tool_button(lang, "go-up-symbolic", "git.push", state, git_push);
    a11y(&push, "pitex.git.push", "git.push");
    header.append(&push);
    let sync = git_tool_button(lang, "emblem-synchronizing-symbolic", "git.sync", state, git_sync);
    a11y(&sync, "pitex.git.sync", "git.sync");
    header.append(&sync);
    let refresh = git_tool_button(lang, "view-refresh-symbolic", "git.refresh", state, git_refresh);
    a11y(&refresh, "pitex.git.refresh", "git.refresh");
    header.append(&refresh);
    repo.append(&header);

    let error = gtk4::Label::new(None);
    error.add_css_class("caption");
    error.add_css_class("error");
    error.set_xalign(0.0);
    error.set_wrap(true);
    error.set_margin_start(8);
    error.set_margin_end(8);
    error.set_visible(false);
    ui.git_error_label.replace(Some(error.clone()));
    repo.append(&error);
    repo.append(&gtk4::Separator::new(gtk4::Orientation::Horizontal));

    // Content: changes column | graph column — a draggable Paned like the
    // macOS HSplitView (`set_position` seeds the old fixed 340).
    let content = gtk4::Paned::new(gtk4::Orientation::Horizontal);
    content.set_vexpand(true);
    content.set_wide_handle(true);
    content.set_position(340);
    content.set_shrink_start_child(false);
    content.set_shrink_end_child(false);

    let left = gtk4::Box::new(gtk4::Orientation::Vertical, 4);
    left.set_width_request(240);
    left.set_margin_start(8);
    left.set_margin_end(8);
    left.set_margin_bottom(8);
    let changes_scroll = gtk4::ScrolledWindow::new();
    changes_scroll.set_vexpand(true);
    let changes_model = crate::git_list::GitList::new();
    let selection = gtk4::NoSelection::new(Some(changes_model.clone()));
    let factory = gtk4::SignalListItemFactory::new();
    factory.connect_bind(move |_, object| {
        use crate::git_list::Row;
        let item = object.downcast_ref::<gtk4::ListItem>().unwrap();
        let row = item.item().unwrap().downcast::<gtk4::glib::BoxedAnyObject>().unwrap();
        let row = row.borrow::<Row>();
        item.set_selectable(false);
        item.set_activatable(matches!(*row, Row::Change(_)));
        let child: gtk4::Widget = match &*row {
            Row::Empty => {
                let label = gtk4::Label::new(Some(&tr(lang, "git.no_changes")));
                label.add_css_class("dim-label");
                label.set_margin_top(12);
                label.upcast()
            }
            Row::Section { staged, count } => git_section_row(
                &tr(lang, if *staged { "git.staged" } else { "git.changes" }), *count,
                if *staged { "list-remove-symbolic" } else { "list-add-symbolic" },
                &tr(lang, if *staged { "git.unstage_all" } else { "git.stage_all" }),
                if *staged { AppState::git_unstage_all } else { AppState::git_stage_all },
            ).upcast(),
            Row::Change(change) => git_change_row(change, lang).upcast(),
        };
        item.set_child(Some(&child));
    });
    factory.connect_unbind(|_, object| {
        object.downcast_ref::<gtk4::ListItem>().unwrap().set_child(None::<&gtk4::Widget>);
    });
    let changes_list = gtk4::ListView::new(Some(selection), Some(factory));
    changes_list.set_single_click_activate(true);
    changes_list.add_css_class("boxed-list");
    a11y(&changes_list, "pitex.git.changes", "git.changes");
    {
        let state = state.clone();
        let model = changes_model.clone();
        changes_list.connect_activate(move |_, position| {
            if let Some(crate::git_list::Row::Change(change)) = model.row(position) {
                if let Ok(mut s) = state.try_borrow_mut() { s.git_open_working_diff(&change); }
            }
        });
    }
    changes_scroll.set_child(Some(&changes_list));
    ui.git_changes_list.replace(Some(changes_list.clone()));
    ui.git_changes_model.replace(Some(changes_model));
    left.append(&changes_scroll);

    // Commit box — TextView (⌃Enter commits) + suggested-action button.
    let commit_box = gtk4::Box::new(gtk4::Orientation::Vertical, 6);
    let commit_scroll = gtk4::ScrolledWindow::new();
    commit_scroll.set_height_request(52);
    commit_scroll.set_propagate_natural_height(true);
    let commit_view = gtk4::TextView::new();
    commit_view.set_wrap_mode(gtk4::WrapMode::Word);
    commit_view.set_top_margin(4);
    commit_view.set_bottom_margin(4);
    commit_view.set_left_margin(6);
    commit_view.set_right_margin(6);
    commit_scroll.set_child(Some(&commit_view));
    commit_scroll.add_css_class("card");
    commit_box.append(&commit_scroll);
    {
        // Buffer → `gitCommitMessage`.
        let state = state.clone();
        commit_view.buffer().connect_changed(move |b| {
            let Ok(mut s) = state.try_borrow_mut() else {
                return;
            };
            s.model.git_commit_message = b.text(&b.start_iter(), &b.end_iter(), false).to_string();
            s.refresh_git_commit_button();
        });
    }
    {
        // ⌃Enter commits — capture on the ancestor so it lands before the
        // view inserts a newline.
        let keyctl = gtk4::EventControllerKey::new();
        keyctl.set_propagation_phase(gtk4::PropagationPhase::Capture);
        let view = commit_view.clone();
        keyctl.connect_key_pressed(move |_, key, _, mods| {
            if key == gtk4::gdk::Key::Return
                && mods.contains(gtk4::gdk::ModifierType::CONTROL_MASK)
                && view.has_focus()
            {
                STATE.with(|s| {
                    if let Some(state) = s.borrow().as_ref() {
                        if let Ok(mut st) = state.try_borrow_mut() {
                            st.git_commit();
                        }
                    }
                });
                return gtk4::glib::Propagation::Stop;
            }
            gtk4::glib::Propagation::Proceed
        });
        commit_box.add_controller(keyctl);
    }
    ui.git_commit_view.replace(Some(commit_view));

    // Commit | Suggest — the right half asks Pitex Agent to draft the
    // message into the box; Commit still requires a message.
    let commit_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
    let commit_button = gtk4::Button::with_label(&tr(lang, "git.commit"));
    commit_button.add_css_class("suggested-action");
    commit_button.set_hexpand(true);
    a11y(&commit_button, "pitex.git.commit", "git.commit");
    {
        let state = state.clone();
        commit_button.connect_clicked(move |_| {
            let Ok(mut s) = state.try_borrow_mut() else {
                return;
            };
            s.git_commit();
        });
    }
    ui.git_commit_button.replace(Some(commit_button.clone()));
    commit_row.append(&commit_button);
    let suggest_button = gtk4::Button::with_label(&tr(lang, "git.suggest"));
    suggest_button.set_hexpand(true);
    compat::initial_tooltip(&suggest_button, &tr(lang, "git.suggest_help"));
    a11y(&suggest_button, "pitex.git.suggest", "git.suggest");
    {
        let state = state.clone();
        suggest_button.connect_clicked(move |_| {
            let Ok(mut s) = state.try_borrow_mut() else {
                return;
            };
            s.git_suggest_message();
        });
    }
    ui.git_suggest_button.replace(Some(suggest_button.clone()));
    commit_row.append(&suggest_button);
    commit_box.append(&commit_row);
    left.append(&commit_box);
    content.set_start_child(Some(&left));

    let right = gtk4::Box::new(gtk4::Orientation::Vertical, 4);
    right.set_hexpand(true);
    right.set_margin_start(8);
    right.set_margin_end(8);
    right.set_margin_top(4);
    let graph_title = gtk4::Label::new(Some(&tr(lang, "git.graph")));
    graph_title.add_css_class("caption");
    graph_title.add_css_class("heading");
    graph_title.set_xalign(0.0);
    right.append(&graph_title);
    let graph_scroll = gtk4::ScrolledWindow::new();
    graph_scroll.set_vexpand(true);
    let graph_list = gtk4::ListBox::new();
    graph_list.set_selection_mode(gtk4::SelectionMode::None);
    a11y(&graph_list, "pitex.git.graph", "git.graph");
    {
        // `toggleGitCommit` — activation expands/collapses the file list.
        let state = state.clone();
        graph_list.connect_row_activated(move |_, row| {
            let hash = row.widget_name();
            let hash = hash.strip_prefix("pitex.git.commit.").unwrap_or(&hash);
            let Ok(mut s) = state.try_borrow_mut() else { return };
            let Some(commit) = s
                .model
                .git_commits
                .iter()
                .find(|c| c.hash == hash)
                .cloned()
            else {
                return;
            };
            s.git_toggle_commit(&commit);
            s.refresh_git_panel();
        });
    }
    graph_scroll.set_child(Some(&graph_list));
    ui.git_graph_list.replace(Some(graph_list.clone()));
    right.append(&graph_scroll);
    content.set_end_child(Some(&right));

    repo.append(&content);
    stack.add_named(&repo, Some("repo"));

    // ── Empty page ──────────────────────────────────────────────────────
    let empty = adw::StatusPage::new();
    empty.set_title(&tr(lang, "git.no_repo"));
    empty.set_icon_name(Some("folder-symbolic"));
    let init = gtk4::Button::with_label(&tr(lang, "git.init"));
    init.add_css_class("suggested-action");
    init.set_halign(gtk4::Align::Center);
    a11y(&init, "pitex.git.init", "git.init");
    {
        let state = state.clone();
        init.connect_clicked(move |_| {
            let Ok(mut s) = state.try_borrow_mut() else {
                return;
            };
            s.git_init();
        });
    }
    empty.set_child(Some(&init));
    stack.add_named(&empty, Some("empty"));
    stack.set_visible_child_name("empty");
    stack.upcast()
}

fn git_refresh(s: &mut AppState) {
    s.refresh_git();
}
fn git_pull(s: &mut AppState) {
    s.git_pull();
}
fn git_push(s: &mut AppState) {
    s.git_push();
}
fn git_sync(s: &mut AppState) {
    s.git_sync();
}

fn git_tool_button(
    lang: &str,
    icon: &str,
    tip: &str,
    state: &Rc<RefCell<AppState>>,
    action: fn(&mut AppState),
) -> gtk4::Button {
    let button = gtk4::Button::from_icon_name(icon);
    compat::initial_tooltip(&button, &tr(lang, tip));
    let state = state.clone();
    button.connect_clicked(move |_| {
        let Ok(mut s) = state.try_borrow_mut() else {
            return;
        };
        action(&mut s);
    });
    button
}

/// Section header row inside the changes list — `git.staged`/`git.changes`
/// label, count, and the stage-all/unstage-all action.
pub(crate) fn git_section_row(
    title: &str,
    count: usize,
    icon: &str,
    tip: &str,
    action: fn(&mut AppState),
) -> gtk4::Box {
    let hbox = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
    hbox.set_margin_start(8);
    hbox.set_margin_end(8);
    hbox.set_margin_top(4);
    hbox.set_margin_bottom(4);
    let label = gtk4::Label::new(Some(&format!("{title} ({count})")));
    label.add_css_class("caption");
    label.add_css_class("heading");
    label.set_xalign(0.0);
    label.set_hexpand(true);
    hbox.append(&label);
    let button = gtk4::Button::from_icon_name(icon);
    compat::initial_tooltip(&button, tip);
    button.add_css_class("flat");
    button.connect_clicked(move |_| {
        STATE.with(|s| {
            if let Some(state) = s.borrow().as_ref() {
                if let Ok(mut st) = state.try_borrow_mut() {
                    action(&mut st);
                }
            }
        });
    });
    hbox.append(&button);
    hbox
}

/// One `GitChange` row — badge letter, file name, path, and the
/// open-file + stage/unstage + discard actions (row activation opens
/// the working-tree diff).
pub(crate) fn git_change_row(change: &GitChange, lang: &str) -> gtk4::Box {
    let hbox = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
    hbox.set_margin_start(8);
    hbox.set_margin_end(8);
    hbox.set_margin_top(2);
    hbox.set_margin_bottom(2);
    hbox.set_widget_name(&format!(
        "gitc:{}:{}",
        if change.staged { "s" } else { "u" },
        change.path
    ));

    let badge = gtk4::Label::new(Some(change.kind.badge()));
    badge.add_css_class("caption");
    badge.add_css_class("monospace");
    badge.add_css_class(match change.kind {
        GitChangeKind::Modified | GitChangeKind::TypeChanged => "warning",
        GitChangeKind::Added | GitChangeKind::Untracked => "success",
        GitChangeKind::Deleted | GitChangeKind::Conflicted => "error",
        GitChangeKind::Renamed | GitChangeKind::Copied => "accent",
    });
    hbox.append(&badge);

    let name = change
        .path
        .rsplit('/')
        .next()
        .unwrap_or(&change.path)
        .to_string();
    let name_label = gtk4::Label::new(Some(&name));
    name_label.set_ellipsize(gtk4::pango::EllipsizeMode::End);
    hbox.append(&name_label);
    if let Some(dir) = change.path.rsplit_once('/').map(|(d, _)| d) {
        let dir_label = gtk4::Label::new(Some(dir));
        dir_label.add_css_class("caption");
        dir_label.add_css_class("dim-label");
        dir_label.set_ellipsize(gtk4::pango::EllipsizeMode::Start);
        dir_label.set_hexpand(true);
        dir_label.set_xalign(0.0);
        hbox.append(&dir_label);
    } else {
        let spacer = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        spacer.set_hexpand(true);
        hbox.append(&spacer);
    }

    let change = change.clone();
    // Cloned before the toggle closure moves `change` — discard renders last.
    let discard_change = change.clone();
    {
        // ↗ opens the file itself — row activation shows its diff.
        let change = change.clone();
        let open = gtk4::Button::from_icon_name("document-open-symbolic");
        compat::initial_tooltip(&open, &tr(lang, "git.open_file"));
        open.add_css_class("flat");
        open.connect_clicked(move |_| {
            STATE.with(|s| {
                if let Some(state) = s.borrow().as_ref() {
                    if let Ok(mut st) = state.try_borrow_mut() {
                        st.git_open_change(&change);
                    }
                }
            });
        });
        hbox.append(&open);
    }
    {
        let toggle = gtk4::Button::from_icon_name(if change.staged {
            "list-remove-symbolic"
        } else {
            "list-add-symbolic"
        });
        compat::initial_tooltip(&toggle, &tr(
            lang,
            if change.staged {
                "git.unstage"
            } else {
                "git.stage"
            },
        ));
        toggle.add_css_class("flat");
        toggle.connect_clicked(move |_| {
            STATE.with(|s| {
                if let Some(state) = s.borrow().as_ref() {
                    if let Ok(mut st) = state.try_borrow_mut() {
                        if change.staged {
                            st.git_unstage(&change);
                        } else {
                            st.git_stage(&change);
                        }
                    }
                }
            });
        });
        hbox.append(&toggle);
    }
    {
        let change = discard_change;
        let discard = gtk4::Button::from_icon_name("user-trash-symbolic");
        compat::initial_tooltip(&discard, &tr(lang, "git.discard"));
        discard.add_css_class("flat");
        discard.connect_clicked(move |_| {
            STATE.with(|s| {
                if let Some(state) = s.borrow().as_ref() {
                    if let Ok(mut st) = state.try_borrow_mut() {
                        st.git_pending_discard = Some(change.clone());
                        st.git_discard_dialog();
                    }
                }
            });
        });
        hbox.append(&discard);
    }
    hbox
}

/// One `GitCommit` row — disclosure chevron, lane dot + connector,
/// subject, ref chips, and the author · date · hash subtitle. Activation
/// toggles the changed-files list (VSCode/GitLens-style expansion).
pub(crate) fn git_commit_row(
    commit: &GitCommit,
    expanded: bool,
    files: Option<&[git_core::GitCommitFile]>,
    busy: bool,
    lang: &'static str,
) -> gtk4::ListBoxRow {
    let row = gtk4::ListBoxRow::new();
    row.set_activatable(true);
    row.set_selectable(false);
    row.set_widget_name(&format!("pitex.git.commit.{}", commit.hash));
    let outer = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    let hbox = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    hbox.set_margin_start(8);
    hbox.set_margin_end(8);
    hbox.set_margin_top(2);
    hbox.set_margin_bottom(2);

    let chevron = gtk4::Image::from_icon_name(if expanded {
        "pan-down-symbolic"
    } else {
        "pan-end-symbolic"
    });
    chevron.add_css_class("dim-label");
    chevron.set_valign(gtk4::Align::Start);
    chevron.set_margin_top(4);
    hbox.append(&chevron);

    let lane = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    let dot = gtk4::Label::new(Some("●"));
    if commit.is_head {
        dot.add_css_class("accent");
    } else {
        dot.add_css_class("dim-label");
    }
    lane.append(&dot);
    let line = gtk4::Separator::new(gtk4::Orientation::Vertical);
    line.set_vexpand(true);
    line.set_opacity(0.3);
    lane.append(&line);
    hbox.append(&lane);

    let vbox = gtk4::Box::new(gtk4::Orientation::Vertical, 2);
    let title = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
    let subject = gtk4::Label::new(Some(&commit.subject));
    subject.set_ellipsize(gtk4::pango::EllipsizeMode::End);
    subject.set_xalign(0.0);
    title.append(&subject);
    for r in &commit.refs {
        let chip = gtk4::Label::new(Some(&format!("[{}]", r.replace("tag: ", ""))));
        chip.add_css_class("caption");
        chip.add_css_class("accent");
        title.append(&chip);
    }
    vbox.append(&title);
    let subtitle = gtk4::Label::new(Some(&format!(
        "{} · {} · {}",
        commit.author, commit.relative_date, commit.hash
    )));
    subtitle.add_css_class("caption");
    subtitle.add_css_class("dim-label");
    subtitle.set_xalign(0.0);
    vbox.append(&subtitle);
    vbox.set_hexpand(true);
    hbox.append(&vbox);
    outer.append(&hbox);

    // Changed files under an expanded commit — each opens the diff.
    if expanded {
        let file_list = gtk4::Box::new(gtk4::Orientation::Vertical, 2);
        file_list.set_margin_start(28);
        file_list.set_margin_top(2);
        file_list.set_margin_bottom(2);
        if busy {
            let spinner = gtk4::Spinner::new();
            spinner.set_halign(gtk4::Align::Start);
            spinner.start();
            file_list.append(&spinner);
        } else {
            for file in files.unwrap_or(&[]) {
                file_list.append(&commit_file_row(commit, file, lang));
            }
        }
        outer.append(&file_list);
    }

    row.set_child(Some(&outer));
    attach_commit_menu(&row, commit, lang);
    row
}

/// A file under an expanded commit — badge + name + dim directory;
/// clicking opens the side-by-side diff (`pitex.git.commitFile`).
fn commit_file_row(
    commit: &GitCommit,
    file: &git_core::GitCommitFile,
    lang: &'static str,
) -> gtk4::Button {
    let button = gtk4::Button::new();
    button.add_css_class("flat");
    button.set_widget_name("pitex.git.commitFile");
    compat::initial_tooltip(&button, &tr(lang, "git.open_diff"));
    let hbox = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
    let badge = gtk4::Label::new(Some(file.kind.badge()));
    badge.add_css_class("monospace");
    badge.add_css_class("caption");
    badge.add_css_class(match file.kind {
        git_core::GitChangeKind::Modified | git_core::GitChangeKind::TypeChanged => "warning",
        git_core::GitChangeKind::Added | git_core::GitChangeKind::Untracked => "success",
        git_core::GitChangeKind::Deleted | git_core::GitChangeKind::Conflicted => "error",
        git_core::GitChangeKind::Renamed | git_core::GitChangeKind::Copied => "accent",
    });
    badge.set_width_chars(2);
    hbox.append(&badge);
    let name = file.path.rsplit('/').next().unwrap_or(&file.path).to_string();
    let name_label = gtk4::Label::new(Some(&name));
    name_label.set_ellipsize(gtk4::pango::EllipsizeMode::End);
    name_label.set_xalign(0.0);
    hbox.append(&name_label);
    if let Some(dir) = file.path.rsplit_once('/').map(|(d, _)| d) {
        let dir_label = gtk4::Label::new(Some(dir));
        dir_label.add_css_class("caption");
        dir_label.add_css_class("dim-label");
        dir_label.set_ellipsize(gtk4::pango::EllipsizeMode::Start);
        dir_label.set_xalign(0.0);
        hbox.append(&dir_label);
    }
    button.set_child(Some(&hbox));
    {
        let commit = commit.clone();
        let file = file.clone();
        button.connect_clicked(move |_| {
            STATE.with(|s| {
                if let Some(state) = s.borrow().as_ref() {
                    if let Ok(mut st) = state.try_borrow_mut() {
                        st.git_open_commit_diff(&commit, file.clone());
                    }
                }
            });
        });
    }
    button
}

/// Right-click popover on a graph row — VSCode's commit context menu:
/// Open Changes, Copy Commit Hash, Copy Commit Message, New Branch.
fn attach_commit_menu(row: &gtk4::ListBoxRow, commit: &GitCommit, lang: &'static str) {
    let click = gtk4::GestureClick::new();
    click.set_button(gtk4::gdk::BUTTON_SECONDARY);
    let commit = commit.clone();
    click.connect_pressed(move |gesture, _n, x, y| {
        let row = gesture.widget();
        let popover = gtk4::Popover::new();
        let vbox = gtk4::Box::new(gtk4::Orientation::Vertical, 2);
        vbox.set_margin_top(4);
        vbox.set_margin_bottom(4);
        vbox.set_margin_start(4);
        vbox.set_margin_end(4);
        popover.set_child(Some(&vbox));

        let open = gtk4::Button::with_label(&tr(lang, "git.open_changes"));
        open.set_widget_name("pitex.git.commit.openChanges");
        open.set_has_frame(false);
        {
            let commit = commit.clone();
            let popover = popover.clone();
            open.connect_clicked(move |_| {
                popover.popdown();
                STATE.with(|s| {
                    if let Some(state) = s.borrow().as_ref() {
                        if let Ok(mut st) = state.try_borrow_mut() {
                            st.git_open_commit_full_diff(&commit);
                        }
                    }
                });
            });
        }
        vbox.append(&open);

        let copy_hash = gtk4::Button::with_label(&tr(lang, "git.copy_hash"));
        copy_hash.set_widget_name("pitex.git.commit.copyHash");
        copy_hash.set_has_frame(false);
        {
            let text = commit.full_hash.clone();
            let popover = popover.clone();
            copy_hash.connect_clicked(move |b| {
                b.clipboard().set_text(&text);
                popover.popdown();
            });
        }
        vbox.append(&copy_hash);

        let copy_message = gtk4::Button::with_label(&tr(lang, "git.copy_message"));
        copy_message.set_widget_name("pitex.git.commit.copyMessage");
        copy_message.set_has_frame(false);
        {
            let text = commit.message.clone();
            let popover = popover.clone();
            copy_message.connect_clicked(move |b| {
                b.clipboard().set_text(&text);
                popover.popdown();
            });
        }
        vbox.append(&copy_message);

        vbox.append(&gtk4::Separator::new(gtk4::Orientation::Horizontal));

        let branch = gtk4::Button::with_label(&tr(lang, "git.branch_new"));
        branch.set_widget_name("pitex.git.commit.newBranch");
        branch.set_has_frame(false);
        {
            let hash = commit.full_hash.clone();
            let popover = popover.clone();
            branch.connect_clicked(move |_| {
                popover.popdown();
                let hash = hash.clone();
                STATE.with(|s| {
                    if let Some(state) = s.borrow().as_ref() {
                        if let Ok(mut st) = state.try_borrow_mut() {
                            st.git_new_branch_dialog(Some(hash.clone()));
                        }
                    }
                });
            });
        }
        vbox.append(&branch);

        popover.set_pointing_to(Some(&gtk4::gdk::Rectangle::new(x as i32, y as i32, 1, 1)));
        popover.set_parent(&row);
        popover.connect_closed(|p| p.unparent());
        popover.popup();
    });
    row.add_controller(click);
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
    compat::initial_tooltip(&reasoning, &tr(lang, "assistant.reasoning_help"));
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
    compat::initial_tooltip(&clear, &tr(lang, "assistant.clear"));
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

    // Past conversations of this project; `/resume` opens it too.
    let history = gtk4::Button::from_icon_name("document-open-recent-symbolic");
    compat::initial_tooltip(&history, &tr(lang, "assistant.history"));
    {
        let state = state.clone();
        history.connect_clicked(move |button| {
            let sessions = {
                let Ok(mut s) = state.try_borrow_mut() else { return };
                let Some(agent) = s.agent.as_mut() else { return };
                if agent.is_running {
                    return;
                }
                agent.load_past_sessions();
                agent.past_sessions.clone()
            };
            show_session_history(button, sessions, lang);
        });
    }
    ui.agent_history_button.replace(Some(history.clone()));
    controls.append(&history);
    let usage = gtk4::Label::new(None);
    usage.set_hexpand(true);
    usage.set_halign(gtk4::Align::End);
    usage.add_css_class("dim-label");
    usage.set_widget_name("pitex.assistant.usage");
    usage.set_visible(false);
    ui.agent_usage_label.replace(Some(usage.clone()));
    controls.append(&usage);
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
    compat::initial_tooltip(&attach, &tr(lang, "assistant.attach_files_help"));
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

    // Slash-command completion — a popover on the entry listing
    // `get_commands` results matching the typed prefix (max 8 rows).
    let slash_list = gtk4::ListBox::new();
    slash_list.set_selection_mode(gtk4::SelectionMode::Single);
    let slash_scroll = gtk4::ScrolledWindow::new();
    slash_scroll.set_max_content_height(280);
    slash_scroll.set_propagate_natural_height(true);
    slash_scroll.set_child(Some(&slash_list));
    let slash_popover = gtk4::Popover::new();
    slash_popover.set_child(Some(&slash_scroll));
    slash_popover.set_position(gtk4::PositionType::Top);
    slash_popover.set_has_arrow(false);
    slash_popover.set_parent(&entry);
    let slash_names: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
    {
        let entry = entry.clone();
        let popover = slash_popover.clone();
        let names = slash_names.clone();
        slash_list.connect_row_activated(move |_, row| {
            if let Some(name) = names.borrow().get(row.index().max(0) as usize) {
                entry.set_text(&format!("/{name} "));
                entry.set_position(-1);
            }
            popover.popdown();
        });
    }
    {
        let state = state.clone();
        let popover = slash_popover.clone();
        let list = slash_list.clone();
        let names = slash_names.clone();
        entry.connect_changed(move |e| {
            let text = e.text().to_string();
            if !text.starts_with('/') || text.contains(char::is_whitespace) {
                popover.popdown();
                return;
            }
            let prefix = text[1..].to_lowercase();
            let Ok(s) = state.try_borrow() else { return };
            let Some(agent) = s.agent.as_ref() else {
                popover.popdown();
                return;
            };
            let matches: Vec<crate::agent::PiSlashCommand> = agent
                .commands
                .iter()
                .filter(|c| c.name.to_lowercase().starts_with(&prefix))
                .take(8)
                .cloned()
                .collect();
            if matches.is_empty() {
                popover.popdown();
                return;
            }
            while let Some(child) = list.first_child() {
                list.remove(&child);
            }
            *names.borrow_mut() = matches.iter().map(|c| c.name.clone()).collect();
            for command in &matches {
                let row = adw::ActionRow::new();
                row.set_title(&format!("/{}", command.name));
                if let Some(description) = &command.description {
                    row.set_subtitle(description);
                }
                if !command.source.is_empty() {
                    let source = gtk4::Label::new(Some(&command.source));
                    source.add_css_class("dim-label");
                    source.add_css_class("caption");
                    source.set_valign(gtk4::Align::Center);
                    row.add_suffix(&source);
                }
                row.set_activatable(true);
                list.append(&row);
            }
            popover.popup();
        });
    }

    let send = gtk4::Button::from_icon_name("go-up-symbolic");
    compat::initial_tooltip(&send, &tr(lang, "assistant.send_help"));
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
    compat::initial_tooltip(&stop, &tr(lang, "assistant.stop_help"));
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

/// `Preview.preview`: SyncTeX status row, toolbar, page view. The whole
/// column swaps for the Markdown preview while a Markdown document is
/// active (`refresh_pdf_ui` toggles the two children).
pub fn build_preview_pane(state: &Rc<RefCell<AppState>>, ui: &UiHandles) -> gtk4::Widget {
    let lang = state.borrow().language;
    let root = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    ui.pdf_column.replace(Some(root.clone()));

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
    compat::initial_tooltip(&external, &tr(lang, "preview.open_external"));
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
    compat::initial_tooltip(&download, &tr(lang, "preview.download"));
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
    compat::initial_tooltip(&prev, &tr(lang, "preview.previous_page"));
    let next = gtk4::Button::from_icon_name("go-next-symbolic");
    compat::initial_tooltip(&next, &tr(lang, "preview.next_page"));
    let page_label = gtk4::Label::new(Some("—"));
    let zoom_out = gtk4::Button::from_icon_name("zoom-out-symbolic");
    compat::initial_tooltip(&zoom_out, &tr(lang, "preview.zoom_out"));
    let zoom_in = gtk4::Button::from_icon_name("zoom-in-symbolic");
    compat::initial_tooltip(&zoom_in, &tr(lang, "preview.zoom_in"));
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
    let markdown = state.borrow().markdown.widget.clone();
    markdown.set_visible(false);
    let outer = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    a11y(&outer, "pitex.inspector", "preview.title");
    outer.append(&root);
    outer.append(&markdown);
    outer.upcast()
}

// ─── Settings ────────────────────────────────────────────────────────────────

/// `SettingsView` — an `adw::PreferencesWindow` with the six panes
/// (TeX Compile / Markdown / Editor / Appearance / AI / Updates) replacing
/// the capsule tab strip.
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

    // ── Markdown ──
    let markdown = adw::PreferencesPage::new();
    markdown.set_title(&tr(lang, "settings.tab.markdown"));
    markdown.set_icon_name(Some("text-x-generic-symbolic"));
    let preview_group = adw::PreferencesGroup::new();
    // The `markdownTab` order — Live preview, Sync scrolling, Preview
    // theme, Preview font size.
    let (live_row, live) = compat::switch_row(&tr(lang, "settings.markdown.live_preview"));
    live_row.set_subtitle(&tr(lang, "settings.markdown.live_preview_note"));
    live.set_active(state.borrow().store.markdown_live_preview());
    {
        let state = state.clone();
        live.connect_active_notify(move |r| {
            if let Ok(mut s) = state.try_borrow_mut() {
                s.store.set_markdown_live_preview(r.is_active());
                s.render_markdown_now();
            }
        });
    }
    preview_group.add(&live_row);
    let (sync_row, sync) = compat::switch_row(&tr(lang, "settings.markdown.sync_scroll"));
    sync_row.set_subtitle(&tr(lang, "settings.markdown.sync_scroll_note"));
    sync.set_active(state.borrow().store.markdown_sync_scroll());
    {
        let state = state.clone();
        sync.connect_active_notify(move |r| {
            if let Ok(mut s) = state.try_borrow_mut() {
                s.store.set_markdown_sync_scroll(r.is_active());
            }
        });
    }
    preview_group.add(&sync_row);
    let theme = adw::ComboRow::new();
    theme.set_title(&tr(lang, "settings.markdown.theme"));
    let theme_items = [
        tr(lang, "settings.markdown.theme.system"),
        tr(lang, "settings.markdown.theme.light"),
        tr(lang, "settings.markdown.theme.dark"),
    ];
    let theme_strs: Vec<&str> = theme_items.iter().map(String::as_str).collect();
    theme.set_model(Some(&gtk4::StringList::new(&theme_strs)));
    theme.set_selected(match state.borrow().store.markdown_theme() {
        "light" => 1,
        "dark" => 2,
        _ => 0,
    });
    {
        let state = state.clone();
        theme.connect_selected_notify(move |r| {
            let v = ["system", "light", "dark"]
                .get(r.selected() as usize)
                .copied()
                .unwrap_or("system");
            if let Ok(mut s) = state.try_borrow_mut() {
                s.store.set_markdown_theme(v);
                s.render_markdown_now();
            }
        });
    }
    preview_group.add(&theme);
    let (size_row, size) =
        compat::spin_row(&tr(lang, "settings.markdown.font_size"), 10.0, 28.0, 1.0);
    size.set_value(state.borrow().store.markdown_font_size());
    {
        let state = state.clone();
        size.connect_value_notify(move |r| {
            if let Ok(mut s) = state.try_borrow_mut() {
                s.store.set_markdown_font_size(r.value());
                s.render_markdown_now();
            }
        });
    }
    preview_group.add(&size_row);
    markdown.add(&preview_group);
    // Markdown twin of the TeX Compile page's default-editor row — shown
    // wherever the Markdown preview exists (every Linux build, and Windows
    // whose `markdown-preview` feature maps to WebView2).
    #[cfg(feature = "markdown-preview")]
    {
        let default_group = adw::PreferencesGroup::new();
        let make_default = adw::ActionRow::new();
        make_default.set_title(&tr(lang, "settings.markdown.make_default_editor"));
        make_default.set_subtitle(&tr(lang, "settings.markdown.make_default_note"));
        make_default.set_activatable(true);
        make_default.connect_activated(|_| {
            let _ = PlatformDefaultEditorRegistration::register_markdown_as_default();
        });
        default_group.add(&make_default);
        markdown.add(&default_group);
    }
    window.add(&markdown);

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
    // No separate light/dark row — each theme preset already picks the
    // app-wide mode it implies (apply_preset sets `appearance.theme`).
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
    // `Toggle("settings.ai.autocompletion")` — Copilot-style ghost text;
    // the completion coordinator re-reads the setting at every fire so
    // the toggle applies without touching the editor.
    let (autocompletion_row, autocompletion) =
        compat::switch_row(&tr(lang, "settings.ai.autocompletion"));
    autocompletion.set_active(state.borrow().store.ai_autocompletion());
    {
        let state = state.clone();
        autocompletion.connect_active_notify(move |r| {
            if let Ok(mut s) = state.try_borrow_mut() {
                s.store.set_ai_autocompletion(r.is_active());
                if !r.is_active() {
                    s.completion.dismiss();
                }
            }
        });
    }
    agent_group.add(&autocompletion_row);
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
    let update_agent = gtk4::Button::with_label(&tr(lang, "settings.ai.update_agent"));
    update_agent.set_valign(gtk4::Align::Center);
    a11y(&update_agent, "pitex.settings.ai.updateAgent", "settings.ai.update_agent");
    custom_provider.add_suffix(&update_agent);
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
    install.set_widget_name("pitex.settings.ai.runtimeStatus");
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
                    STATE.with(|s| {
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

    // `Section("settings.ai.skills")` — installed skills under the agent
    // dir plus an install row (`pi install <source>` on a worker thread).
    let skills_group = adw::PreferencesGroup::new();
    skills_group.set_title(&tr(lang, "settings.ai.skills"));
    let skill_rows: Rc<RefCell<Vec<gtk4::Widget>>> = Rc::new(RefCell::new(Vec::new()));
    // `compat::entry_row` — `adw::EntryRow` needs libadwaita 1.2 but the
    // Ubuntu 22.04 compat build targets 1.1.
    let (install_row, install_entry) = crate::compat::entry_row(&tr(lang, "settings.ai.install_skill"));
    let install_button = gtk4::Button::with_label(&tr(lang, "settings.ai.install"));
    install_button.set_valign(gtk4::Align::Center);
    install_row.add_suffix(&install_button);
    let rebuild: Rc<RefCell<Option<Rc<dyn Fn()>>>> = Rc::new(RefCell::new(None));
    {
        let skills_group = skills_group.clone();
        let skill_rows = skill_rows.clone();
        let install_row = install_row.clone();
        let rebuild = rebuild.clone();
        let cell = rebuild.clone();
        *cell.borrow_mut() = Some(Rc::new(move || {
            for row in skill_rows.borrow_mut().drain(..) {
                skills_group.remove(&row);
            }
            // Re-append the install row last — `add` appends at the end.
            if install_row.parent().is_some() { skills_group.remove(&install_row); }
            for skill in installed_skills() {
                let row = adw::ActionRow::new();
                row.set_title(&skill.name);
                if skill.builtin {
                    row.set_subtitle(&tr(lang, "settings.ai.builtin"));
                } else if !skill.description.is_empty() {
                    row.set_subtitle(&skill.description);
                }
                if !skill.builtin {
                    let remove = gtk4::Button::with_label(&tr(lang, "settings.ai.remove"));
                    remove.set_valign(gtk4::Align::Center);
                    remove.add_css_class("flat");
                    let path = skill.path.clone();
                    let rebuild = rebuild.clone();
                    remove.connect_clicked(move |_| {
                        let _ = std::fs::remove_dir_all(&path);
                        if let Some(f) = rebuild.borrow().as_ref() {
                            f();
                        }
                    });
                    row.add_suffix(&remove);
                }
                skills_group.add(&row);
                skill_rows.borrow_mut().push(row.upcast());
            }
            if skill_rows.borrow().is_empty() {
                let empty = adw::ActionRow::new();
                empty.set_title(&tr(lang, "settings.ai.no_skills"));
                empty.set_sensitive(false);
                skills_group.add(&empty);
                skill_rows.borrow_mut().push(empty.upcast());
            }
            skills_group.add(&install_row);
        }));
    }
    if let Some(f) = rebuild.borrow().as_ref() {
        f();
    }
    {
        let state = state.clone();
        let install_entry = install_entry.clone();
        SKILLS_REBUILD.with(|t| *t.borrow_mut() = rebuild.borrow().clone());
        install_button.connect_clicked(move |_| {
            let source = install_entry.text().trim().to_string();
            if source.is_empty() {
                return;
            }
            install_entry.set_text("");
            if let Ok(s) = state.try_borrow() {
                s.toast(&tr(lang, "settings.ai.skill_installing"));
            }
            std::thread::spawn(move || {
                let result = crate::agent::pi_installer::install_skill(&source);
                gtk4::glib::MainContext::default().invoke(move || {
                    STATE.with(|s| {
                        if let Some(state) = s.borrow().as_ref() {
                            if let Ok(st) = state.try_borrow() {
                                let message = match &result {
                                    Ok(()) => tr(lang, "settings.ai.skill_installed"),
                                    Err(e) => tr1(lang, "settings.ai.skill_failed", e),
                                };
                                st.toast(&message);
                            }
                        }
                    });
                    SKILLS_REBUILD.with(|t| {
                        if let Some(f) = t.borrow().as_ref() {
                            f();
                        }
                    });
                });
            });
        });
    }
    ai.add(&skills_group);
    {
        let state = state.clone();
        let agent_group = agent_group.downgrade();
        let skills_group = skills_group.downgrade();
        let status = install.downgrade();
        update_agent.connect_clicked(move |button| {
            let (Some(agent_group), Some(skills_group), Some(status)) =
                (agent_group.upgrade(), skills_group.upgrade(), status.upgrade()) else { return };
            {
                let Ok(mut s) = state.try_borrow_mut() else { return };
                if crate::agent::pi_installer::installation_in_progress() {
                    s.toast(&tr(lang, "settings.ai.updating_agent"));
                    return;
                }
                if s.agent.as_ref().is_some_and(|agent| agent.is_running) {
                    s.toast(&tr(lang, "settings.ai.update_busy"));
                    return;
                }
                if let Some(agent) = s.agent.as_mut() {
                    agent.shutdown();
                    agent.connection = crate::agent::Connection::Connecting;
                }
                s.refresh_assistant();
            }
            agent_group.set_sensitive(false);
            skills_group.set_sensitive(false);
            button.set_label(&tr(lang, "settings.ai.updating_agent"));
            status.set_subtitle(&tr(lang, "settings.ai.updating_agent"));
            let button = button.clone();
            let state = state.clone();
            gtk4::glib::MainContext::default().spawn_local(async move {
                let result = gtk4::gio::spawn_blocking(crate::agent::pi_installer::update).await
                    .unwrap_or_else(|_| Err("Agent update worker failed.".into()));
                let message = match result {
                    Ok(version) => tr1(lang, "settings.ai.agent_updated", &version),
                    Err(error) => error,
                };
                button.set_label(&tr(lang, "settings.ai.update_agent"));
                status.set_subtitle(&message);
                agent_group.set_sensitive(true);
                skills_group.set_sensitive(true);
                if let Ok(mut s) = state.try_borrow_mut() {
                    if let Some(agent) = s.agent.as_mut() { agent.restart(); }
                    s.refresh_assistant();
                    s.toast(&message);
                }
                SKILLS_REBUILD.with(|slot| {
                    if let Some(rebuild) = slot.borrow().as_ref() { rebuild(); }
                });
            });
        });
    }
    window.add(&ai);

    // ── SSH ──
    // `SSHSettingsPane` — the devices "Open via SSH" can open folders on.
    {
        let ssh_page = adw::PreferencesPage::new();
        ssh_page.set_title(&tr(lang, "settings.tab.ssh"));
        ssh_page.set_icon_name(Some("network-workgroup-symbolic"));
        let group = adw::PreferencesGroup::new();
        group.set_title(&tr(lang, "settings.ssh.connections"));
        group.set_description(Some(&tr(lang, "settings.ssh.note")));
        ssh_page.add(&group);

        // The rows rebuild whenever the store's connection list changes —
        // the SwiftUI `ForEach(store.sshConnections)` redraw. Rows added to
        // a PreferencesGroup are reparented into its internal listbox, so
        // removals must go through the exact widgets `add()` saw.
        let rows: Rc<RefCell<Vec<gtk4::Widget>>> = Rc::new(RefCell::new(Vec::new()));
        let rebuild: Rc<RefCell<Option<Rc<dyn Fn()>>>> = Rc::new(RefCell::new(None));
        *rebuild.borrow_mut() = Some({
            let group = group.clone();
            let window = window.clone();
            let state = state.clone();
            let rebuild = rebuild.clone();
            let rows = rows.clone();
            Rc::new(move || {
                // Drop every row the last pass added.
                for row in rows.borrow_mut().drain(..) {
                    group.remove(&row);
                }
                let connections = state.borrow().store.ssh_connections();
                if connections.is_empty() {
                    let empty = adw::ActionRow::new();
                    empty.set_title(&tr(lang, "settings.ssh.empty"));
                    empty.set_selectable(false);
                    group.add(&empty);
                    rows.borrow_mut().push(empty.upcast());
                }
                for connection in &connections {
                    // The check result is a line under the user@host:port
                    // summary like the Swift `checks[id]` text — not a
                    // suffix, so only the Test and trash buttons sit right.
                    // A PreferencesRow keeps the card styling an ActionRow
                    // would lose inside a plain wrapper box.
                    let row = adw::PreferencesRow::new();
                    a11y(&row, "pitex.settings.ssh.connection", "settings.ssh.connections");
                    let content = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
                    content.set_margin_start(12);
                    content.set_margin_end(12);
                    content.set_margin_top(8);
                    content.set_margin_bottom(8);
                    let text_col = gtk4::Box::new(gtk4::Orientation::Vertical, 2);
                    text_col.set_hexpand(true);
                    text_col.set_valign(gtk4::Align::Center);
                    let title = gtk4::Label::new(Some(&connection.name));
                    title.set_xalign(0.0);
                    let subtitle =
                        gtk4::Label::new(Some(&crate::remote::connection_summary(connection)));
                    subtitle.set_xalign(0.0);
                    subtitle.add_css_class("dim-label");
                    let status_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
                    status_box.set_margin_top(2);
                    status_box.set_visible(false);
                    let status_icon = gtk4::Image::new();
                    status_icon.set_pixel_size(14);
                    status_icon.set_valign(gtk4::Align::Start);
                    let status = gtk4::Label::new(None);
                    status.set_xalign(0.0);
                    status.add_css_class("caption");
                    status.set_wrap(true);
                    status.set_lines(3);
                    status.set_ellipsize(gtk4::pango::EllipsizeMode::End);
                    status_box.append(&status_icon);
                    status_box.append(&status);
                    text_col.append(&title);
                    text_col.append(&subtitle);
                    text_col.append(&status_box);
                    content.append(&text_col);
                    let spinner = gtk4::Spinner::new();
                    spinner.set_visible(false);
                    spinner.set_valign(gtk4::Align::Center);
                    content.append(&spinner);
                    let test = gtk4::Button::with_label(&tr(lang, "settings.ssh.test"));
                    test.set_valign(gtk4::Align::Center);
                    {
                        let connection = connection.clone();
                        let spinner = spinner.clone();
                        let button = test.clone();
                        let test_c = test.clone();
                        let status_box_c = status_box.clone();
                        let status_icon_c = status_icon.clone();
                        let status_c = status.clone();
                        button.connect_clicked(move |_| {
                            // `test(_:)` — `check()` then `which("latexmk")`
                            // off the GTK thread; the result lands via
                            // `MainContext::invoke` inside `test_connection`.
                            spinner.set_visible(true);
                            spinner.start();
                            test_c.set_sensitive(false);
                            let status_box = status_box_c.clone();
                            let status_icon = status_icon_c.clone();
                            let status = status_c.clone();
                            let spinner = spinner.clone();
                            let test = test_c.clone();
                            crate::ssh_ui::test_connection(
                                connection.clone(),
                                lang,
                                Rc::new(move |result| {
                                    spinner.stop();
                                    spinner.set_visible(false);
                                    test.set_sensitive(true);
                                    status_box.set_visible(true);
                                    match result {
                                        Ok(text) => {
                                            status_icon.set_icon_name(Some(
                                                "emblem-ok-symbolic",
                                            ));
                                            status_icon.remove_css_class("error");
                                            status_icon.add_css_class("success");
                                            status.set_text(&text);
                                        }
                                        Err(error) => {
                                            status_icon.set_icon_name(Some(
                                                "dialog-error-symbolic",
                                            ));
                                            status_icon.remove_css_class("success");
                                            status_icon.add_css_class("error");
                                            status.set_text(&error);
                                        }
                                    }
                                }),
                            );
                        });
                    }
                    content.append(&test);
                    let remove = gtk4::Button::from_icon_name("user-trash-symbolic");
                    remove.add_css_class("flat");
                    remove.set_valign(gtk4::Align::Center);
                    crate::compat::initial_tooltip(&remove, &tr(lang, "settings.ssh.remove"));
                    {
                        let state = state.clone();
                        let rebuild = rebuild.clone();
                        let id = connection.id.clone();
                        remove.connect_clicked(move |_| {
                            if let Ok(mut s) = state.try_borrow_mut() {
                                let mut connections = s.store.ssh_connections();
                                connections.retain(|c| c.id != id);
                                s.store.set_ssh_connections(&connections);
                            }
                            if let Some(run) = rebuild.borrow().as_ref() {
                                run();
                            }
                        });
                    }
                    content.append(&remove);
                    row.set_child(Some(&content));
                    group.add(&row);
                    rows.borrow_mut().push(row.upcast());
                }
                let add_row = adw::ActionRow::new();
                let add = gtk4::Button::with_label(&tr(lang, "settings.ssh.add"));
                a11y(&add, "pitex.settings.ssh.add", "settings.ssh.add");
                {
                    let window = window.clone();
                    let rebuild = rebuild.clone();
                    add.connect_clicked(move |_| {
                        let parent = window.clone().upcast::<gtk4::Window>();
                        let rebuild = rebuild.clone();
                        crate::ssh_ui::present_add_connection(
                            &parent,
                            lang,
                            Rc::new(move |_| {
                                if let Some(run) = rebuild.borrow().as_ref() {
                                    run();
                                }
                            }),
                        );
                    });
                }
                add_row.add_suffix(&add);
                add_row.set_selectable(false);
                group.add(&add_row);
                rows.borrow_mut().push(add_row.upcast());
            })
        });
        if let Some(run) = rebuild.borrow().as_ref() {
            run();
        }
        window.add(&ssh_page);
    }

    // ── General ──
    // Settings backup (export/import — the file moves between macOS and
    // Linux/Windows unchanged) plus the update controls. Check GitHub
    // Releases on a worker thread; widgets are touched back on the main
    // loop through `SendWrapper` (they are !Send like every GTK object).
    // The pending UpdateInfo is main-thread `Rc` state shared by the
    // check and install buttons.
    let updates_page = adw::PreferencesPage::new();
    updates_page.set_title(&tr(lang, "settings.tab.general"));
    updates_page.set_icon_name(Some("emblem-system-symbolic"));

    let backup_group = adw::PreferencesGroup::new();
    backup_group.set_title(&tr(lang, "settings.general.section"));
    let export_row = adw::ActionRow::new();
    export_row.set_title(&tr(lang, "settings.general.export"));
    export_row.set_activatable(true);
    a11y(&export_row, "pitex.settings.general.export", "settings.general.export");
    {
        let state = state.clone();
        let window = window.clone();
        export_row.connect_activated(move |row| {
            let row = row.clone();
            let window = window.clone().upcast::<gtk4::Window>();
            let state = state.clone();
            crate::compat::save_file(
                Some(&window),
                &tr(lang, "settings.general.export"),
                Some("pitex-settings.json"),
                move |path| {
                    let json = state.borrow().store.prefs().export_backup();
                    let msg = match std::fs::write(&path, serde_json::to_vec_pretty(&json).unwrap_or_default()) {
                        Ok(()) => tr(lang, "settings.general.exported"),
                        Err(e) => e.to_string(),
                    };
                    row.set_subtitle(&msg);
                },
            );
        });
    }
    backup_group.add(&export_row);
    let import_row = adw::ActionRow::new();
    import_row.set_title(&tr(lang, "settings.general.import"));
    import_row.set_activatable(true);
    a11y(&import_row, "pitex.settings.general.import", "settings.general.import");
    {
        let state = state.clone();
        let window = window.clone();
        import_row.connect_activated(move |row| {
            let row = row.clone();
            let window = window.clone().upcast::<gtk4::Window>();
            let state = state.clone();
            crate::compat::pick_settings_file(
                Some(&window),
                &tr(lang, "settings.general.import"),
                move |path| {
                    let msg = match std::fs::read(&path) {
                        Ok(data) => {
                            let mut s = state.borrow_mut();
                            // Bind first — a match scrutinee keeps its
                            // temporaries (the prefs_mut borrow) alive for
                            // the whole match.
                            let result = s.store.prefs_mut().import_backup(&data);
                            match result {
                                Ok(count) => {
                                    s.reload_imported_settings();
                                    tr1(lang, "settings.general.imported", &count.to_string())
                                }
                                Err(e) => e,
                            }
                        }
                        Err(e) => e.to_string(),
                    };
                    row.set_subtitle(&msg);
                },
            );
        });
    }
    backup_group.add(&import_row);
    let backup_note = gtk4::Label::new(Some(&tr(lang, "settings.general.note")));
    backup_note.set_xalign(0.0);
    backup_note.set_wrap(true);
    backup_note.add_css_class("dim-label");
    backup_note.set_margin_start(12);
    backup_note.set_margin_bottom(8);
    backup_group.add(&backup_note);
    updates_page.add(&backup_group);

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
        let install_row = install_row.clone();
        check_row.connect_activated(move |row| {
            row.set_sensitive(false);
            install_row.set_sensitive(false);
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
                        row.set_sensitive(true);
                        if let Some(install) = rows.install.upgrade() {
                            install.set_sensitive(true);
                        }
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
        let check_row = check_row.clone();
        install_row.connect_activated(move |row| {
            let Some(info) = pending_update.borrow().clone() else { return };
            row.set_sensitive(false);
            check_row.set_sensitive(false);
            row.set_subtitle(&tr(lang, "settings.updates.downloading"));
            std::thread::spawn(move || {
                let outcome = crate::update::download(&info)
                    .and_then(|path| crate::update::install(&path, &info));
                gtk4::glib::MainContext::default().invoke(move || {
                    // The installer must be released even if Settings was closed.
                    if matches!(outcome, Ok(crate::update::InstallOutcome::ExitRequested)) {
                        std::process::exit(0);
                    }
                    UPDATE_ROWS.with(|t| {
                        let rows = t.borrow();
                        let Some(rows) = rows.as_ref() else { return };
                        let Some(row) = rows.install.upgrade() else { return };
                        if let Some(check) = rows.check.upgrade() {
                            check.set_sensitive(true);
                        }
                        row.set_sensitive(outcome.is_err());
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
    /// The skills list rebuild closure — the install worker posts back
    /// through `MainContext::invoke` (Send), so the !Send `Rc` lives here
    /// like `UPDATE_ROWS`.
    static SKILLS_REBUILD: RefCell<Option<Rc<dyn Fn()>>> = const { RefCell::new(None) };
}

struct UpdateRows {
    check: gtk4::glib::WeakRef<adw::ActionRow>,
    install: gtk4::glib::WeakRef<adw::ActionRow>,
    pending: Rc<RefCell<Option<crate::update::UpdateInfo>>>,
}
/// One installed skill row — parsed from `SKILL.md` frontmatter.
struct SkillRow {
    name: String,
    description: String,
    builtin: bool,
    path: PathBuf,
}

/// Skill directory names shipped inside the app bundle — the same set
/// `pi_installer::install_bundled_skills` copies (no remove button).
const BUNDLED_SKILLS: [&str; 5] = [
    "humanizer",
    "latex-compile",
    "latex-doctor",
    "scispace",
    "texlive-runtime-installer",
];

/// Subdirs of `pi_paths::skills_directory()` containing a `SKILL.md`.
fn installed_skills() -> Vec<SkillRow> {
    let mut skills = Vec::new();
    let Ok(entries) = std::fs::read_dir(crate::agent::pi_paths::skills_directory()) else {
        return skills;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let dir_name = entry.file_name().to_string_lossy().into_owned();
        let skill_md = path.join("SKILL.md");
        if !skill_md.is_file() {
            continue;
        }
        let (name, description) = skill_frontmatter(&skill_md);
        skills.push(SkillRow {
            name: name.unwrap_or(dir_name.clone()),
            description,
            builtin: BUNDLED_SKILLS.contains(&dir_name.as_str()),
            path,
        });
    }
    skills.sort_by(|a, b| a.name.cmp(&b.name));
    skills
}

/// `name:`/`description:` from a `SKILL.md` YAML frontmatter block — a
/// simple line parse (no serde_yaml): `description: |` block scalars take
/// their first indented line.
fn skill_frontmatter(path: &PathBuf) -> (Option<String>, String) {
    let Ok(text) = std::fs::read_to_string(path) else {
        return (None, String::new());
    };
    let mut name = None;
    let mut description = String::new();
    let mut in_frontmatter = false;
    let mut block_scalar = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed == "---" {
            if in_frontmatter {
                break;
            }
            in_frontmatter = true;
            continue;
        }
        if !in_frontmatter {
            continue;
        }
        if block_scalar {
            if line.starts_with(' ') || line.starts_with('\t') {
                if !trimmed.is_empty() {
                    description = trimmed.to_string();
                }
            }
            block_scalar = false;
            continue;
        }
        if let Some(value) = trimmed.strip_prefix("name:") {
            name = Some(value.trim().trim_matches('"').trim_matches('\'').to_string());
        } else if let Some(value) = trimmed.strip_prefix("description:") {
            let value = value.trim();
            if value == "|" || value == ">" {
                block_scalar = true;
            } else {
                description = value.trim_matches('"').trim_matches('\'').to_string();
            }
        }
    }
    (name, description)
}

#[cfg(test)]
mod skill_frontmatter_tests {
    use super::skill_frontmatter;

    #[test]
    fn parses_inline_and_block_scalar_descriptions() {
        let dir = std::env::temp_dir().join(format!("pitex-skill-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("SKILL.md");
        std::fs::write(
            &path,
            "---\nname: humanizer\ndescription: |\n  Rewrite AI-sounding text.\nlicense: MIT\n---\n",
        )
        .unwrap();
        let (name, description) = skill_frontmatter(&path);
        assert_eq!(name.as_deref(), Some("humanizer"));
        assert_eq!(description, "Rewrite AI-sounding text.");
        std::fs::write(
            &path,
            "---\nname: \"latex-doctor\"\ndescription: Detect TeX tools.\n---\n",
        )
        .unwrap();
        let (name, description) = skill_frontmatter(&path);
        assert_eq!(name.as_deref(), Some("latex-doctor"));
        assert_eq!(description, "Detect TeX tools.");
        let _ = std::fs::remove_dir_all(&dir);
    }
}

/// The resumable conversations of the current project, newest first.
/// Takes the list by value: `refresh_assistant` calls this while it holds
/// the app state, so only the row click touches `STATE`.
pub(crate) fn show_session_history(
    anchor: &gtk4::Button,
    sessions: Vec<crate::agent::AgentSessionSummary>,
    lang: &'static str,
) {
    let popover = gtk4::Popover::new();
    let list = gtk4::ListBox::new();
    list.set_selection_mode(gtk4::SelectionMode::None);
    if sessions.is_empty() {
        let empty = gtk4::Label::new(Some(&tr(lang, "assistant.history_empty")));
        empty.add_css_class("dim-label");
        empty.set_margin_top(16);
        empty.set_margin_bottom(16);
        empty.set_margin_start(16);
        empty.set_margin_end(16);
        list.append(&empty);
    }
    for session in &sessions {
        let row = gtk4::Box::new(gtk4::Orientation::Vertical, 2);
        row.set_margin_top(4);
        row.set_margin_bottom(4);
        row.set_margin_start(6);
        row.set_margin_end(6);
        let title = gtk4::Label::new(Some(&session.title));
        title.set_xalign(0.0);
        title.set_wrap(true);
        title.set_lines(2);
        title.set_ellipsize(gtk4::pango::EllipsizeMode::End);
        title.set_max_width_chars(48);
        row.append(&title);
        let seconds = session
            .modified
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        let date = gtk4::glib::DateTime::from_unix_local(seconds)
            .and_then(|d| d.format("%Y-%m-%d %H:%M"))
            .map(|s| s.to_string())
            .unwrap_or_default();
        let when = gtk4::Label::new(Some(&date));
        when.set_xalign(0.0);
        when.add_css_class("dim-label");
        when.add_css_class("caption");
        row.append(&when);
        list.append(&row);
    }
    {
        let popover = popover.clone();
        list.connect_row_activated(move |_, row| {
            let Some(session) = sessions.get(row.index().max(0) as usize) else { return };
            let path = session.path.clone();
            popover.popdown();
            crate::app_ui::STATE.with(|s| {
                if let Some(state) = s.borrow().as_ref() {
                    if let Ok(mut st) = state.try_borrow_mut() {
                        if let Some(agent) = st.agent.as_mut() {
                            agent.resume(&path);
                        }
                        st.refresh_assistant();
                    }
                }
            });
        });
    }
    let scroll = gtk4::ScrolledWindow::new();
    scroll.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);
    scroll.set_max_content_height(320);
    scroll.set_propagate_natural_height(true);
    scroll.set_min_content_width(380);
    scroll.set_child(Some(&list));
    popover.set_child(Some(&scroll));
    popover.set_parent(anchor);
    popover.connect_closed(|p| p.unparent());
    popover.popup();
}
