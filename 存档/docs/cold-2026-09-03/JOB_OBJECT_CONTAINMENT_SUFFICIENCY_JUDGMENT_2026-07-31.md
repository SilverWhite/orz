# Job Object Containment Sufficiency Judgment

Date: 2026-07-31
Status: ruled — `CREATE_SUSPENDED` + `AssignProcessToJobObject` containment sufficient for prompt/tool promotion

## 1. Context

The Grok runtime adapter (`assurance/grok_runtime_adapter.py`) currently
has two containment launch paths with different guarantees:

| Launch path | `CREATE_SUSPENDED`? | Post-creation race window |
|---|---|---|
| `contained_run()` (for PowerShell scripts, observer calls) | **Yes** — process frozen, assigned to Job, then resumed | Closed |
| `run_with_job_object_containment()` (unused Popen helper) | **No** — `AssignProcessToJobObject` after Popen | Open |
| `run_grok_headless_once()` (actual `grok --version` binary launch) | **No** — same post-creation assign | Open |

The question: does `PROC_THREAD_ATTRIBUTE_JOB_LIST` (GAK-WIN-001, kernel-level
atomic Job assignment at process creation time) need to be implemented before
prompt/tool execution can be promoted, or is the `CREATE_SUSPENDED` +
`AssignProcessToJobObject` pattern in `contained_run()` sufficient?

## 2. Technical Analysis

### 2.1 CREATE_SUSPENDED mechanism

```
CreateProcess(..., CREATE_SUSPENDED)
  → Kernel creates process object and maps ntdll.dll
  → Initial thread created with suspend count = 1 (FROZEN)
  → ⛔ Zero user-mode code executes (thread at LdrInitializeThunk, not yet entered)
  → Process exists as kernel object only — no DLL init, no main(), no instructions

AssignProcessToJobObject(job_handle, process_handle)
  → Kernel assigns process to Kill-On-Close Job Object
  → All future child processes automatically join the Job (JOB_OBJECT_LIMIT_BREAKAWAY_OK not set)

ResumeThread(main_thread_handle)
  → Kernel decrements suspend count to 0
  → Thread enters LdrInitializeThunk → DLL initialization → entry point
  → ✅ Job membership already established
```

### 2.2 PROC_THREAD_ATTRIBUTE_JOB_LIST mechanism

```
CreateProcess(..., EXTENDED_STARTUPINFO_PRESENT, PROC_THREAD_ATTRIBUTE_JOB_LIST=job_handle)
  → Kernel creates process AND assigns it to the Job atomically during creation
  → Process emerges fully contained — no separate assign step needed
```

### 2.3 Equivalence analysis

| Property | `CREATE_SUSPENDED` + assign | `PROC_THREAD_ATTRIBUTE_JOB_LIST` |
|---|---|---|
| User-mode code executes before containment? | **No** — thread frozen until `ResumeThread` | **No** — kernel assigns atomically |
| Child processes automatically contained? | **Yes** — Job Object inheritance (kernel guarantee) | **Yes** — same kernel guarantee |
| Kill-On-Close works? | **Yes** — `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` | **Yes** — same flag |
| Kernel terminates tree on Job handle close? | **Yes** — kernel walks Job's process list | **Yes** — same kernel behavior |
| Syscalls required | 3 (CreateProcess + Assign + ResumeThread) | 1 (CreateProcess with attributes) |
| Code complexity | Moderate (thread enumeration for ResumeThread) | Low (attributes list in STARTUPINFOEX) |

**From a containment guarantee perspective, the two approaches are equivalent.**
Both ensure that before any user-mode code executes, the process is a member of
the Kill-On-Close Job Object. The difference is purely in implementation
cleanliness — not in security or containment effectiveness.

### 2.4 Theoretical edge cases considered

- **Remote thread injection into suspended process**: A kernel-mode component
  (driver, anti-virus) with sufficient privilege could inject a thread into the
  suspended process between creation and assignment. This requires kernel-level
  access and is outside the project's threat model (if kernel-mode malware is
  present, Job Object containment is the least of your problems).
- **Process exit before assignment**: With `CREATE_SUSPENDED`, the frozen
  thread cannot call `ExitProcess`. The process cannot exit until resumed.
- **Child process escape between resume and grandchild creation**: Not possible.
  Once the parent is in the Job, all children automatically join (kernel guarantee).

## 3. Decision

**`CREATE_SUSPENDED` + `AssignProcessToJobObject` provides containment
equivalent to `PROC_THREAD_ATTRIBUTE_JOB_LIST` for the purposes of prompt/tool
execution promotion.**

`PROC_THREAD_ATTRIBUTE_JOB_LIST` is a **code-quality refinement** (fewer
syscalls, cleaner API, no suspend/resume dance, no thread enumeration), not a
**security prerequisite**. It may be implemented in the future as a
non-urgent improvement.

The `windows_child_tree_owned_cleanup` = `carried-limitation` on Grok's side
**no longer blocks** prompt/tool promotion — the adapter-side
`JobObjectSupervisor` provides equivalent containment independently.

## 4. Implementation

Two launch paths were upgraded to close the post-creation race window:

### 4.1 `run_with_job_object_containment()` (job_object_supervisor.py)

Generic Popen-returning helper. Previously: `Popen` → `assign_process`.
Now: `Popen(CREATE_SUSPENDED)` → `assign_process` → `_resume_main_thread`.

### 4.2 `run_grok_headless_once()` (grok_runtime_adapter.py)

Actual Grok binary launch path. Previously: `Popen(CREATE_NO_WINDOW)` →
`assign_process`. Now: same `CREATE_SUSPENDED` + assign + resume pattern.

Both paths now match `contained_run()`'s containment guarantee.

## 5. Residual Risk

| Risk | Status | Notes |
|---|---|---|
| Post-creation race window (adapter → Grok binary) | **Closed** | `CREATE_SUSPENDED` applied to all Grok Popen paths |
| Post-creation race window (observer helper scripts) | **Closed** | Already covered by `contained_run()` |
| `popen_factory` custom callables not propagating `creationflags` | **Gated** | Code-review item — any custom factory must accept and pass through `creationflags` |
| `PROC_THREAD_ATTRIBUTE_JOB_LIST` not implemented | **Accepted** | Code-quality refinement; no security gap |
| Non-Windows (no Job Objects) | **N/A** | All paths are passthrough — no containment available or needed |

## 6. Verification

- `assurance/tests/test_job_object_supervisor.py` — 11 existing tests + CREATE_SUSPENDED coverage (39 passed, 1 skipped)
- `assurance/tests/test_grok_runtime_adapter.py` — 5 tests with `_resume_main_thread` patch (all pass)
- `assurance/tests/test_grok_prompt_tool_gate.py` — `adapter_containment_provided` path tested end-to-end (9 tests pass)
- `assurance/tests/test_cli_dispatcher.py` — `gsa run --runtime grok` promotion gate chain (20 tests pass)
- Full regression: 1413 tests pass (2026-07-31)
