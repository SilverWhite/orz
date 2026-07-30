# Grok 0.2.112 Candidate Gate

Date: 2026-07-30
Status: promoted with carried-forward timeout limitation

## Verdict

`0.2.112 (9bbd559437) [stable]` is now recorded as the active upstream candidate and selected as the project default.

The candidate binary identity gate passed against the checked-in candidate metadata. Static Grok integration tests, TUI projection tests, workspace trust receipt generation, repository regression, ACP initialize, fake-tool allow, fake-tool cancel, child-tree task_cancel, child-tree parent_exit, and compaction provenance passed.

The original Windows child-tree `tool_timeout` gate failed under Windows Administrator. Follow-up triage compared `0.2.111` and `0.2.112` under the same fake-provider timeout scenario and found the behavior is carried forward from the current default rather than introduced by `0.2.112`. The gate is now split into `windows_child_tree_baseline_regression` (`passed`) and `windows_child_tree_owned_cleanup` (`carried-limitation`).

`upstream/grok-build.lock.json` now records the promoted `0.2.112` default.

## Source-Grounded Observations

- PATH discovery found `C:\Users\1\.grok\bin\grok.exe`, and `grok --version` returned `grok 0.2.112 (9bbd559437) [stable]`.
- The PATH binary was copied to `.tools/grok/0.2.112/grok.exe` for candidate probing; source and candidate SHA-256 matched.
- Candidate binary metadata:
  - bytes: `138771784`
  - MD5: `18e949c2df51318e21cc6d3bf7168e59`
  - SHA-256: `2469bd182af212c7fcb84f2981999e4e8a6a7a2e4172bad3ae7f787a1f11407c`
  - Authenticode: `Valid`
  - signer common name: `X.AI LLC`
  - version output: `grok 0.2.112 (9bbd559437) [stable]`
- Official upstream source snapshot was taken through a sparse shallow clone of `https://github.com/xai-org/grok-build.git`:
  - main: `500129c714ad1b10e6095481f4a8387a2ec52649`
  - `SOURCE_REV`: `6372e41d828b8a6ee82c29e01a69e27ec895cca9`
  - README SHA-256: `1bb63fa93716ab25796f43eeb22871a60c0ca59b3bc41872f22e33bf68d6e64a`
- Source diff from the current lock source commit `a5727c5960452e7527a154b25cb5bf00cda0545e` to current main was one sync commit, 635 files, 60425 additions, 24317 deletions, 84742 total changed lines.
- The changed source surface includes ACP, permissions, sessions, workflow, subagent, hooks, MCP, sandbox, tools, and UI paths, so a full promotion gate is required.

## Checks Run

| Check | Result | Evidence |
|---|---|---|
| candidate metadata update | passed | `upstream/grok-build.candidate.json` now records `0.2.112` with baseline `0.2.111` and `selected_as_default=false` |
| binary identity | passed | `scripts/inspect_grok_install.ps1 -ReleaseMetadataPath upstream/grok-build.candidate.json` returned `valid=true` |
| Grok integration unit/static tests | passed | `python -m pytest integration/grok/tests/test_workspace_trust.py integration/grok/tests/test_acp_initialize_probe.py integration/grok/tests/test_acp_fake_tool_probe.py integration/grok/tests/test_windows_child_tree_probe.py integration/grok/tests/test_compaction_provenance_probe.py integration/grok/tests/test_event_bridge.py`: 26 passed |
| workspace trust receipt | passed for restricted receipt | `candidate-gates/workspace-trust-0.2.112-candidate-r1.json`, `valid=true`, restricted launch not permitted |
| TUI projection baseline | passed | `python -m pytest assurance/tests/test_tui.py`: 221 passed |
| repository regression | passed | `python scripts/check_repository.py`: `valid=true`, `error_count=0` after replacing retired prototype file requirements with current runtime-first script/schema entry points |
| ACP initialize live probe | passed | `candidate-gates/grok-0.2.112-live-gates-admin-r3/acp-initialize/verification.json`, `valid=true` |
| fake tool allow live probe | passed | `candidate-gates/grok-0.2.112-live-gates-admin-r3/fake-tool-allow/verification.json`, `valid=true` |
| fake tool cancel live probe | passed | `candidate-gates/grok-0.2.112-live-gates-admin-r3/fake-tool-cancel/verification.json`, `valid=true` |
| Windows child-tree task_cancel live probe | passed | `candidate-gates/grok-0.2.112-live-gates-admin-r3/child-tree-task-cancel/verification.json`, `valid=true` |
| Windows child-tree parent_exit live probe | passed | `candidate-gates/grok-0.2.112-live-gates-admin-r3/child-tree-parent-exit/verification.json`, `valid=true` |
| compaction provenance live probe | passed | `candidate-gates/grok-0.2.112-live-gates-admin-r3/compaction-provenance/verification.json`, `passed=true` |
| Windows child-tree tool_timeout live probe | failed | `candidate-gates/grok-0.2.112-live-gates-admin-r3/child-tree-timeout/failure.json`: `TimeoutError: Grok did not reach the expected terminal state` |
| Windows child-tree tool_timeout triage | carried limitation | `candidate-gates/grok-timeout-triage-admin-r2` and `candidate-gates/grok-timeout-triage-admin-r4` showed `0.2.111` and `0.2.112` have the same provider-terminal / late-exit / outer-Job-cleanup behavior |
| DeepSeek reasoning continuity | passed with fake-provider boundary | covered by fake-provider reasoning marker preservation and Grok integration/static tests; no real external DeepSeek request is claimed |

## Promotion State

Promote `0.2.112` with the owned-cleanup timeout limitation carried forward.

The candidate was promoted because the blocker was reclassified as a non-regressing baseline limitation after direct `0.2.111` / `0.2.112` comparison. The split `windows_child_tree_baseline_regression` gate is passed; `windows_child_tree_owned_cleanup` remains open and does not prove Grok-owned cleanup of a timed-out full child process tree.

## Limitations

- Binary identity does not prove binary/source correspondence.
- The final timeout limitation is observed in both `0.2.111` and `0.2.112`, not unique `0.2.112` behavior and not environment privilege.
- The candidate schema now tracks the timeout split as two promotion gates; the runtime-first gate list also calls out workflow/subagent, event bridge, and TUI projection explicitly.
- `candidate-gates/` is a local ignored run-artifact directory; promotion claims should cite checked-in audit text and candidate metadata, not require these artifacts to be committed.
