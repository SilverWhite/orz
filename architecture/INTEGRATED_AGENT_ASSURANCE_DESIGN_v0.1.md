# Integrated Agent Assurance Design v0.1

Status: synthesis / corrective design integration

This document preserves the existing design corpus and reorganizes it into one runtime-first architecture. It does not delete or supersede the existing ADRs, protocol drafts, UI specifications, audits, schemas, fixtures, or implementation notes. Its purpose is to make the intended ownership explicit: mature agent runtimes should carry the generic agent platform work, while this repository keeps the parts that are specific to evidence-constrained scientific assurance, local auditability, Windows operation, and the designed CLI/TUI experience.

## 1. Why This Integration Exists

The current repository contains a large amount of custom-built infrastructure. That is useful as probes, fixtures, contracts, and design scaffolding, but it creates the wrong product shape if it becomes the default production path. The product should not grow into a second self-built agent runtime.

The correction is:

- Preserve all key design parts that were already created.
- Preserve the UI functions and visual interaction design because they encode user-specific workflows.
- Make Grok CLI / ACP and comparable mature agent frameworks provide the model loop, session lifecycle, generic tool loop, permission surfaces, compaction, and baseline runtime behavior.
- Keep this repository focused on the assurance layer, adapter contracts, audit ledger, evidence visibility, scientific claim gates, runtime conformance, and the custom control surface.

The previous audits mostly verified whether local components were internally coherent. The missing question was ownership: whether the repository was accidentally implementing too much generic agent runtime itself. This document fixes that boundary.

## 2. Product Shape

The product is a runtime-first guarded agent workbench:

```text
User
  -> Designed CLI/TUI control surface
  -> Runtime control adapter
  -> Mature agent runtime, with Grok ACP as the first-class reference path
  -> Assurance gates and evidence/audit kernel around the runtime
  -> Scientific task protocols, evaluation, regression, and domain profiles
```

The custom UI remains the operator console. The mature runtime remains the execution engine. The assurance kernel remains the trust boundary.

## 3. Non-Negotiable Preservation Rules

1. Existing UI functionality and design intent are retained.
2. Existing general scientific assurance contracts are retained.
3. Existing LIF-specific layering is retained as a profile over the general assurance layer, not as the base runtime.
4. Existing audit documents remain provenance and gap evidence.
5. Existing fixtures remain conformance assets unless explicitly promoted through a decision record.
6. Fake, loopback, and offline paths are not deleted, but they are demoted to test fixtures unless they correspond to a real upstream gap.
7. No new generic production model loop, action kernel, permission engine, session runtime, or transport stack should be added while a mature upstream runtime can provide that function.

## 4. Integrated Ownership Model

### 4.1 Mature Agent Runtime Owns

The upstream runtime should own:

- Model request/response lifecycle.
- Conversation/session lifecycle.
- Tool-call loop and tool result routing.
- Generic command execution and repository operations.
- Permission UI/store when provided by upstream.
- Background tasks and cancellation primitives.
- Compaction mechanics.
- Base sandbox behavior and platform safety primitives.
- ACP or equivalent protocol implementation.

Grok CLI / Grok Build is the first reference runtime because existing design work already targets it. Other mature runtimes can be evaluated through the same adapter and gate model.

### 4.2 Runtime Adapter Owns

The adapter layer should be thin and testable. It owns:

- Launching and supervising the selected runtime.
- Capability discovery and version pinning.
- Translating runtime events into canonical observable events.
- Mapping runtime permissions, terminal states, tool results, and artifacts into assurance receipts.
- Detecting missing upstream capabilities.
- Running conformance probes against fixtures.
- Preventing fake/offline paths from masquerading as the production execution path.

The adapter must not become a hidden replacement runtime.

### 4.3 Assurance Kernel Owns

The assurance layer owns what mature generic runtimes do not own:

- Task contracts, research-task boundaries, and instruction provenance.
- Workspace trust and workspace-first discovery.
- Effective execution envelope and tool availability context.
- Source visibility and full-text evidence rules.
- Browser/PDF acquisition receipts and local evidence store.
- Claim/evidence gates and answer packet verification.
- Audit journal, verifier receipts, recovery checks, and append-only provenance.
- Runtime-neutral scientific assurance schemas.
- Evaluation partitioning, oracle isolation, scoring, and regression curation.
- Domain profile layering, including LIF.

This is the repository's durable value.

### 4.4 UI Owns

The CLI/TUI owns the operator experience:

- Explorer-style spatial model.
- Menu bar, toolbar, address/command bar, find bar, explorer pane, content pane, status bar, dialogs, and properties views.
- Chinese-first labels and copy where the existing UI design uses them.
- Big main-window mode.
- Address/command popup behavior.
- Find popup behavior.
- Chatroom-style collapsible task stream.
- Plan/checklist visibility.
- Gate, receipt, source, artifact, adapter, and journal-event inspection.
- Process usage and progress visibility.

The UI can orchestrate user intent and display assurance state. It must not own core model execution, evidence adjudication, or runtime policy.

### 4.5 Fixtures Own

Fixtures own proof and regression coverage:

- Fake providers.
- Loopback transports.
- Inert Grok fixtures.
- Checkpoint/delta probes.
- Disposable reproduction cases.
- Development challenge sets.

Fixtures are valuable because they make behavior testable. They are not product runtime unless a later ADR explicitly promotes them.

## 5. Preserved Design Areas

### 5.1 ADR And Positioning

The ADRs define the stable product intent:

- Evidence-constrained local agent kernel.
- Cloud runtime deferred.
- Runtime-neutral assurance kernel.
- General science profile layering.

The positioning documents add the corrective rule: use mature agent foundations first and keep this repository's special value in assurance.

### 5.2 Protocol, Evaluation, And Regression

The protocol draft preserves the distinction between model output, action, evidence, claim, artifact, and verifier receipt. The evaluation and regression documents preserve the idea that development fixtures are not valid holdout evidence, that scoring is multidimensional, and that oracle isolation is required before broad claims.

### 5.3 Assurance P0-P5

The P0-P5 audit series remains as the assurance maturity map:

- P0 canonical guarded CLI and core readonly slice.
- P1 conversation identity, archive, and lifecycle.
- P2 sandboxing and guarded execution.
- P3 instruction authority and capability posture.
- P4 audit, compaction, checkpoint, and recovery.
- P4.5 workspace-first integration.
- P5 synthetic user-task preflight.

These should drive gates around the runtime, not duplicate the runtime.

### 5.4 UI And Interaction Design

The UI design is preserved as a first-class surface. It is not merely a skin over a prototype. It captures the operator workflow needed by the user: spatial navigation, command discovery, plan visibility, source/claim inspection, status awareness, and low-friction control over long agent work.

The simplification supplement is retained as a guardrail: the UI should be familiar and efficient, not an overbuilt custom widget museum.

### 5.5 Grok And Upstream-First Runtime Strategy

Grok ACP / Grok Build remains the primary reference runtime path. The repository should use it to avoid reimplementing mature agent features. Candidate runtimes are judged by their ability to supply the generic platform responsibilities listed in section 4.1 and to emit enough observable state for the assurance layer.

Current 2026-07-30 upstream scan and follow-up promotion add these routing constraints:

- The installed PATH `grok` reports `0.2.112 (9bbd559437) [stable]`, and the checked-in promoted lock now points at the verified `0.2.112` binary. Treat `windows_child_tree_baseline_regression` as passed for default-version selection, and `windows_child_tree_owned_cleanup` as a carried-forward limitation rather than proof of Grok-owned timeout cleanup.
- Grok Build is now an open-source upstream base with its own TUI, ACP/headless entrypoints, session/runtime loop, tool dispatch, permissions, sandbox, hooks, MCP, plugins, skills, subagents, workflows, checkpoint/workspace behavior, and configuration surface. These are runtime-owned unless a concrete audit proves a missing capability.
- Grok configuration now exposes useful graft points such as custom models, custom query parameters, environment-sourced HTTP headers, `.grok/agents`, `.grok/workflows`, `.grok/hooks`, MCP, permissions, and subagent/workflow toggles. Local product code should consume or wrap these surfaces, not duplicate them.
- The two retained retrieval subagent designs should be mapped to Grok-native subagent/workflow surfaces where possible, while preserving this repository's capability receipts, child capability enforcement, evidence gates, and redaction rules.
- The preserved UI should display real Grok workflow/subagent/approval/runtime state through normalized events. It must not become a second scheduler or model loop.
- `gsa run --runtime grok` is now an explicit fail-closed promotion gate, not a production prompt/tool launcher. It records the selected retrieval mode, attached tool availability evidence, and timeout split evidence while preserving the canonical default path.
- Claude Code, Gemini CLI, Qwen Code, Goose, Codex CLI, and Aider remain reference frameworks for specific mature patterns: scoped subagents, hooks, MCP/extension discovery, recipes/workflows, sandbox/approval UX, and git-aware edit loops. They are design references, not a reason to switch the primary base away from Grok.

Reference links for this scan:

- [Grok Build repository](https://github.com/xai-org/grok-build)
- [Grok Build open-source announcement](https://x.ai/news/grok-build-open-source)
- [Grok Build overview](https://docs.x.ai/build/overview)
- [Grok Build configuration](https://github.com/xai-org/grok-build/blob/main/crates/codegen/xai-grok-pager/docs/user-guide/05-configuration.md)
- [Claude Code subagents](https://code.claude.com/docs/en/sub-agents)
- [Gemini CLI](https://github.com/google-gemini/gemini-cli)
- [Qwen Code](https://github.com/QwenLM/qwen-code)
- [Goose](https://github.com/block/goose)
- [OpenAI Codex CLI](https://github.com/openai/codex)
- [Aider](https://github.com/aider-ai/aider)

### 5.6 DeepSeek And Provider Bridge

The DeepSeek work is retained as a narrow provider bridge and conformance target. It should not expand into a second generic transport/runtime layer. Provider-private reasoning must remain private; audit records can store digests and derived receipts, not raw hidden reasoning.

### 5.7 Browser, Retrieval, And PDF Evidence

The local browser/PDF design is retained because mature generic runtimes usually do not solve scientific source visibility deeply enough. The browser broker, AI-owned tabs, evidence store, source levels, PDF hashing, and no-credential/no-history boundaries remain part of the assurance layer.

### 5.8 Global Progress, Orientation, And Checklist

Global progress state, orientation guard, plan mode, task checklist, and process usage monitor are retained as operator and audit affordances. They should reflect real runtime events and assurance decisions, not create a second execution engine.

### 5.9 Windows Runtime

Windows-first process supervision, Job Object behavior, and native sandbox audits remain important because the user environment is Windows. They should wrap or supervise mature runtimes where needed, not replace their core logic.

### 5.10 Audit Documents

Audit documents remain design evidence and implementation provenance. They are not thrown away. They also should not be treated as permission to keep expanding custom infrastructure after the ownership correction in this document.

## 6. Main Runtime Path

The integrated default path should be:

1. Select or pin the mature runtime candidate, initially Grok ACP / Grok Build.
2. Run capability discovery and version checks.
3. Establish workspace trust before tool discovery or task execution.
4. Accept user intent through the designed CLI/TUI.
5. Convert user intent into plan/checklist state when appropriate.
6. Launch the mature runtime through the adapter.
7. Stream runtime events into canonical observable events.
8. Apply assurance gates at task, tool, evidence, source, permission, and answer boundaries.
9. Persist journal events, artifacts, receipts, and verifier outcomes.
10. Render the resulting state through the UI.

This path makes the mature runtime do the hard generic agent work while preserving every custom assurance and UI design.

## 7. What Must Stop Expanding As Product Runtime

The following areas should be frozen as fixtures, conformance probes, or narrow adapters unless an explicit gap review proves that upstream cannot provide the needed behavior:

- Self-built production action kernel.
- Self-built production model loop.
- Self-built generic session manager.
- Self-built generic permission broker.
- Loopback transport as a main execution path.
- Fake provider as a main execution path.
- Generic command runner beyond runtime supervision and assurance wrapping.
- New parallel protocol stacks that duplicate ACP or a selected mature runtime protocol.

This does not delete the code. It changes its role.

## 8. UI Preservation Contract

The UI must retain:

- Menu bar.
- Toolbar.
- Address/command bar.
- Find bar.
- Explorer pane.
- Content pane.
- Status bar.
- Dialogs.
- Properties views.
- Big main-window mode.
- Chinese-first operator labels where already designed.
- Task stream with collapsible details.
- Plan/checklist strip.
- Gate, source, claim, artifact, adapter, journal, and verifier views.
- Process usage visibility.
- Runtime state visibility.

The UI may be rewired so that these views reflect a real mature runtime and assurance events. Rewiring is allowed; feature deletion is not.

## 9. Decision Rules For Future Work

Before adding or expanding any subsystem, ask:

1. Is this generic agent runtime functionality?
2. Does Grok ACP / Grok Build or another mature runtime already provide it?
3. Is the local code only needed to adapt, observe, verify, or hard-gate that runtime?
4. Can an existing fixture test the boundary instead of becoming product code?
5. Does the change preserve the designed UI and assurance semantics?

If the answer to question 1 is yes and question 2 is also yes, local code should be an adapter or test, not a new implementation.

## 10. Source Coverage Ledger

The following project design, audit, protocol, fixture, and index documents were included in this integration pass. This ledger is intentionally explicit so later reviews can detect omissions.

### Root

- `CLI_PROJECT_INDEX.md`
- `README.md`

### ADR

- `adr/ADR-0001-evidence-constrained-local-agent-kernel.md`
- `adr/ADR-0002-defer-cloud-runtime.md`
- `adr/ADR-0003-runtime-neutral-assurance-kernel.md`
- `adr/ADR-0004-general-science-profile-layering.md`

### Architecture

- `architecture/ACTION_KERNEL_CONTRACT_v0.1.md`
- `architecture/CLI_UI_INTERACTION_MODEL_v0.1.md`
- `architecture/CLI_UI_SIMPLIFICATION_SUPPLEMENT_v0.1.md`
- `architecture/DEEPSEEK_ADAPTER_CONTRACT_v0.1.md`
- `architecture/DEEPSEEK_EXTERNAL_TRANSPORT_SAFETY_v0.1.md`
- `architecture/D_SALVAGE_MATRIX_v0.1.md`
- `architecture/GENERAL_ASSURANCE_KERNEL_GAP_REGISTER_v0.1.md`
- `architecture/GLOBAL_PROGRESS_SENTINEL_CONTRACT_v0.1.md`
- `architecture/GROK_BUILD_ADAPTATION_v0.1.md`
- `architecture/INTERACTIVE_APPROVAL_LEDGER_CONTRACT_v0.1.md`
- `architecture/LOCAL_BROWSER_RETRIEVAL_AND_PDF_EVIDENCE_v0.1.md`
- `architecture/LOOPBACK_TRANSPORT_AND_PRIVATE_TRANSCRIPT_v0.1.md`
- `architecture/MATURE_AGENT_DESIGN_DECOMPOSITION_v0.2.md`
- `architecture/MODEL_ADAPTER_LOOP_CONTRACT_v0.1.md`
- `architecture/NETWORK_PERMIT_BROKER_CONTRACT_v0.1.md`
- `architecture/OBSERVABLE_EXECUTION_ENVELOPE_v0.1.md`
- `architecture/OPEN_SOURCE_AGENT_GAP_AUDIT_v0.1.md`
- `architecture/PLAN_MODE_AND_PROCESS_USAGE_MONITOR_v0.1.md`
- `architecture/PRODUCT_POSITIONING_AND_REFERENCE_STRATEGY_v0.1.md`
- `architecture/TASK_CHECKLIST_ANNOUNCEMENT_SUPPLEMENT_v0.1.md`
- `architecture/UPSTREAM_FIRST_INTEGRATION_v0.1.md`
- `architecture/UPSTREAM_VERSION_STRATEGY_v0.1.md`
- `architecture/WINDOWS_RUNTIME_CONTRACT_v0.1.md`

### Assurance

- `assurance/README.md`
- `assurance/fixtures/general_science/computational_decay/source-notes.md`

### Docs Audit And Implementation Notes

- `docs/ADAPTER_GATE_AUDIT_2026-07-28.md`
- `docs/CANONICAL_CLI_QUICKSTART_2026-07-26.md`
- `docs/CANONICAL_GUARDED_CLI_P0_AUDIT_2026-07-26.md`
- `docs/CLI_LIFECYCLE_SOURCE_COMPARISON_AND_CODEX_DIRECTION_2026-07-25.md`
- `docs/CLI_SESSION_LIFECYCLE_ADAPTER_AUDIT_2026-07-25.md`
- `docs/CODEX_APP_SERVER_LIFECYCLE_NORMALIZER_AUDIT_2026-07-25.md`
- `docs/DEEPSEEK_CREDENTIAL_HARDENING_2026-07-23.md`
- `docs/DEEPSEEK_REAL_DEVELOPMENT_PROBE_2026-07-23.md`
- `docs/GAK_INJ_001_AUDIT_2026-07-27.md`
- `docs/GAK_SBX_001_WINDOWS_NATIVE_SANDBOX_AUDIT_2026-07-27.md`
- `docs/GENERAL_SCIENTIFIC_ASSURANCE_GAP_REVIEW_2026-07-25.md`
- `docs/GLOBAL_PROGRESS_ATOMIC_JOURNAL_AND_REASON_MIGRATION_AUDIT_2026-07-25.md`
- `docs/GLOBAL_PROGRESS_CHECKPOINT_ADAPTER_AUDIT_2026-07-25.md`
- `docs/GLOBAL_PROGRESS_CONTROLLER_VERIFIER_AUDIT_2026-07-25.md`
- `docs/GLOBAL_PROGRESS_HOLISTIC_GATE_AUDIT_2026-07-25.md`
- `docs/GLOBAL_PROGRESS_JOURNAL_RECOVERY_AUDIT_2026-07-25.md`
- `docs/GLOBAL_PROGRESS_RUNTIME_CONTROLLER_AUDIT_2026-07-25.md`
- `docs/GLOBAL_PROGRESS_SENTINEL_SPIKE_2026-07-21.md`
- `docs/GLOBAL_PROGRESS_STATE_REDUCER_AUDIT_2026-07-25.md`
- `docs/GLOBAL_PROGRESS_TRANSITION_GATE_AUDIT_2026-07-25.md`
- `docs/GROK_ACP_FAKE_TOOL_SPIKE_2026-07-23.md`
- `docs/GROK_ACP_INITIALIZE_AUDIT_2026-07-21.md`
- `docs/GROK_CLI_SPECIALIZATION_ADAPTATION_AUDIT_2026-07-30.md`
- `docs/GROK_COMPACTION_PROVENANCE_2026-07-23.md`
- `docs/GROK_EVENT_BRIDGE_AUDIT_2026-07-21.md`
- `docs/GROK_FAKE_PROVIDER_AUDIT_2026-07-21.md`
- `docs/GROK_FIXTURE_CHECKPOINT_DELTA_SPIKE_2026-07-21.md`
- `docs/GROK_REAL_DEEPSEEK_LAUNCHER_2026-07-23.md`
- `docs/GROK_TOOL_CONTINUITY_AUDIT_2026-07-21.md`
- `docs/GROK_UPSTREAM_CANDIDATE_AUDIT_2026-07-23.md`
- `docs/GROK_UPSTREAM_PROMOTION_2026-07-23.md`
- `docs/GROK_WINDOWS_BINARY_AUDIT_2026-07-21.md`
- `docs/GROK_WINDOWS_CHILD_TREE_PROBE_2026-07-23.md`
- `docs/GROK_WORKSPACE_TRUST_AUDIT_2026-07-21.md`
- `docs/GSA_ARTIFACT_SCHEMA_REGISTRATION_AUDIT_2026-07-25.md`
- `docs/GSA_CORE_READONLY_SLICE_AUDIT_2026-07-25.md`
- `docs/GSA_CROSS_ARTIFACT_LINEAGE_COMPARABILITY_AUDIT_2026-07-25.md`
- `docs/GSA_DISPOSABLE_REPRODUCTION_AUDIT_2026-07-26.md`
- `docs/GSA_SELF_QUESTION_COUNTEREXAMPLE_DESIGN_2026-07-26.md`
- `docs/GSA_VALIDATOR_BRIDGE_AUDIT_2026-07-25.md`
- `docs/ORIENTATION_RUNTIME_GUARD_AUDIT_2026-07-28.md`
- `docs/P1_CONVERSATION_ARCHIVE_LIFECYCLE_AUDIT_2026-07-28.md`
- `docs/P2_5_GUARDED_EXECUTION_AUDIT_2026-07-24.md`
- `docs/P2_DOCKER_SANDBOX_AUDIT_2026-07-24.md`
- `docs/P3_INSTRUCTION_AUTHORITY_AUDIT_2026-07-25.md`
- `docs/P4_5_WORKSPACE_FIRST_INTEGRATION_AUDIT_2026-07-25.md`
- `docs/P4_AUDIT_COMPACTION_RECOVERY_AUDIT_2026-07-25.md`
- `docs/P5_SYNTHETIC_USER_TASK_PREFLIGHT_2026-07-25.md`
- `docs/REPOSITORY_AUDIT_2026-07-21.md`
- `docs/RETRIEVAL_SUBAGENT_AUDIT_2026-07-27.md`
- `docs/SOURCE_FULLTEXT_VISIBILITY_RULE_2026-07-26.md`
- `docs/TOOL_AVAILABILITY_GATE_AUDIT_2026-07-27.md`

### Evaluation

- `evaluation/HUMAN_BASELINE_PROTOCOL_v0.1.md`
- `evaluation/PARTITION_AND_ORACLE_ISOLATION_v0.1.md`
- `evaluation/SCORING_PROTOCOL_v0.1.md`

### Integration

- `integration/grok/README.md`
- `integration/grok/fixtures/workspace-control-surfaces/.grok/skills/inert-fixture/SKILL.md`
- `integration/grok/fixtures/workspace-control-surfaces/AGENTS.md`

### Protocol

- `protocol/PROTOCOL_DRAFT_v0.1.md`

### Prototype

- `prototype/README.md`

### Regression

- `regression/CURATION_PROTOCOL_v0.1.md`

### Runtime

- `runtime/SCENARIO_EXPORT_AND_RUNNER_CONTRACT_v0.1.md`

## 11. Immediate Implementation Consequences

The next implementation work should be ordered around the corrected ownership boundary:

1. Make the real Grok ACP / mature runtime path the main integration slice.
2. Reclassify fake, loopback, and local-only runtime paths as fixtures or conformance probes.
3. Wire the preserved UI to runtime events and assurance receipts instead of local fake execution.
4. Keep assurance gates strict and runtime-neutral.
5. Add explicit labels in code/docs for `runtime-owned`, `adapter-owned`, `assurance-owned`, `ui-owned`, and `fixture-only` modules.
6. Update audits to check ownership drift, not only local correctness.
7. Keep the Grok `0.2.112` promotion boundary explicit: lock metadata and core conformance checks are complete, `windows_child_tree_baseline_regression` is passed, and `windows_child_tree_owned_cleanup` remains a carried limitation before prompt/tool promotion.
8. Convert the retained retrieval subagents into Grok-compatible agent/workflow profiles before adding any new local subagent scheduler.
9. Keep `gsa run --runtime grok` fail-closed until tool availability is `allow` and owned cleanup is `passed`; the command must keep writing receipts without launching prompt/tool execution while those prerequisites are absent.

The desired end state is not less design. It is the same design, with the generic agent platform work moved back onto mature foundations.
