use crate::theme::{Symbols, Theme, ThemeMode};
use luma_domain::{FailureKind, SearchItem};
use luma_protocol::{ActionDescriptorDto, ActionOutcomeDto, Event};
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
            tone: StatusTone::Success,
        }
    }
}

/// Projection for rendering. Not the business source of truth.
#[derive(Clone, Debug)]
pub struct AppState {
    pub route: Route,
    pub prompt: String,
    /// Cursor as a Unicode scalar index into `prompt` (0..=char_count).
    pub prompt_cursor: usize,
    pub active_request: Option<String>,
    pub request_seq_seen: u64,
    pub search_generation: u64,
    pub results: ResultsView,
    pub status: StatusLine,
    pub should_quit: bool,
    pub dirty: bool,
    pub awaiting_actions: Option<AwaitingActions>,
    pub pending_action: Option<PendingAction>,
    pub action_choices: Vec<ActionDescriptorDto>,
    /// Result id that produced `action_choices` — never re-read selection on submit.
    pub action_result_id: Option<String>,
    pub action_selected: usize,
    pub active_operation: Option<String>,
    /// Resolved at init / by tests — render must not re-read the environment.
    pub theme: Theme,
    pub symbols: Symbols,
    /// Module id → display_name from SessionReady catalog.
    pub module_labels: HashMap<String, String>,
    /// Full module catalog from SessionReady (workbench metadata).
    pub module_catalog: Vec<ModuleCatalogEntry>,
    /// Hub windows slice (all visible apps; titles as `title · app`).
    pub hub_windows: Option<HubWindowsState>,
    /// When set, `FlushSearch` should fire after this Instant.
    pub search_debounce_deadline: Option<std::time::Instant>,
    /// Soft Hub windows refresh while the empty Hub is visible.
    pub hub_refresh_deadline: Option<std::time::Instant>,
    pub focus: FocusZone,
    /// Newest-first query strings submitted / flushed for search.
    pub query_history: Vec<String>,
    /// When browsing history with Ctrl-p/n; `None` means “live” prompt.
    pub history_browse: Option<usize>,
    /// Previous prompts when drilling into `n browse` / `proj browse` directories.
    /// Esc pops one level; cleared when the prompt is edited or fully cleared.
    pub browse_nav_stack: Vec<String>,
    /// Selected row in empty-state module Hub.
    pub hub_selected: usize,
    /// Settings route: selected row in module list.
    pub settings_selected: usize,
    pub settings_version: u64,
    pub settings_modules: Vec<SettingsModuleRow>,
    /// Notes / projects roots shown above module toggles.
    pub settings_roots: SettingsRootsInfo,
    /// Active wordbook review session (`wb review`).
    pub wordbook_review: Option<WordbookReviewState>,
    /// Horizontal scroll offset (Unicode scalar index) for long prompts.
    pub prompt_scroll: usize,
    /// When set, allow stacked preview on narrow terminals (e.g. 80×24).
    pub preview_pinned: bool,
    /// Help overlay scroll (line offset).
    pub help_scroll: usize,
    /// Recipe run deferred to the TUI loop (terminal suspend/resume).
    pub pending_recipe_run: Option<luma_domain::RecipeRunPlan>,
    /// Prompt to restore when leaving Commands / Settings / Help overlays.
    pub overlay_restore_prompt: Option<String>,
    /// Command palette selection.
    pub commands_selected: usize,
    /// Async preview body for the selected result (`LoadPreview`).
    pub preview_result_id: Option<String>,
    pub preview_body: Option<String>,
    /// Line offset when preview pane is focused.
    pub preview_scroll: usize,
    /// Monotonic preview request counter (correlated with `PreviewLoaded.preview_id`).
    pub preview_generation: u64,
    /// In-flight preview request id; `None` when idle.
    pub pending_preview_id: Option<u64>,
    /// First visible Hub row index.
    pub hub_scroll: usize,
    /// Last known terminal size — used to size the results viewport.
    pub term_width: u16,
    pub term_height: u16,
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

impl Default for AppState {
    fn default() -> Self {
        let mut state = Self {
            route: Route::Search,
            prompt: String::new(),
            prompt_cursor: 0,
            active_request: None,
            request_seq_seen: 0,
            search_generation: 0,
            results: ResultsView::default(),
            status: StatusLine::default(),
            should_quit: false,
            dirty: true,
            awaiting_actions: None,
            pending_action: None,
            action_choices: Vec::new(),
            action_result_id: None,
            action_selected: 0,
            active_operation: None,
            theme: Theme::resolve(ThemeMode::Auto),
            symbols: Symbols::detect(),
            module_labels: HashMap::new(),
            module_catalog: Vec::new(),
            hub_windows: None,
            search_debounce_deadline: None,
            hub_refresh_deadline: None,
            focus: FocusZone::Prompt,
            query_history: Vec::new(),
            history_browse: None,
            browse_nav_stack: Vec::new(),
            hub_selected: 0,
            settings_selected: 0,
            settings_version: 0,
            settings_modules: Vec::new(),
            settings_roots: SettingsRootsInfo::default(),
            wordbook_review: None,
            prompt_scroll: 0,
            preview_pinned: false,
            help_scroll: 0,
            pending_recipe_run: None,
            overlay_restore_prompt: None,
            commands_selected: 0,
            preview_result_id: None,
            preview_body: None,
            preview_scroll: 0,
            preview_generation: 0,
            pending_preview_id: None,
            hub_scroll: 0,
            term_width: 80,
            term_height: 24,
        };
        state.sync_results_viewport();
        state
    }
}

impl AppState {
    /// Empty Search route showing the module / windows Hub (not a result list).
    pub fn showing_hub(&self) -> bool {
        matches!(self.route, Route::Search)
            && self.prompt.is_empty()
            && self.results.items.is_empty()
    }

    pub fn selected_search_item(&self) -> Option<&luma_domain::SearchItem> {
        self.results.selected_id.as_ref().and_then(|id| {
            self.results
                .items
                .iter()
                .find(|item| item.id.as_str() == id.as_str())
        })
    }

    pub fn command_recipes_selected(&self) -> bool {
        self.selected_search_item()
            .is_some_and(|item| item.module_id.as_str() == "luma.command_recipes")
    }

    /// Soft Hub windows refresh interval while Hub is visible.
    pub const HUB_REFRESH_INTERVAL: std::time::Duration = std::time::Duration::from_secs(2);

    pub fn schedule_hub_refresh(&mut self) {
        if self.showing_hub() {
            self.hub_refresh_deadline =
                Some(std::time::Instant::now() + Self::HUB_REFRESH_INTERVAL);
        } else {
            self.hub_refresh_deadline = None;
        }
    }

    /// `win` / `window` / `windows` targeted search with results on screen.
    pub fn is_win_search(&self) -> bool {
        if !matches!(self.route, Route::Search) || self.results.items.is_empty() {
            return false;
        }
        let token = self
            .prompt
            .split_whitespace()
            .next()
            .unwrap_or("")
            .to_ascii_lowercase();
        matches!(token.as_str(), "win" | "window" | "windows")
    }

    /// Digit shortcuts for window focus: Hub (empty prompt) or win list when list is focused.
    pub fn should_intercept_window_digit(&self) -> bool {
        if self.route != Route::Search || self.active_operation.is_some() {
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
            let start = self.hub_scroll.min(rows.len());
            return rows
                .iter()
                .skip(start)
                .filter(|(kind, _, _, _)| kind == "window")
                .take(9)
                .map(|(_, id, title, _)| (id.clone(), title.clone()))
                .collect();
        }
        if self.is_win_search() {
            let start = self.results.scroll.min(self.results.items.len());
            return self
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
        let start = self.hub_scroll.min(rows.len());
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
        self.prompt.chars().count()
    }

    pub fn clamp_prompt_cursor(&mut self) {
        let len = self.prompt_char_len();
        if self.prompt_cursor > len {
            self.prompt_cursor = len;
        }
    }

    fn prompt_byte_index(&self, char_idx: usize) -> usize {
        self.prompt
            .char_indices()
            .nth(char_idx)
            .map(|(i, _)| i)
            .unwrap_or(self.prompt.len())
    }

    pub fn insert_prompt_char(&mut self, c: char) {
        self.clamp_prompt_cursor();
        let byte = self.prompt_byte_index(self.prompt_cursor);
        self.prompt.insert(byte, c);
        self.prompt_cursor += 1;
    }

    pub fn backspace_prompt(&mut self) {
        self.clamp_prompt_cursor();
        if self.prompt_cursor == 0 {
            return;
        }
        let del_at = self.prompt_cursor - 1;
        let start = self.prompt_byte_index(del_at);
        let end = self.prompt_byte_index(self.prompt_cursor);
        self.prompt.replace_range(start..end, "");
        self.prompt_cursor = del_at;
    }

    pub fn delete_forward_prompt(&mut self) {
        self.clamp_prompt_cursor();
        if self.prompt_cursor >= self.prompt_char_len() {
            return;
        }
        let start = self.prompt_byte_index(self.prompt_cursor);
        let end = self.prompt_byte_index(self.prompt_cursor + 1);
        self.prompt.replace_range(start..end, "");
    }

    pub fn clear_prompt(&mut self) {
        self.prompt.clear();
        self.prompt_cursor = 0;
    }

    /// Readline-style Ctrl-u: delete from start through character before cursor.
    pub fn clear_prompt_to_start(&mut self) {
        self.clamp_prompt_cursor();
        let end = self.prompt_byte_index(self.prompt_cursor);
        self.prompt.replace_range(0..end, "");
        self.prompt_cursor = 0;
    }

    /// Readline-style Ctrl-w: delete the word before the cursor.
    pub fn delete_prompt_word_back(&mut self) {
        self.clamp_prompt_cursor();
        if self.prompt_cursor == 0 {
            return;
        }
        let chars: Vec<char> = self.prompt.chars().collect();
        let mut i = self.prompt_cursor;
        while i > 0 && chars[i - 1].is_whitespace() {
            i -= 1;
        }
        while i > 0 && !chars[i - 1].is_whitespace() {
            i -= 1;
        }
        let start = self.prompt_byte_index(i);
        let end = self.prompt_byte_index(self.prompt_cursor);
        self.prompt.replace_range(start..end, "");
        self.prompt_cursor = i;
    }

    pub fn preview_side_by_side(&self) -> bool {
        self.term_width >= 100 && !self.results.items.is_empty()
    }

    /// Stacked preview under results; pin allows 80×24 manual preview.
    pub fn preview_stacked(&self) -> bool {
        if self.results.items.is_empty() || self.term_width >= 100 {
            return false;
        }
        if self.preview_pinned && self.term_width >= 60 {
            return true;
        }
        self.term_width >= 60 && self.term_height >= 28
    }

    pub fn preview_visible(&self) -> bool {
        self.preview_side_by_side() || self.preview_stacked()
    }

    pub fn sync_results_viewport(&mut self) {
        // Vertical chrome: prompt(3) + status(3); list borders(2); each row = 2 lines.
        let body = self.term_height.saturating_sub(6) as usize;
        let preview_take = if self.preview_stacked() { 8 } else { 0 };
        let list_inner = body.saturating_sub(2 + preview_take);
        let rows = (list_inner / 2).max(1);
        self.results.set_viewport_rows(rows);
        self.ensure_hub_selection_visible();
    }

    /// Header rows inserted by `hub_list_items` (each consumes one data-row slot).
    pub fn hub_header_slots(&self) -> usize {
        let rows = self.hub_rows();
        if rows.is_empty() {
            return 0;
        }
        let start = self.hub_scroll.min(rows.len());
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
        self.results
            .viewport_rows
            .saturating_sub(self.hub_header_slots())
            .max(1)
    }

    pub fn ensure_hub_selection_visible(&mut self) {
        let len = self.hub_rows().len();
        if len == 0 {
            self.hub_scroll = 0;
            self.hub_selected = 0;
            return;
        }
        if self.hub_selected >= len {
            self.hub_selected = len - 1;
        }
        let rows = self.hub_data_capacity();
        if self.hub_selected < self.hub_scroll {
            self.hub_scroll = self.hub_selected;
        } else if self.hub_selected >= self.hub_scroll + rows {
            self.hub_scroll = self.hub_selected + 1 - rows;
        }
        let max_scroll = len.saturating_sub(rows);
        if self.hub_scroll > max_scroll {
            self.hub_scroll = max_scroll;
        }
    }

    /// Keep `prompt_cursor` within the visible horizontal window (`inner_width` = prompt inner cols).
    pub fn ensure_prompt_visible(&mut self, inner_width: usize) {
        use unicode_width::UnicodeWidthStr;
        let budget = inner_width.saturating_sub(4).max(8);
        let chars: Vec<char> = self.prompt.chars().collect();
        if chars.is_empty() {
            self.prompt_scroll = 0;
            return;
        }
        if self.prompt_cursor < self.prompt_scroll {
            self.prompt_scroll = self.prompt_cursor;
        }
        loop {
            let before_cursor: String = chars
                .iter()
                .skip(self.prompt_scroll)
                .take(self.prompt_cursor.saturating_sub(self.prompt_scroll))
                .collect();
            let at_cursor = chars.get(self.prompt_cursor).copied().unwrap_or(' ');
            let line = format!("{before_cursor}{at_cursor}");
            if UnicodeWidthStr::width(line.as_str()) <= budget {
                break;
            }
            if self.prompt_scroll >= self.prompt_cursor {
                break;
            }
            self.prompt_scroll += 1;
        }
        while self.prompt_scroll > 0 {
            let before_cursor: String = chars
                .iter()
                .skip(self.prompt_scroll)
                .take(self.prompt_cursor.saturating_sub(self.prompt_scroll))
                .collect();
            let at_cursor = chars.get(self.prompt_cursor).copied().unwrap_or(' ');
            let line = format!("{before_cursor}{at_cursor}");
            if UnicodeWidthStr::width(line.as_str()) <= budget {
                break;
            }
            self.prompt_scroll += 1;
        }
        if self.prompt_scroll > self.prompt_cursor {
            self.prompt_scroll = self.prompt_cursor;
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
        self.query_history.retain(|h| h != q);
        self.query_history.insert(0, q.to_string());
        self.query_history.truncate(50);
        self.history_browse = None;
    }

    pub fn history_older(&mut self) {
        if self.query_history.is_empty() {
            return;
        }
        let next = match self.history_browse {
            None => 0,
            Some(i) => (i + 1).min(self.query_history.len() - 1),
        };
        self.history_browse = Some(next);
        self.browse_nav_stack.clear();
        self.prompt = self.query_history[next].clone();
        self.prompt_cursor = self.prompt_char_len();
    }

    pub fn history_newer(&mut self) {
        let Some(i) = self.history_browse else {
            return;
        };
        if i == 0 {
            self.history_browse = None;
            self.browse_nav_stack.clear();
            self.clear_prompt();
            return;
        }
        let next = i - 1;
        self.history_browse = Some(next);
        self.browse_nav_stack.clear();
        self.prompt = self.query_history[next].clone();
        self.prompt_cursor = self.prompt_char_len();
    }

    /// Sorted hub rows: windows then modules.
    /// Returns (kind, id, title, query_or_empty) where kind is
    /// "window" | "window_more" | "window_status" | "module".
    pub fn hub_rows(&self) -> Vec<(String, String, String, String)> {
        let mut rows = Vec::new();
        if let Some(hub) = &self.hub_windows {
            if let Some(title) = &hub.status_title {
                rows.push((
                    "window_status".into(),
                    "win:status".into(),
                    title.clone(),
                    "win ".into(),
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
                        "win ".into(),
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

    pub fn apply_engine_event(&mut self, event: Event) -> bool {
        match event {
            Event::SessionReady { modules } => {
                self.module_catalog = modules
                    .iter()
                    .map(|m| ModuleCatalogEntry {
                        id: m.id.clone(),
                        display_name: m.display_name.clone(),
                        enabled: m.enabled,
                        glyph: m.glyph.clone(),
                        suggested_query: m.suggested_query.clone(),
                        empty_hint: m.empty_hint.clone(),
                        supports_browse: m.supports_browse,
                        triggers: m.triggers.clone(),
                    })
                    .collect();
                self.module_labels = modules
                    .into_iter()
                    .map(|m| (m.id, m.display_name))
                    .collect();
                self.status.set("Session ready", StatusTone::Success);
                true
            }
            Event::HubLoaded { windows } => {
                self.hub_windows = windows.map(|w| HubWindowsState {
                    app_name: w.app_name,
                    windows: w
                        .windows
                        .into_iter()
                        .map(|row| HubWindowRow {
                            id: row.id,
                            title: row.title,
                        })
                        .collect(),
                    more: w.more,
                    status_kind: w.status.as_ref().map(|s| s.kind.clone()),
                    status_title: w.status.as_ref().map(|s| s.title.clone()),
                    status_subtitle: w.status.and_then(|s| s.subtitle),
                });
                self.ensure_hub_selection_visible();
                self.schedule_hub_refresh();
                true
            }
            Event::SnapshotLoaded {
                items,
                module_states: _,
            } => {
                self.results.items = items.into_iter().map(|d| d.into_domain()).collect();
                self.results.selected_id = self
                    .results
                    .items
                    .first()
                    .map(|i| i.id.as_str().to_string());
                self.results.scroll = 0;
                self.sync_results_viewport();
                self.status
                    .set("Resynced after lag", crate::view_model::StatusTone::Warning);
                true
            }
            Event::SearchStarted { request_id } => {
                if self.active_request.as_deref() == Some(request_id.as_str()) {
                    self.status.set("Searching…", StatusTone::Progress);
                    true
                } else {
                    false
                }
            }
            Event::ResultsReset { request_id } => {
                if self.active_request.as_deref() == Some(request_id.as_str()) {
                    self.results.items.clear();
                    self.results.selected_id = None;
                    self.request_seq_seen = 0;
                    true
                } else {
                    false
                }
            }
            Event::ResultsChunk {
                request_id,
                sequence,
                upserts,
                removed_ids,
            } => {
                // Empty request_id: module-disable eviction (engine purge). Apply removals
                // regardless of the active search so disabled-module rows leave the UI.
                if request_id.is_empty() {
                    if removed_ids.is_empty() {
                        return false;
                    }
                    self.results.apply_chunk(Vec::new(), &removed_ids);
                    return true;
                }
                if self.active_request.as_deref() != Some(request_id.as_str()) {
                    return false;
                }
                if sequence <= self.request_seq_seen {
                    return false;
                }
                self.request_seq_seen = sequence;
                let items: Vec<_> = upserts.into_iter().map(|d| d.into_domain()).collect();
                self.results.apply_chunk(items, &removed_ids);
                true
            }
            Event::SearchFinished {
                request_id,
                total,
                elapsed_ms,
            } => {
                if self.active_request.as_deref() == Some(request_id.as_str()) {
                    // End the active request so Esc Clear works on the first press.
                    self.active_request = None;
                    let incomplete = {
                        let raw = self.prompt.as_str();
                        let trimmed = raw.trim();
                        !trimmed.is_empty()
                            && !raw.ends_with(|c: char| c.is_whitespace())
                            && !trimmed.chars().any(|c| c.is_whitespace())
                            && self.module_catalog.iter().any(|m| {
                                m.enabled
                                    && m.triggers.iter().any(|t| t.eq_ignore_ascii_case(trimmed))
                            })
                    };
                    let (text, tone) = if incomplete {
                        ("Add space to search".into(), StatusTone::Neutral)
                    } else if total == 0 {
                        ("No results".into(), StatusTone::Neutral)
                    } else {
                        (format!("{elapsed_ms}ms"), StatusTone::Success)
                    };
                    self.status.set(text, tone);
                    true
                } else {
                    false
                }
            }
            Event::SearchCancelled { request_id } => {
                if self.active_request.as_deref() == Some(request_id.as_str()) {
                    self.active_request = None;
                    self.status.set("Search cancelled", StatusTone::Warning);
                    true
                } else {
                    false
                }
            }
            Event::Fatal {
                correlation_id: _,
                message,
            } => {
                self.status
                    .set(format!("Error: {message}"), StatusTone::Error);
                true
            }
            Event::ActionStarted { operation_id } => {
                if self.active_operation.as_deref() != Some(operation_id.as_str()) {
                    return false;
                }
                self.status.set("Running…", StatusTone::Progress);
                true
            }
            Event::WordbookReviewLoaded {
                queue,
                words,
                stats,
            } => {
                if !matches!(self.route, Route::WordbookReview) {
                    return false;
                }
                let word_items = words
                    .into_iter()
                    .map(|w| WordbookReviewWord {
                        id: w.id,
                        term: w.term,
                        phonetic: w.phonetic,
                        meaning: w.meaning,
                        example: w.example,
                    })
                    .collect::<Vec<_>>();
                let finished = word_items.is_empty();
                self.wordbook_review = Some(WordbookReviewState {
                    words: word_items,
                    index: 0,
                    revealed: false,
                    stats: WordbookReviewStats {
                        queue,
                        due: stats.due,
                        new_count: stats.new_count,
                        wrong: stats.wrong,
                        goal: stats.goal,
                        reviewed_today: stats.reviewed_today,
                        remaining_goal: stats.remaining_goal,
                        ..WordbookReviewStats::default()
                    },
                    finished,
                    pending_grade: None,
                });
                if finished {
                    self.status.set(
                        "review queue empty · try wb review new",
                        StatusTone::Warning,
                    );
                } else {
                    self.status
                        .set("review · Enter reveal · 1/2/3 grade", StatusTone::Neutral);
                }
                true
            }
            Event::ActionFinished {
                operation_id,
                outcome,
            } => {
                if self.active_operation.as_deref() != Some(operation_id.as_str()) {
                    return false;
                }
                self.active_operation = None;
                if let luma_protocol::ActionOutcomeDto::InteractiveRecipeRun { plan } = &outcome {
                    self.pending_recipe_run = Some((**plan).clone());
                    self.status
                        .set("recipe ready — running in terminal…", StatusTone::Progress);
                    return true;
                }
                if matches!(self.route, Route::WordbookReview) {
                    if matches!(outcome, luma_protocol::ActionOutcomeDto::Success { .. }) {
                        if let Some(review) = self.wordbook_review.as_mut() {
                            if let Some(action) = review.pending_grade.take() {
                                match action.as_str() {
                                    "known" => review.stats.session_known += 1,
                                    "fuzzy" => review.stats.session_fuzzy += 1,
                                    "unknown" => review.stats.session_unknown += 1,
                                    "mastered" => review.stats.session_mastered += 1,
                                    _ => {}
                                }
                            }
                            review.revealed = false;
                            review.index += 1;
                            if review.index >= review.words.len() {
                                review.finished = true;
                            }
                        }
                    } else if let Some(review) = self.wordbook_review.as_mut() {
                        // A cancelled grade must not remain armed for the next keypress.
                        review.pending_grade = None;
                    }
                    let tone = status_tone_for_outcome(&outcome);
                    if self.wordbook_review.as_ref().is_some_and(|r| r.finished) {
                        if let Some(review) = &self.wordbook_review {
                            self.status.set(
                                format!(
                                    "review done · K{} F{} U{} · goal {} · reviewed {}",
                                    review.stats.session_known,
                                    review.stats.session_fuzzy,
                                    review.stats.session_unknown,
                                    review.stats.goal,
                                    review.stats.reviewed_today
                                ),
                                StatusTone::Success,
                            );
                        }
                    } else {
                        self.status.set(outcome.user_message(), tone);
                    }
                    return true;
                }
                let tone = status_tone_for_outcome(&outcome);
                self.status.set(outcome.user_message(), tone);
                true
            }
            Event::WordbookReviewStatsUpdated { stats } => {
                if let Some(review) = self.wordbook_review.as_mut() {
                    review.stats.due = stats.due;
                    review.stats.new_count = stats.new_count;
                    review.stats.wrong = stats.wrong;
                    review.stats.goal = stats.goal;
                    review.stats.reviewed_today = stats.reviewed_today;
                    review.stats.remaining_goal = stats.remaining_goal;
                }
                true
            }
            Event::DiagnosticRaised { diagnostic } => {
                let settings_conflict =
                    diagnostic.get("settings_update").and_then(|v| v.as_str()) == Some("failed");
                if settings_conflict {
                    let message = diagnostic
                        .get("message")
                        .and_then(|v| v.as_str())
                        .unwrap_or("settings update failed");
                    self.status
                        .set(format!("settings conflict: {message}"), StatusTone::Warning);
                    return true;
                }
                false
            }
            Event::SettingsChanged { version, settings } => {
                self.settings_version = version;
                self.settings_modules.clear();
                self.settings_roots.notes_root = settings
                    .get("notes_root")
                    .and_then(|v| v.as_str())
                    .map(str::to_string);
                self.settings_roots.projects_roots = settings
                    .get("projects_roots")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(str::to_string))
                            .collect()
                    })
                    .unwrap_or_default();
                self.settings_roots.imported_projects = settings
                    .get("imported_projects")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| {
                                v.get("path").and_then(|p| p.as_str()).map(str::to_string)
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                self.settings_roots.loaded = true;
                if let Some(modules) = settings.get("modules").and_then(|v| v.as_array()) {
                    for row in modules {
                        let id = row
                            .get("id")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        if id.is_empty() {
                            continue;
                        }
                        let name = row
                            .get("name")
                            .and_then(|v| v.as_str())
                            .unwrap_or(id.as_str())
                            .to_string();
                        let enabled = row.get("enabled").and_then(|v| v.as_bool()).unwrap_or(true);
                        self.settings_modules
                            .push(SettingsModuleRow { id, name, enabled });
                    }
                }
                if self.settings_selected >= self.settings_modules.len()
                    && !self.settings_modules.is_empty()
                {
                    self.settings_selected = self.settings_modules.len() - 1;
                }
                self.module_labels = self
                    .settings_modules
                    .iter()
                    .map(|m| (m.id.clone(), m.name.clone()))
                    .collect();
                for row in &self.settings_modules {
                    if let Some(entry) = self.module_catalog.iter_mut().find(|m| m.id == row.id) {
                        entry.enabled = row.enabled;
                    }
                }
                self.status
                    .set(format!("settings v{version}"), StatusTone::Neutral);
                true
            }
            Event::PreviewLoaded {
                result_id,
                preview_id,
                body,
            } => {
                if self.pending_preview_id != Some(preview_id) {
                    return false;
                }
                if self.results.selected_id.as_deref() != Some(result_id.as_str()) {
                    return false;
                }
                self.pending_preview_id = None;
                self.preview_result_id = Some(result_id);
                self.preview_body = Some(body);
                self.preview_scroll = 0;
                true
            }
            Event::ActionsAvailable { result_id, actions } => {
                if self.awaiting_actions.is_none() {
                    self.status.set(
                        format!("{result_id}: {} actions", actions.len()),
                        StatusTone::Neutral,
                    );
                    return true;
                }
                self.status.set(
                    format!("{result_id}: {} actions", actions.len()),
                    StatusTone::Neutral,
                );
                true
            }
            Event::ModuleStateChanged { module_id, state } => {
                let enabled = state != "disabled";
                if let Some(entry) = self.module_catalog.iter_mut().find(|m| m.id == module_id) {
                    entry.enabled = enabled;
                }
                if let Some(row) = self.settings_modules.iter_mut().find(|m| m.id == module_id) {
                    row.enabled = enabled;
                }
                true
            }
        }
    }
}

fn status_tone_for_outcome(outcome: &ActionOutcomeDto) -> StatusTone {
    match outcome {
        ActionOutcomeDto::Success { .. } => StatusTone::Success,
        ActionOutcomeDto::Cancelled => StatusTone::Warning,
        ActionOutcomeDto::Failed { kind, .. } => status_tone_for_failure(kind),
        ActionOutcomeDto::InteractiveRecipeRun { .. } => StatusTone::Progress,
        ActionOutcomeDto::InteractiveTerminal { .. } => StatusTone::Progress,
    }
}

fn status_tone_for_failure(kind: &FailureKind) -> StatusTone {
    match kind {
        FailureKind::PermissionRequired { .. } => StatusTone::Permission,
        FailureKind::Warming { .. } => StatusTone::Progress,
        FailureKind::Cancelled => StatusTone::Warning,
        FailureKind::NotConfigured { .. } | FailureKind::Unavailable { .. } => StatusTone::Warning,
        FailureKind::Timeout { .. }
        | FailureKind::InvalidInput { .. }
        | FailureKind::NotFound { .. }
        | FailureKind::Conflict { .. }
        | FailureKind::SecurityDenied { .. }
        | FailureKind::Io { .. }
        | FailureKind::Internal { .. } => StatusTone::Error,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn action_started_ignored_without_active_operation() {
        let mut state = AppState::default();
        let applied = state.apply_engine_event(Event::ActionStarted {
            operation_id: "op-1".into(),
        });
        assert!(!applied);
        assert!(state.active_operation.is_none());
    }

    #[test]
    fn action_started_applies_when_operation_matches() {
        let mut state = AppState {
            active_operation: Some("op-1".into()),
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
            active_operation: Some("op-1".into()),
            ..AppState::default()
        };
        let applied = state.apply_engine_event(Event::ActionFinished {
            operation_id: "op-1".into(),
            outcome: ActionOutcomeDto::Cancelled,
        });
        assert!(applied);
        assert_eq!(state.status.tone, StatusTone::Warning);
        assert!(state.active_operation.is_none());
    }

    #[test]
    fn stale_action_finished_does_not_overwrite_status() {
        let mut state = AppState {
            active_operation: Some("op-current".into()),
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
        assert_eq!(state.active_operation.as_deref(), Some("op-current"));
        assert_eq!(state.status.text, "running current");
    }

    #[test]
    fn action_finished_not_configured_is_warning() {
        let mut state = AppState {
            active_operation: Some("op-2".into()),
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
            active_operation: Some("op-3".into()),
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
            active_operation: Some("op-4".into()),
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
            active_operation: Some("op-5".into()),
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
            term_width: 80,
            term_height: 28,
            ..AppState::default()
        };
        state.results.items.push(SearchItem {
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

        state.term_height = 24;
        assert!(!state.preview_stacked());
        assert!(!state.preview_visible());
    }

    #[test]
    fn empty_request_id_chunk_evicts_removed_ids() {
        use luma_domain::{ActionDescriptor, ActionId, ActionRisk, ModuleId, ResultId, SearchItem};

        let mut state = AppState {
            active_request: Some("req-1".into()),
            ..AppState::default()
        };
        state.results.items.push(SearchItem {
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
        assert!(state.results.items.is_empty());
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
            hub_windows: Some(HubWindowsState {
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
            prompt: "win ".into(),
            focus: FocusZone::List,
            ..Default::default()
        };
        state.results.items = (0..20)
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
        state.results.scroll = 4;
        let targets = state.window_digit_targets();
        assert_eq!(targets.first().map(|(id, _)| id.as_str()), Some("win:4"));
        assert_eq!(targets.get(8).map(|(id, _)| id.as_str()), Some("win:12"));
    }

    #[test]
    fn hub_window_digit_targets_follow_scroll_position() {
        let mut state = AppState {
            hub_windows: Some(HubWindowsState {
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
            ..Default::default()
        };
        state.hub_scroll = 4;
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
        let review = state.wordbook_review.as_ref().unwrap();
        assert!(review.finished);
        assert!(state.status.text.contains("empty"));
    }

    #[test]
    fn wordbook_review_stats_updated_refreshes_counters() {
        let mut state = AppState {
            route: Route::WordbookReview,
            wordbook_review: Some(WordbookReviewState {
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
        let review = state.wordbook_review.as_ref().unwrap();
        assert_eq!(review.stats.reviewed_today, 8);
        assert_eq!(review.stats.remaining_goal, 12);
        assert_eq!(review.stats.due, 5);
    }
}
