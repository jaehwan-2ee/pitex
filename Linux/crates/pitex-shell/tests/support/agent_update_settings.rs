use super::*;
use crate::agent::{pi_installer, pi_paths, AgentCoordinator, PiAgentProcess, PiRPCCommand, PiToolchain};

fn find(root: &gtk4::Widget, name: &str) -> Option<gtk4::Widget> {
    if root.widget_name() == name { return Some(root.clone()); }
    let mut child = root.first_child();
    while let Some(widget) = child {
        if let Some(found) = find(&widget, name) { return Some(found); }
        child = widget.next_sibling();
    }
    None
}

fn wait_for_agent(state: &Rc<RefCell<AppState>>) {
    let deadline = std::time::Instant::now() + Duration::from_secs(30);
    while std::time::Instant::now() < deadline {
        let mut state = state.borrow_mut();
        let agent = state.agent.as_mut().unwrap();
        agent.poll_toolchain();
        while let Some(event) = agent.poll_event() {
            if event.response_command().as_deref() == Some("get_state")
                && event.object.get("success").and_then(|v| v.as_bool()) == Some(true) { return; }
        }
        drop(state);
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!("The agent did not reconnect and answer get_state");
}

/// CI runs this on real GTK 4.6/4.14 and Windows UCRT64, in a fresh process.
#[test]
#[ignore = "requires GTK display, Bun, Node/npm and registry access; run alone"]
fn settings_button_updates_latest_and_restores_failed_update() {
    let _guard = crate::TEST_ENV_LOCK.lock().unwrap();
    adw::init().unwrap();
    let root = std::env::temp_dir().join(format!("pitex agent update's {}", crate::model::uuid_v4()));
    let agent_dir = root.join("pi");
    std::fs::create_dir_all(&agent_dir).unwrap();
    std::env::set_var("PI_CODING_AGENT_DIR", &agent_dir);
    std::env::set_var("XDG_CONFIG_HOME", root.join("config"));
    std::env::set_var("XDG_CACHE_HOME", root.join("cache"));
    let credentials = br#"{"fixture":{"type":"api_key","key":"test-only"}}"#;
    let settings = br#"{"fixture":true}"#;
    let models = br#"{"providers":{}}"#;
    std::fs::write(pi_paths::auth_file(), credentials).unwrap();
    std::fs::write(pi_paths::settings_file(), settings).unwrap();
    std::fs::write(pi_paths::models_file(), models).unwrap();
    let tools = PiToolchain::discover(std::env::vars().collect(), &root, &PiToolchain::SYSTEM_DIRECTORIES);
    assert!(tools.bun.is_some() && tools.node.is_some() && tools.npm.is_some());
    let (exe, args) = tools.npm_command(&["view".into(), format!("{}@latest", pi_installer::PACKAGE_NAME), "version".into()]).unwrap();
    let output = std::process::Command::new(exe).args(args).envs(&tools.environment).output().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let latest = String::from_utf8(output.stdout).unwrap().trim().to_string();
    pi_installer::install_with_tools(&tools).unwrap();
    assert_eq!(pi_installer::installed_version().as_deref(), Some(pi_installer::DESIRED_VERSION));
    let (tx, _rx) = std::sync::mpsc::channel();
    let state = Rc::new(RefCell::new(AppState::new(SettingsStore::new(Preferences::default()), tx, "test".into())));
    let parent = gtk4::Window::new();
    crate::panes::show_settings(&state, &parent);
    let window = gtk4::Window::list_toplevels().into_iter()
        .find_map(|widget| widget.downcast::<adw::PreferencesWindow>().ok()).unwrap();
    let button = find(window.upcast_ref(), "pitex.settings.ai.updateAgent").unwrap()
        .downcast::<gtk4::Button>().unwrap();
    let status = find(window.upcast_ref(), "pitex.settings.ai.runtimeStatus").unwrap()
        .downcast::<adw::ActionRow>().unwrap();
    assert_eq!(button.label().as_deref(), Some("Agent update"));
    let provider_row = button.ancestor(adw::ActionRow::static_type()).unwrap()
        .downcast::<adw::ActionRow>().unwrap();
    assert_eq!(provider_row.title(), "Custom provider");
    // The handler must decline updates while a response is active.
    let mut agent = AgentCoordinator::new(&state.borrow().store);
    let project = root.clone();
    agent.context_provider = Some(Box::new(move || crate::agent::AgentContextSnapshot {
        project_root: Some(project.clone()), ..Default::default()
    }));
    agent.prepare();
    state.borrow_mut().agent = Some(agent);
    wait_for_agent(&state);
    state.borrow_mut().agent.as_mut().unwrap().is_running = true;
    button.emit_clicked();
    assert!(button.is_sensitive());
    assert_eq!(pi_installer::installed_version().as_deref(), Some(pi_installer::DESIRED_VERSION));
    state.borrow_mut().agent.as_mut().unwrap().is_running = false;
    button.emit_clicked();
    assert!(!button.is_sensitive());
    let context = glib::MainContext::default();
    let deadline = std::time::Instant::now() + Duration::from_secs(240);
    while !button.is_sensitive() && std::time::Instant::now() < deadline {
        while context.pending() { context.iteration(false); }
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(button.is_sensitive(), "Update blocked the UI or did not finish");
    assert_eq!(pi_installer::installed_version().as_deref(), Some(latest.as_str()), "{}", status.subtitle().unwrap_or_default());
    assert!(status.subtitle().unwrap().contains(&latest));
    wait_for_agent(&state);
    state.borrow_mut().agent.as_mut().unwrap().shutdown();
    println!("PASS actual Settings button updated via Bun to {latest}");
    window.close();
    parent.close();
    std::thread::sleep(Duration::from_millis(300));
    // Exercise Node/npm too, including the Windows npm.cmd installation route.
    let mut node_tools = tools.clone();
    node_tools.bun = None;
    pi_installer::install_with_tools(&node_tools).unwrap();
    assert_eq!(pi_installer::update_with_tools(&node_tools).unwrap(), latest);
    let mut broken = tools.clone();
    // Node cannot act as Bun: fail after creating a new runtime directory.
    broken.bun = tools.node.clone();
    broken.node = None;
    broken.npm = None;
    assert!(pi_installer::update_with_tools(&broken).is_err());
    assert_eq!(pi_installer::installed_version().as_deref(), Some(latest.as_str()));
    assert_eq!(std::fs::read(pi_paths::auth_file()).unwrap(), credentials);
    assert_eq!(std::fs::read(pi_paths::settings_file()).unwrap(), settings);
    assert_eq!(std::fs::read(pi_paths::models_file()).unwrap(), models);
    assert!(!std::fs::read_dir(&root).unwrap().flatten().any(|e| e.file_name().to_string_lossy().starts_with("pi-runtime-backup-")));
    for runtime in [&tools, &node_tools] {
        let (exe, args) = runtime.launch(&pi_paths::runtime_executable(), &["--mode".into(), "rpc".into(), "--no-session".into()]).unwrap();
        let process = PiAgentProcess::start(&exe, &root, &args, &runtime.environment).unwrap();
        process.send(&PiRPCCommand::GetState, Some("update-check")).unwrap();
        let received = (|| {
            let deadline = std::time::Instant::now() + Duration::from_secs(30);
            while std::time::Instant::now() < deadline {
                if let Ok(event) = process.events.lock().unwrap().recv_timeout(Duration::from_secs(1)) {
                    if event.string("id").as_deref() == Some("update-check") {
                        return event.object.get("success").and_then(|v| v.as_bool()) == Some(true);
                    }
                }
            }
            false
        })();
        process.terminate();
        assert!(received, "Restored runtime RPC failed: {}", process.stderr_text());
    }
    std::thread::sleep(Duration::from_millis(300));
    std::fs::remove_dir_all(&root).unwrap();
    println!("PASS Node/npm latest update, rollback, credential preservation and Bun/Node live RPC");
}
