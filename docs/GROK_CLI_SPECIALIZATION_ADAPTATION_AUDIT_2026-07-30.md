# Grok CLI Specialization Adaptation Audit

Date: 2026-07-30
Status: preliminary graft map / implementation routing audit

## 1. Verdict

The current design is compatible with a mature-agent-framework strategy, and Grok CLI / Grok ACP is the best first graft target already represented in this repository.

The main mismatch is implementation routing: the design says Grok should own model/session/tool/runtime behavior, but the product entrypoints still route through the local canonical fake or direct DeepSeek path. The Grok work exists mostly as conformance scripts, schemas, fixtures, and audit evidence. It has not yet been promoted into a thin product runtime adapter.

Therefore the next implementation should not add another agent loop. It should create a narrow Grok runtime adapter that launches the locked Grok binary, normalizes Grok/ACP/session events, and hands those events to the existing assurance and TUI surfaces.

## 1.1 External Scan Update: 2026-07-30

This update records the follow-up scan requested after the preliminary graft map.

Observed local state:

- `grok --version` on PATH now reports `grok 0.2.112 (9bbd559437) [stable]`.
- Follow-up gates promoted `upstream/grok-build.lock.json` to the verified local binary `0.2.112 (9bbd559437)`.
- Therefore `0.2.112` is now the project default lock, with `windows_child_tree_timeout` recorded only as a carried-forward limitation rather than a proven Grok-owned cleanup guarantee.

Grok Build surfaces that should now be treated as first-class graft points:

- Official open-source upstream repository and documentation for Grok Build.
- TUI, headless mode, and ACP-style embedded runtime entrypoints.
- Session/runtime loop, tool dispatch, permission surfaces, sandbox behavior, checkpoint/workspace behavior, hooks, MCP, plugins, skills, subagents, workflows, and configuration.
- Configurable custom models, custom query parameters, and environment-sourced HTTP headers.
- `.grok/agents`, `.grok/workflows`, `.grok/hooks`, project-scoped MCP/plugins/permissions, `GROK_SUBAGENTS`, and `GROK_WORKFLOWS`.

Implementation consequence:

- The local adapter should become thinner than this document originally implied. It should prefer Grok's native config, subagent, workflow, hook, MCP, permission, and ACP surfaces before creating any local equivalent.
- The two retained retrieval subagents should be mapped onto Grok-native subagent/workflow profiles where possible, with this repository retaining capability receipts, child capability enforcement, source/evidence gates, and redaction.
- UI work should consume normalized Grok runtime/workflow/subagent/approval events. It should not own scheduling, agent spawning, model transport, session persistence, or tool dispatch.

Other mature frameworks remain useful as reference patterns:

- Claude Code: scoped subagents, specialized context, per-subagent tool permissions, hooks, and MCP boundaries.
- Gemini CLI and Qwen Code: terminal-agent extensibility, MCP/extension discovery, provider/model adaptation, and reusable agent/skill surfaces.
- Goose: MCP-native extensions, recipes/workflows, subagents, compaction, and ACP provider shape.
- OpenAI Codex CLI: sandbox/approval and local terminal-agent operating model.
- Aider: git-aware edit loop, repo map, lint/test integration, and prompt caching.

References:

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

## 2. Current Fit Against Grok CLI

### Good Fit

- `upstream/grok-build.lock.json` pins a verified local Grok binary: `0.2.112`, build `9bbd559437`, SHA-256 `2469bd182af212c7fcb84f2981999e4e8a6a7a2e4172bad3ae7f787a1f11407c`.
- PATH discovery and the promoted lock now agree on `grok 0.2.112 (9bbd559437) [stable]`; the remaining timeout issue is tracked as a carried-forward limitation, not a relative upgrade blocker.
- `scripts/inspect_grok_install.ps1` validates the locked binary identity and accepts stable-channel suffixes such as `[stable]` on the version string.
- `integration/grok/` already contains the correct adaptation assets: DeepSeek custom model config, ACP initialize probe, fake tool/permission/cancel probe, workspace trust probe, event bridge, child-tree probe, compaction provenance probe, and fake provider conformance.
- `assurance/tui/events.py` already has event types for model requests, model output, tool proposals, tool lifecycle, permission decisions, plan/checklist, usage samples, and terminal states.
- `assurance/tui/event_source.py` already abstracts event streams behind `EventSource`; this is the right place to add a Grok-backed live source without changing widgets.
- `assurance/tui/bridge.py` is already the sanctioned crossing point from TUI into assurance/runtime code.
- `assurance/adapter_gate.py`, `assurance/workspace_trust.py`, `assurance/tool_availability_gate.py`, `assurance/source_visibility.py`, and `assurance/runtime_preflight.py` already provide most of the assurance gates that should wrap Grok.

### Current Mismatch

- `gsa run` still executes `run_canonical_guarded_cli` or `run_canonical_guarded_cli_real`; this is a local fake/direct-DeepSeek path, not a Grok runtime path.
- `gsa tui --run` uses `assurance.tui.bridge.build_live_run_fn`, which also calls the canonical fake/direct-DeepSeek path.
- Grok launcher logic exists as PowerShell scripts, not as an importable Python adapter that the CLI/TUI can call consistently.
- `scripts/build_grok_event_bridge.py` produces a Grok-specific bridge event schema, but those events are not yet normalized into `runtime/run-event-v0.1.schema.json` or the TUI's `TuiEvent` stream.
- `runtime/SCENARIO_EXPORT_AND_RUNNER_CONTRACT_v0.1.md` still describes `prototype/` as a development probe; the active repo now keeps only `prototype/README.md` as a retired boundary marker.
- The current UI can display permission/tool events, but there is no production interactive ACP permission bridge connected to Grok.

## 3. Primary Graft Points

### 3.1 Runtime Adapter

Add:

- `assurance/grok_runtime_adapter.py`
- `assurance/grok_event_normalizer.py`

Responsibilities:

- Read and validate `upstream/grok-build.lock.json`.
- Call the local binary inspection path before launch.
- Build an isolated Grok HOME/profile/temp/workspace.
- Run workspace trust preflight before launching Grok.
- Start Grok in headless or ACP mode.
- Capture stdout/stderr/session artifacts as metadata-only receipts.
- Normalize Grok ACP/session records into canonical runtime events.
- Return a receipt compatible with existing verification and TUI replay.

This adapter must not implement model transport, session state, tool loop, permission policy, or compaction itself.

### 3.2 CLI Entrypoints

Modify:

- `assurance/cli.py`

Add:

- `gsa grok doctor`
- `gsa grok run --mode fake|deepseek --ask ... --run-root ...`
- Optional later: `gsa grok bridge --run-root ...`

Also change the existing run command only after the Grok path is stable:

- `gsa run --runtime canonical|grok`
- default remains explicit until Grok smoke and TUI wiring pass.

### 3.3 TUI Live Source

Modify:

- `assurance/tui/main.py`
- `assurance/tui/event_source.py`
- `assurance/tui/bridge.py`
- `assurance/tui/projector.py`

Add:

- `--runtime grok`
- `GrokLiveRunEventSource` or a generic `LiveRuntimeEventSource`
- `build_grok_live_run_fn(...)`
- mapping from Grok canonical events to existing UI rows

Preserve all UI design elements. The TUI should display Grok events, not own Grok execution.

### 3.4 Event Normalization

Modify or add:

- `runtime/run-event-v0.1.schema.json`
- `integration/grok/grok-event-bridge-event-v0.1.schema.json`
- `scripts/build_grok_event_bridge.py`
- `assurance/tui/events.py`
- `assurance/tui/bridge.py`

Required normalized event groups:

- `workspace_trust`
- `runtime_preflight`
- `runtime_capability`
- `workflow_started`
- `workflow_task_started`
- `workflow_task_completed`
- `subagent_started`
- `subagent_completed`
- `turn_started`
- `model_request`
- `model_output`
- `tool_proposal`
- `permission_requested`
- `permission_decision`
- `tool_started`
- `tool_completed`
- `runtime_usage_sample`
- `terminal`
- `artifact_registered`

Do not expose raw prompt, raw hidden reasoning, Authorization, or unredacted provider/session records.

### 3.5 Workspace Trust And Control Surfaces

Keep and reuse:

- `scripts/new_grok_workspace_trust_receipt.ps1`
- `assurance/workspace_trust.py`
- `integration/grok/fixtures/workspace-control-surfaces/`

Modify:

- Add a Python-side wrapper in `assurance/grok_runtime_adapter.py`.
- Add a CLI-facing receipt path in `gsa grok run`.

Grok must not launch against a workspace before the trust receipt is produced and checked.

### 3.6 DeepSeek-As-Grok-Provider

Keep:

- `integration/grok/deepseek-custom-model.example.toml`
- `scripts/invoke_grok_real_deepseek_conformance.ps1`
- `assurance/deepseek_adapter.py` as provider-shape knowledge and direct one-shot conformance, not the product runtime.

Modify:

- Move product routing to Grok custom-model config and Grok session execution.
- Prefer Grok's custom model, query parameter, and environment header surfaces before adding local provider transport code.
- Keep direct DeepSeek API as fallback/conformance, not default user path.

### 3.7 Process And Windows Supervision

Keep:

- `assurance/windows_sandbox.py`
- `assurance/sandbox.py`
- `scripts/run_grok_windows_child_tree_probe.py`
- `scripts/invoke_grok_windows_child_tree_probe.ps1`

Modify:

- Adapter should supervise Grok root process and record process tree metadata.
- TUI `UsageSampleEvent` should read from the Grok root process tree during live runs.
- Sandbox enforcement should be represented as observed Grok/runtime state plus local receipts, not a reimplemented generic sandbox.

### 3.8 Subagent And Workflow Graft

Keep:

- `assurance/retrieval_subagent.py`
- `assurance/child_capability_enforcer.py`
- `assurance/capability-delegation-receipt-v0.1.schema.json`
- `assurance/child-capability-enforcement-receipt-v0.1.schema.json`
- `docs/RETRIEVAL_SUBAGENT_AUDIT_2026-07-27.md`

Modify:

- Represent the project-document retrieval subagent and external retrieval subagent as Grok-compatible agent/workflow profiles where possible.
- Keep this repository's child capability receipts, evidence/source gates, redaction, and network capability enforcement as the assurance wrapper around those Grok-native profiles.
- Do not add a local multi-agent scheduler unless a new Grok gap audit proves the native subagent/workflow surfaces cannot preserve the required receipts.

## 4. First Modification Sequence

### Step 0: Candidate Version Gate

Completed for `0.2.112` promotion:

- Candidate metadata was updated for `0.2.112 (9bbd559437)`.
- Binary identity, signature, size/hash, version, workspace trust, ACP initialize, fake tool/permission/cancel, child-tree comparison, compaction provenance, event bridge, TUI projection, and repository regression checks were rerun or reconciled in the promotion docs.
- `0.2.112` is now the promoted lock. Keep a separate follow-up to split `windows_child_tree_timeout` into baseline-regression and Grok-owned cleanup gates.

### Step A: Adapter Skeleton

Create `assurance/grok_runtime_adapter.py` with:

- `GrokRuntimeConfig`
- `GrokRunRequest`
- `GrokRunResult`
- `inspect_grok_runtime()`
- `run_grok_headless_once()`
- `run_grok_acp_once()` or a named ACP fake-only path first

The first version may call existing scripts as subprocesses, but the output contract must be Python-owned and schema-validated.

### Step B: Event Normalizer

Create `assurance/grok_event_normalizer.py`:

- Input: Grok result JSON, session events, session updates, bridge result.
- Output: canonical `runtime/run-event-v0.1` events and TUI-ready event projection.

This is the main specialization graft: it translates Grok's mature runtime into this project's assurance vocabulary.

### Step C: CLI Wiring

Add `gsa grok doctor` and `gsa grok run`.

Do not replace `gsa run` yet. The migration should be visible and reversible until the Grok path has smoke coverage.

### Step D: TUI Wiring

Add `gsa tui --runtime grok --run ...`.

The TUI should consume the same normalized event stream used by the CLI. No widget should import Grok-specific code.

### Step E: Documentation Cleanup

Update stale prototype references in:

- `runtime/SCENARIO_EXPORT_AND_RUNNER_CONTRACT_v0.1.md`
- `architecture/UPSTREAM_FIRST_INTEGRATION_v0.1.md`
- any quickstart that still presents prototype commands as active implementation

The new wording should say: `prototype/` is retired; relevant lessons now live in assurance, runtime schemas, Grok integration fixtures, and git history.

## 5. Files To Treat As Product-Side Adaptation Points

Highest priority:

- `assurance/cli.py`
- `assurance/tui/main.py`
- `assurance/tui/bridge.py`
- `assurance/tui/event_source.py`
- `assurance/tui/events.py`
- `runtime/run-event-v0.1.schema.json`
- `upstream/grok-build.lock.json`
- `scripts/inspect_grok_install.ps1`

New files:

- `assurance/grok_runtime_adapter.py`
- `assurance/grok_event_normalizer.py`
- `assurance/grok_runtime_receipt-v0.1.schema.json`
- `assurance/tests/test_grok_runtime_adapter.py`
- `assurance/tests/test_grok_event_normalizer.py`

Probe/script assets to reuse, not product-copy:

- `scripts/invoke_grok_acp_initialize_probe.ps1`
- `scripts/invoke_grok_acp_fake_tool_probe.ps1`
- `scripts/invoke_grok_fake_provider_conformance.ps1`
- `scripts/invoke_grok_real_deepseek_conformance.ps1`
- `scripts/build_grok_event_bridge.py`
- `scripts/verify_grok_event_bridge.py`
- `scripts/new_grok_workspace_trust_receipt.ps1`

## 6. Non-Graft Areas

Do not revive or recreate:

- `prototype/fep_agent_proto/model_loop.py`
- `prototype/fep_agent_proto/action_kernel.py`
- `prototype/fep_agent_proto/model_transport.py`
- `prototype/fep_agent_proto/approval_ledger.py`
- `prototype/fep_agent_proto/network_broker.py`
- `prototype/fep_agent_proto/loopback_http.py`

Those responsibilities belong to Grok or the selected mature runtime.

Do not put hard gates into Grok fail-open hooks. Hooks can observe and annotate; hard gates remain in the assurance adapter path.

## 7. Open Questions Before Implementation

1. Should the first user-visible Grok path use headless mode or ACP?
   - Recommendation: ACP for tool/permission/session observability; headless only for narrow one-shot smoke.
2. Should `gsa run` default to Grok after the first adapter lands?
   - Recommendation: no. Add `--runtime grok` first, then promote after tests.
3. Should the product call PowerShell launchers directly?
   - Recommendation: only as a temporary bridge. Product contracts should be Python modules with subprocess internals hidden.
4. Should direct DeepSeek remain?
   - Recommendation: yes, but only as direct-provider conformance and fallback diagnostics, not the main agent path.

## 8. Current Verification

Ran:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\inspect_grok_install.ps1
```

Result after checker fix:

- `valid=true`
- bytes/hash/signature checks passed
- version check passed for the then-locked binary `grok 0.2.111 (94172f2aa4) [stable]`

This proved local binary identity for the previous lock, not source correspondence or live model behavior.

Additional environment observation:

```powershell
grok --version
```

returned:

```text
grok 0.2.112 (9bbd559437) [stable]
```

This originally proved only that the PATH-resolved stable Grok was newer than the promoted project lock. Subsequent candidate-gate and timeout-triage docs promoted `0.2.112` as the default lock with `windows_child_tree_timeout` carried forward as an explicit limitation.
