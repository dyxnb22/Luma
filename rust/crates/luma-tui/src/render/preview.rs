use crate::theme::{module_label, ResultKindVisual, Symbols, Theme};
use crate::view_model::{is_informational_kind, AppState, Route};
use luma_domain::ActionRisk;
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

pub(super) fn render_preview(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &AppState,
    theme: &Theme,
    symbols: &Symbols,
) {
    use crate::view_model::FocusZone;

    let focused = matches!(state.focus, FocusZone::Preview) && matches!(state.route, Route::Search);
    let Some(item) = state.search.results.selected_id.as_ref().and_then(|id| {
        state
            .search
            .results
            .items
            .iter()
            .find(|i| i.id.as_str() == id)
    }) else {
        let widget = Paragraph::new(vec![
            Line::from(Span::styled(
                "  Select a result to preview",
                theme.text().add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                "  Tab focuses · Shift-Tab toggles preview",
                theme.muted(),
            )),
        ])
        .style(theme.surface())
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(symbols.border_type())
                .border_style(theme.border(focused))
                .style(theme.surface())
                .title(Span::styled(" PREVIEW ", theme.panel_title(focused))),
        );
        frame.render_widget(widget, area);
        return;
    };

    let module = module_label(item.module_id.as_str(), &state.module_labels);
    let kind = ResultKindVisual::from_kind(&item.kind);
    let risk = match item.primary_action.risk {
        ActionRisk::Safe => "safe",
        ActionRisk::Confirm => "confirm",
        ActionRisk::Destructive => "destructive",
    };
    let mut lines = vec![
        Line::from(Span::styled(
            format!("  {}", item.title),
            theme.text().add_modifier(Modifier::BOLD),
        )),
        Line::from(vec![
            Span::styled("  ", theme.muted()),
            Span::styled(
                format!(" {module} "),
                theme.module_badge().bg(theme.surface_alt_bg),
            ),
            Span::styled(format!("  {}  ", symbols.sep), theme.muted()),
            Span::styled(
                kind.badge().unwrap_or(item.kind.as_str()).to_string(),
                theme.accent_secondary(),
            ),
        ]),
    ];
    if let Some(sub) = &item.subtitle {
        let body_dupes_sub = state
            .preview
            .result_id
            .as_deref()
            .filter(|id| *id == item.id.as_str())
            .and(state.preview.body.as_deref())
            .is_some_and(|body| {
                let body_trim = body.trim();
                body_trim == sub.trim()
                    || body_trim == item.title.trim()
                    || body_trim
                        .lines()
                        .next()
                        .is_some_and(|l| l.trim() == sub.trim() || l.trim() == item.title.trim())
            });
        if !body_dupes_sub {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(format!("  {sub}"), theme.text())));
        }
    } else if state
        .preview
        .result_id
        .as_deref()
        .filter(|id| *id == item.id.as_str())
        .and(state.preview.body.as_deref())
        .is_some_and(|body| body.trim() == item.title.trim())
    {
        // Title already shown; skip empty subtitle when body only echoes title.
    }
    lines.push(Line::from(""));
    if state.preview.result_id.as_deref() == Some(item.id.as_str()) {
        if let Some(body) = &state.preview.body {
            let body_lines: Vec<&str> = body.lines().collect();
            let header_lines = lines.len();
            let visible = (area.height as usize)
                .saturating_sub(header_lines + 2)
                .max(1);
            let max_scroll = body_lines.len().saturating_sub(visible);
            let scroll = state.preview.scroll.min(max_scroll);
            let body_len = body_lines.len();
            for line in body_lines.into_iter().skip(scroll).take(visible) {
                if scroll == 0 && line.trim() == item.title.trim() && body_len == 1 {
                    continue;
                }
                lines.push(Line::from(Span::styled(format!("  {line}"), theme.text())));
            }
            if max_scroll > 0 {
                lines.push(Line::from(Span::styled(
                    format!("  {} {}/{}", symbols.ellipsis, scroll + 1, max_scroll + 1),
                    theme.muted(),
                )));
            }
        }
    } else {
        lines.push(Line::from(Span::styled(
            format!("  Loading preview{}", symbols.ellipsis),
            theme.muted(),
        )));
    }
    lines.push(Line::from(""));
    if is_informational_kind(&item.kind) {
        lines.push(Line::from(Span::styled("  STATUS ONLY", theme.section())));
    } else {
        lines.push(Line::from(Span::styled("  ACTIONS", theme.section())));
        lines.push(Line::from(vec![
            Span::styled(format!("  {} ", symbols.enter), theme.keycap()),
            Span::styled(&item.primary_action.label, theme.action_hint()),
            Span::styled(format!("  {risk}"), theme.muted()),
        ]));
        for sec in &item.secondary_actions {
            lines.push(Line::from(vec![
                Span::styled(format!("  {} ", symbols.sep), theme.accent_secondary()),
                Span::styled(&sec.label, theme.muted()),
            ]));
        }
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("  Ctrl-k", theme.keycap()),
            Span::styled("  more actions", theme.muted()),
        ]));
    }

    let widget = Paragraph::new(lines)
        .style(theme.surface())
        .wrap(Wrap { trim: false })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(symbols.border_type())
                .border_style(theme.border(focused))
                .style(theme.surface())
                .title(Span::styled(" PREVIEW ", theme.panel_title(focused))),
        );
    frame.render_widget(widget, area);
}
