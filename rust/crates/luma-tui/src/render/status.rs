use super::util::{display_width, truncate};
use crate::theme::{Symbols, Theme};
use crate::view_model::{AppState, FocusZone, Route, StatusTone};
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

type Hint = (String, &'static str);

pub(super) fn render_status(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &AppState,
    theme: &Theme,
    symbols: &Symbols,
) {
    let width = area.width as usize;
    let narrow = width < 60;
    let hints = contextual_hints(state, symbols);
    let hints_budget = (width.saturating_mul(2) / 3).clamp(12, 78);
    let (hint_spans, hints_width) = styled_hints(&hints, hints_budget, theme, symbols);

    let status_text = compact_status(state, narrow);
    let status_budget = width.saturating_sub(hints_width + 5).max(6);
    let status_text = truncate(status_text, status_budget, symbols);
    let status_width = display_width(&status_text) + 3;
    let gap = width.saturating_sub(status_width + hints_width).max(1);
    let background = theme.surface_alt();
    let tone = status_style(state.status.tone, theme).bg(theme.surface_alt_bg);

    let mut spans = vec![
        Span::styled(" ", background),
        Span::styled(symbols.status, tone),
        Span::styled(" ", background),
        Span::styled(status_text, tone),
        Span::styled(" ".repeat(gap), background),
    ];
    spans.extend(hint_spans);
    let used = status_width + gap + hints_width;
    if used < width {
        spans.push(Span::styled(" ".repeat(width - used), background));
    }

    frame.render_widget(Paragraph::new(Line::from(spans)).style(background), area);
}

fn compact_status(state: &AppState, narrow: bool) -> &str {
    if narrow && state.route == Route::WordbookReview {
        if state.wordbook.review.as_ref().is_some_and(|r| r.finished) {
            "done"
        } else {
            "review"
        }
    } else if narrow && state.status.text.starts_with("removed ") {
        "removed · dir kept"
    } else {
        state.status.text.as_str()
    }
}

fn contextual_hints(state: &AppState, symbols: &Symbols) -> Vec<Hint> {
    let arrows = format!("{}{}", symbols.up, symbols.down);
    match state.route {
        Route::Search if state.showing_hub() => vec![
            ("1-9".into(), "focus"),
            (arrows, "move"),
            ("Fn+↑/↓".into(), "page"),
            ("Enter".into(), "open"),
            ("Ctrl-/".into(), "commands"),
        ],
        Route::Search if state.focus == FocusZone::Preview => vec![
            ("Fn+↑/↓".into(), "scroll"),
            ("Tab".into(), "focus"),
            ("Esc".into(), "back"),
        ],
        Route::Search if state.is_win_search() && state.focus == FocusZone::List => vec![
            ("1-9".into(), "focus"),
            (arrows, "move"),
            ("Enter".into(), "open"),
            ("Ctrl-k".into(), "actions"),
            ("Tab".into(), "focus"),
        ],
        Route::Search
            if !state.search.results.items.is_empty() && state.focus == FocusZone::List =>
        {
            vec![
                (arrows, "move"),
                ("Enter".into(), "run"),
                ("Ctrl-k".into(), "actions"),
                ("S-Tab".into(), "preview"),
                ("?".into(), "help"),
            ]
        }
        Route::Search => vec![
            (arrows, "move"),
            ("Enter".into(), "search"),
            ("Ctrl-k".into(), "actions"),
            ("Tab".into(), "focus"),
            ("S-Tab".into(), "preview"),
            ("?".into(), "help"),
        ],
        Route::ActionPicker => vec![
            (arrows, "move"),
            ("Fn+↑/↓".into(), "page"),
            ("1-9".into(), "pick"),
            ("Enter".into(), "run"),
            ("Esc".into(), "back"),
        ],
        Route::Settings => vec![
            (arrows, "move"),
            ("Fn+↑/↓".into(), "page"),
            ("Space".into(), "toggle"),
            ("Esc".into(), "back"),
        ],
        Route::Commands => vec![
            ("Type".into(), "filter"),
            ("Fn+↑/↓".into(), "page"),
            ("Enter".into(), "run"),
            ("Esc".into(), "back"),
        ],
        Route::ConfirmAction | Route::QuitConfirm => {
            vec![("Enter".into(), "confirm"), ("Esc".into(), "cancel")]
        }
        Route::Help => vec![
            (arrows, "scroll"),
            ("Fn+↑/↓".into(), "page"),
            ("Esc".into(), "back"),
        ],
        Route::WordbookReview => {
            if state.wordbook.review.as_ref().is_some_and(|r| r.finished) {
                vec![("Esc".into(), "back")]
            } else {
                vec![
                    ("1/2/3".into(), "grade"),
                    ("Esc".into(), "exit"),
                    ("m".into(), "master"),
                    ("s".into(), "skip"),
                ]
            }
        }
    }
}

fn styled_hints(
    hints: &[Hint],
    budget: usize,
    theme: &Theme,
    symbols: &Symbols,
) -> (Vec<Span<'static>>, usize) {
    let mut spans = Vec::new();
    let mut used = 0;
    for (index, (key, label)) in hints.iter().enumerate() {
        let separator = usize::from(index > 0) * 2;
        let piece_width = separator + display_width(key) + 1 + display_width(label);
        if used + piece_width > budget {
            if used + display_width(symbols.ellipsis) <= budget {
                spans.push(Span::styled(
                    symbols.ellipsis.to_string(),
                    theme.muted().bg(theme.surface_alt_bg),
                ));
                used += display_width(symbols.ellipsis);
            }
            break;
        }
        if index > 0 {
            spans.push(Span::styled("  ", theme.muted().bg(theme.surface_alt_bg)));
            used += 2;
        }
        spans.push(Span::styled(
            key.clone(),
            theme.keycap().bg(theme.surface_alt_bg),
        ));
        spans.push(Span::styled(
            format!(" {label}"),
            theme.muted().bg(theme.surface_alt_bg),
        ));
        used += display_width(key) + 1 + display_width(label);
    }
    (spans, used)
}

fn status_style(tone: StatusTone, theme: &Theme) -> Style {
    match tone {
        StatusTone::Neutral => theme.text(),
        StatusTone::Success => theme.success(),
        StatusTone::Progress => theme.accent(),
        StatusTone::Warning => theme.warning(),
        StatusTone::Error => theme.error(),
        StatusTone::Permission => theme.permission(),
    }
}
