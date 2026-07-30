# Grok 0.2.112 Timeout Triage

Date: 2026-07-30
Status: resolved for default-version promotion; split gates implemented;
carried-forward owned-cleanup limitation remains open

## Verdict

The Windows child-tree `tool_timeout` failure is not a `0.2.112` regression.

Administrator triage runs compared `0.2.111` and `0.2.112` with the same fake provider, Python executable, firewall path, child-tree fixture, `tool_timeout_ms=3000`, and outer timeout/grace settings. Both versions behaved the same:

- the fake provider completed the fixed two-primary-request flow;
- Grok emitted the terminal marker `LIF_PROCESS_TREE_PROBE_OK`;
- Grok did not exit promptly after provider terminal success;
- Grok exited only after the fake provider server timeout path;
- the root/child/grandchild fixture processes were still visible before the outer Job Object was closed;
- closing the outer Job Object removed the remaining nonce-matched process tree.

This means `tool_timeout` is a carried-forward containment limitation relative to the current `0.2.111` default, not evidence that `0.2.112` worsened the behavior.

## Evidence

| Run | Versions | Timeout settings | Result |
|---|---|---|---|
| `candidate-gates/grok-timeout-triage-admin-r2` | `0.2.111`, `0.2.112` | `timeout_seconds=60`, `exit_grace_seconds=10` | both failed before result generation; both diagnostics showed provider terminal success, Grok return code `0`, live child tree before outer Job close, and zero residue after Job close |
| `candidate-gates/grok-timeout-triage-admin-r4` | `0.2.111`, `0.2.112` | `timeout_seconds=90`, `exit_grace_seconds=30` | same outcome as r2 |

Representative diagnostic fields from r4:

| Version | `elapsed_seconds` | `grok.returncode_before_job_close` | `provider.result.terminal_state` | pre-close nonce process count | post-close nonce process count |
|---|---:|---:|---|---:|---:|
| `0.2.111` | `127.366` | `0` | `succeeded` | `4` | `0` |
| `0.2.112` | `127.02` | `0` | `succeeded` | `4` | `0` |

## Promotion Impact

`0.2.112` can be selected as the default only with a precise limitation:

- accepted: no observed regression relative to `0.2.111` for this timeout scenario;
- accepted: outer Job Object containment removes the remaining process tree;
- not proven: Grok-owned timeout cleanup of the full child process tree;
- not proven: real external model reasoning continuity in this receipt.

The candidate metadata now records this as two gates:

- `windows_child_tree_baseline_regression`: `passed`;
- `windows_child_tree_owned_cleanup`: `carried-limitation`.

## Follow-Up

The gate has been redesigned into two separate checks:

1. baseline-regression gate: candidate behavior must not be worse than the current lock;
2. owned-cleanup gate: Grok itself must terminate the timed-out child tree before harness cleanup.

Only the first check is satisfied for the `0.2.112` default-version promotion.
The split verifier emits `prompt_tool_promotion_ready=false` while
`windows_child_tree_owned_cleanup` remains a carried limitation.
