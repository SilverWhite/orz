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
    /// The record's started_at does not place it in the sweep's window.
    OutsideWindow,
}

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
        SweepRefusal::OutsideWindow => "outside_window",
    }
}

fn audit_targets(records: &[&ProcessTreeRecord], refusal: &SweepRefusal) -> Vec<AuditTarget> {
    records
        .iter()
        .map(|r| AuditTarget {
            call_id: r.call_id.clone(),
            pid: r.pid,
            image_sha256: r.image_sha256.clone(),
            started_at: r.started_at,
            refusal: refusal_key(refusal).to_string(),
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
    /// (the testability seam). `alive_of(pid)` / `image_hash_of(pid)` are
    /// host probes; `run_started_at` bounds the window (records older than it
    /// are cross-run orphans; `None` = finalize sweep over this run's rows).
    /// `now_ms` is only used to keep the signature explicit about ordering.
    pub fn plan_sweep(
        &self,
        records: &[ProcessTreeRecord],
        run_started_at: Option<u64>,
        require_parents_dead: bool,
        alive_of: &dyn Fn(u32) -> bool,
        image_hash_of: &dyn Fn(u32) -> Option<String>,
    ) -> Vec<SweepDecision> {
        let mut decisions = Vec::new();
        for record in records {
            // ① parent chain dead. The recorded chain includes the spawning
            // orz pid — for this run's own rows that pid is alive, so the
            // start sweep refuses them (in-flight protection, §4.7.1 第 14
            // 条). The finalize sweep runs at shutdown with
            // `require_parents_dead = false`: the run is over, its leaked
            // children are exactly the target, and the pid-reuse guard (②)
            // still refuses anything whose image does not match its record.
            if require_parents_dead && record.parent_chain.iter().any(|pid| alive_of(*pid)) {
                decisions.push(SweepDecision::Refuse(
                    record.clone(),
                    SweepRefusal::ParentAlive,
                ));
                continue;
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
            // ③ window: start sweep takes previous-run rows; finalize sweep
            // takes this run's own rows.
            let in_window = match run_started_at {
                Some(start) => record.started_at < start,
                None => true,
            };
            if !in_window {
                decisions.push(SweepDecision::Refuse(
                    record.clone(),
                    SweepRefusal::OutsideWindow,
                ));
                continue;
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
        self.append_audit_row(
            reason,
            "planned",
            &audit_targets(&reaped, &SweepRefusal::NotRunning),
        );

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
        self.append_audit_row(
            reason,
            "executed",
            &audit_targets(&killed, &SweepRefusal::NotRunning),
        );
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

    /// Spawn a real, long-lived child (`ping`) and return its pid + the
    /// normalized image fingerprint the sweep would compare.
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

    fn unique_dir(tag: &str) -> PathBuf {
        let n = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir =
            std::env::temp_dir().join(format!("orz-proctree-{}-{}-{}", tag, std::process::id(), n));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// 三条件正例：父链死（探针 pid 不存在）+ 指纹匹配 + 窗口内（上一 run
    /// 的记录）→ Reap，且 kill 真的终止子进程。
    #[test]
    fn sweep_reaps_a_matching_orphan_and_kills_it() {
        #[cfg(windows)]
        {
            let pid = spawn_long_lived_child();
            let fingerprint = image_hash_of_pid(pid).expect("probe child fingerprint");
            let registry = ProcessTreeRegistry::new(&unique_dir("reap"));
            let record = ProcessTreeRecord {
                call_id: "call-orphan-1".into(),
                pid,
                parent_chain: vec![u32::MAX - 1], // never a live pid → parent dead
                image_sha256: Some(fingerprint),
                started_at: 1, // before any run window → in window
                run_id: "RUN-PREV".into(),
                job_name: "call".into(),
            };
            let decisions = registry.plan_sweep(
                &[record],
                Some(u64::MAX / 2),
                true,
                &pid_alive,
                &image_hash_of_pid,
            );
            assert_eq!(decisions.len(), 1);
            assert!(
                matches!(decisions[0], SweepDecision::Reap(_)),
                "a matching orphan must be reaped: {decisions:?}"
            );
            kill_pid(pid).expect("kill the orphan");
            // 等待内核回收句柄后确认死亡。
            std::thread::sleep(std::time::Duration::from_millis(200));
            assert!(!pid_alive(pid), "the orphan must be dead after the kill");
        }
    }

    /// 判据 5 负例矩阵：三条件各自不满足的进程一律不被杀。
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

            // ① 父链活：parent_chain = 本进程（在跑重活保护，§4.7.1 第 14 条）。
            let live_parent = ProcessTreeRecord {
                call_id: "call-in-flight".into(),
                pid,
                parent_chain: vec![std::process::id()],
                image_sha256: Some(fingerprint.clone()),
                started_at: 1,
                run_id: String::new(),
                job_name: "call".into(),
            };
            // ② 指纹不匹配：pid 复用防护。
            let wrong_fingerprint = ProcessTreeRecord {
                call_id: "call-reuse".into(),
                pid,
                parent_chain: vec![u32::MAX - 1],
                image_sha256: Some("0".repeat(64)),
                started_at: 1,
                run_id: String::new(),
                job_name: "call".into(),
            };
            // ② 指纹不可得：归属不明一律不杀。
            let unknown_fingerprint = ProcessTreeRecord {
                call_id: "call-unknown".into(),
                pid,
                parent_chain: vec![u32::MAX - 1],
                image_sha256: None,
                started_at: 1,
                run_id: String::new(),
                job_name: "call".into(),
            };
            let decisions = registry.plan_sweep(
                &[live_parent, wrong_fingerprint, unknown_fingerprint],
                Some(now_ms),
                true,
                &pid_alive,
                &image_hash_of_pid,
            );
            assert_eq!(decisions.len(), 3);
            assert!(
                matches!(
                    &decisions[0],
                    SweepDecision::Refuse(_, SweepRefusal::ParentAlive)
                ),
                "a live parent must refuse the kill: {decisions:?}"
            );
            assert!(
                matches!(
                    &decisions[1],
                    SweepDecision::Refuse(_, SweepRefusal::FingerprintMismatch)
                ),
                "a fingerprint mismatch (pid reuse) must refuse: {decisions:?}"
            );
            assert!(
                matches!(
                    &decisions[2],
                    SweepDecision::Refuse(_, SweepRefusal::FingerprintUnknown)
                ),
                "an unknown fingerprint must refuse: {decisions:?}"
            );

            // ③ 窗口外：start 扫除不碰本 run 之后才登记的记录。
            let future_record = ProcessTreeRecord {
                call_id: "call-future".into(),
                pid,
                parent_chain: vec![u32::MAX - 1],
                image_sha256: Some(fingerprint),
                started_at: now_ms + 60_000,
                run_id: String::new(),
                job_name: "call".into(),
            };
            let decisions = registry.plan_sweep(
                &[future_record],
                Some(now_ms),
                true,
                &pid_alive,
                &image_hash_of_pid,
            );
            assert!(matches!(
                decisions[0],
                SweepDecision::Refuse(_, SweepRefusal::OutsideWindow)
            ));

            // 硬化 (a)：不在登记表内的 pid 根本不进矩阵（plan 只看记录）。
            // 进程仍活着（从未被扫除触碰）。
            assert!(pid_alive(pid), "the probe child must survive every refusal");
            kill_pid(pid).expect("cleanup");
        }
    }

    /// 收尾扫除（run_shutdown）：`require_parents_dead = false`——本 run 的
    /// 泄漏子进程（父=本进程，仍活）在收尾时被回收。
    #[test]
    fn finalize_sweep_reaps_this_run_leaks_without_parent_death() {
        #[cfg(windows)]
        {
            let pid = spawn_long_lived_child();
            let fingerprint = image_hash_of_pid(pid).expect("probe child fingerprint");
            let registry = ProcessTreeRegistry::new(&unique_dir("finalize"));
            let record = ProcessTreeRecord {
                call_id: "call-leak".into(),
                pid,
                parent_chain: vec![std::process::id()], // alive during shutdown
                image_sha256: Some(fingerprint),
                started_at: 1,
                run_id: String::new(),
                job_name: "call".into(),
            };
            let decisions =
                registry.plan_sweep(&[record], None, false, &pid_alive, &image_hash_of_pid);
            assert!(matches!(decisions[0], SweepDecision::Reap(_)));
            kill_pid(pid).expect("kill the leak");
        }
    }

    /// 登记表文件面：写读回一致；审计行落 sweep-log.jsonl；drain 出队即清。
    #[test]
    fn registry_roundtrip_and_audit_rows() {
        let dir = unique_dir("roundtrip");
        // 登记表目录 = dir/.gsa/process_trees（new() 拼接 .gsa/process_trees）。
        std::fs::create_dir_all(&dir).unwrap();
        let registry = ProcessTreeRegistry::new(&dir);
        let record = ProcessTreeRecord {
            call_id: "call-reg-1".into(),
            pid: 4242,
            parent_chain: vec![std::process::id()],
            image_sha256: Some("a".repeat(64)),
            started_at: 42,
            run_id: "RUN-X".into(),
            job_name: "call".into(),
        };
        registry.register(&record).expect("register");
        let loaded = registry.load_all();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].call_id, "call-reg-1");
        assert_eq!(loaded[0].pid, 4242);
        assert_eq!(
            loaded[0].image_sha256.as_deref(),
            Some("a".repeat(64).as_str())
        );

        // 审计行（planned + executed）落盘：kill 闭包用 Ok(())，真实回收
        // 语义由上两个测试用真实进程覆盖；此处只验文件与 drain 面。
        let record2 = ProcessTreeRecord {
            call_id: "call-reg-1".into(),
            pid: 4242,
            parent_chain: vec![u32::MAX - 1],
            image_sha256: Some("a".repeat(64)),
            started_at: 42,
            run_id: "RUN-X".into(),
            job_name: "call".into(),
        };
        let decisions = registry.plan_sweep(&[record2], None, false, &|_| true, &|_| {
            Some("a".repeat(64))
        });
        registry.execute_sweep(decisions, "run_shutdown", &|_| Ok(()));
        let log = std::fs::read_to_string(
            dir.join(".gsa")
                .join("process_trees")
                .join("sweep-log.jsonl"),
        )
        .expect("sweep log exists");
        assert!(log.contains("\"phase\":\"planned\""), "{log}");
        assert!(log.contains("\"phase\":\"executed\""), "{log}");

        // drain 语义：facts 出队即清空（planned + executed 两行）。
        let facts = registry.drain_facts();
        assert_eq!(facts.len(), 2);
        assert!(registry.drain_facts().is_empty());

        let _ = std::fs::remove_dir_all(&dir);
    }
}
