use std::path::Path;
use rusqlite::{Connection, OpenFlags};

pub enum JournalMode { Wal }

impl JournalMode {
    pub fn for_db_path(_path: &Path) -> Self { Self::Wal }
    pub fn open(&self, path: &Path) -> rusqlite::Result<Connection> {
        Connection::open(path)
    }
    pub fn open_readonly(&self, path: &Path) -> rusqlite::Result<Connection> {
        Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX)
    }
}
