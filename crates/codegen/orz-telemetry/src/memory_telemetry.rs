use crate::events::TelemetryEvent;

pub struct MemorySearch {
    pub session_id: String,
    pub query_length: usize,
    pub keyword_count: usize,
    pub result_count: usize,
    pub top_score: f64,
    pub min_score_threshold: f64,
    pub search_mode: String,
    pub duration_ms: u64,
    pub vec_available: bool,
    pub source: String,
}

pub struct MemorySearchEmpty {
    pub session_id: String,
    pub query_length: usize,
    pub keyword_count: usize,
    pub min_score_threshold: f64,
    pub search_mode: String,
    pub duration_ms: u64,
    pub vec_available: bool,
    pub source: String,
}

pub struct MemoryWatcherSync {
    pub session_id: String,
    pub dirty_file_count: usize,
    pub claimed: bool,
    pub reindexed_count: usize,
    pub embedded_count: usize,
    pub duration_ms: u64,
}

impl TelemetryEvent for MemorySearch {
    fn event_name(&self) -> &'static str { "memory_search" }
}
impl TelemetryEvent for MemorySearchEmpty {
    fn event_name(&self) -> &'static str { "memory_search_empty" }
}
impl TelemetryEvent for MemoryWatcherSync {
    fn event_name(&self) -> &'static str { "memory_watcher_sync" }
}
