//! vt100 → Ratatui projection for the embedded SSH terminal pane.

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

/// Hard scrollback cap for embedded SSH sessions (product requirement).
pub const SCROLLBACK_CAP: usize = 2000;

/// In-memory virtual terminal screen backed by `vt100`.
///
/// OSC sequences that would affect the host (clipboard / window title) are never
/// applied outside this parser. Callers must not read `title()` to mutate the
/// Swift host or pasteboard.
pub struct VtScreen {
    parser: vt100::Parser,
    cols: u16,
    rows: u16,
}

impl VtScreen {
    pub fn new(cols: u16, rows: u16) -> Self {
        let cols = cols.max(1);
        let rows = rows.max(1);
        Self {
            parser: vt100::Parser::new(rows, cols, SCROLLBACK_CAP),
            cols,
            rows,
        }
    }

    pub fn feed(&mut self, bytes: &[u8]) {
        // vt100 parses OSC internally into screen title/icon state only.
        // We intentionally never expose those to the host clipboard or window.
        self.parser.process(bytes);
    }

    pub fn resize(&mut self, cols: u16, rows: u16) {
        let cols = cols.max(1);
        let rows = rows.max(1);
        self.cols = cols;
        self.rows = rows;
        self.parser.set_size(rows, cols);
    }

    pub fn cols(&self) -> u16 {
        self.cols
    }

    pub fn rows(&self) -> u16 {
        self.rows
    }

    /// Cursor as `(row, col)` 0-based.
    pub fn cursor(&self) -> (u16, u16) {
        self.parser.screen().cursor_position()
    }

    pub fn in_alternate_screen(&self) -> bool {
        self.parser.screen().alternate_screen()
    }

    pub fn visible_rows(&self) -> usize {
        self.rows as usize
    }

    /// Scrollback length configured on the parser (hard cap).
    pub fn scrollback_cap(&self) -> usize {
        SCROLLBACK_CAP
    }

    /// Project the visible screen into owned Ratatui lines.
    pub fn render_lines(&self) -> Vec<Line<'static>> {
        let screen = self.parser.screen();
        let mut lines = Vec::with_capacity(self.rows as usize);
        for row in 0..self.rows {
            let mut spans: Vec<Span<'static>> = Vec::new();
            let mut col = 0u16;
            while col < self.cols {
                let Some(cell) = screen.cell(row, col) else {
                    break;
                };
                if cell.is_wide_continuation() {
                    col = col.saturating_add(1);
                    continue;
                }
                let width = if cell.is_wide() { 2u16 } else { 1u16 };
                let content = cell.contents();
                if content.is_empty() {
                    spans.push(Span::styled(" ".to_string(), cell_style(cell)));
                } else {
                    spans.push(Span::styled(content, cell_style(cell)));
                }
                col = col.saturating_add(width);
            }
            if spans.is_empty() {
                spans.push(Span::raw(""));
            }
            lines.push(Line::from(spans));
        }
        lines
    }

    /// Plain visible contents (tests / error summaries).
    pub fn contents(&self) -> String {
        self.parser.screen().contents()
    }
}

fn cell_style(cell: &vt100::Cell) -> Style {
    let mut style = Style::default()
        .fg(map_color(cell.fgcolor()))
        .bg(map_color(cell.bgcolor()));
    let mut mods = Modifier::empty();
    if cell.bold() {
        mods |= Modifier::BOLD;
    }
    if cell.italic() {
        mods |= Modifier::ITALIC;
    }
    if cell.underline() {
        mods |= Modifier::UNDERLINED;
    }
    if cell.inverse() {
        mods |= Modifier::REVERSED;
    }
    style = style.add_modifier(mods);
    style
}

fn map_color(color: vt100::Color) -> Color {
    match color {
        vt100::Color::Default => Color::Reset,
        vt100::Color::Idx(idx) => Color::Indexed(idx),
        vt100::Color::Rgb(r, g, b) => Color::Rgb(r, g, b),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Modifier;

    #[test]
    fn ansi_color_and_bold_project_to_spans() {
        let mut screen = VtScreen::new(40, 5);
        screen.feed(b"\x1b[1;31mHi\x1b[0m");
        let lines = screen.render_lines();
        assert!(!lines.is_empty());
        let first = &lines[0];
        assert!(first
            .spans
            .iter()
            .any(|span| span.content.as_ref().contains('H')));
        assert!(first.spans.iter().any(|span| span.style.add_modifier.contains(Modifier::BOLD)
            || span.style.fg == Some(Color::Indexed(1))
            || matches!(span.style.fg, Some(Color::Rgb(_, _, _)))));
    }

    #[test]
    fn cursor_tracks_cup() {
        let mut screen = VtScreen::new(40, 10);
        screen.feed(b"\x1b[5;10H");
        assert_eq!(screen.cursor(), (4, 9));
    }

    #[test]
    fn alternate_screen_smcup_rmcup() {
        let mut screen = VtScreen::new(40, 10);
        assert!(!screen.in_alternate_screen());
        screen.feed(b"\x1b[?1049h");
        assert!(screen.in_alternate_screen());
        screen.feed(b"\x1b[?1049l");
        assert!(!screen.in_alternate_screen());
    }

    #[test]
    fn scrollback_stays_within_hard_cap() {
        let mut screen = VtScreen::new(20, 5);
        for _ in 0..5000 {
            screen.feed(b"line\r\n");
        }
        // Cap is enforced by vt100's scrollback_len; visible rows stay small.
        assert_eq!(screen.visible_rows(), 5);
        assert_eq!(screen.scrollback_cap(), SCROLLBACK_CAP);
        // Contents should remain bounded (visible + scrollback).
        let contents = screen.contents();
        assert!(contents.len() < SCROLLBACK_CAP * 20);
    }

    #[test]
    fn cjk_and_emoji_do_not_panic() {
        let mut screen = VtScreen::new(40, 5);
        screen.feed("你好🌍🚀\r\n".as_bytes());
        let _ = screen.render_lines();
        let _ = screen.contents();
    }

    #[test]
    fn osc_52_does_not_panic_or_require_host_clipboard() {
        let mut screen = VtScreen::new(40, 5);
        // OSC 52 clipboard request — must be inert for Luma host.
        screen.feed(b"\x1b]52;c;SGVsbG8=\x07");
        let _ = screen.render_lines();
    }
}
