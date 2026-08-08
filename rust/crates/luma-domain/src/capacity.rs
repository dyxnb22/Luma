//! Personal-scale collection limits shared by storage adapters and in-memory test repositories.
//!
//! These are hard limits rather than silent truncation: once a collection is full, updating an
//! existing row remains valid but adding a new row must report a clear capacity error.

pub const MAX_SNIPPETS: usize = 1_000;
pub const MAX_QUICKLINKS: usize = 1_000;
pub const MAX_TIMERS: usize = 256;
pub const MAX_PINNED_CLIPBOARD_ROWS: usize = 100;
pub const MAX_UNPINNED_CLIPBOARD_ROWS: usize = 500;

/// Input bounds are domain policy too: production adapters and their in-memory
/// test doubles must reject the same oversized user data.
pub const MAX_CLIPBOARD_ENTRY_BYTES: usize = 256 * 1024;
pub const MAX_QUICKLINK_TRIGGER_BYTES: usize = 1_024;
pub const MAX_QUICKLINK_URL_BYTES: usize = 64 * 1024;
pub const MAX_SNIPPET_TRIGGER_BYTES: usize = 1_024;
pub const MAX_SNIPPET_BODY_BYTES: usize = 64 * 1024;
