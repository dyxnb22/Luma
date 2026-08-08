use crate::theme::{Symbols, Theme};
use crate::view_model::{AppState, Route};
use ratatui::layout::{Constraint, Direction, Layout, Margin, Position, Rect};
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
    frame.render_widget(Block::default().style(theme.canvas()), area);

    let horizontal_margin = u16::from(area.width >= 48);
    let workspace = area.inner(Margin {
        horizontal: horizontal_margin,
        vertical: 0,
    });
    let spacing = u16::from(workspace.height >= 12);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(3),
            Constraint::Length(1),
        ])
        .spacing(spacing)
        .split(workspace);

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
            .spacing(1)
            .split(body);
        results::render_results(frame, cols[0], state, theme, symbols);
        preview::render_preview(frame, cols[1], state, theme, symbols);
    } else if state.preview_stacked() {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(3), Constraint::Length(8)])
            .spacing(1)
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
    use unicode_segmentation::UnicodeSegmentation;
    let cursor = if focused { symbols.cursor } else { " " };
    let graphemes: Vec<&str> = state.search.prompt.graphemes(true).collect();
    let before: String = graphemes
        .iter()
        .skip(state.search.prompt_scroll)
        .take(
            state
                .search
                .prompt_cursor
                .saturating_sub(state.search.prompt_scroll),
        )
        .copied()
        .collect();
    let after: String = graphemes
        .iter()
        .skip(state.search.prompt_cursor)
        .copied()
        .collect();
    let prompt_prefix = format!(" {} ", symbols.search);
    let cursor_offset =
        util::display_width(&prompt_prefix).saturating_add(util::display_width(&before));
    let mut spans = vec![
        Span::styled(prompt_prefix, theme.accent()),
        Span::styled(before, theme.text()),
        Span::styled(cursor, theme.accent()),
        Span::styled(after, theme.text()),
    ];
    if state.search.prompt.is_empty() {
        spans.push(Span::styled(
            " Search everything or type / for commands",
            theme.muted(),
        ));
    }
    let line = Line::from(spans);
    let command_mode = state.search.prompt.trim_start().starts_with('/');
    let mode = if command_mode {
        if focused {
            " COMMAND · INPUT "
        } else {
            " COMMAND · TAB TO INPUT "
        }
    } else if focused {
        " GLOBAL SEARCH · INPUT "
    } else {
        " GLOBAL SEARCH · TAB TO INPUT "
    };
    let mode_style = if command_mode {
        theme.accent_secondary()
    } else {
        theme.muted()
    };
    let widget = Paragraph::new(line).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(symbols.border_type())
            .border_style(theme.border(focused))
            .style(theme.surface())
            .title(Span::styled(" LUMA ", theme.title()))
            .title(Line::from(Span::styled(mode, mode_style)).right_aligned()),
    );
    frame.render_widget(widget, area);
    if focused && area.width > 2 && area.height > 2 {
        // Ratatui otherwise hides the real terminal cursor and only our decorative glyph remains.
        // SwiftTerm anchors IME marked text and candidate windows to that real cursor, so leaving
        // it at the final ANSI write position puts composition at the bottom-right of the window.
        let content_right = area.right().saturating_sub(2);
        let cursor_x = area
            .x
            .saturating_add(1)
            .saturating_add(cursor_offset as u16)
            .min(content_right);
        frame.set_cursor_position(Position::new(cursor_x, area.y.saturating_add(1)));
    }
}

#[cfg(test)]
mod tests;
