//! Process-tree lifecycle — registration + orphan sweep
//! (FUS-HOST-RESOURCE-SAFETY §4.2 items 3-4 / §4.8 表 2, 0z S2).
//!
//! Every tool child flows through `ProcessGroup::attach_pid`, which delivers a
//! [`xai_tty_utils::SpawnObservation`] to the ambient sink the host installs
//! around each call ([`OrzHost::call_tool_inner`]). The sink turns the
//! observation into a persistent record under `.gsa/process_trees/` — the
//! registry the sweep works from.
//!
//! **Sweep** (three conditions, all three must hold — design §4.2 item 4;
//! 归属不明一律不杀):
//! ① parent chain dead — every recorded ancestor pid is gone from the host;
//! ② fingerprint match — the candidate process's *current* normalized image
//!    hash equals the registered one (the pid-reuse guard: a recycled pid now
//!    running a different executable is refused, never killed);
//! ③ window — the record started before this run began (cross-run orphan) or,
//!    at finalize, belongs to this run and is still alive.
//!
//! **Hardenings** (§4.8 表 2): (a) only registered pids are ever candidates —
//! no registry row, no kill; (b) the fingerprint is the normalized image path
//! (canonicalized + lowercased) — stable across query sites; (c) audit before
//! action: the planned row is persisted (and handed to the journal face)
//! before the first `TerminateProcess`/`kill` fires.

use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// One registered tool-child tree. File shape of
/// `.gsa/process_trees/<call_id>-<pid>.json` (per-call keyed, per-child
/// disambiguated — one call may attach more than one child).
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ProcessTreeRecord {
    pub call_id: String,
    pub pid: u32,
    /// Ancestor pids at registration time (the spawning orz process first).
    /// The sweep requires the whole chain dead — pid reuse can only make a
    /// parent look *alive*, which refuses the kill (fail-safe direction).
    #[serde(default)]
    pub parent_chain: Vec<u32>,
    /// sha256 over the normalized image path at registration.
    #[serde(default)]
    pub image_sha256: Option<String>,
    /// Unix milliseconds.
    pub started_at: u64,
    /// The run that spawned the child (registry rows survive the run).
    #[serde(default)]
    pub run_id: String,
    /// Root `run` + per-call granularity marker.
    #[serde(default)]
    pub job_name: String,
    // 0aw（2026-09-20 裁决 ④）：原 `action_class` 字段随动作分类器一并退役
    // ——它只服务 hard 档树杀的「重档 call_id 集」枚举，该面已不存在。
    // 历史登记行携带的该键经 `serde(default)` + 宽松反序列化自然兼容：
    // 旧文件多出的键在默认 serde 语义下被忽略，扫除三条件不依赖它。
}

/// Why a sweep row was refused — the negative side of the three-condition
/// matrix (判据 5: 不满足三条件的进程不被杀).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SweepRefusal {
    /// The pid is not alive any more (nothing to reap).
    NotRunning,
    /// A recorded ancestor pid is still alive — the tree's owner may still be
    /// working (also covers this run's own in-flight children; §4.7.1 第 14 条
    /// protects in-flight product surfaces).
    ParentAlive,
    /// The candidate's current image hash differs from the registered one —
    /// pid reuse or identity drift; never kill on a mismatch.
    FingerprintMismatch,
    /// No fingerprint available on either side — 归属不明.
    FingerprintUnknown,
    /// The candidate's process-creation time drifted from the registered
    /// `started_at` beyond tolerance — same-image pid reuse (review F-BE-5).
    CreationTimeMismatch,
    /// The record belongs to another run (finalize sweep scope guard,
    /// review F-BE-1) — never touched by this run.
    ForeignRun,
    /// The record's started_at does not place it in the sweep's window.
    OutsideWindow,
}

/// Which sweep is being planned (see [`ProcessTreeRegistry::plan_sweep`]).
#[derive(Clone, Copy, Debug)]
pub enum SweepMode<'a> {
    /// Assembly-time orphan sweep over previous-run rows.
    Start { run_started_at: u64 },
    /// Shutdown sweep over THIS run's rows only.
    Finalize { own_run_id: &'a str, own_pid: u32 },
}

/// Creation-time tolerance for the pid-reuse guard (review F-BE-5): the
/// registry stores the attach-time clock; the candidate's kernel creation
/// time may drift by scheduling/clock granularity, so anything within this
/// window counts as the same process. 5 s is far below any realistic
/// pid-recycle horizon and far above clock jitter.
pub const CREATION_TIME_TOLERANCE_MS: u64 = 5_000;

/// The sweep's decision for one registry record.
#[derive(Clone, Debug)]
pub enum SweepDecision {
    Reap(ProcessTreeRecord),
    Refuse(ProcessTreeRecord, SweepRefusal),
}

/// Sweep facts for the journal face (`process_tree_reaped`).
#[derive(Clone, Debug)]
pub struct SweepFact {
    /// `parent_abort` (cross-run orphan swept at start) or `run_shutdown`
    /// (this run's own leftovers, reaped at finalize).
    pub reason: &'static str,
    /// planned = the audit-first row (before any kill); executed = after.
    pub phase: &'static str,
    pub pids: Vec<u32>,
    pub call_ids: Vec<String>,
}

/// One audit-row target (plain data — decoupled from live records).
#[derive(Clone, Debug)]
struct AuditTarget {
    call_id: String,
    pid: u32,
    image_sha256: Option<String>,
    started_at: u64,
    /// Machine key of the refusal (`""` on planned/executed rows).
    refusal: String,
}

fn refusal_key(why: &SweepRefusal) -> &'static str {
    match why {
        SweepRefusal::NotRunning => "not_running",
        SweepRefusal::ParentAlive => "parent_alive",
        SweepRefusal::FingerprintMismatch => "fingerprint_mismatch",
        SweepRefusal::FingerprintUnknown => "fingerprint_unknown",
        SweepRefusal::CreationTimeMismatch => "creation_time_mismatch",
        SweepRefusal::ForeignRun => "foreign_run",
        SweepRefusal::OutsideWindow => "outside_window",
    }
}

/// Reap targets carry an explicit `"reap"` label instead of a borrowed
/// refusal key (review F-BE-12: `"not_running"` on planned/executed rows
/// was a false observation-face label).
fn audit_targets_reap(records: &[&ProcessTreeRecord]) -> Vec<AuditTarget> {
    records
        .iter()
        .map(|r| AuditTarget {
            call_id: r.call_id.clone(),
            pid: r.pid,
            image_sha256: r.image_sha256.clone(),
            started_at: r.started_at,
            refusal: "reap".to_string(),
        })
        .collect()
}

/// The persistent registry under `.gsa/process_trees/`.
pub struct ProcessTreeRegistry {
    dir: PathBuf,
    /// Sweep facts accumulated since the last drain (loop journal face).
    pending_facts: Mutex<Vec<SweepFact>>,
}

impl ProcessTreeRegistry {
    /// The registry for a workspace: `{cwd}/.gsa/process_trees/`.
    pub fn new(cwd: &Path) -> Self {
        Self {
            dir: cwd.join(".gsa").join("process_trees"),
            pending_facts: Mutex::new(Vec::new()),
        }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Persist one registration row. Best-effort at the call site — a failed
    /// registration only means the sweep will never consider this child
    /// (hardening (a): no record, no kill), never blocks the spawn.
    pub fn register(&self, record: &ProcessTreeRecord) -> io::Result<()> {
        write_record(&self.dir, record)
    }

    fn record_path(&self, call_id: &str, pid: u32) -> PathBuf {
        // Filesystem-safe: call ids are tool-call identifiers; strip anything
        // that could escape the directory.
        let safe: String = call_id
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        self.dir.join(format!("{safe}-{pid}.json"))
    }

    /// Load every registration row currently on disk.
    pub fn load_all(&self) -> Vec<ProcessTreeRecord> {
        let mut records = Vec::new();
        let Ok(entries) = std::fs::read_dir(&self.dir) else {
            return records;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            if let Ok(text) = std::fs::read_to_string(&path)
                && let Ok(record) = serde_json::from_str::<ProcessTreeRecord>(&text)
            {
                records.push(record);
            }
        }
        records
    }

    /// Drop one record (after a successful reap, or when the call completes
    /// cleanly and the tree is gone).
    pub fn remove(&self, call_id: &str, pid: u32) {
        let _ = std::fs::remove_file(self.record_path(call_id, pid));
    }

    /// Plan the sweep for `records` — PURE decision matrix, no side effects
    /// (the testability seam).
    ///
    /// `mode` selects the sweep:
    /// - [`SweepMode::Start`] (assembly): window = records older than
    ///   `run_started_at`; the full three conditions hold for every row — a
    ///   parallel instance's live children are protected by their parent
    ///   chain (its orz pid is alive → ParentAlive).
    /// - [`SweepMode::Finalize`] (shutdown): ONLY rows registered by this run
    ///   (`run_id == own_run_id`) are candidates; foreign rows are skipped
    ///   untouched — review F-BE-1 (2026-09-13): the old signature let the
    ///   finalize sweep reap a *parallel instance's* in-flight children
    ///   (shared registry dir + fingerprint matches by construction + parent
    ///   check skipped). Within own rows the parent check exempts only our
    ///   own pid (review F-BE-1); any other live ancestor still refuses.
    ///
    /// Hardening (review F-BE-5): the image fingerprint alone cannot see pid
    /// reuse by the *same* executable, so the sweep additionally compares the
    /// candidate's process-creation time against the registered `started_at`
    /// (tolerance [`CREATION_TIME_TOLERANCE_MS`]) whenever both sides are
    /// available; a mismatch refuses the kill.
    pub fn plan_sweep(
        &self,
        records: &[ProcessTreeRecord],
        mode: SweepMode<'_>,
        alive_of: &dyn Fn(u32) -> bool,
        image_hash_of: &dyn Fn(u32) -> Option<String>,
        creation_time_of: &dyn Fn(u32) -> Option<u64>,
    ) -> Vec<SweepDecision> {
        let mut decisions = Vec::new();
        'records: for record in records {
            // ① Start sweep: the full parent chain must be dead — this is the
            // in-flight protection (§4.7.1 第 14 条): a parallel instance's
            // children carry its live orz pid and are refused here.
            if let SweepMode::Start { .. } = mode
                && record.parent_chain.iter().any(|pid| alive_of(*pid))
            {
                decisions.push(SweepDecision::Refuse(
                    record.clone(),
                    SweepRefusal::ParentAlive,
                ));
                continue;
            }
            // Finalize: foreign rows are another run's business — skip both
            // the kill AND the cleanup (their owner will reap them).
            if let SweepMode::Finalize {
                own_run_id,
                own_pid,
            } = mode
            {
                if record.run_id != own_run_id {
                    decisions.push(SweepDecision::Refuse(
                        record.clone(),
                        SweepRefusal::ForeignRun,
                    ));
                    continue;
                }
                // Within own rows: only our own pid is exempt from the
                // parent-alive check; a live foreign ancestor refuses.
                let other_live_ancestor = record
                    .parent_chain
                    .iter()
                    .any(|pid| *pid != own_pid && alive_of(*pid));
                if other_live_ancestor {
                    decisions.push(SweepDecision::Refuse(
                        record.clone(),
                        SweepRefusal::ParentAlive,
                    ));
                    continue;
                }
            }
            // Not running at all — nothing to reap; the record can go.
            if !alive_of(record.pid) {
                decisions.push(SweepDecision::Refuse(
                    record.clone(),
                    SweepRefusal::NotRunning,
                ));
                continue;
            }
            // ② fingerprint match (pid-reuse guard). Either side unknown →
            // 归属不明一律不杀: the refusal class records which side failed.
            let Some(registered) = record.image_sha256.clone() else {
                decisions.push(SweepDecision::Refuse(
                    record.clone(),
                    SweepRefusal::FingerprintUnknown,
                ));
                continue;
            };
            let Some(current) = image_hash_of(record.pid) else {
                decisions.push(SweepDecision::Refuse(
                    record.clone(),
                    SweepRefusal::FingerprintUnknown,
                ));
                continue;
            };
            if registered != current {
                decisions.push(SweepDecision::Refuse(
                    record.clone(),
                    SweepRefusal::FingerprintMismatch,
                ));
                continue;
            }
            // ②b creation-time guard (review F-BE-5): same image + reused pid
            // is refused when the creation time drifted from the registry.
            if let Some(created) = creation_time_of(record.pid)
                && record.started_at != 0
                && created.abs_diff(record.started_at) > CREATION_TIME_TOLERANCE_MS
            {
                decisions.push(SweepDecision::Refuse(
                    record.clone(),
                    SweepRefusal::CreationTimeMismatch,
                ));
                continue;
            }
            // ③ window: start sweep takes rows older than this run's start.
            if let SweepMode::Start { run_started_at } = mode
                && record.started_at >= run_started_at
            {
                decisions.push(SweepDecision::Refuse(
                    record.clone(),
                    SweepRefusal::OutsideWindow,
                ));
                continue 'records;
            }
            decisions.push(SweepDecision::Reap(record.clone()));
        }
        decisions
    }

    /// Execute a planned sweep: audit-first (planned fact + registry audit
    /// row) before the first kill, then the executed fact. Kills are
    /// per-pid `TerminateProcess`/`SIGKILL`; per-pid because the kernel job
    /// that owned the tree is gone (that is precisely the orphan scenario).
    pub fn execute_sweep(
        &self,
        decisions: Vec<SweepDecision>,
        reason: &'static str,
        kill_of: &dyn Fn(u32) -> io::Result<()>,
    ) {
        let reaped: Vec<&ProcessTreeRecord> = decisions
            .iter()
            .filter_map(|d| match d {
                SweepDecision::Reap(record) => Some(record),
                SweepDecision::Refuse(..) => None,
            })
            .collect();
        if reaped.is_empty() {
            // Refusals still get their audit row: the negative matrix is
            // evidence too (判据 5's mechanical face on the observation side).
            // NotRunning rows are dropped here (review F-BE-5): a stale
            // record left in the shared registry would keep feeding future
            // sweeps and grow the pid-reuse collision surface.
            for (record, why) in decisions.iter().filter_map(|d| match d {
                SweepDecision::Refuse(record, why) => Some((record, why)),
                SweepDecision::Reap(..) => None,
            }) {
                if matches!(why, SweepRefusal::NotRunning) {
                    self.remove(&record.call_id, record.pid);
                }
            }
            let refusals: Vec<(&ProcessTreeRecord, &SweepRefusal)> = decisions
                .iter()
                .filter_map(|d| match d {
                    SweepDecision::Refuse(record, why) => Some((record, why)),
                    SweepDecision::Reap(..) => None,
                })
                .collect();
            if !refusals.is_empty() {
                let rows: Vec<AuditTarget> = refusals
                    .iter()
                    .map(|(record, why)| AuditTarget {
                        call_id: record.call_id.clone(),
                        pid: record.pid,
                        image_sha256: record.image_sha256.clone(),
                        started_at: record.started_at,
                        refusal: refusal_key(why).to_string(),
                    })
                    .collect();
                self.append_audit_row(reason, "refused", &rows);
            }
            return;
        }

        // Audit first (hardening (c)): the planned fact is observable before
        // the first kill fires.
        self.push_fact(SweepFact {
            reason,
            phase: "planned",
            pids: reaped.iter().map(|r| r.pid).collect(),
            call_ids: reaped.iter().map(|r| r.call_id.clone()).collect(),
        });
        self.append_audit_row(reason, "planned", &audit_targets_reap(&reaped));

        let mut killed: Vec<&ProcessTreeRecord> = Vec::new();
        for record in &reaped {
            match kill_of(record.pid) {
                Ok(()) => killed.push(record),
                Err(e) => {
                    tracing::warn!(
                        pid = record.pid,
                        call_id = %record.call_id,
                        "process tree sweep: kill failed: {e}"
                    );
                }
            }
            self.remove(&record.call_id, record.pid);
        }

        if !killed.is_empty() {
            self.push_fact(SweepFact {
                reason,
                phase: "executed",
                pids: killed.iter().map(|r| r.pid).collect(),
                call_ids: killed.iter().map(|r| r.call_id.clone()).collect(),
            });
        }
        self.append_audit_row(reason, "executed", &audit_targets_reap(&killed));
    }

    /// Drain the accumulated sweep facts for the journal face
    /// (`process_tree_reaped` events). Drain semantics like the idle-kill face.
    pub fn drain_facts(&self) -> Vec<SweepFact> {
        std::mem::take(&mut *self.pending_facts.lock().unwrap())
    }

    fn push_fact(&self, fact: SweepFact) {
        self.pending_facts.lock().unwrap().push(fact);
    }

    /// Append one audit row to `sweep-log.jsonl` — the registry-side audit
    /// trail that also covers sweeps that ran before the run journal existed.
    fn append_audit_row(&self, reason: &str, phase: &str, rows: &[AuditTarget]) {
        if rows.is_empty() {
            return;
        }
        let row = serde_json::json!({
            "event": "process_tree_sweep",
            "reason": reason,
            "phase": phase,
            "targets": rows
                .iter()
                .map(|t| serde_json::json!({
                    "call_id": t.call_id,
                    "pid": t.pid,
                    "image_sha256": t.image_sha256,
                    "started_at": t.started_at,
                    "refusal": t.refusal,
                }))
                .collect::<Vec<_>>(),
            "at_ms": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0),
        });
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.dir.join("sweep-log.jsonl"))
        {
            use std::io::Write;
            let _ = writeln!(file, "{row}");
        }
    }
}

/// Persist one registration row (free fn so the ambient sink closure — which
/// captures only the directory, not the registry — can share the writer).
pub fn write_record(dir: &Path, record: &ProcessTreeRecord) -> io::Result<()> {
    std::fs::create_dir_all(dir)?;
    let safe: String = record
        .call_id
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let path = dir.join(format!("{safe}-{}.json", record.pid));
    let text = serde_json::to_string_pretty(record)
        .map_err(|e| io::Error::other(format!("serialize process tree record: {e}")))?;
    std::fs::write(&path, text)
}

/// Kill one orphan pid — `TerminateProcess` (Windows) / `SIGKILL` (unix).
pub fn kill_pid(pid: u32) -> io::Result<()> {
    #[cfg(windows)]
    {
        use windows::Win32::Foundation::CloseHandle;
        use windows::Win32::System::Threading::{OpenProcess, PROCESS_TERMINATE, TerminateProcess};
        unsafe {
            let handle = OpenProcess(PROCESS_TERMINATE, false, pid)
                .map_err(|e| io::Error::other(format!("OpenProcess({pid}): {e}")))?;
            let result = TerminateProcess(handle, 1);
            let _ = CloseHandle(handle);
            result.map_err(|e| io::Error::other(format!("TerminateProcess({pid}): {e}")))
        }
    }
    #[cfg(not(windows))]
    {
        let rc = unsafe { libc::kill(pid as libc::pid_t, libc::SIGKILL) };
        if rc != 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }
}

/// Is `pid` alive right now (host probe for the sweep matrix)?
pub fn pid_alive(pid: u32) -> bool {
    #[cfg(windows)]
    {
        use windows::Win32::Foundation::CloseHandle;
        use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};
        unsafe {
            match OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) {
                Ok(handle) => {
                    let _ = CloseHandle(handle);
                    true
                }
                Err(_) => false,
            }
        }
    }
    #[cfg(not(windows))]
    {
        // kill(pid, 0) = existence probe without signalling.
        let rc = unsafe { libc::kill(pid as libc::pid_t, 0) };
        rc == 0
    }
}

/// The process creation time of `pid` in Unix milliseconds (sweep probe;
/// `None` = unavailable — the guard then stays silent for this candidate).
pub fn creation_time_of_pid(pid: u32) -> Option<u64> {
    #[cfg(windows)]
    {
        use windows::Win32::Foundation::CloseHandle;
        use windows::Win32::Foundation::FILETIME;
        use windows::Win32::System::Threading::{
            GetProcessTimes, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
        };
        unsafe {
            let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
            let (mut create, mut exit, mut kernel, mut user) = (
                FILETIME::default(),
                FILETIME::default(),
                FILETIME::default(),
                FILETIME::default(),
            );
            let ok = GetProcessTimes(handle, &mut create, &mut exit, &mut kernel, &mut user);
            let _ = CloseHandle(handle);
            if !ok.is_ok() {
                return None;
            }
            // FILETIME = 100-ns intervals since 1601-01-01.
            let ft = ((create.dwHighDateTime as u64) << 32) | create.dwLowDateTime as u64;
            const EPOCH_DIFF_100NS: u64 = 11_644_473_600 * 10_000_000;
            Some(ft.saturating_sub(EPOCH_DIFF_100NS) / 10_000)
        }
    }
    #[cfg(not(windows))]
    {
        // Linux: field 22 of /proc/<pid>/stat = starttime in clock ticks
        // since boot; without boot-time anchoring we report unavailable —
        // the guard stays silent (fail-safe direction).
        None
    }
}

/// The current normalized image hash of `pid` (sweep probe; `None` = unknown).
pub fn image_hash_of_pid(pid: u32) -> Option<String> {
    #[cfg(windows)]
    {
        use windows::Win32::Foundation::CloseHandle;
        use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};
        unsafe {
            let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
            let hash = xai_tty_utils::image_fingerprint_from_handle(handle);
            let _ = CloseHandle(handle);
            hash
        }
    }
    #[cfg(not(windows))]
    {
        xai_tty_utils::image_fingerprint_from_pid(pid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Spawn a real, long-lived child (`ping`) and return its pid.
    #[cfg(windows)]
    fn spawn_long_lived_child() -> u32 {
        let child = std::process::Command::new("cmd")
            .args(["/c", "ping -n 30 127.0.0.1 > NUL"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("spawn sweep probe child");
        child.id()
    }

    /// A guaranteed-dead pid (spawn + reap) — reserved pseudo-pids like
    /// 0xFFFFFFFE are queryable on Windows and NOT dead.
    #[cfg(windows)]
    fn dead_pid() -> u32 {
        let mut child = std::process::Command::new("cmd")
            .args(["/c", "exit 0"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("spawn dead-pid probe");
        let pid = child.id();
        child.wait().expect("reap dead-pid probe");
        std::thread::sleep(std::time::Duration::from_millis(100));
        pid
    }

    /// Same contract as the Windows variant (spawn + reap = guaranteed-dead
    /// pid) so the unix test target compiles and runs the sweep tests too.
    #[cfg(unix)]
    fn dead_pid() -> u32 {
        let mut child = std::process::Command::new("sh")
            .args(["-c", "exit 0"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("spawn dead-pid probe");
        let pid = child.id();
        child.wait().expect("reap dead-pid probe");
        std::thread::sleep(std::time::Duration::from_millis(100));
        pid
    }

    fn unique_dir(tag: &str) -> PathBuf {
        let n = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir =
            std::env::temp_dir().join(format!("orz-proctree-{tag}-{}-{n}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn record(
        pid: u32,
        fingerprint: Option<String>,
        parent: Vec<u32>,
        run: &str,
    ) -> ProcessTreeRecord {
        ProcessTreeRecord {
            call_id: format!("call-{pid}"),
            pid,
            parent_chain: parent,
            image_sha256: fingerprint,
            started_at: 1,
            run_id: run.to_string(),
            job_name: "call".into(),
        }
    }

    /// 三条件正例（start sweep）：父链死 + 指纹匹配 + 窗口内 → Reap，
    /// kill 真的终止子进程（判据 5 正例）。
    #[test]
    fn sweep_reaps_a_matching_orphan_and_kills_it() {
        #[cfg(windows)]
        {
            let pid = spawn_long_lived_child();
            let fingerprint = image_hash_of_pid(pid).expect("probe child fingerprint");
            let registry = ProcessTreeRegistry::new(&unique_dir("reap"));
            let mut record = record(pid, Some(fingerprint), vec![dead_pid()], "RUN-PREV");
            record.started_at = creation_time_of_pid(pid).unwrap_or(1);
            let decisions = registry.plan_sweep(
                &[record],
                SweepMode::Start {
                    run_started_at: u64::MAX / 2,
                },
                &pid_alive,
                &image_hash_of_pid,
                &creation_time_of_pid,
            );
            assert!(
                matches!(decisions[0], SweepDecision::Reap(_)),
                "{decisions:?}"
            );
            kill_pid(pid).expect("kill the orphan");
            std::thread::sleep(std::time::Duration::from_millis(200));
            assert!(!pid_alive(pid), "the orphan must be dead after the kill");
        }
    }

    /// 判据 5 负例矩阵 + review F-BE-5 创建时间守卫：三条件各自不满足的
    /// 进程一律不被杀。
    #[test]
    fn sweep_refuses_every_condition_mismatch() {
        #[cfg(windows)]
        {
            let pid = spawn_long_lived_child();
            let fingerprint = image_hash_of_pid(pid).expect("probe child fingerprint");
            let registry = ProcessTreeRegistry::new(&unique_dir("refuse"));
            let now_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64;

            // ① 父链活（本进程）。
            let live_parent = record(pid, Some(fingerprint.clone()), vec![std::process::id()], "");
            // ② 指纹不匹配（pid 复用防护）。
            let wrong_fp = record(pid, Some("0".repeat(64)), vec![dead_pid()], "");
            // ② 指纹不可得。
            let unknown_fp = record(pid, None, vec![dead_pid()], "");
            // ②b 探针缺席 → 守卫静默（started_at=1 与 probe None）。
            let probe_absent = record(pid, Some(fingerprint.clone()), vec![dead_pid()], "");

            let decisions = registry.plan_sweep(
                &[live_parent, wrong_fp, unknown_fp, probe_absent],
                SweepMode::Start {
                    run_started_at: now_ms,
                },
                &pid_alive,
                &image_hash_of_pid,
                &|_| None,
            );
            assert!(
                matches!(
                    &decisions[0],
                    SweepDecision::Refuse(_, SweepRefusal::ParentAlive)
                ),
                "{decisions:?}"
            );
            assert!(matches!(
                &decisions[1],
                SweepDecision::Refuse(_, SweepRefusal::FingerprintMismatch)
            ));
            assert!(matches!(
                &decisions[2],
                SweepDecision::Refuse(_, SweepRefusal::FingerprintUnknown)
            ));
            assert!(
                matches!(decisions[3], SweepDecision::Reap(_)),
                "{decisions:?}"
            );

            // ②b 正例：真实创建时间（≈now）vs started_at=1 → drift → refuse。
            let time_drift = record(pid, Some(fingerprint.clone()), vec![dead_pid()], "");
            let decisions = registry.plan_sweep(
                &[time_drift],
                SweepMode::Start {
                    run_started_at: now_ms,
                },
                &pid_alive,
                &image_hash_of_pid,
                &creation_time_of_pid,
            );
            match &decisions[0] {
                SweepDecision::Refuse(_, SweepRefusal::CreationTimeMismatch) => {}
                other => panic!("creation-time guard should refuse, got {other:?}"),
            }

            // ③ 窗口外（start sweep 不碰未来登记）。
            let mut future = record(pid, Some(fingerprint), vec![dead_pid()], "");
            future.started_at = now_ms + 60_000;
            let decisions = registry.plan_sweep(
                &[future],
                SweepMode::Start {
                    run_started_at: now_ms,
                },
                &pid_alive,
                &image_hash_of_pid,
                &|_| None,
            );
            assert!(matches!(
                decisions[0],
                SweepDecision::Refuse(_, SweepRefusal::OutsideWindow)
            ));

            assert!(pid_alive(pid), "the probe child must survive every refusal");
            kill_pid(pid).expect("cleanup");
        }
    }

    /// Review F-BE-1：finalize 只豁免**本 run** 的行；他 run 的在跑记录
    /// （ForeignRun）绝不触碰——同工作区并行实例互杀的封堵钉子。
    #[test]
    fn finalize_sweep_never_touches_foreign_rows() {
        #[cfg(windows)]
        {
            let pid = spawn_long_lived_child();
            let fingerprint = image_hash_of_pid(pid).expect("probe child fingerprint");
            let registry = ProcessTreeRegistry::new(&unique_dir("foreign"));

            // 模拟"另一个实例"的在跑记录：run_id=RUN-OTHER。
            let foreign = record(
                pid,
                Some(fingerprint.clone()),
                vec![dead_pid()],
                "RUN-OTHER",
            );
            let decisions = registry.plan_sweep(
                &[foreign],
                SweepMode::Finalize {
                    own_run_id: "RUN-MINE",
                    own_pid: std::process::id(),
                },
                &pid_alive,
                &image_hash_of_pid,
                &creation_time_of_pid,
            );
            assert!(
                matches!(
                    &decisions[0],
                    SweepDecision::Refuse(_, SweepRefusal::ForeignRun)
                ),
                "a foreign run's live child must never be reaped at finalize: {decisions:?}"
            );

            // 本 run 的同形记录 → Reap（父链 = 本进程，豁免成立）。
            let own = record(pid, Some(fingerprint), vec![std::process::id()], "RUN-MINE");
            let decisions = registry.plan_sweep(
                &[own],
                SweepMode::Finalize {
                    own_run_id: "RUN-MINE",
                    own_pid: std::process::id(),
                },
                &pid_alive,
                &image_hash_of_pid,
                &|_| None,
            );
            assert!(
                matches!(decisions[0], SweepDecision::Reap(_)),
                "{decisions:?}"
            );
            kill_pid(pid).expect("kill the leak");
        }
    }

    /// 登记表文件面：写读回一致；NotRunning 行清理；审计行 refusal 标签；
    /// drain 出队即清。
    #[test]
    fn registry_roundtrip_and_audit_rows() {
        let dir = unique_dir("roundtrip");
        std::fs::create_dir_all(&dir).unwrap();
        let registry = ProcessTreeRegistry::new(&dir);
        let record = ProcessTreeRecord {
            call_id: "call-reg-1".into(),
            pid: 4242,
            parent_chain: vec![dead_pid()],
            image_sha256: Some("a".repeat(64)),
            started_at: 42,
            run_id: "RUN-X".into(),
            job_name: "call".into(),
        };
        registry.register(&record).expect("register");
        let loaded = registry.load_all();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].call_id, "call-reg-1");
        assert_eq!(loaded[0].run_id, "RUN-X");

        // NotRunning 拒绝行被清理（review F-BE-5：陈旧记录不再喂未来 sweep）。
        let decisions = registry.plan_sweep(
            &[record.clone()],
            SweepMode::Finalize {
                own_run_id: "RUN-X",
                own_pid: std::process::id(),
            },
            &|_| false,
            &|_| Some("a".repeat(64)),
            &|_| None,
        );
        assert!(matches!(
            &decisions[0],
            SweepDecision::Refuse(_, SweepRefusal::NotRunning)
        ));
        registry.execute_sweep(decisions, "run_shutdown", &|_| Ok(()));
        assert!(
            registry.load_all().is_empty(),
            "the stale row must be dropped"
        );

        let log = std::fs::read_to_string(
            dir.join(".gsa")
                .join("process_trees")
                .join("sweep-log.jsonl"),
        )
        .expect("sweep log exists");
        assert!(log.contains("\"phase\":\"refused\""), "{log}");
        assert!(log.contains("\"refusal\":\"not_running\""), "{log}");
        let facts = registry.drain_facts();
        assert!(facts.is_empty(), "no kill, no facts: {facts:?}");

        // planned/executed 行的 refusal 标签 = "reap"（review F-BE-12）。
        let record2 = ProcessTreeRecord {
            call_id: "call-reg-2".into(),
            pid: 4243,
            parent_chain: vec![],
            image_sha256: Some("b".repeat(64)),
            started_at: 43,
            run_id: "RUN-X".into(),
            job_name: "call".into(),
        };
        registry.register(&record2).unwrap();
        let decisions = registry.plan_sweep(
            &[record2],
            SweepMode::Finalize {
                own_run_id: "RUN-X",
                own_pid: std::process::id(),
            },
            &|_| true,
            &|_| Some("b".repeat(64)),
            &|_| None,
        );
        registry.execute_sweep(decisions, "run_shutdown", &|_| Ok(()));
        let log = std::fs::read_to_string(
            dir.join(".gsa")
                .join("process_trees")
                .join("sweep-log.jsonl"),
        )
        .unwrap();
        assert!(log.contains("\"refusal\":\"reap\""), "{log}");
        let facts = registry.drain_facts();
        assert_eq!(facts.len(), 2, "planned + executed");
        assert!(registry.drain_facts().is_empty());

        let _ = std::fs::remove_dir_all(&dir);
    }
}
