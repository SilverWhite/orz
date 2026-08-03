use std::path::{Path, PathBuf};
use rusqlite::Connection;

#[derive(Clone, Copy)]
pub enum JournalMode { Wal, Memory }
impl JournalMode {
    pub fn for_db_path(_: &Path) -> Self { Self::Wal }
    pub fn effective_db_path(&self, p: &Path) -> PathBuf { p.to_path_buf() }
    pub fn open_readonly(&self, path: &Path) -> rusqlite::Result<Connection> {
        Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX)
    }
    pub fn open(&self, path: &Path) -> rusqlite::Result<Connection> { Connection::open(path) }
}
