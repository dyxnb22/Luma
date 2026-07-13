use crate::effect::Effect;
use crate::msg::Msg;
use crate::view_model::{
    ActionsIntent, AppState, AwaitingActions, PendingAction, Route, StatusTone,
};
use luma_protocol::{ActionDescriptorDto, Event};

/// Pure synchronous reducer. Must not perform I/O.
pub fn update(state: &mut AppState, msg: Msg) -> Vec<Effect> {
    state.dirty = true;
    match msg {
        Msg::KeyChar(c) => {
            if matches!(
                state.route,
                Route::ConfirmAction
                    | Route::ActionPicker
                    | Route::Help
                    | Route::Doctor
                    | Route::QuitConfirm
            ) {
                // Typing leaves modal routes and resumes search.
                clear_action_ui(state);
                state.route = Route::Search;
            }
            state.prompt.push(c);
            schedule_search(state)
        }
        Msg::Backspace => {
            if matches!(
                state.route,
                Route::ConfirmAction | Route::ActionPicker | Route::QuitConfirm
            ) {
                return vec![Effect::None];
            }
            state.prompt.pop();
            schedule_search(state)
        }
        Msg::Submit => match state.route {
            Route::ConfirmAction => confirm_pending(state),
            Route::ActionPicker => submit_picker_selection(state),
            Route::QuitConfirm => {
                state.should_quit = true;
                cancel_active(state)
            }
            Route::Search | Route::Help | Route::Doctor => {
                if let Some(effects) = flush_pending_search_or_continue(state) {
                    return effects;
                }
                if state.prompt.trim() == ":doctor" {
                    state.prompt.clear();
                    state.route = Route::Doctor;
                    state.status.set("doctor", StatusTone::Neutral);
                    return vec![Effect::RunDoctor];
                }
                if state.prompt.trim() == ":help" || state.prompt.trim() == "?" {
                    state.prompt.clear();
                    state.route = Route::Help;
                    state.status.set("help", StatusTone::Neutral);
                    return vec![Effect::None];
                }
                request_primary_actions(state)
            }
        },
        Msg::OpenActions => {
            if let Some(effects) = flush_pending_search_or_continue(state) {
                return effects;
            }
            request_action_picker(state)
        }
        Msg::SelectNext => {
            if state.route == Route::ActionPicker {
                if !state.action_choices.is_empty() {
                    state.action_selected =
                        (state.action_selected + 1).min(state.action_choices.len() - 1);
                }
                return vec![Effect::None];
            }
            state.results.select_next();
            vec![Effect::None]
        }
        Msg::SelectPrev => {
            if state.route == Route::ActionPicker {
                state.action_selected = state.action_selected.saturating_sub(1);
                return vec![Effect::None];
            }
            state.results.select_prev();
            vec![Effect::None]
        }
        Msg::OpenHelp => {
            state.route = Route::Help;
            state.status.set("help", StatusTone::Neutral);
            vec![Effect::None]
        }
        Msg::OpenDoctor => {
            state.route = Route::Doctor;
            state.status.set("doctor", StatusTone::Neutral);
            vec![Effect::RunDoctor]
        }
        Msg::Quit => {
            if state.route == Route::QuitConfirm {
                state.should_quit = true;
                cancel_active(state)
            } else {
                clear_action_ui(state);
                state.route = Route::QuitConfirm;
                state.status.set("Quit Luma?", StatusTone::Warning);
                vec![Effect::None]
            }
        }
        Msg::Cancel => cancel_msg(state),
        Msg::FlushSearch => {
            state.search_debounce_deadline = None;
            begin_search(state)
        }
        Msg::Redraw | Msg::Resize | Msg::Tick => vec![Effect::None],
        Msg::Engine(event) => apply_engine(state, event),
    }
}

fn clear_action_ui(state: &mut AppState) {
    state.awaiting_actions = None;
    state.pending_action = None;
    state.action_choices.clear();
    state.action_result_id = None;
    state.action_selected = 0;
}

fn request_primary_actions(state: &mut AppState) -> Vec<Effect> {
    let Some(result_id) = state.results.selected_id.clone() else {
        state.status.set("no result selected", StatusTone::Warning);
        return vec![Effect::None];
    };
    state.awaiting_actions = Some(AwaitingActions {
        intent: ActionsIntent::Primary,
        result_id: result_id.clone(),
    });
    state.status.set("resolving actions…", StatusTone::Progress);
    vec![Effect::ListActions { result_id }]
}

fn request_action_picker(state: &mut AppState) -> Vec<Effect> {
    if state.route != Route::Search {
        return vec![Effect::None];
    }
    let Some(result_id) = state.results.selected_id.clone() else {
        state.status.set("no result selected", StatusTone::Warning);
        return vec![Effect::None];
    };
    state.awaiting_actions = Some(AwaitingActions {
        intent: ActionsIntent::Picker,
        result_id: result_id.clone(),
    });
    state.status.set("loading actions…", StatusTone::Progress);
    vec![Effect::ListActions { result_id }]
}

fn confirm_pending(state: &mut AppState) -> Vec<Effect> {
    let Some(pending) = state.pending_action.take() else {
        state.route = Route::Search;
        return vec![Effect::None];
    };
    state.route = Route::Search;
    execute_action(state, pending.result_id, pending.action, true)
}

fn submit_picker_selection(state: &mut AppState) -> Vec<Effect> {
    let Some(result_id) = state.action_result_id.take() else {
        state.route = Route::Search;
        clear_action_ui(state);
        return vec![Effect::None];
    };
    let Some(action) = state.action_choices.get(state.action_selected).cloned() else {
        state.route = Route::Search;
        clear_action_ui(state);
        return vec![Effect::None];
    };
    state.action_choices.clear();
    state.action_selected = 0;
    if action.needs_confirmation() {
        state.pending_action = Some(PendingAction {
            result_id,
            action: action.clone(),
        });
        state.route = Route::ConfirmAction;
        state.status.set(
            format!("confirm {}? Enter=yes Esc=no", action.label),
            StatusTone::Warning,
        );
        vec![Effect::None]
    } else {
        state.route = Route::Search;
        execute_action(state, result_id, action, false)
    }
}

fn execute_action(
    state: &mut AppState,
    result_id: String,
    action: ActionDescriptorDto,
    confirmation: bool,
) -> Vec<Effect> {
    state.search_generation = state.search_generation.saturating_add(1);
    let operation_id = format!("op-{}", state.search_generation);
    state.active_operation = Some(operation_id.clone());
    state
        .status
        .set(format!("running {}", action.label), StatusTone::Progress);
    vec![Effect::ExecuteAction {
        operation_id,
        result_id,
        action_id: action.id,
        confirmation,
    }]
}

fn begin_primary_or_confirm(
    state: &mut AppState,
    result_id: String,
    actions: Vec<ActionDescriptorDto>,
) -> Vec<Effect> {
    let primary_id = state
        .results
        .items
        .iter()
        .find(|i| i.id.as_str() == result_id)
        .map(|i| i.primary_action.id.as_str().to_string());
    let Some(primary_id) = primary_id else {
        state.status.set("no result selected", StatusTone::Warning);
        return vec![Effect::None];
    };
    let Some(action) = actions.into_iter().find(|a| a.id == primary_id) else {
        state.status.set(
            format!("module contract violation: primary action `{primary_id}` missing"),
            StatusTone::Error,
        );
        return vec![Effect::None];
    };
    if action.needs_confirmation() {
        state.pending_action = Some(PendingAction {
            result_id,
            action: action.clone(),
        });
        state.route = Route::ConfirmAction;
        state.status.set(
            format!("confirm {}? Enter=yes Esc=no", action.label),
            StatusTone::Warning,
        );
        vec![Effect::None]
    } else {
        execute_action(state, result_id, action, false)
    }
}

fn cancel_msg(state: &mut AppState) -> Vec<Effect> {
    if matches!(state.route, Route::ConfirmAction | Route::ActionPicker) {
        clear_action_ui(state);
        state.route = Route::Search;
        state.status.set("cancelled", StatusTone::Warning);
        return vec![Effect::None];
    }
    if state.route != Route::Search {
        state.route = Route::Search;
        return vec![Effect::None];
    }
    if let Some(operation_id) = state.active_operation.clone() {
        state.status.set("cancelling action…", StatusTone::Progress);
        return vec![Effect::CancelOperation { operation_id }];
    }
    if state.active_request.is_some() {
        let effects = cancel_active(state);
        state.status.set("cancelled", StatusTone::Warning);
        effects
    } else if !state.prompt.is_empty() {
        state.prompt.clear();
        state.results.items.clear();
        state.results.selected_id = None;
        state.active_request = None;
        vec![Effect::None]
    } else {
        state.should_quit = true;
        vec![Effect::None]
    }
}

fn apply_engine(state: &mut AppState, event: Event) -> Vec<Effect> {
    if let Event::ActionsAvailable { result_id, actions } = event {
        let Some(pending) = state.awaiting_actions.take() else {
            state.status.set(
                format!("{result_id}: {} actions", actions.len()),
                StatusTone::Neutral,
            );
            return vec![Effect::None];
        };
        if pending.result_id != result_id {
            // Stale / mismatched response for a different result — keep waiting.
            state.awaiting_actions = Some(pending);
            return vec![Effect::None];
        }
        match pending.intent {
            ActionsIntent::Primary => {
                return begin_primary_or_confirm(state, result_id, actions);
            }
            ActionsIntent::Picker => {
                if actions.is_empty() {
                    state
                        .status
                        .set("no actions available", StatusTone::Warning);
                    return vec![Effect::None];
                }
                state.action_result_id = Some(result_id);
                state.action_choices = actions;
                state.action_selected = 0;
                state.route = Route::ActionPicker;
                state
                    .status
                    .set("pick action · Enter run · Esc back", StatusTone::Neutral);
                return vec![Effect::None];
            }
        }
    }
    let _ = state.apply_engine_event(event);
    vec![Effect::None]
}

fn schedule_search(state: &mut AppState) -> Vec<Effect> {
    // Cancel in-flight work immediately so typing stays responsive, but delay the
    // new Search until the quiet period so bursts don't thrash modules.
    let effects = cancel_active(state);
    // Stale results must not remain actionable during debounce.
    state.results.items.clear();
    state.results.selected_id = None;
    clear_action_ui(state);
    if state.prompt.is_empty() {
        state.search_debounce_deadline = None;
        state.status.set("Ready", StatusTone::Success);
        return effects;
    }
    state.search_debounce_deadline =
        Some(std::time::Instant::now() + std::time::Duration::from_millis(80));
    state.status.set("Typing…", StatusTone::Progress);
    effects
}

fn flush_pending_search_or_continue(state: &mut AppState) -> Option<Vec<Effect>> {
    if state.search_debounce_deadline.is_some() {
        state.search_debounce_deadline = None;
        return Some(begin_search(state));
    }
    None
}

fn begin_search(state: &mut AppState) -> Vec<Effect> {
    clear_action_ui(state);
    state.search_debounce_deadline = None;
    let mut effects = cancel_active(state);
    if state.prompt.is_empty() {
        state.results.items.clear();
        state.results.selected_id = None;
        state.status.set("Ready", StatusTone::Success);
        return effects;
    }
    let request_id = next_request_id(state);
    state.active_request = Some(request_id.clone());
    state.request_seq_seen = 0;
    state.results.items.clear();
    state.results.selected_id = None;
    effects.push(Effect::Search {
        request_id,
        query: state.prompt.clone(),
    });
    effects
}

fn cancel_active(state: &mut AppState) -> Vec<Effect> {
    if let Some(request_id) = state.active_request.take() {
        vec![Effect::CancelSearch { request_id }]
    } else {
        Vec::new()
    }
}

fn next_request_id(state: &mut AppState) -> String {
    state.search_generation = state.search_generation.saturating_add(1);
    format!("req-{}", state.search_generation)
}

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
mod tests {
    use super::*;
    use crate::view_model::StatusTone;
    use luma_domain::{ActionDescriptor, ActionId, ActionRisk, ModuleId, ResultId, SearchItem};
    use luma_protocol::{ActionDescriptorDto, Event, SearchItemDto};

    #[test]
    fn typing_schedules_search_then_flush_cancels_old() {
        let mut state = AppState::default();
        let effects = update(&mut state, Msg::KeyChar('a'));
        assert!(state.search_debounce_deadline.is_some());
        assert!(!effects.iter().any(|e| matches!(e, Effect::Search { .. })));

        let effects = update(&mut state, Msg::FlushSearch);
        assert!(matches!(effects.last(), Some(Effect::Search { .. })));
        let first = state.active_request.clone().unwrap();

        let effects = update(&mut state, Msg::KeyChar('p'));
        assert!(effects
            .iter()
            .any(|e| matches!(e, Effect::CancelSearch { request_id } if request_id == &first)));
        assert!(!effects.iter().any(|e| matches!(e, Effect::Search { .. })));
        let effects = update(&mut state, Msg::FlushSearch);
        assert!(effects.iter().any(|e| matches!(e, Effect::Search { .. })));
    }

    #[test]
    fn late_chunk_from_old_request_is_ignored() {
        let mut state = AppState::default();
        let _ = update(&mut state, Msg::KeyChar('a'));
        let _ = update(&mut state, Msg::FlushSearch);
        let active = state.active_request.clone().unwrap();

        let _ = update(&mut state, Msg::KeyChar('b'));
        let _ = update(&mut state, Msg::FlushSearch);
        let new_active = state.active_request.clone().unwrap();
        assert_ne!(active, new_active);

        let applied = state.apply_engine_event(Event::ResultsChunk {
            request_id: active,
            sequence: 1,
            upserts: vec![SearchItemDto {
                id: "old".into(),
                module_id: "mock".into(),
                title: "old".into(),
                subtitle: None,
                kind: "mock".into(),
                score: 1.0,
                primary_action_id: "open".into(),
                primary_action_label: "Open".into(),
                ..Default::default()
            }],
            removed_ids: vec![],
        });
        assert!(!applied);
        assert!(state.results.items.is_empty());
    }

    #[test]
    fn chunk_for_active_request_updates_results() {
        let mut state = AppState::default();
        let _ = update(&mut state, Msg::KeyChar('x'));
        let _ = update(&mut state, Msg::FlushSearch);
        let active = state.active_request.clone().unwrap();
        let applied = state.apply_engine_event(Event::ResultsChunk {
            request_id: active,
            sequence: 1,
            upserts: vec![SearchItemDto {
                id: "1".into(),
                module_id: "mock".into(),
                title: "Alpha".into(),
                subtitle: None,
                kind: "mock".into(),
                score: 10.0,
                primary_action_id: "open".into(),
                primary_action_label: "Open".into(),
                ..Default::default()
            }],
            removed_ids: vec![],
        });
        assert!(applied);
        assert_eq!(state.results.items.len(), 1);
        assert_eq!(state.results.selected_id.as_deref(), Some("1"));
    }

    #[test]
    fn typing_clears_stale_results_and_submit_flushes_search_only() {
        let mut state = AppState::default();
        state.prompt = "old".into();
        let _ = update(&mut state, Msg::FlushSearch);
        let applied = state.apply_engine_event(Event::ResultsChunk {
            request_id: state.active_request.clone().unwrap(),
            sequence: 1,
            upserts: vec![SearchItemDto {
                id: "stale".into(),
                module_id: "mock".into(),
                title: "stale hit".into(),
                subtitle: None,
                kind: "mock".into(),
                score: 1.0,
                primary_action_id: "open".into(),
                primary_action_label: "Open".into(),
                ..Default::default()
            }],
            removed_ids: vec![],
        });
        assert!(applied);
        assert_eq!(state.results.items.len(), 1);
        state.results.selected_id = Some("stale".into());

        // One more character — debounce pending, old selection must be cleared.
        let _ = update(&mut state, Msg::KeyChar('x'));
        assert!(state.search_debounce_deadline.is_some());
        assert!(state.results.items.is_empty());
        assert!(state.results.selected_id.is_none());

        let effects = update(&mut state, Msg::Submit);
        assert!(
            effects.iter().any(|e| matches!(e, Effect::Search { .. })),
            "Submit while debounce pending should flush a new search"
        );
        assert!(
            !effects
                .iter()
                .any(|e| matches!(e, Effect::ExecuteAction { .. } | Effect::ListActions { .. })),
            "must not act on the stale selection"
        );
    }

    #[test]
    fn doctor_meta_emits_run_doctor_not_primary() {
        let mut state = AppState::default();
        state.results.items.push(SearchItem {
            id: ResultId::new("danger"),
            module_id: ModuleId::new("mock"),
            title: "Danger".into(),
            subtitle: None,
            kind: "mock".into(),
            score: 1.0,
            primary_action: ActionDescriptor {
                id: ActionId::new("open"),
                label: "Open".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            },
            secondary_actions: vec![],
        });
        state.results.selected_id = Some("danger".into());
        state.prompt = ":doctor".into();
        let effects = update(&mut state, Msg::Submit);
        assert_eq!(effects, vec![Effect::RunDoctor]);
        assert_eq!(state.route, Route::Doctor);
        assert_eq!(state.status.text, "doctor");
    }

    #[test]
    fn help_meta_does_not_run_doctor_or_primary_status() {
        let mut state = AppState::default();
        state.prompt = ":help".into();
        let effects = update(&mut state, Msg::Submit);
        assert_eq!(effects, vec![Effect::None]);
        assert_eq!(state.route, Route::Help);
        assert_eq!(state.status.text, "help");
    }

    #[test]
    fn cancel_quits_from_empty_search() {
        let mut state = AppState::default();
        let _ = update(&mut state, Msg::Cancel);
        assert!(state.should_quit);
    }

    #[test]
    fn submit_lists_actions_for_selected_result() {
        let mut state = AppState::default();
        state.results.items.push(SearchItem {
            id: ResultId::new("1"),
            module_id: ModuleId::new("mock"),
            title: "One".into(),
            subtitle: None,
            kind: "mock".into(),
            score: 1.0,
            primary_action: ActionDescriptor {
                id: ActionId::new("open"),
                label: "Open".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            },
            secondary_actions: vec![],
        });
        state.results.selected_id = Some("1".into());
        let effects = update(&mut state, Msg::Submit);
        assert_eq!(
            effects,
            vec![Effect::ListActions {
                result_id: "1".into()
            }]
        );
        assert_eq!(
            state.awaiting_actions,
            Some(AwaitingActions {
                intent: ActionsIntent::Primary,
                result_id: "1".into(),
            })
        );
    }

    #[test]
    fn actions_available_enters_confirm_for_destructive() {
        let mut state = AppState::default();
        state.results.items.push(SearchItem {
            id: ResultId::new("1"),
            module_id: ModuleId::new("mock"),
            title: "One".into(),
            subtitle: None,
            kind: "mock".into(),
            score: 1.0,
            primary_action: ActionDescriptor {
                id: ActionId::new("force"),
                label: "Force".into(),
                risk: ActionRisk::Destructive,
                confirmation: true,
            },
            secondary_actions: vec![],
        });
        state.results.selected_id = Some("1".into());
        state.awaiting_actions = Some(AwaitingActions {
            intent: ActionsIntent::Primary,
            result_id: "1".into(),
        });
        let effects = update(
            &mut state,
            Msg::Engine(Event::ActionsAvailable {
                result_id: "1".into(),
                actions: vec![ActionDescriptorDto {
                    id: "force".into(),
                    label: "Force".into(),
                    risk: ActionRisk::Destructive,
                    confirmation: true,
                }],
            }),
        );
        assert_eq!(effects, vec![Effect::None]);
        assert_eq!(state.route, Route::ConfirmAction);
        assert!(state.pending_action.is_some());
    }

    #[test]
    fn missing_primary_action_reports_contract_violation() {
        let mut state = AppState::default();
        state.results.items.push(SearchItem {
            id: ResultId::new("1"),
            module_id: ModuleId::new("mock"),
            title: "One".into(),
            subtitle: None,
            kind: "mock".into(),
            score: 1.0,
            primary_action: ActionDescriptor {
                id: ActionId::new("create"),
                label: "Create".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            },
            secondary_actions: vec![],
        });
        state.results.selected_id = Some("1".into());
        state.awaiting_actions = Some(AwaitingActions {
            intent: ActionsIntent::Primary,
            result_id: "1".into(),
        });
        let effects = update(
            &mut state,
            Msg::Engine(Event::ActionsAvailable {
                result_id: "1".into(),
                actions: vec![ActionDescriptorDto {
                    id: "open".into(),
                    label: "Open".into(),
                    risk: ActionRisk::Safe,
                    confirmation: false,
                }],
            }),
        );
        assert_eq!(effects, vec![Effect::None]);
        assert_eq!(state.route, Route::Search);
        assert!(state.pending_action.is_none());
        assert!(state.status.text.contains("module contract violation"));
        assert_eq!(state.status.tone, StatusTone::Error);
    }

    #[test]
    fn confirm_submit_executes_with_confirmation_true() {
        let mut state = AppState::default();
        state.route = Route::ConfirmAction;
        state.pending_action = Some(PendingAction {
            result_id: "1".into(),
            action: ActionDescriptorDto {
                id: "force".into(),
                label: "Force".into(),
                risk: ActionRisk::Destructive,
                confirmation: true,
            },
        });
        let effects = update(&mut state, Msg::Submit);
        assert!(matches!(
            effects.as_slice(),
            [Effect::ExecuteAction {
                action_id,
                confirmation: true,
                ..
            }] if action_id == "force"
        ));
        assert_eq!(state.route, Route::Search);
    }

    #[test]
    fn tab_opens_action_picker() {
        let mut state = AppState::default();
        state.results.selected_id = Some("1".into());
        let effects = update(&mut state, Msg::OpenActions);
        assert_eq!(
            effects,
            vec![Effect::ListActions {
                result_id: "1".into()
            }]
        );
        assert_eq!(
            state.awaiting_actions,
            Some(AwaitingActions {
                intent: ActionsIntent::Picker,
                result_id: "1".into(),
            })
        );
        let effects = update(
            &mut state,
            Msg::Engine(Event::ActionsAvailable {
                result_id: "1".into(),
                actions: vec![
                    ActionDescriptorDto {
                        id: "open".into(),
                        label: "Open".into(),
                        risk: ActionRisk::Safe,
                        confirmation: false,
                    },
                    ActionDescriptorDto {
                        id: "delete".into(),
                        label: "Delete".into(),
                        risk: ActionRisk::Destructive,
                        confirmation: true,
                    },
                ],
            }),
        );
        assert_eq!(effects, vec![Effect::None]);
        assert_eq!(state.route, Route::ActionPicker);
        assert_eq!(state.action_choices.len(), 2);
        assert_eq!(state.action_result_id.as_deref(), Some("1"));
    }

    #[test]
    fn mismatched_actions_available_is_ignored() {
        let mut state = AppState::default();
        state.awaiting_actions = Some(AwaitingActions {
            intent: ActionsIntent::Picker,
            result_id: "A".into(),
        });
        let effects = update(
            &mut state,
            Msg::Engine(Event::ActionsAvailable {
                result_id: "B".into(),
                actions: vec![ActionDescriptorDto {
                    id: "delete".into(),
                    label: "Delete".into(),
                    risk: ActionRisk::Destructive,
                    confirmation: true,
                }],
            }),
        );
        assert_eq!(effects, vec![Effect::None]);
        assert_eq!(state.route, Route::Search);
        assert!(state.action_choices.is_empty());
        assert_eq!(
            state
                .awaiting_actions
                .as_ref()
                .map(|a| a.result_id.as_str()),
            Some("A")
        );
    }

    #[test]
    fn picker_submit_uses_pinned_result_id_not_selection() {
        let mut state = AppState::default();
        state.route = Route::ActionPicker;
        state.action_result_id = Some("A".into());
        state.results.selected_id = Some("B".into());
        state.action_choices = vec![ActionDescriptorDto {
            id: "delete".into(),
            label: "Delete".into(),
            risk: ActionRisk::Destructive,
            confirmation: true,
        }];
        state.action_selected = 0;
        let effects = update(&mut state, Msg::Submit);
        assert_eq!(effects, vec![Effect::None]);
        assert_eq!(state.route, Route::ConfirmAction);
        assert_eq!(
            state.pending_action.as_ref().map(|p| p.result_id.as_str()),
            Some("A")
        );
    }

    #[test]
    fn search_finished_clears_active_request_so_esc_clears_immediately() {
        let mut state = AppState::default();
        let _ = update(&mut state, Msg::KeyChar('a'));
        let _ = update(&mut state, Msg::FlushSearch);
        let request_id = state.active_request.clone().expect("active request");
        let applied = state.apply_engine_event(Event::SearchFinished {
            request_id,
            total: 1,
            elapsed_ms: 3,
        });
        assert!(applied);
        assert!(state.active_request.is_none());
        state.results.items.push(SearchItem {
            id: ResultId::new("1"),
            module_id: ModuleId::new("mock"),
            title: "One".into(),
            subtitle: None,
            kind: "mock".into(),
            score: 1.0,
            primary_action: ActionDescriptor {
                id: ActionId::new("open"),
                label: "Open".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            },
            secondary_actions: vec![],
        });
        state.prompt = "a".into();
        let effects = update(&mut state, Msg::Cancel);
        assert_eq!(effects, vec![Effect::None]);
        assert!(state.prompt.is_empty());
        assert!(state.results.items.is_empty());
        assert!(!state.should_quit);
    }

    #[test]
    fn quit_opens_confirm_then_enter_exits() {
        let mut state = AppState::default();
        let effects = update(&mut state, Msg::Quit);
        assert_eq!(effects, vec![Effect::None]);
        assert_eq!(state.route, Route::QuitConfirm);
        assert!(!state.should_quit);

        let effects = update(&mut state, Msg::Submit);
        assert!(state.should_quit);
        assert!(
            effects.is_empty()
                || effects
                    .iter()
                    .all(|e| matches!(e, Effect::None | Effect::CancelSearch { .. }))
        );
    }

    #[test]
    fn quit_confirm_esc_returns_to_search() {
        let mut state = AppState::default();
        let _ = update(&mut state, Msg::Quit);
        let _ = update(&mut state, Msg::Cancel);
        assert_eq!(state.route, Route::Search);
        assert!(!state.should_quit);
    }
}
