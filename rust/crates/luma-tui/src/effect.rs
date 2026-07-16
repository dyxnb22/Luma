#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Effect {
    /// Ask the engine (mock or real) to search. Runner owns I/O.
    Search { request_id: String, query: String },
    /// Cancel an in-flight search.
    CancelSearch { request_id: String },
    /// Load settings projection for the Settings route.
    GetSettings,
    /// Toggle a module via engine registry + persistence when available.
    UpdateSettings {
        module_id: String,
        enabled: bool,
        expected_version: u64,
    },
    /// Load detail body for the preview pane.
    LoadPreview { result_id: String, preview_id: u64 },
    /// Refresh Hub windows slice + modules.
    LoadHub,
    /// Load wordbook review queue (`due` / `new` / `wrong`).
    LoadWordbookReview { queue: String },
    /// Refresh goal/due counters during an active review session.
    RefreshWordbookReviewStats,
    /// Reconcile UI after broadcast lag (cached engine results).
    GetSnapshot,
    /// Ask the engine for primary + secondary actions for a result.
    ListActions { result_id: String },
    /// Execute an action, optionally with confirmation.
    ExecuteAction {
        operation_id: String,
        result_id: String,
        action_id: String,
        confirmation: bool,
    },
    /// Cancel an in-flight action operation.
    CancelOperation { operation_id: String },
    /// Persist recipe run metadata after interactive execution.
    RecordRecipeRun {
        recipe_id: String,
        result: luma_domain::RecipeRunOutcome,
        now_unix: i64,
    },
    /// Run an interactive subprocess in the current terminal (TUI main thread only).
    RunInteractiveTerminal {
        program: String,
        args: Vec<String>,
        record_alias: Option<String>,
        operation_id: String,
    },
    /// No-op placeholder for redraw coalescing.
    None,
}
