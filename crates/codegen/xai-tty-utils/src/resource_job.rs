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
    /// Static concurrent-process ceiling for the job tree.
    pub active_process: Option<u32>,
    /// CPU rate ceiling per scheduling interval, in percent (1..=100).
    pub cpu_rate_percent: Option<u32>,
}

impl JobLimits {
    /// True when no axis is limited (nothing to enforce).
    pub fn is_empty(&self) -> bool {
        self.commit_limit_bytes.is_none()
            && self.active_process.is_none()
            && self.cpu_rate_percent.is_none()
    }

    /// One-line mechanical rendering (audit / observation text).
    pub fn describe(&self) -> String {
        let commit = self
            .commit_limit_bytes
            .map(|b| format!("{:.2} GiB", b as f64 / (1024.0 * 1024.0 * 1024.0)))
            .unwrap_or_else(|| "unlimited".to_string());
        let procs = self
            .active_process
            .map(|n| n.to_string())
            .unwrap_or_else(|| "unlimited".to_string());
        let cpu = self
            .cpu_rate_percent
            .map(|n| format!("{n}%"))
            .unwrap_or_else(|| "unlimited".to_string());
        format!("commit={commit} active_process={procs} cpu_rate={cpu}")
    }
}

/// What the kernel reports back after the ceilings were set — the read-back
/// face of "the limit is really in force" (design §6 判据 12).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct JobReadback {
    pub commit_limit_bytes: Option<u64>,
    pub active_process: Option<u32>,
    pub cpu_rate_percent: Option<u32>,
}

impl JobReadback {
    /// True when at least one ceiling came back from the kernel.
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
}

#[cfg(windows)]
impl Drop for RunResourceJob {
    fn drop(&mut self) {
        // Closing the handle is what fires `KILL_ON_JOB_CLOSE`. Production keeps
        // the run job in the global slot for the process lifetime, so this runs
        // at process exit (or when a test replaces the handle).
        let _ = unsafe { windows::Win32::Foundation::CloseHandle(self.job) };
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
#[cfg(windows)]
fn inspect_run_job(limits: JobLimits) -> io::Result<Arc<RunResourceJob>> {
    let job = windows_create_job()?;
    if let Err(e) = apply_limits_to_job(job, &limits) {
        let _ = unsafe { windows::Win32::Foundation::CloseHandle(job) };
        return Err(e);
    }
    let readback = match readback_job(job) {
        Ok(readback) => readback,
        Err(e) => {
            let _ = unsafe { windows::Win32::Foundation::CloseHandle(job) };
            return Err(e);
        }
    };
    Ok(Arc::new(RunResourceJob {
        limits,
        readback,
        job,
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
        active_process,
        cpu_rate_percent,
    })
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
            "commit=unlimited active_process=unlimited cpu_rate=unlimited"
        );
    }

    #[test]
    fn limits_describe_renders_each_axis() {
        let limits = JobLimits {
            commit_limit_bytes: Some(2 * 1024 * 1024 * 1024),
            active_process: Some(16),
            cpu_rate_percent: Some(80),
        };
        assert!(!limits.is_empty());
        assert_eq!(
            limits.describe(),
            "commit=2.00 GiB active_process=16 cpu_rate=80%"
        );
    }

    /// 判据 12 的机械面（S1 部分）：内核接受并回报我们设置的三条上限。
    #[cfg(windows)]
    #[test]
    fn windows_job_limits_are_kernel_visible_after_install() {
        let _guard = topology_lock();
        let limits = JobLimits {
            commit_limit_bytes: Some(1_500_000_000),
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

    /// 两级拓扑（复核 F-1）：调用级 job 只带 `KILL_ON_JOB_CLOSE`，限项在 run 级
    /// job 上；`attach` 之后子进程确实**同时**在 run job 内（先根后子），
    /// `readback_limits()` 报的是这条调用实际受制的限项。
    #[cfg(windows)]
    #[test]
    fn per_call_group_nests_under_the_run_job_and_reports_its_ceilings() {
        let _guard = topology_lock();
        let run = crate::replace_global_run_job_for_tests(JobLimits {
            commit_limit_bytes: Some(3_000_000_000),
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
            active_process: Some(1),
            cpu_rate_percent: Some(50),
        })
        .expect("install run ceilings");
        assert!(!job.is_kernel_enforced());
    }
}
