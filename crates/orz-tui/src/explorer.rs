//! Explorer pane — the left sidebar (Phase 3 slice #9): a lazy file tree of
//! the session working directory plus a live event-count section, and the
//! session list (double-Esc mode) — a read-only view over `.gsa/runs/`
//! journals (SESSION_PERSISTENCE_AND_LAYOUT_COMPACTION_v0.1: the TUI never
//! owns a session store; Enter only reports the replay hint).
//!
//! Python reference: `ExplorerPane` (widgets.py) — display-only there; the
//! Rust pane adds keyboard navigation (design doc GRR L7-002).

use std::path::{Path, PathBuf};

/// A tree node — a directory (expandable) or a file (has an object URI).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeNode {
    pub label: String,
    pub children: Vec<TreeNode>,
    pub expanded: bool,
    /// Files carry a `file://`-style display path (the /open URI).
    pub uri: Option<String>,
}

/// One discovered run — the session list's row unit (v1: one entry per run
/// journal; the host writes no session.json markers, so everything derives
/// from the run dir + its events.jsonl).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionEntry {
    pub run_dir: PathBuf,
    /// Full run dir name, `RUN-{session8}-{n}`.
    pub session_id: String,
    /// First prompt text (peeked from the journal), truncated later.
    pub prompt_preview: String,
    /// Turn number `n + 1` (parsed from the dir name suffix).
    pub turn_count: u32,
    /// `YYYY-MM-DD` from the run dir's modified time (no marker timestamps
    /// exist in the Rust journal world).
    pub date: String,
}

/// The explorer pane state.
#[derive(Debug, Clone)]
pub struct ExplorerPane {
    pub tree: Vec<TreeNode>,
    /// `load()` has run (runner loads once at loop start — never in the
    /// render path, keeping unit tests fs-free).
    pub loaded: bool,
    /// Flat preorder index over the visible (expanded) rows.
    pub selected: usize,
    /// Events section (`/toggle-events`) — live counts derived from the
    /// event log.
    pub show_events: bool,
    /// Session-list mode (double-Esc toggles tree ↔ sessions).
    pub session_mode: bool,
    /// The double-Esc entry auto-showed a hidden explorer — exit restores
    /// the prior visibility (review D2-3: a transient gesture must not
    /// permanently mutate the user's layout preference).
    pub session_auto_shown: bool,
    pub sessions: Vec<SessionEntry>,
    pub session_selected: usize,
    /// (kind, count) pairs sorted by count desc, top 10.
    pub events: Vec<(String, usize)>,
}

impl Default for ExplorerPane {
    fn default() -> Self {
        Self {
            tree: Vec::new(),
            loaded: false,
            selected: 0,
            show_events: true,
            session_mode: false,
            session_auto_shown: false,
            sessions: Vec::new(),
            session_selected: 0,
            events: Vec::new(),
        }
    }
}

impl ExplorerPane {
    /// Load the cwd tree (two levels below the root), dirs first, `.gsa` /
    /// `.git` skipped at every level. Depth-bounded so the initial scan is
    /// cheap; the tree is a v1 snapshot (no live fs refresh).
    pub fn load(&mut self, cwd: &Path) {
        self.tree = load_dir(cwd, 0);
        self.loaded = true;
        self.selected = 0;
    }

    /// Visible rows in preorder (an expanded node exposes its children).
    pub fn visible_len(&self) -> usize {
        visible_count(&self.tree)
    }

    fn visible_node_mut(&mut self, index: usize) -> Option<&mut TreeNode> {
        let mut remaining = index;
        walk_mut(&mut self.tree, &mut remaining)
    }

    pub fn move_selection(&mut self, delta: isize) {
        let len = self.visible_len();
        if len == 0 {
            return;
        }
        let next = (self.selected as isize + delta).clamp(0, len as isize - 1) as usize;
        self.selected = next;
    }

    /// ← on an expanded dir collapses it; ← on a collapsed dir / file
    /// selects the parent row (review P3-2 — the parent is the previous
    /// visible row at depth-1, not the previous sibling).
    pub fn collapse_or_parent(&mut self) {
        if self.visible_node_mut(self.selected).is_some_and(|n| n.expanded) {
            let Some(node) = self.visible_node_mut(self.selected) else {
                return;
            };
            node.expanded = false;
            self.clamp_selection();
            return;
        }
        if let Some(parent) = self.parent_index(self.selected) {
            self.selected = parent;
        }
    }

    /// Index of the selected row's parent in the visible preorder (None for
    /// root-level rows).
    fn parent_index(&self, selected: usize) -> Option<usize> {
        let visible = flatten(&self.tree, 0);
        let (depth, _) = *visible.get(selected)?;
        if depth == 0 {
            return None;
        }
        visible[..selected]
            .iter()
            .rposition(|(d, _)| *d == depth - 1)
    }

    /// Expand (→) or descend: an expanded dir returns false (selection
    /// moves to its first child by the caller via move_selection(1)).
    pub fn expand_or_descend(&mut self) -> bool {
        let Some(node) = self.visible_node_mut(self.selected) else {
            return false;
        };
        if !node.children.is_empty() && !node.expanded {
            node.expanded = true;
            self.clamp_selection();
            true
        } else {
            false
        }
    }

    /// Activate the selected row: files yield their URI (the caller opens
    /// it); directories toggle expand (returning None).
    pub fn activate(&mut self) -> Option<String> {
        let (uri, toggled) = {
            let node = self.visible_node_mut(self.selected)?;
            if node.uri.is_some() {
                (node.uri.clone(), false)
            } else {
                node.expanded = !node.expanded;
                (None, true)
            }
        };
        if toggled {
            self.clamp_selection();
        }
        uri
    }

    /// Keep `selected` within the visible rows after an expand/collapse.
    fn clamp_selection(&mut self) {
        let len = self.visible_len();
        if len > 0 {
            self.selected = self.selected.min(len - 1);
        }
    }

    /// Window top so the selected row is visible (v1: naive bottom-anchored
    /// jump-scroll — the selection stays at the window's last row when the
    /// tree overflows the pane height; pure, no scroll state, review P3-3).
    pub fn visible_window(&self, height: usize) -> usize {
        if height == 0 {
            return 0;
        }
        let sel = self.selected;
        if sel >= height {
            sel + 1 - height
        } else {
            0
        }
    }

    /// Rebuild the events section counts from the event log (kind strings).
    pub fn refresh_events(&mut self, events_log: &[String]) {
        let mut counts: std::collections::BTreeMap<&str, usize> =
            std::collections::BTreeMap::new();
        for kind in events_log {
            *counts.entry(kind.as_str()).or_default() += 1;
        }
        let mut v: Vec<(String, usize)> = counts
            .into_iter()
            .map(|(k, c)| (k.to_string(), c))
            .collect();
        v.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        v.truncate(10);
        self.events = v;
    }

    /// Reload the session list from `{cwd}/.gsa/runs/` (read-only scan).
    pub fn reload_sessions(&mut self, cwd: &Path) {
        self.sessions = discover_sessions(cwd);
        self.session_selected = 0;
    }

    pub fn session_selection_up(&mut self) {
        if self.session_selected > 0 {
            self.session_selected -= 1;
        }
    }

    pub fn session_selection_down(&mut self) {
        if self.session_selected + 1 < self.sessions.len() {
            self.session_selected += 1;
        }
    }

    pub fn selected_session(&self) -> Option<&SessionEntry> {
        self.sessions.get(self.session_selected)
    }
}

/// Recursive directory load (depth-bounded).
fn load_dir(dir: &Path, depth: usize) -> Vec<TreeNode> {
    if depth >= 2 {
        return Vec::new();
    }
    let Ok(rd) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut entries: Vec<(bool, String, PathBuf)> = Vec::new();
    for e in rd.flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        // Evidence dirs are internal (the TUI already surfaces the journal);
        // skip them at every level so the tree stays navigable.
        if name == ".gsa" || name == ".git" {
            continue;
        }
        let is_dir = e.file_type().map(|t| t.is_dir()).unwrap_or(false);
        entries.push((is_dir, name, e.path()));
    }
    // Dirs first, then by name (byte order — deterministic).
    entries.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
    entries
        .into_iter()
        .map(|(is_dir, name, path)| {
            if is_dir {
                TreeNode {
                    label: name,
                    children: load_dir(&path, depth + 1),
                    expanded: false,
                    uri: None,
                }
            } else {
                TreeNode {
                    label: name,
                    children: Vec::new(),
                    expanded: false,
                    uri: Some(path.display().to_string()),
                }
            }
        })
        .collect()
}

fn visible_count(nodes: &[TreeNode]) -> usize {
    nodes
        .iter()
        .map(|n| 1 + if n.expanded { visible_count(&n.children) } else { 0 })
        .sum()
}

/// Preorder flatten with depths — the pane's selection model, shared with
/// the renderer (widgets.rs) and the parent lookup.
pub(crate) fn flatten(nodes: &[TreeNode], depth: usize) -> Vec<(usize, &TreeNode)> {
    let mut out = Vec::new();
    for n in nodes {
        out.push((depth, n));
        if n.expanded {
            out.extend(flatten(&n.children, depth + 1));
        }
    }
    out
}

fn walk_mut<'a>(nodes: &'a mut [TreeNode], remaining: &mut usize) -> Option<&'a mut TreeNode> {
    for node in nodes {
        if *remaining == 0 {
            return Some(node);
        }
        *remaining -= 1;
        if node.expanded && let Some(found) = walk_mut(&mut node.children, remaining) {
            return Some(found);
        }
    }
    None
}

/// Discover run journals under `{cwd}/.gsa/runs/` — newest first by dir
/// name (Python parity: `sorted(iterdir(), reverse=True)`; `RUN-{s}-{n}`
/// names sort in sequence order). Qualifies on `events.jsonl` presence and
/// peeks the first prompt (up to 10 lines) for the preview column.
pub fn discover_sessions(cwd: &Path) -> Vec<SessionEntry> {
    let runs = cwd.join(".gsa").join("runs");
    let Ok(rd) = std::fs::read_dir(&runs) else {
        return Vec::new();
    };
    let mut entries: Vec<SessionEntry> = Vec::new();
    for e in rd.flatten() {
        let dir = e.path();
        let Some(name) = dir.file_name().map(|n| n.to_string_lossy().into_owned()) else {
            continue;
        };
        let Some(rest) = name.strip_prefix("RUN-") else {
            continue;
        };
        let Some((_sid, n_str)) = rest.rsplit_once('-') else {
            continue;
        };
        let Ok(n) = n_str.parse::<u32>() else {
            continue;
        };
        let events_path = dir.join("events.jsonl");
        if !events_path.is_file() {
            continue;
        }
        let date = std::fs::metadata(&dir)
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(format_date)
            .unwrap_or_default();
        let preview = crate::journal_tail::peek_first_prompt(&events_path)
            .unwrap_or_default();
        entries.push(SessionEntry {
            run_dir: dir,
            session_id: name,
            prompt_preview: preview,
            turn_count: n + 1,
            date,
        });
    }
    // Newest first: sessions by id desc, runs within a session by turn
    // index NUMERICALLY desc (review D1-1 — lexicographic name sort breaks
    // past turn 9: "-10" < "-2").
    entries.sort_by(|a, b| {
        let (sid_a, n_a) = run_parts(&a.session_id);
        let (sid_b, n_b) = run_parts(&b.session_id);
        sid_b.cmp(&sid_a).then_with(|| n_b.cmp(&n_a))
    });
    entries
}

/// `RUN-{session8}-{n}` → (session part, numeric turn index).
fn run_parts(id: &str) -> (String, u32) {
    id.strip_prefix("RUN-")
        .and_then(|r| r.rsplit_once('-'))
        .map(|(sid, n)| (sid.to_string(), n.parse().unwrap_or(0)))
        .unwrap_or_else(|| (id.to_string(), 0))
}

/// `YYYY-MM-DD` from a file mtime (chrono is already a workspace dep).
fn format_date(t: std::time::SystemTime) -> Option<String> {
    let dt: chrono::DateTime<chrono::Utc> = t.into();
    Some(dt.format("%Y-%m-%d").to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "orz-tui-explorer-{tag}-{}-{}",
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

    #[test]
    fn cwd_scan_dirs_first_skips_gsa_depth_limited() {
        let dir = temp_dir("scan");
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::create_dir_all(dir.join("src").join("deep")).unwrap();
        std::fs::create_dir_all(dir.join("src").join("deep").join("deeper")).unwrap();
        std::fs::create_dir_all(dir.join(".gsa")).unwrap();
        std::fs::create_dir_all(dir.join(".git")).unwrap();
        std::fs::write(dir.join("b.txt"), "b").unwrap();
        std::fs::write(dir.join("a.txt"), "a").unwrap();

        let mut pane = ExplorerPane::default();
        pane.load(&dir);
        assert!(pane.loaded);

        // Dirs first, then files (name order); .gsa/.git skipped.
        let labels: Vec<&str> = pane.tree.iter().map(|n| n.label.as_str()).collect();
        assert_eq!(labels, ["src", "a.txt", "b.txt"]);
        // Two levels below the root: src (level 1) → deep (level 2);
        // deeper (level 3) is pruned.
        assert!(pane.tree[0].children.iter().any(|c| c.label == "deep"));
        assert!(
            pane.tree[0].children[0].children.is_empty(),
            "level-3 entries pruned"
        );

        // Files carry a URI; dirs do not.
        assert!(pane.tree[1].uri.is_some());
        assert!(pane.tree[0].uri.is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn tree_nav_keys_move_selection_expand_collapse() {
        let dir = temp_dir("nav");
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::write(dir.join("src").join("lib.rs"), "// x").unwrap();
        std::fs::write(dir.join("readme.md"), "x").unwrap();

        let mut pane = ExplorerPane::default();
        pane.load(&dir);
        assert_eq!(pane.visible_len(), 2); // src, readme.md

        // Expand the dir (→): its child appears; selection stays.
        pane.selected = 0;
        assert!(pane.expand_or_descend());
        assert_eq!(pane.visible_len(), 3);
        assert!(pane.tree[0].expanded);

        // Move down over the child, then up again.
        pane.move_selection(1);
        assert_eq!(pane.selected, 1);
        pane.move_selection(-1);
        assert_eq!(pane.selected, 0);

        // Activate a file → URI; activate a dir → toggles expand.
        pane.selected = 1;
        let uri = pane.activate();
        assert!(uri.as_deref().unwrap().ends_with("lib.rs"));
        pane.selected = 0;
        assert!(pane.activate().is_none(), "dir activation yields no URI");
        assert!(!pane.tree[0].expanded, "dir toggled back to collapsed");

        // ← on a leaf selects the PARENT row (review P3-2), not the
        // previous sibling.
        pane.selected = 0;
        pane.expand_or_descend(); // expand src again
        pane.selected = 1; // on lib.rs (parent = src at index 0)
        pane.collapse_or_parent();
        assert_eq!(pane.selected, 0, "← moves to the parent row");
        assert!(pane.tree[0].expanded, "the parent itself stays expanded");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn selection_clamps_after_collapse() {
        let dir = temp_dir("clamp");
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::write(dir.join("src").join("a.rs"), "x").unwrap();
        std::fs::write(dir.join("src").join("b.rs"), "x").unwrap();

        let mut pane = ExplorerPane::default();
        pane.load(&dir);
        pane.selected = 0;
        pane.expand_or_descend();
        pane.move_selection(2); // now on b.rs (visible: src, a.rs, b.rs)
        assert_eq!(pane.selected, 2);
        // ← on the expanded dir collapses it; the children vanish and the
        // selection clamps into the remaining rows.
        pane.selected = 0;
        pane.collapse_or_parent();
        assert_eq!(pane.visible_len(), 1, "collapsed src hides children");
        assert_eq!(pane.selected, 0, "selection clamps into range");
        // Moving far past the end clamps to the last visible row.
        pane.move_selection(5);
        assert_eq!(pane.selected, 0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn discover_sessions_newest_first_and_prompt_peek() {
        let dir = temp_dir("discover");
        let runs = dir.join(".gsa").join("runs");
        std::fs::create_dir_all(&runs).unwrap();

        // Runs for session a1b2c3d4 (incl. turn 10 — the numeric-sort
        // regression, review D1-1); one for z9y8x7w6 (newer name).
        for (sid, _n) in [
            ("RUN-a1b2c3d4-0", 0u32),
            ("RUN-a1b2c3d4-1", 1),
            ("RUN-a1b2c3d4-10", 10),
            ("RUN-z9y8x7w6-0", 0),
        ] {
            let run_dir = runs.join(sid);
            std::fs::create_dir_all(&run_dir).unwrap();
            let ev = orz_assurance::journal::RunEvent::new(
                sid.to_string(),
                0,
                orz_assurance::journal::EventType::RunStarted,
                "m".into(),
                None,
                "run-event-v0.1.schema.json".into(),
                serde_json::json!({"prompt": format!("第 {sid} 个提示")}),
                orz_assurance::journal::Redaction::None,
                "2026-08-05T00:00:00Z".into(),
            );
            let mut ev = ev;
            orz_assurance::seal_event(&mut ev).unwrap();
            std::fs::write(
                run_dir.join("events.jsonl"),
                format!("{}\n", serde_json::to_string(&ev).unwrap()),
            )
            .unwrap();
        }
        // A dir without events.jsonl must not qualify.
        std::fs::create_dir_all(runs.join("RUN-nosuch00-0")).unwrap();

        let sessions = discover_sessions(&dir);
        // Newest first: z9y8x7w6-0 sorts above a1b2c3d4's runs; within the
        // session, turns sort NUMERICALLY — -10 above -1 above -0 (review
        // D1-1: lexicographic order would put -10 between -2 and -1).
        assert_eq!(sessions.len(), 4);
        assert!(sessions[0].session_id.starts_with("RUN-z9y8x7w6"));
        let a_ids: Vec<&str> = sessions
            .iter()
            .filter(|s| s.session_id.starts_with("RUN-a1b2c3d4"))
            .map(|s| s.session_id.as_str())
            .collect();
        assert_eq!(
            a_ids,
            ["RUN-a1b2c3d4-10", "RUN-a1b2c3d4-1", "RUN-a1b2c3d4-0"],
            "numeric turn order within a session"
        );
        // Turn count from the name suffix.
        let second = sessions
            .iter()
            .find(|s| s.session_id == "RUN-a1b2c3d4-1")
            .unwrap();
        assert_eq!(second.turn_count, 2);
        // Prompt preview peeked from the sealed journal.
        assert_eq!(second.prompt_preview, "第 RUN-a1b2c3d4-1 个提示");
        // Date derived from the dir mtime.
        assert!(!second.date.is_empty() && second.date.len() == 10);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn empty_runs_yields_empty_list() {
        let dir = temp_dir("empty");
        // No .gsa at all → empty.
        assert!(discover_sessions(&dir).is_empty());
        // .gsa/runs exists but empty → empty.
        std::fs::create_dir_all(dir.join(".gsa").join("runs")).unwrap();
        assert!(discover_sessions(&dir).is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn refresh_events_counts_and_sorts() {
        let mut pane = ExplorerPane::default();
        pane.refresh_events(&[
            "run_started".into(),
            "model_output".into(),
            "run_started".into(),
            "gate_decision".into(),
        ]);
        assert_eq!(
            pane.events,
            vec![
                ("run_started".to_string(), 2),
                ("gate_decision".to_string(), 1),
                ("model_output".to_string(), 1),
            ]
        );
    }

    #[test]
    fn session_nav_helpers() {
        let dir = temp_dir("selnav");
        std::fs::create_dir_all(dir.join(".gsa").join("runs")).unwrap();
        let mut pane = ExplorerPane::default();
        pane.reload_sessions(&dir);
        assert_eq!(pane.session_selected, 0);
        assert!(pane.selected_session().is_none());
        pane.session_selection_down(); // no-op on empty
        assert_eq!(pane.session_selected, 0);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
