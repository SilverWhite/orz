//! Kernel-enforced resource ceilings for a run's tool process trees
//! (FUS-HOST-RESOURCE-SAFETY §4.7, 0z S1).
//!
//! The framework does not schedule the host and does not sandbox the run —
//! both are user rulings (design §2). What it *does* do is put a ceiling where
//! the 2026-09-12 real-machine run died twice: the tool process trees orz
//! itself spawns. On Windows that ceiling is enforced by the kernel through a
//! Job Object — not by framework sampling, and not by asking the model to be
//! careful:
//!
//! | limit | Win32 knob | what it bounds |
//! |---|---|---|
//! | commit | `JOB_OBJECT_LIMIT_JOB_MEMORY` | committed virtual memory of the tree (the line Run B blew) |
//! | concurrency | `JOB_OBJECT_LIMIT_ACTIVE_PROCESS` | static process-count ceiling (replaces dynamic `CARGO_BUILD_JOBS` injection) |
//! | CPU rate | `JOB_OBJECT_CPU_RATE_CONTROL_HARD_CAP` | per-scheduling-interval CPU ceiling |
//! | tree teardown | `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` | no orphans when orz dies (design §4.2) |
//!
//! # Two levels, root first (design §4.2 item 1 / §4.7; restored 2026-09-12)
//!
//! One **run-level** job carries the ceilings and lives as long as the process
//! ([`install_global_run_job`]); every **tool call** gets its own job carrying
//! `KILL_ON_JOB_CLOSE` (the tree-teardown granularity). A spawned tool process
//! is assigned to the run job **first** and to its call job **second** — the
//! order Microsoft documents for building a valid hierarchy:
//!
//! > To ensure that the job hierarchy is valid, first assign all processes to
//! > the job at the root of the hierarchy, then assign a subset of processes to
//! > the immediate child job object, and so on.
//! > — Nested Jobs, learn.microsoft.com/windows/win32/procthread/nested-jobs
//!
//! Nesting is a Windows 8+ property of *process assignment*, not of linking two
//! job handles: "A process can be associated with more than one job in a
//! hierarchy of nested jobs" (Job Objects page). S1 originally read
//! `AssignProcessToJobObject(job, job) -> ERROR_INVALID_HANDLE` as "nesting is
//! unavailable"; that call passes a **job** handle where a **process** handle
//! belongs, so it said nothing about nesting. The independent review
//! (`docs/audits/0Z_S1_INDEPENDENT_REVIEW_2026-09-12.md`, F-1) reproduced the
//! correct route: assign to a plain job, then to a ceiling-carrying job, and
//! the ceiling still refuses an over-commit.
//!
//! Why the root level matters (Microsoft's wording): `JOB_OBJECT_LIMIT_JOB_MEMORY`
//! "causes all processes associated with the job to limit the job-wide sum of
//! their committed memory". With the ceilings on the run job the bound is the
//! **run's** aggregate — N concurrent tool calls share one ceiling instead of
//! each holding a full copy — and the CPU-rate and active-process limits become
//! run-wide as well. The per-call job keeps deciding *when* a tree dies.
//!
//! **A job cannot bound disk.** Volume headroom stays with the pre-dispatch
//! gate and the reclaim ladder (design §4.7 盘侧例外).
//!
//! Linux has no per-job equivalent without cgroups (which the run deliberately
//! does not require), so the handle records the limits and reports
//! `enforced: false` — an honest reading, never a silent claim of enforcement.

use std::io;
use std::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};
use std::sync::{Arc, Mutex};

/// Kernel-enforced ceiling for tool process trees.
///
/// All fields are optional: `None` means "no limit on this axis", which is the
/// default (`JobLimits::default()`) so a run that injects nothing keeps the
/// pre-0z behavior.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct JobLimits {
    /// Commit (virtual memory) ceiling for the job tree, in bytes.
    pub commit_limit_bytes: Option<u64>,
    /// 0bc（2026-09-20 裁决 ②，Windows）：commit **临限通知**阈值——
    /// 与硬顶 [`Self::commit_limit_bytes`] 不同：内核不拒任何分配，只在
    /// 作业 commit 触到阈值时向作业完成端口投 `JOB_OBJECT_MSG_NOTIFICATION_
    /// LIMIT`；宿主取件后记事件＋软提示（不由 orz 硬顶／硬拒）。非 Windows
    /// 平台记录意图、如实报非强制（同 `enforced: false` 口径）。
    pub commit_notification_bytes: Option<u64>,
    /// Static concurrent-process ceiling for the job tree.
    pub active_process: Option<u32>,
    /// CPU rate ceiling per scheduling interval, in percent (1..=100).
    pub cpu_rate_percent: Option<u32>,
}

impl JobLimits {
    /// True when no axis is limited and no notification is wired (nothing to
    /// enforce and nothing to observe).
    pub fn is_empty(&self) -> bool {
        self.commit_limit_bytes.is_none()
            && self.commit_notification_bytes.is_none()
            && self.active_process.is_none()
            && self.cpu_rate_percent.is_none()
    }

    /// One-line mechanical rendering (audit / observation text).
    pub fn describe(&self) -> String {
        fn gib(bytes: u64) -> String {
            format!("{:.2} GiB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
        }
        let commit = self
            .commit_limit_bytes
            .map(gib)
            .unwrap_or_else(|| "unlimited".to_string());
        let notify = self
            .commit_notification_bytes
            .map(gib)
            .unwrap_or_else(|| "unlimited".to_string());
        let procs = self
            .active_process
            .map(|n| n.to_string())
            .unwrap_or_else(|| "unlimited".to_string());
        let cpu = self
            .cpu_rate_percent
            .map(|n| format!("{n}%"))
            .unwrap_or_else(|| "unlimited".to_string());
        format!("commit={commit} commit_notify={notify} active_process={procs} cpu_rate={cpu}")
    }
}

/// What the kernel reports back after the ceilings were set — the read-back
/// face of "the limit is really in force" (design §6 判据 12).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct JobReadback {
    pub commit_limit_bytes: Option<u64>,
    /// 0bc：kernel 可见的 commit 临限通知阈值（`None` = 未装/读不回）。
    /// **不属"天花板"轴**——它不拒绝任何分配，只投递通知消息。
    pub commit_notification_bytes: Option<u64>,
    pub active_process: Option<u32>,
    pub cpu_rate_percent: Option<u32>,
}

impl JobReadback {
    /// True when at least one **ceiling** came back from the kernel (the
    /// notification face is observation, not enforcement — see
    /// [`Self::commit_notification_bytes`]).
    pub fn is_enforced(&self) -> bool {
        self.commit_limit_bytes.is_some()
            || self.active_process.is_some()
            || self.cpu_rate_percent.is_some()
    }
}

/// The run's **root job**: it holds the ceilings for the whole run and its
/// handle is the kernel's reference for the run's process hierarchy.
///
/// The handle lives as long as the process (the global slot below), so
/// `KILL_ON_JOB_CLOSE` on this job is the crash-path teardown of design §4.2:
/// however orz dies — including `abort()`, which runs no destructors — closing
/// the handle terminates every tool process still associated with the run.
pub struct RunResourceJob {
    limits: JobLimits,
    readback: JobReadback,
    #[cfg(windows)]
    job: windows::Win32::Foundation::HANDLE,
    /// 0bc 裁决 ②：commit 临限通知的作业完成端口（`None` = 未装通知面）。
    /// 由 [`Self::drain_notifications`] 轮询取件；随本结构一并释放。
    #[cfg(windows)]
    notification_port: Option<windows::Win32::Foundation::HANDLE>,
}

#[cfg(windows)]
unsafe impl Send for RunResourceJob {}
#[cfg(windows)]
unsafe impl Sync for RunResourceJob {}

impl RunResourceJob {
    /// Install the run's root job: `KILL_ON_JOB_CLOSE` + the ceilings, then read
    /// back what the kernel accepted (design §6 判据 12).
    ///
    /// An axis the kernel refuses shows up as `None` in [`Self::readback`], so
    /// `is_kernel_enforced` never over-claims. The handle is kept open: it is the
    /// root of the run's job hierarchy and the ceilings' enforcement point.
    pub fn install(limits: JobLimits) -> io::Result<Arc<Self>> {
        inspect_run_job(limits)
    }

    /// 0bc 探针/对照入口：以显式限位方向装置临限通知（生产入口
    /// [`Self::install`] 恒用 [`COMMIT_NOTIFICATION_MODE`]）。
    #[cfg(windows)]
    pub fn install_with_notification_mode(
        limits: JobLimits,
        mode: CommitNotificationMode,
    ) -> io::Result<Arc<Self>> {
        inspect_run_job_with_mode(limits, mode)
    }

    /// The ceilings this run decided on.
    pub fn limits(&self) -> JobLimits {
        self.limits
    }

    /// What the kernel reports as in force for these ceilings.
    pub fn readback(&self) -> JobReadback {
        self.readback
    }

    /// True when at least one ceiling is kernel-enforced.
    pub fn is_kernel_enforced(&self) -> bool {
        self.readback.is_enforced()
    }

    /// One-line mechanical rendering for the session/audit face.
    pub fn describe(&self) -> String {
        format!(
            "{} (kernel_enforced={})",
            self.limits.describe(),
            self.is_kernel_enforced()
        )
    }

    /// Assign an already-associated process handle to the run's root job.
    ///
    /// Contract (Nested Jobs): the caller assigns to the root **before** the
    /// per-call job, so the per-call job is always a subset of the root.
    #[cfg(windows)]
    pub(crate) fn assign_process_handle(
        &self,
        process: windows::Win32::Foundation::HANDLE,
    ) -> io::Result<()> {
        assign_handle_to_job(self.job, process)
    }

    /// Is `pid` associated with the run's root job? The mechanical face of the
    /// end-to-end test (design §6 判据 12: the tree really is inside the job).
    #[cfg(windows)]
    pub fn contains_process(&self, pid: u32) -> io::Result<bool> {
        process_in_job(self.job, pid)
    }

    /// Non-Windows: no kernel job, so nothing is ever contained.
    #[cfg(not(windows))]
    pub fn contains_process(&self, _pid: u32) -> io::Result<bool> {
        Ok(false)
    }

    /// Hard-tier tree kill (design §4.8 表 1 / §4.7.1 第 14 条, 0z S2):
    /// terminate every process associated with the run's root job — the whole
    /// run's tool tree, not one call. Constraints live with the caller (only
    /// hard tier, audit first via `resource_exhausted(planned)`, only heavy
    /// classes, readable failure afterwards); this handle only guarantees the
    /// kernel-level blast radius is exactly the run's job tree.
    ///
    /// Non-Windows: no kernel job — the caller's process-group teardown is the
    /// only face, so this reports honest non-enforcement.
    #[cfg(windows)]
    pub fn kill(&self) -> io::Result<()> {
        use windows::Win32::System::JobObjects::TerminateJobObject;

        unsafe { TerminateJobObject(self.job, 1) }
            .map_err(|e| io::Error::other(format!("TerminateJobObject(run): {e}")))
    }

    /// Non-Windows: no kernel job to terminate — honest non-enforcement.
    #[cfg(not(windows))]
    pub fn kill(&self) -> io::Result<()> {
        Err(io::Error::other(
            "run job kill: not kernel-enforced on this platform",
        ))
    }

    /// 0bc 裁决 ②：取**本轮新到**的 commit 临限通知（非阻塞；队列空即返回，
    /// 不等待）。Windows＝作业完成端口轮询；其它平台恒空（无该面，如实）。
    #[cfg(windows)]
    pub fn drain_notifications(&self) -> Vec<JobNotification> {
        drain_job_notifications(self.job, self.notification_port)
    }

    /// 非 Windows：无内核作业＝无通知面，如实返回空。
    #[cfg(not(windows))]
    pub fn drain_notifications(&self) -> Vec<JobNotification> {
        Vec::new()
    }

    /// 临限通知面是否已装置（观测面可用性；阈值读数见 [`Self::readback`]）。
    pub fn notification_installed(&self) -> bool {
        #[cfg(windows)]
        {
            self.notification_port.is_some()
        }
        #[cfg(not(windows))]
        {
            false
        }
    }
}

#[cfg(windows)]
impl Drop for RunResourceJob {
    fn drop(&mut self) {
        // Closing the handle is what fires `KILL_ON_JOB_CLOSE`. Production keeps
        // the run job in the global slot for the process lifetime, so this runs
        // at process exit (or when a test replaces the handle).
        let _ = unsafe { windows::Win32::Foundation::CloseHandle(self.job) };
        if let Some(port) = self.notification_port {
            // 完成端口是普通内核句柄：作业句柄关闭后没有待取件，直接释放。
            let _ = unsafe { windows::Win32::Foundation::CloseHandle(port) };
        }
    }
}

/// The process-wide root job, when one was installed. `Mutex<Option<_>>` rather
/// than `OnceLock` so the test seam below can replace it; the production path
/// installs exactly once.
static GLOBAL_RUN_JOB: Mutex<Option<Arc<RunResourceJob>>> = Mutex::new(None);

fn global_slot() -> std::sync::MutexGuard<'static, Option<Arc<RunResourceJob>>> {
    GLOBAL_RUN_JOB
        .lock()
        .unwrap_or_else(|poison| poison.into_inner())
}

/// The process-wide run ceilings, when one was installed.
///
/// orz serves one run per process (the `-p` and ACP paths both build the host
/// once at assembly), so process lifetime is the run lifetime this handle
/// tracks. Every [`crate::ProcessGroup`] attaches its child to this job.
pub fn global_run_job() -> Option<Arc<RunResourceJob>> {
    global_slot().clone()
}

/// Install the process-wide ceilings. First caller wins (later calls return the
/// existing handle): the ceilings are a run-level contract, not a per-call
/// tuning knob.
pub fn install_global_run_job(limits: JobLimits) -> io::Result<Arc<RunResourceJob>> {
    let mut slot = global_slot();
    if let Some(existing) = slot.as_ref() {
        return Ok(existing.clone());
    }
    let job = RunResourceJob::install(limits)?;
    *slot = Some(job.clone());
    Ok(job)
}

/// Replace the process-wide ceilings — the **test seam** for suites that need a
/// deliberately small ceiling to observe enforcement.
///
/// Dropping the previous job closes its handle, which (by `KILL_ON_JOB_CLOSE`)
/// terminates any process still associated with it. Production code must not
/// call this; `install_global_run_job` is the only production entry point.
pub fn replace_global_run_job_for_tests(limits: JobLimits) -> io::Result<Arc<RunResourceJob>> {
    let job = RunResourceJob::install(limits)?;
    let mut slot = global_slot();
    *slot = Some(job.clone());
    Ok(job)
}

/// 同 [`replace_global_run_job_for_tests`]，但显式指定临限通知方向（0bc
/// 探针用；生产路径不采用）。
#[cfg(all(test, windows))]
fn replace_global_run_job_for_tests_with_mode(
    limits: JobLimits,
    mode: CommitNotificationMode,
) -> io::Result<Arc<RunResourceJob>> {
    let job = RunResourceJob::install_with_notification_mode(limits, mode)?;
    let mut slot = global_slot();
    *slot = Some(job.clone());
    Ok(job)
}

/// How many times a spawn path failed to put its child into the run's job —
/// the mechanical visibility face for "the ceilings are not in force here"
/// (independent review F-9). A failure is never silent, only counted.
static ATTACH_FAILURES: AtomicU64 = AtomicU64::new(0);

/// Count one failed attach (called by the spawn paths that swallow the error).
pub fn record_attach_failure() {
    ATTACH_FAILURES.fetch_add(1, AtomicOrdering::Relaxed);
}

/// Total attach failures observed in this process (observation face / S2).
pub fn attach_failure_count() -> u64 {
    ATTACH_FAILURES.load(AtomicOrdering::Relaxed)
}

// ---------------------------------------------------------------------------
// Spawn registration sink (FUS-HOST-RESOURCE-SAFETY §4.2 item 3, 0z S2)
// ---------------------------------------------------------------------------

/// One child-spawn observation, delivered to the ambient sink by
/// [`crate::ProcessGroup::attach_pid`] — the single choke point every tool
/// child already flows through. The host owns the call id and the session
/// volume and turns this into the `.gsa/process_trees/<call_id>.json` record;
/// the executor layer (orz-tools) attaches children without knowing about
/// either, which is why the seam lives in the lowest common crate.
#[derive(Clone, Debug)]
pub struct SpawnObservation {
    pub pid: u32,
    /// sha256 over the **normalized** (canonicalized, lowercased) executable
    /// image path — the fingerprint the sweep's condition ② re-queries on the
    /// candidate process and compares against the registry record. `None` when
    /// the platform cannot provide it (the sweep then refuses to kill:
    /// 归属不明一律不杀).
    pub image_sha256: Option<String>,
    /// Unix milliseconds.
    pub started_at: u64,
    /// FUS-HOST-RESOURCE-SAFETY §4.8 表 1 ③ (0z S2R, user ruling 2026-09-13:
    /// hard-tier tree kill = per-call-job face, option (a)): a **duplicated**
    /// handle to this call's job, owned by the receiver. The host keeps it
    /// for the dispatch's lifetime so the hard tier can `TerminateJobObject`
    /// exactly the heavy calls' trees — whole-tree granularity of a call job
    /// without the run job's blast radius. `0` when duplication failed (the
    /// kill face then skips this call). Closing the duplicate is the
    /// receiver's duty.
    #[cfg(windows)]
    pub job_handle_dup: isize,
}

type SinkFn = dyn Fn(SpawnObservation) + Send + Sync;

/// The ambient registration sink — `Some` exactly while a tool call is being
/// dispatched by the host on this seam.
static SPAWN_SINK: Mutex<Option<Arc<SinkFn>>> = Mutex::new(None);

fn spawn_sink_slot() -> std::sync::MutexGuard<'static, Option<Arc<SinkFn>>> {
    SPAWN_SINK
        .lock()
        .unwrap_or_else(|poison| poison.into_inner())
}

/// Install the registration sink for the duration of one tool call. Returns a
/// guard that RESTORES the previous sink on drop (review F-BE-9: clearing the
/// slot unconditionally would unregister spawns of a still-running call when
/// a concurrent call finishes first). Cross-thread attribution races remain
/// possible — the sweep never trusts the label alone (pid + fingerprint +
/// creation-time + window decide the kill), so the race degrades audit
/// attribution, never kill safety.
pub fn set_spawn_sink(sink: Arc<SinkFn>) -> SpawnSinkGuard {
    let previous = spawn_sink_slot().replace(sink);
    SpawnSinkGuard { previous }
}

/// Guard restoring the previous spawn sink on drop.
pub struct SpawnSinkGuard {
    previous: Option<Arc<SinkFn>>,
}

impl Drop for SpawnSinkGuard {
    fn drop(&mut self) {
        *spawn_sink_slot() = self.previous.take();
    }
}

/// Deliver one spawn observation to the ambient sink, if any. Best-effort by
/// contract: registration failure never blocks a spawn.
pub fn register_spawn(observation: SpawnObservation) {
    let sink = spawn_sink_slot().clone();
    if let Some(sink) = sink {
        sink(observation);
    }
}

/// Is `pid` alive right now (kill-face test probe; existence only).
#[cfg(windows)]
pub fn process_alive(pid: u32) -> bool {
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

/// Non-Windows existence probe: `kill(pid, 0)`.
#[cfg(not(windows))]
pub fn process_alive(pid: u32) -> bool {
    let rc = unsafe { libc::kill(pid as libc::pid_t, 0) };
    rc == 0
}

/// Duplicate the call-job handle for the host's live-call registry (see
/// [`SpawnObservation::job_handle_dup`]).
#[cfg(windows)]
pub fn duplicate_job_handle(job: windows::Win32::Foundation::HANDLE) -> io::Result<isize> {
    use windows::Win32::Foundation::{DUPLICATE_SAME_ACCESS, DuplicateHandle};
    use windows::Win32::System::Threading::GetCurrentProcess;

    let mut dup = Default::default();
    unsafe {
        DuplicateHandle(
            GetCurrentProcess(),
            job,
            GetCurrentProcess(),
            &mut dup,
            0,
            false,
            DUPLICATE_SAME_ACCESS,
        )
    }
    .map_err(|e| io::Error::other(format!("DuplicateHandle(call job): {e}")))?;
    Ok(dup.0 as isize)
}

/// Terminate every process in the job behind a duplicated call-job handle
/// (the hard tier's per-call kill face; design §4.8 表 1 ③ as ruled —
/// option (a), 2026-09-13). `handle == 0` is the "no handle" sentinel.
#[cfg(windows)]
pub fn terminate_job_handle(handle: isize) -> io::Result<()> {
    use windows::Win32::System::JobObjects::TerminateJobObject;

    if handle == 0 {
        return Err(io::Error::other(
            "terminate_job_handle: no duplicated handle",
        ));
    }
    unsafe { TerminateJobObject(windows::Win32::Foundation::HANDLE(handle as _), 1) }
        .map_err(|e| io::Error::other(format!("TerminateJobObject(call): {e}")))
}

/// Non-Windows: no call-job kill face (the process-group kill stays the
/// executor's own responsibility).
#[cfg(not(windows))]
pub fn terminate_job_handle(_handle: isize) -> io::Result<()> {
    Err(io::Error::other(
        "terminate_job_handle: not kernel-enforced on this platform",
    ))
}

/// The normalized image fingerprint for a still-open process handle:
/// `QueryFullProcessImageNameW`, canonicalized + lowercased, sha256'd.
#[cfg(windows)]
pub fn image_fingerprint_from_handle(handle: windows::Win32::Foundation::HANDLE) -> Option<String> {
    use windows::Win32::System::Threading::PROCESS_NAME_WIN32;
    use windows::Win32::System::Threading::QueryFullProcessImageNameW;
    use windows::core::PWSTR;

    let mut buf = [0u16; 1024];
    let mut size = buf.len() as u32;
    let result = unsafe {
        QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_WIN32,
            PWSTR(buf.as_mut_ptr()),
            &mut size,
        )
    };
    if result.is_err() {
        return None;
    }
    let path = String::from_utf16_lossy(&buf[..size as usize]);
    // Normalize: Windows paths are case-insensitive and may arrive with mixed
    // separators / prefixes; a canonical, lowercased form keeps the fingerprint
    // stable across query sites (hardening b).
    Some(sha256_hex_string(&path.replace('/', "\\").to_lowercase()))
}

/// Windows fingerprint by pid: open a query handle and read the image path.
#[cfg(windows)]
pub fn image_fingerprint_from_pid(pid: u32) -> Option<String> {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};

    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let hash = image_fingerprint_from_handle(handle);
        let _ = CloseHandle(handle);
        hash
    }
}

/// Linux fingerprint: the `/proc/<pid>/exe` target, sha256'd.
#[cfg(target_os = "linux")]
pub fn image_fingerprint_from_pid(pid: u32) -> Option<String> {
    let target = std::fs::read_link(format!("/proc/{pid}/exe")).ok()?;
    Some(sha256_hex_string(&target.display().to_string()))
}

/// Other platforms: no image fingerprint — the sweep refuses to kill
/// fingerprint-less candidates (归属不明一律不杀).
#[cfg(not(any(windows, target_os = "linux")))]
pub fn image_fingerprint_from_pid(_pid: u32) -> Option<String> {
    None
}

/// sha256 hex of a string (small local helper — keeps the fingerprint format
/// owned by this module).
pub fn sha256_hex_string(input: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    let digest = hasher.finalize();
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

// ---------------------------------------------------------------------------
// Windows implementation
// ---------------------------------------------------------------------------

#[cfg(windows)]
fn assign_handle_to_job(
    job: windows::Win32::Foundation::HANDLE,
    process: windows::Win32::Foundation::HANDLE,
) -> io::Result<()> {
    use windows::Win32::System::JobObjects::AssignProcessToJobObject;

    unsafe { AssignProcessToJobObject(job, process) }
        .map_err(|e| io::Error::other(format!("AssignProcessToJobObject(run): {e}")))
}

#[cfg(windows)]
fn process_in_job(job: windows::Win32::Foundation::HANDLE, pid: u32) -> io::Result<bool> {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::JobObjects::IsProcessInJob;
    use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};

    let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) }
        .map_err(|e| io::Error::other(format!("OpenProcess({pid}) for job query: {e}")))?;
    let mut inside = windows::core::BOOL::default();
    let result = unsafe { IsProcessInJob(process, Some(job), &mut inside) };
    let _ = unsafe { CloseHandle(process) };
    result.map_err(|e| io::Error::other(format!("IsProcessInJob({pid}): {e}")))?;
    Ok(inside.as_bool())
}

/// Create the run's root job (`KILL_ON_JOB_CLOSE` + ceilings) and keep it.
/// 0bc 临限通知的**限位方向**（真机探针钉住，2026-09-20；见
/// [`CommitNotificationMode`]）：设计 §11 裁决 ② 的应用面语义是
/// "commit 触到阈值时通知"＝`High`；LOW 限位在真机上表现为"低于限位即
/// 通知"（装置当刻即投递一条），不合本用。原语方向修订记于 0bc 总结文档。
#[cfg(windows)]
pub const COMMIT_NOTIFICATION_MODE: CommitNotificationMode = CommitNotificationMode::High;

#[cfg(windows)]
fn inspect_run_job(limits: JobLimits) -> io::Result<Arc<RunResourceJob>> {
    inspect_run_job_with_mode(limits, COMMIT_NOTIFICATION_MODE)
}

/// 装置入口（方向参数化；生产恒 [`COMMIT_NOTIFICATION_MODE`]，测试/探针可
/// 显式指定方向做对照）。
#[cfg(windows)]
fn inspect_run_job_with_mode(
    limits: JobLimits,
    mode: CommitNotificationMode,
) -> io::Result<Arc<RunResourceJob>> {
    let job = windows_create_job()?;
    if let Err(e) = apply_limits_to_job(job, &limits) {
        let _ = unsafe { windows::Win32::Foundation::CloseHandle(job) };
        return Err(e);
    }
    // 0bc 裁决 ②（2026-09-20）：commit 临限通知＝**观测面**（非天花板）。
    // 装置失败不拆 run——readback 如实以 `None` 落（宁缺勿假），其余面照常。
    // 先装通知、后读回：readback 的 `commit_notification_bytes` 由内核查询
    // 给出（查询面不支持时保持 `None`，不冒认）。
    let notification_port = match limits.commit_notification_bytes {
        Some(bytes) => install_commit_notification(job, bytes, mode).ok(),
        None => None,
    };
    let readback = match readback_job(job) {
        Ok(readback) => readback,
        Err(e) => {
            if let Some(port) = notification_port {
                let _ = unsafe { windows::Win32::Foundation::CloseHandle(port) };
            }
            let _ = unsafe { windows::Win32::Foundation::CloseHandle(job) };
            return Err(e);
        }
    };
    Ok(Arc::new(RunResourceJob {
        limits,
        readback,
        job,
        notification_port,
    }))
}

/// Linux has no job object without cgroups (design §3.4): record the intent and
/// report honestly that nothing is kernel-enforced.
#[cfg(not(windows))]
fn inspect_run_job(limits: JobLimits) -> io::Result<Arc<RunResourceJob>> {
    Ok(Arc::new(RunResourceJob {
        limits,
        readback: JobReadback::default(),
    }))
}

#[cfg(windows)]
pub(crate) fn windows_create_job() -> io::Result<windows::Win32::Foundation::HANDLE> {
    use std::mem::{size_of, zeroed};
    use windows::Win32::System::JobObjects::{
        CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
        JobObjectExtendedLimitInformation, SetInformationJobObject,
    };
    use windows::core::PCWSTR;

    let job = unsafe { CreateJobObjectW(None, PCWSTR::null()) }
        .map_err(|e| io::Error::other(format!("CreateJobObjectW: {e}")))?;

    // KILL_ON_JOB_CLOSE rides on every job: when orz dies, so does the tree
    // (design §4.2 — no orphans on the crash path).
    let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = unsafe { zeroed() };
    info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
    let result = unsafe {
        SetInformationJobObject(
            job,
            JobObjectExtendedLimitInformation,
            (&info as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
            size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
        )
    };
    if let Err(e) = result {
        let _ = unsafe { windows::Win32::Foundation::CloseHandle(job) };
        return Err(io::Error::other(format!("SetInformationJobObject: {e}")));
    }
    Ok(job)
}

/// Apply `limits` (plus `KILL_ON_JOB_CLOSE`) to an existing job handle.
#[cfg(windows)]
pub(crate) fn apply_limits_to_job(
    job: windows::Win32::Foundation::HANDLE,
    limits: &JobLimits,
) -> io::Result<()> {
    use std::mem::{size_of, zeroed};
    use windows::Win32::System::JobObjects::{
        JOB_OBJECT_CPU_RATE_CONTROL_ENABLE, JOB_OBJECT_CPU_RATE_CONTROL_HARD_CAP,
        JOB_OBJECT_LIMIT_ACTIVE_PROCESS, JOB_OBJECT_LIMIT_JOB_MEMORY,
        JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE, JOBOBJECT_CPU_RATE_CONTROL_INFORMATION,
        JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectCpuRateControlInformation,
        JobObjectExtendedLimitInformation, SetInformationJobObject,
    };

    let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = unsafe { zeroed() };
    info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
    if let Some(bytes) = limits.commit_limit_bytes {
        info.JobMemoryLimit = bytes as usize;
        info.BasicLimitInformation.LimitFlags |= JOB_OBJECT_LIMIT_JOB_MEMORY;
    }
    if let Some(processes) = limits.active_process {
        info.BasicLimitInformation.ActiveProcessLimit = processes;
        info.BasicLimitInformation.LimitFlags |= JOB_OBJECT_LIMIT_ACTIVE_PROCESS;
    }
    unsafe {
        SetInformationJobObject(
            job,
            JobObjectExtendedLimitInformation,
            (&info as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
            size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
        )
    }
    .map_err(|e| io::Error::other(format!("SetInformationJobObject(limits): {e}")))?;

    if let Some(percent) = limits.cpu_rate_percent {
        let mut cpu: JOBOBJECT_CPU_RATE_CONTROL_INFORMATION = unsafe { zeroed() };
        cpu.ControlFlags =
            JOB_OBJECT_CPU_RATE_CONTROL_ENABLE | JOB_OBJECT_CPU_RATE_CONTROL_HARD_CAP;
        // `CpuRate` is in hundredths of a percent.
        cpu.Anonymous.CpuRate = percent.clamp(1, 100).saturating_mul(100);
        unsafe {
            SetInformationJobObject(
                job,
                JobObjectCpuRateControlInformation,
                (&cpu as *const JOBOBJECT_CPU_RATE_CONTROL_INFORMATION).cast(),
                size_of::<JOBOBJECT_CPU_RATE_CONTROL_INFORMATION>() as u32,
            )
        }
        .map_err(|e| io::Error::other(format!("SetInformationJobObject(cpu rate): {e}")))?;
    }
    Ok(())
}

/// Read back the ceilings a job currently carries.
#[cfg(windows)]
pub(crate) fn readback_job(job: windows::Win32::Foundation::HANDLE) -> io::Result<JobReadback> {
    use std::mem::{size_of, zeroed};
    use windows::Win32::System::JobObjects::{
        JOB_OBJECT_CPU_RATE_CONTROL_ENABLE, JOB_OBJECT_LIMIT_ACTIVE_PROCESS,
        JOB_OBJECT_LIMIT_JOB_MEMORY, JOBOBJECT_CPU_RATE_CONTROL_INFORMATION,
        JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectCpuRateControlInformation,
        JobObjectExtendedLimitInformation, QueryInformationJobObject,
    };

    let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = unsafe { zeroed() };
    unsafe {
        QueryInformationJobObject(
            Some(job),
            JobObjectExtendedLimitInformation,
            (&mut info as *mut JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
            size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            None,
        )
    }
    .map_err(|e| io::Error::other(format!("QueryInformationJobObject(limits): {e}")))?;

    let flags = info.BasicLimitInformation.LimitFlags;
    let commit_limit_bytes = flags
        .contains(JOB_OBJECT_LIMIT_JOB_MEMORY)
        .then_some(info.JobMemoryLimit as u64);
    let active_process = flags
        .contains(JOB_OBJECT_LIMIT_ACTIVE_PROCESS)
        .then_some(info.BasicLimitInformation.ActiveProcessLimit);

    let mut cpu: JOBOBJECT_CPU_RATE_CONTROL_INFORMATION = unsafe { zeroed() };
    let cpu_result = unsafe {
        QueryInformationJobObject(
            Some(job),
            JobObjectCpuRateControlInformation,
            (&mut cpu as *mut JOBOBJECT_CPU_RATE_CONTROL_INFORMATION).cast(),
            size_of::<JOBOBJECT_CPU_RATE_CONTROL_INFORMATION>() as u32,
            None,
        )
    };
    let cpu_rate_percent = match cpu_result {
        Ok(())
            if cpu
                .ControlFlags
                .contains(JOB_OBJECT_CPU_RATE_CONTROL_ENABLE) =>
        {
            let rate = unsafe { cpu.Anonymous.CpuRate };
            Some((rate / 100).clamp(1, 100))
        }
        _ => None,
    };

    Ok(JobReadback {
        commit_limit_bytes,
        commit_notification_bytes: readback_commit_notification(job),
        active_process,
        cpu_rate_percent,
    })
}

/// `JOB_OBJECT_MSG_NOTIFICATION_LIMIT`（winnt.h `#define`＝11；作业消息族
/// 1..=13 之一）。完成端口上的**通知消息号**——非 win32metadata 常量，
/// 故本地定义（值经头文件现场核读）。
#[cfg(windows)]
pub const JOB_OBJECT_MSG_NOTIFICATION_LIMIT: u32 = 11;

/// 一条 commit 临限通知（内核 → 完成端口 → 宿主取件的观测事实；0bc 裁决 ②）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct JobNotification {
    /// 触发的临限阈值（字节；`0` = 内核报文里读不回——如实，不补 0 语义）。
    pub limit_bytes: u64,
    /// 触发时刻的作业 commit 读数（字节）。
    pub used_bytes: u64,
    /// 内核报的 `LimitFlags` 原值（观测留档）。
    pub limit_flags: u32,
    /// 内核报的 `ViolationLimitFlags` 原值。
    pub violation_flags: u32,
}

/// 临限通知的限位方向（0bc 真机探针，2026-09-20）：
///
/// - `Low` ＝ `JOB_OBJECT_LIMIT_JOB_MEMORY_LOW`／`JobLowMemoryLimit`；
/// - `High` ＝ `JOB_OBJECT_LIMIT_JOB_MEMORY_HIGH`／`JobHighMemoryLimit`。
///
/// 方向语义以真机为准（探针测试 `notification_semantics_probe` 的读数为
/// 准；设计裁决 ② 的应用面要求"commit 触到阈值时通知"，探针据此择向并
/// 把结论钉进测试注释）。枚举保留双值：探针与对照需要两个方向都可达。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommitNotificationMode {
    /// 低频限位（低水位方向）。
    Low,
    /// 高频限位（触到阈值方向）。
    High,
}

/// 0bc 裁决 ②（2026-09-20 用户令）：给作业装 commit 临限通知——
/// 建完成端口 → `JobObjectAssociateCompletionPortInformation` 关联 →
/// `JOBOBJECT_NOTIFICATION_LIMIT_INFORMATION_2`（`JOB_OBJECT_LIMIT_JOB_MEMORY_LOW`
/// ＋ `JobLowMemoryLimit`）。返回可轮询的端口句柄。
///
/// 与硬顶的分工：`JOB_OBJECT_LIMIT_JOB_MEMORY` 会让**分配本身**失败；
/// 通知式不改任何分配结果，只投递 `JOB_OBJECT_MSG_NOTIFICATION_LIMIT`。
/// 阈值语义由真机测试钉住（见 tests 的 `commit_notification_*`）。
#[cfg(windows)]
fn install_commit_notification(
    job: windows::Win32::Foundation::HANDLE,
    bytes: u64,
    mode: CommitNotificationMode,
) -> io::Result<windows::Win32::Foundation::HANDLE> {
    use std::mem::{size_of, zeroed};
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::System::IO::CreateIoCompletionPort;
    use windows::Win32::System::JobObjects::{
        JOB_OBJECT_LIMIT_JOB_MEMORY_HIGH, JOB_OBJECT_LIMIT_JOB_MEMORY_LOW,
        JOBOBJECT_ASSOCIATE_COMPLETION_PORT, JOBOBJECT_NOTIFICATION_LIMIT_INFORMATION_2,
        JobObjectAssociateCompletionPortInformation, JobObjectNotificationLimitInformation2,
        SetInformationJobObject,
    };

    // `INVALID_HANDLE_VALUE` ⇒ 新建端口（作业通知的文档化用法）；并发线程数
    // 1（单一非阻塞取件方，本面不派工作项）。
    let invalid = HANDLE(-1isize as *mut core::ffi::c_void);
    let port = unsafe { CreateIoCompletionPort(invalid, None, 0, 1) }
        .map_err(|e| io::Error::other(format!("CreateIoCompletionPort: {e}")))?;

    let associate = JOBOBJECT_ASSOCIATE_COMPLETION_PORT {
        CompletionKey: core::ptr::null_mut(),
        CompletionPort: port,
    };
    let assoc_result = unsafe {
        SetInformationJobObject(
            job,
            JobObjectAssociateCompletionPortInformation,
            (&associate as *const JOBOBJECT_ASSOCIATE_COMPLETION_PORT).cast(),
            size_of::<JOBOBJECT_ASSOCIATE_COMPLETION_PORT>() as u32,
        )
    };
    if let Err(e) = assoc_result {
        let _ = unsafe { windows::Win32::Foundation::CloseHandle(port) };
        return Err(io::Error::other(format!(
            "SetInformationJobObject(associate completion port): {e}"
        )));
    }

    let mut info: JOBOBJECT_NOTIFICATION_LIMIT_INFORMATION_2 = unsafe { zeroed() };
    match mode {
        CommitNotificationMode::Low => {
            info.LimitFlags = JOB_OBJECT_LIMIT_JOB_MEMORY_LOW;
            info.JobLowMemoryLimit = bytes;
        }
        CommitNotificationMode::High => {
            info.LimitFlags = JOB_OBJECT_LIMIT_JOB_MEMORY_HIGH;
            info.Anonymous1.JobHighMemoryLimit = bytes;
        }
    }
    let notify_result = unsafe {
        SetInformationJobObject(
            job,
            JobObjectNotificationLimitInformation2,
            (&info as *const JOBOBJECT_NOTIFICATION_LIMIT_INFORMATION_2).cast(),
            size_of::<JOBOBJECT_NOTIFICATION_LIMIT_INFORMATION_2>() as u32,
        )
    };
    if let Err(e) = notify_result {
        let _ = unsafe { windows::Win32::Foundation::CloseHandle(port) };
        return Err(io::Error::other(format!(
            "SetInformationJobObject(notification limit): {e}"
        )));
    }
    Ok(port)
}

/// 读回 commit 临限（内核查询面；查询不支持/未装 ⇒ `None`——不冒认）。
#[cfg(windows)]
fn readback_commit_notification(job: windows::Win32::Foundation::HANDLE) -> Option<u64> {
    use std::mem::{size_of, zeroed};
    use windows::Win32::System::JobObjects::{
        JOB_OBJECT_LIMIT_JOB_MEMORY_HIGH, JOB_OBJECT_LIMIT_JOB_MEMORY_LOW,
        JOBOBJECT_NOTIFICATION_LIMIT_INFORMATION_2, JobObjectNotificationLimitInformation2,
        QueryInformationJobObject,
    };
    let mut info: JOBOBJECT_NOTIFICATION_LIMIT_INFORMATION_2 = unsafe { zeroed() };
    let result = unsafe {
        QueryInformationJobObject(
            Some(job),
            JobObjectNotificationLimitInformation2,
            (&mut info as *mut JOBOBJECT_NOTIFICATION_LIMIT_INFORMATION_2).cast(),
            size_of::<JOBOBJECT_NOTIFICATION_LIMIT_INFORMATION_2>() as u32,
            None,
        )
    };
    if result.is_err() {
        return None;
    }
    let flags = info.LimitFlags;
    if flags.contains(JOB_OBJECT_LIMIT_JOB_MEMORY_HIGH) {
        let high = unsafe { info.Anonymous1.JobHighMemoryLimit };
        if high != 0 {
            return Some(high);
        }
    }
    if flags.contains(JOB_OBJECT_LIMIT_JOB_MEMORY_LOW) {
        return Some(info.JobLowMemoryLimit);
    }
    None
}

/// 非阻塞取件：排空完成端口队列并翻译 `JOB_OBJECT_MSG_NOTIFICATION_LIMIT`。
/// 队列空（超时）或端口错误即停止——等待下一次派发轮询。
#[cfg(windows)]
fn drain_job_notifications(
    job: windows::Win32::Foundation::HANDLE,
    port: Option<windows::Win32::Foundation::HANDLE>,
) -> Vec<JobNotification> {
    use windows::Win32::System::IO::{GetQueuedCompletionStatus, OVERLAPPED};

    let Some(port) = port else {
        return Vec::new();
    };
    let mut out = Vec::new();
    loop {
        let mut transferred: u32 = 0;
        let mut key: usize = 0;
        let mut overlapped: *mut OVERLAPPED = core::ptr::null_mut();
        let result = unsafe {
            GetQueuedCompletionStatus(port, &mut transferred, &mut key, &mut overlapped, 0)
        };
        match result {
            Ok(()) => {
                // 0bc 只收 commit 临限；进程增删/CPU 限等其它作业消息不属本面。
                if transferred == JOB_OBJECT_MSG_NOTIFICATION_LIMIT
                    && let Some(notification) = query_commit_violation(job)
                {
                    out.push(notification);
                }
            }
            // 超时（队列空）或端口错误：停止取件（错误不吞事实——下一轮再试）。
            Err(_) => break,
        }
    }
    out
}

/// 通知详情：`JOBOBJECT_LIMIT_VIOLATION_INFORMATION(_2)` 查询（v2 优先，
/// v1 兜底——`JobLowMemoryLimit` 只存在于 v2/通知限位族）。
#[cfg(windows)]
fn query_commit_violation(job: windows::Win32::Foundation::HANDLE) -> Option<JobNotification> {
    use std::mem::{size_of, zeroed};
    use windows::Win32::System::JobObjects::{
        JOBOBJECT_LIMIT_VIOLATION_INFORMATION, JOBOBJECT_LIMIT_VIOLATION_INFORMATION_2,
        JobObjectLimitViolationInformation, JobObjectLimitViolationInformation2,
        QueryInformationJobObject,
    };

    let mut info2: JOBOBJECT_LIMIT_VIOLATION_INFORMATION_2 = unsafe { zeroed() };
    let v2 = unsafe {
        QueryInformationJobObject(
            Some(job),
            JobObjectLimitViolationInformation2,
            (&mut info2 as *mut JOBOBJECT_LIMIT_VIOLATION_INFORMATION_2).cast(),
            size_of::<JOBOBJECT_LIMIT_VIOLATION_INFORMATION_2>() as u32,
            None,
        )
    };
    if v2.is_ok() {
        let high = unsafe { info2.Anonymous1.JobHighMemoryLimit };
        let low = info2.JobLowMemoryLimit;
        // 装了 LOW 限位时低限成员是触发的那个；防呆：低限为 0 时取高限。
        let limit_bytes = if low != 0 { low } else { high };
        return Some(JobNotification {
            limit_bytes,
            used_bytes: info2.JobMemory,
            limit_flags: info2.LimitFlags.0,
            violation_flags: info2.ViolationLimitFlags.0,
        });
    }

    let mut info: JOBOBJECT_LIMIT_VIOLATION_INFORMATION = unsafe { zeroed() };
    let v1 = unsafe {
        QueryInformationJobObject(
            Some(job),
            JobObjectLimitViolationInformation,
            (&mut info as *mut JOBOBJECT_LIMIT_VIOLATION_INFORMATION).cast(),
            size_of::<JOBOBJECT_LIMIT_VIOLATION_INFORMATION>() as u32,
            None,
        )
    };
    if v1.is_ok() {
        return Some(JobNotification {
            limit_bytes: info.JobMemoryLimit,
            used_bytes: info.JobMemory,
            limit_flags: info.LimitFlags.0,
            violation_flags: info.ViolationLimitFlags.0,
        });
    }
    None
}

/// 取件当前全局 run job 的临限通知（无 run job ⇒ 空；宿主派发面调用）。
pub fn drain_global_run_notifications() -> Vec<JobNotification> {
    global_run_job()
        .map(|job| job.drain_notifications())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Serializes every test that installs/replaces the process-wide run job or
    /// spawns a bounded child: the slot is process-wide by design, so parallel
    /// tests would otherwise observe each other's ceilings.
    static TOPOLOGY_LOCK: Mutex<()> = Mutex::new(());

    fn topology_lock() -> std::sync::MutexGuard<'static, ()> {
        TOPOLOGY_LOCK
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
    }

    #[test]
    fn empty_limits_describe_unlimited() {
        let limits = JobLimits::default();
        assert!(limits.is_empty());
        assert_eq!(
            limits.describe(),
            "commit=unlimited commit_notify=unlimited active_process=unlimited cpu_rate=unlimited"
        );
    }

    #[test]
    fn limits_describe_renders_each_axis() {
        let limits = JobLimits {
            commit_limit_bytes: Some(2 * 1024 * 1024 * 1024),
            commit_notification_bytes: Some(8 * 1024 * 1024 * 1024),
            active_process: Some(16),
            cpu_rate_percent: Some(80),
        };
        assert!(!limits.is_empty());
        assert_eq!(
            limits.describe(),
            "commit=2.00 GiB commit_notify=8.00 GiB active_process=16 cpu_rate=80%"
        );
    }

    /// 判据 12 的机械面（S1 部分）：内核接受并回报我们设置的三条上限。
    #[cfg(windows)]
    #[test]
    fn windows_job_limits_are_kernel_visible_after_install() {
        let _guard = topology_lock();
        let limits = JobLimits {
            commit_limit_bytes: Some(1_500_000_000),
            commit_notification_bytes: None,
            active_process: Some(7),
            cpu_rate_percent: Some(80),
        };
        let job = RunResourceJob::install(limits).expect("install run ceilings");
        assert!(job.is_kernel_enforced(), "readback must show the limits");
        let readback = job.readback();
        // The kernel ROUNDS the commit ceiling down to commit/page granularity
        // (observed: 1_500_000_000 → 1_499_996_160), so the read-back is the
        // authority and the check tolerates one page of slack below the ask.
        let commit = readback.commit_limit_bytes.expect("commit ceiling");
        assert!(
            (1_500_000_000 - 65_536..=1_500_000_000).contains(&commit),
            "commit ceiling must be the request rounded down by at most a page, got {commit}"
        );
        assert_eq!(readback.active_process, Some(7));
        assert_eq!(readback.cpu_rate_percent, Some(80));
        assert_eq!(job.limits(), limits);
        assert!(job.describe().contains("kernel_enforced=true"));
    }

    /// 判据 12：一条上限轴都没有时，句柄必须自报"未强制"，不得假装生效。
    #[cfg(windows)]
    #[test]
    fn windows_empty_limits_report_not_enforced() {
        let _guard = topology_lock();
        let job = RunResourceJob::install(JobLimits::default()).expect("install run ceilings");
        assert!(!job.is_kernel_enforced());
        assert!(!job.readback().is_enforced());
        assert!(job.describe().contains("kernel_enforced=false"));
    }

    /// 0bc 裁决 ②（2026-09-20）：临限通知阈值装置后内核可见，且它**不属
    /// 天花板轴**——没有任何强制轴时句柄仍自报"未强制"（通知是观测面）。
    #[cfg(windows)]
    #[test]
    fn commit_notification_round_trips_through_readback() {
        let _guard = topology_lock();
        let threshold = 256 * 1024 * 1024;
        let job = RunResourceJob::install(JobLimits {
            commit_limit_bytes: None,
            commit_notification_bytes: Some(threshold),
            active_process: None,
            cpu_rate_percent: None,
        })
        .expect("install run job with a notification only");
        assert!(
            job.notification_installed(),
            "notification face must be wired"
        );
        assert_eq!(
            job.readback().commit_notification_bytes,
            Some(threshold),
            "readback must show the notification threshold (or stay None honestly)"
        );
        assert!(
            !job.is_kernel_enforced(),
            "a notification is observation, not a ceiling"
        );
        assert!(job.describe().contains("commit_notify=0.25 GiB"));
        // 没有子进程 ⇒ 队列空：非阻塞取件立即返回。
        assert!(job.drain_notifications().is_empty());
    }

    /// 0bc 探针（S1 勘定的真机面）：commit 临限通知的**方向语义**——LOW 与
    /// HIGH 两个限位在"装置当刻"与"上行跨越阈值"两个场景下的行为对照。
    /// 结论钉生产方向 [`COMMIT_NOTIFICATION_MODE`]（打印读数；`--nocapture`）。
    #[cfg(windows)]
    #[test]
    fn notification_semantics_probe() {
        let _guard = topology_lock();
        let threshold = 256 * 1024 * 1024;

        // E1: HIGH、无子进程——装置当刻应无消息。
        let high_job = RunResourceJob::install_with_notification_mode(
            JobLimits {
                commit_limit_bytes: None,
                commit_notification_bytes: Some(threshold),
                active_process: None,
                cpu_rate_percent: None,
            },
            CommitNotificationMode::High,
        )
        .expect("install HIGH notification");
        let e1 = high_job.drain_notifications();
        println!(
            "E1 HIGH no-process immediate drain: {} msg {e1:?}",
            e1.len()
        );

        // E3: LOW、无子进程——对照（若"低于限位"方向成真，这里应即时有件）。
        let low_job = RunResourceJob::install_with_notification_mode(
            JobLimits {
                commit_limit_bytes: None,
                commit_notification_bytes: Some(threshold),
                active_process: None,
                cpu_rate_percent: None,
            },
            CommitNotificationMode::Low,
        )
        .expect("install LOW notification");
        let e3 = low_job.drain_notifications();
        println!("E3 LOW no-process immediate drain: {} msg {e3:?}", e3.len());
        drop(low_job);

        // E2: HIGH + 1 GiB 子进程——预期在 commit 上行跨过阈值时点火。
        let run = replace_global_run_job_for_tests_with_mode(
            JobLimits {
                commit_limit_bytes: None,
                commit_notification_bytes: Some(threshold),
                active_process: None,
                cpu_rate_percent: None,
            },
            CommitNotificationMode::High,
        )
        .expect("install HIGH notification on the global run job");
        let mut group = crate::ProcessGroup::new().expect("per-call job");
        let mut child = spawn_over_committing_child();
        group.attach_std(&child).expect("attach committing child");
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
        let mut seen: Vec<JobNotification> = Vec::new();
        let mut child_alive_when_first_seen = false;
        while seen.is_empty() && std::time::Instant::now() < deadline {
            seen.extend(run.drain_notifications());
            if seen.is_empty() {
                std::thread::sleep(std::time::Duration::from_millis(50));
            } else {
                child_alive_when_first_seen = child.try_wait().ok().flatten().is_none();
            }
        }
        println!(
            "E2 HIGH + committing child: {} msg {seen:?} child_alive_when_first_seen={child_alive_when_first_seen}",
            seen.len()
        );
        let output = cmd_output(child);
        println!("E2 child tail: {}", output.lines().last().unwrap_or(""));

        // 方向勘定（真机数据，2026-09-20 首航 ＋ 复核运行）：
        // - 两个方向在**装置当刻**都静默（无子进程 ⇒ 无越限事件）；
        // - HIGH 限位在子进程把 commit 推过阈值后投递一条，violation flags
        //   报 0x200＝HIGH 被越——即"超过上限"方向（本用所需：临限＝触到）；
        //   投递有延迟（取件时子进程可能已退出；报告里的 JobMemory 是取回
        //   时刻的读数，不是越限当刻）——生产方向
        //   [`COMMIT_NOTIFICATION_MODE`] 由此钉为 `High`。
        assert!(e1.is_empty(), "install alone must not post a notification");
        // LOW 方向不在本用；其装置语义按真机读数打印保留（不作断言）。
        assert!(
            !seen.is_empty(),
            "HIGH must fire when the child's commit climbs past the threshold"
        );
        assert_eq!(
            seen[0].limit_flags & 0x200,
            0x200,
            "the violation report must name the high-memory limit: {:?}",
            seen[0]
        );
    }

    /// 0bc 裁决 ② 真机钉：commit 跨过临限阈值时，完成端口投递
    /// `JOB_OBJECT_MSG_NOTIFICATION_LIMIT`——宿主取件拿到读数（阈值、当时的
    /// 作业 commit、内核旗标原值）。子进程照常完成（通知不改分配结果）。
    #[cfg(windows)]
    #[test]
    fn commit_notification_fires_on_the_completion_port() {
        let _guard = topology_lock();
        let threshold = 64 * 1024 * 1024;
        let run = crate::replace_global_run_job_for_tests(JobLimits {
            commit_limit_bytes: None,
            commit_notification_bytes: Some(threshold),
            active_process: None,
            cpu_rate_percent: None,
        })
        .expect("install notification-only run job");

        let mut group = crate::ProcessGroup::new().expect("per-call job");
        let child = spawn_over_committing_child();
        group.attach_std(&child).expect("attach committing child");

        // PowerShell 冷启动可达数秒：非阻塞轮询直到有件或超时。
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
        let mut seen = Vec::new();
        while seen.is_empty() && std::time::Instant::now() < deadline {
            seen.extend(run.drain_notifications());
            if seen.is_empty() {
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
        }
        let output = cmd_output(child);
        assert!(
            !seen.is_empty(),
            "commit crossing the notification limit must deliver a message; child said: {output}"
        );
        let notification = seen[0];
        assert_eq!(
            notification.limit_bytes, threshold,
            "violation report must name the configured threshold: {notification:?}"
        );
        assert!(
            notification.used_bytes > 0,
            "violation report must carry a reading: {notification:?}"
        );
        // 通知式 ≠ 硬顶：1 GiB 的 commit 在无 commit 硬顶时照常成功。
        assert!(
            output.contains("COMMITTED"),
            "notification must not block the allocation; child said: {output}"
        );
    }

    /// 0bc 负对照：子进程峰值（≈1 GiB）**低于**阈值（4 GiB）⇒ 两个方向都
    /// 无跨越事件，完成端口全程静默（装置当刻也无件——这是"越限事件触发、
    /// 不是状态轮询"的钉子）。覆盖子进程全程（linger 5 秒）＋ 退出后 3 秒。
    #[cfg(windows)]
    #[test]
    fn commit_notification_stays_silent_below_the_threshold() {
        let _guard = topology_lock();
        let run = crate::replace_global_run_job_for_tests(JobLimits {
            commit_limit_bytes: None,
            commit_notification_bytes: Some(4 * 1024 * 1024 * 1024),
            active_process: None,
            cpu_rate_percent: None,
        })
        .expect("install run job with a 4 GiB notification threshold");
        assert!(
            run.drain_notifications().is_empty(),
            "install alone must not post a notification"
        );

        let mut group = crate::ProcessGroup::new().expect("per-call job");
        let child = spawn_lingering_committing_child();
        group.attach_std(&child).expect("attach committing child");

        let mut seen = Vec::new();
        // 子进程全程（≈5 秒 linger）：轮询取件。
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while std::time::Instant::now() < deadline {
            seen.extend(run.drain_notifications());
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        let output = cmd_output(child);
        // 退出后的余量窗口（投递有延迟，给 3 秒）。
        let after = std::time::Instant::now() + std::time::Duration::from_secs(3);
        while std::time::Instant::now() < after {
            seen.extend(run.drain_notifications());
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        assert!(
            seen.is_empty(),
            "no commit limit was crossed; the port must stay silent — child: {output}, got: {seen:?}"
        );
    }

    /// 提交 1 GiB 后驻留 5 秒（把"上行越限当刻"与"退出下行"分开观察）。
    #[cfg(windows)]
    fn spawn_lingering_committing_child() -> std::process::Child {
        use std::process::Stdio;

        let mut cmd = std::process::Command::new("powershell");
        cmd.args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "$a = New-Object byte[] 1073741824; Write-Output 'COMMITTED'; Start-Sleep -Seconds 5",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
        cmd.spawn().expect("spawn lingering over-committing child")
    }

    /// 两级拓扑（复核 F-1）：调用级 job 只带 `KILL_ON_JOB_CLOSE`，限项在 run 级
    /// job 上；`attach` 之后子进程确实**同时**在 run job 内（先根后子），
    /// `readback_limits()` 报的是这条调用实际受制的限项。
    #[cfg(windows)]
    #[test]
    fn per_call_group_nests_under_the_run_job_and_reports_its_ceilings() {
        let _guard = topology_lock();
        let run = crate::replace_global_run_job_for_tests(JobLimits {
            commit_limit_bytes: Some(3_000_000_000),
            commit_notification_bytes: None,
            active_process: Some(11),
            cpu_rate_percent: Some(80),
        })
        .expect("install run ceilings");

        let mut group = crate::ProcessGroup::new().expect("per-call job");
        // The call job itself must NOT duplicate the ceilings — that duplication
        // was the S1 shape the review withdrew (N concurrent calls each holding a
        // full copy instead of one run-wide bound).
        assert!(
            !group
                .readback_call_job_limits()
                .expect("call job readback")
                .is_enforced(),
            "the per-call job carries teardown, not the run ceilings"
        );

        let readback = group.readback_limits().expect("effective readback");
        let commit = readback.commit_limit_bytes.expect("run commit ceiling");
        assert!(
            (3_000_000_000 - 65_536..=3_000_000_000).contains(&commit),
            "effective commit ceiling came back as {commit}"
        );
        assert_eq!(readback.active_process, Some(11));
        assert_eq!(readback.cpu_rate_percent, Some(80));
        assert_eq!(run.readback(), readback);

        // End-to-end at this layer: a real child of this group is inside the run
        // job (design §6 判据 12's mechanical face).
        let mut cmd = std::process::Command::new("cmd");
        cmd.args(["/c", "ping -n 8 127.0.0.1 > NUL"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        let mut child = cmd.spawn().expect("spawn a long-lived child");
        group.attach_std(&child).expect("attach child");
        assert!(
            run.contains_process(child.id()).expect("query run job"),
            "the tool child must be associated with the run job"
        );
        let _ = child.kill();
        let _ = child.wait();
    }

    /// 复核 F-1 的执行面：**run 级** commit 上限对「调用级 job 的子进程」同样
    /// 咬合 —— 调用侧一条显式限项都不给，1 GiB 提交仍必须被拒。
    #[cfg(windows)]
    #[test]
    fn run_job_ceiling_bounds_a_child_of_a_per_call_group() {
        let _guard = topology_lock();
        crate::replace_global_run_job_for_tests(JobLimits {
            commit_limit_bytes: Some(300 * 1024 * 1024),
            commit_notification_bytes: None,
            active_process: None,
            cpu_rate_percent: None,
        })
        .expect("install a small run ceiling");

        let mut group = crate::ProcessGroup::new().expect("per-call job");
        let child = spawn_over_committing_child();
        group.attach_std(&child).expect("attach child");
        let output = cmd_output(child);
        assert!(
            output.contains("OutOfMemoryException"),
            "the run-level ceiling must bound the call's tree; child said: {output}"
        );
    }

    /// 判据 12 的执行面：调用级显式限项仍然自己咬合（显式覆盖通道未被拓扑改动
    /// 削弱）——子进程请求 1 GiB，调用级只给 256 MiB，分配必须失败。
    #[cfg(windows)]
    #[test]
    fn commit_ceiling_actually_stops_an_over_committing_child() {
        let _guard = topology_lock();
        let mut group = crate::ProcessGroup::new_with_limits(JobLimits {
            commit_limit_bytes: Some(256 * 1024 * 1024),
            commit_notification_bytes: None,
            active_process: None,
            cpu_rate_percent: None,
        })
        .expect("per-call job with a commit ceiling");
        assert!(
            group
                .readback_call_job_limits()
                .expect("call job readback")
                .is_enforced(),
            "explicit limits live on the call job"
        );
        let child = spawn_over_committing_child();
        group
            .attach_std(&child)
            .expect("attach child to the bounded job");
        let output = cmd_output(child);
        assert!(
            output.contains("OutOfMemoryException"),
            "the job commit ceiling must refuse the 1 GiB commit; child said: {output}"
        );
    }

    /// PowerShell commits the array on assignment; under any commit ceiling a
    /// 1 GiB array cannot be committed. Piped on purpose: inherited stdio would
    /// make the assertions above vacuous (observed while writing this test).
    #[cfg(windows)]
    fn spawn_over_committing_child() -> std::process::Child {
        use std::process::Stdio;

        let mut cmd = std::process::Command::new("powershell");
        cmd.args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "$a = New-Object byte[] 1073741824; Write-Output 'COMMITTED'",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
        cmd.spawn().expect("spawn over-committing child")
    }

    #[cfg(windows)]
    fn cmd_output(child: std::process::Child) -> String {
        let out = child.wait_with_output().expect("wait for the probe child");
        let mut text = String::from_utf8_lossy(&out.stdout).into_owned();
        text.push_str(&String::from_utf8_lossy(&out.stderr));
        text
    }

    /// 0z S2R F-BE-3(a)（2026-09-13 用户裁决，per-call 杀面）：复制调用级
    /// job 句柄 → TerminateJobObject(dup) → **该调用**的子进程树整树死亡，
    /// 而 job 外的无辜子进程存活——爆半径 = 单调用树的内核级证明。
    #[cfg(windows)]
    #[test]
    fn per_call_job_terminate_kills_its_tree_and_spares_outsiders() {
        let _guard = topology_lock();
        let mut child = std::process::Command::new("cmd")
            .args(["/c", "ping -n 30 127.0.0.1 > NUL"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("spawn the in-call probe child");
        let mut outsider = std::process::Command::new("cmd")
            .args(["/c", "ping -n 30 127.0.0.1 > NUL"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("spawn the outside probe child");

        let mut group = crate::ProcessGroup::new().expect("per-call job");
        group.attach_std(&child).expect("attach the in-call child");

        // 派发面拿到复制句柄（SpawnObservation 的同一路径）。
        let dup = group
            .duplicate_call_job_handle()
            .expect("duplicate call job");
        assert!(dup != 0, "the duplicated handle must be non-sentinel");

        crate::resource_job::terminate_job_handle(dup).expect("terminate the call job");

        // in-call 子进程死亡（整树粒度由内核保证）。注意：Windows 上被
        // 终止但未 reap 的进程对象仍可 OpenProcess 成功——对持有的 Child
        // 用 try_wait 判定退出（我们持有句柄，语义精确）。
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        let mut in_call_dead = false;
        while std::time::Instant::now() < deadline {
            if matches!(child.try_wait(), Ok(Some(_))) {
                in_call_dead = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        assert!(in_call_dead, "the in-call child must die with its call job");
        // job 外子进程存活（爆半径不越界），随后清理。
        assert!(
            matches!(outsider.try_wait(), Ok(None)),
            "the outsider must NOT be in the call job's blast radius"
        );
        let _ = outsider.kill();
        let _ = outsider.wait();
    }

    /// 复核 F-9：attach 失败必须有**机械可见面**（单调计数），不能只留一行日志
    /// ——否则"限项没生效"这件事在真机上完全不可观测。
    #[test]
    fn attach_failures_are_counted() {
        let before = attach_failure_count();
        record_attach_failure();
        assert!(
            attach_failure_count() >= before + 1,
            "the counter must be monotonic"
        );
    }

    /// Non-Windows records the intent and reports it as not kernel-enforced.
    #[cfg(not(windows))]
    #[test]
    fn non_windows_limits_report_not_enforced() {
        let _guard = topology_lock();
        let job = RunResourceJob::install(JobLimits {
            commit_limit_bytes: Some(1024),
            commit_notification_bytes: None,
            active_process: Some(1),
            cpu_rate_percent: Some(50),
        })
        .expect("install run ceilings");
        assert!(!job.is_kernel_enforced());
    }
}
