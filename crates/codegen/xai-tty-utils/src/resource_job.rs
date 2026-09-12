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
//! # Scope: per tool call, decided per run (S1 finding, 2026-09-12)
//!
//! Design §4.7 described two levels — one run-level job carrying the ceilings
//! plus one per-call job nested inside it. The nesting half does not exist on
//! this platform in the form we need: `AssignProcessToJobObject(job, job)`
//! returns `ERROR_INVALID_HANDLE` (reproduced), and the documented nesting
//! routes are implicit rather than explicit — a job nested in another job is
//! created by a process that *is itself* in the outer job. That would require
//! putting `orz.exe` into the job, which caps orz's own commit and lets the
//! job's `KILL_ON_JOB_CLOSE` kill the agent — strictly worse than the failure
//! it prevents (§4.8 rejected per-process limits for the same class of reason).
//!
//! So S1 enforces the ceilings on **every per-call job** while keeping
//! **one** run-level decision: [`install_global_run_job`] takes the ceilings
//! once, proves the kernel accepts them (read-back, §6 判据 12), and every
//! [`crate::ProcessGroup`] created afterwards carries them. One heavy call —
//! the shape of both 2026-09-12 incidents — is bounded exactly as designed.
//! The residual (N *concurrent* calls each get their own ceiling rather than
//! one shared aggregate) is registered for S2, where the process-tree
//! lifecycle (design §4.2/B) owns the hierarchy question.
//!
//! **A job cannot bound disk.** Volume headroom stays with the pre-dispatch
//! gate and the reclaim ladder (design §4.7 盘侧例外).
//!
//! Linux has no per-job equivalent without cgroups (which the run deliberately
//! does not require), so the handle records the limits and reports
//! `enforced: false` — an honest reading, never a silent claim of enforcement.

use std::io;
use std::sync::{Arc, OnceLock};

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

/// The run's ceiling decision plus the kernel's read-back of it.
pub struct RunResourceJob {
    limits: JobLimits,
    readback: JobReadback,
}

impl RunResourceJob {
    /// Install the ceilings and read back what the kernel accepted.
    ///
    /// The ceilings themselves live on the per-call jobs ([`crate::ProcessGroup`]);
    /// this handle is the run-level decision + its proof. An axis the kernel
    /// refuses shows up as `None` in [`Self::readback`], so `is_kernel_enforced`
    /// never over-claims.
    pub fn install(limits: JobLimits) -> io::Result<Arc<Self>> {
        let readback = probe_readback(&limits)?;
        Ok(Arc::new(Self { limits, readback }))
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
}

static GLOBAL_RUN_JOB: OnceLock<Arc<RunResourceJob>> = OnceLock::new();

/// The process-wide run ceiling decision, when one was installed.
///
/// orz serves one run per process (the `-p` and ACP paths both build the host
/// once at assembly), so process lifetime is the run lifetime this handle
/// tracks. Every [`crate::ProcessGroup`] created afterwards carries these
/// ceilings — see [`crate::ProcessGroup::new`].
pub fn global_run_job() -> Option<Arc<RunResourceJob>> {
    GLOBAL_RUN_JOB.get().cloned()
}

/// Install the process-wide ceilings. First caller wins (later calls return the
/// existing handle): the ceilings are a run-level contract, not a per-call
/// tuning knob.
pub fn install_global_run_job(limits: JobLimits) -> io::Result<Arc<RunResourceJob>> {
    if let Some(existing) = GLOBAL_RUN_JOB.get() {
        return Ok(existing.clone());
    }
    let job = RunResourceJob::install(limits)?;
    let _ = GLOBAL_RUN_JOB.set(job.clone());
    Ok(job)
}

// ---------------------------------------------------------------------------
// Windows implementation
// ---------------------------------------------------------------------------

/// Create a job with `KILL_ON_JOB_CLOSE` + `limits` and read back what stuck.
#[cfg(windows)]
fn probe_readback(limits: &JobLimits) -> io::Result<JobReadback> {
    let job = windows_create_job()?;
    let applied = apply_limits_to_job(job, limits);
    let readback = applied.and_then(|()| readback_job(job));
    let _ = unsafe { windows::Win32::Foundation::CloseHandle(job) };
    readback
}

#[cfg(not(windows))]
fn probe_readback(_limits: &JobLimits) -> io::Result<JobReadback> {
    Ok(JobReadback::default())
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
        let job = RunResourceJob::install(JobLimits::default()).expect("install run ceilings");
        assert!(!job.is_kernel_enforced());
        assert!(!job.readback().is_enforced());
        assert!(job.describe().contains("kernel_enforced=false"));
    }

    /// The per-call job the spawn path actually uses carries the run ceilings —
    /// this is the seam that makes the ceiling real for a heavy tool call.
    #[cfg(windows)]
    #[test]
    fn per_call_job_inherits_the_run_ceilings() {
        let run = crate::install_global_run_job(JobLimits {
            commit_limit_bytes: Some(3_000_000_000),
            active_process: Some(11),
            cpu_rate_percent: Some(80),
        })
        .expect("install run ceilings");
        let group = crate::ProcessGroup::new().expect("per-call job");
        let readback = group.readback_limits().expect("readback");
        let commit = readback
            .commit_limit_bytes
            .expect("commit ceiling on the call job");
        assert!(
            (3_000_000_000 - 65_536..=3_000_000_000).contains(&commit),
            "call job commit ceiling came back as {commit}"
        );
        assert_eq!(readback.active_process, Some(11));
        assert_eq!(readback.cpu_rate_percent, Some(80));
        assert_eq!(
            run.readback(),
            readback,
            "the run decision is what the call job carries"
        );
    }

    /// 判据 12 的执行面：job 的 commit 上限真的挡住超额提交 —— 子进程请求
    /// 1 GiB，job 只给 256 MiB，分配必须失败。
    #[cfg(windows)]
    #[test]
    fn commit_ceiling_actually_stops_an_over_committing_child() {
        use std::process::Stdio;

        let mut group = crate::ProcessGroup::new_with_limits(JobLimits {
            commit_limit_bytes: Some(256 * 1024 * 1024),
            active_process: None,
            cpu_rate_percent: None,
        })
        .expect("per-call job with a commit ceiling");
        // PowerShell commits the array on assignment; with a 256 MiB job
        // ceiling a 1 GiB array cannot be committed.
        let mut cmd = std::process::Command::new("powershell");
        cmd.args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "$a = New-Object byte[] 1073741824; Write-Output 'COMMITTED'",
        ])
        // Piped on purpose: inherited stdio would make the assertion below
        // vacuous (observed while writing this test).
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
        let child = cmd.spawn().expect("spawn over-committing child");
        group
            .attach_std(&child)
            .expect("attach child to the bounded job");
        let output = cmd_output(child);
        assert!(
            output.contains("OutOfMemoryException"),
            "the job commit ceiling must refuse the 1 GiB commit; child said: {output}"
        );
    }

    #[cfg(windows)]
    fn cmd_output(child: std::process::Child) -> String {
        let out = child.wait_with_output().expect("wait for the probe child");
        let mut text = String::from_utf8_lossy(&out.stdout).into_owned();
        text.push_str(&String::from_utf8_lossy(&out.stderr));
        text
    }

    /// Non-Windows records the intent and reports it as not kernel-enforced.
    #[cfg(not(windows))]
    #[test]
    fn non_windows_limits_report_not_enforced() {
        let job = RunResourceJob::install(JobLimits {
            commit_limit_bytes: Some(1024),
            active_process: Some(1),
            cpu_rate_percent: Some(50),
        })
        .expect("install run ceilings");
        assert!(!job.is_kernel_enforced());
    }
}
