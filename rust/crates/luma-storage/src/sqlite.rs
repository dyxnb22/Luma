//! Shared SQLite connection settings for LumaNext stores.

use rusqlite::{Connection, OpenFlags};
use std::path::Path;
use std::time::Duration;

const BUSY_TIMEOUT: Duration = Duration::from_millis(5000);

pub fn open_connection(path: &Path) -> rusqlite::Result<Connection> {
    let conn = Connection::open(path)?;
    let _ = conn.pragma_update(None, "journal_mode", "WAL");
    conn.busy_timeout(BUSY_TIMEOUT)?;
    Ok(conn)
}

/// Open an existing DB read-only. Does not create the file and does not change journal_mode.
pub fn open_readonly_connection(path: &Path) -> rusqlite::Result<Connection> {
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    conn.busy_timeout(BUSY_TIMEOUT)?;
    Ok(conn)
}
