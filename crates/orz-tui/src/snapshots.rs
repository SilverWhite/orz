//! Snapshot discovery for the snapshot selector (Phase 3 slice #10) — a
//! read-only scan of `.gsa/runs/RUN-*/events.jsonl` for `snapshot_created`
//! events (the loop records one per pre-mutation snapshot), enriched with
//! the manifest's file count and creation date. The selector is the
//! TUI-side entry to `AcpServer::restore_snapshot` (slice #8); restoring
//! always targets the live session's worktree, so the run a snapshot came
//! from is provenance only.
//!
//! NOTE: distinct from `crate::snapshot` (render-snapshot test helpers).
//!
//! No `orz_assurance` import: bridge.rs declares itself the only module
//! allowed to import it, so journal lines deserialize into a minimal local
//! struct instead (journal_tail.rs:18 has the same precedent). Discovery is
//! lenient — malformed lines are skipped, no chain validation (the tail
//! parses the same way).
//!
//! v1 boundaries (design review D3): restore is FULL-only — the runner
//! always passes `scope = None` (selective revert is deferred; the host API
//! supports it). The manifest file count/date are a lightweight direct read
//! of `manifests/{hash}.json`, NOT a public SnapshotStore API call.

use std::path::Path;

/// One discoverable pre-mutation snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotEntry {
    /// Full 64-hex snapshot hash (content-addressed manifest key).
    pub hash: String,
    /// Tool that triggered the snapshot (e.g. `search_replace`); `"—"` when
    /// the journal payload lacks the field.
    pub tool: String,
    /// Worktree-relative target paths recorded at snapshot time.
    pub targets: Vec<String>,
    /// Full run dir name, `RUN-{session8}-{n}` (provenance only).
    pub run_id: String,
    /// `YYYY-MM-DD` from the manifest file's mtime (fallback: run dir).
    pub date: String,
    /// File count from the manifest (`None` → render "—" — manifest
    /// missing or corrupt).
    pub file_count: Option<usize>,
}

/// Minimal journal-line shape — enough to find `snapshot_created` events
/// without importing orz_assurance (see module doc). Deserialized manually
/// from a `serde_json::Value` to avoid a serde-derive dependency. The run
/// id comes from the run dir name, not the line.
struct JournalLine {
    event_type: String,
    payload: serde_json::Value,
}

impl JournalLine {
    fn parse(line: &str) -> Option<Self> {
        let v: serde_json::Value = serde_json::from_str(line).ok()?;
        Some(Self {
            event_type: v.get("event_type")?.as_str()?.to_string(),
            payload: v.get("payload")?.clone(),
        })
    }
}

/// Scan every `RUN-{session8}-{n}` journal for successful
/// `snapshot_created` events. Order: newest run first (numeric turn order —
/// review D1-1, lexicographic breaks past turn 9), journal order within a
/// run (stable sort), deduped by hash keeping the newest occurrence.
///
/// v1 limits (module doc): full journal read at open time (no live refresh
/// while the selector is open), no staleness filtering beyond dedup.
pub fn discover_snapshots(cwd: &Path) -> Vec<SnapshotEntry> {
    let runs = cwd.join(".gsa").join("runs");
    let Ok(rd) = std::fs::read_dir(&runs) else {
        return Vec::new();
    };
    let mut found: Vec<SnapshotEntry> = Vec::new();
    for e in rd.flatten() {
        let dir = e.path();
        let Some(name) = dir.file_name().map(|n| n.to_string_lossy().into_owned()) else {
            continue;
        };
        let Some((_sid, n_str)) = name.strip_prefix("RUN-").and_then(|r| r.rsplit_once('-')) else {
            continue;
        };
        if n_str.parse::<u32>().is_err() {
            continue;
        }
        let events_path = dir.join("events.jsonl");
        let Ok(text) = std::fs::read_to_string(&events_path) else {
            continue;
        };
        // Fallback date when a snapshot has no manifest on disk.
        let dir_date = std::fs::metadata(&dir)
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(crate::explorer::format_date)
            .unwrap_or_default();
        for line in text.lines() {
            let Some(line) = JournalLine::parse(line) else {
                continue; // malformed line — lenient, mirror journal_tail
            };
            if line.event_type != "snapshot_created" {
                continue;
            }
            // snapshot_hash / snapshot_error are mutually exclusive
            // (bridge.rs) — error events have no hash and nothing to
            // restore, so they fall out here.
            let Some(hash) = line
                .payload
                .get("snapshot_hash")
                .and_then(|v| v.as_str())
                .filter(|h| !h.is_empty())
                .map(str::to_string)
            else {
                continue;
            };
            let tool = line
                .payload
                .get("tool")
                .and_then(|v| v.as_str())
                .unwrap_or("—")
                .to_string();
            let targets = line
                .payload
                .get("targets")
                .and_then(|v| v.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str().map(str::to_string))
                        .collect()
                })
                .unwrap_or_default();
            let manifest = cwd
                .join(".gsa")
                .join("snapshots")
                .join("manifests")
                .join(format!("{hash}.json"));
            let file_count = std::fs::read_to_string(&manifest)
                .ok()
                .and_then(|m| serde_json::from_str::<serde_json::Value>(&m).ok())
                .and_then(|v| v.as_array().map(|a| a.len()));
            let date = std::fs::metadata(&manifest)
                .ok()
                .and_then(|m| m.modified().ok())
                .and_then(crate::explorer::format_date)
                .unwrap_or_else(|| dir_date.clone());
            found.push(SnapshotEntry {
                hash,
                tool,
                targets,
                run_id: name.clone(),
                date,
                file_count,
            });
        }
    }
    // Newest first (numeric turn order — D1-1); the stable sort keeps
    // journal order within a run; dedup keeps the newest per hash.
    found.sort_by(|a, b| {
        let (sid_a, n_a) = crate::explorer::run_parts(&a.run_id);
        let (sid_b, n_b) = crate::explorer::run_parts(&b.run_id);
        sid_b.cmp(&sid_a).then_with(|| n_b.cmp(&n_a))
    });
    let mut seen = std::collections::HashSet::new();
    found.retain(|e| seen.insert(e.hash.clone()));
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use orz_assurance::journal::{EventType, Redaction, RunEvent};

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "orz-tui-snapshots-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// A sealed journal line, mirroring the payload shape the loop records
    /// (orz-loop controller.rs: snapshot_hash | snapshot_error, tool,
    /// targets).
    fn sealed_line(
        run_id: &str,
        event_type: EventType,
        payload: serde_json::Value,
    ) -> String {
        let mut ev = RunEvent::new(
            run_id.to_string(),
            0,
            event_type,
            "m".into(),
            None,
            "run-event-v0.1.schema.json".into(),
            payload,
            Redaction::None,
            "2026-08-05T00:00:00Z".into(),
        );
        orz_assurance::seal_event(&mut ev).unwrap();
        format!("{}\n", serde_json::to_string(&ev).unwrap())
    }

    fn write_journal(run_dir: &Path, lines: &[String]) {
        std::fs::create_dir_all(run_dir).unwrap();
        std::fs::write(
            run_dir.join("events.jsonl"),
            lines.concat(),
        )
        .unwrap();
    }

    fn created(tool: &str, targets: &[&str], hash: &str) -> serde_json::Value {
        serde_json::json!({
            "tool": tool,
            "targets": targets,
            "snapshot_hash": hash,
        })
    }

    #[test]
    fn discovers_snapshot_created_across_runs() {
        let dir = temp_dir("discover");
        let runs = dir.join(".gsa").join("runs");
        write_journal(
            &runs.join("RUN-a1b2c3d4-0"),
            &[
                sealed_line("RUN-a1b2c3d4-0", EventType::RunStarted, serde_json::json!({"prompt": "x"})),
                sealed_line("RUN-a1b2c3d4-0", EventType::SnapshotCreated, created("search_replace", &["lib.rs"], "a".repeat(64).as_str())),
                "not json at all\n".to_string(), // malformed — skipped
                sealed_line("RUN-a1b2c3d4-0", EventType::ModelOutput, serde_json::json!({"content": "x"})),
            ],
        );
        // A snapshot_error event carries no hash — nothing to restore.
        write_journal(
            &runs.join("RUN-b0b0b0b0-0"),
            &[sealed_line(
                "RUN-b0b0b0b0-0",
                EventType::SnapshotCreated,
                serde_json::json!({"tool": "edit_file", "targets": ["x"], "snapshot_error": "disk full"}),
            )],
        );
        // A dir without events.jsonl does not qualify.
        std::fs::create_dir_all(runs.join("RUN-nosuch00-0")).unwrap();

        let found = discover_snapshots(&dir);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].run_id, "RUN-a1b2c3d4-0");
        assert_eq!(found[0].tool, "search_replace");
        assert_eq!(found[0].targets, ["lib.rs"]);
        assert_eq!(found[0].hash, "a".repeat(64));
        // No manifest written → file count unknown, date from run dir.
        assert_eq!(found[0].file_count, None);
        assert!(!found[0].date.is_empty() && found[0].date.len() == 10);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn sorts_newest_first_numeric_within_session() {
        let dir = temp_dir("sort");
        let runs = dir.join(".gsa").join("runs");
        // The D1-1 numeric-sort regression: turn 10 must sort above turn 1
        // within the session (lexicographic name order would misplace it).
        for (sid, n) in [
            ("RUN-a1b2c3d4", 0u32),
            ("RUN-a1b2c3d4", 1),
            ("RUN-a1b2c3d4", 10),
            ("RUN-z9y8x7w6", 0),
        ] {
            let run_id = format!("{sid}-{n}");
            write_journal(
                &runs.join(&run_id),
                &[sealed_line(
                    &run_id,
                    EventType::SnapshotCreated,
                    created("write_file", &["a.txt"], &format!("h{run_id}")),
                )],
            );
        }
        let found = discover_snapshots(&dir);
        assert_eq!(found.len(), 4);
        // Newest session name first, then NUMERIC turn order.
        assert_eq!(found[0].run_id, "RUN-z9y8x7w6-0");
        let a: Vec<&str> = found
            .iter()
            .filter(|e| e.run_id.starts_with("RUN-a1b2c3d4"))
            .map(|e| e.run_id.as_str())
            .collect();
        assert_eq!(
            a,
            ["RUN-a1b2c3d4-10", "RUN-a1b2c3d4-1", "RUN-a1b2c3d4-0"],
            "numeric turn order within a session (review D1-1)"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn dedupes_by_hash_keeping_newest() {
        let dir = temp_dir("dedup");
        let runs = dir.join(".gsa").join("runs");
        // Same content-addressed hash recorded in two runs (same state
        // snapshotted twice) — the newer run's row wins.
        for (sid, n) in [("RUN-a1b2c3d4", 0u32), ("RUN-a1b2c3d4", 1)] {
            let run_id = format!("{sid}-{n}");
            write_journal(
                &runs.join(&run_id),
                &[sealed_line(&run_id, EventType::SnapshotCreated, created("edit_file", &["a.txt"], &"c".repeat(64)))],
            );
        }
        let found = discover_snapshots(&dir);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].run_id, "RUN-a1b2c3d4-1", "newest occurrence wins");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn enriches_file_count_and_date_from_manifest() {
        let dir = temp_dir("manifest");
        let runs = dir.join(".gsa").join("runs");
        let snapshots = dir.join(".gsa").join("snapshots");
        let run_id = "RUN-a1b2c3d4-0";
        let with_manifest = "d".repeat(64);
        let without_manifest = "e".repeat(64);
        write_journal(
            &runs.join(run_id),
            &[
                sealed_line(run_id, EventType::SnapshotCreated, created("write_file", &["a.txt"], &with_manifest)),
                sealed_line(run_id, EventType::SnapshotCreated, created("write_file", &["b.txt"], &without_manifest)),
            ],
        );
        // Canonical manifest shape: a JSON array of SnapshotEntry.
        std::fs::create_dir_all(snapshots.join("manifests")).unwrap();
        std::fs::write(
            snapshots.join("manifests").join(format!("{with_manifest}.json")),
            serde_json::json!([
                {"path": "a.txt", "object_sha256": "abc", "size": 1, "mtime_nanos": 0},
                {"path": "sub/b.txt", "object_sha256": "def", "size": 2, "mtime_nanos": 0},
            ])
            .to_string(),
        )
        .unwrap();

        let found = discover_snapshots(&dir);
        assert_eq!(found.len(), 2);
        let with = found.iter().find(|e| e.hash == with_manifest).unwrap();
        assert_eq!(with.file_count, Some(2));
        assert!(!with.date.is_empty() && with.date.len() == 10);
        let without = found.iter().find(|e| e.hash == without_manifest).unwrap();
        assert_eq!(without.file_count, None, "missing manifest → None");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn empty_or_missing_runs_yields_empty() {
        let dir = temp_dir("empty");
        // No .gsa at all → empty.
        assert!(discover_snapshots(&dir).is_empty());
        // .gsa/runs exists but empty → empty.
        std::fs::create_dir_all(dir.join(".gsa").join("runs")).unwrap();
        assert!(discover_snapshots(&dir).is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
