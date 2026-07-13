#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Effect {
    /// Ask the engine (mock or real) to search. Runner owns I/O.
    Search { request_id: String, query: String },
    /// Cancel an in-flight search.
    CancelSearch { request_id: String },
    /// Request engine diagnostics for the doctor route.
    RunDoctor,
    /// Request a redacted diagnostics export.
    ExportDiagnostics,
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
