# Runtime-First Follow-up Alignment

Date: 2026-07-30

Status: alignment complete; retrieval mode plumbing and two retrieval profile
drafts implemented; remaining implementation slices remain explicit follow-ups

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
| Retrieval mode selection | assurance-owned policy around runtime tools | The only valid modes are `local_browser`, `framework_fallback`, and `off`. Runtime tool calls may perform retrieval, but the selected mode and receipt semantics remain explicit. | First CLI/TUI/receipt plumbing is complete; prompt/tool execution remains blocked until containment and follow-up observation gates land. |
| Project-document retrieval subagent | Grok-compatible profile plus assurance wrapper | One project-doc retrieval agent draft now exists at `.grok/agents/gsa-project-doc-retrieval.md`. It may read project docs and produce retrieval/capability receipts. It must not become a local scheduler. | Workflow Rhai draft remains blocked until concrete syntax is verified. |
| External retrieval subagent | Grok-compatible profile plus assurance wrapper | One external retrieval agent draft now exists at `.grok/agents/gsa-external-retrieval.md`. Network capability, source visibility, PDF evidence, redaction, and fallback mode receipts remain in this repository. | Runtime event capture for network/tool permission state. Workflow Rhai draft remains blocked until concrete syntax is verified. |
| Tool availability | Grok registry/permission observed through assurance receipts | Grok owns tool registry, tool dispatch, and permission UX. This repository owns the explicit availability receipt, UI/prompt visibility, and tool-belief stagnation checks. | First observation slice is complete: `gsa grok observe-tools` records static registry/permission controls and projects them into the existing tool availability gate. ACP permission semantics remain degraded unless a valid fake-tool verification receipt is attached. |
| Global Progress / Orientation | assurance-owned guard over Grok workflow state | Grok workflow state can be consumed as input. Global Progress, Orientation Guard, neutral inquiry, counterexample gate, and diagnostic coverage decisions remain outside Grok workflow policy. | First lifecycle projection slice is complete: metadata-only Grok workflow/subagent events can be projected into GPS journal events and neutral Orientation context. |
| UI projection | UI-owned workbench consuming normalized runtime events | The TUI should display Grok workflow/subagent/tool/permission/runtime events and assurance receipts. It must not own scheduling, model loop, tool dispatch, or session persistence. | Expand beyond the first-slice `version-smoke` projection after stronger containment. |
| Prototype references | fixture-only / git-history reference | `prototype/` is retired and contains only a boundary README. Active support code now lives in `assurance/`, `runtime/`, `integration/grok/`, scripts, schemas, and tests. | Clean remaining historical audit prose only when it appears in active routing or quickstarts. |

## Non-Goals

- Do not replace `gsa run` with Grok yet.
- Do not add a local multi-agent scheduler.
- Do not check in speculative `.grok/agents` or `.grok/workflows` files until
  their syntax is verified against the selected Grok build.
- Do not treat hooks as hard gates; hooks may observe or annotate only.
- Do not treat the carried-forward `windows_child_tree_timeout` limitation as a
  passed Grok-owned cleanup guarantee.

## Next Implementation Order

1. Done: add explicit retrieval mode plumbing for Grok prompt/tool runs.
2. Done: verify Grok agent-profile discovery and add exactly two retrieval
   profile drafts. Workflow Rhai drafts remain blocked until concrete syntax is
   verified.
3. Done: add Grok tool registry and permission observation receipts.
   `gsa grok observe-tools --include-tool-availability` observes
   `grok inspect --json`, `grok --help`, and `grok agent --help` without
   prompt/tool execution. Without attached ACP fake-tool verification, the
   observation receipt is valid but the availability projection remains
   `block` because permission-request semantics are still degraded.
4. Done: map Grok workflow/subagent lifecycle events into Global Progress and
   Orientation inputs. `grok_lifecycle_projection` converts supported
   metadata-only workflow/subagent events into GPS journal events and a neutral
   `[GROK_LIFECYCLE_CONTEXT v0.1]` block. It fail-closes on unsupported event
   types, content-bearing fields, or step/direction binding mismatches.
5. Split `windows_child_tree_timeout` into baseline-regression and Grok-owned
   cleanup gates before promoting prompt/tool mode.

This alignment is deliberately narrow: it updates ownership and route status so
the next implementation slice has a clean target.
