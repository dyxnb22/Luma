use crate::view_model::{AppState, Route};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use ratatui::Frame;

/// Pure projection. Must not mutate state or start tasks.
pub fn render(frame: &mut Frame<'_>, state: &AppState) {
    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(3),
            Constraint::Length(3),
        ])
        .split(area);

    render_prompt(frame, chunks[0], state);
    match state.route {
        Route::Search => render_results(frame, chunks[1], state),
        Route::Help => render_help(frame, chunks[1]),
        Route::Doctor => render_doctor(frame, chunks[1], state),
        Route::QuitConfirm => render_help(frame, chunks[1]),
        Route::ConfirmAction => render_confirm(frame, chunks[1], state),
        Route::ActionPicker => render_action_picker(frame, chunks[1], state),
    }
    render_status(frame, chunks[2], state);
}

fn render_prompt(frame: &mut Frame<'_>, area: Rect, state: &AppState) {
    let text = format!("> {}", state.prompt);
    let widget = Paragraph::new(text).block(Block::default().borders(Borders::ALL).title("luma"));
    frame.render_widget(widget, area);
}

fn render_results(frame: &mut Frame<'_>, area: Rect, state: &AppState) {
    let items: Vec<ListItem> = state
        .results
        .items
        .iter()
        .map(|item| {
            let selected = state
                .results
                .selected_id
                .as_deref()
                .is_some_and(|id| id == item.id.as_str());
            let prefix = if selected { ">" } else { " " };
            let line = Line::from(vec![
                Span::raw(format!("{prefix} ")),
                Span::styled(
                    item.title.clone(),
                    if selected {
                        Style::default().add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                    },
                ),
            ]);
            ListItem::new(line)
        })
        .collect();
    let list = List::new(items).block(Block::default().borders(Borders::ALL).title("results"));
    frame.render_widget(list, area);
}

fn render_confirm(frame: &mut Frame<'_>, area: Rect, state: &AppState) {
    let label = state
        .pending_action
        .as_ref()
        .map(|p| p.action.label.as_str())
        .unwrap_or("action");
    let text = format!("Confirm {label}?\n\nEnter = yes with confirmation\nEsc = cancel");
    let widget =
        Paragraph::new(text).block(Block::default().borders(Borders::ALL).title("confirm"));
    frame.render_widget(widget, area);
}

fn render_action_picker(frame: &mut Frame<'_>, area: Rect, state: &AppState) {
    let items: Vec<ListItem> = state
        .action_choices
        .iter()
        .enumerate()
        .map(|(idx, action)| {
            let selected = idx == state.action_selected;
            let prefix = if selected { ">" } else { " " };
            let risk = format!("{:?}", action.risk).to_lowercase();
            let confirm = if action.confirmation {
                " · confirm"
            } else {
                ""
            };
            let line = Line::from(vec![
                Span::raw(format!("{prefix} ")),
                Span::styled(
                    format!("{} ({risk}{confirm})", action.label),
                    if selected {
                        Style::default().add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                    },
                ),
            ]);
            ListItem::new(line)
        })
        .collect();
    let list = List::new(items).block(Block::default().borders(Borders::ALL).title("actions"));
    frame.render_widget(list, area);
}

fn render_help(frame: &mut Frame<'_>, area: Rect) {
    let text = "Keys: type to search | Up/Down select | Enter primary action | Tab action list | Esc cancel/back | ? help | :doctor Enter | Ctrl-C quit\nConfirm/Destructive actions ask before running.";
    let widget = Paragraph::new(text).block(Block::default().borders(Borders::ALL).title("help"));
    frame.render_widget(widget, area);
}

fn render_doctor(frame: &mut Frame<'_>, area: Rect, state: &AppState) {
    let text = state.doctor_diagnostic.as_ref().map_or_else(
        || "Doctor\nWaiting for engine diagnostics…".to_string(),
        |diagnostic| format!("Doctor\n{diagnostic}"),
    );
    let widget = Paragraph::new(text).block(Block::default().borders(Borders::ALL).title("doctor"));
    frame.render_widget(widget, area);
}

fn render_status(frame: &mut Frame<'_>, area: Rect, state: &AppState) {
    let widget = Paragraph::new(state.status.text.as_str())
        .block(Block::default().borders(Borders::ALL).title("status"));
    frame.render_widget(widget, area);
}
