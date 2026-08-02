// Local replacement for deleted xai_sqlite_journal::JournalMode.
use rusqlite::Connection;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy)]
pub enum JournalMode { Wal, Truncate, Memory }

impl JournalMode {
    pub fn for_db_path(_path: &Path) -> Self {
        // Always use WAL for local paths (network mount detection removed).
        JournalMode::Wal
    }
    pub fn effective_db_path(&self, path: &Path) -> PathBuf {
        path.to_path_buf()
    }
    pub fn as_str(&self) -> &'static str {
        match self { JournalMode::Wal => "wal", JournalMode::Truncate => "truncate", JournalMode::Memory => "memory" }
    }
    pub fn apply(&self, conn: &Connection) -> rusqlite::Result<()> {
        match self {
            JournalMode::Wal => conn.execute_batch("PRAGMA journal_mode=WAL;"),
            JournalMode::Truncate => conn.execute_batch("PRAGMA journal_mode=TRUNCATE;"),
            JournalMode::Memory => conn.execute_batch("PRAGMA journal_mode=MEMORY;"),
        }
    }
}
