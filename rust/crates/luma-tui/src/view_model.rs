use crate::theme::{Symbols, Theme, ThemeMode};
use luma_domain::SearchItem;
use luma_protocol::ActionDescriptorDto;
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Route {
    Search,
    Help,
    Settings,
    WordbookReview,
    Commands,
    QuitConfirm,
    ConfirmAction,
    ActionPicker,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ActionsIntent {
    /// Enter: resolve primary (or first) action, then confirm/execute.
    Primary,
    /// Ctrl-k: show full action picker.
    Picker,
    /// Command Recipes shortcut (`r`/`c`/`f`).
    RecipeShortcut { action_id: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AwaitingActions {
    pub intent: ActionsIntent,
    pub result_id: String,
}

#[derive(Clone, Debug)]
pub struct PendingAction {
    pub result_id: String,
    pub action: ActionDescriptorDto,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum FocusZone {
    #[default]
    Prompt,
    List,
    Preview,
}

#[derive(Clone, Debug, Default)]
pub struct ResultsView {
    pub items: Vec<SearchItem>,
    pub selected_id: Option<String>,
    /// First visible result index (persisted; render must not invent scroll alone).
    pub scroll: usize,
    /// How many result *rows* fit in the list pane (each result uses 2 terminal lines).
    pub viewport_rows: usize,
}

impl ResultsView {
    pub fn selected_index(&self) -> Option<usize> {
        self.selected_id
            .as_ref()
            .and_then(|id| self.items.iter().position(|i| i.id.as_str() == id))
    }

    pub fn select_at(&mut self, idx: usize) {
        if self.items.is_empty() {
            return;
        }
        let idx = idx.min(self.items.len() - 1);
        self.selected_id = Some(self.items[idx].id.as_str().to_string());
        self.ensure_selection_visible();
    }

    pub fn select_offset(&mut self, delta: isize) {
        if self.items.is_empty() {
            return;
        }
        let idx = self.selected_index().unwrap_or(0) as isize;
        let next = (idx + delta).clamp(0, (self.items.len() - 1) as isize) as usize;
        self.select_at(next);
    }

    pub fn select_next(&mut self) {
        self.select_offset(1);
    }

    pub fn select_prev(&mut self) {
        self.select_offset(-1);
    }

    pub fn ensure_selection_visible(&mut self) {
        let Some(sel) = self.selected_index() else {
            self.scroll = 0;
            return;
        };
        let rows = self.viewport_rows.max(1);
        if sel < self.scroll {
            self.scroll = sel;
        } else if sel >= self.scroll + rows {
            self.scroll = sel + 1 - rows;
        }
        let max_scroll = self.items.len().saturating_sub(rows);
        if self.scroll > max_scroll {
            self.scroll = max_scroll;
        }
    }

    pub fn set_viewport_rows(&mut self, rows: usize) {
        self.viewport_rows = rows.max(1);
        self.ensure_selection_visible();
    }

    pub fn apply_chunk(&mut self, upserts: Vec<SearchItem>, removed_ids: &[String]) {
        for id in removed_ids {
            self.items.retain(|i| i.id.as_str() != id);
        }
        for item in upserts {
            if let Some(existing) = self.items.iter_mut().find(|i| i.id == item.id) {
                *existing = item;
            } else {
                self.items.push(item);
            }
        }
        self.items.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        // Defense-in-depth: mirror engine result cap so lag/resync cannot grow UI unbounded.
        const MAX_TUI_RESULTS: usize = 512;
        if self.items.len() > MAX_TUI_RESULTS {
            self.items.truncate(MAX_TUI_RESULTS);
        }
        if self.selected_id.is_none() {
            self.selected_id = self.items.first().map(|i| i.id.as_str().to_string());
        } else if let Some(sel) = &self.selected_id {
            if !self.items.iter().any(|i| i.id.as_str() == sel) {
                self.selected_id = self.items.first().map(|i| i.id.as_str().to_string());
            }
        }
        self.ensure_selection_visible();
    }
}

/// State that belongs to the prompt/search lifecycle: query text, request identity, result
/// projection, debounce, history, and browse navigation. Keeping these fields together makes
/// search resets explicit instead of scattering them across the whole application state.
#[derive(Clone, Debug, Default)]
pub struct SearchState {
    pub prompt: String,
    /// Cursor as a Unicode scalar index into `prompt` (0..=char_count).
    pub prompt_cursor: usize,
    /// Horizontal scroll offset (Unicode scalar index) for long prompts.
    pub prompt_scroll: usize,
    pub active_request: Option<String>,
    pub request_seq_seen: u64,
    pub search_generation: u64,
    pub results: ResultsView,
    /// When set, `FlushSearch` should fire after this Instant.
    pub debounce_deadline: Option<std::time::Instant>,
    /// Newest-first query strings submitted / flushed for search.
    pub query_history: Vec<String>,
    /// When browsing history with Ctrl-p/n; `None` means “live” prompt.
    pub history_browse: Option<usize>,
    /// Previous prompts when drilling into browse directories.
    pub browse_nav_stack: Vec<String>,
}

/// State that belongs to action resolution and execution. Review data remains separate because
/// it is a dedicated route/session with its own projection semantics.
#[derive(Clone, Debug, Default)]
pub struct ActionsState {
    /// Monotonic counter for action operation ids (separate from search request ids).
    pub operation_generation: u64,
    pub awaiting_actions: Option<AwaitingActions>,
    pub pending_action: Option<PendingAction>,
    pub action_choices: Vec<ActionDescriptorDto>,
    /// Result id that produced `action_choices` — never re-read selection on submit.
    pub action_result_id: Option<String>,
    pub action_selected: usize,
    pub active_operation: Option<String>,
}

/// Preview state is intentionally separate from search results: preview requests can outlive a
/// selection change and are guarded by their own generation.
#[derive(Clone, Debug, Default)]
pub struct PreviewState {
    /// When set, allow stacked preview on narrow terminals (e.g. 80×24).
    pub pinned: bool,
    /// Async preview body for the selected result (`LoadPreview`).
    pub result_id: Option<String>,
    pub body: Option<String>,
    /// Line offset when preview pane is focused.
    pub scroll: usize,
    /// Monotonic preview request counter (correlated with `PreviewLoaded.preview_id`).
    pub generation: u64,
    /// In-flight preview request id; `None` when idle.
    pub pending_id: Option<u64>,
}

/// Structured status tone — render colors from this, not string parsing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StatusTone {
    #[default]
    Neutral,
    Success,
    Progress,
    Warning,
    Error,
    Permission,
}

#[derive(Clone, Debug)]
pub struct StatusLine {
    pub text: String,
    pub tone: StatusTone,
}

impl StatusLine {
    pub fn set(&mut self, text: impl Into<String>, tone: StatusTone) {
        self.text = text.into();
        self.tone = tone;
    }
}

impl Default for StatusLine {
    fn default() -> Self {
        Self {
            text: "Ready".into(),
            tone: StatusTone::Neutral,
        }
    }
}

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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SettingsModuleRow {
    pub id: String,
    pub name: String,
    pub enabled: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SettingsRootsInfo {
    pub notes_root: Option<String>,
    pub projects_roots: Vec<String>,
    pub imported_projects: Vec<String>,
    /// True after at least one SettingsChanged event (avoids Hub false "set root").
    pub loaded: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WordbookReviewWord {
    pub id: i64,
    pub term: String,
    pub phonetic: String,
    pub meaning: String,
    pub example: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WordbookReviewStats {
    pub queue: String,
    pub due: i64,
    pub new_count: i64,
    pub wrong: i64,
    pub goal: i64,
    pub reviewed_today: i64,
    pub remaining_goal: i64,
    pub session_known: u32,
    pub session_fuzzy: u32,
    pub session_unknown: u32,
    pub session_skipped: u32,
    pub session_mastered: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WordbookReviewState {
    pub words: Vec<WordbookReviewWord>,
    pub index: usize,
    pub revealed: bool,
    pub stats: WordbookReviewStats,
    pub finished: bool,
    pub pending_grade: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModuleCatalogEntry {
    pub id: String,
    pub display_name: String,
    pub enabled: bool,
    pub glyph: Option<String>,
    pub suggested_query: Option<String>,
    pub empty_hint: Option<String>,
    pub supports_browse: bool,
    pub triggers: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HubWindowRow {
    pub id: String,
    pub title: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HubWindowsState {
    pub app_name: String,
    pub windows: Vec<HubWindowRow>,
    pub more: Option<u32>,
    pub status_kind: Option<String>,
    pub status_title: Option<String>,
    pub status_subtitle: Option<String>,
}

/// State owned by the empty-state Hub and the window switcher.
#[derive(Clone, Debug, Default)]
pub struct HubState {
    pub windows: Option<HubWindowsState>,
    pub refresh_deadline: Option<std::time::Instant>,
    pub selected: usize,
    pub scroll: usize,
}

/// State owned by the settings overlay and its optimistic versioned updates.
#[derive(Clone, Debug, Default)]
pub struct SettingsState {
    pub selected: usize,
    pub version: u64,
    pub modules: Vec<SettingsModuleRow>,
    pub roots: SettingsRootsInfo,
}

/// State owned by transient help/command overlays and prompt restoration.
#[derive(Clone, Debug, Default)]
pub struct OverlayState {
    pub help_scroll: usize,
    pub commands_selected: usize,
    pub restore_prompt: Option<String>,
}

/// Last known terminal geometry. Rendering and viewport calculations read this snapshot only.
#[derive(Clone, Copy, Debug)]
pub struct TerminalState {
    pub width: u16,
    pub height: u16,
}

impl Default for TerminalState {
    fn default() -> Self {
        Self {
            width: 80,
            height: 24,
        }
    }
}

/// Runtime hand-off state for effects that temporarily leave the TUI.
#[derive(Clone, Debug, Default)]
pub struct RuntimeState {
    pub pending_recipe_run: Option<luma_domain::RecipeRunPlan>,
}

/// Wordbook owns a review session independently of search results and action resolution.
#[derive(Clone, Debug, Default)]
pub struct WordbookState {
    pub review: Option<WordbookReviewState>,
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
mod tests {
    use super::*;
    use luma_domain::FailureKind;
    use luma_protocol::{ActionOutcomeDto, Event, SearchItemDto};

    #[test]
    fn action_started_ignored_without_active_operation() {
        let mut state = AppState::default();
        let applied = state.apply_engine_event(Event::ActionStarted {
            operation_id: "op-1".into(),
        });
        assert!(!applied);
        assert!(state.actions.active_operation.is_none());
    }

    #[test]
    fn action_started_applies_when_operation_matches() {
        let mut state = AppState {
            actions: ActionsState {
                active_operation: Some("op-1".into()),
                ..ActionsState::default()
            },
            ..AppState::default()
        };
        let applied = state.apply_engine_event(Event::ActionStarted {
            operation_id: "op-1".into(),
        });
        assert!(applied);
        assert_eq!(state.status.text, "Running…");
    }

    #[test]
    fn action_finished_ignored_without_active_operation() {
        let mut state = AppState::default();
        let applied = state.apply_engine_event(Event::ActionFinished {
            operation_id: "op-2".into(),
            outcome: ActionOutcomeDto::Success {
                message: Some("ok".into()),
            },
        });
        assert!(!applied);
    }

    #[test]
    fn action_finished_cancelled_is_warning() {
        let mut state = AppState {
            actions: ActionsState {
                active_operation: Some("op-1".into()),
                ..ActionsState::default()
            },
            ..AppState::default()
        };
        let applied = state.apply_engine_event(Event::ActionFinished {
            operation_id: "op-1".into(),
            outcome: ActionOutcomeDto::Cancelled,
        });
        assert!(applied);
        assert_eq!(state.status.tone, StatusTone::Warning);
        assert!(state.actions.active_operation.is_none());
    }

    #[test]
    fn stale_action_finished_does_not_overwrite_status() {
        let mut state = AppState {
            actions: ActionsState {
                active_operation: Some("op-current".into()),
                ..ActionsState::default()
            },
            ..AppState::default()
        };
        state.status.set("running current", StatusTone::Progress);
        let applied = state.apply_engine_event(Event::ActionFinished {
            operation_id: "op-old".into(),
            outcome: ActionOutcomeDto::Success {
                message: Some("stale ok".into()),
            },
        });
        assert!(!applied);
        assert_eq!(
            state.actions.active_operation.as_deref(),
            Some("op-current")
        );
        assert_eq!(state.status.text, "running current");
    }

    #[test]
    fn action_finished_not_configured_is_warning() {
        let mut state = AppState {
            actions: ActionsState {
                active_operation: Some("op-2".into()),
                ..ActionsState::default()
            },
            ..AppState::default()
        };
        let applied = state.apply_engine_event(Event::ActionFinished {
            operation_id: "op-2".into(),
            outcome: ActionOutcomeDto::failed(FailureKind::NotConfigured {
                remediation: "set notes_root".into(),
            }),
        });
        assert!(applied);
        assert_eq!(state.status.tone, StatusTone::Warning);
        assert!(state.status.text.contains("set notes_root"));
    }

    #[test]
    fn action_finished_unavailable_is_warning() {
        let mut state = AppState {
            actions: ActionsState {
                active_operation: Some("op-3".into()),
                ..ActionsState::default()
            },
            ..AppState::default()
        };
        let applied = state.apply_engine_event(Event::ActionFinished {
            operation_id: "op-3".into(),
            outcome: ActionOutcomeDto::failed(FailureKind::Unavailable {
                reason: "signed host required".into(),
                retryable: false,
            }),
        });
        assert!(applied);
        assert_eq!(state.status.tone, StatusTone::Warning);
        assert!(state.status.text.contains("signed host required"));
    }

    #[test]
    fn action_finished_permission_is_permission_tone() {
        let mut state = AppState {
            actions: ActionsState {
                active_operation: Some("op-4".into()),
                ..ActionsState::default()
            },
            ..AppState::default()
        };
        let applied = state.apply_engine_event(Event::ActionFinished {
            operation_id: "op-4".into(),
            outcome: ActionOutcomeDto::failed(FailureKind::PermissionRequired {
                capability: "accessibility".into(),
                guidance: "Open Settings".into(),
            }),
        });
        assert!(applied);
        assert_eq!(state.status.tone, StatusTone::Permission);
    }

    #[test]
    fn action_finished_success_is_success() {
        let mut state = AppState {
            actions: ActionsState {
                active_operation: Some("op-5".into()),
                ..ActionsState::default()
            },
            ..AppState::default()
        };
        let applied = state.apply_engine_event(Event::ActionFinished {
            operation_id: "op-5".into(),
            outcome: ActionOutcomeDto::Success {
                message: Some("Opened Safari".into()),
            },
        });
        assert!(applied);
        assert_eq!(state.status.tone, StatusTone::Success);
    }

    #[test]
    fn preview_stacked_on_tall_narrow_terminal() {
        use luma_domain::{ActionDescriptor, ActionId, ActionRisk, ModuleId, ResultId, SearchItem};

        let mut state = AppState {
            terminal: TerminalState {
                width: 80,
                height: 28,
            },
            ..AppState::default()
        };
        state.search.results.items.push(SearchItem {
            id: ResultId::new("1"),
            module_id: ModuleId::new("luma.notes"),
            title: "Note".into(),
            subtitle: None,
            kind: "note".into(),
            score: 1.0,
            primary_action: ActionDescriptor {
                id: ActionId::new("open"),
                label: "Open".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            },
            secondary_actions: vec![],
            ui_intent: None,
            action_payload: None,
        });
        assert!(!state.preview_side_by_side());
        assert!(state.preview_stacked());
        assert!(state.preview_visible());

        state.terminal.height = 24;
        assert!(state.preview_stacked());
        assert!(state.preview_visible());
        state.terminal.height = 23;
        assert!(!state.preview_stacked());
        assert!(!state.preview_visible());
    }

    #[test]
    fn empty_request_id_chunk_evicts_removed_ids() {
        use luma_domain::{ActionDescriptor, ActionId, ActionRisk, ModuleId, ResultId, SearchItem};

        let mut state = AppState {
            search: SearchState {
                active_request: Some("req-1".into()),
                ..SearchState::default()
            },
            ..AppState::default()
        };
        state.search.results.items.push(SearchItem {
            id: ResultId::new("clip:1"),
            module_id: ModuleId::new("luma.clipboard"),
            title: "x".into(),
            subtitle: None,
            kind: "clip".into(),
            score: 1.0,
            primary_action: ActionDescriptor {
                id: ActionId::new("copy"),
                label: "Copy".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            },
            secondary_actions: vec![],
            ui_intent: None,
            action_payload: None,
        });
        let applied = state.apply_engine_event(Event::ResultsChunk {
            request_id: String::new(),
            sequence: 0,
            upserts: vec![],
            removed_ids: vec!["clip:1".into()],
        });
        assert!(applied);
        assert!(state.search.results.items.is_empty());
    }

    #[test]
    fn module_state_changed_updates_catalog_enabled() {
        let mut state = AppState::default();
        state.module_catalog.push(ModuleCatalogEntry {
            id: "luma.projects".into(),
            display_name: "Projects".into(),
            enabled: true,
            glyph: None,
            suggested_query: None,
            empty_hint: None,
            supports_browse: false,
            triggers: vec![],
        });
        let applied = state.apply_engine_event(Event::ModuleStateChanged {
            module_id: "luma.projects".into(),
            state: "disabled".into(),
        });
        assert!(applied);
        assert!(!state.module_catalog[0].enabled);
    }

    #[test]
    fn hub_row_window_digit_skips_status_more_and_modules() {
        let state = AppState {
            hub: HubState {
                windows: Some(HubWindowsState {
                    app_name: "all".into(),
                    windows: vec![
                        HubWindowRow {
                            id: "win:1".into(),
                            title: "A".into(),
                        },
                        HubWindowRow {
                            id: "win:2".into(),
                            title: "B".into(),
                        },
                    ],
                    more: Some(3),
                    status_kind: Some("permission_required".into()),
                    status_title: Some("grant AX".into()),
                    status_subtitle: None,
                }),
                ..HubState::default()
            },
            module_catalog: vec![ModuleCatalogEntry {
                id: "luma.apps".into(),
                display_name: "Apps".into(),
                enabled: true,
                glyph: None,
                suggested_query: Some("app ".into()),
                empty_hint: None,
                supports_browse: false,
                triggers: vec![],
            }],
            ..Default::default()
        };
        let rows = state.hub_rows();
        let status_idx = rows
            .iter()
            .position(|(k, ..)| k == "window_status")
            .unwrap();
        let first_win_idx = rows.iter().position(|(k, ..)| k == "window").unwrap();
        let more_idx = rows.iter().position(|(k, ..)| k == "window_more").unwrap();
        let module_idx = rows.iter().position(|(k, ..)| k == "module").unwrap();
        assert_eq!(state.hub_row_window_digit(status_idx), None);
        assert_eq!(state.hub_row_window_digit(first_win_idx), Some(1));
        assert_eq!(state.hub_row_window_digit(first_win_idx + 1), Some(2));
        assert_eq!(state.hub_row_window_digit(more_idx), None);
        assert_eq!(state.hub_row_window_digit(module_idx), None);
    }

    #[test]
    fn window_digit_targets_follow_scroll_position() {
        let mut state = AppState {
            search: SearchState {
                prompt: "/win ".into(),
                ..SearchState::default()
            },
            focus: FocusZone::List,
            ..Default::default()
        };
        state.search.results.items = (0..20)
            .map(|i| SearchItem {
                id: luma_domain::ResultId::new(format!("win:{i}")),
                module_id: luma_domain::ModuleId::new("luma.windows"),
                title: format!("Window {i}"),
                subtitle: None,
                kind: "window".into(),
                score: 1.0,
                primary_action: luma_domain::ActionDescriptor {
                    id: luma_domain::ActionId::new("focus"),
                    label: "Focus".into(),
                    risk: luma_domain::ActionRisk::Safe,
                    confirmation: false,
                },
                secondary_actions: vec![],
                ui_intent: None,
                action_payload: None,
            })
            .collect();
        state.search.results.scroll = 4;
        let targets = state.window_digit_targets();
        assert_eq!(targets.first().map(|(id, _)| id.as_str()), Some("win:4"));
        assert_eq!(targets.get(8).map(|(id, _)| id.as_str()), Some("win:12"));
    }

    #[test]
    fn hub_window_digit_targets_follow_scroll_position() {
        let mut state = AppState {
            hub: HubState {
                windows: Some(HubWindowsState {
                    app_name: "all".into(),
                    windows: (0..12)
                        .map(|i| HubWindowRow {
                            id: format!("win:{i}"),
                            title: format!("Window {i}"),
                        })
                        .collect(),
                    more: None,
                    status_kind: None,
                    status_title: None,
                    status_subtitle: None,
                }),
                ..HubState::default()
            },
            ..Default::default()
        };
        state.hub.scroll = 4;
        assert_eq!(
            state
                .window_digit_targets()
                .first()
                .map(|(id, _)| id.as_str()),
            Some("win:4")
        );
        let rows = state.hub_rows();
        let row_index = rows.iter().position(|(_, id, _, _)| id == "win:4").unwrap();
        assert_eq!(state.hub_row_window_digit(row_index), Some(1));
    }

    #[test]
    fn wordbook_review_loaded_empty_finishes() {
        let mut state = AppState {
            route: Route::WordbookReview,
            ..Default::default()
        };
        let applied = state.apply_engine_event(Event::WordbookReviewLoaded {
            queue: "due".into(),
            words: vec![],
            stats: luma_protocol::WordbookStatsDto {
                due: 0,
                new_count: 0,
                wrong: 0,
                goal: 20,
                reviewed_today: 5,
                remaining_goal: 15,
            },
        });
        assert!(applied);
        let review = state.wordbook.review.as_ref().unwrap();
        assert!(review.finished);
        assert!(state.status.text.contains("empty"));
    }

    #[test]
    fn wordbook_review_stats_updated_refreshes_counters() {
        let mut state = AppState {
            route: Route::WordbookReview,
            wordbook: WordbookState {
                review: Some(WordbookReviewState {
                    words: vec![],
                    index: 0,
                    revealed: false,
                    stats: WordbookReviewStats {
                        reviewed_today: 3,
                        remaining_goal: 10,
                        ..Default::default()
                    },
                    finished: true,
                    pending_grade: None,
                }),
            },
            ..Default::default()
        };
        let applied = state.apply_engine_event(Event::WordbookReviewStatsUpdated {
            stats: luma_protocol::WordbookStatsDto {
                due: 5,
                new_count: 2,
                wrong: 1,
                goal: 20,
                reviewed_today: 8,
                remaining_goal: 12,
            },
        });
        assert!(applied);
        let review = state.wordbook.review.as_ref().unwrap();
        assert_eq!(review.stats.reviewed_today, 8);
        assert_eq!(review.stats.remaining_goal, 12);
        assert_eq!(review.stats.due, 5);
    }

    fn catalog_with_notes_trigger() -> Vec<ModuleCatalogEntry> {
        vec![ModuleCatalogEntry {
            id: "luma.notes".into(),
            display_name: "Notes".into(),
            enabled: true,
            glyph: None,
            suggested_query: Some("/n ".into()),
            empty_hint: None,
            supports_browse: true,
            triggers: vec!["n".into(), "note".into(), "notes".into()],
        }]
    }

    #[test]
    fn bare_n_is_not_incomplete_slash_trigger() {
        let state = AppState {
            search: SearchState {
                prompt: "n".into(),
                ..SearchState::default()
            },
            module_catalog: catalog_with_notes_trigger(),
            ..AppState::default()
        };
        assert!(state.incomplete_slash_trigger().is_none());
    }

    #[test]
    fn slash_n_is_incomplete_slash_trigger() {
        let state = AppState {
            search: SearchState {
                prompt: "/n".into(),
                ..SearchState::default()
            },
            module_catalog: catalog_with_notes_trigger(),
            ..AppState::default()
        };
        assert_eq!(state.incomplete_slash_trigger().as_deref(), Some("n"));
    }

    #[test]
    fn snapshot_loaded_sorts_by_score() {
        let mut state = AppState::default();
        let applied = state.apply_engine_event(Event::SnapshotLoaded {
            items: vec![
                SearchItemDto {
                    id: "low".into(),
                    module_id: "luma.notes".into(),
                    title: "low".into(),
                    score: 1.0,
                    ..SearchItemDto::default()
                },
                SearchItemDto {
                    id: "high".into(),
                    module_id: "luma.notes".into(),
                    title: "high".into(),
                    score: 9.0,
                    ..SearchItemDto::default()
                },
            ],
            module_states: Default::default(),
        });
        assert!(applied);
        assert_eq!(state.search.results.items[0].id.as_str(), "high");
        assert_eq!(state.search.results.selected_id.as_deref(), Some("high"));
    }

    #[test]
    fn snapshot_loaded_ignored_during_active_search() {
        let mut state = AppState {
            search: SearchState {
                active_request: Some("req-live".into()),
                ..SearchState::default()
            },
            ..AppState::default()
        };
        state.search.results.items.push(luma_domain::SearchItem {
            id: luma_domain::ResultId::new("keep"),
            module_id: luma_domain::ModuleId::new("luma.notes"),
            title: "keep".into(),
            subtitle: None,
            kind: "note".into(),
            score: 1.0,
            primary_action: luma_domain::ActionDescriptor {
                id: luma_domain::ActionId::new("open"),
                label: "Open".into(),
                risk: luma_domain::ActionRisk::Safe,
                confirmation: false,
            },
            secondary_actions: vec![],
            ui_intent: None,
            action_payload: None,
        });
        let applied = state.apply_engine_event(Event::SnapshotLoaded {
            items: vec![SearchItemDto {
                id: "stale".into(),
                module_id: "luma.notes".into(),
                title: "stale".into(),
                score: 9.0,
                ..SearchItemDto::default()
            }],
            module_states: Default::default(),
        });
        assert!(!applied);
        assert_eq!(state.search.results.items[0].id.as_str(), "keep");
    }

    #[test]
    fn preview_loaded_clears_pending_on_selection_mismatch() {
        let mut state = AppState {
            preview: PreviewState {
                pending_id: Some(3),
                result_id: Some("note:a".into()),
                ..PreviewState::default()
            },
            search: SearchState {
                results: ResultsView {
                    selected_id: Some("note:b".into()),
                    ..ResultsView::default()
                },
                ..SearchState::default()
            },
            ..AppState::default()
        };
        let applied = state.apply_engine_event(Event::PreviewLoaded {
            result_id: "note:a".into(),
            preview_id: 3,
            body: "body".into(),
        });
        assert!(!applied);
        assert!(state.preview.pending_id.is_none());
        assert!(state.preview.body.is_none());
    }

    #[test]
    fn search_finished_bare_n_is_not_add_space_hint() {
        let mut state = AppState {
            search: SearchState {
                prompt: "n".into(),
                active_request: Some("req-1".into()),
                ..SearchState::default()
            },
            module_catalog: catalog_with_notes_trigger(),
            ..AppState::default()
        };
        let applied = state.apply_engine_event(Event::SearchFinished {
            request_id: "req-1".into(),
            total: 0,
            elapsed_ms: 12,
        });
        assert!(applied);
        assert_eq!(state.status.text, "No results");
    }

    #[test]
    fn search_finished_slash_n_shows_add_space_hint() {
        let mut state = AppState {
            search: SearchState {
                prompt: "/n".into(),
                active_request: Some("req-1".into()),
                ..SearchState::default()
            },
            module_catalog: catalog_with_notes_trigger(),
            ..AppState::default()
        };
        let applied = state.apply_engine_event(Event::SearchFinished {
            request_id: "req-1".into(),
            total: 0,
            elapsed_ms: 12,
        });
        assert!(applied);
        assert_eq!(state.status.text, "Add space to search");
    }
}
