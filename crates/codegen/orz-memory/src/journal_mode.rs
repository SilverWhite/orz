use std::path::Path;
use rusqlite::{Connection, OpenFlags};

pub enum JournalMode { Wal }

impl JournalMode {
    pub fn for_db_path(_path: &Path) -> Self { Self::Wal }
    pub fn open(&self, path: &Path) -> rusqlite::Result<Connection> {
        let conn = Connection::open(path)?;
        // busy_timeout + journal pragma live here (see the callers' doc
        // comments — `index.rs`/`schema.rs`/`backend.rs` reference this helper
        // as the single place they are applied). The fork trimmed the enum to
        // `Wal`; the pragma application was dropped along with the other modes.
        conn.busy_timeout(std::time::Duration::from_millis(5000))?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        Ok(conn)
    }
    pub fn open_readonly(&self, path: &Path) -> rusqlite::Result<Connection> {
        Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX)
    }
}
