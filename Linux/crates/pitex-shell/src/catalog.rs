//! Assistant model catalog. One conversation session stays alive. Other
//! backends contribute rows only after a probe, and the menu is a pure
//! projection of those reports. Linux and Windows share this module.

use crate::settings::visible_agent_providers;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CatalogModel {
    pub id: String,
    pub provider: String,
    pub name: String,
    pub context_window: Option<u64>,
}

impl CatalogModel {
    pub(crate) fn picker_title(&self) -> String {
        if self.name == self.id {
            self.id.clone()
        } else {
            format!("{} ({})", self.name, self.id)
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct CatalogRowId {
    pub backend: String,
    pub model_id: String,
    pub provider: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum SlotState {
    Pending,
    Ready(Vec<CatalogModel>),
    Unavailable,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Slot {
    pub backend: String,
    pub state: SlotState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CatalogFacts<'a> {
    pub slots: &'a [Slot],
    pub active_backend: &'a str,
    pub current: Option<CatalogModel>,
    pub generation: u64,
    pub accepts_picks: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CatalogRow {
    pub id: CatalogRowId,
    pub model: CatalogModel,
    pub label: String,
    pub section: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CatalogMenu {
    pub rows: Vec<CatalogRow>,
    pub selection: Option<CatalogRowId>,
    pub accepts_picks: bool,
    pub generation: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum SelectionIntent {
    Row(CatalogRowId),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum PickEffect {
    None,
    SetModel(CatalogModel),
    Switch {
        backend: String,
        model: Option<CatalogModel>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SessionSnapshot {
    pub is_running: bool,
    pub backend: String,
    pub current: Option<CatalogModel>,
}

pub(crate) fn gtk_slots() -> Vec<Slot> {
    visible_agent_providers()
        .map(|choice| Slot {
            backend: choice.id.to_string(),
            state: SlotState::Pending,
        })
        .collect()
}

pub(crate) fn note(slots: &mut [Slot], backend: &str, state: SlotState) {
    if let Some(slot) = slots.iter_mut().find(|slot| slot.backend == backend) {
        slot.state = state;
    }
}

pub(crate) fn rearm_unavailable(slots: &mut [Slot]) -> bool {
    let mut changed = false;
    for slot in slots.iter_mut() {
        if matches!(slot.state, SlotState::Unavailable) {
            slot.state = SlotState::Pending;
            changed = true;
        }
    }
    changed
}

pub(crate) fn menu(facts: &CatalogFacts<'_>, title: impl Fn(&str) -> String) -> CatalogMenu {
    let mut rows = Vec::new();
    for slot in facts.slots {
        let SlotState::Ready(models) = &slot.state else {
            continue;
        };
        if models.is_empty() {
            continue;
        }
        let section = title(&slot.backend);
        let providers: std::collections::HashSet<&str> =
            models.iter().map(|model| model.provider.as_str()).collect();
        let disambiguate = providers.len() > 1;
        for model in models {
            let mut label = format!("{section} · {}", model.picker_title());
            if disambiguate {
                label = format!("{label} · {}", model.provider);
            }
            rows.push(CatalogRow {
                id: CatalogRowId {
                    backend: slot.backend.clone(),
                    model_id: model.id.clone(),
                    provider: model.provider.clone(),
                },
                model: model.clone(),
                label,
                section: section.clone(),
            });
        }
    }
    let selection = rows.iter().find_map(|row| {
        let current = facts.current.as_ref()?;
        if row.id.backend == facts.active_backend
            && row.model.id == current.id
            && row.model.provider == current.provider
        {
            Some(row.id.clone())
        } else {
            None
        }
    });
    CatalogMenu {
        accepts_picks: facts.accepts_picks && !rows.is_empty(),
        generation: facts.generation,
        rows,
        selection,
    }
}

pub(crate) fn effect(
    intent: &SelectionIntent,
    facts: &CatalogFacts<'_>,
    shown_generation: u64,
) -> PickEffect {
    if shown_generation != facts.generation {
        return PickEffect::None;
    }
    match intent {
        SelectionIntent::Row(id) => {
            if !facts.accepts_picks {
                return PickEffect::None;
            }
            let Some(model) = model_in_slots(facts.slots, id) else {
                return PickEffect::None;
            };
            if id.backend == facts.active_backend
                && facts.current.as_ref().is_some_and(|current| {
                    current.id == model.id && current.provider == model.provider
                })
            {
                return PickEffect::None;
            }
            if id.backend == facts.active_backend {
                PickEffect::SetModel(model)
            } else {
                PickEffect::Switch {
                    backend: id.backend.clone(),
                    model: Some(model),
                }
            }
        }
    }
}

pub(crate) fn session_after(effect: &PickEffect, session: &SessionSnapshot) -> SessionSnapshot {
    match effect {
        PickEffect::None => session.clone(),
        PickEffect::SetModel(model) => SessionSnapshot {
            is_running: session.is_running,
            backend: session.backend.clone(),
            current: Some(model.clone()),
        },
        PickEffect::Switch { backend, model } => SessionSnapshot {
            is_running: false,
            backend: backend.clone(),
            current: model.clone(),
        },
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct OfferedBackend {
    pub id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DisabledBackends {
    ids: Vec<String>,
}

impl DisabledBackends {
    pub(crate) fn parse(stored: Option<&[String]>, offered: &[OfferedBackend]) -> Self {
        let Some(stored) = stored else {
            return Self { ids: Vec::new() };
        };
        let mut ids = Vec::new();
        for id in stored {
            if offered.iter().any(|backend| backend.id == *id) && !ids.iter().any(|kept| kept == id) {
                ids.push(id.clone());
            }
        }
        ids.sort_by_key(|id| {
            offered
                .iter()
                .position(|backend| backend.id == *id)
                .unwrap_or(usize::MAX)
        });
        Self { ids }
    }

    pub(crate) fn toggled(&self, id: &str, disabled: bool, offered: &[OfferedBackend]) -> Self {
        if !offered.iter().any(|backend| backend.id == id) {
            return self.clone();
        }
        let mut ids = self.ids.clone();
        if disabled {
            if !ids.iter().any(|kept| kept == id) {
                ids.push(id.to_string());
            }
        } else {
            ids.retain(|kept| kept != id);
        }
        ids.sort_by_key(|id| {
            offered
                .iter()
                .position(|backend| backend.id == *id)
                .unwrap_or(usize::MAX)
        });
        Self { ids }
    }

    pub(crate) fn contains(&self, id: &str) -> bool {
        self.ids.iter().any(|kept| kept == id)
    }

    pub(crate) fn ids(&self) -> &[String] {
        &self.ids
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum SessionTarget {
    Active(String),
    Unsupported,
    Idle,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum SessionPlan {
    Keep,
    Switch(String),
    Stop,
}

pub(crate) struct EnablementFacts<'a> {
    pub offered: &'a [OfferedBackend],
    pub stored_disabled: Option<&'a [String]>,
    pub normalized_provider: &'a str,
    pub normalized_is_on_device: bool,
    pub session_open: bool,
    pub previous_slots: &'a [Slot],
    pub seeded: &'a [(&'a str, Vec<CatalogModel>)],
}

pub(crate) struct EnablementPlan {
    pub enabled: Vec<String>,
    pub target: SessionTarget,
    pub session: SessionPlan,
    pub slots: Vec<Slot>,
}

pub(crate) fn gtk_offered() -> Vec<OfferedBackend> {
    visible_agent_providers()
        .map(|choice| OfferedBackend {
            id: choice.id.to_string(),
        })
        .collect()
}

pub(crate) fn enablement_plan(facts: &EnablementFacts<'_>) -> EnablementPlan {
    let disabled = DisabledBackends::parse(facts.stored_disabled, facts.offered);
    let enabled: Vec<String> = facts
        .offered
        .iter()
        .filter(|backend| !disabled.contains(&backend.id))
        .map(|backend| backend.id.clone())
        .collect();
    let target = if enabled.iter().any(|id| id == facts.normalized_provider) {
        SessionTarget::Active(facts.normalized_provider.to_string())
    } else if facts.normalized_is_on_device
        && !facts
            .offered
            .iter()
            .any(|backend| backend.id == facts.normalized_provider)
    {
        SessionTarget::Unsupported
    } else if let Some(first) = enabled.first() {
        SessionTarget::Active(first.clone())
    } else {
        SessionTarget::Idle
    };
    let session = match &target {
        SessionTarget::Active(id) if id == facts.normalized_provider => SessionPlan::Keep,
        SessionTarget::Active(id) => SessionPlan::Switch(id.clone()),
        SessionTarget::Unsupported => SessionPlan::Keep,
        SessionTarget::Idle if facts.session_open => SessionPlan::Stop,
        SessionTarget::Idle => SessionPlan::Keep,
    };
    let slots = project_slots(&enabled, facts.previous_slots, facts.seeded);
    EnablementPlan {
        enabled,
        target,
        session,
        slots,
    }
}

pub(crate) fn project_slots(
    enabled: &[String],
    previous: &[Slot],
    seeded: &[(&str, Vec<CatalogModel>)],
) -> Vec<Slot> {
    enabled
        .iter()
        .map(|id| {
            if let Some((_, models)) = seeded.iter().find(|(backend, _)| *backend == id) {
                if !models.is_empty() {
                    return Slot {
                        backend: id.clone(),
                        state: SlotState::Ready(models.clone()),
                    };
                }
            }
            if let Some(kept) = previous.iter().find(|slot| slot.backend == *id) {
                return kept.clone();
            }
            Slot {
                backend: id.clone(),
                state: SlotState::Pending,
            }
        })
        .collect()
}

fn model_in_slots(slots: &[Slot], id: &CatalogRowId) -> Option<CatalogModel> {
    let slot = slots.iter().find(|slot| slot.backend == id.backend)?;
    let SlotState::Ready(models) = &slot.state else {
        return None;
    };
    models
        .iter()
        .find(|model| model.id == id.model_id && model.provider == id.provider)
        .cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model(id: &str, provider: &str, name: &str) -> CatalogModel {
        CatalogModel {
            id: id.into(),
            provider: provider.into(),
            name: name.into(),
            context_window: None,
        }
    }

    fn ready(backend: &str, models: Vec<CatalogModel>) -> Slot {
        Slot {
            backend: backend.into(),
            state: SlotState::Ready(models),
        }
    }

    fn titles(backend: &str) -> String {
        match backend {
            "pi" => "pi (provider sign-in)".into(),
            "claude" => "Claude Code (your login)".into(),
            "apple-fm" => "Apple Foundation Models (on this Mac)".into(),
            other => other.into(),
        }
    }

    fn facts<'a>(
        slots: &'a [Slot],
        active: &'a str,
        current: Option<CatalogModel>,
    ) -> CatalogFacts<'a> {
        CatalogFacts {
            slots,
            active_backend: active,
            current,
            generation: 3,
            accepts_picks: true,
        }
    }

    #[test]
    fn menu_merges_models_from_each_ready_backend() {
        let slots = vec![
            ready("pi", vec![model("sonnet", "anthropic", "Sonnet")]),
            ready(
                "claude",
                vec![model("claude-sonnet", "claude", "Claude Sonnet")],
            ),
        ];
        let current = model("sonnet", "anthropic", "Sonnet");
        let menu = menu(&facts(&slots, "pi", Some(current)), titles);
        assert_eq!(
            menu.rows
                .iter()
                .map(|row| row.label.as_str())
                .collect::<Vec<_>>(),
            [
                "pi (provider sign-in) · Sonnet (sonnet)",
                "Claude Code (your login) · Claude Sonnet (claude-sonnet)",
            ]
        );
        assert_eq!(menu.rows[0].section, "pi (provider sign-in)");
        assert_eq!(menu.rows[1].section, "Claude Code (your login)");
        assert_eq!(
            menu.selection.as_ref().map(|id| id.backend.as_str()),
            Some("pi")
        );
        assert!(menu.accepts_picks);
    }

    #[test]
    fn menu_suffixes_the_rpc_provider_when_one_backend_lists_several() {
        let slots = vec![ready(
            "pi",
            vec![
                model("sonnet", "anthropic", "Sonnet"),
                model("gpt", "openai", "gpt"),
            ],
        )];
        let menu = menu(&facts(&slots, "pi", None), titles);
        assert_eq!(
            menu.rows
                .iter()
                .map(|row| row.label.as_str())
                .collect::<Vec<_>>(),
            [
                "pi (provider sign-in) · Sonnet (sonnet) · anthropic",
                "pi (provider sign-in) · gpt · openai",
            ]
        );
    }

    #[test]
    fn unavailable_backend_is_omitted() {
        let slots = vec![
            ready("pi", vec![model("sonnet", "anthropic", "Sonnet")]),
            Slot {
                backend: "claude".into(),
                state: SlotState::Unavailable,
            },
            Slot {
                backend: "apple-fm".into(),
                state: SlotState::Pending,
            },
            ready("extra", vec![]),
        ];
        let menu = menu(&facts(&slots, "pi", None), titles);
        assert_eq!(menu.rows.len(), 1);
        assert_eq!(menu.rows[0].id.backend, "pi");
        assert!(!menu.rows.iter().any(|row| row.id.backend == "claude"));
        assert!(!menu.rows.iter().any(|row| row.id.backend == "apple-fm"));
        assert!(!menu.rows.iter().any(|row| row.id.backend == "extra"));
    }

    #[test]
    fn gtk_targets_omit_hidden_clis_and_apple_fm() {
        let ids: Vec<_> = gtk_slots().into_iter().map(|slot| slot.backend).collect();
        assert_eq!(ids, ["pi", "claude"]);
    }

    #[test]
    fn same_backend_pick_sets_only_the_model() {
        let slots = vec![ready(
            "pi",
            vec![
                model("sonnet", "anthropic", "Sonnet"),
                model("opus", "anthropic", "Opus"),
            ],
        )];
        let current = model("sonnet", "anthropic", "Sonnet");
        let built = menu(&facts(&slots, "pi", Some(current.clone())), titles);
        let session = SessionSnapshot {
            is_running: true,
            backend: "pi".into(),
            current: Some(current.clone()),
        };
        let effect = effect(
            &SelectionIntent::Row(built.rows[1].id.clone()),
            &facts(&slots, "pi", session.current.clone()),
            built.generation,
        );
        let after = session_after(&effect, &session);
        assert!(matches!(effect, PickEffect::SetModel(ref model) if model.id == "opus"));
        assert!(after.is_running);
        assert_eq!(after.backend, "pi");
        assert_eq!(after.current.unwrap().id, "opus");
    }

    #[test]
    fn repeated_pick_of_the_current_model_does_nothing() {
        let slots = vec![ready("pi", vec![model("sonnet", "anthropic", "Sonnet")])];
        let current = model("sonnet", "anthropic", "Sonnet");
        let built = menu(&facts(&slots, "pi", Some(current.clone())), titles);
        let effect = effect(
            &SelectionIntent::Row(built.selection.unwrap()),
            &facts(&slots, "pi", Some(current)),
            3,
        );
        assert_eq!(effect, PickEffect::None);
    }

    #[test]
    fn cross_backend_pick_switches_backend_and_model() {
        let slots = vec![
            ready("pi", vec![model("sonnet", "anthropic", "Sonnet")]),
            ready(
                "claude",
                vec![model("claude-sonnet", "claude", "Claude Sonnet")],
            ),
        ];
        let current = model("sonnet", "anthropic", "Sonnet");
        let built = menu(&facts(&slots, "pi", Some(current.clone())), titles);
        let session = SessionSnapshot {
            is_running: false,
            backend: "pi".into(),
            current: Some(current.clone()),
        };
        let effect = effect(
            &SelectionIntent::Row(built.rows[1].id.clone()),
            &facts(&slots, "pi", Some(current)),
            built.generation,
        );
        let after = session_after(&effect, &session);
        match effect {
            PickEffect::Switch { backend, model } => {
                assert_eq!(backend, "claude");
                assert_eq!(model.unwrap().id, "claude-sonnet");
            }
            other => panic!("expected a switch, got {other:?}"),
        }
        assert_eq!(after.backend, "claude");
        assert!(!after.is_running);
        assert_eq!(after.current.unwrap().id, "claude-sonnet");
    }

    #[test]
    fn cross_backend_pick_mid_turn_clears_the_running_turn() {
        let slots = vec![
            ready("pi", vec![model("sonnet", "anthropic", "Sonnet")]),
            ready(
                "claude",
                vec![model("claude-sonnet", "claude", "Claude Sonnet")],
            ),
        ];
        let current = model("sonnet", "anthropic", "Sonnet");
        let built = menu(&facts(&slots, "pi", Some(current.clone())), titles);
        let session = SessionSnapshot {
            is_running: true,
            backend: "pi".into(),
            current: Some(current.clone()),
        };
        let effect = effect(
            &SelectionIntent::Row(built.rows[1].id.clone()),
            &facts(&slots, "pi", Some(current)),
            3,
        );
        let after = session_after(&effect, &session);
        assert!(session.is_running);
        assert!(!after.is_running);
        assert_eq!(after.backend, "claude");
    }

    #[test]
    fn stale_generation_and_closed_picker_do_not_apply() {
        let slots = vec![
            ready("pi", vec![model("sonnet", "anthropic", "Sonnet")]),
            ready(
                "claude",
                vec![model("claude-sonnet", "claude", "Claude Sonnet")],
            ),
        ];
        let built = menu(&facts(&slots, "pi", None), titles);
        let stale = effect(
            &SelectionIntent::Row(built.rows[1].id.clone()),
            &facts(&slots, "pi", None),
            2,
        );
        assert_eq!(stale, PickEffect::None);
        let mut closed = facts(&slots, "pi", None);
        closed.accepts_picks = false;
        let blocked = effect(&SelectionIntent::Row(built.rows[1].id.clone()), &closed, 3);
        assert_eq!(blocked, PickEffect::None);
    }

    fn offered(ids: &[&str]) -> Vec<OfferedBackend> {
        ids.iter()
            .map(|id| OfferedBackend {
                id: (*id).to_string(),
            })
            .collect()
    }

    fn plan(
        stored: Option<&[String]>,
        provider: &str,
        on_device: bool,
        open: bool,
        ids: &[&str],
    ) -> EnablementPlan {
        let offered = offered(ids);
        enablement_plan(&EnablementFacts {
            offered: &offered,
            stored_disabled: stored,
            normalized_provider: provider,
            normalized_is_on_device: on_device,
            session_open: open,
            previous_slots: &[],
            seeded: &[],
        })
    }

    #[test]
    fn a_missing_denylist_enables_every_offered_backend_and_keeps_the_session() {
        let decided = plan(None, "claude", false, true, &["pi", "claude"]);
        assert_eq!(decided.enabled, vec!["pi".to_string(), "claude".to_string()]);
        assert_eq!(decided.session, SessionPlan::Keep);
        assert_eq!(decided.target, SessionTarget::Active("claude".into()));
    }

    #[test]
    fn unchecking_the_live_backend_switches_to_the_first_remaining_one() {
        let stored = vec!["pi".to_string()];
        let running = plan(Some(&stored), "pi", false, true, &["pi", "claude"]);
        assert_eq!(running.session, SessionPlan::Switch("claude".into()));
        let idle = plan(Some(&stored), "pi", false, false, &["pi", "claude"]);
        assert_eq!(idle.session, SessionPlan::Switch("claude".into()));
    }

    #[test]
    fn unchecking_every_backend_stops_an_open_session_and_is_allowed() {
        let stored = vec!["pi".to_string(), "claude".to_string()];
        let running = plan(Some(&stored), "pi", false, true, &["pi", "claude"]);
        assert_eq!(running.target, SessionTarget::Idle);
        assert_eq!(running.session, SessionPlan::Stop);
        assert!(running.enabled.is_empty());
        let closed = plan(Some(&stored), "pi", false, false, &["pi", "claude"]);
        assert_eq!(closed.session, SessionPlan::Keep);
    }

    #[test]
    fn a_disabled_backend_is_omitted_from_the_slots() {
        let decided = plan(Some(&["claude".into()]), "pi", false, false, &["pi", "claude"]);
        assert_eq!(
            decided
                .slots
                .iter()
                .map(|slot| slot.backend.as_str())
                .collect::<Vec<_>>(),
            vec!["pi"]
        );
    }

    #[test]
    fn hidden_cli_ids_are_dropped_from_the_denylist_and_do_not_move_the_session() {
        let offered = offered(&["pi", "claude"]);
        let stored = vec!["codex".into(), "grok".into(), "agy".into(), "apple-fm".into()];
        let disabled = DisabledBackends::parse(Some(&stored), &offered);
        assert!(disabled.ids().is_empty());
        let decided = plan(Some(&stored), "pi", false, true, &["pi", "claude"]);
        assert_eq!(decided.session, SessionPlan::Keep);
        assert_eq!(decided.target, SessionTarget::Active("pi".into()));
    }

    #[test]
    fn an_on_device_session_this_host_cannot_offer_stays_put() {
        let decided = plan(None, "apple-fm", true, true, &["pi", "claude"]);
        assert_eq!(decided.target, SessionTarget::Unsupported);
        assert_eq!(decided.session, SessionPlan::Keep);
        assert_eq!(decided.enabled, vec!["pi".to_string(), "claude".to_string()]);
    }

    #[test]
    fn toggling_the_same_box_twice_is_a_no_op() {
        let offered = offered(&["pi", "claude"]);
        let once = DisabledBackends::parse(None, &offered).toggled("pi", true, &offered);
        let twice = once.toggled("pi", true, &offered);
        assert_eq!(once, twice);
        let restored = twice.toggled("pi", false, &offered);
        assert!(restored.ids().is_empty());
    }
}
