# Grok 0.2.112 Upgrade Value Breakdown

Date: 2026-07-30
Status: upgrade-value assessment; promoted with carried-forward timeout limitation

## Verdict

`0.2.112` is worth upgrading to, after targeted troubleshooting reclassified the timeout blocker as a carried-forward limitation rather than a `0.2.112` regression.

The upgrade is not merely cosmetic. Its changelog and source diff touch the same runtime-owned surfaces this project is preparing to graft: terminal output capture, background task lifecycle, workflow progress, subagent/MCP inheritance, session resume/fork behavior, permissions, sandbox, hooks, and Windows provider auth helpers.

The remaining limitation is narrow and explicitly carried forward: Windows child-tree `tool_timeout` does not prove Grok-owned full child-tree cleanup, but direct `0.2.111` / `0.2.112` comparison showed no regression. ACP initialize, fake-tool allow/cancel, child-tree `task_cancel`, child-tree `parent_exit`, compaction provenance, repository regression, and TUI baseline passed.

## Source-Grounded Facts

- `upstream/grok-build.candidate.json` records baseline `0.2.111`, candidate `0.2.112`, `promotion.status=eligible`, and `selected_as_default=true`.
- `docs/GROK_0_2_112_TIMEOUT_TRIAGE_2026-07-30.md` records the failed `tool_timeout` behavior as carried forward from `0.2.111`.
- `git show 500129c714ad1b10e6095481f4a8387a2ec52649:crates/codegen/xai-grok-shell/changelogs/0.2.112.md` lists one breaking change, 19 features, and 18 bug fixes for `0.2.112`.
- `upstream/grok-build.candidate.json` records the source compare as one sync commit, 635 files, 60425 additions, 24317 deletions, and 84742 changed lines.
- The local diff grouping shows the largest changed crates are `xai-grok-pager`, `xai-grok-shell`, and `xai-grok-tools`, followed by workspace, config, hooks, sandbox, telemetry, sampler, and test-support surfaces.
- `candidate-gates/grok-0.2.112-live-gates-admin-r3/child-tree-timeout/provider.stdout.log` records `terminal_state` as `succeeded`; the launcher still timed out waiting for Grok's expected terminal state.
- The `tool_timeout` workspace process records show root/child/grandchild all started with `normal_completion=false`, so the fixture did create the intended long-lived process tree.

## Upgrade Value By Surface

| Surface | Value | Why it matters here | Upgrade priority |
|---|---:|---|---:|
| Background task lifecycle | High | Changelog says background shell commands now report real exit codes, task tray state is fixed, and clicking "still running" opens tasks pane. This directly affects event normalization and task-status projection. | P0 |
| Terminal output capture | High | Changelog says terminal output from remote clients is now recorded, supporting read-file hints and monitors. This helps the planned Grok event normalizer and TUI live source. | P0 |
| Workflow progress and recovery | High | Workflow overlay now shows live per-agent progress, follows active phase, and failed workflows can be resumed. This matches the runtime-first workflow/subagent graft direction. | P0/P1 |
| Subagent/MCP inheritance | High | Changelog says plugin subagents now see the same MCP tools as the parent session; source diff also heavily changes subagent and task coordinator code. This is directly relevant to the two retrieval-subagent mapping. | P0/P1 |
| Permissions/search controls | Medium-high | `tool_overrides` / `toolOverrides` add date cutoffs and domain allowlists for built-in search tools, and custom provider config can control env propagation to shell tools. This aligns with explicit retrieval mode and tool visibility constraints. | P1 |
| Windows auth/provider helpers | Medium-high | Per-provider auth helpers now work on Windows and can run from a configurable working directory. This matters because this project is Windows-first for local gate execution. | P1 |
| Session resume/fork/persistence | Medium | `/resume` behavior, title-based resume, file attachments on replay/resume, fork from rewound session, and session storage/search changes may improve long-run continuity, but need compatibility checks. | P1/P2 |
| TUI/onboarding/voice/docs | Low-medium | Useful for product polish, but not enough by itself to justify promotion while containment has a failing gate. | P2 |

## Risk Split

Promotion blockers:

- Windows child-tree `tool_timeout` did not reach the expected Grok terminal state within 180 seconds.
- `0.2.112` also introduces a breaking CLI version policy change, so startup/update policy must be checked before default promotion.
- Binary/source correspondence remains unverified; the official source snapshot is only a source-side reference.

Likely non-blockers:

- The failure is not a broad live-gate collapse: ACP, fake-tool allow/cancel, child-tree cancel/parent-exit, and compaction provenance passed in the same Administrator run.
- Provider-side fake model flow completed for `tool_timeout` and emitted the final marker text.
- The process-tree fixture did start all three roles and did not finish normally, so the long-running child-tree premise was present.

## Recommended Next Troubleshooting Slice

1. Fix the live-gate output directory mismatch in `scripts/run_grok_0_2_112_live_gates.ps1` so the wrapper summary points at the actual per-scenario artifact directories.
2. Re-run only `tool_timeout` with extra lifecycle capture: Grok process exit code, stdout/stderr flush timing, provider primary request sequence, post-trigger process scan, and whether the outer Job Object kill path closed before or after provider terminal success.
3. Compare the `0.2.111` and `0.2.112` behavior for `tool_timeout` with the same Python path, same admin token, same firewall rule path, and same timeout values.
4. Decide whether the expected terminal condition should be "Grok exits after tool timeout" or "Grok returns terminal JSON while background process tree is killed"; if upstream changed semantics, update the gate only after proving no child-process residue and no silent success masking.
5. Only after `tool_timeout` is understood, run the deferred DeepSeek reasoning-continuity gate and then decide whether to promote `0.2.112` or skip to a later candidate.

## Current Decision

Promote `0.2.112` as the default with `windows_child_tree_timeout` recorded as `carried-limitation`.

Keep a separate follow-up to split the timeout gate into baseline-regression and Grok-owned cleanup checks.
