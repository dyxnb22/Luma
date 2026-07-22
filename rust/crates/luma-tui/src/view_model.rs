use crate::theme::{Symbols, Theme, ThemeMode};
use std::collections::HashMap;

mod input;
mod status;
mod surfaces;

pub use input::*;
pub use status::*;
pub use surfaces::*;

#[cfg(test)]
use luma_domain::SearchItem;

/// Projection for rendering. Not the business source of truth.
#[derive(Clone, Debug)]
pub struct AppState {
    pub route: Route,
    pub search: SearchState,
    pub actions: ActionsState,
    pub preview: PreviewState,
    pub status: StatusLine,
    pub should_quit: bool,
    pub dirty: bool,
    /// Resolved at init / by tests — render must not re-read the environment.
    pub theme: Theme,
    pub symbols: Symbols,
    /// Module id → display_name from SessionReady catalog.
    pub module_labels: HashMap<String, String>,
    /// Full module catalog from SessionReady (workbench metadata).
    pub module_catalog: Vec<ModuleCatalogEntry>,
    /// Empty-state module hub and window switcher state.
    pub hub: HubState,
    pub focus: FocusZone,
    /// Settings overlay and its versioned module/root projection.
    pub settings: SettingsState,
    /// Active wordbook review session (`/wb review`).
    pub wordbook: WordbookState,
    /// Help, command palette, and overlay prompt restoration.
    pub overlay: OverlayState,
    /// Deferred hand-offs that leave the TUI temporarily.
    pub runtime: RuntimeState,
    /// Last known terminal size — used to size the results viewport.
    pub terminal: TerminalState,
}

impl Default for AppState {
    fn default() -> Self {
        let mut state = Self {
            route: Route::Search,
            search: SearchState::default(),
            actions: ActionsState::default(),
            preview: PreviewState::default(),
            status: StatusLine::default(),
            should_quit: false,
            dirty: true,
            theme: Theme::resolve(ThemeMode::Auto),
            symbols: Symbols::detect(),
            module_labels: HashMap::new(),
            module_catalog: Vec::new(),
            hub: HubState::default(),
            focus: FocusZone::Prompt,
            settings: SettingsState::default(),
            wordbook: WordbookState::default(),
            overlay: OverlayState::default(),
            runtime: RuntimeState::default(),
            terminal: TerminalState::default(),
        };
        state.sync_results_viewport();
        state
    }
}

impl AppState {
    /// Empty Search route showing the module / windows Hub (not a result list).
    pub fn showing_hub(&self) -> bool {
        matches!(self.route, Route::Search)
            && self.search.prompt.is_empty()
            && self.search.results.items.is_empty()
    }

    pub fn selected_search_item(&self) -> Option<&luma_domain::SearchItem> {
        self.search.results.selected_id.as_ref().and_then(|id| {
            self.search
                .results
                .items
                .iter()
                .find(|item| item.id.as_str() == id.as_str())
        })
    }

    /// Slash-prefixed bare module trigger (`/n`, not `n` or `/n `).
    /// Unprefixed input is always a global search under the strict command format.
    pub fn incomplete_slash_trigger(&self) -> Option<String> {
        let is_prefix = |token: &str| {
            token == "help"
                || self
                    .module_catalog
                    .iter()
                    .any(|m| m.enabled && m.triggers.iter().any(|t| t.eq_ignore_ascii_case(token)))
        };
        let query =
            luma_domain::Query::parse_with_prefixes_strict(&self.search.prompt, 50, is_prefix);
        if !query.is_incomplete_trigger(is_prefix) {
            return None;
        }
        Some(
            luma_domain::strip_command_prefix(&self.search.prompt)
                .trim()
                .to_ascii_lowercase(),
        )
    }

    pub fn command_recipes_selected(&self) -> bool {
        self.selected_search_item()
            .is_some_and(|item| item.module_id.as_str() == "luma.command_recipes")
    }

    /// Soft Hub windows refresh interval while Hub is visible.
    pub const HUB_REFRESH_INTERVAL: std::time::Duration = std::time::Duration::from_secs(2);

    pub fn schedule_hub_refresh(&mut self) {
        if self.showing_hub() {
            self.hub.refresh_deadline =
                Some(std::time::Instant::now() + Self::HUB_REFRESH_INTERVAL);
        } else {
            self.hub.refresh_deadline = None;
        }
    }

    /// Slash-prefixed `/win` / `/window` / `/windows` targeted search with results on screen.
    pub fn is_win_search(&self) -> bool {
        if !matches!(self.route, Route::Search) || self.search.results.items.is_empty() {
            return false;
        }
        let Some(token) = self.search.prompt.trim_start().strip_prefix('/') else {
            return false;
        };
        let token = token
            .split_whitespace()
            .next()
            .unwrap_or("")
            .to_ascii_lowercase();
        matches!(token.as_str(), "win" | "window" | "windows")
    }

    /// Digit shortcuts for window focus: Hub (empty prompt) or win list when list is focused.
    pub fn should_intercept_window_digit(&self) -> bool {
        if self.route != Route::Search || self.actions.active_operation.is_some() {
            return false;
        }
        if self.showing_hub() {
            return true;
        }
        self.is_win_search() && self.focus == FocusZone::List
    }

    /// First nine focusable window rows: `(result_id, title)` — hub windows or win search rows.
    pub fn window_digit_targets(&self) -> Vec<(String, String)> {
        if self.showing_hub() {
            let rows = self.hub_rows();
            let start = self.hub.scroll.min(rows.len());
            return rows
                .iter()
                .skip(start)
                .filter(|(kind, _, _, _)| kind == "window")
                .take(9)
                .map(|(_, id, title, _)| (id.clone(), title.clone()))
                .collect();
        }
        if self.is_win_search() {
            let start = self
                .search
                .results
                .scroll
                .min(self.search.results.items.len());
            return self
                .search
                .results
                .items
                .iter()
                .skip(start)
                .filter(|i| i.module_id.as_str() == "luma.windows" && i.kind == "window")
                .take(9)
                .map(|i| (i.id.as_str().to_string(), i.title.clone()))
                .collect();
        }
        Vec::new()
    }

    /// 1-based digit label for a hub row index, if that row is a focusable window in 1..=9.
    pub fn hub_row_window_digit(&self, row_index: usize) -> Option<usize> {
        let rows = self.hub_rows();
        let start = self.hub.scroll.min(rows.len());
        let mut window_idx = 0usize;
        for (i, (kind, _, _, _)) in rows.iter().enumerate().skip(start) {
            if kind != "window" {
                continue;
            }
            window_idx += 1;
            if i == row_index && window_idx <= 9 {
                return Some(window_idx);
            }
        }
        None
    }

    pub fn prompt_char_len(&self) -> usize {
        self.search.prompt.chars().count()
    }

    pub fn clamp_prompt_cursor(&mut self) {
        let len = self.prompt_char_len();
        if self.search.prompt_cursor > len {
            self.search.prompt_cursor = len;
        }
    }

    fn prompt_byte_index(&self, char_idx: usize) -> usize {
        self.search
            .prompt
            .char_indices()
            .nth(char_idx)
            .map(|(i, _)| i)
            .unwrap_or(self.search.prompt.len())
    }

    pub fn insert_prompt_char(&mut self, c: char) {
        self.clamp_prompt_cursor();
        let byte = self.prompt_byte_index(self.search.prompt_cursor);
        self.search.prompt.insert(byte, c);
        self.search.prompt_cursor += 1;
    }

    pub fn backspace_prompt(&mut self) {
        self.clamp_prompt_cursor();
        if self.search.prompt_cursor == 0 {
            return;
        }
        let del_at = self.search.prompt_cursor - 1;
        let start = self.prompt_byte_index(del_at);
        let end = self.prompt_byte_index(self.search.prompt_cursor);
        self.search.prompt.replace_range(start..end, "");
        self.search.prompt_cursor = del_at;
    }

    pub fn delete_forward_prompt(&mut self) {
        self.clamp_prompt_cursor();
        if self.search.prompt_cursor >= self.prompt_char_len() {
            return;
        }
        let start = self.prompt_byte_index(self.search.prompt_cursor);
        let end = self.prompt_byte_index(self.search.prompt_cursor + 1);
        self.search.prompt.replace_range(start..end, "");
    }

    pub fn clear_prompt(&mut self) {
        self.search.prompt.clear();
        self.search.prompt_cursor = 0;
    }

    /// Readline-style Ctrl-u: delete from start through character before cursor.
    pub fn clear_prompt_to_start(&mut self) {
        self.clamp_prompt_cursor();
        let end = self.prompt_byte_index(self.search.prompt_cursor);
        self.search.prompt.replace_range(0..end, "");
        self.search.prompt_cursor = 0;
    }

    /// Readline-style Ctrl-w: delete the word before the cursor.
    pub fn delete_prompt_word_back(&mut self) {
        self.clamp_prompt_cursor();
        if self.search.prompt_cursor == 0 {
            return;
        }
        let chars: Vec<char> = self.search.prompt.chars().collect();
        let mut i = self.search.prompt_cursor;
        while i > 0 && chars[i - 1].is_whitespace() {
            i -= 1;
        }
        while i > 0 && !chars[i - 1].is_whitespace() {
            i -= 1;
        }
        let start = self.prompt_byte_index(i);
        let end = self.prompt_byte_index(self.search.prompt_cursor);
        self.search.prompt.replace_range(start..end, "");
        self.search.prompt_cursor = i;
    }

    pub fn preview_side_by_side(&self) -> bool {
        self.terminal.width >= 100 && !self.search.results.items.is_empty()
    }

    /// Stacked preview under results; pin allows 80×24 manual preview.
    pub fn preview_stacked(&self) -> bool {
        if self.search.results.items.is_empty() || self.terminal.width >= 100 {
            return false;
        }
        if self.preview.pinned && self.terminal.width >= 60 {
            return true;
        }
        self.terminal.width >= 60 && self.terminal.height >= 24
    }

    pub fn preview_visible(&self) -> bool {
        self.preview_side_by_side() || self.preview_stacked()
    }

    pub fn sync_results_viewport(&mut self) {
        // Vertical chrome: prompt(3) + status(3); list borders(2); each row = 2 lines.
        let body = self.terminal.height.saturating_sub(6) as usize;
        let preview_take = if self.preview_stacked() { 8 } else { 0 };
        let list_inner = body.saturating_sub(2 + preview_take);
        let rows = (list_inner / 2).max(1);
        self.search.results.set_viewport_rows(rows);
        self.ensure_hub_selection_visible();
    }

    /// Header rows inserted by `hub_list_items` (each consumes one data-row slot).
    pub fn hub_header_slots(&self) -> usize {
        let rows = self.hub_rows();
        if rows.is_empty() {
            return 0;
        }
        let start = self.hub.scroll.min(rows.len());
        let module_start = rows.iter().position(|(k, _, _, _)| k == "module");
        let mut slots = 0;
        if start == 0
            && rows
                .get(start)
                .is_some_and(|(k, _, _, _)| k.starts_with("window"))
        {
            slots += 1;
        }
        if module_start == Some(start) {
            slots += 1;
        }
        slots
    }

    pub fn hub_data_capacity(&self) -> usize {
        self.search
            .results
            .viewport_rows
            .saturating_sub(self.hub_header_slots())
            .max(1)
    }

    pub fn ensure_hub_selection_visible(&mut self) {
        let len = self.hub_rows().len();
        if len == 0 {
            self.hub.scroll = 0;
            self.hub.selected = 0;
            return;
        }
        if self.hub.selected >= len {
            self.hub.selected = len - 1;
        }
        let rows = self.hub_data_capacity();
        if self.hub.selected < self.hub.scroll {
            self.hub.scroll = self.hub.selected;
        } else if self.hub.selected >= self.hub.scroll + rows {
            self.hub.scroll = self.hub.selected + 1 - rows;
        }
        let max_scroll = len.saturating_sub(rows);
        if self.hub.scroll > max_scroll {
            self.hub.scroll = max_scroll;
        }
    }

    /// Keep `prompt_cursor` within the visible horizontal window (`inner_width` = prompt inner cols).
    pub fn ensure_prompt_visible(&mut self, inner_width: usize) {
        use unicode_width::UnicodeWidthStr;
        let budget = inner_width.saturating_sub(4).max(8);
        let chars: Vec<char> = self.search.prompt.chars().collect();
        if chars.is_empty() {
            self.search.prompt_scroll = 0;
            return;
        }
        if self.search.prompt_cursor < self.search.prompt_scroll {
            self.search.prompt_scroll = self.search.prompt_cursor;
        }
        loop {
            let before_cursor: String = chars
                .iter()
                .skip(self.search.prompt_scroll)
                .take(
                    self.search
                        .prompt_cursor
                        .saturating_sub(self.search.prompt_scroll),
                )
                .collect();
            let at_cursor = chars.get(self.search.prompt_cursor).copied().unwrap_or(' ');
            let line = format!("{before_cursor}{at_cursor}");
            if UnicodeWidthStr::width(line.as_str()) <= budget {
                break;
            }
            if self.search.prompt_scroll >= self.search.prompt_cursor {
                break;
            }
            self.search.prompt_scroll += 1;
        }
        while self.search.prompt_scroll > 0 {
            let before_cursor: String = chars
                .iter()
                .skip(self.search.prompt_scroll)
                .take(
                    self.search
                        .prompt_cursor
                        .saturating_sub(self.search.prompt_scroll),
                )
                .collect();
            let at_cursor = chars.get(self.search.prompt_cursor).copied().unwrap_or(' ');
            let line = format!("{before_cursor}{at_cursor}");
            if UnicodeWidthStr::width(line.as_str()) <= budget {
                break;
            }
            self.search.prompt_scroll += 1;
        }
        if self.search.prompt_scroll > self.search.prompt_cursor {
            self.search.prompt_scroll = self.search.prompt_cursor;
        }
    }

    pub fn cycle_focus(&mut self) {
        self.focus = match self.focus {
            FocusZone::Prompt => FocusZone::List,
            FocusZone::List if self.preview_visible() => FocusZone::Preview,
            FocusZone::List => FocusZone::Prompt,
            FocusZone::Preview => FocusZone::Prompt,
        };
    }

    pub fn push_query_history(&mut self, query: &str) {
        let q = query.trim();
        if q.is_empty() {
            return;
        }
        self.search.query_history.retain(|h| h != q);
        self.search.query_history.insert(0, q.to_string());
        self.search.query_history.truncate(50);
        self.search.history_browse = None;
    }

    pub fn history_older(&mut self) {
        if self.search.query_history.is_empty() {
            return;
        }
        let next = match self.search.history_browse {
            None => 0,
            Some(i) => (i + 1).min(self.search.query_history.len() - 1),
        };
        self.search.history_browse = Some(next);
        self.search.browse_nav_stack.clear();
        self.search.prompt = self.search.query_history[next].clone();
        self.search.prompt_cursor = self.prompt_char_len();
    }

    pub fn history_newer(&mut self) {
        let Some(i) = self.search.history_browse else {
            return;
        };
        if i == 0 {
            self.search.history_browse = None;
            self.search.browse_nav_stack.clear();
            self.clear_prompt();
            return;
        }
        let next = i - 1;
        self.search.history_browse = Some(next);
        self.search.browse_nav_stack.clear();
        self.search.prompt = self.search.query_history[next].clone();
        self.search.prompt_cursor = self.prompt_char_len();
    }

    /// Sorted hub rows: windows then modules.
    /// Returns (kind, id, title, query_or_empty) where kind is
    /// "window" | "window_more" | "window_status" | "module".
    pub fn hub_rows(&self) -> Vec<(String, String, String, String)> {
        let mut rows = Vec::new();
        if let Some(hub) = &self.hub.windows {
            if let Some(title) = &hub.status_title {
                rows.push((
                    "window_status".into(),
                    "win:status".into(),
                    title.clone(),
                    "/win ".into(),
                ));
            }
            for w in &hub.windows {
                rows.push((
                    "window".into(),
                    w.id.clone(),
                    w.title.clone(),
                    String::new(),
                ));
            }
            if let Some(n) = hub.more {
                if n > 0 {
                    rows.push((
                        "window_more".into(),
                        "win:more".into(),
                        format!("{n} more → win"),
                        "/win ".into(),
                    ));
                }
            }
        }
        let mut modules: Vec<_> = self
            .module_catalog
            .iter()
            .filter(|m| m.enabled)
            .cloned()
            .collect();
        if modules.is_empty() {
            // Fallback while SessionReady catalog is empty.
            modules = self
                .module_labels
                .iter()
                .map(|(id, name)| ModuleCatalogEntry {
                    id: id.clone(),
                    display_name: name.clone(),
                    enabled: true,
                    glyph: None,
                    suggested_query: None,
                    empty_hint: None,
                    supports_browse: false,
                    triggers: vec![],
                })
                .collect();
        }
        modules.sort_by(|a, b| {
            a.display_name
                .to_lowercase()
                .cmp(&b.display_name.to_lowercase())
        });
        for m in modules {
            let query = m.suggested_query.clone().unwrap_or_else(|| {
                let key =
                    m.id.strip_prefix("luma.")
                        .unwrap_or(m.id.as_str())
                        .split('.')
                        .next()
                        .unwrap_or(m.id.as_str());
                let trigger = match key {
                    "apps" => "app",
                    "windows" => "win",
                    "clipboard" => "clip",
                    "notes" => "n",
                    "quicklinks" => "ql",
                    "snippets" => "s",
                    "projects" => "proj",
                    other => other,
                };
                format!("{trigger} ")
            });
            let query = if query.trim_start().starts_with('/') {
                query
            } else {
                format!("/{query}")
            };
            rows.push(("module".into(), m.id.clone(), m.display_name.clone(), query));
        }
        rows
    }

    /// Sorted hub rows: (module_id, display_name, suggested_query).
    pub fn hub_entries(&self) -> Vec<(String, String, String)> {
        self.hub_rows()
            .into_iter()
            .filter(|(kind, _, _, _)| kind == "module")
            .map(|(_, id, name, query)| (id, name, query))
            .collect()
    }
}

#[cfg(test)]
mod tests;
