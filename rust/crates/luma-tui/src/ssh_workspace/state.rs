//! Pure SSH Workspace view state (no PTY handles).

use crate::ssh_workspace::screen::VtScreen;
use crate::ssh_workspace::shelf::ShelfState;
use ratatui::text::Line;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SshConnectionPhase {
    Starting,
    Authenticating,
    Connected,
    Disconnected,
    Failed,
}

impl SshConnectionPhase {
    pub fn header_label(self) -> &'static str {
        match self {
            Self::Starting => "Connecting",
            Self::Authenticating => "Authenticating",
            Self::Connected => "Connected",
            Self::Disconnected => "Connection lost",
            Self::Failed => "Failed",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SshWorkspaceFocus {
    #[default]
    Terminal,
    Shelf,
    Leader,
}

#[derive(Clone, Debug)]
pub struct SshWorkspaceState {
    pub alias: String,
    pub hostname: String,
    pub user: String,
    pub port: u16,
    pub title: String,
    pub phase: SshConnectionPhase,
    pub status_detail: String,
    pub exit_code: Option<i32>,
    pub focus: SshWorkspaceFocus,
    pub shelf_visible: bool,
    pub fullscreen_chrome: bool,
    pub term_cols: u16,
    pub term_rows: u16,
    /// Snapshot for render (updated by effects after vt100 feed).
    pub lines: Vec<Line<'static>>,
    pub cursor_row: u16,
    pub cursor_col: u16,
    pub program: String,
    pub args: Vec<String>,
    pub environment: Vec<(String, String)>,
    pub record_alias: Option<String>,
    pub error_summary: String,
    pub disconnect_confirm: bool,
    pub leader_armed: bool,
    pub shelf: ShelfState,
}

impl SshWorkspaceState {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        alias: String,
        hostname: String,
        user: String,
        port: u16,
        title: String,
        program: String,
        args: Vec<String>,
        environment: Vec<(String, String)>,
        record_alias: Option<String>,
        term_cols: u16,
        term_rows: u16,
    ) -> Self {
        Self {
            alias,
            hostname,
            user,
            port,
            title,
            phase: SshConnectionPhase::Starting,
            status_detail: String::new(),
            exit_code: None,
            focus: SshWorkspaceFocus::Terminal,
            shelf_visible: false,
            fullscreen_chrome: false,
            term_cols: term_cols.max(1),
            term_rows: term_rows.max(1),
            lines: Vec::new(),
            cursor_row: 0,
            cursor_col: 0,
            program,
            args,
            environment,
            record_alias,
            error_summary: String::new(),
            disconnect_confirm: false,
            leader_armed: false,
            shelf: ShelfState::from_recipes(&[], true),
        }
    }

    pub fn with_shelf_recipes(mut self, recipes: &[luma_domain::Recipe]) -> Self {
        self.shelf = ShelfState::from_recipes(recipes, true);
        self
    }

    pub fn with_shelf_recipes_and_meta(
        mut self,
        recipes: &[luma_domain::Recipe],
        meta: &std::collections::BTreeMap<String, luma_domain::RecipeMetadata>,
    ) -> Self {
        self.shelf = ShelfState::from_recipes_with_meta(recipes, meta, true);
        self
    }

    pub fn header_text(&self) -> String {
        let detail = if self.status_detail.is_empty() {
            self.phase.header_label().to_string()
        } else {
            self.status_detail.clone()
        };
        format!(
            "{} · {}@{}:{} · {}",
            self.alias, self.user, self.hostname, self.port, detail
        )
    }

    pub fn apply_screen(&mut self, screen: &VtScreen) {
        self.lines = screen.render_lines();
        let (row, col) = screen.cursor();
        self.cursor_row = row;
        self.cursor_col = col;
    }

    /// Layout: whether the command shelf should occupy a side column.
    pub fn side_shelf_layout(terminal_width: u16) -> bool {
        terminal_width >= 118
    }

    pub fn overlay_shelf_layout(terminal_width: u16) -> bool {
        (80..118).contains(&terminal_width)
    }

    pub fn fullpage_shelf_layout(terminal_width: u16) -> bool {
        terminal_width < 80
    }

    pub fn shelf_width(terminal_width: u16) -> u16 {
        if terminal_width >= 118 {
            let remaining = terminal_width.saturating_sub(72);
            remaining.clamp(36, 44)
        } else {
            40
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_includes_phase() {
        let state = SshWorkspaceState::new(
            "prod".into(),
            "1.2.3.4".into(),
            "root".into(),
            22,
            "prod".into(),
            "/usr/bin/ssh".into(),
            vec![],
            vec![],
            Some("prod".into()),
            80,
            24,
        );
        assert!(state.header_text().contains("Connecting"));
        assert!(state.header_text().contains("root@1.2.3.4:22"));
    }

    #[test]
    fn layout_thresholds() {
        assert!(SshWorkspaceState::side_shelf_layout(118));
        assert!(!SshWorkspaceState::side_shelf_layout(117));
        assert!(SshWorkspaceState::overlay_shelf_layout(100));
        assert!(SshWorkspaceState::fullpage_shelf_layout(79));
        assert_eq!(SshWorkspaceState::shelf_width(160), 44);
    }
}
