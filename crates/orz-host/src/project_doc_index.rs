//! Project-doc index (GAP-RETRIEVAL-TOOLS 2026-08-10 / GAP-PROJECT-DOC-INDEX-CACHE
//! 2026-08-11) — the internal retrieval lane's real discovery/query tool
//! (ADR-0010 §3.7.4/§3.7.5).
//!
//! Host-owned synchronous tool (run_tests precedent): the model never
//! supplies a path walk — discovery is workspace-wide under a FIXED
//! exclusion set (.git/.gsa/target/node_modules/.venv/archive dirs), and
//! the query matches keywords against relative path / title / headings.
//! `include_content=true` returns (capped) full text — the mechanical
//! evidence collector maps that to `full_text_observed`; the metadata
//! default maps to `metadata_only`.
//!
//! Cache/incremental scan (GAP-PROJECT-DOC-INDEX-CACHE): every query still
//! runs a full-tree metadata walk (stat only — zero content reads), diffed
//! against a snapshot keyed by `size + mtime(secs, nanos)`. Unchanged
//! files are never re-read; changed files re-extract title/headings;
//! deleted files drop out. The snapshot persists best-effort at
//! `{cwd}/.gsa/project-doc-index/cache.json` (cwd-level, not per-session —
//! the index is a property of the workspace) and is loaded lazily on the
//! first query. Correctness first: the stat walk guarantees structural
//! add/remove is always captured; any cache failure degrades to a full
//! scan — query results never change, only speed.
//!
//! Registered boundaries (GAP-PROJECT-DOC-INDEX-CACHE 2026-08-11):
//! - A content edit that keeps both size and mtime is invisible to the
//!   diff (known limitation) — escape hatch `ORZ_PROJECT_DOC_INDEX_FORCE_RESCAN=1`
//!   (or `set_force_rescan`) forces a full re-extract.
//! - Cache write is non-atomic (sidecar precedent); a torn write heals
//!   through the corrupt→rebuild path.
//! - No cross-process lock: concurrent sessions in one cwd use
//!   last-writer-wins + self-healing diff.
//! - `metadata.modified()` failures record (0,0) → the file re-extracts on
//!   every query (conservative).
//! - Windows path-case changes trigger one redundant rebuild (harmless).
//! - Cache entries' `path` is never trusted from the on-disk cache — it is
//!   rebuilt from the walk on every reuse, so a tampered cache can only
//!   skew title/headings of files that already exist (review D1-1).
//! - A file whose `metadata()` fails mid-walk (vanished / permission) is
//!   simply not indexed this round; the old discover would index it with
//!   size=0 (review C3-4).
//! - Dangling symlinks with doc extensions are skipped (the old walk would
//!   index them with size=0) — a deliberate improvement (review P3-3).

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use orz_loop::host::{ToolError, ToolResult};
use serde::{Deserialize, Serialize};

/// Cache envelope schema version (sidecar convention).
const CACHE_SCHEMA_VERSION: &str = "0.1.0-draft";

/// Document/source extensions the index recognizes.
const DOC_EXTENSIONS: &[&str] = &[
    "md", "txt", "rst", "adoc", "ron", "toml", "yaml", "yml", "json", "rs", "py", "ts", "js", "c",
    "h", "cpp", "hpp", "go", "java", "rb", "sh", "ps1", "html", "css",
];

/// Directory names never traversed.
const EXCLUDED_DIRS: &[&str] = &[
    ".git",
    ".gsa",
    "target",
    "node_modules",
    ".venv",
    "venv",
    "存档",
    "archive",
    "dist",
    "build",
    ".hidden",
];

/// One indexed document.
#[derive(Debug, Clone, Serialize)]
pub struct DocEntry {
    pub path: String,
    pub relative_path: String,
    pub title: String,
    pub headings: Vec<String>,
    pub size: u64,
}

/// Query output shape.
#[derive(Debug, Serialize)]
pub struct QueryResult {
    pub results: Vec<QueryHit>,
    pub total: usize,
    pub truncated: bool,
}

/// One matching document with optional content.
#[derive(Debug, Serialize)]
pub struct QueryHit {
    pub path: String,
    pub relative_path: String,
    pub title: String,
    pub matched: Vec<String>,
    pub size: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_sha256: Option<String>,
}

/// Cache snapshot entry = indexed doc + the diff key. mtime is stored as
/// (secs, nanos) explicitly — serde SystemTime serialization shape is
/// unstable, and nanos preserve NTFS 100ns granularity within a second.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CachedEntry {
    pub path: String,
    pub relative_path: String,
    pub title: String,
    pub headings: Vec<String>,
    pub size: u64,
    pub mtime_secs: u64,
    pub mtime_nanos: u32,
}

/// On-disk cache envelope (`{cwd}/.gsa/project-doc-index/cache.json`).
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CachedIndex {
    schema_version: String,
    cwd_fingerprint: String,
    built_at_secs: u64,
    entries: Vec<CachedEntry>,
}

/// In-memory snapshot, Arc-shared between queries (entries always sorted by
/// relative_path — keeps query result order deterministic).
#[derive(Debug, Default)]
struct CachedSnapshot {
    entries: Vec<CachedEntry>,
}

/// One metadata-only walk result (zero content reads).
struct FileStat {
    relative_path: String,
    path: PathBuf,
    size: u64,
    mtime_secs: u64,
    mtime_nanos: u32,
}

/// The workspace document index — snapshot + mtime-diffed incremental scan.
/// Every query walks the tree for metadata only; unchanged files are never
/// re-read. The snapshot persists across runs under `.gsa/project-doc-index/`.
#[derive(Debug)]
pub struct ProjectDocIndex {
    cwd: PathBuf,
    /// cwd identity for cross-run cache reuse (dunce::canonicalize strips
    /// `\\?\` verbatim prefixes; computed once at construction — the only
    /// constructor IO; zero per-query IO).
    fingerprint: String,
    /// Current snapshot; None = not yet built (first query loads the
    /// on-disk cache and diff-walks the tree). Arc so the read side shares
    /// an immutable snapshot outside the lock. `Option` distinguishes "not
    /// built yet" from a genuinely empty workspace — an empty snapshot must
    /// not re-load/re-persist on every query (review D2-3).
    state: Mutex<Option<Arc<CachedSnapshot>>>,
    /// Escape-hatch seam: OR of this flag and the env var (tests set this
    /// instead of env — parallel tests would clobber each other's env).
    force_rescan: AtomicBool,
}

impl ProjectDocIndex {
    pub fn new(cwd: PathBuf) -> Self {
        Self {
            fingerprint: fingerprint_of(&cwd),
            cwd,
            state: Mutex::new(None),
            force_rescan: AtomicBool::new(false),
        }
    }

    /// The tool's declared contract (mirrors the ToolDef the registry
    /// advertises — kept in sync with `query`'s argument parsing).
    pub fn tool_def() -> orz_loop::host::ToolDef {
        orz_loop::host::ToolDef {
            name: "project_doc_index".to_string(),
            description: "Index and search the workspace's project documents \
                 (markdown/docs + source files, excluding .git/.gsa/target/\
                 node_modules/.venv/archives). `query` matches keywords \
                 against paths/titles/headings (empty = list all); \
                 `include_content` (default false) returns capped full \
                 content for the hits; `max_results` (default 10) and \
                 `max_content_bytes` (default 16384) bound the output."
                .to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Keyword(s) to match" },
                    "include_content": { "type": "boolean", "description": "Return capped full content" },
                    "max_results": { "type": "integer", "description": "Result cap (default 10)" },
                    "max_content_bytes": { "type": "integer", "description": "Per-hit content cap (default 16384)" },
                },
            }),
        }
    }

    /// Scan the workspace once and return every recognized document —
    /// always a forced full re-extract (never served from the diff
    /// snapshot), and the result is persisted as the new baseline.
    pub fn discover(&self) -> Vec<DocEntry> {
        let snap = self.refresh_forced();
        snap.entries
            .iter()
            .map(|e| DocEntry {
                path: e.path.clone(),
                relative_path: e.relative_path.clone(),
                title: e.title.clone(),
                headings: e.headings.clone(),
                size: e.size,
            })
            .collect()
    }

    /// Run one query against the diff-refreshed snapshot. `args` is the
    /// tool-call argument object (parsed strictly — unknown shapes are
    /// explicit errors, never a silent empty result). Matching, content
    /// reads and serialization all happen outside the snapshot lock — the
    /// metadata walk + diff + changed-file re-extract + best-effort
    /// persist hold it (no awaits anywhere).
    pub fn query(&self, args: &serde_json::Value) -> Result<ToolResult, ToolError> {
        let query = args
            .get("query")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_lowercase();
        let include_content = args
            .get("include_content")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let max_results = args
            .get("max_results")
            .and_then(|v| v.as_u64())
            .unwrap_or(10)
            .min(100) as usize;
        let max_content_bytes = args
            .get("max_content_bytes")
            .and_then(|v| v.as_u64())
            .unwrap_or(16_384)
            .max(1024) as usize;

        let snap = self.refresh();
        let mut hits: Vec<QueryHit> = Vec::new();
        let mut truncated = false;
        for entry in &snap.entries {
            // Keyword match against path/title/headings (case-insensitive).
            let haystack = format!(
                "{} {} {}",
                entry.relative_path.to_lowercase(),
                entry.title.to_lowercase(),
                entry.headings.join(" ").to_lowercase(),
            );
            let matched = if query.is_empty() {
                true
            } else {
                query.split_whitespace().all(|kw| haystack.contains(kw))
            };
            if !matched {
                continue;
            }
            if hits.len() >= max_results {
                truncated = true;
                break;
            }
            let path = PathBuf::from(&entry.path);
            let (content, content_sha256) = if include_content {
                match std::fs::read(&path) {
                    Ok(bytes) => {
                        let digest =
                            orz_assurance::sha256_hex(&bytes[..bytes.len().min(max_content_bytes)]);
                        let text =
                            String::from_utf8_lossy(&bytes[..bytes.len().min(max_content_bytes)])
                                .to_string();
                        let content = if bytes.len() > max_content_bytes {
                            format!("{text}\n[truncated — {} bytes total]", bytes.len())
                        } else {
                            text
                        };
                        (Some(content), Some(digest))
                    }
                    Err(e) => {
                        tracing::warn!("project_doc_index read failed ({}): {e}", path.display());
                        (None, None)
                    }
                }
            } else {
                (None, None)
            };
            hits.push(QueryHit {
                path: entry.path.clone(),
                relative_path: entry.relative_path.clone(),
                title: entry.title.clone(),
                matched: entry.headings.clone(),
                size: entry.size,
                content,
                content_sha256,
            });
        }

        let result = QueryResult {
            results: hits,
            total: 0, // filled below
            truncated,
        };
        let mut result = result;
        result.total = result.results.len();
        let json = serde_json::to_string(&result)
            .map_err(|e| ToolError::ExecutionFailed(format!("project_doc_index serialize: {e}")))?;
        Ok(ToolResult {
            output: json,
            exit_code: Some(0),
            output_encoding: None,
            structured: None,
        })
    }

    /// Diff-refresh the snapshot in one locked critical section (no awaits —
    /// this module is fully synchronous, so a std Mutex never spans an
    /// await point): lazy cache load on first build, metadata-only tree
    /// walk, diff against the old snapshot, re-extract only changed/new
    /// files, drop removed ones, re-sort, persist when dirty, Arc-swap.
    /// Concurrent callers merge into one build: the second lane reuses the
    /// freshly swapped snapshot instead of re-walking.
    fn refresh(&self) -> Arc<CachedSnapshot> {
        let mut guard = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let force = self.force_rescan_effective();
        // First build: adopt the on-disk cache as the diff baseline
        // (skipped under the force hatch — a forced rescan must not trust
        // the cache). load_cache warns and returns None on any failure.
        let was_none = guard.is_none();
        let mut loaded = false;
        if was_none
            && !force
            && let Some(cached) = self.load_cache()
        {
            *guard = Some(Arc::new(CachedSnapshot {
                entries: cached.entries,
            }));
            loaded = true;
        }
        // Metadata-only tree walk — runs on EVERY query by design (D-2).
        // A concurrent second caller reuses the snapshot below (skipping
        // re-extract and persist) but still pays this stat walk.
        let (stats, root_ok) = stat_pass(&self.cwd);
        if !root_ok {
            // Transient walk failure (unreadable root): never delete or
            // persist — keep the baseline snapshot as-is (review P2-2).
            return guard.as_ref().cloned().unwrap_or_default();
        }
        let old: HashMap<&str, &CachedEntry> = guard
            .as_ref()
            .map(|s| {
                s.entries
                    .iter()
                    .map(|e| (e.relative_path.as_str(), e))
                    .collect()
            })
            .unwrap_or_default();
        let mut new_entries: Vec<CachedEntry> = Vec::with_capacity(stats.len());
        let mut changed = 0usize;
        let mut reused = 0usize;
        for st in &stats {
            // Unchanged (size + mtime equal) → reuse title/headings only.
            // `path` is ALWAYS rebuilt from the walk (`st.path`) — the
            // on-disk cache is never trusted with paths (review D1-1).
            // A (0,0) mtime (platform reports none) never reuses, so such
            // files conservatively re-extract on every query (review P2-1).
            let cached = old.get(st.relative_path.as_str()).copied();
            let unchanged = !force
                && (st.mtime_secs != 0 || st.mtime_nanos != 0)
                && cached.is_some_and(|e| {
                    e.size == st.size
                        && e.mtime_secs == st.mtime_secs
                        && e.mtime_nanos == st.mtime_nanos
                });
            if unchanged {
                let e = cached.expect("checked above");
                reused += 1;
                new_entries.push(CachedEntry {
                    path: st.path.to_string_lossy().to_string(),
                    relative_path: st.relative_path.clone(),
                    title: e.title.clone(),
                    headings: e.headings.clone(),
                    size: st.size,
                    mtime_secs: st.mtime_secs,
                    mtime_nanos: st.mtime_nanos,
                });
            } else {
                changed += 1;
                new_entries.push(entry_from_stat(&self.cwd, st));
            }
        }
        // Not-reused = re-extracted + genuinely removed; only drives the
        // persist-dirty decision (removed files never enter the new
        // snapshot either way).
        let not_reused = old.len() - reused;
        new_entries.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
        let snap = Arc::new(CachedSnapshot {
            entries: new_entries,
        });
        if force || changed > 0 || not_reused > 0 || (was_none && !loaded) {
            self.persist_cache(&snap);
        }
        *guard = Some(snap.clone());
        snap
    }

    /// discover()'s path: force a full re-extract (skip cache load and the
    /// diff reuse), persist the result as the new baseline.
    fn refresh_forced(&self) -> Arc<CachedSnapshot> {
        let mut guard = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let (stats, root_ok) = stat_pass(&self.cwd);
        if !root_ok {
            return guard.as_ref().cloned().unwrap_or_default();
        }
        let mut entries: Vec<CachedEntry> = stats
            .iter()
            .map(|st| entry_from_stat(&self.cwd, st))
            .collect();
        entries.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
        let snap = Arc::new(CachedSnapshot { entries });
        self.persist_cache(&snap);
        *guard = Some(snap.clone());
        snap
    }

    /// `{cwd}/.gsa/project-doc-index/cache.json` — cwd-level (the index is
    /// a property of the workspace, not of a session). `.gsa` is in
    /// EXCLUDED_DIRS, so the cache never indexes itself.
    fn cache_path(&self) -> PathBuf {
        self.cwd
            .join(".gsa")
            .join("project-doc-index")
            .join("cache.json")
    }

    /// Sidecar load discipline: NotFound or read failure → None silently
    /// (normal first run / transient unreadability); parse failure, schema
    /// mismatch or cwd mismatch → warn + None (full rebuild). Never
    /// panics, never accepts a stale-format cache.
    fn load_cache(&self) -> Option<CachedIndex> {
        let path = self.cache_path();
        let Ok(text) = std::fs::read_to_string(&path) else {
            return None;
        };
        let cached: CachedIndex = match serde_json::from_str(&text) {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!(
                    "project-doc-index cache corrupt ({}): {e} — rebuilding",
                    path.display()
                );
                return None;
            }
        };
        if cached.schema_version != CACHE_SCHEMA_VERSION {
            tracing::warn!(
                "project-doc-index cache schema mismatch ({} != {}): {} — rebuilding",
                cached.schema_version,
                CACHE_SCHEMA_VERSION,
                path.display()
            );
            return None;
        }
        if cached.cwd_fingerprint != self.fingerprint {
            tracing::warn!(
                "project-doc-index cache cwd mismatch (cached {:?}, now {:?}): {} — rebuilding",
                cached.cwd_fingerprint,
                self.fingerprint,
                path.display()
            );
            return None;
        }
        Some(cached)
    }

    /// Best-effort write — never fails the run: serialize/build-dir/write
    /// failures are all warn+return. Non-atomic overwrite (sidecar
    /// precedent); a torn write heals through load_cache's corrupt path.
    fn persist_cache(&self, snap: &CachedSnapshot) {
        let built_at_secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let cached = CachedIndex {
            schema_version: CACHE_SCHEMA_VERSION.to_string(),
            cwd_fingerprint: self.fingerprint.clone(),
            built_at_secs,
            entries: snap.entries.clone(),
        };
        let json = match serde_json::to_string_pretty(&cached) {
            Ok(j) => j,
            Err(e) => {
                tracing::warn!("project-doc-index cache serialize failed: {e}");
                return;
            }
        };
        let path = self.cache_path();
        if let Some(parent) = path.parent()
            && let Err(e) = std::fs::create_dir_all(parent)
        {
            tracing::warn!(
                "project-doc-index cache mkdir failed ({}): {e}",
                parent.display()
            );
            return;
        }
        if let Err(e) = std::fs::write(&path, json) {
            tracing::warn!(
                "project-doc-index cache write failed ({}): {e}",
                path.display()
            );
        }
    }

    /// flag || env `ORZ_PROJECT_DOC_INDEX_FORCE_RESCAN` == "1".
    fn force_rescan_effective(&self) -> bool {
        self.force_rescan.load(Ordering::Relaxed)
            || std::env::var("ORZ_PROJECT_DOC_INDEX_FORCE_RESCAN").as_deref() == Ok("1")
    }

    /// Test/ops seam — set instead of env so parallel tests never clobber
    /// each other's process env.
    #[doc(hidden)]
    pub fn set_force_rescan(&self, v: bool) {
        self.force_rescan.store(v, Ordering::Relaxed);
    }
}

/// cwd identity for the cross-run cache: canonical form (dunce strips
/// `\\?\` verbatim prefixes on Windows), falling back to the raw path.
/// Case-form changes (D:\CLI vs d:\cli) cause one redundant rebuild — harmless.
fn fingerprint_of(cwd: &Path) -> String {
    dunce::canonicalize(cwd)
        .unwrap_or_else(|_| cwd.to_path_buf())
        .to_string_lossy()
        .to_string()
}

/// Full-tree metadata walk (zero content reads). Exclusion/extension/
/// symlink semantics mirror `discover`'s walk: read_dir failures skip the
/// directory; excluded dirs by name; doc files by extension. Returns the
/// walk results plus whether the ROOT directory itself was readable — an
/// unreadable root makes the empty result NON-authoritative: callers must
/// not delete snapshot entries or persist from it (review P2-2).
fn stat_pass(cwd: &Path) -> (Vec<FileStat>, bool) {
    let mut stats = Vec::new();
    let root_ok = std::fs::read_dir(cwd).is_ok();
    let mut stack: Vec<PathBuf> = vec![cwd.to_path_buf()];
    let excluded: HashSet<&str> = EXCLUDED_DIRS.iter().copied().collect();
    while let Some(dir) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in rd.flatten() {
            let path = entry.path();
            let name = entry.file_name();
            let name_str = name.to_string_lossy().to_string();
            if path.is_dir() {
                if !excluded.contains(name_str.as_str()) {
                    stack.push(path);
                }
            } else if is_doc_file(&path)
                && let Some(st) = file_stat(cwd, &path)
            {
                stats.push(st);
            }
        }
    }
    (stats, root_ok)
}

/// Metadata for one doc file; None when the file vanished mid-walk (rare
/// TOCTOU — it is simply not indexed this round).
fn file_stat(cwd: &Path, path: &Path) -> Option<FileStat> {
    let md = std::fs::metadata(path).ok()?;
    let size = md.len();
    let (mtime_secs, mtime_nanos) = match md.modified() {
        Ok(t) => {
            let dur = t.duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
            (dur.as_secs(), dur.subsec_nanos())
        }
        // Platform reports no mtime: (0,0) — every diff sees "changed" →
        // conservative re-extract each query (registered trade-off).
        Err(_) => (0, 0),
    };
    let relative_path = path
        .strip_prefix(cwd)
        .unwrap_or(path)
        .to_string_lossy()
        .to_string();
    Some(FileStat {
        relative_path,
        path: path.to_path_buf(),
        size,
        mtime_secs,
        mtime_nanos,
    })
}

/// Build a cache entry from a metadata walk result (re-extracts title/
/// headings — the only content read in the whole refresh path).
fn entry_from_stat(cwd: &Path, st: &FileStat) -> CachedEntry {
    let entry = entry_from_path(cwd, &st.path);
    CachedEntry {
        path: entry.path,
        relative_path: entry.relative_path,
        title: entry.title,
        headings: entry.headings,
        // size comes from the walk's stat — single source so the diff key
        // and the entry never disagree across the stat/extract TOCTOU
        // (review P3-1).
        size: st.size,
        mtime_secs: st.mtime_secs,
        mtime_nanos: st.mtime_nanos,
    }
}

/// Recognized document/source extensions (case-insensitive).
fn is_doc_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| DOC_EXTENSIONS.contains(&e.to_lowercase().as_str()))
        .unwrap_or(false)
}

/// Build one entry from a path (title = file stem; headings from markdown
/// `#` lines, capped for memory safety).
fn entry_from_path(cwd: &Path, path: &Path) -> DocEntry {
    let relative_path = path
        .strip_prefix(cwd)
        .unwrap_or(path)
        .to_string_lossy()
        .to_string();
    let title = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| relative_path.clone());
    let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    let headings = extract_headings(path);
    DocEntry {
        path: path.to_string_lossy().to_string(),
        relative_path,
        title,
        headings,
        size,
    }
}

/// Markdown `#`-heading extraction (capped reads — a huge doc's headings
/// still come from its first 256KB).
fn extract_headings(path: &Path) -> Vec<String> {
    let Ok(bytes) = std::fs::read(path) else {
        return Vec::new();
    };
    let head = &bytes[..bytes.len().min(256 * 1024)];
    let text = String::from_utf8_lossy(head);
    text.lines()
        .filter_map(|l| {
            let t = l.trim_start();
            if t.starts_with("# ") {
                Some(t.trim_start_matches('#').trim().to_string())
            } else {
                None
            }
        })
        .take(64)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_dir() -> PathBuf {
        let n = std::process::id();
        let dir = std::env::temp_dir().join(format!(
            "orz-doc-index-test-{n}-{:?}",
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("docs")).unwrap();
        dir
    }

    #[test]
    fn discovery_excludes_system_dirs_and_indexes_docs() {
        let dir = test_dir();
        std::fs::write(dir.join("README.md"), "# Readme\nhello world").unwrap();
        std::fs::write(dir.join("docs/api.md"), "# API\nfn main").unwrap();
        std::fs::write(dir.join("main.rs"), "fn main() {}").unwrap();
        std::fs::create_dir_all(dir.join(".git")).unwrap();
        std::fs::write(dir.join(".git/config"), "x").unwrap();
        std::fs::create_dir_all(dir.join("target")).unwrap();
        std::fs::write(dir.join("target/big.md"), "x").unwrap();
        std::fs::create_dir_all(dir.join("node_modules")).unwrap();
        std::fs::write(dir.join("node_modules/lib.md"), "x").unwrap();

        let index = ProjectDocIndex::new(dir.clone());
        let entries = index.discover();
        let rels: Vec<String> = entries
            .iter()
            .map(|e| e.relative_path.replace('\\', "/"))
            .collect();
        assert_eq!(
            rels,
            vec!["README.md", "docs/api.md", "main.rs"],
            "{rels:?}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn query_matches_keywords_and_returns_content_capped() {
        let dir = test_dir();
        std::fs::write(dir.join("README.md"), "# Readme\nhello world").unwrap();
        std::fs::write(dir.join("docs/api.md"), "# API\nthe api docs").unwrap();
        let index = ProjectDocIndex::new(dir.clone());

        // Keyword hit on headings.
        let out = index.query(&serde_json::json!({"query": "api"})).unwrap();
        assert!(out.output.contains("api.md"), "{}", out.output);
        assert!(!out.output.contains("README.md"), "{}", out.output);
        // include_content returns capped full text with digest.
        let out = index
            .query(&serde_json::json!({"query": "readme", "include_content": true}))
            .unwrap();
        assert!(out.output.contains("# Readme"), "{}", out.output);
        assert!(out.output.contains("content_sha256"), "{}", out.output);
        // Empty query lists everything.
        let out = index.query(&serde_json::json!({})).unwrap();
        assert!(out.output.contains("README.md"), "{}", out.output);
        assert!(out.output.contains("api.md"), "{}", out.output);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn max_results_bounds_and_truncates() {
        let dir = test_dir();
        for i in 0..5 {
            std::fs::write(dir.join(format!("doc{i}.md")), format!("# D{i}")).unwrap();
        }
        let index = ProjectDocIndex::new(dir.clone());
        let out = index
            .query(&serde_json::json!({"query": "", "max_results": 2}))
            .unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&out.output).unwrap();
        assert_eq!(parsed["results"].as_array().unwrap().len(), 2);
        assert_eq!(parsed["truncated"], true);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Behavioral "no re-read" proof: content replaced with the same size
    /// and a restored mtime must be invisible — headings come from content,
    /// so the old heading staying visible proves the file was never
    /// re-extracted.
    #[test]
    fn incremental_unchanged_files_are_not_reextracted() {
        let dir = test_dir();
        let file = dir.join("README.md");
        std::fs::write(&file, "# One\n").unwrap();
        let index = ProjectDocIndex::new(dir.clone());
        let out = index.query(&serde_json::json!({"query": "one"})).unwrap();
        assert!(out.output.contains("README.md"), "{}", out.output);
        let mtime = std::fs::metadata(&file).unwrap().modified().unwrap();
        // Same size (6 bytes), new content, mtime restored → diff sees no change.
        std::fs::write(&file, "# Two\n").unwrap();
        filetime::set_file_mtime(&file, filetime::FileTime::from_system_time(mtime)).unwrap();
        let out = index.query(&serde_json::json!({"query": "two"})).unwrap();
        assert!(!out.output.contains("README.md"), "{}", out.output);
        let out = index.query(&serde_json::json!({"query": "one"})).unwrap();
        assert!(out.output.contains("README.md"), "{}", out.output);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Both diff triggers re-extract: a size change, and an mtime change
    /// with identical size.
    #[test]
    fn changed_mtime_or_size_triggers_reextract() {
        let dir = test_dir();
        let file = dir.join("README.md");
        std::fs::write(&file, "# One\n").unwrap();
        let index = ProjectDocIndex::new(dir.clone());
        let _ = index.query(&serde_json::json!({})).unwrap();
        // Size change → re-extract.
        std::fs::write(&file, "# Two\nnew\n").unwrap();
        let out = index.query(&serde_json::json!({"query": "two"})).unwrap();
        assert!(out.output.contains("README.md"), "{}", out.output);
        // Same size (10 bytes) + mtime bumped → re-extract.
        std::fs::write(&file, "# Thir\nne\n").unwrap();
        let old = std::fs::metadata(&file).unwrap().modified().unwrap();
        filetime::set_file_mtime(
            &file,
            filetime::FileTime::from_system_time(old + std::time::Duration::from_secs(10)),
        )
        .unwrap();
        let out = index.query(&serde_json::json!({"query": "thir"})).unwrap();
        assert!(out.output.contains("README.md"), "{}", out.output);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn added_modified_deleted_files_update_snapshot() {
        let dir = test_dir();
        std::fs::write(dir.join("a.md"), "# A\n").unwrap();
        std::fs::write(dir.join("c.md"), "# C\n").unwrap();
        let index = ProjectDocIndex::new(dir.clone());
        let out = index.query(&serde_json::json!({})).unwrap();
        assert!(out.output.contains("a.md"), "{}", out.output);
        assert!(out.output.contains("c.md"), "{}", out.output);
        // Add b, modify c, delete a.
        std::fs::write(dir.join("b.md"), "# B\n").unwrap();
        std::fs::write(dir.join("c.md"), "# C2\n").unwrap();
        std::fs::remove_file(dir.join("a.md")).unwrap();
        let out = index.query(&serde_json::json!({})).unwrap();
        assert!(out.output.contains("b.md"), "{}", out.output);
        assert!(out.output.contains("c.md"), "{}", out.output);
        assert!(!out.output.contains("a.md"), "{}", out.output);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Persistence round-trip: first query writes a valid cache; a steady-
    /// state query does not rewrite it (zero write amplification); a fresh
    /// index adopts the on-disk cache (a same-size + restored-mtime edit
    /// stays invisible to the new instance).
    #[test]
    fn persistence_round_trip_and_steady_state_no_write() {
        let dir = test_dir();
        let file = dir.join("README.md");
        std::fs::write(&file, "# Readme\n").unwrap();
        let index = ProjectDocIndex::new(dir.clone());
        let _ = index
            .query(&serde_json::json!({"query": "readme"}))
            .unwrap();
        let cache = dir.join(".gsa/project-doc-index/cache.json");
        assert!(cache.exists(), "cache file should be written");
        let parsed: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&cache).unwrap()).unwrap();
        assert_eq!(parsed["schema_version"], "0.1.0-draft");
        assert_eq!(parsed["entries"].as_array().unwrap().len(), 1);
        let mtime = std::fs::metadata(&cache).unwrap().modified().unwrap();
        // Steady state: unchanged tree → no re-persist.
        let _ = index
            .query(&serde_json::json!({"query": "readme"}))
            .unwrap();
        let mtime2 = std::fs::metadata(&cache).unwrap().modified().unwrap();
        assert_eq!(
            mtime, mtime2,
            "steady-state query must not rewrite the cache"
        );
        // Cross-run load: "# Readme\n" (8 bytes) → "# Twoooo\n" (8 bytes)
        // with the mtime restored — a fresh index loads the old cache and
        // keeps the old heading.
        let t0 = std::fs::metadata(&file).unwrap().modified().unwrap();
        std::fs::write(&file, "# Twoooo\n").unwrap();
        filetime::set_file_mtime(&file, filetime::FileTime::from_system_time(t0)).unwrap();
        let index2 = ProjectDocIndex::new(dir.clone());
        let out = index2
            .query(&serde_json::json!({"query": "twoooo"}))
            .unwrap();
        assert!(!out.output.contains("README.md"), "{}", out.output);
        let out = index2
            .query(&serde_json::json!({"query": "readme"}))
            .unwrap();
        assert!(out.output.contains("README.md"), "{}", out.output);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn corrupt_cache_warns_and_rebuilds() {
        let dir = test_dir();
        std::fs::write(dir.join("README.md"), "# Readme\n").unwrap();
        let cache = dir.join(".gsa/project-doc-index/cache.json");
        std::fs::create_dir_all(cache.parent().unwrap()).unwrap();
        std::fs::write(&cache, "{ not json !!!").unwrap();
        let index = ProjectDocIndex::new(dir.clone());
        let out = index
            .query(&serde_json::json!({"query": "readme"}))
            .unwrap();
        assert!(out.output.contains("README.md"), "{}", out.output);
        // The corrupt file was rebuilt into valid JSON.
        let parsed: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&cache).unwrap()).unwrap();
        assert_eq!(parsed["schema_version"], "0.1.0-draft");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn schema_version_mismatch_rebuilds() {
        let dir = test_dir();
        std::fs::write(dir.join("README.md"), "# Readme\n").unwrap();
        let cache = dir.join(".gsa/project-doc-index/cache.json");
        std::fs::create_dir_all(cache.parent().unwrap()).unwrap();
        std::fs::write(
            &cache,
            r#"{"schema_version":"9.9.9","cwd_fingerprint":"x","built_at_secs":0,"entries":[]}"#,
        )
        .unwrap();
        let index = ProjectDocIndex::new(dir.clone());
        let out = index
            .query(&serde_json::json!({"query": "readme"}))
            .unwrap();
        assert!(out.output.contains("README.md"), "{}", out.output);
        let parsed: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&cache).unwrap()).unwrap();
        assert_eq!(parsed["schema_version"], "0.1.0-draft");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn cwd_fingerprint_mismatch_rebuilds() {
        let dir = test_dir();
        std::fs::write(dir.join("README.md"), "# Readme\n").unwrap();
        let cache = dir.join(".gsa/project-doc-index/cache.json");
        std::fs::create_dir_all(cache.parent().unwrap()).unwrap();
        std::fs::write(
            &cache,
            r#"{"schema_version":"0.1.0-draft","cwd_fingerprint":"some-other-dir","built_at_secs":0,"entries":[]}"#,
        )
        .unwrap();
        let index = ProjectDocIndex::new(dir.clone());
        let out = index
            .query(&serde_json::json!({"query": "readme"}))
            .unwrap();
        assert!(out.output.contains("README.md"), "{}", out.output);
        let parsed: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&cache).unwrap()).unwrap();
        assert_eq!(parsed["cwd_fingerprint"], index.fingerprint);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Escape hatch: the force flag re-extracts even a same-size +
    /// restored-mtime edit, then increments resume from the new baseline.
    #[test]
    fn force_rescan_ignores_cache() {
        let dir = test_dir();
        let file = dir.join("README.md");
        std::fs::write(&file, "# One\n").unwrap();
        let index = ProjectDocIndex::new(dir.clone());
        let _ = index.query(&serde_json::json!({"query": "one"})).unwrap();
        let mtime = std::fs::metadata(&file).unwrap().modified().unwrap();
        std::fs::write(&file, "# Two\n").unwrap();
        filetime::set_file_mtime(&file, filetime::FileTime::from_system_time(mtime)).unwrap();
        index.set_force_rescan(true);
        let out = index.query(&serde_json::json!({"query": "two"})).unwrap();
        assert!(out.output.contains("README.md"), "{}", out.output);
        index.set_force_rescan(false);
        // Incremental resume: the force snapshot is now the baseline.
        let out = index.query(&serde_json::json!({"query": "two"})).unwrap();
        assert!(out.output.contains("README.md"), "{}", out.output);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Concurrent queries across both lanes (dual-concurrency shape): all
    /// succeed, agree on hits, and leave a valid cache behind.
    #[test]
    fn concurrent_queries_are_consistent() {
        let dir = test_dir();
        for i in 0..20 {
            std::fs::write(dir.join(format!("doc{i:02}.md")), format!("# D{i}\n")).unwrap();
        }
        let index = Arc::new(ProjectDocIndex::new(dir.clone()));
        std::thread::scope(|s| {
            for t in 0..8 {
                let index = index.clone();
                s.spawn(move || {
                    for q in 0..10 {
                        let args = serde_json::json!({
                            "query": "d",
                            "max_results": 100,
                            "include_content": q % 2 == 0,
                        });
                        let out = index.query(&args).unwrap();
                        assert_eq!(out.exit_code, Some(0));
                        let parsed: serde_json::Value = serde_json::from_str(&out.output).unwrap();
                        assert_eq!(parsed["total"], 20, "thread {t} query {q}");
                        assert_eq!(parsed["results"].as_array().unwrap().len(), 20);
                    }
                });
            }
        });
        // The cache file must be valid JSON after the concurrent storm.
        let parsed: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(dir.join(".gsa/project-doc-index/cache.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(parsed["entries"].as_array().unwrap().len(), 20);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// discover() stays a forced full scan: a same-size + restored-mtime
    /// edit is visible there even though the diff snapshot would miss it.
    #[test]
    fn discover_after_refresh_returns_full_scan() {
        let dir = test_dir();
        let file = dir.join("a.md");
        std::fs::write(&file, "# A\n").unwrap();
        let index = ProjectDocIndex::new(dir.clone());
        let _ = index.query(&serde_json::json!({})).unwrap();
        let mtime = std::fs::metadata(&file).unwrap().modified().unwrap();
        std::fs::write(&file, "# B\n").unwrap();
        filetime::set_file_mtime(&file, filetime::FileTime::from_system_time(mtime)).unwrap();
        let entries = index.discover();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].relative_path, "a.md");
        assert_eq!(entries[0].headings, vec!["B".to_string()]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A transiently unreadable root must neither wipe the snapshot nor
    /// re-persist the cache (review P2-2): the baseline is kept as-is.
    #[test]
    fn unreadable_root_keeps_snapshot_and_cache() {
        let dir = test_dir();
        std::fs::write(dir.join("a.md"), "# A\n").unwrap();
        let index = ProjectDocIndex::new(dir.clone());
        let _ = index.query(&serde_json::json!({})).unwrap();
        let cache = dir.join(".gsa/project-doc-index/cache.json");
        let mtime = std::fs::metadata(&cache).unwrap().modified().unwrap();
        // Rename the workspace away: the walk's root becomes unreadable
        // while the cache file survives at the new location.
        let moved = dir.with_extension("moved");
        std::fs::rename(&dir, &moved).unwrap();
        let out = index.query(&serde_json::json!({})).unwrap();
        // The old snapshot still serves results (metadata-only).
        assert!(out.output.contains("a.md"), "{}", out.output);
        // No re-persist: the moved cache file's mtime is untouched.
        let moved_cache = moved.join(".gsa/project-doc-index/cache.json");
        assert!(moved_cache.exists());
        let mtime2 = std::fs::metadata(&moved_cache).unwrap().modified().unwrap();
        assert_eq!(mtime, mtime2, "unreadable root must not re-persist");
        let _ = std::fs::remove_dir_all(&moved);
    }

    /// The on-disk cache is never trusted with paths (review D1-1): a
    /// tampered entry whose `path` points outside the workspace must not be
    /// read — the reuse branch rebuilds `path` from the walk.
    #[test]
    fn tampered_cache_path_is_not_read() {
        let dir = test_dir();
        let file = dir.join("a.md");
        std::fs::write(&file, "# A\n").unwrap();
        let secret =
            std::env::temp_dir().join(format!("orz-doc-index-secret-{}", std::process::id()));
        std::fs::write(&secret, "TOP SECRET CONTENT").unwrap();
        let index = ProjectDocIndex::new(dir.clone());
        let _ = index.query(&serde_json::json!({})).unwrap();
        let md = std::fs::metadata(&file).unwrap();
        let dur = md
            .modified()
            .unwrap()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap();
        // Tamper: same relative_path + size + mtime (so the diff would
        // reuse), but `path` → the decoy secret file.
        let tampered = serde_json::json!({
            "schema_version": "0.1.0-draft",
            "cwd_fingerprint": index.fingerprint,
            "built_at_secs": 0,
            "entries": [{
                "path": secret.to_string_lossy(),
                "relative_path": "a.md",
                "title": "A",
                "headings": ["A"],
                "size": md.len(),
                "mtime_secs": dur.as_secs(),
                "mtime_nanos": dur.subsec_nanos(),
            }]
        });
        let cache = dir.join(".gsa/project-doc-index/cache.json");
        std::fs::create_dir_all(cache.parent().unwrap()).unwrap();
        std::fs::write(&cache, tampered.to_string()).unwrap();
        // A FRESH index adopts the tampered cache, but include_content must
        // read a.md (the walk path), never the secret.
        let index2 = ProjectDocIndex::new(dir.clone());
        let out = index2
            .query(&serde_json::json!({"query": "a", "include_content": true}))
            .unwrap();
        assert!(out.output.contains("# A"), "{}", out.output);
        assert!(!out.output.contains("TOP SECRET"), "{}", out.output);
        let _ = std::fs::remove_file(&secret);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
