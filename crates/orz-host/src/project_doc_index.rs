//! Project-doc index (GAP-RETRIEVAL-TOOLS 2026-08-10) — the internal
//! retrieval lane's real discovery/query tool (ADR-0010 §3.7.4/§3.7.5).
//!
//! Host-owned synchronous tool (run_tests precedent): the model never
//! supplies a path walk — discovery is workspace-wide under a FIXED
//! exclusion set (.git/.gsa/target/node_modules/.venv/archive dirs), and
//! the query matches keywords against relative path / title / headings.
//! `include_content=true` returns (capped) full text — the mechanical
//! evidence collector maps that to `full_text_observed`; the metadata
//! default maps to `metadata_only`.
//!
//! Registered boundary: the index is rebuilt on every query (correctness
//! first; a cached/mtime-diffed index is a later optimization).

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use orz_loop::host::{ToolError, ToolResult};
use serde::Serialize;

/// Document/source extensions the index recognizes.
const DOC_EXTENSIONS: &[&str] = &[
    "md", "txt", "rst", "adoc", "ron", "toml", "yaml", "yml", "json", "rs", "py", "ts", "js",
    "c", "h", "cpp", "hpp", "go", "java", "rb", "sh", "ps1", "html", "css",
];

/// Directory names never traversed.
const EXCLUDED_DIRS: &[&str] = &[
    ".git", ".gsa", "target", "node_modules", ".venv", "venv", "存档", "archive", "dist",
    "build", ".hidden",
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

/// The workspace document index — stateless scan per query.
#[derive(Debug)]
pub struct ProjectDocIndex {
    cwd: PathBuf,
}

impl ProjectDocIndex {
    pub fn new(cwd: PathBuf) -> Self {
        Self { cwd }
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

    /// Scan the workspace once and return every recognized document.
    pub fn discover(&self) -> Vec<DocEntry> {
        let mut entries = Vec::new();
        let mut stack: Vec<PathBuf> = vec![self.cwd.clone()];
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
                } else if is_doc_file(&path) {
                    entries.push(entry_from_path(&self.cwd, &path));
                }
            }
        }
        entries.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
        entries
    }

    /// Run one query against a fresh scan. `args` is the tool-call argument
    /// object (parsed strictly — unknown shapes are explicit errors, never
    /// a silent empty result).
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

        let mut hits: Vec<QueryHit> = Vec::new();
        let mut truncated = false;
        for entry in self.discover() {
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
                        let text = String::from_utf8_lossy(&bytes[..bytes.len().min(max_content_bytes)])
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
        })
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
        let dir = std::env::temp_dir().join(format!("orz-doc-index-test-{n}-{:?}", std::thread::current().id()));
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
        assert_eq!(rels, vec!["README.md", "docs/api.md", "main.rs"], "{rels:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn query_matches_keywords_and_returns_content_capped() {
        let dir = test_dir();
        std::fs::write(dir.join("README.md"), "# Readme\nhello world").unwrap();
        std::fs::write(dir.join("docs/api.md"), "# API\nthe api docs").unwrap();
        let index = ProjectDocIndex::new(dir.clone());

        // Keyword hit on headings.
        let out = index
            .query(&serde_json::json!({"query": "api"}))
            .unwrap();
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
}
