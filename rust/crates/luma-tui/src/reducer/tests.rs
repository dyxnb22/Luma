use super::navigation::seed_record_edit;
use super::navigation::{browse_query_parent, drill_into_browse};
use super::*;
use crate::view_model::{ActionsIntent, AwaitingActions, PendingAction, StatusTone};
use luma_domain::{ActionDescriptor, ActionId, ActionRisk, ModuleId, ResultId, SearchItem};
use luma_protocol::{ActionDescriptorDto, Event, SearchItemDto};

#[test]
fn typing_schedules_search_then_flush_cancels_old() {
    let mut state = AppState::default();
    let effects = update(&mut state, Msg::KeyChar('a'));
    assert!(state.search.debounce_deadline.is_some());
    assert!(!effects.iter().any(|e| matches!(e, Effect::Search { .. })));

    let effects = update(&mut state, Msg::FlushSearch);
    assert!(matches!(effects.last(), Some(Effect::Search { .. })));
    let first = state.search.active_request.clone().unwrap();

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
    let active = state.search.active_request.clone().unwrap();

    let _ = update(&mut state, Msg::KeyChar('b'));
    let _ = update(&mut state, Msg::FlushSearch);
    let new_active = state.search.active_request.clone().unwrap();
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
    assert!(state.search.results.items.is_empty());
}

#[test]
fn chunk_for_active_request_updates_results() {
    let mut state = AppState::default();
    let _ = update(&mut state, Msg::KeyChar('x'));
    let _ = update(&mut state, Msg::FlushSearch);
    let active = state.search.active_request.clone().unwrap();
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
    assert_eq!(state.search.results.items.len(), 1);
    assert_eq!(state.search.results.selected_id.as_deref(), Some("1"));
}

#[test]
fn typing_keeps_results_during_debounce_submit_flushes_search() {
    let mut state = AppState::default();
    state.search.prompt = "old".into();
    let _ = update(&mut state, Msg::FlushSearch);
    let applied = state.apply_engine_event(Event::ResultsChunk {
        request_id: state.search.active_request.clone().unwrap(),
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
    assert_eq!(state.search.results.items.len(), 1);
    state.search.results.selected_id = Some("stale".into());

    // One more character — debounce pending; keep prior rows to avoid empty flash.
    let _ = update(&mut state, Msg::KeyChar('x'));
    assert!(state.search.debounce_deadline.is_some());
    assert_eq!(state.search.results.items.len(), 1);
    assert_eq!(state.status.text, "Typing…");

    let effects = update(&mut state, Msg::Submit);
    assert!(
        effects.iter().any(|e| matches!(e, Effect::Search { .. })),
        "Submit while debounce pending should flush a new search"
    );
    assert!(state.search.results.items.is_empty());
    assert!(state.search.debounce_deadline.is_none());
    assert!(
        !effects
            .iter()
            .any(|e| matches!(e, Effect::ExecuteAction { .. } | Effect::ListActions { .. })),
        "must not act on the stale selection"
    );
}

#[test]
fn hub_digit_focuses_third_window() {
    let mut state = AppState::default();
    state.hub.windows = Some(crate::view_model::HubWindowsState {
        app_name: "all".into(),
        windows: vec![
            crate::view_model::HubWindowRow {
                id: "win:1".into(),
                title: "A".into(),
            },
            crate::view_model::HubWindowRow {
                id: "win:2".into(),
                title: "B".into(),
            },
            crate::view_model::HubWindowRow {
                id: "win:3".into(),
                title: "C".into(),
            },
        ],
        more: None,
        status_kind: Some("permission_required".into()),
        status_title: Some("hint".into()),
        status_subtitle: None,
    });
    let effects = update(&mut state, Msg::PickWindowDigit(3));
    assert!(effects.iter().any(|e| matches!(
        e,
        Effect::ExecuteAction { result_id, action_id, .. }
        if result_id == "win:3" && action_id == "focus"
    )));
}

#[test]
fn win_digit_only_when_list_focused() {
    let mut state = AppState::default();
    state.search.prompt = "/win ".into();
    state.search.prompt_cursor = state.prompt_char_len();
    state.search.results.items.push(SearchItem {
        id: ResultId::new("win:a"),
        module_id: ModuleId::new("luma.windows"),
        title: "A".into(),
        subtitle: None,
        kind: "window".into(),
        score: 1.0,
        primary_action: ActionDescriptor {
            id: ActionId::new("focus"),
            label: "Focus".into(),
            risk: ActionRisk::Safe,
            confirmation: false,
        },
        secondary_actions: vec![],
        ui_intent: None,
        action_payload: None,
    });
    state.focus = FocusZone::Prompt;
    let effects = update(&mut state, Msg::PickWindowDigit(1));
    assert_eq!(effects, vec![Effect::None]);
    state.focus = FocusZone::List;
    let effects = update(&mut state, Msg::PickWindowDigit(1));
    assert!(effects.iter().any(|e| matches!(
        e,
        Effect::ExecuteAction { result_id, action_id, .. }
        if result_id == "win:a" && action_id == "focus"
    )));
}

#[test]
fn help_meta_does_not_run_primary_status() {
    let mut state = AppState::default();
    state.search.prompt = "/help".into();
    let effects = update(&mut state, Msg::Submit);
    assert_eq!(effects, vec![Effect::None]);
    assert_eq!(state.route, Route::Help);
    assert_eq!(state.status.text, "help");
}

#[test]
fn slash_help_meta_opens_help_without_search() {
    let mut state = AppState::default();
    state.search.prompt = "/help".into();
    let effects = update(&mut state, Msg::Submit);
    assert_eq!(effects, vec![Effect::None]);
    assert_eq!(state.route, Route::Help);
    assert_eq!(state.status.text, "help");
}

#[test]
fn meta_command_does_not_require_enter_to_flush_debounce() {
    let mut state = AppState::default();
    for c in "/commands".chars() {
        let _ = update(&mut state, Msg::KeyChar(c));
    }
    assert!(state.search.debounce_deadline.is_some());

    let effects = update(&mut state, Msg::Submit);

    assert_eq!(effects, vec![Effect::None]);
    assert_eq!(state.route, Route::Commands);
    assert!(state.search.prompt.is_empty());
    assert_eq!(state.overlay.restore_prompt.as_deref(), Some("/commands"));
    assert!(state.search.debounce_deadline.is_none());
}

#[test]
fn esc_from_commands_restores_meta_prompt() {
    let mut state = AppState::default();
    state.search.prompt = "/commands".into();
    state.search.prompt_cursor = state.prompt_char_len();
    let _ = update(&mut state, Msg::Submit);
    assert_eq!(state.route, Route::Commands);
    assert!(state.search.prompt.is_empty());

    let _ = update(&mut state, Msg::Cancel);
    assert_eq!(state.route, Route::Search);
    assert_eq!(state.search.prompt, "/commands");
    assert!(state.overlay.restore_prompt.is_none());
}

#[test]
fn esc_clears_prompt_and_debounce() {
    let mut state = AppState::default();
    state.search.prompt = "clip hello".into();
    state.search.prompt_cursor = state.prompt_char_len();
    let _ = update(&mut state, Msg::KeyChar('x'));
    assert!(state.search.debounce_deadline.is_some());

    let _ = update(&mut state, Msg::Cancel);

    assert!(state.search.prompt.is_empty());
    assert!(state.search.debounce_deadline.is_none());
}

#[test]
fn esc_to_empty_prompt_reloads_hub() {
    let mut state = AppState::default();
    state.search.prompt = "app safari".into();
    state.search.prompt_cursor = state.prompt_char_len();
    let effects = update(&mut state, Msg::Cancel);
    assert!(state.search.prompt.is_empty());
    assert!(effects.iter().any(|e| matches!(e, Effect::LoadHub)));
}

#[test]
fn cancel_opens_quit_confirm_from_empty_search() {
    let mut state = AppState::default();
    let _ = update(&mut state, Msg::Cancel);
    assert_eq!(state.route, Route::QuitConfirm);
    assert!(!state.should_quit);
}

#[test]
fn browse_query_parent_pops_one_path_component() {
    assert_eq!(
        browse_query_parent("/n browse /Notes/Inbox/nested"),
        Some("/n browse /Notes/Inbox".into())
    );
    assert_eq!(
        browse_query_parent("/proj browse /dev/app"),
        Some("/proj browse /dev".into())
    );
    assert_eq!(browse_query_parent("/n browse"), None);
    assert_eq!(browse_query_parent("/n hello"), None);
    assert_eq!(
        browse_query_parent("/n browse /Notes"),
        Some("/n browse /".into())
    );
    assert_eq!(
        browse_query_parent("/n browse /Notes/Inbox"),
        Some("/n browse /Notes".into())
    );
}

#[test]
fn records_category_browse_uses_rec_trigger_and_category_payload() {
    let mut state = AppState::default();
    let item = SearchItem {
        id: ResultId::new("rec:cat:电影"),
        module_id: ModuleId::new("luma.records"),
        title: "电影/".into(),
        subtitle: Some("Enter to browse category".into()),
        kind: "category".into(),
        score: 1.0,
        primary_action: ActionDescriptor {
            id: ActionId::new("browse"),
            label: "Browse".into(),
            risk: ActionRisk::Safe,
            confirmation: false,
        },
        secondary_actions: vec![],
        ui_intent: Some("browse".into()),
        action_payload: Some(serde_json::json!({
            "browse_trigger": "rec",
            "category": "电影",
        })),
    };
    let effects = drill_into_browse(&mut state, &item);
    assert_eq!(state.search.prompt, "/rec browse 电影");
    assert!(effects
        .iter()
        .any(|e| matches!(e, Effect::Search { query, .. } if query == "/rec browse 电影")));
}

#[test]
fn browse_requires_explicit_routing_payload() {
    let mut state = AppState::default();
    let item = SearchItem {
        id: ResultId::new("rec:cat:纪录片"),
        module_id: ModuleId::new("custom.ledger"),
        title: "纪录片/".into(),
        subtitle: None,
        kind: "category".into(),
        score: 1.0,
        primary_action: ActionDescriptor {
            id: ActionId::new("browse"),
            label: "Browse".into(),
            risk: ActionRisk::Safe,
            confirmation: false,
        },
        secondary_actions: vec![],
        ui_intent: Some("browse".into()),
        action_payload: Some(serde_json::json!({ "category": "纪录片" })),
    };
    let _ = drill_into_browse(&mut state, &item);
    assert_eq!(state.search.prompt, "");
    assert_eq!(state.status.tone, StatusTone::Error);
}

#[test]
fn records_actions_seed_prompt_for_rate_and_note() {
    let mut state = AppState::default();
    let item = SearchItem {
        id: ResultId::new("rec:42"),
        module_id: ModuleId::new("luma.records"),
        title: "沙丘".into(),
        subtitle: None,
        kind: "record".into(),
        score: 1.0,
        primary_action: ActionDescriptor {
            id: ActionId::new("open"),
            label: "View".into(),
            risk: ActionRisk::Safe,
            confirmation: false,
        },
        secondary_actions: vec![],
        ui_intent: None,
        action_payload: None,
    };
    let _ = seed_record_edit(&mut state, &item, "rate");
    assert_eq!(state.search.prompt, "/rec rate 42 ");
    let _ = seed_record_edit(&mut state, &item, "note");
    assert_eq!(state.search.prompt, "/rec note 42 ");
}

#[test]
fn esc_pops_browse_nav_stack_then_clears_at_root() {
    let mut state = AppState::default();
    state.search.prompt = "/n browse".into();
    state.search.prompt_cursor = state.prompt_char_len();
    state.search.results.items.push(SearchItem {
        id: ResultId::new("browse:n:/tmp/notes/Inbox"),
        module_id: ModuleId::new("luma.notes"),
        title: "Inbox/".into(),
        subtitle: Some("/tmp/notes/Inbox".into()),
        kind: "directory".into(),
        score: 1.0,
        primary_action: ActionDescriptor {
            id: ActionId::new("browse"),
            label: "Browse".into(),
            risk: ActionRisk::Safe,
            confirmation: false,
        },
        secondary_actions: vec![],
        ui_intent: Some("browse".into()),
        action_payload: Some(serde_json::json!({
            "browse_trigger": "n",
            "path": "/tmp/notes/Inbox",
        })),
    });
    state.search.results.selected_id = Some("browse:n:/tmp/notes/Inbox".into());
    let _ = update(&mut state, Msg::Submit);
    assert_eq!(state.search.prompt, "/n browse /tmp/notes/Inbox");
    assert_eq!(state.search.browse_nav_stack, vec!["/n browse".to_string()]);

    state.search.active_request = None;
    let effects = update(&mut state, Msg::Cancel);
    assert!(
        effects.iter().any(|e| matches!(e, Effect::Search { .. })),
        "expected search after browse-up: {effects:?}"
    );
    assert_eq!(state.search.prompt, "/n browse");
    assert!(state.search.browse_nav_stack.is_empty());

    state.search.active_request = None;
    let effects = update(&mut state, Msg::Cancel);
    assert!(
        effects.iter().any(|e| matches!(e, Effect::LoadHub)),
        "expected LoadHub after clearing to hub: {effects:?}"
    );
    assert!(state.search.prompt.is_empty());
    assert!(!state.should_quit);
}

#[test]
fn ctrl_u_clears_browse_stack_for_home() {
    let mut state = AppState::default();
    state.search.prompt = "/n browse /tmp/notes/Inbox".into();
    state.search.prompt_cursor = state.prompt_char_len();
    state.search.browse_nav_stack = vec!["/n browse".into()];
    let _ = update(&mut state, Msg::ClearToStart);
    assert!(state.search.prompt.is_empty());
    assert!(state.search.browse_nav_stack.is_empty());
}

#[test]
fn prompt_cursor_inserts_in_middle() {
    let mut state = AppState::default();
    let _ = update(&mut state, Msg::KeyChar('a'));
    let _ = update(&mut state, Msg::KeyChar('c'));
    let _ = update(&mut state, Msg::CursorLeft);
    let _ = update(&mut state, Msg::KeyChar('b'));
    assert_eq!(state.search.prompt, "abc");
    assert_eq!(state.search.prompt_cursor, 2);
}

#[test]
fn page_down_moves_selection() {
    let mut state = AppState::default();
    for i in 0..12 {
        state.search.results.items.push(SearchItem {
            id: ResultId::new(format!("{i}")),
            module_id: ModuleId::new("mock"),
            title: format!("Item {i}"),
            subtitle: None,
            kind: "mock".into(),
            score: (12 - i) as f64,
            primary_action: ActionDescriptor {
                id: ActionId::new("open"),
                label: "Open".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            },
            secondary_actions: vec![],
            ui_intent: None,
            action_payload: None,
        });
    }
    state.search.results.selected_id = Some("0".into());
    let _ = update(&mut state, Msg::SelectPageDown);
    assert_eq!(state.search.results.selected_id.as_deref(), Some("5"));
}

#[test]
fn action_picker_digit_runs_action() {
    let mut state = AppState::default();
    state.route = Route::ActionPicker;
    state.actions.action_result_id = Some("r1".into());
    state.actions.action_choices = vec![
        ActionDescriptorDto {
            id: "a".into(),
            label: "A".into(),
            risk: luma_domain::ActionRisk::Safe,
            confirmation: false,
        },
        ActionDescriptorDto {
            id: "b".into(),
            label: "B".into(),
            risk: luma_domain::ActionRisk::Safe,
            confirmation: false,
        },
    ];
    let effects = update(&mut state, Msg::PickActionDigit(2));
    assert!(
        effects
            .iter()
            .any(|e| matches!(e, Effect::ExecuteAction { action_id, .. } if action_id == "b")),
        "{effects:?}"
    );
}

#[test]
fn submit_lists_actions_for_selected_result() {
    let mut state = AppState::default();
    state.search.results.items.push(SearchItem {
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
        ui_intent: None,
        action_payload: None,
    });
    state.search.results.selected_id = Some("1".into());
    let effects = update(&mut state, Msg::Submit);
    assert_eq!(
        effects,
        vec![Effect::ListActions {
            result_id: "1".into()
        }]
    );
    assert_eq!(
        state.actions.awaiting_actions,
        Some(AwaitingActions {
            intent: ActionsIntent::Primary,
            result_id: "1".into(),
        })
    );
}

#[test]
fn actions_available_enters_confirm_for_destructive() {
    let mut state = AppState::default();
    state.search.results.items.push(SearchItem {
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
        ui_intent: None,
        action_payload: None,
    });
    state.search.results.selected_id = Some("1".into());
    state.actions.awaiting_actions = Some(AwaitingActions {
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
    assert!(state.actions.pending_action.is_some());
}

#[test]
fn missing_primary_action_reports_contract_violation() {
    let mut state = AppState::default();
    state.search.results.items.push(SearchItem {
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
        ui_intent: None,
        action_payload: None,
    });
    state.search.results.selected_id = Some("1".into());
    state.actions.awaiting_actions = Some(AwaitingActions {
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
    assert!(state.actions.pending_action.is_none());
    assert!(state.status.text.contains("module contract violation"));
    assert_eq!(state.status.tone, StatusTone::Error);
}

#[test]
fn confirm_submit_executes_with_confirmation_true() {
    let mut state = AppState::default();
    state.route = Route::ConfirmAction;
    state.actions.pending_action = Some(PendingAction {
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
fn second_execute_rejected_while_action_active() {
    let mut state = AppState::default();
    state.actions.active_operation = Some("op-1".into());
    state.search.results.items = vec![SearchItem {
        id: ResultId::new("1"),
        module_id: ModuleId::new("luma.fake"),
        title: "hit".into(),
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
        ui_intent: None,
        action_payload: None,
    }];
    state.search.results.selected_id = Some("1".into());
    state.actions.awaiting_actions = Some(AwaitingActions {
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
    assert_eq!(state.actions.active_operation.as_deref(), Some("op-1"));
    assert!(state.status.text.contains("already running"));
}

#[test]
fn tab_opens_action_picker() {
    let mut state = AppState::default();
    state.search.results.selected_id = Some("1".into());
    let effects = update(&mut state, Msg::OpenActions);
    assert_eq!(
        effects,
        vec![Effect::ListActions {
            result_id: "1".into()
        }]
    );
    assert_eq!(
        state.actions.awaiting_actions,
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
    assert_eq!(state.actions.action_choices.len(), 2);
    assert_eq!(state.actions.action_result_id.as_deref(), Some("1"));
}

#[test]
fn mismatched_actions_available_is_ignored() {
    let mut state = AppState::default();
    state.actions.awaiting_actions = Some(AwaitingActions {
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
    assert!(state.actions.action_choices.is_empty());
    assert_eq!(
        state
            .actions
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
    state.actions.action_result_id = Some("A".into());
    state.search.results.selected_id = Some("B".into());
    state.actions.action_choices = vec![ActionDescriptorDto {
        id: "delete".into(),
        label: "Delete".into(),
        risk: ActionRisk::Destructive,
        confirmation: true,
    }];
    state.actions.action_selected = 0;
    let effects = update(&mut state, Msg::Submit);
    assert_eq!(effects, vec![Effect::None]);
    assert_eq!(state.route, Route::ConfirmAction);
    assert_eq!(
        state
            .actions
            .pending_action
            .as_ref()
            .map(|p| p.result_id.as_str()),
        Some("A")
    );
}

#[test]
fn search_finished_clears_active_request_so_esc_clears_immediately() {
    let mut state = AppState::default();
    let _ = update(&mut state, Msg::KeyChar('a'));
    let _ = update(&mut state, Msg::FlushSearch);
    let request_id = state.search.active_request.clone().expect("active request");
    let applied = state.apply_engine_event(Event::SearchFinished {
        request_id,
        total: 1,
        elapsed_ms: 3,
    });
    assert!(applied);
    assert!(state.search.active_request.is_none());
    state.search.results.items.push(SearchItem {
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
        ui_intent: None,
        action_payload: None,
    });
    state.search.prompt = "a".into();
    let effects = update(&mut state, Msg::Cancel);
    assert!(
        effects.iter().any(|e| matches!(e, Effect::LoadHub)),
        "expected LoadHub after Esc to empty: {effects:?}"
    );
    assert!(state.search.prompt.is_empty());
    assert!(state.search.results.items.is_empty());
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

#[test]
fn settings_ctrl_u_does_not_edit_prompt() {
    let mut state = AppState::default();
    state.search.prompt = "keep me".into();
    state.search.prompt_cursor = state.prompt_char_len();
    state.route = Route::Settings;
    state.settings.modules = vec![crate::view_model::SettingsModuleRow {
        id: "luma.fake".into(),
        name: "Fake".into(),
        enabled: true,
    }];
    let effects = update(&mut state, Msg::ClearToStart);
    assert_eq!(effects, vec![Effect::None]);
    assert_eq!(state.search.prompt, "keep me");
    assert!(state.search.debounce_deadline.is_none());
}

#[test]
fn stale_preview_loaded_is_ignored() {
    let mut state = AppState::default();
    state.search.results.items.push(luma_domain::SearchItem {
        id: luma_domain::ResultId::new("note:a"),
        module_id: luma_domain::ModuleId::new("luma.notes"),
        title: "a".into(),
        subtitle: None,
        kind: "note".into(),
        score: 1.0,
        primary_action: luma_domain::ActionDescriptor {
            id: luma_domain::ActionId::new("open"),
            label: "Open".into(),
            risk: luma_domain::ActionRisk::Safe,
            confirmation: false,
        },
        secondary_actions: vec![],
        ui_intent: None,
        action_payload: None,
    });
    state.search.results.selected_id = Some("note:a".into());
    let effects = preview_effect(&mut state);
    let Effect::LoadPreview {
        preview_id: first_id,
        ..
    } = &effects[0]
    else {
        panic!("{effects:?}");
    };
    let first_id = *first_id;

    // New request for same selection after bump (simulate re-select).
    state.preview.pending_id = None;
    state.preview.body = None;
    let effects2 = preview_effect(&mut state);
    let Effect::LoadPreview {
        preview_id: second_id,
        ..
    } = &effects2[0]
    else {
        panic!("{effects2:?}");
    };
    assert_ne!(first_id, *second_id);

    let applied = state.apply_engine_event(Event::PreviewLoaded {
        result_id: "note:a".into(),
        preview_id: first_id,
        body: "STALE".into(),
    });
    assert!(!applied);
    assert_ne!(state.preview.body.as_deref(), Some("STALE"));

    let applied = state.apply_engine_event(Event::PreviewLoaded {
        result_id: "note:a".into(),
        preview_id: *second_id,
        body: "FRESH".into(),
    });
    assert!(applied);
    assert_eq!(state.preview.body.as_deref(), Some("FRESH"));
}

#[test]
fn toggle_setting_uses_update_settings_cas() {
    let mut state = AppState::default();
    state.route = Route::Settings;
    state.settings.version = 3;
    state.settings.modules = vec![crate::view_model::SettingsModuleRow {
        id: "luma.fake".into(),
        name: "Fake".into(),
        enabled: true,
    }];
    let effects = update(&mut state, Msg::ToggleSetting);
    assert_eq!(
        effects,
        vec![Effect::UpdateSettings {
            module_id: "luma.fake".into(),
            enabled: false,
            expected_version: 3,
        }]
    );
    // No optimistic flip — wait for SettingsChanged.
    assert!(state.settings.modules[0].enabled);
}

#[test]
fn hub_window_enter_focuses_without_prompt() {
    let mut state = AppState::default();
    state.hub.windows = Some(crate::view_model::HubWindowsState {
        app_name: "Cursor".into(),
        windows: vec![crate::view_model::HubWindowRow {
            id: "win:pid:1|num:1".into(),
            title: "Luma".into(),
        }],
        more: None,
        status_kind: None,
        status_title: None,
        status_subtitle: None,
    });
    state.hub.selected = 0;
    let effects = update(&mut state, Msg::Submit);
    assert!(state.search.prompt.is_empty());
    assert!(effects.iter().any(|e| matches!(
        e,
        Effect::ExecuteAction {
            action_id,
            result_id,
            ..
        } if action_id == "focus" && result_id == "win:pid:1|num:1"
    )));
    assert!(!effects.iter().any(|e| matches!(e, Effect::LoadHub)));
}

#[test]
fn hub_window_enter_respects_active_operation() {
    let mut state = AppState::default();
    state.actions.active_operation = Some("op-1".into());
    state.hub.windows = Some(crate::view_model::HubWindowsState {
        app_name: "Cursor".into(),
        windows: vec![crate::view_model::HubWindowRow {
            id: "win:pid:1|num:1".into(),
            title: "Luma".into(),
        }],
        more: None,
        status_kind: None,
        status_title: None,
        status_subtitle: None,
    });
    state.hub.selected = 0;
    let effects = update(&mut state, Msg::Submit);
    assert_eq!(effects, vec![Effect::None]);
    assert_eq!(state.actions.active_operation.as_deref(), Some("op-1"));
}

#[test]
fn empty_prompt_schedules_load_hub() {
    let mut state = AppState::default();
    state.search.prompt = "app x".into();
    state.search.prompt_cursor = state.prompt_char_len();
    let effects = update(&mut state, Msg::ClearToStart);
    assert!(state.search.prompt.is_empty());
    assert!(effects.iter().any(|e| matches!(e, Effect::LoadHub)));
}

#[test]
fn hub_more_row_opens_win_trigger() {
    let mut state = AppState::default();
    state.hub.windows = Some(crate::view_model::HubWindowsState {
        app_name: "Cursor".into(),
        windows: vec![crate::view_model::HubWindowRow {
            id: "win:pid:1|num:1".into(),
            title: "Luma".into(),
        }],
        more: Some(3),
        status_kind: None,
        status_title: None,
        status_subtitle: None,
    });
    state.hub.selected = 1; // more row
    let _effects = update(&mut state, Msg::Submit);
    assert_eq!(state.search.prompt, "/win ");
}

#[test]
fn hub_status_row_opens_win() {
    let mut state = AppState::default();
    state.hub.windows = Some(crate::view_model::HubWindowsState {
        app_name: "Windows".into(),
        windows: vec![],
        more: None,
        status_kind: Some("permission_required".into()),
        status_title: Some("Permission required (accessibility)".into()),
        status_subtitle: Some("Grant Accessibility".into()),
    });
    state.hub.selected = 0;
    let _ = update(&mut state, Msg::Submit);
    assert_eq!(state.search.prompt, "/win ");
}

#[test]
fn seed_config_primary_skips_action_picker() {
    use luma_domain::{ActionDescriptor, ActionId, ActionRisk, ModuleId, ResultId, SearchItem};
    let mut state = AppState::default();
    let item = SearchItem {
        id: ResultId::new("notes:configure"),
        module_id: ModuleId::new("luma.notes"),
        title: "Configure".into(),
        subtitle: Some("hint".into()),
        kind: "not_configured".into(),
        score: 0.0,
        primary_action: ActionDescriptor {
            id: ActionId::new("seed_config"),
            label: "Show command".into(),
            risk: ActionRisk::Safe,
            confirmation: false,
        },
        secondary_actions: vec![],
        ui_intent: Some("seed_config".into()),
        action_payload: None,
    };
    state.search.results.items = vec![item.clone()];
    state.search.results.selected_id = Some(item.id.as_str().to_string());
    let effects = request_primary_actions(&mut state);
    assert!(
        effects
            .iter()
            .all(|e| !matches!(e, Effect::ListActions { .. })),
        "seed_config primary should not open action picker: {effects:?}"
    );
    assert!(state.status.text.contains("luma config set"));
}

#[test]
fn projects_without_imports_show_import_guidance() {
    use luma_domain::{ActionDescriptor, ActionId, ActionRisk, ModuleId, ResultId, SearchItem};
    let mut state = AppState::default();
    let item = SearchItem {
        id: ResultId::new("proj:not-configured"),
        module_id: ModuleId::new("luma.projects"),
        title: "No imported projects".into(),
        subtitle: Some("/proj add /path".into()),
        kind: "not_configured".into(),
        score: 0.0,
        primary_action: ActionDescriptor {
            id: ActionId::new("seed_config"),
            label: "Show command".into(),
            risk: ActionRisk::Safe,
            confirmation: false,
        },
        secondary_actions: vec![],
        ui_intent: Some("seed_config".into()),
        action_payload: None,
    };
    state.search.results.items = vec![item.clone()];
    state.search.results.selected_id = Some(item.id.as_str().into());
    let _ = request_primary_actions(&mut state);
    assert!(state.status.text.contains("/proj add"));
    assert!(!state.status.text.contains("--projects-root"));
}

#[test]
fn hub_rows_order_window_then_module() {
    let mut state = AppState::default();
    state.hub.windows = Some(crate::view_model::HubWindowsState {
        app_name: "Cursor".into(),
        windows: vec![crate::view_model::HubWindowRow {
            id: "win:a".into(),
            title: "A".into(),
        }],
        more: None,
        status_kind: None,
        status_title: None,
        status_subtitle: None,
    });
    state.module_catalog = vec![crate::view_model::ModuleCatalogEntry {
        id: "luma.apps".into(),
        display_name: "Apps".into(),
        enabled: true,
        glyph: None,
        suggested_query: Some("app ".into()),
        empty_hint: None,
        supports_browse: false,
        triggers: vec![],
    }];
    let rows = state.hub_rows();
    assert_eq!(rows[0].0, "window");
    assert!(rows.iter().any(|(k, ..)| k == "module"));
    assert!(rows
        .iter()
        .any(|(kind, _, _, query)| { kind == "module" && query == "/app " }));
}

#[test]
fn hub_loaded_clamps_selection() {
    let mut state = AppState::default();
    state.hub.selected = 50;
    state.hub.scroll = 40;
    state.module_catalog = vec![crate::view_model::ModuleCatalogEntry {
        id: "luma.apps".into(),
        display_name: "Apps".into(),
        enabled: true,
        glyph: None,
        suggested_query: Some("app ".into()),
        empty_hint: None,
        supports_browse: false,
        triggers: vec![],
    }];
    let _ = state.apply_engine_event(Event::HubLoaded {
        windows: Some(luma_protocol::HubWindowsDto {
            app_name: "Cursor".into(),
            windows: vec![luma_protocol::HubWindowDto {
                id: "win:a".into(),
                title: "A".into(),
            }],
            more: None,
            status: None,
        }),
    });
    assert!(state.hub.selected < state.hub_rows().len());
    assert!(state.hub.scroll <= state.hub.selected);
}

#[test]
fn refresh_hub_loads_when_showing_hub() {
    let mut state = AppState::default();
    assert!(state.showing_hub());
    let effects = update(&mut state, Msg::RefreshHub);
    assert!(effects.iter().any(|e| matches!(e, Effect::LoadHub)));
    assert!(state.hub.refresh_deadline.is_some());
    assert!(!state.dirty);
}

#[test]
fn focus_gained_reloads_hub() {
    let mut state = AppState::default();
    let effects = update(&mut state, Msg::FocusGained);
    assert!(effects.iter().any(|e| matches!(e, Effect::LoadHub)));
}

#[test]
fn refresh_hub_skips_when_not_on_hub() {
    let mut state = AppState::default();
    state.search.prompt = "app ".into();
    state.hub.refresh_deadline = Some(std::time::Instant::now());
    let effects = update(&mut state, Msg::RefreshHub);
    assert!(!effects.iter().any(|e| matches!(e, Effect::LoadHub)));
    assert!(state.hub.refresh_deadline.is_none());
}

#[test]
fn help_cancel_from_hub_restores_ready_status() {
    let mut state = AppState::default();
    let _ = update(&mut state, Msg::OpenHelp);
    let _ = update(&mut state, Msg::Cancel);
    assert_eq!(state.route, Route::Search);
    assert!(state.showing_hub());
    assert_eq!(state.status.text, "Ready");
}

#[test]
fn esc_clear_prompt_clears_awaiting_actions() {
    let mut state = AppState::default();
    state.search.prompt = "clip ".into();
    state.search.prompt_cursor = state.prompt_char_len();
    state.actions.awaiting_actions = Some(AwaitingActions {
        intent: ActionsIntent::Picker,
        result_id: "clip:1".into(),
    });
    let effects = update(&mut state, Msg::Cancel);
    assert!(effects.iter().any(|e| matches!(e, Effect::LoadHub)));
    assert!(state.actions.awaiting_actions.is_none());
    assert!(state.search.prompt.is_empty());
}

#[test]
fn begin_search_clears_pending_preview() {
    let mut state = AppState::default();
    state.search.prompt = "clip foo".into();
    state.search.prompt_cursor = state.prompt_char_len();
    state.preview.pending_id = Some(7);
    state.preview.result_id = Some("clip:1".into());
    state.preview.body = Some("old".into());
    let _ = begin_search(&mut state);
    assert!(state.preview.pending_id.is_none());
    assert!(state.preview.result_id.is_none());
    assert!(state.preview.body.is_none());
}

#[test]
fn open_help_saves_restore_prompt() {
    let mut state = AppState::default();
    state.search.prompt = "clip foo".into();
    state.search.prompt_cursor = state.prompt_char_len();
    let _ = update(&mut state, Msg::OpenHelp);
    assert_eq!(state.route, Route::Help);
    assert_eq!(state.overlay.restore_prompt.as_deref(), Some("clip foo"));
    let _ = update(&mut state, Msg::Cancel);
    assert_eq!(state.route, Route::Search);
    assert_eq!(state.search.prompt, "clip foo");
    assert!(state.overlay.restore_prompt.is_none());
}

#[test]
fn typing_from_help_discards_restore_prompt() {
    let mut state = AppState::default();
    state.search.prompt = "/help".into();
    state.search.prompt_cursor = state.prompt_char_len();
    let _ = update(&mut state, Msg::Submit);
    assert_eq!(state.route, Route::Help);
    assert_eq!(state.overlay.restore_prompt.as_deref(), Some("/help"));
    let _ = update(&mut state, Msg::KeyChar('x'));
    assert_eq!(state.route, Route::Search);
    assert!(state.overlay.restore_prompt.is_none());
    assert!(state.search.prompt.contains('x'));
}

#[test]
fn backspace_from_help_exits_to_search() {
    let mut state = AppState::default();
    state.search.prompt = "ab".into();
    state.search.prompt_cursor = state.prompt_char_len();
    state.overlay.restore_prompt = Some("ab".into());
    state.route = Route::Help;
    let _ = update(&mut state, Msg::Backspace);
    assert_eq!(state.route, Route::Search);
    assert!(state.overlay.restore_prompt.is_none());
    assert_eq!(state.search.prompt, "a");
}

#[test]
fn project_remove_refresh_keeps_success_feedback_and_clears_row() {
    let mut state = AppState::default();
    state.search.prompt = "/proj remove files".into();
    state.search.prompt_cursor = state.prompt_char_len();
    state.search.results.items.push(SearchItem {
        id: ResultId::new("proj:remove:files"),
        module_id: ModuleId::new("luma.projects"),
        title: "Remove files".into(),
        subtitle: Some("config only".into()),
        kind: "command".into(),
        score: 1.0,
        primary_action: ActionDescriptor {
            id: ActionId::new("remove_project"),
            label: "Remove".into(),
            risk: ActionRisk::Confirm,
            confirmation: true,
        },
        secondary_actions: vec![],
        ui_intent: None,
        action_payload: None,
    });
    state.search.results.selected_id = Some("proj:remove:files".into());
    state.actions.active_operation = Some("op-remove".into());

    let settings = Event::SettingsChanged {
        version: 2,
        settings: serde_json::json!({
            "modules": [],
            "projects_roots": [],
            "imported_projects": []
        }),
    };
    let effects = apply_engine(&mut state, settings);
    assert!(!effects
        .iter()
        .any(|effect| matches!(effect, Effect::Search { .. })));
    assert!(state.search.results.items.is_empty());

    let _ = apply_engine(
        &mut state,
        Event::ActionFinished {
            operation_id: "op-remove".into(),
            outcome: luma_protocol::ActionOutcomeDto::Success {
                message: Some("settings updated".into()),
            },
        },
    );
    assert!(state.status.text.contains("directory kept"));
    assert_eq!(state.status.tone, StatusTone::Success);
}

#[test]
fn win_digit_ignores_non_window_rows() {
    let mut state = AppState::default();
    state.search.prompt = "/win ".into();
    state.search.prompt_cursor = state.prompt_char_len();
    state.focus = FocusZone::List;
    state.search.results.items.push(SearchItem {
        id: ResultId::new("win:status"),
        module_id: ModuleId::new("luma.windows"),
        title: "Permission".into(),
        subtitle: None,
        kind: "permission_required".into(),
        score: 1.0,
        primary_action: ActionDescriptor {
            id: ActionId::new("noop"),
            label: "OK".into(),
            risk: ActionRisk::Safe,
            confirmation: false,
        },
        secondary_actions: vec![],
        ui_intent: None,
        action_payload: None,
    });
    state.search.results.items.push(SearchItem {
        id: ResultId::new("win:a"),
        module_id: ModuleId::new("luma.windows"),
        title: "A".into(),
        subtitle: None,
        kind: "window".into(),
        score: 1.0,
        primary_action: ActionDescriptor {
            id: ActionId::new("focus"),
            label: "Focus".into(),
            risk: ActionRisk::Safe,
            confirmation: false,
        },
        secondary_actions: vec![],
        ui_intent: None,
        action_payload: None,
    });
    let effects = update(&mut state, Msg::PickWindowDigit(1));
    assert!(effects.iter().any(|e| matches!(
        e,
        Effect::ExecuteAction { result_id, .. } if result_id == "win:a"
    )));
}
