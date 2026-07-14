#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Effect {
    /// Ask the engine (mock or real) to search. Runner owns I/O.
    Search { request_id: String, query: String },
    /// Cancel an in-flight search.
    CancelSearch { request_id: String },
    /// Request engine diagnostics for the doctor route.
    RunDoctor,
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
    /// No-op placeholder for redraw coalescing.
    None,
}
