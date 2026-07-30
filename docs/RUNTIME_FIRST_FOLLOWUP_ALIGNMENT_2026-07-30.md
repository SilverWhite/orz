# Runtime-First Follow-up Alignment

Date: 2026-07-30

Status: alignment complete; retrieval mode plumbing, two retrieval profile drafts,
tool/permission observation, lifecycle projection, timeout gate split,
fail-closed Grok prompt/tool promotion gate, and adapter-side Job Object
containment (P1) implemented; actual prompt/tool execution remains unpromoted

## Purpose

This note closes the document-status alignment step after the Grok runtime first
slice. It does not promote the prompt/tool execution path. It records which
follow-up surfaces are now runtime-owned, adapter-owned, assurance-owned, or
fixture-only so later work does not rebuild a local generic agent runtime by
accident.

Source routes checked:

- `CLI_PROJECT_INDEX.md`
- `docs/CN_AGENT_BASE_GRAFT_VALUE_REVIEW_2026-07-30.md`
- `docs/GROK_CLI_SPECIALIZATION_ADAPTATION_AUDIT_2026-07-30.md`
- `architecture/INTEGRATED_AGENT_ASSURANCE_DESIGN_v0.1.md`
- `architecture/GROK_BUILD_ADAPTATION_v0.1.md`
- `runtime/SCENARIO_EXPORT_AND_RUNNER_CONTRACT_v0.1.md`
- `prototype/README.md`

## Alignment Decisions

| Surface | Current owner | Alignment decision | Still pending |
|---|---|---|---|
| Retrieval mode selection | assurance-owned policy around runtime tools | The only valid modes are `local_browser`, `framework_fallback`, and `off`. Runtime tool calls may perform retrieval, but the selected mode and receipt semantics remain explicit. | First CLI/TUI/receipt plumbing is complete; prompt/tool execution remains blocked until Grok-owned cleanup or equivalent stronger containment is proven. |
| Project-document retrieval subagent | Grok-compatible profile plus assurance wrapper | One project-doc retrieval agent draft now exists at `.grok/agents/gsa-project-doc-retrieval.md`. It may read project docs and produce retrieval/capability receipts. It must not become a local scheduler. | P2 Rhai syntax verified: `gsa-retrieval.rhai` workflow draft written using Grok 0.2.112 bundled `create-workflow` skill reference. Full smoke-check requires live Grok prompt/tool session. |
| External retrieval subagent | Grok-compatible profile plus assurance wrapper | One external retrieval agent draft now exists at `.grok/agents/gsa-external-retrieval.md`. Network capability, source visibility, PDF evidence, redaction, and fallback mode receipts remain in this repository. | Runtime event capture for network/tool permission state. Workflow draft verified for Rhai syntax; full smoke-check pending. |
| Tool availability | Grok registry/permission observed through assurance receipts | Grok owns tool registry, tool dispatch, and permission UX. This repository owns the explicit availability receipt, UI/prompt visibility, and tool-belief stagnation checks. | P3 ACP dual verification closed: both `allow_once` and `cancel_permission` fake-tool verifications required for `decision: allow`. End-to-end test proves full promotion chain (dual ACP + containment → allow). `--output-gate-receipt` writes gate receipt for promotion gate consumption. |
| Global Progress / Orientation | assurance-owned guard over Grok workflow state | Grok workflow state can be consumed as input. Global Progress, Orientation Guard, neutral inquiry, counterexample gate, and diagnostic coverage decisions remain outside Grok workflow policy. | First lifecycle projection slice is complete: metadata-only Grok workflow/subagent events can be projected into GPS journal events and neutral Orientation context. |
| UI projection | UI-owned workbench consuming normalized runtime events | The TUI should display Grok workflow/subagent/tool/permission/runtime events and assurance receipts. It must not own scheduling, model loop, tool dispatch, or session persistence. | Expand beyond the first-slice `version-smoke` projection after stronger containment. |
| Prototype references | fixture-only / git-history reference | `prototype/` is retired and contains only a boundary README. Active support code now lives in `assurance/`, `runtime/`, `integration/grok/`, scripts, schemas, and tests. | Clean remaining historical audit prose only when it appears in active routing or quickstarts. |
| Windows child-tree timeout | split candidate/prompt-tool gate | `windows_child_tree_baseline_regression` is now separate from `windows_child_tree_owned_cleanup`. The baseline-regression gate can pass for default-version selection while owned-cleanup remains `carried-limitation`. | P1 adapter-side Job Object containment implemented: `JobObjectSupervisor` wraps Grok root process in a Kill-On-Close Job, providing equivalent containment without Grok-owned cleanup. `grok_prompt_tool_gate` now accepts `adapter_containment_provided` as sufficient for `containment_requirement_satisfied`. |
| Grok prompt/tool run entry | explicit fail-closed promotion gate | `gsa run --runtime grok` now writes a `grok_prompt_tool_promotion_gate_receipt` and preserves the canonical default. It does not launch Grok prompt/tool execution. | The gate is `allow` when: tool availability is `allow` AND adapter containment is active (Job Object assigned) OR Grok-owned cleanup is proven. `--grok-adapter-receipt` accepts the runtime adapter receipt as containment evidence. |
| Adapter Job Object containment | adapter-side process tree supervision | `JobObjectSupervisor` wraps every Grok launch in a Kill-On-Close Job Object. Phase 1 uses post-creation `AssignProcessToJobObject`; GAK-WIN-001 `PROC_THREAD_ATTRIBUTE_JOB_LIST` planned for prompt/tool refactor. The adapter receipt records `job_object_created/assigned`, `containment_provider`, and `residue_scan_scope`. | First slice complete: `version_smoke` runs contained; promotion gate accepts adapter containment. Full prompt/tool mode still requires containment + tool availability + a new adapter mode. |

## Non-Goals

- Do not replace `gsa run` with Grok yet.
- Do not add a local multi-agent scheduler.
- Do not check in speculative `.grok/agents` or `.grok/workflows` files until
  their syntax is verified against the selected Grok build.
- Do not treat hooks as hard gates; hooks may observe or annotate only.
- Do not treat the carried-forward `windows_child_tree_owned_cleanup`
  limitation as a passed Grok-owned cleanup guarantee.

## Next Implementation Order

1. Done: add explicit retrieval mode plumbing for Grok prompt/tool runs.
2. Done: verify Grok agent-profile discovery and add exactly two retrieval
   profile drafts. Workflow Rhai drafts remain blocked until concrete syntax is
   verified.
3. Done: add Grok tool registry and permission observation receipts.
   `gsa grok observe-tools --include-tool-availability` observes
   `grok inspect --json`, `grok --help`, and `grok agent --help` without
   prompt/tool execution. Without attached valid ACP fake-tool verification
   for both `allow_once` and `cancel_permission`, the observation receipt is
   valid for static observation but the availability projection remains
   `block` because permission-request semantics are still degraded.
4. Done: map Grok workflow/subagent lifecycle events into Global Progress and
   Orientation inputs. `grok_lifecycle_projection` converts supported
   metadata-only workflow/subagent events into GPS journal events and a neutral
   `[GROK_LIFECYCLE_CONTEXT v0.1]` block. It fail-closes on unsupported event
   types, content-bearing fields, or step/direction binding mismatches.
5. Done: split `windows_child_tree_timeout` into
   `windows_child_tree_baseline_regression` and
   `windows_child_tree_owned_cleanup`. The split receipt keeps
   `prompt_tool_promotion_ready=false` while owned cleanup remains a
   carried limitation.
6. Done: add `gsa run --runtime grok` as an explicit fail-closed
   prompt/tool promotion gate. The receipt records retrieval mode, attached
   tool availability evidence, timeout split evidence, blocking reasons, and
   `prompt_tool_execution_attempted=false`; the canonical `gsa run` default is
   unchanged.

7. Done (P1): add adapter-side Job Object containment.

8. Done (P2): verify Grok Rhai workflow syntax and write retrieval workflow
   draft.

9. Done (P3): close ACP permission dual verification gap.
   `gsa grok observe-tools --output-gate-receipt` writes the gate receipt
   for promotion gate consumption.  End-to-end test proves that dual ACP
   verification (allow_once + cancel_permission) plus adapter containment
   produces `decision: "allow"` in the promotion gate.  The tool
   availability gate now supports full `allow` when both scenarios are
   verified.  Full Rhai reference obtained from Grok 0.2.112 bundled
   `create-workflow` skill.  `.grok/workflows/gsa-retrieval.rhai` drafts
   a parallel project-doc + external retrieval workflow.  Profile draft
   verifier updated to accept Rhai workflows with structural validation
   instead of rejecting them as premature.
   `assurance/job_object_supervisor.py` provides `JobObjectSupervisor` —
   Kill-On-Close Job Object wrapping the Grok root process.  The adapter
   assigns every Grok launch to the Job; closing the handle kills all
   remaining children.  The promotion gate accepts
   `adapter_containment_provided` as an alternative containment path.
   Phase 1 uses post-creation `AssignProcessToJobObject`; GAK-WIN-001
   `PROC_THREAD_ATTRIBUTE_JOB_LIST` upgrade planned for prompt/tool
   refactor.

This alignment is deliberately narrow: it updates ownership and route status so
the next implementation slice has a clean target.
