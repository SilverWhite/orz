# Design-to-Implementation Gap Audit v0.1

Date: 2026-08-02
Status: audit complete; task list frozen
Scope: all architecture/protocol/evaluation/regression design documents vs `assurance/` implementation
Baseline: Grok Build 0.2.112 as upstream runtime reference

## 1. Audit Method

1. Read all 26 architecture docs, 3 protocol docs, 3 evaluation docs, 1 regression doc.
2. Read all ~105 production modules in `assurance/`.
3. Cross-referenced each design requirement against implementation.
4. Applied `UPSTREAM_FIRST_INTEGRATION_v0.1.md` ownership model to reclassify:
   - Items delegated to Grok Build → not gaps
   - Items intentionally frozen → not gaps
   - Items Grok already covers → not gaps
   - Remaining items → real gaps

## 2. Ownership Model (from UPSTREAM_FIRST_INTEGRATION_v0.1.md)

| Capability | Owner | Our action |
|-----------|-------|-----------|
| Model loop, tool loop, retry, HTTP/SSE, session lifecycle | Grok Build | Adopt directly |
| Permission system, sandbox profiles | Grok Build | Configure + Windows-verify |
| TUI, headless, ACP, workspace, MCP, hooks | Grok Build | Adopt; verify conformance |
| Custom model provider config | Grok Build TOML | Use; verify thinking continuity |
| SourceRouter, EvidenceKernel, ValidatorBridge | Assurance kernel | Retain as product differentiator |
| ScenarioExporter, LeakScanner, EvaluationRunner | Assurance kernel | Retain as hard gates |
| GateDecision, TaskContract, ClaimBoundary | Assurance kernel | Retain; systemize |
| DeepSeek preflight, Windows containment | Assurance kernel | Retain; narrow adapter |

## 3. Items Intentionally Frozen (NOT gaps)

Per `UPSTREAM_FIRST_INTEGRATION_v0.1.md` §3:

> "以下组件冻结为 disposable conformance fixtures，不再横向扩展：
> 通用 model transport、loopback HTTP、fake HTTPS provider、通用 model loop、
> 网络 permit broker 与交互 approval ledger。若 Grok Build 已满足同一语义，
> 正式集成直接删除对应生产需求"

| # | Frozen item | Grok replacement |
|---|------------|-----------------|
| 1 | Loopback HTTP transport | Grok ACP JSON-RPC stdio as transport |
| 2 | Mock tool loop (mock_echo) | Grok ACP real tool loop, already verified for continuity |
| 3 | Interactive approval ledger (network-approvals.jsonl) | Grok `--permission-mode`, `session/request_permission` |
| 4 | In-process fake HTTPS provider | `integration/grok/deepseek-custom-model.example.toml` + `scripts/fake_deepseek_provider.py` |
| 5 | DenyAll/Scripted/Interactive PermitBroker types | Grok `--permission-mode` + `--allow`/`--deny` |

## 4. Items Already Covered by Grok (NOT gaps)

| # | Item | How Grok covers it |
|---|------|-------------------|
| 1 | SSE parsing | Grok internally consumes provider SSE; assurance only parses Grok NDJSON `streaming-json` |
| 2 | Retry loop | Explicit zero-retry policy is an assurance boundary — by design, not omission |

## 5. Real Gaps — Prioritized Task List

### P0 — Must resolve before real DeepSeek network access

**GAK-01: DeepSeek thinking multi-turn continuity real-API probe**
- Source: `DEEPSEEK_ADAPTER_CONTRACT_v0.1.md` §2.2, `UPSTREAM_FIRST_INTEGRATION_v0.1.md` §4
- Problem: "当前唯一尚未证明可直接复用的关键点" — when Grok custom model path calls real DeepSeek, does it automatically preserve `reasoning_content` across tool-call turns?
- What to do: Run a real DeepSeek API call through Grok ACP with a thinking + tool-call prompt. Verify that `reasoning_content` is:
  1. Returned by DeepSeek in the first assistant message
  2. Preserved and sent back in the follow-up request after tool result
  3. Not leaked into Grok `updates.jsonl` or our `events.jsonl`
- Files: `integration/grok/deepseek-custom-model.example.toml`, `grok_runtime_adapter.py`
- Acceptance: A conformance probe receipt showing pass/fail on each check

**GAK-02: GateDecision warn/defer systemization**
- Source: `PROTOCOL_DRAFT_v0.1.md` §9
- Problem: Most gates use allow/block binary. The protocol requires five decisions:
  - `pass` — evidence sufficient, no concern
  - `warn` — pass with caveat (e.g., incomplete but not blocking)
  - `block` — hard stop, must not proceed
  - `defer` — evidence insufficient to decide, need more
  - `not_applicable` — gate does not apply to this action/evidence
- Current state: `grok_tool_permission_observer.py` already outputs allow/defer/block.
  `instruction_provenance_gate.py` and `source_visibility.py` mostly use binary allow/block.
- What to do:
  1. Standardize all gate receipt schemas to include `decision` enum with all five values
  2. Update `instruction_provenance_gate.py` to output warn when injection patterns are borderline
  3. Update `source_visibility.py` to output defer when visibility is insufficient for claim
  4. Add `not_applicable` path for gates that are skipped by mode
- Acceptance: Every gate function returns one of the five decisions; verifier checks the enum

### P1 — Should resolve before production use

**GAK-03: Grok ACP permission default behavior audit**
- Source: Agent investigation of `grok_runtime_adapter.py` lines 1141–1148
- Problem: `GrokAcpSession` auto-approves `allow_once` when no user callback is attached, recording authority as `"adapter"`. This is silent auto-approval — deny-by-default may be more appropriate for guarded/strict modes.
- What to do:
  1. Audit all call sites of `run_grok_acp_once` to see which have user callbacks
  2. Decide whether `default=deny` or `default=allow_once` per mode
  3. Document the decision in the adapter contract
- Acceptance: Explicit policy documented; auto-approve path gated on mode

**GAK-04: idempotency_key in ActionProposal**
- Source: `PROTOCOL_DRAFT_v0.1.md` §7.1
- Problem: High-risk actions have no stable `idempotency_key`. Protocol states this should trigger `ACT-IDEMPOTENCY-001`.
- What to do:
  1. Add `idempotency_key` field to action proposal schema
  2. Generate it as `sha256(action_type + arguments_digest + run_id)`
  3. Gate check: if risk_class is high and no idempotency_key → warn/defer
- Acceptance: Schema updated; gate check enforced

### P2 — Quality improvements

**GAK-05: Unified cross-file verification**
- Source: `ACTION_KERNEL_CONTRACT_v0.1.md` §5
- Problem: Each verifier works independently. No single `action-kernel-result.json` that cross-checks process result, session record, journal hash chain, and artifact digests together.
- Current partial coverage: `grok_session_verifier.py` does 3-way cross-check (receipt ↔ events.jsonl ↔ acp_transcript.jsonl) with 7 checks. This pattern should be generalized.
- What to do:
  1. Define `action-kernel-result-v0.1.schema.json` with cross-file verification fields
  2. Implement `verify_action_kernel()` that consumes all output files from a run
  3. Wire into runner receipt generation
- Acceptance: One function that reads all output files and produces a single verification summary

**GAK-06: Win32 cancellation staged escalation**
- Source: `存档/architecture/runtime-spikes/WINDOWS_RUNTIME_CONTRACT_v0.1.md` WIN-PROC-003
- Problem: Timeout handling in `windows_sandbox.py` skips CTRL_BREAK stage. Goes directly to Job Object close → TerminateProcess.
- What to do: Add CTRL_BREAK_EVENT to process group before job close, with a short grace period
- Acceptance: Timeout path records: method requested, actual escalation, exit code, terminal state — all separated

### P3 — Minor, low-effort

**GAK-07: DeepSeek preflight — deprecated model aliases**
- Source: `DEEPSEEK_ADAPTER_CONTRACT_v0.1.md` §2.1
- Problem: `adapter_preflight.py` does not reject `deepseek-chat` / `deepseek-reasoner` (retired 2026-07-24)
- What to do: Add explicit rejection of deprecated aliases in preflight
- Acceptance: Preflight fails with clear error for deprecated model names
- Effort: ~5 lines

**GAK-08: DeepSeek preflight — temperature + thinking mutual exclusion**
- Source: `DEEPSEEK_ADAPTER_CONTRACT_v0.1.md` §2.2
- Problem: Thinking mode ignores `temperature`/`top_p`/`presence_penalty`/`frequency_penalty`. Preflight should reject profiles that declare both.
- What to do: Add cross-field check in `adapter_preflight.py`
- Acceptance: Preflight fails when thinking mode + sampling params coexist
- Effort: ~10 lines

**GAK-09: /models capability discovery**
- Source: `DEEPSEEK_ADAPTER_CONTRACT_v0.1.md` §2.1
- Problem: Cannot confirm requested model ID exists at provider before making API call
- What to do: Add optional preflight check that calls DeepSeek `/models` endpoint (when network available) to validate model ID
- Acceptance: Preflight can optionally verify model availability
- Effort: ~30 lines

## 6. Already Verified — No Action Needed

These design requirements were flagged in the initial audit but are confirmed implemented:

| Requirement | Implementation file |
|------------|-------------------|
| EvidenceKernel (action/evidence/claim state machine) | `evidence_kernel.py` |
| ClaimBoundary (measured/direct_comparison/bridge_hypothesis) | `claim_boundary.py` |
| CaseRetrievalGuard (blind-first, precommitment before retrieval) | `case_retrieval_guard.py` |
| Global Progress Sentinel (direction tracking + WARN injection) | `global_progress_state.py`, `global_review_mode.py`, `diagnostic_coverage.py` |
| ScenarioExporter + EvaluationRunner (oracle isolation) | `scenario_exporter.py`, `evaluation_runner.py` |
| LeakScanner (dual-channel mechanical + semantic) | `leak_scanner.py` |
| Full retrieval pipeline (browser → PDF → evidence → claim) | `browser_retrieval.py`, `retrieval_workflow.py`, `retrieval_subagent.py`, `evidence_store.py`, `pdf_evidence.py`, `claim_binding.py` |
| FindingRegistry + Permission Record separation | `finding_registry.py` |
| AuditLedger (hash-chained, metadata-only, signed seal) | `audit.py`, `audit_integration.py` |
| Grok session cross-verification (receipt ↔ events ↔ transcript) | `grok_session_verifier.py` |
| Job Object creation-time assignment (GAK-WIN-001) | `windows_sandbox.py` (PROC_THREAD_ATTRIBUTE_JOB_LIST) |
| Grok ACP real tool loop (thinking continuity verified) | `integration/grok/README.md` confirmed |
| Loopback fake SSE provider | `scripts/fake_deepseek_provider.py` |

## 7. Summary

```
14 items initially flagged
  - 5 intentionally frozen (delegated to Grok)
  - 2 already covered by Grok
  ───────────────────────────────
  = 7 real gaps remaining
    P0 (2): thinking continuity probe, GateDecision systemization
    P1 (2): permission default audit, idempotency_key
    P2 (2): cross-file verification, Win32 cancellation
    P3 (3): deprecated model check, temp/thinking cross-check, /models discovery
```

## 8. References

- [`UPSTREAM_FIRST_INTEGRATION_v0.1.md`](UPSTREAM_FIRST_INTEGRATION_v0.1.md) — ownership boundary
- [`INTEGRATED_AGENT_ASSURANCE_DESIGN_v0.1.md`](INTEGRATED_AGENT_ASSURANCE_DESIGN_v0.1.md) — runtime-first architecture
- [`GROK_BUILD_ADAPTATION_v0.1.md`](GROK_BUILD_ADAPTATION_v0.1.md) — Grok capability matrix
- [`PROTOCOL_DRAFT_v0.1.md`](../protocol/PROTOCOL_DRAFT_v0.1.md) — protocol state machines and records
- [`ACTION_KERNEL_CONTRACT_v0.1.md`](ACTION_KERNEL_CONTRACT_v0.1.md) — execution order and cross-file verification
- [`WINDOWS_RUNTIME_CONTRACT_v0.1.md`](../存档/architecture/runtime-spikes/WINDOWS_RUNTIME_CONTRACT_v0.1.md) — process containment invariants
- [`DEEPSEEK_ADAPTER_CONTRACT_v0.1.md`](DEEPSEEK_ADAPTER_CONTRACT_v0.1.md) — DeepSeek-specific adapter requirements
