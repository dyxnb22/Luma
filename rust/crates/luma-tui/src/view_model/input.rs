use luma_domain::SearchItem;
use luma_protocol::ActionDescriptorDto;

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
