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
    Backend(String),
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
        SelectionIntent::Backend(backend) => {
            if !facts.slots.iter().any(|slot| slot.backend == *backend) {
                return PickEffect::None;
            }
            if facts.active_backend == backend {
                return PickEffect::None;
            }
            PickEffect::Switch {
                backend: backend.clone(),
                model: None,
            }
        }
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
        let settings = effect(&SelectionIntent::Backend("claude".into()), &closed, 3);
        assert!(matches!(settings, PickEffect::Switch { model: None, .. }));
    }

    #[test]
    fn settings_backend_for_the_hidden_clis_does_nothing() {
        let slots = gtk_slots();
        let codex = effect(
            &SelectionIntent::Backend("codex".into()),
            &facts(&slots, "pi", None),
            3,
        );
        assert_eq!(codex, PickEffect::None);
        let grok = effect(
            &SelectionIntent::Backend("grok".into()),
            &facts(&slots, "pi", None),
            3,
        );
        assert_eq!(grok, PickEffect::None);
        let apple = effect(
            &SelectionIntent::Backend("apple-fm".into()),
            &facts(&slots, "pi", None),
            3,
        );
        assert_eq!(apple, PickEffect::None);
        let same = effect(
            &SelectionIntent::Backend("pi".into()),
            &facts(&slots, "pi", None),
            3,
        );
        assert_eq!(same, PickEffect::None);
    }
}
