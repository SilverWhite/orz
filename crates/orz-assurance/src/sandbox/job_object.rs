//! Windows Job Object containment (GAK-WIN-001 pattern, kill-on-close).
//!
//! The supervisor owns a Job Object with `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`:
//! when the last handle to the job closes, the kernel terminates every
//! process inside it. Dropping the supervisor (or calling [`close`]) therefore
//! kills the whole contained process tree.
//!
//! [`JobObjectSupervisor::spawn_contained`] spawns a child suspended
//! (`CREATE_SUSPENDED`), binds it into the job, then resumes its main thread —
//! the race-free containment pattern documented in
//! `docs/JOB_OBJECT_CONTAINMENT_SUFFICIENCY_JUDGMENT_2026-07-31.md` (equivalent
//! to `PROC_THREAD_ATTRIBUTE_JOB_LIST`; the attribute list is only a code-quality
//! optimisation, not a security requirement).
//!
//! Non-Windows targets: the supervisor is a fail-closed stub — every operation
//! returns [`SandboxError::UnsupportedPlatform`]. Containment on those
//! platforms is provided by the platform-native sandbox (orz-sandbox).
//!
//! Known boundary: when the *caller* is itself already inside a job object
//! (e.g. a CI runner) and that job does not allow breakaway,
//! [`JobObjectSupervisor::assign_process`] fails with a Windows error — the
//! supervisor reports the failure instead of silently running uncontained
//! (fail-closed).

use std::process::{Child, Command};
use std::sync::Mutex;

/// Errors from the job object supervisor.
#[derive(Debug, thiserror::Error)]
pub enum SandboxError {
    /// Containment is not available on this platform (fail-closed).
    #[error("job object containment is not available on this platform")]
    UnsupportedPlatform,

    /// A Win32 call failed; message string so the error type compiles everywhere.
    #[error("windows job object error: {0}")]
    Windows(String),

    /// Underlying process/io error.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// Could not resume the contained child's main thread.
    #[error("failed to resume contained process {pid}: {reason}")]
    ResumeFailed { pid: u32, reason: String },
}

/// Kill-on-close Job Object supervisor.
#[derive(Debug)]
pub struct JobObjectSupervisor {
    #[cfg(windows)]
    job: Mutex<Option<std::os::windows::io::OwnedHandle>>,
    #[cfg(not(windows))]
    _not_windows: (),
}

impl JobObjectSupervisor {
    /// Create a Job Object with `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`.
    pub fn new() -> Result<Self, SandboxError> {
        #[cfg(windows)]
        {
            use std::mem::size_of;
            use std::os::windows::io::FromRawHandle;
            use windows::Win32::System::JobObjects::{
                CreateJobObjectW, JobObjectExtendedLimitInformation, SetInformationJobObject,
                JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
            };

            unsafe {
                let job = CreateJobObjectW(None, None)
                    .map_err(|e| SandboxError::Windows(format!("CreateJobObjectW: {e}")))?;
                let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
                info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
                let info_ptr = &info as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION as *const core::ffi::c_void;
                if let Err(e) = SetInformationJobObject(
                    job,
                    JobObjectExtendedLimitInformation,
                    info_ptr,
                    size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
                ) {
                    let _ = windows::Win32::Foundation::CloseHandle(job);
                    return Err(SandboxError::Windows(format!("SetInformationJobObject: {e}")));
                }
                let handle = std::os::windows::io::OwnedHandle::from_raw_handle(job.0);
                Ok(Self {
                    job: Mutex::new(Some(handle)),
                })
            }
        }
        #[cfg(not(windows))]
        {
            Err(SandboxError::UnsupportedPlatform)
        }
    }

    /// True while the job handle is open (i.e. the kill-on-close guarantee is armed).
    pub fn is_active(&self) -> bool {
        #[cfg(windows)]
        {
            self.job.lock().unwrap().is_some()
        }
        #[cfg(not(windows))]
        {
            false
        }
    }

    /// True if the process with the given PID is inside this job.
    pub fn is_assigned(&self, pid: u32) -> Result<bool, SandboxError> {
        #[cfg(windows)]
        {
            use windows::Win32::Foundation::FALSE;
            use windows::Win32::System::JobObjects::IsProcessInJob;
            use windows::Win32::System::Threading::{
                OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
            };

            let job = self.open_job()?;
            unsafe {
                let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid)
                    .map_err(|e| SandboxError::Windows(format!("OpenProcess({pid}): {e}")))?;
                let mut in_job = FALSE;
                let result = IsProcessInJob(process, Some(job), &mut in_job)
                    .map_err(|e| SandboxError::Windows(format!("IsProcessInJob: {e}")));
                let _ = windows::Win32::Foundation::CloseHandle(process);
                result?;
                Ok(in_job.as_bool())
            }
        }
        #[cfg(not(windows))]
        {
            Err(SandboxError::UnsupportedPlatform)
        }
    }

    /// Assign an already-running process (by PID) into the job.
    pub fn assign_process(&self, pid: u32) -> Result<(), SandboxError> {
        #[cfg(windows)]
        {
            use windows::Win32::System::JobObjects::AssignProcessToJobObject;
            use windows::Win32::System::Threading::{
                OpenProcess, PROCESS_SET_QUOTA, PROCESS_TERMINATE,
            };

            let job = self.open_job()?;
            unsafe {
                let process = OpenProcess(PROCESS_SET_QUOTA | PROCESS_TERMINATE, false, pid)
                    .map_err(|e| SandboxError::Windows(format!("OpenProcess({pid}): {e}")))?;
                let result = AssignProcessToJobObject(job, process)
                    .map_err(|e| SandboxError::Windows(format!("AssignProcessToJobObject({pid}): {e}")));
                let _ = windows::Win32::Foundation::CloseHandle(process);
                result
            }
        }
        #[cfg(not(windows))]
        {
            Err(SandboxError::UnsupportedPlatform)
        }
    }

    /// Spawn `cmd` inside the job: CREATE_SUSPENDED → assign → resume.
    ///
    /// Overwrites any caller-set creation flags with `CREATE_SUSPENDED` (the
    /// child is resumed by this method before returning).
    pub fn spawn_contained(&self, cmd: &mut Command) -> Result<Child, SandboxError> {
        #[cfg(windows)]
        {
            use std::os::windows::io::AsRawHandle;
            use std::os::windows::process::CommandExt;
            use windows::Win32::Foundation::HANDLE;
            use windows::Win32::System::JobObjects::AssignProcessToJobObject;
            use windows::Win32::System::Threading::CREATE_SUSPENDED;

            let job = self.open_job()?;
            cmd.creation_flags(CREATE_SUSPENDED.0);
            let mut child = cmd.spawn()?;
            let pid = child.id();
            let process_handle = HANDLE(child.as_raw_handle());
            unsafe {
                if let Err(e) = AssignProcessToJobObject(job, process_handle) {
                    let _ = child.kill();
                    return Err(SandboxError::Windows(format!(
                        "AssignProcessToJobObject({pid}): {e}"
                    )));
                }
            }
            if let Err(e) = resume_main_thread(pid) {
                let _ = child.kill();
                return Err(e);
            }
            Ok(child)
        }
        #[cfg(not(windows))]
        {
            Err(SandboxError::UnsupportedPlatform)
        }
    }

    /// Close the job handle now (kills the contained process tree).
    pub fn close(&mut self) {
        #[cfg(windows)]
        {
            let mut guard = self.job.lock().unwrap();
            guard.take();
        }
    }

    #[cfg(windows)]
    fn open_job(&self) -> Result<windows::Win32::Foundation::HANDLE, SandboxError> {
        use std::os::windows::io::AsRawHandle;
        use windows::Win32::Foundation::HANDLE;

        self.job
            .lock()
            .unwrap()
            .as_ref()
            .map(|h| HANDLE(h.as_raw_handle()))
            .ok_or(SandboxError::Windows("job object already closed".to_string()))
    }
}

/// Resume the main thread of a process created with CREATE_SUSPENDED.
///
/// The main thread is the first thread entry owned by `pid` in the thread
/// snapshot (threads are enumerated in creation order).
#[cfg(windows)]
fn resume_main_thread(pid: u32) -> Result<(), SandboxError> {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Thread32First, Thread32Next, THREADENTRY32, TH32CS_SNAPTHREAD,
    };
    use windows::Win32::System::Threading::{OpenThread, ResumeThread, THREAD_SUSPEND_RESUME};

    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0)
            .map_err(|e| SandboxError::Windows(format!("CreateToolhelp32Snapshot: {e}")))?;
        let mut entry: THREADENTRY32 = std::mem::zeroed();
        entry.dwSize = std::mem::size_of::<THREADENTRY32>() as u32;
        let mut main_thread: Option<u32> = None;
        if Thread32First(snapshot, &mut entry).is_ok() {
            loop {
                if entry.th32OwnerProcessID == pid {
                    main_thread = Some(entry.th32ThreadID);
                    break;
                }
                if Thread32Next(snapshot, &mut entry).is_err() {
                    break;
                }
            }
        }
        let _ = CloseHandle(snapshot);
        let tid = main_thread.ok_or(SandboxError::ResumeFailed {
            pid,
            reason: "main thread not found in thread snapshot".to_string(),
        })?;
        let thread = OpenThread(THREAD_SUSPEND_RESUME, false, tid)
            .map_err(|e| SandboxError::Windows(format!("OpenThread({tid}): {e}")))?;
        let previous = ResumeThread(thread);
        let _ = CloseHandle(thread);
        if previous == u32::MAX {
            return Err(SandboxError::ResumeFailed {
                pid,
                reason: "ResumeThread failed".to_string(),
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(windows)]
    fn wait_for_exit(child: &mut Child, timeout: std::time::Duration) -> Option<std::process::ExitStatus> {
        let deadline = std::time::Instant::now() + timeout;
        loop {
            if let Some(status) = child.try_wait().unwrap() {
                return Some(status);
            }
            if std::time::Instant::now() >= deadline {
                return None;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
    }

    #[cfg(windows)]
    fn long_running_command() -> Command {
        // ping -n 30 ≈ 30 s runtime; long enough for containment tests.
        let mut cmd = Command::new("ping");
        cmd.args(["-n", "30", "127.0.0.1"]);
        cmd
    }

    #[cfg(windows)]
    #[test]
    fn new_creates_active_supervisor() {
        let supervisor = JobObjectSupervisor::new().unwrap();
        assert!(supervisor.is_active());
        assert!(!supervisor.is_assigned(std::process::id()).unwrap());
    }

    #[cfg(windows)]
    #[test]
    fn contained_process_is_assigned_and_dies_on_close() {
        let supervisor = JobObjectSupervisor::new().unwrap();
        let mut child = supervisor.spawn_contained(&mut long_running_command()).unwrap();
        let pid = child.id();
        assert!(supervisor.is_assigned(pid).unwrap());

        // The child must still be running while the job is open.
        assert!(child.try_wait().unwrap().is_none());

        // Close the job → kernel kills the whole tree. The property to assert
        // is *early termination*: ping -n 30 would naturally run ~30 s; the
        // kill-on-close must terminate it well before that (exit code on
        // job termination is not meaningful — it may be 0).
        let started = std::time::Instant::now();
        drop(supervisor);
        let status = wait_for_exit(&mut child, std::time::Duration::from_secs(10))
            .expect("contained child should die when the job closes");
        let elapsed = started.elapsed();
        assert!(
            elapsed < std::time::Duration::from_secs(8),
            "contained child took {elapsed:?} to die — kill-on-close not effective"
        );
        eprintln!("contained child terminated after {elapsed:?} (exit {status:?})");
    }

    #[cfg(windows)]
    #[test]
    fn contained_process_runs_and_exits_normally() {
        let supervisor = JobObjectSupervisor::new().unwrap();
        let mut cmd = Command::new("ping");
        cmd.args(["-n", "1", "127.0.0.1"]);
        let mut child = supervisor.spawn_contained(&mut cmd).unwrap();
        let status = wait_for_exit(&mut child, std::time::Duration::from_secs(10))
            .expect("short ping should exit quickly");
        assert!(status.success());
    }

    #[cfg(windows)]
    #[test]
    fn assign_existing_process_by_pid() {
        let supervisor = JobObjectSupervisor::new().unwrap();
        let mut child = long_running_command().spawn().unwrap();
        let pid = child.id();
        supervisor.assign_process(pid).unwrap();
        assert!(supervisor.is_assigned(pid).unwrap());
        let _ = child.kill();
        let _ = child.wait();
    }

    #[cfg(windows)]
    #[test]
    fn close_then_operations_fail() {
        let mut supervisor = JobObjectSupervisor::new().unwrap();
        supervisor.close();
        assert!(!supervisor.is_active());
        let mut cmd = Command::new("cmd");
        cmd.args(["/c", "exit 0"]);
        assert!(matches!(
            supervisor.spawn_contained(&mut cmd),
            Err(SandboxError::Windows(_))
        ));
    }

    #[cfg(not(windows))]
    #[test]
    fn non_windows_is_fail_closed() {
        let supervisor = JobObjectSupervisor::new();
        assert!(matches!(supervisor, Err(SandboxError::UnsupportedPlatform)));
    }
}
