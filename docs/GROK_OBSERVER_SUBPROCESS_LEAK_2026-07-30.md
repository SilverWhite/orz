# Grok Observer Subprocess Leak Audit

Date: 2026-07-30
Status: discovered, root-caused, fixed (subprocess containment extension)

## 1. Discovery

During the afternoon `0.2.112` candidate gate evaluation, `grok inspect --json`
was called via Python `subprocess.run` with stdout piped and truncated via
`| head`.  After the calling script exited, a `grok` process (PID 128, started
2026-07-30 21:29) remained alive for over 24 minutes.  A second `grok --help`
call produced the same pattern.  The processes consumed no CPU and had no
visible children; they were hung waiting for the closed pipe to drain.

Later in the same session, `gsa grok observe-tools` was confirmed to run
three Grok subprocess calls through the same vulnerable `subprocess.run`
path — any one of which could produce the same orphan.

## 2. Symptoms

| Call site | Command | Pipe scenario | Observed |
|---|---|---|---|
| `_run_json_command` (observer) | `grok inspect --json` | stdout → pipe, `| head` truncates | Grok hangs, no CPU, pipe never signals close |
| `_run_text_command` (observer) | `grok --help` | same | Same |
| `_run_text_command` (observer) | `grok agent --help` | same | Same (presumed) |
| `_run_json_command` (adapter) | `PowerShell inspect_grok_install.ps1` | stdout → pipe | Same risk surface |
| `_run_json_command` (adapter) | `PowerShell new_grok_workspace_trust_receipt.ps1` | stdout → pipe | Same risk surface |

Root cause: `subprocess.run` captures stdout via pipe.  When the reader
closes the pipe (e.g. `head` exits after N lines, or the Python script ends
before reading all output), the pipe breaks.  On Unix, the writer receives
`SIGPIPE` and exits.  On Windows, there is no `SIGPIPE` equivalent — the
writer (`grok`) never learns the pipe is closed and hangs indefinitely.

This is **not** the same as the `tool_timeout` child-tree leak
(`GROK_0_2_112_TIMEOUT_TRIAGE_2026-07-30`), which concerns Grok's failure to
terminate its own tool subprocesses on timeout.  This leak occurs when Grok
**itself** is the subprocess, called by the project's observer/adapter code.

## 3. Relationship to Existing Documentation

| Document | Scope | Relationship |
|---|---|---|
| `GROK_0_2_112_TIMEOUT_TRIAGE` | Grok tool-execution child-tree cleanup | Different leak surface (Grok as parent of tool subprocesses) |
| `GROK_0_2_112_CANDIDATE_GATE` | Gate split decision | Records `owned_cleanup` as carried-limitation but does not cover observer subprocess hang |
| `GAK_SBX_001_WINDOWS_NATIVE_SANDBOX_AUDIT` | Windows Job Object fundamentals | Provides the `KILL_ON_JOB_CLOSE` pattern reused by the fix |
| `RUNTIME_FIRST_FOLLOWUP_ALIGNMENT` | Ownership and implementation status | Updated to reference subprocess containment extension |

## 4. Fix

**Commit**: `80b22da` — Extend JobObjectSupervisor containment to all Grok subprocess paths

**Strategy**: Add `contained_run()` to `assurance/job_object_supervisor.py` — a
drop-in replacement for `subprocess.run` that wraps every call in a
Kill-On-Close Job Object.

```
contained_run(command)
  → JobObjectSupervisor 创建 Kill-On-Close Job
  → Popen(command, stdout=PIPE, stderr=PIPE)
  → supervisor.assign_process(pid)
  → process.communicate(timeout)
  → finally: supervisor.close()
      → kernel 关闭 Job handle
      → JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE 触发
      → 进程树被内核强制终止
```

**Coverage** (before → after):

| Entry point | Before | After |
|---|---|---|
| `run_grok_headless_once` Grok launch | ✅ `JobObjectSupervisor` (P1) | ✅ unchanged |
| `observe_grok_tool_permission_surfaces` ×3 | ❌ bare `subprocess.run` | ✅ `contained_run` |
| `inspect_grok_runtime` (PowerShell) | ❌ bare `subprocess.run` | ✅ `contained_run` |
| `_workspace_trust` (PowerShell) | ❌ bare `subprocess.run` | ✅ `contained_run` |

## 5. Community Context

The same class of Windows orphan-process problem was independently reported
and fixed in **openclaw/openclaw PR #115535** (2026-07-29):

> "Windows users running MCP servers, agent subprocesses, or other Node-based
> process trees ended up with orphaned child processes after graceful
> `taskkill /T` refused to terminate a console process."

The openclaw fix uses the same escalation pattern (graceful-first →
conditional forced termination).  This confirms the issue is a **generic
Windows agent-framework concern**, not specific to Grok or to this project.

Grok's own GitHub (`xai-org/grok-build`) does not track issues publicly —
the Issues page is empty.  No upstream acknowledgment of either the
`tool_timeout` child-tree leak or the observer subprocess hang has been found.

## 6. Residual Risk

| Risk | Status | Notes |
|---|---|---|
| Grok `tool_timeout` child-tree leak | `carried-limitation` | Grok does not prove it cleans up timed-out tool subprocesses; outer Job Object in probe harness provides fallback |
| Observer subprocess hang (this audit) | **closed** | `contained_run()` now wraps all Grok subprocess calls |
| Future Grok subprocess entry points | **gated** | New subprocess calls to `grok` binary must use `contained_run()` or explicit `JobObjectSupervisor`; review required in PR |
| Non-Windows platforms | **not applicable** | `contained_run` is a passthrough to `subprocess.run` on non-Windows, where `SIGPIPE` handles pipe close |
| `CONTAINMENT_ENABLED = False` test bypass | **contained** | Tests patches subprocess calls with mocks; flag is for explicit opt-out only |

## 7. Verification

- `test_grok_observe_tools_output_gate_receipt_writes_file` — CLI observer with `--output-gate-receipt`
- `test_run_runtime_grok_allow_with_dual_acp_and_containment` — full promotion chain with containment
- `test_job_object_supervisor.py` (11 tests) — Job Object creation, assignment, kill-on-close, child inheritance, context manager
- Full regression: 1297 passed, 13 skipped, 0 failures
- Live verification: the hung `grok` PID 128 residue was killed, no new residues observed after subsequent `gsa grok observe-tools` calls
