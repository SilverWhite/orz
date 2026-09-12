//! Reclaim ladder — cache classification + delayed deletion
//! (FUS-HOST-RESOURCE-SAFETY §4.6 / §4.6.1, 0z S2).
//!
//! The recycle bin is **cancelled** (user final ruling 2026-09-12): a same-
//! volume bin frees no space, and the delayed window already covers the
//! recoverable value. What remains is one boundary — the **reclaim budget**:
//!
//! | class | verdict | action |
//! |---|---|---|
//! | `cache` | regenerable (build products, package caches, run scratch) | **delayed delete** — enters the pending set, deleted after the window expires with no failure feedback |
//! | `unknown` | unclassifiable or outside the whitelist | **rejected** (fail-closed; never guessed, never binned) |
//! | `evidence` | `.gsa` chain, journals, `.git`, user sources | **rejected** (never auto-reclaimed) |
//!
//! Ordering (§4.6): this run's scratch → this workspace's build/package
//! caches → (windowed) earlier runs' scratch. The ladder never deletes the
//! product surface of a **running** heavy action (§4.7.1 item 14): the
//! planning face takes the in-flight write targets and refuses anything under
//! them. Over-budget candidates shrink or the whole reclaim is rejected —
//! **no second confirmation is ever asked of the model** (§4.6.1 item 4).
//!
//! Audit-first (four disciplines, §4.6): the `reclaim_performed` fact —
//! `outcome ∈ pending_delete / permanent / rejected` — is produced BEFORE any
//! deletion touches the disk.

use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// The delayed-delete window in tool-call rounds (design §4.6.1 item 1:
/// default 2, expandable to 3 — never beyond).
pub const DEFAULT_WINDOW_ROUNDS: u32 = 2;
pub const MAX_WINDOW_ROUNDS: u32 = 3;

/// The single-reclaim budget (bytes): the one boundary the delayed window
/// keeps (§4.6.1 末段). `ORZ_RECLAIM_BUDGET_BYTES` overrides.
pub fn reclaim_budget_bytes() -> u64 {
    std::env::var("ORZ_RECLAIM_BUDGET_BYTES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(8 * 1024 * 1024 * 1024)
}

/// Reclaim classification (design §4.6 table).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReclaimClass {
    Cache,
    Unknown,
    Evidence,
}

impl ReclaimClass {
    pub fn as_str(&self) -> &'static str {
        match self {
            ReclaimClass::Cache => "cache",
            ReclaimClass::Unknown => "unknown",
            ReclaimClass::Evidence => "evidence",
        }
    }
}

/// Audit outcome (design §4.6 回报 / §4.6.1: no `trash` state exists).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReclaimOutcome {
    PendingDelete,
    Permanent,
    Rejected,
}

impl ReclaimOutcome {
    pub fn as_str(&self) -> &'static str {
        match self {
            ReclaimOutcome::PendingDelete => "pending_delete",
            ReclaimOutcome::Permanent => "permanent",
            ReclaimOutcome::Rejected => "rejected",
        }
    }
}

/// Classify one candidate path against the whitelist root. Pure.
///
/// Evidence surfaces (`.gsa`, `.git`) are refused even when they sit under a
/// cache-ish name; cache membership is decided by well-known regenerable
/// directory names (`target`, `node_modules`, `__pycache__`, `dist`, scratch
/// `tmp*`); everything else is `unknown` — fail-closed.
pub fn classify_path(workspace_root: &Path, path: &Path) -> ReclaimClass {
    // The path must live INSIDE the whitelist root (never the root itself,
    // never a bare volume / filesystem root — OPS discipline).
    let Ok(root) = dunce::canonicalize(workspace_root) else {
        return ReclaimClass::Unknown;
    };
    let Ok(candidate) = dunce::canonicalize(path) else {
        return ReclaimClass::Unknown;
    };
    if candidate == root {
        return ReclaimClass::Unknown;
    }
    if !candidate.starts_with(&root) {
        return ReclaimClass::Unknown;
    }
    let rel = match candidate.strip_prefix(&root) {
        Ok(rel) => rel,
        Err(_) => return ReclaimClass::Unknown,
    };
    let components: Vec<String> = rel
        .components()
        .filter_map(|c| c.as_os_str().to_str())
        .map(|s| s.to_ascii_lowercase())
        .collect();
    if components.is_empty() {
        return ReclaimClass::Unknown;
    }
    // Evidence faces are absolute refusals, wherever they sit.
    if components.iter().any(|c| c == ".gsa" || c == ".git") {
        return ReclaimClass::Evidence;
    }
    // Regenerable faces: build products, package caches, run scratch.
    if components.iter().any(|c| {
        matches!(
            c.as_str(),
            "target" | "node_modules" | "__pycache__" | ".pytest_cache" | "dist"
        ) || c == "incremental"
            || c.starts_with("tmp")
            || c.starts_with(".tmp")
    }) {
        return ReclaimClass::Cache;
    }
    ReclaimClass::Unknown
}

/// One candidate the planner may act on.
#[derive(Clone, Debug)]
pub struct ReclaimCandidate {
    pub path: PathBuf,
    pub size_bytes: u64,
}

/// One pending deletion in the delayed window.
#[derive(Clone, Debug)]
pub struct PendingDeletion {
    pub path: PathBuf,
    pub size_bytes: u64,
    /// The tool-call round the candidate was enqueued (window counting).
    pub enqueued_round: u32,
}

/// A planner decision for one candidate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReclaimDecision {
    /// Enter the pending set (soft tier) — freed bytes count as 0.
    Pending,
    /// Delete now (reclaim-direct / hard tier, or a window expiry).
    Permanent,
    /// Refuse, with the machine reason carried to the audit face.
    Reject(RejectReason),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RejectReason {
    /// Not a `cache` class path — `unknown` refuses, evidence refuses.
    NotCache,
    /// Inside a running heavy action's write target (§4.7.1 item 14).
    InFlightSurface,
    /// The candidate exceeds the whole single-reclaim budget on its own.
    OverBudget,
}

/// One `reclaim_performed` row for the journal face.
#[derive(Clone, Debug)]
pub struct ReclaimFact {
    pub class: &'static str,
    pub outcome: &'static str,
    pub tier: String,
    pub paths: Vec<String>,
    pub freed_bytes: u64,
    pub window_rounds: Option<u32>,
    pub budget_bytes: Option<u64>,
}

/// The reclaim ladder state (one per host).
pub struct ReclaimLadder {
    workspace_root: PathBuf,
    pending: Mutex<Vec<PendingDeletion>>,
    window_rounds: u32,
    budget_bytes: u64,
    facts: Mutex<Vec<ReclaimFact>>,
}

impl ReclaimLadder {
    pub fn new(workspace_root: &Path) -> Self {
        Self {
            workspace_root: workspace_root.to_path_buf(),
            pending: Mutex::new(Vec::new()),
            window_rounds: DEFAULT_WINDOW_ROUNDS,
            budget_bytes: reclaim_budget_bytes(),
            facts: Mutex::new(Vec::new()),
        }
    }

    /// The configured window (clamped to the 3-round ceiling).
    pub fn window_rounds(&self) -> u32 {
        self.window_rounds
    }

    pub fn budget_bytes(&self) -> u64 {
        self.budget_bytes
    }

    /// Plan one reclaim pass — PURE decisions, no side effects.
    ///
    /// `direct` = reclaim-direct/hard tier: skip the window (survival first,
    /// §4.6.1 item 6). `current_round` drives window counting. Soft tier:
    /// cache candidates enter the pending set (0 bytes freed this pass).
    pub fn plan(
        &self,
        candidates: &[ReclaimCandidate],
        direct: bool,
        current_round: u32,
        in_flight_write_targets: &[PathBuf],
    ) -> Vec<(ReclaimCandidate, ReclaimDecision)> {
        let mut decisions = Vec::new();
        // Budget accounting starts from what the window already owes.
        let mut remaining = self.budget_bytes.saturating_sub(
            self.pending
                .lock()
                .unwrap()
                .iter()
                .map(|p| p.size_bytes)
                .sum(),
        );
        for candidate in candidates {
            if classify_path(&self.workspace_root, &candidate.path) != ReclaimClass::Cache {
                decisions.push((
                    candidate.clone(),
                    ReclaimDecision::Reject(RejectReason::NotCache),
                ));
                continue;
            }
            // §4.7.1 item 14: never touch a running heavy action's surface.
            let canonical =
                dunce::canonicalize(&candidate.path).unwrap_or_else(|_| candidate.path.clone());
            if in_flight_write_targets
                .iter()
                .any(|target| starts_canonical(target, &canonical))
            {
                decisions.push((
                    candidate.clone(),
                    ReclaimDecision::Reject(RejectReason::InFlightSurface),
                ));
                continue;
            }
            if candidate.size_bytes > remaining {
                // Over-budget: shrink or reject, never ask (§4.6.1 item 4).
                decisions.push((
                    candidate.clone(),
                    ReclaimDecision::Reject(RejectReason::OverBudget),
                ));
                continue;
            }
            remaining = remaining.saturating_sub(candidate.size_bytes);
            if direct {
                decisions.push((candidate.clone(), ReclaimDecision::Permanent));
            } else {
                decisions.push((candidate.clone(), ReclaimDecision::Pending));
                self.pending.lock().unwrap().push(PendingDeletion {
                    path: candidate.path.clone(),
                    size_bytes: candidate.size_bytes,
                    enqueued_round: current_round,
                });
            }
        }
        decisions
    }

    /// Expire the pending set: rows whose window has passed become
    /// `Permanent` actions. Pure: returns the paths to delete now.
    pub fn expire_window(&self, current_round: u32) -> Vec<PendingDeletion> {
        let mut pending = self.pending.lock().unwrap();
        let (due, keep): (Vec<_>, Vec<_>) = pending
            .drain(..)
            .partition(|p| current_round.saturating_sub(p.enqueued_round) >= self.window_rounds);
        *pending = keep;
        due
    }

    /// Execute a planned pass. `delete_now` faces (Permanent / window expiry)
    /// are removed AFTER the facts for this pass were produced — audit-first
    /// by construction. Returns the freed byte count.
    pub fn execute(
        &self,
        tier: &str,
        decisions: &[(ReclaimCandidate, ReclaimDecision)],
        delete: &dyn Fn(&Path) -> io::Result<()>,
    ) -> u64 {
        let mut freed = 0u64;
        let mut permanent: Vec<&ReclaimCandidate> = Vec::new();
        let mut pending_rows: Vec<&ReclaimCandidate> = Vec::new();
        let mut rejected: Vec<(&ReclaimCandidate, RejectReason)> = Vec::new();
        for (candidate, decision) in decisions {
            match decision {
                ReclaimDecision::Permanent => permanent.push(candidate),
                ReclaimDecision::Pending => pending_rows.push(candidate),
                ReclaimDecision::Reject(reason) => rejected.push((candidate, *reason)),
            }
        }

        // ---- facts BEFORE any deletion ----
        if !pending_rows.is_empty() {
            self.push_fact(ReclaimFact {
                class: "cache",
                outcome: "pending_delete",
                tier: tier.to_string(),
                paths: pending_rows
                    .iter()
                    .map(|c| c.path.display().to_string())
                    .collect(),
                freed_bytes: 0,
                window_rounds: Some(self.window_rounds),
                budget_bytes: Some(self.budget_bytes),
            });
        }
        if !permanent.is_empty() {
            self.push_fact(ReclaimFact {
                class: "cache",
                outcome: "permanent",
                tier: tier.to_string(),
                paths: permanent
                    .iter()
                    .map(|c| c.path.display().to_string())
                    .collect(),
                freed_bytes: permanent.iter().map(|c| c.size_bytes).sum(),
                window_rounds: None,
                budget_bytes: Some(self.budget_bytes),
            });
        }
        for (candidate, reason) in &rejected {
            let _ = reason; // the reason class travels on the decision; the
            // journal face keeps the outcome tri-state (audit detail face
            // carries specifics).
            self.push_fact(ReclaimFact {
                class: "cache",
                outcome: "rejected",
                tier: tier.to_string(),
                paths: vec![candidate.path.display().to_string()],
                freed_bytes: 0,
                window_rounds: None,
                budget_bytes: Some(self.budget_bytes),
            });
        }

        // ---- deletion only after the facts ----
        for candidate in permanent {
            if delete(&candidate.path).is_ok() {
                freed += candidate.size_bytes;
            }
        }
        freed
    }

    /// Delete the window-expired rows (called once per round from the trigger
    /// seam). Audit-first: the `permanent` fact lands before the removals.
    pub fn execute_expiry(
        &self,
        current_round: u32,
        delete: &dyn Fn(&Path) -> io::Result<()>,
    ) -> u64 {
        let due = self.expire_window(current_round);
        if due.is_empty() {
            return 0;
        }
        let freed_total: u64 = due.iter().map(|p| p.size_bytes).sum();
        self.push_fact(ReclaimFact {
            class: "cache",
            outcome: "permanent",
            tier: "soft".to_string(),
            paths: due.iter().map(|p| p.path.display().to_string()).collect(),
            freed_bytes: freed_total,
            window_rounds: None,
            budget_bytes: Some(self.budget_bytes),
        });
        let mut freed = 0u64;
        for p in &due {
            if delete(&p.path).is_ok() {
                freed += p.size_bytes;
            }
        }
        freed
    }

    /// Drain the accumulated facts for the journal face.
    pub fn drain_facts(&self) -> Vec<ReclaimFact> {
        std::mem::take(&mut *self.facts.lock().unwrap())
    }

    /// Current fact count without draining (the audit-first probe).
    pub fn facts_len_snapshot(&self) -> usize {
        self.facts.lock().unwrap().len()
    }

    fn push_fact(&self, fact: ReclaimFact) {
        self.facts.lock().unwrap().push(fact);
    }
}

/// True when `candidate` is `target` itself or lives under it (both sides
/// pre-canonicalized where possible).
fn starts_canonical(target: &Path, candidate: &Path) -> bool {
    let target = dunce::canonicalize(target).unwrap_or_else(|_| target.to_path_buf());
    candidate.starts_with(&target)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch_root(tag: &str) -> PathBuf {
        let n = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir =
            std::env::temp_dir().join(format!("orz-reclaim-{tag}-{}-{n}", std::process::id()));
        std::fs::create_dir_all(dir.join("target").join("debug").join("incremental")).unwrap();
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::create_dir_all(dir.join(".gsa").join("runs")).unwrap();
        std::fs::create_dir_all(dir.join("target").join("in-flight")).unwrap();
        std::fs::create_dir_all(dir.join("tmp_rebuild_scratch")).unwrap();
        std::fs::create_dir_all(dir.join("node_modules")).unwrap();
        std::fs::create_dir_all(dir.join(".git")).unwrap();
        dir
    }

    fn write_file(dir: &Path, rel: &str, bytes: usize) -> PathBuf {
        let path = dir.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, vec![b'x'; bytes]).unwrap();
        path
    }

    fn dir_size(path: &Path) -> u64 {
        let mut total = 0;
        if let Ok(entries) = std::fs::read_dir(path) {
            for e in entries.flatten() {
                if e.path().is_dir() {
                    total += dir_size(&e.path());
                } else {
                    total += e.metadata().map(|m| m.len()).unwrap_or(0);
                }
            }
        }
        total
    }

    /// 分类矩阵：cache 命中 / unknown 拒绝 / evidence 绝对拒绝 / 白名单外拒绝。
    #[test]
    fn classification_matrix_matches_the_final_ruling() {
        let root = scratch_root("classify");
        assert_eq!(
            classify_path(
                &root,
                &root.join("target").join("debug").join("incremental")
            ),
            ReclaimClass::Cache
        );
        assert_eq!(
            classify_path(&root, &root.join("node_modules")),
            ReclaimClass::Cache
        );
        assert_eq!(
            classify_path(&root, &root.join("tmp_rebuild_scratch")),
            ReclaimClass::Cache
        );
        // 用户源码：unknown → 拒绝。
        assert_eq!(
            classify_path(&root, &root.join("src")),
            ReclaimClass::Unknown
        );
        assert_eq!(
            classify_path(&root, &root.join("src").join("main.rs")),
            ReclaimClass::Unknown
        );
        // 证据面：绝对拒绝（即便名字像 cache）。
        assert_eq!(
            classify_path(&root, &root.join(".gsa").join("runs")),
            ReclaimClass::Evidence
        );
        assert_eq!(
            classify_path(&root, &root.join(".git")),
            ReclaimClass::Evidence
        );
        // 白名单外 / 盘根本身：unknown。
        assert_eq!(
            classify_path(&root, &std::env::temp_dir()),
            ReclaimClass::Unknown
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    /// 判据 11：窗口语义（入待删集合 → 过窗才真删，窗内可回退）+ 超预算
    /// 缩减/拒绝不询问 + 在跑重活产物面保护。
    #[test]
    fn window_budget_and_inflight_matrix() {
        let root = scratch_root("matrix");
        let ladder = ReclaimLadder::new(&root);
        let incr = root.join("target").join("debug").join("incremental");
        let in_flight = root.join("target").join("in-flight");
        let size = dir_size(&incr);

        // 在跑重活的产物面（§4.7.1 第 14 条）：拒绝，不删。
        let decisions = ladder.plan(
            &[
                ReclaimCandidate {
                    path: incr.clone(),
                    size_bytes: size,
                },
                ReclaimCandidate {
                    path: in_flight.clone(),
                    size_bytes: 10,
                },
            ],
            false,
            1,
            &[in_flight.clone()],
        );
        assert!(matches!(decisions[0].1, ReclaimDecision::Pending));
        assert!(
            matches!(
                decisions[1].1,
                ReclaimDecision::Reject(RejectReason::InFlightSurface)
            ),
            "an in-flight heavy action's product surface must be refused"
        );

        // 窗口语义：round 1 入队；round 2 未到期；round 3 到期真删。
        assert!(ladder.expire_window(1).is_empty(), "round 1: nothing due");
        assert!(
            ladder.expire_window(2).is_empty(),
            "round 2: window (2) not expired"
        );
        let due = ladder.expire_window(3);
        assert_eq!(due.len(), 1, "round 3: the row is due");
        // 再入队后窗内回退 = 不删（expire 不含它）。
        let _ = ladder.plan(
            &[ReclaimCandidate {
                path: incr.clone(),
                size_bytes: size,
            }],
            false,
            4,
            &[],
        );
        assert!(
            ladder.expire_window(5).is_empty(),
            "round 4+2>5? no: 4+2=6 > 5 → not due"
        );

        // 超预算拒绝：单候选超过整笔预算 → rejected，不删、不询问。
        let fat = ReclaimCandidate {
            path: incr.clone(),
            size_bytes: ladder.budget_bytes() + 1,
        };
        let decisions = ladder.plan(&[fat], true, 6, &[]);
        assert!(matches!(
            decisions[0].1,
            ReclaimDecision::Reject(RejectReason::OverBudget)
        ));

        // reclaim-direct（hard 档）：跳过窗口直接删。
        let decisions = ladder.plan(
            &[ReclaimCandidate {
                path: incr.clone(),
                size_bytes: size,
            }],
            true,
            7,
            &[],
        );
        assert!(matches!(decisions[0].1, ReclaimDecision::Permanent));
        let freed = ladder.execute("hard", &decisions, &|p| {
            std::fs::remove_dir_all(p).map_err(io::Error::from)
        });
        assert_eq!(freed, size, "direct reclaim frees the full size");

        let _ = std::fs::remove_dir_all(&root);
    }

    /// 审计先行：facts（pending_delete / permanent / rejected）在删除前落。
    #[test]
    fn facts_land_before_any_deletion() {
        let root = scratch_root("audit");
        let ladder = ReclaimLadder::new(&root);
        let incr = root.join("target").join("debug").join("incremental");
        let size = dir_size(&incr);

        // 删除闭包先抓取 facts 计数：删除发生时 permanent fact 必须已存在。
        let snapshot_at_delete = std::sync::Arc::new(std::sync::Mutex::new(None::<usize>));
        let capture = snapshot_at_delete.clone();
        let candidates = vec![ReclaimCandidate {
            path: incr.clone(),
            size_bytes: size,
        }];
        let decisions = ladder.plan(&candidates, true, 1, &[]);
        let freed = ladder.execute("hard", &decisions, &|p| {
            *capture.lock().unwrap() = Some(ladder.facts_len_snapshot());
            std::fs::remove_dir_all(p).map_err(io::Error::from)
        });
        assert_eq!(freed, size);
        let snapshot = snapshot_at_delete.lock().unwrap().expect("delete happened");
        assert!(
            snapshot >= 1,
            "the permanent fact must exist BEFORE the deletion ran"
        );

        // 全部 facts 可 drain（permanent 一行，0 字节谎报不可能：freed= size）。
        let facts = ladder.drain_facts();
        assert_eq!(facts.len(), snapshot);
        assert_eq!(facts[0].outcome, "permanent");
        assert_eq!(facts[0].class, "cache");
        assert_eq!(facts[0].freed_bytes, size);

        let _ = std::fs::remove_dir_all(&root);
    }
}
