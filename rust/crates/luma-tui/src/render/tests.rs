use super::*;
use crate::theme::{Symbols, Theme, ThemeMode};
use crate::view_model::{
    ActionsState, HubState, ResultsView, SearchState, SettingsState, TerminalState, WordbookState,
};
use luma_domain::{ActionDescriptor, ActionId, ActionRisk, ModuleId, ResultId, SearchItem};
use ratatui::backend::TestBackend;
use ratatui::style::{Color, Modifier};
use ratatui::Terminal;

fn sample_item(id: &str, title: &str, module: &str, subtitle: &str) -> SearchItem {
    SearchItem {
        id: ResultId::new(id),
        module_id: ModuleId::new(module),
        title: title.into(),
        subtitle: Some(subtitle.into()),
        kind: "app".into(),
        score: 10.0,
        primary_action: ActionDescriptor {
            id: ActionId::new("launch"),
            label: "Launch".into(),
            risk: ActionRisk::Safe,
            confirmation: false,
        },
        secondary_actions: vec![],
        ui_intent: None,
        action_payload: None,
    }
}

fn sample_kind(
    id: &str,
    title: &str,
    module: &str,
    kind: &str,
    subtitle: &str,
    action: &str,
) -> SearchItem {
    SearchItem {
        id: ResultId::new(id),
        module_id: ModuleId::new(module),
        title: title.into(),
        subtitle: Some(subtitle.into()),
        kind: kind.into(),
        score: 1.0,
        primary_action: ActionDescriptor {
            id: ActionId::new("act"),
            label: action.into(),
            risk: ActionRisk::Safe,
            confirmation: false,
        },
        secondary_actions: vec![],
        ui_intent: None,
        action_payload: None,
    }
}

fn state_with_results() -> AppState {
    AppState {
        theme: Theme::dark(),
        symbols: Symbols::unicode(),
        search: SearchState {
            prompt: "app saf".into(),
            results: ResultsView {
                items: vec![
                    sample_item("1", "Safari", "apps", "/Applications/Safari.app"),
                    sample_item(
                        "2",
                        "Safari Technology Preview",
                        "apps",
                        "/Applications/Safari Technology Preview.app",
                    ),
                ],
                selected_id: Some("1".into()),
                ..Default::default()
            },
            ..SearchState::default()
        },
        status: crate::view_model::StatusLine {
            text: "2 results".into(),
            tone: StatusTone::Success,
        },
        ..AppState::default()
    }
}

fn buffer_flat(buffer: &ratatui::buffer::Buffer) -> String {
    let mut out = String::with_capacity((buffer.area.width * buffer.area.height) as usize);
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            out.push(buffer[(x, y)].symbol().chars().next().unwrap_or(' '));
        }
        out.push('\n');
    }
    out
}

fn draw(state: &AppState, w: u16, h: u16) -> (String, ratatui::buffer::Buffer) {
    let backend = TestBackend::new(w, h);
    let mut terminal = Terminal::new(backend).expect("terminal");
    terminal.draw(|f| render(f, state)).expect("draw");
    let buffer = terminal.backend().buffer().clone();
    (buffer_flat(&buffer), buffer)
}

#[test]
fn hub_layout_80x24_last_row_visible() {
    let state = AppState {
        module_catalog: (0..12)
            .map(|i| crate::view_model::ModuleCatalogEntry {
                id: format!("luma.mod{i}"),
                display_name: format!("Module {i}"),
                enabled: true,
                glyph: None,
                suggested_query: Some(format!("m{i} ")),
                empty_hint: None,
                supports_browse: false,
                triggers: vec![],
            })
            .collect(),
        hub: HubState {
            windows: Some(crate::view_model::HubWindowsState {
                app_name: "Cursor".into(),
                windows: vec![crate::view_model::HubWindowRow {
                    id: "win:a".into(),
                    title: "Editor".into(),
                }],
                more: None,
                status_kind: None,
                status_title: None,
                status_subtitle: None,
            }),
            ..HubState::default()
        },
        ..AppState::default()
    };
    let (flat, buffer) = draw(&state, 80, 24);
    assert_eq!(buffer.area.height, 24);
    let last_row: String = (0..buffer.area.width)
        .map(|x| buffer[(x, 23)].symbol().chars().next().unwrap_or(' '))
        .collect();
    assert!(
        last_row.contains("Enter") || flat.contains("Enter open"),
        "hub status hints should appear on last row: {last_row:?}"
    );
}

#[test]
fn render_search_80x24_smoke() {
    let (flat, _) = draw(&state_with_results(), 80, 24);
    assert!(flat.contains("Luma"), "brand title missing: {flat}");
    assert!(flat.contains("Safari"), "result title missing: {flat}");
    assert!(flat.contains("Apps"), "module label missing: {flat}");
    assert!(flat.contains("Launch"), "action hint missing: {flat}");
}

#[test]
fn footer_says_run_when_results_present_and_list_focused() {
    let mut state = state_with_results();
    state.focus = crate::view_model::FocusZone::List;
    let (flat, _) = draw(&state, 100, 30);
    assert!(
        flat.contains("Enter run") || flat.contains("run"),
        "expected Enter run in footer: {flat}"
    );
    assert!(
        !flat.contains("Enter search"),
        "should not say Enter search with list results: {flat}"
    );
}

#[test]
fn render_search_light_80x24() {
    let mut state = state_with_results();
    state.theme = Theme::resolve(ThemeMode::Light);
    let (flat, _) = draw(&state, 80, 24);
    assert!(flat.contains("Luma"));
    assert!(flat.contains("Safari"));
}

#[test]
fn render_ascii_symbols_fallback() {
    let mut state = state_with_results();
    state.symbols = Symbols::ascii();
    let (flat, _) = draw(&state, 80, 24);
    assert!(flat.contains('>'), "ascii selected marker missing: {flat}");
    assert!(flat.contains("Ret"), "ascii enter hint missing: {flat}");
    assert!(!flat.contains('›'));
    assert!(!flat.contains('↵'));
}

#[test]
fn render_match_highlight_requires_underline_on_query() {
    let state = state_with_results();
    let (_, buffer) = draw(&state, 80, 24);
    let mut found_underline = false;
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            let cell = &buffer[(x, y)];
            if matches!(cell.symbol(), "S" | "a" | "f")
                && cell.modifier.contains(Modifier::UNDERLINED)
                && cell.fg == Color::Cyan
            {
                found_underline = true;
            }
        }
    }
    assert!(
        found_underline,
        "expected underlined cyan match cells for query 'saf'"
    );
}

#[test]
fn render_kind_badge_permission_visible() {
    let state = AppState {
        theme: Theme::dark(),
        symbols: Symbols::unicode(),
        search: SearchState {
            results: ResultsView {
                items: vec![sample_kind(
                    "p",
                    "Accessibility permission required",
                    "luma.clipboard",
                    "permission",
                    "Open System Settings",
                    "Open Settings",
                )],
                selected_id: Some("p".into()),
                ..Default::default()
            },
            ..SearchState::default()
        },
        ..AppState::default()
    };
    let (flat, _) = draw(&state, 80, 24);
    assert!(
        flat.contains("permission"),
        "permission badge missing: {flat}"
    );
}

#[test]
fn render_kind_badge_warming_visible() {
    let state = AppState {
        theme: Theme::dark(),
        symbols: Symbols::unicode(),
        search: SearchState {
            results: ResultsView {
                items: vec![sample_kind(
                    "w",
                    "App index warming",
                    "luma.apps",
                    "warming",
                    "cache refresh",
                    "Wait",
                )],
                selected_id: Some("w".into()),
                ..Default::default()
            },
            ..SearchState::default()
        },
        ..AppState::default()
    };
    let (flat, _) = draw(&state, 80, 24);
    assert!(flat.contains("loading"), "loading badge missing: {flat}");
}

#[test]
fn render_kind_badge_unavailable_visible() {
    let state = AppState {
        theme: Theme::dark(),
        symbols: Symbols::unicode(),
        search: SearchState {
            results: ResultsView {
                items: vec![sample_kind(
                    "u",
                    "Feature is unavailable",
                    "luma.example",
                    "unavailable",
                    "Not available locally",
                    "Details",
                )],
                selected_id: Some("u".into()),
                ..Default::default()
            },
            ..SearchState::default()
        },
        ..AppState::default()
    };
    let (flat, _) = draw(&state, 80, 24);
    assert!(
        flat.contains("unavailable"),
        "unavailable badge missing: {flat}"
    );
}

#[test]
fn render_kind_badge_not_configured_visible() {
    let state = AppState {
        theme: Theme::dark(),
        symbols: Symbols::unicode(),
        search: SearchState {
            results: ResultsView {
                items: vec![sample_kind(
                    "c",
                    "Choose a Notes root folder",
                    "luma.notes",
                    "not_configured",
                    "NotConfigured",
                    "Configure",
                )],
                selected_id: Some("c".into()),
                ..Default::default()
            },
            ..SearchState::default()
        },
        ..AppState::default()
    };
    let (flat, _) = draw(&state, 80, 24);
    assert!(flat.contains("setup"), "setup badge missing: {flat}");
}

#[test]
fn render_search_120x40_scroll_cue() {
    let mut items = Vec::new();
    for i in 0..30 {
        items.push(sample_item(
            &format!("extra-{i}"),
            &format!("Safari Extra {i}"),
            "luma.apps",
            "/Applications/Extra.app",
        ));
    }
    let mut state = AppState {
        theme: Theme::dark(),
        symbols: Symbols::unicode(),
        search: SearchState {
            prompt: "app saf".into(),
            results: ResultsView {
                selected_id: Some("extra-20".into()),
                items,
                ..Default::default()
            },
            ..SearchState::default()
        },
        terminal: TerminalState {
            width: 120,
            height: 40,
        },
        ..AppState::default()
    };
    state.sync_results_viewport();
    state.search.results.ensure_selection_visible();
    let (flat, _) = draw(&state, 120, 40);
    assert!(
        flat.contains('↑') || flat.contains('↓'),
        "scroll cue missing: {flat}"
    );
}

#[test]
fn truncate_uses_display_width_for_cjk() {
    let symbols = Symbols::unicode();
    let out = truncate("中文标题测试", 6, &symbols);
    assert!(display_width(&out) <= 6, "width overflow: {out}");
    assert!(out.contains('…') || out.ends_with('…'));
}

#[test]
fn highlight_query_skips_module_trigger() {
    assert_eq!(highlight_query("app saf"), "saf");
    assert_eq!(highlight_query("safari"), "safari");
}

#[test]
fn render_confirm_overlay_shows_target() {
    use crate::view_model::PendingAction;
    use luma_protocol::ActionDescriptorDto;

    let mut state = state_with_results();
    state.route = Route::ConfirmAction;
    state.actions.pending_action = Some(PendingAction {
        result_id: "1".into(),
        action: ActionDescriptorDto {
            id: "quit".into(),
            label: "Force Quit".into(),
            risk: ActionRisk::Destructive,
            confirmation: true,
        },
    });
    let (flat, _) = draw(&state, 80, 24);
    assert!(flat.contains("DESTRUCTIVE") || flat.contains("Force Quit"));
    assert!(flat.contains("Safari"));
}

#[test]
fn render_wordbook_progress_and_summary_are_consistent() {
    let mut state = AppState {
        route: Route::WordbookReview,
        wordbook: WordbookState {
            review: Some(crate::view_model::WordbookReviewState {
                words: vec![
                    crate::view_model::WordbookReviewWord {
                        id: 1,
                        term: "alpha".into(),
                        phonetic: String::new(),
                        meaning: "first".into(),
                        example: String::new(),
                    },
                    crate::view_model::WordbookReviewWord {
                        id: 2,
                        term: "beta".into(),
                        phonetic: String::new(),
                        meaning: "second".into(),
                        example: String::new(),
                    },
                ],
                index: 2,
                revealed: false,
                stats: crate::view_model::WordbookReviewStats {
                    queue: "due".into(),
                    due: 0,
                    goal: 20,
                    reviewed_today: 12,
                    remaining_goal: 8,
                    session_known: 1,
                    session_fuzzy: 0,
                    session_unknown: 0,
                    session_skipped: 0,
                    session_mastered: 1,
                    ..Default::default()
                },
                finished: true,
                pending_grade: None,
            }),
        },
        ..AppState::default()
    };
    state.terminal.width = 80;
    state.terminal.height = 24;
    let (flat, _) = draw(&state, 80, 24);
    assert!(flat.contains("2/2"), "completed progress missing: {flat}");
    assert!(!flat.contains("3/2"), "progress overflowed: {flat}");
    assert!(flat.contains("Mastered 1"), "mastered stat missing: {flat}");
    assert!(flat.contains("today 12"), "today stat missing: {flat}");

    state.wordbook.review.as_mut().unwrap().finished = false;
    state.wordbook.review.as_mut().unwrap().index = 0;
    let (flat, _) = draw(&state, 80, 24);
    assert!(flat.contains("1/2"), "current progress missing: {flat}");
}

#[test]
fn render_wordbook_confirm_shows_current_word() {
    let state = AppState {
        route: Route::ConfirmAction,
        wordbook: WordbookState {
            review: Some(crate::view_model::WordbookReviewState {
                words: vec![crate::view_model::WordbookReviewWord {
                    id: 42,
                    term: "ephemeral".into(),
                    phonetic: String::new(),
                    meaning: "short-lived".into(),
                    example: String::new(),
                }],
                index: 0,
                revealed: true,
                stats: Default::default(),
                finished: false,
                pending_grade: Some("mastered".into()),
            }),
        },
        actions: ActionsState {
            pending_action: Some(crate::view_model::PendingAction {
                result_id: "wb:42".into(),
                action: luma_protocol::ActionDescriptorDto {
                    id: "mastered".into(),
                    label: "mastered".into(),
                    risk: ActionRisk::Confirm,
                    confirmation: true,
                },
            }),
            ..ActionsState::default()
        },
        ..AppState::default()
    };
    let (flat, _) = draw(&state, 80, 24);
    assert!(
        flat.contains("Target: ephemeral"),
        "word target missing: {flat}"
    );
}

#[test]
fn wide_review_hides_search_preview() {
    let mut state = AppState {
        route: Route::WordbookReview,
        wordbook: WordbookState {
            review: Some(crate::view_model::WordbookReviewState {
                words: vec![crate::view_model::WordbookReviewWord {
                    id: 1,
                    term: "alpha".into(),
                    phonetic: String::new(),
                    meaning: "first".into(),
                    example: String::new(),
                }],
                index: 0,
                revealed: false,
                stats: Default::default(),
                finished: false,
                pending_grade: None,
            }),
        },
        search: SearchState {
            results: ResultsView {
                items: vec![sample_item("1", "Preview result", "apps", "body")],
                selected_id: Some("1".into()),
                ..Default::default()
            },
            ..SearchState::default()
        },
        ..AppState::default()
    };
    state.terminal.width = 120;
    let (flat, _) = draw(&state, 120, 40);
    assert!(
        flat.contains("wordbook review"),
        "review body missing: {flat}"
    );
    assert!(
        !flat.contains(" preview "),
        "search preview leaked into review: {flat}"
    );

    state.terminal.width = 43;
    let (flat, _) = draw(&state, 43, 20);
    assert!(flat.contains("1/2/3"), "narrow grade hint missing: {flat}");
    assert!(flat.contains("Esc"), "narrow exit hint missing: {flat}");

    state.wordbook.review.as_mut().unwrap().finished = true;
    let (flat, _) = draw(&state, 43, 20);
    assert!(flat.contains("done"), "narrow done status missing: {flat}");
    assert!(
        flat.contains("Esc back"),
        "narrow done hint missing: {flat}"
    );
}

#[test]
fn settings_overlay_keeps_selected_module_visible() {
    let state = AppState {
        route: Route::Settings,
        settings: SettingsState {
            selected: 24,
            modules: (0..30)
                .map(|i| crate::view_model::SettingsModuleRow {
                    id: format!("luma.module{i}"),
                    name: format!("Module {i}"),
                    enabled: true,
                })
                .collect(),
            ..SettingsState::default()
        },
        ..AppState::default()
    };
    let (flat, _) = draw(&state, 80, 24);
    assert!(
        flat.contains("Module 24"),
        "selected module not visible: {flat}"
    );
}

#[test]
fn render_fatal_status_uses_error_color() {
    let mut state = state_with_results();
    state.status.set("Error: boom", StatusTone::Error);
    let (_, buffer) = draw(&state, 80, 24);
    let mut saw_error = false;
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            let cell = &buffer[(x, y)];
            if cell.symbol() == "E" && cell.fg == Color::Red {
                saw_error = true;
            }
        }
    }
    assert!(saw_error, "expected red error status cells");
}

#[test]
fn hub_window_rows_show_digit_hints() {
    let state = AppState {
        hub: HubState {
            windows: Some(crate::view_model::HubWindowsState {
                app_name: "all".into(),
                windows: vec![
                    crate::view_model::HubWindowRow {
                        id: "win:1".into(),
                        title: "Alpha".into(),
                    },
                    crate::view_model::HubWindowRow {
                        id: "win:2".into(),
                        title: "Beta".into(),
                    },
                ],
                more: None,
                status_kind: Some("permission_required".into()),
                status_title: Some("grant AX".into()),
                status_subtitle: None,
            }),
            ..HubState::default()
        },
        ..AppState::default()
    };
    let (flat, _) = draw(&state, 80, 24);
    assert!(flat.contains("[1]"), "first window should show [1]: {flat}");
    assert!(
        flat.contains("[2]"),
        "second window should show [2]: {flat}"
    );
    assert!(
        !flat.contains("grant AX[1]"),
        "status row must not be numbered"
    );
}

#[test]
fn win_search_window_rows_show_digit_hints() {
    let state = AppState {
        search: SearchState {
            prompt: "/win ".into(),
            results: crate::view_model::ResultsView {
                items: vec![
                    luma_domain::SearchItem {
                        id: luma_domain::ResultId::new("win:status"),
                        module_id: luma_domain::ModuleId::new("luma.windows"),
                        title: "Permission".into(),
                        subtitle: None,
                        kind: "permission_required".into(),
                        score: 1.0,
                        primary_action: luma_domain::ActionDescriptor {
                            id: luma_domain::ActionId::new("noop"),
                            label: "OK".into(),
                            risk: luma_domain::ActionRisk::Safe,
                            confirmation: false,
                        },
                        secondary_actions: vec![],
                        ui_intent: None,
                        action_payload: None,
                    },
                    luma_domain::SearchItem {
                        id: luma_domain::ResultId::new("win:a"),
                        module_id: luma_domain::ModuleId::new("luma.windows"),
                        title: "Alpha".into(),
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
                    },
                ],
                selected_id: Some("win:a".into()),
                ..Default::default()
            },
            ..SearchState::default()
        },
        focus: crate::view_model::FocusZone::List,
        ..AppState::default()
    };
    let (flat, _) = draw(&state, 80, 24);
    assert!(flat.contains("[1]"), "window row should show [1]: {flat}");
}
