use crate::theme::{Symbols, Theme};
use crate::view_model::{AppState, Route};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;
mod overlays;
mod preview;
mod results;
mod status;
mod util;
mod wordbook;

#[cfg(test)]
use crate::view_model::StatusTone;
#[cfg(test)]
use util::{display_width, highlight_query, truncate};

use overlays::*;

/// Pure projection. Must not mutate state, start tasks, or read the environment.
pub fn render(frame: &mut Frame<'_>, state: &AppState) {
    render_with(frame, state, &state.theme, &state.symbols);
}

fn render_with(frame: &mut Frame<'_>, state: &AppState, theme: &Theme, symbols: &Symbols) {
    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(3),
            Constraint::Length(3),
        ])
        .split(area);

    let prompt_focused = matches!(state.focus, crate::view_model::FocusZone::Prompt)
        && matches!(state.route, Route::Search);
    render_prompt(frame, chunks[0], state, theme, symbols, prompt_focused);

    let body = chunks[1];
    if state.route == Route::WordbookReview
        || (state.wordbook.review.is_some() && matches!(state.route, Route::ConfirmAction))
    {
        wordbook::render_wordbook_review(frame, body, state, theme, symbols);
    } else if state.preview_side_by_side() {
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(58), Constraint::Percentage(42)])
            .split(body);
        results::render_results(frame, cols[0], state, theme, symbols);
        preview::render_preview(frame, cols[1], state, theme, symbols);
    } else if state.preview_stacked() {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(3), Constraint::Length(8)])
            .split(body);
        results::render_results(frame, rows[0], state, theme, symbols);
        preview::render_preview(frame, rows[1], state, theme, symbols);
    } else {
        results::render_results(frame, body, state, theme, symbols);
    }
    status::render_status(frame, chunks[2], state, theme, symbols);

    match state.route {
        Route::Search | Route::WordbookReview => {}
        Route::Help => render_overlay_help(frame, area, state, theme, symbols),
        Route::Settings => render_overlay_settings(frame, area, state, theme, symbols),
        Route::Commands => render_overlay_commands(frame, area, state, theme, symbols),
        Route::QuitConfirm => render_overlay_quit(frame, area, theme, symbols),
        Route::ConfirmAction => render_overlay_confirm(frame, area, state, theme, symbols),
        Route::ActionPicker => render_overlay_action_picker(frame, area, state, theme, symbols),
    }
}

fn render_prompt(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &AppState,
    theme: &Theme,
    symbols: &Symbols,
    focused: bool,
) {
    let inner_w = area.width.saturating_sub(2) as usize;
    let _ = inner_w;
    let cursor = if focused { symbols.cursor } else { " " };
    let chars: Vec<char> = state.search.prompt.chars().collect();
    let before: String = chars
        .iter()
        .skip(state.search.prompt_scroll)
        .take(
            state
                .search
                .prompt_cursor
                .saturating_sub(state.search.prompt_scroll),
        )
        .collect();
    let after: String = chars.iter().skip(state.search.prompt_cursor).collect();
    let line = Line::from(vec![
        Span::styled("  ", theme.muted()),
        Span::styled(before, theme.text()),
        Span::styled(cursor, theme.accent()),
        Span::styled(after, theme.text()),
    ]);
    let widget = Paragraph::new(line).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(theme.border(focused))
            .title(Span::styled(" Luma ", theme.title())),
    );
    frame.render_widget(widget, area);
}

#[cfg(test)]
mod tests;
