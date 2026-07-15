//! Shared SQLite connection settings for LumaNext stores.

use rusqlite::Connection;
use std::path::Path;
use std::time::Duration;

const BUSY_TIMEOUT: Duration = Duration::from_millis(5000);

pub fn open_connection(path: &Path) -> rusqlite::Result<Connection> {
    let conn = Connection::open(path)?;
    let _ = conn.pragma_update(None, "journal_mode", "WAL");
    conn.busy_timeout(BUSY_TIMEOUT)?;
    Ok(conn)
}
