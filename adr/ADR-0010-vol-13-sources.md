# ADR-0010 分卷 13：§13 设计与证据来源

> 本卷为 [`ADR-0010：ORZ 融合运行时、同构 Agent 与设计权威重整`](ADR-0010-fusion-runtime-and-agent-architecture.md)（AUTH-ADR-0010，ORZ 当前自然语言设计的唯一权威基线）的物理分卷 13（§13 设计与证据来源）；规范正文以所指分卷章节为准，分卷总目录见主文件。分卷为物理拆分、**语义零增删零改写**——本行以下正文与分卷前原文逐字一致；卷题与本声明为分卷批新增（分卷批：0ca，2026-09-28）。

## 13. 设计与证据来源

当前 accepted ADR：

- `adr/ADR-0003-runtime-neutral-assurance-kernel.md`
- `adr/ADR-0005-neutral-inquiry-thresholds-finalized.md`
- `adr/ADR-0006-credential-target-registry.md`
- `adr/ADR-0007-transport-retry-policy.md`
- `adr/ADR-0008-tool-round-budget.md`
- `adr/ADR-0009-write-placement-policy.md`

历史设计输入（归档后按 §12 路由）：

- `存档/architecture/pre-adr-0010/INTEGRATED_AGENT_LOOP_AND_FORK_DESIGN_v0.2.md`
- `存档/architecture/pre-adr-0010/IMPLEMENTATION_DEVIATION_AND_CORRECTION_v0.1.md`
- `存档/docs/design-inputs/CN_AGENT_BASE_GRAFT_VALUE_REVIEW_2026-07-30.md`
- `存档/docs/design-inputs/GSA_SELF_QUESTION_COUNTEREXAMPLE_DESIGN_2026-07-26.md`
- `存档/docs/implementation-history/INQUIRY_FIX_AND_BLACKBOARD_PARTITION_2026-08-08.md`
- `存档/docs/implementation-history/RUN_STALL_GUARDS_PLAN_2026-08-08.md`
- `存档/docs/implementation-history/FIX_PLAN_2026-08-06.md`
- `docs/RETRIEVAL_SUBAGENT_AUDIT_2026-07-27.md`
- `存档/docs/design-inputs/SOURCE_FULLTEXT_VISIBILITY_RULE_2026-07-26.md`
- `存档/architecture/pre-adr-0010/LOCAL_BROWSER_RETRIEVAL_AND_PDF_EVIDENCE_v0.1.md`
- `存档/architecture/pre-adr-0010/SESSION_PERSISTENCE_AND_LAYOUT_COMPACTION_v0.1.md`
- `architecture/PYTHON_REFERENCE_SPEC_CONTRACT_v0.1.md`

Schema 与机械证据：

- `assurance/retrieval-result-v0.1.schema.json`
- `assurance/retrieval-session-close-receipt-v0.1.schema.json`
- `runtime/neutral-inquiry-event-payload-v0.1.schema.json`
- `runtime/orientation-checkpoint-event-payload-v0.1.schema.json`
- `runtime/run-event-v0.1.schema.json`
- `runtime/retrieval-completion-check-event-payload-v0.1.schema.json`
- `assurance/diagnostic_coverage.py`
- `assurance/global_review_mode.py`
- `docs/CODEX_APP_SERVER_LIFECYCLE_NORMALIZER_AUDIT_2026-07-25.md`
- `docs/CLI_SESSION_LIFECYCLE_ADAPTER_AUDIT_2026-07-25.md`
- `orz/crates/orz-tui/src/explorer.rs`
- `orz/crates/orz-tui/src/view_model.rs`
- `orz/crates/orz-loop/src/controller.rs`
- `orz/crates/orz-loop/src/agents/retrieval.rs`
- `存档/architecture/runtime-spikes/WINDOWS_RUNTIME_CONTRACT_v0.1.md`
- `docs/GROK_WINDOWS_CHILD_TREE_PROBE_2026-07-23.md`
- `docs/GROK_OBSERVER_SUBPROCESS_LEAK_2026-07-30.md`
- `docs/GAK_SBX_001_WINDOWS_NATIVE_SANDBOX_AUDIT_2026-07-27.md`
- `docs/DEEPSEEK_CREDENTIAL_HARDENING_2026-07-23.md`

当前派生设计输入与实施证据（v1.2–v1.8 补写引用；只作来源路由，不新增裁决语义）：

- `docs/TOOL_AVAILABILITY_PROBE_DESIGN_2026-08-13.md`（§3.5 v1.8 单一探针面来源）
- `docs/RETRIEVAL_SOURCE_WEIGHTING_DESIGN_2026-08-12.md` 与 `docs/SOURCE_QUALITY_SEED_LISTS_2026-08-12.md`（§3.7 条 12 v1.6/v1.7 来源）
- `docs/RETRIEVAL_MECHANICAL_CONTROLS_DESIGN_2026-08-13.md`（§3.7 条 12 第二层与引用纪律的 P0-B 派生设计）
- `runtime/run-event-v0.2.schema.json`
- `runtime/retrieval-result-event-payload-v0.2.schema.json`
- `runtime/tool-availability-check-event-payload-v0.2.schema.json`
- `runtime/source-quality-seed-lists-v0.1.json`
- `orz/crates/orz-loop/src/tool_probe.rs`
- `orz/crates/orz-assurance/src/source_weighting.rs`

