use super::util::{display_width, truncate};
use crate::theme::{Symbols, Theme};
use crate::view_model::{AppState, Route, StatusTone};
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

pub(super) fn render_status(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &AppState,
    theme: &Theme,
    symbols: &Symbols,
) {
    let status_style = status_style(state.status.tone, theme);
    let hints = match state.route {
        Route::Search if state.showing_hub() => {
            format!(
                "1-9 focus {}{}{} move {} Enter open {} Ctrl-/ commands",
                symbols.sep, symbols.up, symbols.down, symbols.sep, symbols.sep
            )
        }
        Route::Search => {
            if state.focus == crate::view_model::FocusZone::Preview {
                format!(
                    "PgUp/Dn scroll {} Tab focus {} Esc back",
                    symbols.sep, symbols.sep
                )
            } else if state.is_win_search() && state.focus == crate::view_model::FocusZone::List {
                format!(
                    "1-9 focus {}{} move {} Enter open {} Ctrl-k actions {} Tab focus",
                    symbols.sep, symbols.up, symbols.sep, symbols.sep, symbols.sep
                )
            } else if !state.search.results.items.is_empty()
                && state.focus == crate::view_model::FocusZone::List
            {
                format!(
                    "{}{} move {} Enter run {} Ctrl-k actions {} Tab focus {} S-Tab preview {} Esc back {} ? help",
                    symbols.up,
                    symbols.down,
                    symbols.sep,
                    symbols.sep,
                    symbols.sep,
                    symbols.sep,
                    symbols.sep,
                    symbols.sep
                )
            } else {
                format!(
                    "{}{} move {} Enter search {} Ctrl-k actions {} Tab focus {} S-Tab preview {} Esc back {} ? help",
                    symbols.up,
                    symbols.down,
                    symbols.sep,
                    symbols.sep,
                    symbols.sep,
                    symbols.sep,
                    symbols.sep,
                    symbols.sep
                )
            }
        }
        Route::ActionPicker => format!(
            "{}{} 1-9 {} Enter Run {} Esc Back",
            symbols.up, symbols.down, symbols.sep, symbols.sep
        ),
        Route::Settings => format!(
            "{}{} Enter/Space Toggle {} Esc Back",
            symbols.up, symbols.down, symbols.sep
        ),
        Route::Commands => format!("Enter Run {} Esc Back", symbols.sep),
        Route::ConfirmAction | Route::QuitConfirm => {
            format!("Enter Confirm {} Esc Cancel", symbols.sep)
        }
        Route::Help => format!(
            "{}{} / PgUp PgDn scroll {} Esc Back",
            symbols.up, symbols.down, symbols.sep
        ),
        Route::WordbookReview => {
            if state.wordbook.review.as_ref().is_some_and(|r| r.finished) {
                "Esc back".into()
            } else {
                "Enter/Space reveal · 1/2/3 grade · m master · s skip · Esc exit".into()
            }
        }
    };

    let inner_w = area.width.saturating_sub(2) as usize;
    let narrow = inner_w < 60;
    let (status_text, hints) = if narrow {
        let compact_hints = match state.route {
            Route::Search if state.showing_hub() => "1-9 · ↑↓ · Enter".to_string(),
            Route::Search => "Enter · Ctrl-k · Esc".to_string(),
            Route::ActionPicker => "1-9 · Enter · Esc".to_string(),
            Route::Settings => "↑↓ · Space · Esc".to_string(),
            Route::Commands => "Enter · Esc".to_string(),
            Route::ConfirmAction | Route::QuitConfirm => "Enter yes · Esc no".to_string(),
            Route::Help => "↑↓ · Esc".to_string(),
            Route::WordbookReview => {
                if state.wordbook.review.as_ref().is_some_and(|r| r.finished) {
                    "Esc back".to_string()
                } else {
                    "1/2/3 · s skip · Esc".to_string()
                }
            }
        };
        let compact_status = if state.route == Route::WordbookReview {
            if state.wordbook.review.as_ref().is_some_and(|r| r.finished) {
                "done"
            } else {
                "review"
            }
        } else if state.status.text.starts_with("removed ") {
            "removed · dir kept"
        } else {
            state.status.text.as_str()
        };
        (compact_status.to_string(), compact_hints)
    } else {
        (state.status.text.clone(), hints)
    };
    let hints_budget = (inner_w / 2).clamp(16, 60);
    let hints = truncate(&hints, hints_budget, symbols);
    let status_budget = inner_w.saturating_sub(display_width(&hints) + 3).max(8);
    let status_text = truncate(&status_text, status_budget, symbols);

    let line = Line::from(vec![
        Span::styled(format!(" {status_text}  "), status_style),
        Span::styled(hints, theme.key_hint()),
    ]);
    let widget = Paragraph::new(line).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(theme.border(false))
            .title(Span::styled(" status ", theme.muted())),
    );
    frame.render_widget(widget, area);
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
