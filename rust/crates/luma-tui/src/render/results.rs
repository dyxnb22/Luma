use super::util::{
    display_width, highlight_query, highlighted_spans, pad_line_to_width, pad_right, truncate,
};
use crate::theme::{module_glyph, module_label, ResultKindVisual, Symbols, Theme};
use crate::view_model::{AppState, Route, StatusTone};
use luma_domain::SearchItem;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem};
use ratatui::Frame;

pub(super) fn render_results(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &AppState,
    theme: &Theme,
    symbols: &Symbols,
) {
    use crate::view_model::FocusZone;

    let list_focused =
        matches!(state.focus, FocusZone::List) && matches!(state.route, Route::Search);
    let inner_height = area.height.saturating_sub(2) as usize;
    let inner_width = area.width.saturating_sub(2);
    let rows_capacity = (inner_height / 2).max(1);
    let total = state.search.results.items.len();
    let scroll = if total == 0 {
        0
    } else {
        state
            .search
            .results
            .scroll
            .min(total.saturating_sub(rows_capacity.min(total)))
    };
    let selected_idx = state.search.results.selected_index().unwrap_or(0);
    let visible_count = if total == 0 {
        0
    } else {
        rows_capacity.min(total.saturating_sub(scroll))
    };
    let has_above = scroll > 0;
    let has_below = scroll + visible_count < total;

    let mut items: Vec<ListItem> = Vec::new();
    if state.search.results.items.is_empty() {
        if state.search.prompt.trim().is_empty() {
            items.extend(hub_list_items(state, theme, symbols));
        } else {
            items.push(empty_state_item(state, theme, symbols));
        }
    } else {
        let query = highlight_query(&state.search.prompt);
        for item in state
            .search
            .results
            .items
            .iter()
            .skip(scroll)
            .take(visible_count)
        {
            let selected = state
                .search
                .results
                .selected_id
                .as_deref()
                .is_some_and(|id| id == item.id.as_str());
            let win_digit = if state.is_win_search()
                && state.focus == FocusZone::List
                && item.module_id.as_str() == "luma.windows"
                && item.kind == "window"
            {
                state
                    .window_digit_targets()
                    .iter()
                    .position(|(id, _)| id == item.id.as_str())
                    .map(|i| i + 1)
            } else {
                None
            };
            items.push(result_row(
                item,
                selected,
                inner_width,
                &query,
                theme,
                symbols,
                &state.module_labels,
                win_digit,
            ));
        }
    }

    let title = if state.search.results.items.is_empty() {
        if state.search.prompt.trim().is_empty() {
            let hub_total = state.hub_rows().len();
            let sel = state.hub.selected + 1;
            let mut scroll_marks = String::new();
            if state.hub.scroll > 0 {
                scroll_marks.push_str(symbols.up);
            }
            if state.hub.scroll + state.hub_data_capacity() < hub_total {
                scroll_marks.push_str(symbols.down);
            }
            let scroll_part = if scroll_marks.is_empty() {
                String::new()
            } else {
                format!(" {scroll_marks}")
            };
            if hub_total > 0 {
                format!(
                    " hub {} {}/{}{scroll_part} ",
                    symbols.sep,
                    sel.min(hub_total),
                    hub_total
                )
            } else {
                " hub ".to_string()
            }
        } else {
            " results ".to_string()
        }
    } else {
        let mut scroll_marks = String::new();
        if has_above {
            scroll_marks.push_str(symbols.up);
        }
        if has_below {
            scroll_marks.push_str(symbols.down);
        }
        let scroll_part = if scroll_marks.is_empty() {
            String::new()
        } else {
            format!(" {scroll_marks}")
        };
        format!(
            " results {} {}/{}{scroll_part} ",
            symbols.sep,
            selected_idx + 1,
            total
        )
    };

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(theme.border(list_focused))
            .title(Span::styled(title, theme.muted())),
    );
    frame.render_widget(list, area);
}

fn hub_module_title(state: &AppState, module_id: &str, title: &str) -> String {
    if !state.settings.roots.loaded {
        return title.to_string();
    }
    let needs_notes = module_id == "luma.notes"
        && state
            .settings
            .roots
            .notes_root
            .as_ref()
            .map(|s| s.is_empty())
            .unwrap_or(true);
    let needs_projects = module_id == "luma.projects"
        && (state.settings.roots.projects_roots.is_empty()
            || state.settings.roots.imported_projects.is_empty());
    if needs_notes || needs_projects {
        if module_id == "luma.projects" && !state.settings.roots.projects_roots.is_empty() {
            format!("{title} · import")
        } else {
            format!("{title} · set root")
        }
    } else {
        title.to_string()
    }
}

fn hub_list_items(state: &AppState, theme: &Theme, symbols: &Symbols) -> Vec<ListItem<'static>> {
    let rows = state.hub_rows();
    if rows.is_empty() {
        return vec![ListItem::new(vec![
            Line::from(Span::styled("  Waiting for modules…", theme.muted())),
            Line::from(Span::styled(
                "  Loading modules in the background",
                theme.key_hint(),
            )),
        ])];
    }
    let mut out = Vec::new();
    let mut shown_windows = false;
    let mut shown_modules = false;
    let viewport = state.hub_data_capacity();
    let start = state.hub.scroll.min(rows.len());
    let end = (start + viewport).min(rows.len());
    let module_global_start = rows.iter().position(|(k, _, _, _)| k == "module");
    let window = &rows[start..end];
    for (idx, (kind, _id, title, query)) in window.iter().enumerate().map(|(i, r)| (start + i, r)) {
        if (kind == "window" || kind == "window_more" || kind == "window_status")
            && !shown_windows
            && start == 0
        {
            shown_windows = true;
            let header = match state.hub.windows.as_ref().map(|h| h.app_name.as_str()) {
                Some("all") | Some("") | None => "  Windows".to_string(),
                Some(app) => format!("  Windows · {app}"),
            };
            let hint = "  Enter focuses window · 1-9 focus · ↑↓ move";
            out.push(ListItem::new(vec![
                Line::from(Span::styled(header, theme.title())),
                Line::from(Span::styled(hint, theme.key_hint())),
            ]));
        }
        if kind == "module" && !shown_modules && module_global_start == Some(idx) {
            shown_modules = true;
            out.push(ListItem::new(vec![
                Line::from(Span::styled("  Modules", theme.title())),
                Line::from(Span::styled(
                    "  Enter opens trigger · ↑↓ move",
                    theme.key_hint(),
                )),
            ]));
        }
        let selected = idx == state.hub.selected;
        let prefix = if selected { symbols.selected } else { " " };
        let style = if selected {
            theme.selected_row()
        } else {
            theme.text()
        };
        let muted = if selected {
            theme.selected_row()
        } else {
            theme.muted()
        };
        let right = match kind.as_str() {
            "window" => state
                .hub_row_window_digit(idx)
                .map(|d| format!("[{d}]"))
                .unwrap_or_default(),
            "window_more" | "window_status" => "win".to_string(),
            _ => query.clone(),
        };
        let right = truncate(
            &right,
            (state.terminal.width.saturating_sub(6) as usize / 3).max(8),
            symbols,
        );
        let content_width = state.terminal.width.saturating_sub(4) as usize;
        let guidance = if kind == "window_status" {
            state
                .hub
                .windows
                .as_ref()
                .and_then(|h| h.status_subtitle.as_ref())
                .cloned()
        } else {
            None
        };
        let display_title = if kind == "module" {
            hub_module_title(state, _id, title)
        } else {
            title.clone()
        };
        let display_title = truncate(
            &display_title,
            content_width
                .saturating_sub(display_width(&right) + 4)
                .max(8),
            symbols,
        );
        let mut lines = vec![Line::from(vec![
            Span::styled(format!(" {prefix} {display_title}"), style),
            Span::styled(format!("  {right}"), muted),
        ])];
        if let Some(sub) = guidance {
            lines.push(Line::from(Span::styled(
                format!(
                    "    {}",
                    truncate(&sub, content_width.saturating_sub(4), symbols)
                ),
                muted,
            )));
        } else {
            lines.push(Line::from(""));
        }
        out.push(ListItem::new(lines));
    }
    out
}

fn empty_state_item(state: &AppState, theme: &Theme, symbols: &Symbols) -> ListItem<'static> {
    let (title, detail) = if state.search.debounce_deadline.is_some() {
        (
            format!("Keep typing{}", symbols.ellipsis),
            "Search runs after a short pause".to_string(),
        )
    } else if let Some((trigger, hint)) = incomplete_trigger_hint(state) {
        (
            format!("Add a space to enter `{trigger}`"),
            hint.unwrap_or_else(|| format!("Try `{trigger} ` or `{trigger} query`")),
        )
    } else if state.search.active_request.is_some() && state.status.tone == StatusTone::Progress {
        (
            format!("Searching{}", symbols.ellipsis),
            "Results appear as modules respond".to_string(),
        )
    } else if state.search.prompt.trim().is_empty() {
        (
            "Type to search".to_string(),
            format!(
                "Try: /app safari {} /n browse {} /clip",
                symbols.sep, symbols.sep
            ),
        )
    } else if let Some(hint) = empty_hint_for_prompt(state) {
        ("No results".to_string(), hint)
    } else {
        (
            "No results".to_string(),
            "Adjust the query or try another module trigger".to_string(),
        )
    };
    ListItem::new(vec![
        Line::from(Span::styled(format!("  {title}"), theme.muted())),
        Line::from(Span::styled(format!("  {detail}"), theme.key_hint())),
    ])
}

/// Slash-prefixed bare module trigger without trailing space (`/n`, not `n` or `/n `).
fn incomplete_trigger_hint(state: &AppState) -> Option<(String, Option<String>)> {
    let token = state.incomplete_slash_trigger()?;
    let module = state
        .module_catalog
        .iter()
        .find(|m| m.enabled && m.triggers.iter().any(|t| t.eq_ignore_ascii_case(&token)));
    let display = format!("/{token}");
    Some((display, module.and_then(|m| m.empty_hint.clone())))
}

/// Prefer the targeted module's `empty_hint` when the prompt starts with its trigger.
fn empty_hint_for_prompt(state: &AppState) -> Option<String> {
    let token = state
        .search
        .prompt
        .trim_start()
        .strip_prefix('/')?
        .split_whitespace()
        .next()?
        .to_ascii_lowercase();
    state
        .module_catalog
        .iter()
        .find(|m| m.enabled && m.triggers.iter().any(|t| t.eq_ignore_ascii_case(&token)))
        .and_then(|m| m.empty_hint.clone())
}

#[allow(clippy::too_many_arguments)]
fn result_row(
    item: &SearchItem,
    selected: bool,
    width: u16,
    query: &str,
    theme: &Theme,
    symbols: &Symbols,
    module_labels: &std::collections::HashMap<String, String>,
    win_digit: Option<usize>,
) -> ListItem<'static> {
    let kind = ResultKindVisual::from_kind(&item.kind);
    let glyph = module_glyph(item.module_id.as_str());
    let module = module_label(item.module_id.as_str(), module_labels);
    let action = match kind {
        ResultKindVisual::Warming => format!("{} Wait", symbols.ellipsis),
        _ => format!("{} {}", symbols.enter, item.primary_action.label),
    };
    let prefix = if selected { symbols.selected } else { " " };
    let row_bg = if selected {
        Style::default().bg(theme.selected_bg)
    } else {
        Style::default()
    };
    let base = theme.kind_style(kind, selected).patch(row_bg);
    let muted = if selected {
        Style::default().fg(theme.selected_fg).bg(theme.selected_bg)
    } else {
        theme.muted()
    };
    let hint = match kind {
        ResultKindVisual::Permission => {
            theme.permission().patch(row_bg).add_modifier(if selected {
                Modifier::BOLD
            } else {
                Modifier::empty()
            })
        }
        ResultKindVisual::Unavailable | ResultKindVisual::NotConfigured => {
            theme.warning().patch(row_bg).add_modifier(if selected {
                Modifier::BOLD
            } else {
                Modifier::empty()
            })
        }
        ResultKindVisual::Warming => muted,
        ResultKindVisual::Normal => {
            if selected {
                Style::default()
                    .fg(theme.accent)
                    .bg(theme.selected_bg)
                    .add_modifier(Modifier::BOLD)
            } else {
                theme.action_hint()
            }
        }
    };
    let badge = if selected {
        Style::default().fg(theme.muted).bg(theme.selected_bg)
    } else {
        theme.module_badge()
    };

    let kind_badge = kind.badge().unwrap_or("");
    let prefix_w = display_width(&format!("{prefix} {glyph} "));
    let right_w = display_width(&format!(
        "{} {module}  {action}",
        if kind_badge.is_empty() {
            String::new()
        } else {
            format!(" {kind_badge}")
        }
    ));
    let title_budget = (width as usize).saturating_sub(prefix_w + right_w).max(8);
    let title = truncate(&item.title, title_budget, symbols);
    let subtitle = item
        .subtitle
        .as_deref()
        .map(|s| truncate(s, width.saturating_sub(4) as usize, symbols))
        .unwrap_or_default();

    let mut title_spans = vec![Span::styled(format!("{prefix} {glyph} "), base)];
    title_spans.extend(highlighted_spans(
        &pad_right(&title, title_budget),
        query,
        base,
        theme.match_highlight(selected).patch(row_bg),
    ));
    if !kind_badge.is_empty() {
        title_spans.push(Span::styled(format!(" {kind_badge}"), hint));
    }
    if let Some(d) = win_digit {
        title_spans.push(Span::styled(format!(" [{d}]"), hint));
    }
    title_spans.push(Span::styled(format!(" {module}"), badge));
    title_spans.push(Span::styled(format!("  {action}"), hint));
    pad_line_to_width(&mut title_spans, width as usize, row_bg);

    let mut sub_spans = if subtitle.is_empty() {
        vec![Span::styled("    ", muted)]
    } else {
        vec![Span::styled(format!("    {subtitle}"), muted)]
    };
    pad_line_to_width(&mut sub_spans, width as usize, row_bg);

    ListItem::new(vec![Line::from(title_spans), Line::from(sub_spans)])
}
