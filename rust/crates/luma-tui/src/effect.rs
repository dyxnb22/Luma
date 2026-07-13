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
    /// No-op placeholder for redraw coalescing.
    None,
}
