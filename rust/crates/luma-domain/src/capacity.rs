//! Personal-scale collection limits shared by storage adapters and in-memory test repositories.
//!
//! These are hard limits rather than silent truncation: once a collection is full, updating an
//! existing row remains valid but adding a new row must report a clear capacity error.

pub const MAX_SNIPPETS: usize = 1_000;
pub const MAX_QUICKLINKS: usize = 1_000;
pub const MAX_TIMERS: usize = 256;
pub const MAX_SSH_METADATA_ROWS: usize = 1_000;
pub const MAX_PINNED_CLIPBOARD_ROWS: usize = 100;
