//! Port of `SSHViews.swift` — the three SSH sheets as transient modal
//! windows: "Add SSH Connection" (config hosts or a manual form), "Open
//! via SSH" (device + remote folder) and `RemoteFolderBrowser`. Every
//! ssh call runs on a worker thread; results land through `idle_add_once`.

use gtk4::prelude::*;
use gtk4::glib;
use libadwaita as adw;
use adw::prelude::*;
use remote_core::{
    RemoteDirectoryListing, RemoteMirror, RemoteProject, SshClient, SshConfigParser,
    SshConnection, SshHostEntry,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::app_ui::{a11y, main_window, STATE};
use crate::l10n::{tr, tr1};

/// Shared sheet chrome — the SwiftUI `SheetHeader` (title left, close
/// button right) above the body column.
fn sheet(
    parent: &gtk4::Window,
    lang: &str,
    title_key: &str,
    width: i32,
) -> (adw::Window, gtk4::Box) {
    let window = adw::Window::new();
    window.set_transient_for(Some(parent));
    window.set_modal(true);
    window.set_default_size(width, -1);
    window.set_title(Some(&tr(lang, title_key)));
    let body = gtk4::Box::new(gtk4::Orientation::Vertical, 14);
    body.set_margin_start(20);
    body.set_margin_end(20);
    body.set_margin_top(20);
    body.set_margin_bottom(20);
    let head = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    let title = gtk4::Label::new(Some(&tr(lang, title_key)));
    title.add_css_class("title-2");
    title.set_xalign(0.0);
    title.set_hexpand(true);
    head.append(&title);
    let close = gtk4::Button::from_icon_name("window-close-symbolic");
    close.add_css_class("flat");
    crate::compat::initial_tooltip(&close, &tr(lang, "ssh.close"));
    {
        let window = window.clone();
        close.connect_clicked(move |_| window.close());
    }
    head.append(&close);
    body.append(&head);
    window.set_content(Some(&body));
    (window, body)
}

/// `store.sshConnections` — the saved devices, read through the shared
/// app state (the same store the Settings page writes).
fn saved_connections() -> Vec<SshConnection> {
    STATE.with(|s| {
        s.borrow()
            .as_ref()
            .map(|st| st.borrow().store.ssh_connections())
            .unwrap_or_default()
    })
}

fn save_connections(connections: &[SshConnection]) {
    STATE.with(|s| {
        if let Some(st) = s.borrow().as_ref() {
            if let Ok(mut st) = st.try_borrow_mut() {
                st.store.set_ssh_connections(connections);
            }
        }
    });
}

/// A `Rc<RefCell<Option<Rc<dyn Fn()>>>>` lets a rebuild closure run
/// itself from inside the widgets it creates — SwiftUI's @State redraw.
type RebuildSlot = Rc<RefCell<Option<Rc<dyn Fn()>>>>;

fn rebuild(slot: &RebuildSlot) {
    if let Some(run) = slot.borrow().as_ref() {
        run();
    }
}

thread_local! {
    /// Worker results cross back through `MainContext::invoke` (Send
    /// only), so the !Send handlers live here keyed by request id — the
    /// same pattern `UPDATE_ROWS`/`SKILLS_REBUILD` use in panes.rs.
    static SSH_CHECK_HANDLERS: RefCell<std::collections::HashMap<u64, Rc<dyn Fn(Result<String, String>)>>> =
        RefCell::new(std::collections::HashMap::new());
    static SSH_BROWSERS: RefCell<std::collections::HashMap<u64, BrowserHandles>> =
        RefCell::new(std::collections::HashMap::new());
}

static SSH_REQUEST_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

/// The browser's main-thread half — invoked when a `go` result lands and
/// the generation still matches.
#[derive(Clone)]
struct BrowserHandles {
    generation: Rc<Cell<u64>>,
    apply: Rc<dyn Fn(Result<RemoteDirectoryListing, String>)>,
}

fn next_request_id() -> u64 {
    SSH_REQUEST_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

/// `test(_:)` — `check()` then `which("latexmk")` on a worker; the
/// formatted result (or the error text) lands on the main thread.
pub fn test_connection(
    connection: SshConnection,
    lang: &'static str,
    apply: Rc<dyn Fn(Result<String, String>)>,
) {
    let id = next_request_id();
    SSH_CHECK_HANDLERS.with(|m| {
        m.borrow_mut().insert(id, apply);
    });
    std::thread::spawn(move || {
        let client = SshClient::new(connection);
        let result = client
            .check()
            .map(|home| match client.which("latexmk") {
                Ok(Some(path)) => crate::l10n::tr1(lang, "settings.ssh.test_tex", &path),
                _ => crate::l10n::tr1(lang, "settings.ssh.test_no_tex", &home),
            })
            .map_err(|e| e.to_string());
        glib::MainContext::default().invoke(move || {
            SSH_CHECK_HANDLERS.with(|m| {
                if let Some(apply) = m.borrow_mut().remove(&id) {
                    apply(result);
                }
            });
        });
    });
}

// ─── AddSSHConnectionSheet ───────────────────────────────────────────────────

/// "Add SSH Connection": hosts from `~/.ssh/config`, or entered by hand.
/// `on_added` runs after the connection is stored — Open via SSH selects
/// the new device.
pub fn present_add_connection(
    parent: &gtk4::Window,
    lang: &'static str,
    on_added: Rc<dyn Fn(SshConnection)>,
) {
    let (window, body) = sheet(parent, lang, "ssh.add.title", 540);

    let manual = Rc::new(Cell::new(false));
    let discovered: Rc<RefCell<Vec<SshHostEntry>>> = Rc::new(RefCell::new(
        SshConfigParser::load_hosts(&SshConfigParser::default_config_path()),
    ));

    // Host list — `List(available, selection:)`; rows carry the alias.
    let hosts = gtk4::ListBox::new();
    hosts.set_selection_mode(gtk4::SelectionMode::Single);
    hosts.add_css_class("boxed-list");
    a11y(&hosts, "pitex.ssh.add.hosts", "ssh.add.from_config");
    let list_scroll = gtk4::ScrolledWindow::new();
    list_scroll.set_min_content_height(260);
    list_scroll.set_max_content_height(260);
    list_scroll.set_propagate_natural_height(true);
    list_scroll.set_child(Some(&hosts));

    // Manual form — the `manualForm` fields.
    let form = gtk4::Box::new(gtk4::Orientation::Vertical, 8);
    form.set_margin_top(8);
    form.set_margin_bottom(8);
    let labeled = |title: &str, entry: &gtk4::Entry| -> gtk4::Box {
        let row = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
        let label = gtk4::Label::new(Some(title));
        label.set_xalign(0.0);
        label.set_width_chars(12);
        row.append(&label);
        row.append(entry);
        row
    };
    let name_entry = gtk4::Entry::new();
    name_entry.set_placeholder_text(Some("Mac mini"));
    let host_entry = gtk4::Entry::new();
    host_entry.set_placeholder_text(Some("mac-mini.local"));
    let user_entry = gtk4::Entry::new();
    user_entry.set_placeholder_text(Some(&tr(lang, "ssh.field.optional")));
    let port_entry = gtk4::Entry::new();
    port_entry.set_placeholder_text(Some("22"));
    let key_entry = gtk4::Entry::new();
    key_entry.set_placeholder_text(Some(&tr(lang, "ssh.field.optional")));
    form.append(&labeled(&tr(lang, "ssh.field.name"), &name_entry));
    form.append(&labeled(&tr(lang, "ssh.field.host"), &host_entry));
    form.append(&labeled(&tr(lang, "ssh.field.user"), &user_entry));
    form.append(&labeled(&tr(lang, "ssh.field.port"), &port_entry));
    let key_row = labeled(&tr(lang, "ssh.field.key"), &key_entry);
    let choose = gtk4::Button::with_label(&tr(lang, "ssh.field.choose"));
    {
        let key_entry = key_entry.clone();
        let parent = window.clone().upcast::<gtk4::Window>();
        choose.connect_clicked(move |_| {
            let key_entry = key_entry.clone();
            crate::compat::pick_files(Some(&parent), "Private Key", move |paths| {
                if let Some(path) = paths.first() {
                    key_entry.set_text(&path.to_string_lossy());
                }
            });
        });
    }
    key_row.append(&choose);
    form.append(&key_row);
    let form_scroll = gtk4::ScrolledWindow::new();
    form_scroll.set_min_content_height(260);
    form_scroll.set_max_content_height(260);
    form_scroll.set_propagate_natural_height(true);
    form_scroll.set_child(Some(&form));

    let stack = gtk4::Stack::new();
    stack.add_named(&list_scroll, Some("hosts"));
    stack.add_named(&form_scroll, Some("manual"));
    stack.set_visible_child_name("hosts");
    body.append(&stack);

    // Inline validation line — the sheet's `problem` text.
    let problem = gtk4::Label::new(None);
    problem.set_xalign(0.0);
    problem.add_css_class("error");
    problem.add_css_class("caption");
    problem.set_wrap(true);
    problem.set_visible(false);
    body.append(&problem);

    // Footer: manual toggle, rescan (hosts mode only), Add.
    let footer = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    let toggle = gtk4::Button::with_label(&tr(lang, "ssh.add.manual"));
    a11y(&toggle, "pitex.ssh.add.manual", "ssh.add.manual");
    footer.append(&toggle);
    let rescan = gtk4::Button::from_icon_name("view-refresh-symbolic");
    rescan.add_css_class("flat");
    crate::compat::initial_tooltip(&rescan, &tr(lang, "ssh.add.rescan"));
    footer.append(&rescan);
    let spacer = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
    spacer.set_hexpand(true);
    footer.append(&spacer);
    let add = gtk4::Button::with_label(&tr(lang, "ssh.add.confirm"));
    add.add_css_class("suggested-action");
    a11y(&add, "pitex.ssh.add.confirm", "ssh.add.confirm");
    footer.append(&add);
    body.append(&footer);

    // `available` — config hosts not saved yet.
    let available = {
        let discovered = discovered.clone();
        move || -> Vec<SshHostEntry> {
            let saved: std::collections::HashSet<String> = saved_connections()
                .iter()
                .map(|c| c.destination.clone())
                .collect();
            discovered
                .borrow()
                .iter()
                .filter(|e| !saved.contains(&e.alias))
                .cloned()
                .collect()
        }
    };

    // `manualConnection` — trims every field like the Swift form.
    let manual_connection = {
        let name_entry = name_entry.clone();
        let host_entry = host_entry.clone();
        let user_entry = user_entry.clone();
        let port_entry = port_entry.clone();
        let key_entry = key_entry.clone();
        move || -> SshConnection {
            let host = host_entry.text().trim().to_string();
            let name = name_entry.text().trim().to_string();
            let user = user_entry.text().trim().to_string();
            let key = key_entry.text().trim().to_string();
            let port = port_entry.text().trim().to_string();
            SshConnection::new(
                if name.is_empty() { &host } else { &name },
                &host,
                if user.is_empty() { None } else { Some(user) },
                port.parse::<i64>().ok(),
                if key.is_empty() { None } else { Some(key) },
            )
        }
    };

    // Rebuild the host rows from `available`; `reload()` preselects the
    // first row when nothing is selected.
    let rebuild_hosts = {
        let hosts = hosts.clone();
        let available = available.clone();
        move || {
            while let Some(child) = hosts.first_child() {
                hosts.remove(&child);
            }
            let entries = available();
            if entries.is_empty() {
                let empty = gtk4::Label::new(Some(&tr(lang, "ssh.add.none_found")));
                empty.add_css_class("dim-label");
                empty.set_margin_top(30);
                empty.set_margin_bottom(30);
                let row = gtk4::ListBoxRow::new();
                row.set_selectable(false);
                row.set_child(Some(&empty));
                hosts.append(&row);
            } else {
                for entry in &entries {
                    let content = gtk4::Box::new(gtk4::Orientation::Horizontal, 10);
                    content.set_margin_start(10);
                    content.set_margin_end(10);
                    content.set_margin_top(6);
                    content.set_margin_bottom(6);
                    content.append(&gtk4::Image::from_icon_name("computer-symbolic"));
                    let text = gtk4::Box::new(gtk4::Orientation::Vertical, 2);
                    let alias = gtk4::Label::new(Some(&entry.alias));
                    alias.set_xalign(0.0);
                    let summary = gtk4::Label::new(Some(&entry.summary()));
                    summary.set_xalign(0.0);
                    summary.add_css_class("caption");
                    summary.add_css_class("dim-label");
                    text.append(&alias);
                    text.append(&summary);
                    content.append(&text);
                    let row = gtk4::ListBoxRow::new();
                    row.set_child(Some(&content));
                    row.set_widget_name(&entry.alias);
                    hosts.append(&row);
                }
                if hosts.selected_row().is_none() {
                    if let Some(row) = hosts.row_at_index(0) {
                        hosts.select_row(Some(&row));
                    }
                }
            }
        }
    };

    // `pending`/`problem` — Add sensitivity and the inline error.
    let update = {
        let manual = manual.clone();
        let stack = stack.clone();
        let hosts = hosts.clone();
        let add = add.clone();
        let problem = problem.clone();
        let toggle = toggle.clone();
        let rescan = rescan.clone();
        let host_entry = host_entry.clone();
        let port_entry = port_entry.clone();
        let available = available.clone();
        let manual_connection = manual_connection.clone();
        move || {
            let is_manual = manual.get();
            stack.set_visible_child_name(if is_manual { "manual" } else { "hosts" });
            toggle.set_label(&tr(
                lang,
                if is_manual { "ssh.add.from_config" } else { "ssh.add.manual" },
            ));
            rescan.set_visible(!is_manual);
            let ready;
            let mut message: Option<String> = None;
            if is_manual {
                let connection = manual_connection();
                let port_text = port_entry.text().trim().to_string();
                if !host_entry.text().is_empty() {
                    if !port_text.is_empty() && port_text.parse::<i64>().is_err() {
                        message = Some(tr(lang, "ssh.field.port_invalid"));
                    } else {
                        message = connection.validation_error().map(str::to_string);
                    }
                }
                ready = connection.validation_error().is_none()
                    && (port_text.is_empty() || port_text.parse::<i64>().is_ok());
            } else {
                ready = hosts
                    .selected_row()
                    .and_then(|row| {
                        let alias = row.widget_name().to_string();
                        available().into_iter().find(|e| e.alias == alias)
                    })
                    .is_some();
            }
            add.set_sensitive(ready);
            problem.set_visible(message.is_some());
            if let Some(message) = &message {
                problem.set_text(message);
            }
        }
    };

    rebuild_hosts();
    update();

    hosts.connect_row_selected({
        let update = update.clone();
        move |_, _| update()
    });
    for entry in [&name_entry, &host_entry, &user_entry, &port_entry, &key_entry] {
        entry.connect_changed({
            let update = update.clone();
            move |_| update()
        });
    }
    toggle.connect_clicked({
        let manual = manual.clone();
        let update = update.clone();
        move |_| {
            manual.set(!manual.get());
            update();
        }
    });
    rescan.connect_clicked({
        let discovered = discovered.clone();
        let update = update.clone();
        move |_| {
            *discovered.borrow_mut() =
                SshConfigParser::load_hosts(&SshConfigParser::default_config_path());
            rebuild_hosts();
            update();
        }
    });
    add.connect_clicked({
        let window = window.clone();
        let hosts = hosts.clone();
        let manual = manual.clone();
        let available = available.clone();
        let manual_connection = manual_connection.clone();
        move |_| {
            // `pending` — the manual connection, or the selected host.
            let connection = if manual.get() {
                let connection = manual_connection();
                if connection.validation_error().is_some() {
                    return;
                }
                connection
            } else {
                let Some(row) = hosts.selected_row() else { return };
                let alias = row.widget_name().to_string();
                let Some(entry) = available().into_iter().find(|e| e.alias == alias)
                else {
                    return;
                };
                SshConnection::from_config_host(&entry)
            };
            let mut connections = saved_connections();
            connections.push(connection.clone());
            save_connections(&connections);
            on_added(connection);
            window.close();
        }
    });

    window.present();
}

// ─── RemoteFolderBrowser ─────────────────────────────────────────────────────

/// `deletingLastPathComponent` for a POSIX remote path — "/" stays "/".
fn remote_parent(path: &str) -> String {
    let trimmed = path.trim_end_matches('/');
    match trimmed.rfind('/') {
        Some(0) | None => "/".to_string(),
        Some(slash) => trimmed[..slash].to_string(),
    }
}

/// `appendingPathComponent` for a POSIX remote path.
fn remote_join(base: &str, name: &str) -> String {
    if base.ends_with('/') {
        format!("{base}{name}")
    } else {
        format!("{base}/{name}")
    }
}

/// Folder picker for a remote device: path field with an up button, the
/// current folder's subfolders (click to enter), TeX/Markdown files shown
/// dimmed for orientation, and "Use Folder" for the folder being shown.
fn present_remote_browser(
    parent: &gtk4::Window,
    lang: &'static str,
    connection: SshConnection,
    initial: String,
    on_choose: Rc<dyn Fn(String)>,
) {
    const HINT_EXTENSIONS: &[&str] = &["tex", "bib", "md", "markdown", "sty", "cls"];

    let (window, body) = sheet(parent, lang, "ssh.browse.title", 540);

    let listing: Rc<RefCell<Option<RemoteDirectoryListing>>> = Rc::new(RefCell::new(None));
    let loading = Rc::new(Cell::new(false));
    let generation = Rc::new(Cell::new(0u64));

    // Path row: up button + editable path field (Enter navigates).
    let path_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    let up = gtk4::Button::from_icon_name("go-up-symbolic");
    up.add_css_class("flat");
    crate::compat::initial_tooltip(&up, &tr(lang, "ssh.browse.up"));
    a11y(&up, "pitex.ssh.browse.up", "ssh.browse.up");
    path_row.append(&up);
    let path_entry = gtk4::Entry::new();
    path_entry.set_hexpand(true);
    a11y(&path_entry, "pitex.ssh.browse.path", "ssh.browse.title");
    path_row.append(&path_entry);
    body.append(&path_row);

    let use_current = gtk4::Button::with_label(&tr(lang, "ssh.browse.use_current"));
    use_current.add_css_class("flat");
    use_current.set_halign(gtk4::Align::Start);
    body.append(&use_current);

    // The ZStack: listing under a loading spinner / error label overlay.
    let overlay = gtk4::Overlay::new();
    overlay.set_size_request(-1, 300);
    let list = gtk4::ListBox::new();
    list.set_selection_mode(gtk4::SelectionMode::None);
    list.add_css_class("boxed-list");
    a11y(&list, "pitex.ssh.browse.list", "ssh.browse.title");
    let list_scroll = gtk4::ScrolledWindow::new();
    list_scroll.set_child(Some(&list));
    overlay.set_child(Some(&list_scroll));
    let spinner = gtk4::Spinner::new();
    spinner.set_visible(false);
    overlay.add_overlay(&spinner);
    let error_label = gtk4::Label::new(None);
    error_label.add_css_class("error");
    error_label.set_wrap(true);
    error_label.set_visible(false);
    overlay.add_overlay(&error_label);
    body.append(&overlay);

    let footer = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    let spacer = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
    spacer.set_hexpand(true);
    footer.append(&spacer);
    let cancel = gtk4::Button::with_label(&tr(lang, "ssh.open.cancel"));
    {
        let window = window.clone();
        cancel.connect_clicked(move |_| window.close());
    }
    footer.append(&cancel);
    let use_folder = gtk4::Button::with_label(&tr(lang, "ssh.browse.use_folder"));
    use_folder.add_css_class("suggested-action");
    a11y(&use_folder, "pitex.ssh.browse.useFolder", "ssh.browse.use_folder");
    footer.append(&use_folder);
    body.append(&footer);

    // Folder rows navigate through `go`; the slot exists before `go` so
    // row closures can reach it.
    let go_slot: Rc<RefCell<Option<Rc<dyn Fn(String)>>>> = Rc::new(RefCell::new(None));

    // Rerender rows + sensitivity from the `listing`/`loading` cells.
    let render = {
        let list = list.clone();
        let listing = listing.clone();
        let loading = loading.clone();
        let spinner = spinner.clone();
        let error_label = error_label.clone();
        let up = up.clone();
        let use_folder = use_folder.clone();
        let use_current = use_current.clone();
        let go_slot = go_slot.clone();
        move |error: Option<String>| {
            let is_loading = loading.get();
            spinner.set_visible(is_loading);
            if is_loading {
                spinner.start();
            } else {
                spinner.stop();
            }
            list.set_opacity(if is_loading { 0.4 } else { 1.0 });
            error_label.set_visible(error.is_some() && !is_loading);
            if let Some(error) = &error {
                error_label.set_text(error);
            }
            let current = listing.borrow().clone();
            while let Some(child) = list.first_child() {
                list.remove(&child);
            }
            if let Some(current) = &current {
                for folder in &current.folders {
                    let row = gtk4::ListBoxRow::new();
                    let inner = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
                    inner.set_margin_start(6);
                    inner.set_margin_top(4);
                    inner.set_margin_bottom(4);
                    inner.append(&gtk4::Image::from_icon_name("folder-symbolic"));
                    let name = gtk4::Label::new(Some(folder));
                    name.set_xalign(0.0);
                    inner.append(&name);
                    row.set_child(Some(&inner));
                    row.set_activatable(true);
                    // Folder activation navigates into it.
                    let path = remote_join(&current.path, folder);
                    let go_slot = go_slot.clone();
                    row.connect_activate(move |_| {
                        if let Some(go) = go_slot.borrow().as_ref() {
                            go(path.clone());
                        }
                    });
                    list.append(&row);
                }
                for file in &current.files {
                    let ext = file
                        .rsplit('.')
                        .next()
                        .map(|e| e.to_lowercase())
                        .unwrap_or_default();
                    if !HINT_EXTENSIONS.contains(&ext.as_str()) {
                        continue;
                    }
                    let row = gtk4::ListBoxRow::new();
                    row.set_activatable(false);
                    row.set_selectable(false);
                    let inner = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
                    inner.set_margin_start(6);
                    inner.set_margin_top(4);
                    inner.set_margin_bottom(4);
                    inner.append(&gtk4::Image::from_icon_name(
                        "x-office-document-symbolic",
                    ));
                    let name = gtk4::Label::new(Some(file));
                    name.set_xalign(0.0);
                    inner.append(&name);
                    inner.add_css_class("dim-label");
                    row.set_child(Some(&inner));
                    list.append(&row);
                }
                if current.folders.is_empty() && current.files.is_empty() {
                    let row = gtk4::ListBoxRow::new();
                    row.set_activatable(false);
                    row.set_selectable(false);
                    let empty = gtk4::Label::new(Some(&tr(lang, "ssh.browse.empty")));
                    empty.add_css_class("dim-label");
                    empty.set_margin_top(20);
                    empty.set_margin_bottom(20);
                    row.set_child(Some(&empty));
                    list.append(&row);
                }
            }
            up.set_sensitive(
                current
                    .as_ref()
                    .map(|l| l.path != "/")
                    .unwrap_or(false),
            );
            use_folder.set_sensitive(current.is_some() && !is_loading);
            use_current.set_sensitive(current.is_some());
        }
    };

    // `go(path)` — bump the generation, load on a worker, land the
    // listing through `MainContext::invoke` (Send-only); stale results
    // drop like `task?.cancel()` makes them. The !Send apply side lives
    // in `SSH_BROWSERS` so only the id crosses the thread boundary.
    let browser_id = next_request_id();
    {
        let apply: Rc<dyn Fn(Result<RemoteDirectoryListing, String>)> = {
            let listing = listing.clone();
            let loading = loading.clone();
            let path_entry = path_entry.clone();
            let render = render.clone();
            Rc::new(move |result| {
                loading.set(false);
                match result {
                    Ok(result) => {
                        path_entry.set_text(&result.path);
                        *listing.borrow_mut() = Some(result);
                        render(None);
                    }
                    Err(error) => {
                        // Keep the previous listing's path.
                        if let Some(current) = listing.borrow().as_ref() {
                            path_entry.set_text(&current.path);
                        }
                        render(Some(error));
                    }
                }
            })
        };
        SSH_BROWSERS.with(|m| {
            m.borrow_mut().insert(
                browser_id,
                BrowserHandles {
                    generation: generation.clone(),
                    apply,
                },
            );
        });
    }
    let go: Rc<dyn Fn(String)> = {
        let loading = loading.clone();
        let generation = generation.clone();
        let render = render.clone();
        Rc::new(move |path: String| {
            generation.set(generation.get() + 1);
            let expected = generation.get();
            loading.set(true);
            render(None);
            let client = SshClient::new(connection.clone());
            std::thread::spawn(move || {
                let result = client
                    .list_directory(&path)
                    .map_err(|e| e.to_string());
                glib::MainContext::default().invoke(move || {
                    SSH_BROWSERS.with(|m| {
                        let handles = m.borrow().get(&browser_id).cloned();
                        if let Some(handles) = handles {
                            if handles.generation.get() == expected {
                                (handles.apply)(result);
                            }
                        }
                    });
                });
            });
        })
    };
    *go_slot.borrow_mut() = Some(go.clone());

    up.connect_clicked({
        let listing = listing.clone();
        let go = go.clone();
        move |_| {
            if let Some(current) = listing.borrow().as_ref() {
                go(remote_parent(&current.path));
            }
        }
    });
    path_entry.connect_activate({
        let go = go.clone();
        move |entry| go(entry.text().to_string())
    });
    let choose = {
        let listing = listing.clone();
        let window = window.clone();
        let on_choose = on_choose.clone();
        move || {
            if let Some(current) = listing.borrow().as_ref() {
                on_choose(current.path.clone());
                window.close();
            }
        }
    };
    use_current.connect_clicked({
        let choose = choose.clone();
        move |_| choose()
    });
    use_folder.connect_clicked(move |_| choose());
    // `onDisappear { task?.cancel() }` — a closing sheet drops the
    // result and frees the handler slot.
    window.connect_close_request(move |_| {
        generation.set(generation.get() + 1);
        SSH_BROWSERS.with(|m| m.borrow_mut().remove(&browser_id));
        glib::Propagation::Proceed
    });

    go(initial);
    window.present();
}

// ─── OpenViaSSHSheet ─────────────────────────────────────────────────────────

/// File → "Open via SSH…": pick a device, then a folder on it.
pub fn present_open_via_ssh(lang: &'static str) {
    let Some(parent) = main_window() else { return };
    let (window, body) = sheet(&parent, lang, "ssh.open.title", 560);

    let connections = saved_connections();
    let selected: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));
    let folder: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));

    let heading = gtk4::Label::new(Some(&tr(lang, "ssh.open.folder")));
    heading.add_css_class("heading");
    heading.set_xalign(0.0);
    body.append(&heading);

    let card_inner = gtk4::Box::new(gtk4::Orientation::Vertical, 12);
    card_inner.add_css_class("card");
    card_inner.set_size_request(-1, 150);
    card_inner.set_margin_start(16);
    card_inner.set_margin_end(16);
    card_inner.set_margin_top(16);
    card_inner.set_margin_bottom(16);
    card_inner.set_valign(gtk4::Align::Center);
    card_inner.set_halign(gtk4::Align::Fill);
    body.append(&card_inner);

    let note = gtk4::Label::new(Some(&tr(lang, "ssh.open.note")));
    note.add_css_class("caption");
    note.add_css_class("dim-label");
    note.set_xalign(0.0);
    note.set_wrap(true);
    body.append(&note);

    let footer = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    let spacer = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
    spacer.set_hexpand(true);
    footer.append(&spacer);
    let cancel = gtk4::Button::with_label(&tr(lang, "ssh.open.cancel"));
    {
        let window = window.clone();
        cancel.connect_clicked(move |_| window.close());
    }
    footer.append(&cancel);
    let open = gtk4::Button::with_label(&tr(lang, "ssh.open.confirm"));
    open.add_css_class("suggested-action");
    a11y(&open, "pitex.ssh.open.confirm", "ssh.open.confirm");
    footer.append(&open);
    body.append(&footer);

    // The card contents rebuild whenever the device or folder changes —
    // the Swift sheet's `connection`/`folder` @State.
    let rebuild_slot: RebuildSlot = Rc::new(RefCell::new(None));
    *rebuild_slot.borrow_mut() = Some({
        let card_inner = card_inner.clone();
        let window = window.clone();
        let selected = selected.clone();
        let folder = folder.clone();
        let open = open.clone();
        let rebuild_slot = rebuild_slot.clone();
        Rc::new(move || {
            while let Some(child) = card_inner.first_child() {
                card_inner.remove(&child);
            }
            let connections = saved_connections();
            let connection = selected
                .borrow()
                .as_ref()
                .and_then(|id| connections.iter().find(|c| &c.id == id))
                .or_else(|| connections.first())
                .cloned();
            let Some(connection) = connection else {
                let empty = gtk4::Label::new(Some(&tr(lang, "ssh.open.no_connections")));
                empty.add_css_class("dim-label");
                card_inner.append(&empty);
                let add_btn = gtk4::Button::with_label(&tr(lang, "ssh.open.add_connection"));
                {
                    let window = window.clone();
                    let selected = selected.clone();
                    let rebuild_slot = rebuild_slot.clone();
                    add_btn.connect_clicked(move |_| {
                        let parent = window.clone().upcast::<gtk4::Window>();
                        let selected = selected.clone();
                        let rebuild_slot = rebuild_slot.clone();
                        present_add_connection(
                            &parent,
                            lang,
                            Rc::new(move |added| {
                                *selected.borrow_mut() = Some(added.id.clone());
                                rebuild(&rebuild_slot);
                            }),
                        );
                    });
                }
                card_inner.append(&add_btn);
                open.set_sensitive(false);
                return;
            };

            // Device menu — `Menu { devices; Divider; Add } label:
            // on_device` as a MenuButton whose label carries the name.
            let device_button = gtk4::MenuButton::new();
            device_button.set_label(&tr1(lang, "ssh.open.on_device", &connection.name));
            device_button.set_halign(gtk4::Align::Start);
            a11y(&device_button, "pitex.ssh.open.device", "ssh.open.title");
            let popover = gtk4::Popover::new();
            let popover_box = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
            for candidate in &connections {
                let item = gtk4::Button::with_label(&candidate.name);
                item.set_has_frame(false);
                {
                    let popover = popover.clone();
                    let selected = selected.clone();
                    let folder = folder.clone();
                    let rebuild_slot = rebuild_slot.clone();
                    let id = candidate.id.clone();
                    item.connect_clicked(move |_| {
                        popover.popdown();
                        *selected.borrow_mut() = Some(id.clone());
                        *folder.borrow_mut() = None;
                        rebuild(&rebuild_slot);
                    });
                }
                popover_box.append(&item);
            }
            popover_box.append(&gtk4::Separator::new(gtk4::Orientation::Horizontal));
            let add_item = gtk4::Button::with_label(&tr(lang, "ssh.open.add_connection"));
            add_item.set_has_frame(false);
            {
                let popover = popover.clone();
                let window = window.clone();
                let selected = selected.clone();
                let folder = folder.clone();
                let rebuild_slot = rebuild_slot.clone();
                add_item.connect_clicked(move |_| {
                    popover.popdown();
                    let parent = window.clone().upcast::<gtk4::Window>();
                    let selected = selected.clone();
                    let folder = folder.clone();
                    let rebuild_slot = rebuild_slot.clone();
                    present_add_connection(
                        &parent,
                        lang,
                        Rc::new(move |added| {
                            *selected.borrow_mut() = Some(added.id.clone());
                            *folder.borrow_mut() = None;
                            rebuild(&rebuild_slot);
                        }),
                    );
                });
            }
            popover_box.append(&add_item);
            popover.set_child(Some(&popover_box));
            device_button.set_popover(Some(&popover));
            card_inner.append(&device_button);

            // Folder row: the chosen path + Change, or the Choose button.
            let chosen = folder.borrow().clone();
            if let Some(path) = chosen {
                let row = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
                row.append(&gtk4::Image::from_icon_name("folder-symbolic"));
                let label = gtk4::Label::new(Some(&path));
                label.set_ellipsize(gtk4::pango::EllipsizeMode::Middle);
                label.set_max_width_chars(30);
                label.set_xalign(0.0);
                label.set_hexpand(true);
                row.append(&label);
                let change = gtk4::Button::with_label(&tr(lang, "ssh.open.change"));
                {
                    let window = window.clone();
                    let folder = folder.clone();
                    let rebuild_slot = rebuild_slot.clone();
                    let initial = path.clone();
                    let connection = connection.clone();
                    change.connect_clicked(move |_| {
                        let parent = window.clone().upcast::<gtk4::Window>();
                        let folder = folder.clone();
                        let rebuild_slot = rebuild_slot.clone();
                        present_remote_browser(
                            &parent,
                            lang,
                            connection.clone(),
                            initial.clone(),
                            Rc::new(move |chosen| {
                                *folder.borrow_mut() = Some(chosen);
                                rebuild(&rebuild_slot);
                            }),
                        );
                    });
                }
                row.append(&change);
                card_inner.append(&row);
            } else {
                let choose_btn = gtk4::Button::with_label(&tr(lang, "ssh.open.choose"));
                a11y(&choose_btn, "pitex.ssh.open.choose", "ssh.open.choose");
                {
                    let window = window.clone();
                    let folder = folder.clone();
                    let rebuild_slot = rebuild_slot.clone();
                    let connection = connection.clone();
                    choose_btn.connect_clicked(move |_| {
                        let parent = window.clone().upcast::<gtk4::Window>();
                        let folder = folder.clone();
                        let rebuild_slot = rebuild_slot.clone();
                        present_remote_browser(
                            &parent,
                            lang,
                            connection.clone(),
                            String::new(),
                            Rc::new(move |chosen| {
                                *folder.borrow_mut() = Some(chosen);
                                rebuild(&rebuild_slot);
                            }),
                        );
                    });
                }
                card_inner.append(&choose_btn);
            }
            open.set_sensitive(folder.borrow().is_some());
        })
    });

    // `.onAppear` — preselect `lastSSHConnectionID ?? first`.
    let last = STATE.with(|s| {
        s.borrow()
            .as_ref()
            .and_then(|st| st.borrow().store.last_ssh_connection())
    });
    *selected.borrow_mut() =
        last.filter(|id| connections.iter().any(|c| &c.id == id))
            .or_else(|| connections.first().map(|c| c.id.clone()));
    rebuild(&rebuild_slot);

    open.connect_clicked({
        let window = window.clone();
        let selected = selected.clone();
        let folder = folder.clone();
        move |_| {
            let connections = saved_connections();
            let Some(connection) = selected
                .borrow()
                .as_ref()
                .and_then(|id| connections.iter().find(|c| &c.id == id))
                .or_else(|| connections.first())
                .cloned()
            else {
                return;
            };
            let Some(remote_root) = folder.borrow().clone() else { return };
            // `openRemote` — remember the device, prepare the mirror and
            // route it like any folder open.
            STATE.with(|s| {
                if let Some(st) = s.borrow().as_ref() {
                    if let Ok(mut st) = st.try_borrow_mut() {
                        st.store.set_last_ssh_connection(&connection.id);
                    }
                }
            });
            window.close();
            match RemoteMirror::prepare(
                RemoteProject::new(connection, remote_root),
                &RemoteMirror::default_store(),
            ) {
                Ok(mirror) => {
                    let root = mirror.root();
                    STATE.with(|s| {
                        if let Some(st) = s.borrow().as_ref() {
                            if let Ok(mut st) = st.try_borrow_mut() {
                                st.open_routed(root);
                            }
                        }
                    });
                }
                Err(error) => {
                    STATE.with(|s| {
                        if let Some(st) = s.borrow().as_ref() {
                            if let Ok(mut st) = st.try_borrow_mut() {
                                st.model.open_failed(error.to_string());
                                st.refresh_phase();
                            }
                        }
                    });
                }
            }
        }
    });

    window.present();
}
