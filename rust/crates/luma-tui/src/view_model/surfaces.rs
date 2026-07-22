use luma_domain::RecipeRunPlan;

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
    pub pending_recipe_run: Option<RecipeRunPlan>,
}

/// Wordbook owns a review session independently of search results and action resolution.
#[derive(Clone, Debug, Default)]
pub struct WordbookState {
    pub review: Option<WordbookReviewState>,
}
