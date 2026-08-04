// Stub for deleted xai-codebase-graph crate.
use std::path::PathBuf;

pub struct IndexManager;
pub struct IndexManagerHandle;
pub struct IndexManagerConfig {
    cwd: PathBuf,
    cache_path: Option<PathBuf>,
}

impl IndexManagerConfig {
    pub fn new(cwd: PathBuf) -> Self { Self { cwd, cache_path: None } }
    pub fn with_cache_path(mut self, path: PathBuf) -> Self { self.cache_path = Some(path); self }
}

impl IndexManager {
    pub fn spawn(config: IndexManagerConfig) -> std::sync::Arc<IndexManagerHandle> {
        std::sync::Arc::new(IndexManagerHandle)
    }
}

impl IndexManagerHandle {
    pub fn shutdown(&self) {}
    pub fn notify(&self, _event: FileEvent) {}
    pub fn rebuild(&self) -> Result<(), String> { Ok(()) }
    pub fn send_event(&self, _event: FileEvent) -> Result<(), String> { Ok(()) }
    pub fn send_events(&self, _events: Vec<FileEvent>) -> Result<(), String> { Ok(()) }
    pub fn get_stats(&self) -> Result<IndexStats, QueryError> { Ok(IndexStats::default()) }
    pub async fn find_references(&self, _symbol: String, _file: Option<PathBuf>) -> Result<Vec<SymbolLocation>, QueryError> { Ok(vec![]) }
    pub async fn find_definitions(&self, _symbol: String, _file: Option<PathBuf>) -> Result<Vec<SymbolLocation>, QueryError> { Ok(vec![]) }
    pub async fn goto_definition(&self, _file: PathBuf, _line: usize, _col: usize) -> Result<QueryResult, QueryError> { Ok(QueryResult { locations: vec![] }) }
    pub async fn goto_references(&self, _file: PathBuf, _line: usize, _col: usize, _include_def: bool) -> Result<QueryResult, QueryError> { Ok(QueryResult { locations: vec![] }) }
    pub fn get_file_count(&self) -> Option<usize> { Some(0) }
}

#[derive(Debug, Clone, Default)]
pub struct IndexStats {
    pub indexed_files: usize,
    pub files: usize,
    pub definitions: usize,
    pub references: usize,
}

#[derive(Debug, Clone)]
pub struct FileEvent {
    pub path: String,
    pub kind: FileEventKind,
}
impl FileEvent {
    pub fn new(paths: Vec<PathBuf>, kind: FileEventKind) -> Self {
        let path = paths.into_iter().next().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default();
        Self { path, kind }
    }
    pub fn created(path: PathBuf) -> Self { Self { path: path.to_string_lossy().into_owned(), kind: FileEventKind::Created } }
    pub fn removed(path: PathBuf) -> Self { Self { path: path.to_string_lossy().into_owned(), kind: FileEventKind::Removed } }
    pub fn modified(path: PathBuf) -> Self { Self { path: path.to_string_lossy().into_owned(), kind: FileEventKind::Modified } }
    pub fn renamed(from: PathBuf, _to: PathBuf) -> Self { Self { path: from.to_string_lossy().into_owned(), kind: FileEventKind::Renamed } }
}

#[derive(Debug, Clone, PartialEq)]
pub enum FileEventKind { Created, Removed, Modified, Renamed }

#[derive(Debug)]
pub struct QueryResult {
    pub locations: Vec<SymbolLocation>,
}
#[derive(Debug)]
pub struct SymbolLocation {
    pub path: String,
    pub line: usize,
    pub column: usize,
    pub matched_symbol: Option<String>,
}
#[derive(Debug)]
pub struct QueryError;
impl std::fmt::Display for QueryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "stub") }
}
