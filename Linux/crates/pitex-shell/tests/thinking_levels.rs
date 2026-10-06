//! The reasoning picker must reflect what the *current model* supports: pi
//! reports the set via `get_available_thinking_levels`, and that set can omit
//! `minimal`/`xhigh` or include `max` depending on the model. Model-setting
//! operations are serialized by request IDs — replies echoing a different id
//! are stale and must not restore old choices.

use pitex_shell::agent::{AgentCoordinator, PiRPCEvent};
use pitex_shell::settings::{Preferences, SettingsStore};
use serde_json::{json, Value};

fn coordinator() -> AgentCoordinator {
    let store = SettingsStore::new(Preferences::standard());
    AgentCoordinator::new(&store)
}

fn response(command: &str, id: Option<&str>, data: Value) -> PiRPCEvent {
    let mut obj = json!({
        "type": "response",
        "success": true,
        "command": command,
        "data": data,
    });
    if let Some(id) = id {
        obj["id"] = json!(id);
    }
    PiRPCEvent::new(obj.as_object().unwrap().clone()).unwrap()
}

/// Marks a model-settings refresh in flight the way `send_model_settings`
/// does (no real process in tests).
fn begin_refresh(agent: &mut AgentCoordinator, id: &str) {
    agent.model_settings_request_id = Some(id.to_string());
    agent.is_updating_model_settings = true;
    agent.thinking_levels.clear();
}

/// Drive one full state → capabilities chain with `id`.
fn complete_refresh(agent: &mut AgentCoordinator, id: &str, levels: &[&str]) {
    agent.handle(&response(
        "get_state",
        Some(id),
        json!({"model": {"provider": "anthropic", "id": "claude"}, "thinkingLevel": levels[0]}),
    ));
    agent.handle(&response(
        "get_available_thinking_levels",
        Some(id),
        json!({"levels": levels}),
    ));
}

#[test]
fn levels_stay_empty_until_reported() {
    let agent = coordinator();
    assert!(agent.thinking_levels.is_empty());
    assert!(!agent.is_updating_model_settings);
}

#[test]
fn levels_response_replaces_choices() {
    let mut agent = coordinator();
    begin_refresh(&mut agent, "r1");
    assert!(agent.thinking_levels.is_empty());
    complete_refresh(&mut agent, "r1", &["off", "low", "medium", "high"]);
    assert_eq!(
        agent.thinking_levels,
        vec!["off", "low", "medium", "high"]
    );
    assert!(!agent.is_updating_model_settings);
    assert!(agent.model_settings_request_id.is_none());
}

#[test]
fn levels_response_can_expose_max() {
    let mut agent = coordinator();
    begin_refresh(&mut agent, "r2");
    complete_refresh(
        &mut agent,
        "r2",
        &["off", "minimal", "low", "medium", "high", "xhigh", "max"],
    );
    assert!(agent.thinking_levels.contains(&"max".to_string()));
}

#[test]
fn non_reasoning_model_reports_only_off() {
    let mut agent = coordinator();
    begin_refresh(&mut agent, "r3");
    complete_refresh(&mut agent, "r3", &["off"]);
    assert_eq!(agent.thinking_levels, vec!["off"]);
}

#[test]
fn stale_response_with_wrong_id_is_ignored() {
    let mut agent = coordinator();
    begin_refresh(&mut agent, "current");
    // A reply from an earlier refresh must not land.
    agent.handle(&response(
        "get_available_thinking_levels",
        Some("old-request"),
        json!({"levels": ["off", "low"]}),
    ));
    assert!(agent.thinking_levels.is_empty());
    assert!(agent.is_updating_model_settings);
    // … and neither must a response with no id at all.
    agent.handle(&response(
        "get_available_thinking_levels",
        None,
        json!({"levels": ["off", "low"]}),
    ));
    assert!(agent.thinking_levels.is_empty());
    assert!(agent.is_updating_model_settings);
    // The current request still completes normally.
    complete_refresh(&mut agent, "current", &["off", "high"]);
    assert_eq!(agent.thinking_levels, vec!["off", "high"]);
}

#[test]
fn invalid_level_report_sets_status_and_keeps_picker_empty() {
    let mut agent = coordinator();
    begin_refresh(&mut agent, "r4");
    // `levels` must contain the effective thinkingLevel — here get_state
    // reported "off" but the capability list omits it.
    agent.handle(&response(
        "get_state",
        Some("r4"),
        json!({"model": {"provider": "p", "id": "m"}, "thinkingLevel": "off"}),
    ));
    agent.handle(&response(
        "get_available_thinking_levels",
        Some("r4"),
        json!({"levels": ["low", "high"]}),
    ));
    assert!(agent.thinking_levels.is_empty());
    assert!(!agent.is_updating_model_settings);
    assert!(agent.status_message.is_some());
}

#[test]
fn selection_rejects_level_outside_available_set() {
    let mut agent = coordinator();
    agent.select_thinking_level("max");
    assert_eq!(agent.thinking_level, "off");
    assert!(!agent.is_updating_model_settings);
}
