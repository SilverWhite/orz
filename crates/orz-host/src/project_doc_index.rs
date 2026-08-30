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
//! v2 (RETRIEVAL-ORCHESTRATION-MECHANICAL 0k 第二批, 2026-08-30 设计轮
//! 定稿 + 用户裁决采纳)：git 工作区下升级为「git HEAD 基线 + 工作树增量层
//! + Blake3 内容哈希 + 驻留索引」——
//! - 基线层 = 首次构建全量扫描一次（stat_pass，受版本控制文件经
//!   `git ls-files -z` 标记 tracked）；
//! - 增量层 = `git status --porcelain -z --no-renames --untracked-files=all`
//!   报告 modified/untracked/deleted，只重读变更文件（blake3 内容哈希 +
//!   title/headings）；未跟踪条目在刷新时做存在性核验（补 git status 对
//!   未跟踪删除的盲区）；
//! - 驻留 = 首次构建后，dirty 未置 + 刷新节流窗口内 query 零 IO（不跑
//!   全树 stat）；
//! - 写后失效 = host 在写类工具（search_replace / run_terminal_cmd）成功
//!   完成后 `mark_dirty()`，模型刚写的内容下一 query 立即可搜；
//! - 同 size+mtime 修改残余盲区收敛：仅当 git status 与 stat 双 clean 才
//!   信任（git 对 stat-clean 不重 hash）；模型自身写入由写钩子覆盖，逃生阀
//!   `ORZ_PROJECT_DOC_INDEX_FORCE_RESCAN=1` 保留为机械兜底。
//! - 非 git 工作区（无 .git / git 不可用）→ 回退 v1 全树 stat + size/mtime
//!   增量语义（行为不变）。
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
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use orz_loop::host::{ToolError, ToolResult};
use serde::{Deserialize, Serialize};

/// Cache envelope schema version (sidecar convention).
/// v2 (2026-08-30)：新增 `blake3` 内容哈希与 `tracked` git 基线标记，
/// `CachedIndex` 新增 `git_mode`/`git_head`；旧 0.1.0-draft 缓存按
/// schema mismatch 全量重建（load_cache 的既有语义）。
const CACHE_SCHEMA_VERSION: &str = "0.2.0-draft";

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
    /// v2 (2026-08-30)：条目内容身份——变更/新文件重提取时对全文计算
    /// Blake3（跨 run 快照比对的稳定身份，补 size+mtime 盲区与 mtime
    /// 语义不稳）。读取失败 → None（保守：该条目下次刷新视为变更）。
    pub blake3: Option<String>,
    /// v2：条目是否属于 git HEAD 基线（受版本控制）。git 增量刷新只对
    /// 未跟踪条目做存在性核验（补 git status 对未跟踪删除的盲区）。
    pub tracked: bool,
}

/// On-disk cache envelope (`{cwd}/.gsa/project-doc-index/cache.json`).
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CachedIndex {
    schema_version: String,
    cwd_fingerprint: String,
    built_at_secs: u64,
    /// v2：快照构建时的工作区是否处于 git 模式（非 git 工作区恒 false）。
    #[serde(default)]
    git_mode: bool,
    /// v2：构建时 `git rev-parse HEAD` 的提交哈希（provenance；变更检测
    /// 由 git status 承担，不据此门控）。
    #[serde(default)]
    git_head: Option<String>,
    entries: Vec<CachedEntry>,
}

/// In-memory snapshot, Arc-shared between queries (entries always sorted by
/// relative_path — keeps query result order deterministic).
#[derive(Debug, Clone, Default)]
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
/// v2 (2026-08-30)：git 工作区下驻留索引 + git 增量（query 零全树 stat）；
/// 非 git 工作区保持 v1「每 query 全树 metadata 步行」。快照跨 run 持久化
/// 于 `.gsa/project-doc-index/`。
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
    /// v2：写后失效——写类工具成功完成后置位；下一 query 增量刷新。
    dirty: AtomicBool,
    /// v2：刷新节流窗口（毫秒）。默认 30000；0 = 每次 query 强制增量刷新。
    refresh_interval_ms: AtomicU64,
    /// v2：上次增量刷新时刻（`Option` 区分「尚未构建」）。
    last_refresh: Mutex<Option<std::time::Instant>>,
    /// v2：git 模式探针结果缓存（`rev-parse --is-inside-work-tree` 一次；
    /// git 命令失败 → 回退 v1，本进程内不再重探）。
    git_mode: AtomicBool,
}

impl ProjectDocIndex {
    pub fn new(cwd: PathBuf) -> Self {
        Self {
            fingerprint: fingerprint_of(&cwd),
            cwd,
            state: Mutex::new(None),
            force_rescan: AtomicBool::new(false),
            dirty: AtomicBool::new(false),
            refresh_interval_ms: AtomicU64::new(
                std::env::var("ORZ_PROJECT_DOC_INDEX_REFRESH_MS")
                    .ok()
                    .and_then(|s| s.trim().parse().ok())
                    .unwrap_or(30_000),
            ),
            last_refresh: Mutex::new(None),
            git_mode: AtomicBool::new(false),
        }
    }

    /// v2：写后失效——写类工具（search_replace / run_terminal_cmd）成功
    /// 完成后由 host 调用；下一 query 在节流窗口内也强制增量刷新。
    pub fn mark_dirty(&self) {
        self.dirty.store(true, Ordering::Relaxed);
    }

    /// Test/ops seam —— 刷新节流窗口（毫秒；0 = 每次 query 强制刷新）。
    #[doc(hidden)]
    pub fn set_refresh_interval_ms(&self, ms: u64) {
        self.refresh_interval_ms.store(ms, Ordering::Relaxed);
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
            ..Default::default()
        })
    }

    /// Diff-refresh the snapshot in one locked critical section (no awaits —
    /// this module is fully synchronous, so a std Mutex never spans an
    /// await point): lazy cache load on first build, metadata-only tree
    /// walk, diff against the old snapshot, re-extract only changed/new
    /// files, drop removed ones, re-sort, persist when dirty, Arc-swap.
    /// Concurrent callers merge into one build: the second lane reuses the
    /// freshly swapped snapshot instead of re-walking.
    ///
    /// v2 (2026-08-30)：git 工作区走驻留 + git 增量——首次构建全量一次，
    /// 之后 dirty 未置 + 节流窗口内直接返回（零 IO）；非 git 工作区保持
    /// v1「每 query 全树 stat 步行」语义（行为不变）。
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
            self.git_mode.store(cached.git_mode, Ordering::Relaxed);
            *guard = Some(Arc::new(CachedSnapshot {
                entries: cached.entries,
            }));
            loaded = true;
        }
        if guard.is_none() {
            // 首次构建（本进程）：全量扫描一次作为基线。git 模式下用
            // `git ls-files` 标记受版本控制条目（git HEAD 基线层）。
            self.git_mode
                .store(git_available(&self.cwd), Ordering::Relaxed);
            let tracked = if self.git_mode.load(Ordering::Relaxed) {
                git_tracked_set(&self.cwd)
            } else {
                None
            };
            let (stats, root_ok) = stat_pass(&self.cwd);
            let snap = if root_ok {
                let mut entries: Vec<CachedEntry> = stats
                    .iter()
                    .map(|st| entry_from_stat(&self.cwd, st, tracked.as_ref()))
                    .collect();
                entries.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
                Arc::new(CachedSnapshot { entries })
            } else {
                Arc::new(CachedSnapshot {
                    entries: Vec::new(),
                })
            };
            self.last_refresh
                .lock()
                .unwrap()
                .replace(std::time::Instant::now());
            if root_ok {
                self.persist_cache(&snap);
            }
            *guard = Some(snap.clone());
            return snap;
        }
        // 驻留路径：git 模式下 dirty 未置 + 节流窗口内 → 零 IO 直接返回。
        // （非 git 保持 v1「每 query 全树 stat」语义，不受节流窗口影响。）
        let git_mode = self.git_mode.load(Ordering::Relaxed);
        let dirty = self.dirty.swap(false, Ordering::Relaxed);
        let interval = self.refresh_interval_ms.load(Ordering::Relaxed);
        let within_window = self
            .last_refresh
            .lock()
            .unwrap()
            .is_some_and(|t| t.elapsed() < std::time::Duration::from_millis(interval));
        if git_mode && !force && !dirty && interval != 0 && within_window {
            return guard.as_ref().cloned().unwrap_or_default();
        }
        // 刷新：git 增量优先；git 命令失败 → 回退 v1 全树 stat（保守，
        // 结果正确、仅速度降级）。
        let mut snap = guard.as_ref().cloned().unwrap_or_default();
        let changed = if git_mode {
            match self.git_incremental(Arc::make_mut(&mut snap)) {
                Some(changed_any) => changed_any,
                None => {
                    self.git_mode.store(false, Ordering::Relaxed);
                    v1_refresh(&self.cwd, Arc::make_mut(&mut snap), force)
                }
            }
        } else {
            v1_refresh(&self.cwd, Arc::make_mut(&mut snap), force)
        };
        self.last_refresh
            .lock()
            .unwrap()
            .replace(std::time::Instant::now());
        if force || changed || (was_none && !loaded) {
            self.persist_cache(&snap);
        }
        *guard = Some(snap.clone());
        snap
    }

    /// v2：git 工作树增量刷新——`git status --porcelain -z` 报告
    /// modified/untracked/deleted，只重读变更文件（blake3 + title/headings）；
    /// 未跟踪条目做存在性核验（补 git status 对未跟踪删除的盲区）。
    /// `Some(changed)` = 成功；`None` = git 命令失败（调用方回退 v1）。
    fn git_incremental(&self, snap: &mut CachedSnapshot) -> Option<bool> {
        let changes = git_status_changes(&self.cwd)?;
        let mut entries = std::mem::take(&mut snap.entries);
        // 第一遍：删除集（deleted 状态 + 非文档变更 + 排除目录内路径）。
        // 先收集再统一删除，避免 remove 后 HashMap 索引失效。
        let mut to_remove: std::collections::HashSet<String> = std::collections::HashSet::new();
        for (rel, change) in &changes {
            let key = normalize_rel(rel);
            let full_path = self
                .cwd
                .join(rel.replace('/', std::path::MAIN_SEPARATOR_STR));
            match change {
                GitChange::Deleted => {
                    to_remove.insert(key);
                }
                GitChange::Present { .. } => {
                    if path_in_excluded_dir(rel) || !is_doc_file(&full_path) {
                        to_remove.insert(key);
                    }
                }
            }
        }
        let mut changed_any = !to_remove.is_empty();
        entries.retain(|e| !to_remove.contains(&normalize_rel(&e.relative_path)));
        // 第二遍：present 变更重提取（update 或 push）。索引重建自当前
        // entries（删除后索引已稳定）。
        let mut by_path: std::collections::HashMap<String, usize> = entries
            .iter()
            .enumerate()
            .map(|(i, e)| (normalize_rel(&e.relative_path), i))
            .collect();
        for (rel, change) in &changes {
            let GitChange::Present { tracked } = change else {
                continue;
            };
            let key = normalize_rel(rel);
            let full_path = self
                .cwd
                .join(rel.replace('/', std::path::MAIN_SEPARATOR_STR));
            if path_in_excluded_dir(rel) || !is_doc_file(&full_path) {
                continue;
            }
            if let Some(st) = file_stat(&self.cwd, &full_path) {
                let mut e = entry_from_stat(&self.cwd, &st, None);
                // git 报告的 Present（modified/added/untracked）：
                // 只有 `??`（untracked）是未跟踪，其余为受版本控制。
                e.tracked = *tracked;
                match by_path.get(&key) {
                    Some(&i) => entries[i] = e,
                    None => {
                        by_path.insert(key.clone(), entries.len());
                        entries.push(e);
                    }
                }
                changed_any = true;
            }
        }
        // 未跟踪条目的存在性核验（git status 不报告未跟踪文件的删除）。
        let mut i = 0;
        while i < entries.len() {
            if !entries[i].tracked && !std::path::Path::new(&entries[i].path).exists() {
                entries.remove(i);
                changed_any = true;
            } else {
                i += 1;
            }
        }
        entries.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
        snap.entries = entries;
        Some(changed_any)
    }

    /// discover()'s path: force a full re-extract (skip cache load and the
    /// diff reuse), persist the result as the new baseline.
    fn refresh_forced(&self) -> Arc<CachedSnapshot> {
        let mut guard = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let tracked = if self.git_mode.load(Ordering::Relaxed) {
            git_tracked_set(&self.cwd)
        } else {
            None
        };
        let (stats, root_ok) = stat_pass(&self.cwd);
        if !root_ok {
            return guard.as_ref().cloned().unwrap_or_default();
        }
        let mut entries: Vec<CachedEntry> = stats
            .iter()
            .map(|st| entry_from_stat(&self.cwd, st, tracked.as_ref()))
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
            git_mode: self.git_mode.load(Ordering::Relaxed),
            git_head: git_head_sha(&self.cwd),
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
/// headings + Blake3 内容哈希 — the only content read in the whole refresh
/// path). `tracked_set` = git HEAD 受版本控制文件集合（None = 非 git /
/// 未知，全部标未跟踪；git 增量路径在调用后按状态覆盖）。
fn entry_from_stat(
    cwd: &Path,
    st: &FileStat,
    tracked_set: Option<&HashSet<String>>,
) -> CachedEntry {
    let entry = entry_from_path(cwd, &st.path);
    let (headings, blake3) = extract_headings_and_hash(&st.path);
    CachedEntry {
        path: entry.path,
        relative_path: entry.relative_path,
        title: entry.title,
        headings,
        // size comes from the walk's stat — single source so the diff key
        // and the entry never disagree across the stat/extract TOCTOU
        // (review P3-1).
        size: st.size,
        mtime_secs: st.mtime_secs,
        mtime_nanos: st.mtime_nanos,
        blake3,
        tracked: tracked_set.is_some_and(|set| set.contains(&normalize_rel(&st.relative_path))),
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

/// v2：全文流式读 + Blake3 内容哈希（峰值内存有界——1MB 分块；headings
/// 只取前 256KB，与 `extract_headings` 口径一致）。读取失败 → (空, None)
/// （保守：该条目下次刷新视为变更）。
fn extract_headings_and_hash(path: &Path) -> (Vec<String>, Option<String>) {
    use std::io::Read;
    let Ok(mut file) = std::fs::File::open(path) else {
        return (Vec::new(), None);
    };
    let mut hasher = blake3::Hasher::new();
    let mut head: Vec<u8> = Vec::new();
    let mut buf = [0u8; 1 << 20];
    loop {
        match file.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                hasher.update(&buf[..n]);
                if head.len() < 256 * 1024 {
                    let take = n.min(256 * 1024 - head.len());
                    head.extend_from_slice(&buf[..take]);
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => return (Vec::new(), None),
        }
    }
    let text = String::from_utf8_lossy(&head);
    let headings = text
        .lines()
        .filter_map(|l| {
            let t = l.trim_start();
            if t.starts_with("# ") {
                Some(t.trim_start_matches('#').trim().to_string())
            } else {
                None
            }
        })
        .take(64)
        .collect();
    (headings, Some(hasher.finalize().to_hex().to_string()))
}

/// v2：git 工作区探针——`git rev-parse --is-inside-work-tree` 成功且输出
/// `true`。失败/无 git → false（回退 v1）。
fn git_available(cwd: &Path) -> bool {
    std::process::Command::new("git")
        .arg("-C")
        .arg(cwd)
        .args(["rev-parse", "--is-inside-work-tree"])
        .output()
        .ok()
        .is_some_and(|out| {
            out.status.success() && String::from_utf8_lossy(&out.stdout).trim() == "true"
        })
}

/// v2：`git ls-files -z` 受版本控制文件集合（规范化相对路径，Windows
/// 反斜杠统一为正斜杠）。失败 → None（调用方按无基线处理）。
fn git_tracked_set(cwd: &Path) -> Option<HashSet<String>> {
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(cwd)
        .args(["ls-files", "-z"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let mut set = HashSet::new();
    for rec in out.stdout.split(|&b| b == 0) {
        if rec.is_empty() {
            continue;
        }
        if let Ok(rel) = std::str::from_utf8(rec) {
            set.insert(normalize_rel(rel));
        }
    }
    Some(set)
}

/// v2：`git rev-parse HEAD`（provenance；best-effort，失败 → None）。
fn git_head_sha(cwd: &Path) -> Option<String> {
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(cwd)
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// v2：git status 变更集——`--porcelain -z --no-renames
/// --untracked-files=all`（NUL 分隔，无 rename 两条路径的解析歧义）。
/// 失败/畸形输出 → None（调用方回退 v1 全树 stat）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GitChange {
    Deleted,
    Present { tracked: bool },
}

fn git_status_changes(cwd: &Path) -> Option<Vec<(String, GitChange)>> {
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(cwd)
        .args([
            "status",
            "--porcelain",
            "-z",
            "--no-renames",
            "--untracked-files=all",
        ])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let mut changes = Vec::new();
    let stdout = out.stdout;
    let mut i = 0usize;
    while i < stdout.len() {
        let nul = stdout[i..].iter().position(|&b| b == 0).map(|p| i + p)?;
        let rec = &stdout[i..nul];
        if rec.len() >= 4 {
            let status = &rec[..2];
            let rel = std::str::from_utf8(&rec[3..]).ok()?.to_string();
            let deleted = status.contains(&b'D');
            let untracked = status == b"??";
            changes.push((
                rel,
                if deleted {
                    GitChange::Deleted
                } else {
                    GitChange::Present {
                        tracked: !untracked,
                    }
                },
            ));
        }
        i = nul + 1;
    }
    Some(changes)
}

/// 相对路径规范化（git 正斜杠 vs Windows 反斜杠统一为 `/`）。
fn normalize_rel(rel: &str) -> String {
    rel.replace('\\', "/")
}

/// 路径是否落在 EXCLUDED_DIRS 任一目录内（与 stat_pass 语义一致——
/// `.git/.gsa/target/...` 永不入索引）。
fn path_in_excluded_dir(rel: &str) -> bool {
    rel.split('/')
        .take(rel.split('/').count().saturating_sub(1))
        .any(|c| EXCLUDED_DIRS.contains(&c))
}

/// v1 回退路径：全树 metadata 步行 + 与既有快照 diff（既有语义——
/// 每 query 全树 stat，只重读变更文件）。返回条目是否变化（驱动持久化）；
/// 根目录不可读时保持快照原样（P2-2）。
fn v1_refresh(cwd: &Path, snap: &mut CachedSnapshot, force: bool) -> bool {
    let (stats, root_ok) = stat_pass(cwd);
    if !root_ok {
        return false;
    }
    let old: HashMap<&str, &CachedEntry> = snap
        .entries
        .iter()
        .map(|e| (e.relative_path.as_str(), e))
        .collect();
    let mut new_entries: Vec<CachedEntry> = Vec::with_capacity(stats.len());
    let mut reused = 0usize;
    for st in &stats {
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
                blake3: e.blake3.clone(),
                tracked: e.tracked,
            });
        } else {
            new_entries.push(entry_from_stat(cwd, st, None));
        }
    }
    let not_reused = old.len() - reused;
    new_entries.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
    snap.entries = new_entries;
    (stats.len() - reused) > 0 || not_reused > 0
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
        assert_eq!(parsed["schema_version"], "0.2.0-draft");
        assert_eq!(parsed["git_mode"], false);
        assert_eq!(
            parsed["entries"].as_array().unwrap()[0]["blake3"].is_string(),
            true
        );
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
        assert_eq!(parsed["schema_version"], "0.2.0-draft");
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
        assert_eq!(parsed["schema_version"], "0.2.0-draft");
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
            r#"{"schema_version":"0.2.0-draft","cwd_fingerprint":"some-other-dir","built_at_secs":0,"git_mode":false,"git_head":null,"entries":[]}"#,
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
            "schema_version": "0.2.0-draft",
            "cwd_fingerprint": index.fingerprint,
            "built_at_secs": 0,
            "git_mode": false,
            "git_head": null,
            "entries": [{
                "path": secret.to_string_lossy(),
                "relative_path": "a.md",
                "title": "A",
                "headings": ["A"],
                "size": md.len(),
                "mtime_secs": dur.as_secs(),
                "mtime_nanos": dur.subsec_nanos(),
                "blake3": "a".repeat(64),
                "tracked": false,
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

    // ── v2 (2026-08-30): git 基线 + 工作树增量层 + Blake3 + 驻留 ──

    fn run_git(dir: &Path, args: &[&str]) {
        // 完全本地化：避免并行测试竞争共享的用户级 git 配置
        // （`~/.config/git/ignore` 权限告警 / 锁竞争 → 偶发失败）。
        let global_config = dir.join(".git-test-config");
        std::fs::write(&global_config, "").unwrap();
        let status = std::process::Command::new("git")
            .arg("-C")
            .arg(dir)
            .args(args)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", &global_config)
            .status()
            .unwrap();
        assert!(status.success(), "git {args:?} failed");
    }

    fn git_commit(dir: &Path) {
        run_git(dir, &["add", "-A"]);
        run_git(dir, &["commit", "-q", "-m", "c"]);
    }

    /// git 工作区夹具；git 不可用时跳过（CI/无 git 环境不阻塞）。
    fn git_dir() -> Option<PathBuf> {
        let dir = test_dir();
        run_git(&dir, &["init", "-q"]);
        run_git(&dir, &["config", "user.email", "t@example.invalid"]);
        run_git(&dir, &["config", "user.name", "t"]);
        if !git_available(&dir) {
            let _ = std::fs::remove_dir_all(&dir);
            return None;
        }
        Some(dir)
    }

    #[test]
    fn git_baseline_incremental_add_modify_delete_and_untracked() {
        let Some(dir) = git_dir() else { return };
        std::fs::write(dir.join("README.md"), "# Readme\n").unwrap();
        std::fs::write(dir.join("tracked.md"), "# Tracked\n").unwrap();
        git_commit(&dir);
        std::fs::write(dir.join("untracked.md"), "# Untracked\n").unwrap();

        let index = ProjectDocIndex::new(dir.clone());
        index.set_refresh_interval_ms(0); // 测试确定性：每 query 刷新
        let out = index.query(&serde_json::json!({})).unwrap();
        assert!(out.output.contains("README.md"), "{}", out.output);
        assert!(out.output.contains("tracked.md"), "{}", out.output);
        assert!(out.output.contains("untracked.md"), "{}", out.output);

        // 修改受版本控制文件 → 增量重提取。
        std::fs::write(dir.join("tracked.md"), "# Tracked2\n").unwrap();
        let out = index
            .query(&serde_json::json!({"query": "tracked2"}))
            .unwrap();
        assert!(out.output.contains("tracked.md"), "{}", out.output);
        // 新增未跟踪文档 → 增量入索引。
        std::fs::write(dir.join("new.md"), "# New\n").unwrap();
        let out = index.query(&serde_json::json!({"query": "new"})).unwrap();
        assert!(out.output.contains("new.md"), "{}", out.output);
        // 删除受版本控制文件 → 增量移除。
        std::fs::remove_file(dir.join("README.md")).unwrap();
        let out = index.query(&serde_json::json!({})).unwrap();
        assert!(!out.output.contains("README.md"), "{}", out.output);
        // 删除未跟踪文件 → 存在性核验移除。
        std::fs::remove_file(dir.join("untracked.md")).unwrap();
        let out = index.query(&serde_json::json!({})).unwrap();
        assert!(!out.output.contains("untracked.md"), "{}", out.output);

        // 缓存条目：受版本控制 tracked=true、未跟踪 tracked=false，均带 blake3。
        let cache = dir.join(".gsa/project-doc-index/cache.json");
        let parsed: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&cache).unwrap()).unwrap();
        assert_eq!(parsed["git_mode"], true);
        assert_eq!(parsed["git_head"].is_string(), true);
        let entries = parsed["entries"].as_array().unwrap();
        let tracked = entries
            .iter()
            .find(|e| e["relative_path"].as_str().unwrap().ends_with("tracked.md"))
            .unwrap();
        assert_eq!(tracked["tracked"], true);
        assert_eq!(tracked["blake3"].is_string(), true);
        let new_entry = entries
            .iter()
            .find(|e| e["relative_path"].as_str().unwrap().ends_with("new.md"))
            .unwrap();
        assert_eq!(new_entry["tracked"], false);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 驻留快路径：dirty 未置 + 节流窗口内 query 零 IO（外部直接改盘在
    /// 窗口内不可见）；`mark_dirty()`（写后失效钩子）强制下一 query 刷新。
    #[test]
    fn git_resident_fast_path_and_write_after_invalidation() {
        let Some(dir) = git_dir() else { return };
        std::fs::write(dir.join("a.md"), "# One\n").unwrap();
        git_commit(&dir);
        let index = ProjectDocIndex::new(dir.clone());
        // 默认节流窗口（30s）：首 query 构建后进入驻留。
        let out = index.query(&serde_json::json!({"query": "one"})).unwrap();
        assert!(out.output.contains("a.md"), "{}", out.output);
        // 外部直接改盘（无写钩子）→ 窗口内快路径，旧内容仍可见。
        std::fs::write(dir.join("a.md"), "# Two\n").unwrap();
        let out = index.query(&serde_json::json!({"query": "two"})).unwrap();
        assert!(!out.output.contains("a.md"), "{}", out.output);
        // 写后失效钩子 → 下一 query 增量刷新，新内容可见。
        index.mark_dirty();
        let out = index.query(&serde_json::json!({"query": "two"})).unwrap();
        assert!(out.output.contains("a.md"), "{}", out.output);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 逃生阀兜底：`set_force_rescan` 全量重建，任何盲区内容修改可见
    /// （同 size+mtime 的残留盲区登记于模块文档——git 可能经 racy-clean
    /// 内容比较自行捕获，也可能双 clean 不可察，机械兜底是逃生阀）。
    #[test]
    fn git_escape_hatch_full_rebuild_sees_same_size_mtime_edit() {
        let Some(dir) = git_dir() else { return };
        let file = dir.join("a.md");
        std::fs::write(&file, "# One\n").unwrap();
        git_commit(&dir);
        let index = ProjectDocIndex::new(dir.clone());
        index.set_refresh_interval_ms(0);
        let _ = index.query(&serde_json::json!({})).unwrap();
        // 同 size + 恢复 mtime + git status clean → 盲区（登记语义）。
        let mtime = std::fs::metadata(&file).unwrap().modified().unwrap();
        std::fs::write(&file, "# Two\n").unwrap();
        filetime::set_file_mtime(&file, filetime::FileTime::from_system_time(mtime)).unwrap();
        // 逃生阀全量重建 → 可见。
        index.set_force_rescan(true);
        let out = index.query(&serde_json::json!({"query": "two"})).unwrap();
        assert!(out.output.contains("a.md"), "{}", out.output);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 写后失效 + 正常 mtime 变化（模型真实写入形态）→ 立即可搜。
    #[test]
    fn git_write_after_normal_edit_is_visible() {
        let Some(dir) = git_dir() else { return };
        std::fs::write(dir.join("a.md"), "# One\n").unwrap();
        git_commit(&dir);
        let index = ProjectDocIndex::new(dir.clone());
        let _ = index.query(&serde_json::json!({"query": "one"})).unwrap();
        std::fs::write(dir.join("a.md"), "# Two\n").unwrap();
        index.mark_dirty();
        let out = index.query(&serde_json::json!({"query": "two"})).unwrap();
        assert!(out.output.contains("a.md"), "{}", out.output);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 排除目录内的变更（.gsa 缓存自身、target 等）不进入增量刷新。
    #[test]
    fn git_incremental_skips_excluded_dirs() {
        let Some(dir) = git_dir() else { return };
        std::fs::write(dir.join("a.md"), "# A\n").unwrap();
        git_commit(&dir);
        std::fs::create_dir_all(dir.join("target")).unwrap();
        std::fs::write(dir.join("target/junk.md"), "# Junk\n").unwrap();
        std::fs::create_dir_all(dir.join(".gsa")).unwrap();
        std::fs::write(dir.join(".gsa/x.md"), "# Gsa\n").unwrap();
        let index = ProjectDocIndex::new(dir.clone());
        index.set_refresh_interval_ms(0);
        let out = index.query(&serde_json::json!({})).unwrap();
        assert!(out.output.contains("a.md"), "{}", out.output);
        assert!(!out.output.contains("junk.md"), "{}", out.output);
        assert!(!out.output.contains("x.md"), "{}", out.output);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
