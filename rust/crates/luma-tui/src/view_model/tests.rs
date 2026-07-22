use super::*;
use luma_domain::FailureKind;
use luma_protocol::{ActionOutcomeDto, Event, SearchItemDto};

#[test]
fn action_started_ignored_without_active_operation() {
    let mut state = AppState::default();
    let applied = state.apply_engine_event(Event::ActionStarted {
        operation_id: "op-1".into(),
    });
    assert!(!applied);
    assert!(state.actions.active_operation.is_none());
}

#[test]
fn action_started_applies_when_operation_matches() {
    let mut state = AppState {
        actions: ActionsState {
            active_operation: Some("op-1".into()),
            ..ActionsState::default()
        },
        ..AppState::default()
    };
    let applied = state.apply_engine_event(Event::ActionStarted {
        operation_id: "op-1".into(),
    });
    assert!(applied);
    assert_eq!(state.status.text, "Running…");
}

#[test]
fn action_finished_ignored_without_active_operation() {
    let mut state = AppState::default();
    let applied = state.apply_engine_event(Event::ActionFinished {
        operation_id: "op-2".into(),
        outcome: ActionOutcomeDto::Success {
            message: Some("ok".into()),
        },
    });
    assert!(!applied);
}

#[test]
fn action_finished_cancelled_is_warning() {
    let mut state = AppState {
        actions: ActionsState {
            active_operation: Some("op-1".into()),
            ..ActionsState::default()
        },
        ..AppState::default()
    };
    let applied = state.apply_engine_event(Event::ActionFinished {
        operation_id: "op-1".into(),
        outcome: ActionOutcomeDto::Cancelled,
    });
    assert!(applied);
    assert_eq!(state.status.tone, StatusTone::Warning);
    assert!(state.actions.active_operation.is_none());
}

#[test]
fn stale_action_finished_does_not_overwrite_status() {
    let mut state = AppState {
        actions: ActionsState {
            active_operation: Some("op-current".into()),
            ..ActionsState::default()
        },
        ..AppState::default()
    };
    state.status.set("running current", StatusTone::Progress);
    let applied = state.apply_engine_event(Event::ActionFinished {
        operation_id: "op-old".into(),
        outcome: ActionOutcomeDto::Success {
            message: Some("stale ok".into()),
        },
    });
    assert!(!applied);
    assert_eq!(
        state.actions.active_operation.as_deref(),
        Some("op-current")
    );
    assert_eq!(state.status.text, "running current");
}

#[test]
fn action_finished_not_configured_is_warning() {
    let mut state = AppState {
        actions: ActionsState {
            active_operation: Some("op-2".into()),
            ..ActionsState::default()
        },
        ..AppState::default()
    };
    let applied = state.apply_engine_event(Event::ActionFinished {
        operation_id: "op-2".into(),
        outcome: ActionOutcomeDto::failed(FailureKind::NotConfigured {
            remediation: "set notes_root".into(),
        }),
    });
    assert!(applied);
    assert_eq!(state.status.tone, StatusTone::Warning);
    assert!(state.status.text.contains("set notes_root"));
}

#[test]
fn action_finished_unavailable_is_warning() {
    let mut state = AppState {
        actions: ActionsState {
            active_operation: Some("op-3".into()),
            ..ActionsState::default()
        },
        ..AppState::default()
    };
    let applied = state.apply_engine_event(Event::ActionFinished {
        operation_id: "op-3".into(),
        outcome: ActionOutcomeDto::failed(FailureKind::Unavailable {
            reason: "signed host required".into(),
            retryable: false,
        }),
    });
    assert!(applied);
    assert_eq!(state.status.tone, StatusTone::Warning);
    assert!(state.status.text.contains("signed host required"));
}

#[test]
fn action_finished_permission_is_permission_tone() {
    let mut state = AppState {
        actions: ActionsState {
            active_operation: Some("op-4".into()),
            ..ActionsState::default()
        },
        ..AppState::default()
    };
    let applied = state.apply_engine_event(Event::ActionFinished {
        operation_id: "op-4".into(),
        outcome: ActionOutcomeDto::failed(FailureKind::PermissionRequired {
            capability: "accessibility".into(),
            guidance: "Open Settings".into(),
        }),
    });
    assert!(applied);
    assert_eq!(state.status.tone, StatusTone::Permission);
}

#[test]
fn action_finished_success_is_success() {
    let mut state = AppState {
        actions: ActionsState {
            active_operation: Some("op-5".into()),
            ..ActionsState::default()
        },
        ..AppState::default()
    };
    let applied = state.apply_engine_event(Event::ActionFinished {
        operation_id: "op-5".into(),
        outcome: ActionOutcomeDto::Success {
            message: Some("Opened Safari".into()),
        },
    });
    assert!(applied);
    assert_eq!(state.status.tone, StatusTone::Success);
}

#[test]
fn preview_stacked_on_tall_narrow_terminal() {
    use luma_domain::{ActionDescriptor, ActionId, ActionRisk, ModuleId, ResultId, SearchItem};

    let mut state = AppState {
        terminal: TerminalState {
            width: 80,
            height: 28,
        },
        ..AppState::default()
    };
    state.search.results.items.push(SearchItem {
        id: ResultId::new("1"),
        module_id: ModuleId::new("luma.notes"),
        title: "Note".into(),
        subtitle: None,
        kind: "note".into(),
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
    assert!(!state.preview_side_by_side());
    assert!(state.preview_stacked());
    assert!(state.preview_visible());

    state.terminal.height = 24;
    assert!(state.preview_stacked());
    assert!(state.preview_visible());
    state.terminal.height = 23;
    assert!(!state.preview_stacked());
    assert!(!state.preview_visible());
}

#[test]
fn empty_request_id_chunk_evicts_removed_ids() {
    use luma_domain::{ActionDescriptor, ActionId, ActionRisk, ModuleId, ResultId, SearchItem};

    let mut state = AppState {
        search: SearchState {
            active_request: Some("req-1".into()),
            ..SearchState::default()
        },
        ..AppState::default()
    };
    state.search.results.items.push(SearchItem {
        id: ResultId::new("clip:1"),
        module_id: ModuleId::new("luma.clipboard"),
        title: "x".into(),
        subtitle: None,
        kind: "clip".into(),
        score: 1.0,
        primary_action: ActionDescriptor {
            id: ActionId::new("copy"),
            label: "Copy".into(),
            risk: ActionRisk::Safe,
            confirmation: false,
        },
        secondary_actions: vec![],
        ui_intent: None,
        action_payload: None,
    });
    let applied = state.apply_engine_event(Event::ResultsChunk {
        request_id: String::new(),
        sequence: 0,
        upserts: vec![],
        removed_ids: vec!["clip:1".into()],
    });
    assert!(applied);
    assert!(state.search.results.items.is_empty());
}

#[test]
fn module_state_changed_updates_catalog_enabled() {
    let mut state = AppState::default();
    state.module_catalog.push(ModuleCatalogEntry {
        id: "luma.projects".into(),
        display_name: "Projects".into(),
        enabled: true,
        glyph: None,
        suggested_query: None,
        empty_hint: None,
        supports_browse: false,
        triggers: vec![],
    });
    let applied = state.apply_engine_event(Event::ModuleStateChanged {
        module_id: "luma.projects".into(),
        state: "disabled".into(),
    });
    assert!(applied);
    assert!(!state.module_catalog[0].enabled);
}

#[test]
fn hub_row_window_digit_skips_status_more_and_modules() {
    let state = AppState {
        hub: HubState {
            windows: Some(HubWindowsState {
                app_name: "all".into(),
                windows: vec![
                    HubWindowRow {
                        id: "win:1".into(),
                        title: "A".into(),
                    },
                    HubWindowRow {
                        id: "win:2".into(),
                        title: "B".into(),
                    },
                ],
                more: Some(3),
                status_kind: Some("permission_required".into()),
                status_title: Some("grant AX".into()),
                status_subtitle: None,
            }),
            ..HubState::default()
        },
        module_catalog: vec![ModuleCatalogEntry {
            id: "luma.apps".into(),
            display_name: "Apps".into(),
            enabled: true,
            glyph: None,
            suggested_query: Some("app ".into()),
            empty_hint: None,
            supports_browse: false,
            triggers: vec![],
        }],
        ..Default::default()
    };
    let rows = state.hub_rows();
    let status_idx = rows
        .iter()
        .position(|(k, ..)| k == "window_status")
        .unwrap();
    let first_win_idx = rows.iter().position(|(k, ..)| k == "window").unwrap();
    let more_idx = rows.iter().position(|(k, ..)| k == "window_more").unwrap();
    let module_idx = rows.iter().position(|(k, ..)| k == "module").unwrap();
    assert_eq!(state.hub_row_window_digit(status_idx), None);
    assert_eq!(state.hub_row_window_digit(first_win_idx), Some(1));
    assert_eq!(state.hub_row_window_digit(first_win_idx + 1), Some(2));
    assert_eq!(state.hub_row_window_digit(more_idx), None);
    assert_eq!(state.hub_row_window_digit(module_idx), None);
}

#[test]
fn window_digit_targets_follow_scroll_position() {
    let mut state = AppState {
        search: SearchState {
            prompt: "/win ".into(),
            ..SearchState::default()
        },
        focus: FocusZone::List,
        ..Default::default()
    };
    state.search.results.items = (0..20)
        .map(|i| SearchItem {
            id: luma_domain::ResultId::new(format!("win:{i}")),
            module_id: luma_domain::ModuleId::new("luma.windows"),
            title: format!("Window {i}"),
            subtitle: None,
            kind: "window".into(),
            score: 1.0,
            primary_action: luma_domain::ActionDescriptor {
                id: luma_domain::ActionId::new("focus"),
                label: "Focus".into(),
                risk: luma_domain::ActionRisk::Safe,
                confirmation: false,
            },
            secondary_actions: vec![],
            ui_intent: None,
            action_payload: None,
        })
        .collect();
    state.search.results.scroll = 4;
    let targets = state.window_digit_targets();
    assert_eq!(targets.first().map(|(id, _)| id.as_str()), Some("win:4"));
    assert_eq!(targets.get(8).map(|(id, _)| id.as_str()), Some("win:12"));
}

#[test]
fn hub_window_digit_targets_follow_scroll_position() {
    let mut state = AppState {
        hub: HubState {
            windows: Some(HubWindowsState {
                app_name: "all".into(),
                windows: (0..12)
                    .map(|i| HubWindowRow {
                        id: format!("win:{i}"),
                        title: format!("Window {i}"),
                    })
                    .collect(),
                more: None,
                status_kind: None,
                status_title: None,
                status_subtitle: None,
            }),
            ..HubState::default()
        },
        ..Default::default()
    };
    state.hub.scroll = 4;
    assert_eq!(
        state
            .window_digit_targets()
            .first()
            .map(|(id, _)| id.as_str()),
        Some("win:4")
    );
    let rows = state.hub_rows();
    let row_index = rows.iter().position(|(_, id, _, _)| id == "win:4").unwrap();
    assert_eq!(state.hub_row_window_digit(row_index), Some(1));
}

#[test]
fn wordbook_review_loaded_empty_finishes() {
    let mut state = AppState {
        route: Route::WordbookReview,
        ..Default::default()
    };
    let applied = state.apply_engine_event(Event::WordbookReviewLoaded {
        queue: "due".into(),
        words: vec![],
        stats: luma_protocol::WordbookStatsDto {
            due: 0,
            new_count: 0,
            wrong: 0,
            goal: 20,
            reviewed_today: 5,
            remaining_goal: 15,
        },
    });
    assert!(applied);
    let review = state.wordbook.review.as_ref().unwrap();
    assert!(review.finished);
    assert!(state.status.text.contains("empty"));
}

#[test]
fn wordbook_review_stats_updated_refreshes_counters() {
    let mut state = AppState {
        route: Route::WordbookReview,
        wordbook: WordbookState {
            review: Some(WordbookReviewState {
                words: vec![],
                index: 0,
                revealed: false,
                stats: WordbookReviewStats {
                    reviewed_today: 3,
                    remaining_goal: 10,
                    ..Default::default()
                },
                finished: true,
                pending_grade: None,
            }),
        },
        ..Default::default()
    };
    let applied = state.apply_engine_event(Event::WordbookReviewStatsUpdated {
        stats: luma_protocol::WordbookStatsDto {
            due: 5,
            new_count: 2,
            wrong: 1,
            goal: 20,
            reviewed_today: 8,
            remaining_goal: 12,
        },
    });
    assert!(applied);
    let review = state.wordbook.review.as_ref().unwrap();
    assert_eq!(review.stats.reviewed_today, 8);
    assert_eq!(review.stats.remaining_goal, 12);
    assert_eq!(review.stats.due, 5);
}

fn catalog_with_notes_trigger() -> Vec<ModuleCatalogEntry> {
    vec![ModuleCatalogEntry {
        id: "luma.notes".into(),
        display_name: "Notes".into(),
        enabled: true,
        glyph: None,
        suggested_query: Some("/n ".into()),
        empty_hint: None,
        supports_browse: true,
        triggers: vec!["n".into(), "note".into(), "notes".into()],
    }]
}

#[test]
fn bare_n_is_not_incomplete_slash_trigger() {
    let state = AppState {
        search: SearchState {
            prompt: "n".into(),
            ..SearchState::default()
        },
        module_catalog: catalog_with_notes_trigger(),
        ..AppState::default()
    };
    assert!(state.incomplete_slash_trigger().is_none());
}

#[test]
fn slash_n_is_incomplete_slash_trigger() {
    let state = AppState {
        search: SearchState {
            prompt: "/n".into(),
            ..SearchState::default()
        },
        module_catalog: catalog_with_notes_trigger(),
        ..AppState::default()
    };
    assert_eq!(state.incomplete_slash_trigger().as_deref(), Some("n"));
}

#[test]
fn snapshot_loaded_sorts_by_score() {
    let mut state = AppState::default();
    let applied = state.apply_engine_event(Event::SnapshotLoaded {
        items: vec![
            SearchItemDto {
                id: "low".into(),
                module_id: "luma.notes".into(),
                title: "low".into(),
                score: 1.0,
                ..SearchItemDto::default()
            },
            SearchItemDto {
                id: "high".into(),
                module_id: "luma.notes".into(),
                title: "high".into(),
                score: 9.0,
                ..SearchItemDto::default()
            },
        ],
        module_states: Default::default(),
    });
    assert!(applied);
    assert_eq!(state.search.results.items[0].id.as_str(), "high");
    assert_eq!(state.search.results.selected_id.as_deref(), Some("high"));
}

#[test]
fn snapshot_loaded_ignored_during_active_search() {
    let mut state = AppState {
        search: SearchState {
            active_request: Some("req-live".into()),
            ..SearchState::default()
        },
        ..AppState::default()
    };
    state.search.results.items.push(luma_domain::SearchItem {
        id: luma_domain::ResultId::new("keep"),
        module_id: luma_domain::ModuleId::new("luma.notes"),
        title: "keep".into(),
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
    let applied = state.apply_engine_event(Event::SnapshotLoaded {
        items: vec![SearchItemDto {
            id: "stale".into(),
            module_id: "luma.notes".into(),
            title: "stale".into(),
            score: 9.0,
            ..SearchItemDto::default()
        }],
        module_states: Default::default(),
    });
    assert!(!applied);
    assert_eq!(state.search.results.items[0].id.as_str(), "keep");
}

#[test]
fn preview_loaded_clears_pending_on_selection_mismatch() {
    let mut state = AppState {
        preview: PreviewState {
            pending_id: Some(3),
            result_id: Some("note:a".into()),
            ..PreviewState::default()
        },
        search: SearchState {
            results: ResultsView {
                selected_id: Some("note:b".into()),
                ..ResultsView::default()
            },
            ..SearchState::default()
        },
        ..AppState::default()
    };
    let applied = state.apply_engine_event(Event::PreviewLoaded {
        result_id: "note:a".into(),
        preview_id: 3,
        body: "body".into(),
    });
    assert!(!applied);
    assert!(state.preview.pending_id.is_none());
    assert!(state.preview.body.is_none());
}

#[test]
fn search_finished_bare_n_is_not_add_space_hint() {
    let mut state = AppState {
        search: SearchState {
            prompt: "n".into(),
            active_request: Some("req-1".into()),
            ..SearchState::default()
        },
        module_catalog: catalog_with_notes_trigger(),
        ..AppState::default()
    };
    let applied = state.apply_engine_event(Event::SearchFinished {
        request_id: "req-1".into(),
        total: 0,
        elapsed_ms: 12,
    });
    assert!(applied);
    assert_eq!(state.status.text, "No results");
}

#[test]
fn search_finished_slash_n_shows_add_space_hint() {
    let mut state = AppState {
        search: SearchState {
            prompt: "/n".into(),
            active_request: Some("req-1".into()),
            ..SearchState::default()
        },
        module_catalog: catalog_with_notes_trigger(),
        ..AppState::default()
    };
    let applied = state.apply_engine_event(Event::SearchFinished {
        request_id: "req-1".into(),
        total: 0,
        elapsed_ms: 12,
    });
    assert!(applied);
    assert_eq!(state.status.text, "Add space to search");
}
