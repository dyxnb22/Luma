use super::*;
use crate::theme::{Symbols, Theme, ThemeMode};
use crate::view_model::{
    ActionsState, FocusZone, HubState, ResultsView, SearchState, SettingsState, TerminalState,
    WordbookState,
};
use luma_domain::{ActionDescriptor, ActionId, ActionRisk, ModuleId, ResultId, SearchItem};
use ratatui::backend::{Backend, TestBackend};
use ratatui::layout::Position;
use ratatui::style::Modifier;
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
                commands: vec![],
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
    assert!(flat.contains("LUMA"), "brand title missing: {flat}");
    assert!(flat.contains("Safari"), "result title missing: {flat}");
    assert!(flat.contains("Apps"), "module label missing: {flat}");
    assert!(flat.contains("Launch"), "action hint missing: {flat}");
}

#[test]
fn result_refresh_replaces_loading_copy_with_wide_character_subtitle() {
    let backend = TestBackend::new(100, 30);
    let mut terminal = Terminal::new(backend).expect("terminal");
    let mut loading = AppState {
        search: SearchState {
            prompt: "/wb data retention".into(),
            active_request: Some("req-1".into()),
            ..SearchState::default()
        },
        status: crate::view_model::StatusLine {
            text: "Searching…".into(),
            tone: StatusTone::Progress,
        },
        ..AppState::default()
    };
    terminal
        .draw(|frame| render(frame, &loading))
        .expect("loading draw");

    loading.search.results = ResultsView {
        items: vec![sample_item(
            "wordbook:data-retention",
            "data retention",
            "wordbook",
            "数据保留期限",
        )],
        selected_id: Some("wordbook:data-retention".into()),
        ..ResultsView::default()
    };
    let completed = terminal
        .draw(|frame| render(frame, &loading))
        .expect("results draw");

    // Inspect Ratatui's frame, not TestBackend's text grid. A real terminal advances two cells
    // when it receives a CJK glyph; TestBackend intentionally stores it in one cell only.
    let buffer = completed.buffer;
    let subtitle_y = (0..buffer.area.height)
        .find(|&y| (0..buffer.area.width).any(|x| buffer[(x, y)].symbol() == "数"))
        .expect("subtitle row");
    let subtitle_x = (0..buffer.area.width)
        .find(|&x| buffer[(x, subtitle_y)].symbol() == "数")
        .expect("subtitle start");
    for (offset, character) in "数据保留期限".chars().enumerate() {
        let x = subtitle_x + (offset as u16 * 2);
        assert_eq!(buffer[(x, subtitle_y)].symbol(), character.to_string());
        assert_eq!(
            buffer[(x + 1, subtitle_y)].symbol(),
            " ",
            "wide character at x={x} must reserve a blank trailing terminal cell"
        );
    }
}

#[test]
fn render_uses_layered_canvas_and_surface_backgrounds() {
    let state = state_with_results();
    let (_, buffer) = draw(&state, 80, 24);
    assert_eq!(buffer[(0, 3)].bg, state.theme.canvas_bg);
    assert_eq!(buffer[(2, 1)].bg, state.theme.surface_bg);
    assert_ne!(state.theme.canvas_bg, state.theme.surface_bg);
}

#[test]
fn overlay_panel_clears_underlying_glyphs_before_applying_its_background() {
    let theme = Theme::dark();
    let backend = TestBackend::new(40, 12);
    let mut terminal = Terminal::new(backend).expect("terminal");
    let panel = Rect::new(5, 3, 20, 6);
    let completed = terminal
        .draw(|frame| {
            frame.render_widget(
                Paragraph::new(vec![Line::from("LEAK".repeat(10)); 12]),
                frame.area(),
            );
            fill_overlay_panel(frame, panel, &theme);
        })
        .expect("draw");

    for y in panel.top()..panel.bottom() {
        for x in panel.left()..panel.right() {
            assert_eq!(
                completed.buffer[(x, y)].symbol(),
                " ",
                "underlying glyph survived at ({x}, {y})"
            );
            assert_eq!(completed.buffer[(x, y)].bg, theme.panel_bg);
        }
    }
}

#[test]
fn selected_result_band_fills_the_panel_width() {
    let state = state_with_results();
    let (_, buffer) = draw(&state, 80, 24);
    let selected_y = (0..buffer.area.height)
        .find(|&y| {
            (0..buffer.area.width).any(|x| buffer[(x, y)].symbol() == state.symbols.selected)
        })
        .expect("selected marker");
    for x in 2..78 {
        assert_eq!(
            buffer[(x, selected_y)].bg,
            state.theme.selected_bg,
            "selected band ended before x={x}"
        );
    }
}

#[test]
fn prompt_exposes_global_search_and_command_modes() {
    let empty = AppState::default();
    let (flat, _) = draw(&empty, 80, 24);
    assert!(flat.contains("GLOBAL SEARCH"));
    assert!(flat.contains("INPUT"));
    assert!(flat.contains("Search everything or type / for commands"));

    let mut command = state_with_results();
    command.search.prompt = "/app saf".into();
    let (flat, _) = draw(&command, 80, 24);
    assert!(flat.contains("COMMAND"));
    assert!(!flat.contains("GLOBAL SEARCH"));
}

#[test]
fn prompt_explicitly_shows_when_tab_must_restore_input_focus() {
    let mut state = state_with_results();
    state.focus = FocusZone::List;
    let (flat, _) = draw(&state, 80, 24);
    assert!(flat.contains("TAB TO INPUT"), "{flat}");
}

#[test]
fn prompt_renders_cjk_emoji_and_combining_graphemes_from_the_first_cell() {
    let mut state = AppState::default();
    state.search.prompt = "数据🙂e\u{301}".into();
    state.search.prompt_cursor = state.prompt_char_len();
    let (_, buffer) = draw(&state, 80, 24);

    for grapheme in ["数", "据", "🙂", "e\u{301}"] {
        assert!(
            buffer.content.iter().any(|cell| cell.symbol() == grapheme),
            "prompt omitted grapheme {grapheme:?}"
        );
    }
}

#[test]
fn prompt_places_the_real_terminal_cursor_at_the_grapheme_cursor() {
    let mut state = AppState::default();
    state.search.prompt = "数据🙂e\u{301}".into();
    state.search.prompt_cursor = 3;
    state.search.prompt_scroll = 1;

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("terminal");
    terminal.draw(|frame| render(frame, &state)).expect("draw");

    // Workspace x=1, prompt border=1, " ⌕ "=3, visible "据🙂"=4.
    assert_eq!(
        terminal
            .backend_mut()
            .get_cursor_position()
            .expect("cursor position"),
        Position::new(9, 1)
    );
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
    assert!(flat.contains("LUMA"));
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
    assert!(
        !flat.contains('╭'),
        "ASCII mode should avoid rounded chrome"
    );
}

#[test]
fn command_palette_renders_syntax_placeholders_not_seed_query() {
    let state = AppState {
        route: Route::Commands,
        module_catalog: vec![crate::view_model::ModuleCatalogEntry {
            id: "luma.projects".into(),
            display_name: "Projects".into(),
            enabled: true,
            glyph: Some("P".into()),
            suggested_query: Some("/proj ".into()),
            empty_hint: None,
            supports_browse: true,
            triggers: vec!["proj".into()],
            commands: vec![crate::view_model::CommandCatalogEntry {
                syntax: "/proj add <path>".into(),
                description: "Import an existing project".into(),
                query: "/proj add ".into(),
                example: Some("/proj add ~/Code/luma".into()),
            }],
        }],
        ..AppState::default()
    };
    let (flat, _) = draw(&state, 100, 30);
    assert!(
        flat.contains("/proj add <path>"),
        "parameter placeholder missing: {flat}"
    );
    assert!(flat.contains("Develop"), "task group missing: {flat}");
}

#[test]
fn action_picker_renders_number_shortcuts_next_to_actions() {
    let state = AppState {
        route: Route::ActionPicker,
        actions: ActionsState {
            action_result_id: Some("1".into()),
            action_choices: vec![
                luma_protocol::ActionDescriptorDto {
                    id: "open".into(),
                    label: "Open".into(),
                    risk: ActionRisk::Safe,
                    confirmation: false,
                },
                luma_protocol::ActionDescriptorDto {
                    id: "copy".into(),
                    label: "Copy".into(),
                    risk: ActionRisk::Safe,
                    confirmation: false,
                },
            ],
            ..ActionsState::default()
        },
        search: SearchState {
            results: ResultsView {
                items: vec![sample_item("1", "Safari", "apps", "browser")],
                selected_id: Some("1".into()),
                ..ResultsView::default()
            },
            ..SearchState::default()
        },
        ..AppState::default()
    };
    let (flat, _) = draw(&state, 80, 24);
    assert!(flat.contains("[1] Open"), "{flat}");
    assert!(flat.contains("[2] Copy"), "{flat}");
}

#[test]
fn help_overlay_keeps_module_syntax_discoverable_when_scrolled() {
    let mut state = AppState {
        route: Route::Help,
        terminal: TerminalState {
            width: 100,
            height: 30,
        },
        module_catalog: vec![crate::view_model::ModuleCatalogEntry {
            id: "luma.projects".into(),
            display_name: "Projects".into(),
            enabled: true,
            glyph: Some("P".into()),
            suggested_query: Some("/proj ".into()),
            empty_hint: None,
            supports_browse: true,
            triggers: vec!["proj".into()],
            commands: vec![crate::view_model::CommandCatalogEntry {
                syntax: "/proj add <path>".into(),
                description: "Import an existing project".into(),
                query: "/proj add ".into(),
                example: Some("/proj add ~/Code/luma".into()),
            }],
        }],
        ..AppState::default()
    };
    state.overlay.help_scroll = state.help_scroll_max();
    let (flat, _) = draw(&state, 100, 30);
    assert!(flat.contains("HELP"));
    assert!(flat.contains("Enabled module commands:"));
    assert!(flat.contains("/proj add <path>"));
    assert!(flat.contains("Fn+↑/↓ page"));
}

#[test]
fn compact_terminal_keeps_every_overlay_inside_the_frame() {
    for route in [
        Route::Help,
        Route::Settings,
        Route::Commands,
        Route::QuitConfirm,
        Route::ConfirmAction,
        Route::ActionPicker,
    ] {
        let state = AppState {
            route,
            terminal: TerminalState {
                width: 28,
                height: 8,
            },
            ..AppState::default()
        };
        let (_, buffer) = draw(&state, 28, 8);
        assert_eq!(buffer.area.width, 28);
        assert_eq!(buffer.area.height, 8);
    }
}

#[test]
fn render_match_highlight_requires_underline_on_query() {
    let state = state_with_results();
    let accent = state.theme.accent;
    let (_, buffer) = draw(&state, 80, 24);
    let mut found_underline = false;
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            let cell = &buffer[(x, y)];
            if matches!(cell.symbol(), "S" | "a" | "f")
                && cell.modifier.contains(Modifier::UNDERLINED)
                && cell.fg == accent
            {
                found_underline = true;
            }
        }
    }
    assert!(
        found_underline,
        "expected underlined accent match cells for query 'saf'"
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
                    "Choose a project folder",
                    "luma.projects",
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
fn empty_preview_explains_tab_and_shift_tab_correctly() {
    let mut state = state_with_results();
    state.terminal = TerminalState {
        width: 120,
        height: 40,
    };
    state.search.results.selected_id = None;

    let (flat, _) = draw(&state, 120, 40);

    assert!(flat.contains("Tab focuses"), "{flat}");
    assert!(flat.contains("Shift-Tab toggles preview"), "{flat}");
    assert!(!flat.contains("Shift-Tab moves focus"), "{flat}");
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
        flat.contains("WORDBOOK REVIEW"),
        "review body missing: {flat}"
    );
    assert!(
        !flat.contains(" PREVIEW "),
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
    let error = state.theme.error;
    let (_, buffer) = draw(&state, 80, 24);
    let mut saw_error = false;
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            let cell = &buffer[(x, y)];
            if cell.symbol() == "E" && cell.fg == error {
                saw_error = true;
            }
        }
    }
    assert!(saw_error, "expected semantic error-color status cells");
}

#[test]
fn hub_window_rows_show_digit_hints() {
    let state = AppState {
        focus: FocusZone::List,
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
fn hub_prompt_focus_keeps_digits_available_for_search() {
    let state = AppState {
        hub: HubState {
            windows: Some(crate::view_model::HubWindowsState {
                app_name: "all".into(),
                windows: vec![crate::view_model::HubWindowRow {
                    id: "win:1".into(),
                    title: "Alpha".into(),
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
    let (flat, _) = draw(&state, 80, 24);
    assert!(flat.contains("type to search"), "{flat}");
    assert!(!flat.contains("[1]"), "{flat}");
    assert!(!flat.contains("1-9 direct"), "{flat}");
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
