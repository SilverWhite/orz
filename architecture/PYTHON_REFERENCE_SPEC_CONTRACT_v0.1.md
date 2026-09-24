> **pre-ADR-0010 冻结状态头（2026-09-24，审查修复批）**：本文件属 pre-ADR-0010 时期冻结 fixture／development-only spike——**非 `current`、仅历史基线**，不作为实现依据；当前设计权威＝[`ADR-0010`](../adr/ADR-0010-fusion-runtime-and-agent-architecture.md)（本状态头为其 §7.2 归档规则的**就地冻结**轻量替代，不移档不改正文，2026-09-24 批登记）。

# Python Reference-Spec Contract v0.1

**状态**: 事实/设计约束（2026-08-06，Phase 3 slice #14 定稿）。本文件把 FORK 架构文档 §7 声明的"Python 项目 = reference spec + conformance suite + schema authority"物化为可执行的契约：权威范围、schema 注册表、轨标识约定、豁免登记、同步纪律与变更流程。

## 1. 目的与范围

Python 项目（`D:\CLI`）在融合架构中不再是生产 runtime——生产 runtime 是 Rust（`D:\CLI\orz`）。Python 的六个角色（历史对照 `存档/architecture/pre-adr-0010/FORK_ARCHITECTURE_AND_DESIGN_LANGUAGE_v0.2.md` §7 与 `README.md`；当前裁决见 ADR-0010）：

| 角色 | 落地 |
|---|---|
| Assurance spec reference（金版） | 本文档 + `runtime/*.schema.json`（run-event envelope + 34 事件 payload）+ `assurance/*.schema.json`（receipt/contract 体系） |
| Conformance test suite | `runtime/tests/test_run_event_conformance.py` + `runtime/tests/test_run_event_journal_validation.py` + `assurance/run_event_journal_validation.py`（**#7 最终验证器，已落地**）+ `assurance/tests/`（test_p0_contracts 等）+ CI 门禁 `scripts/check_repository.py` |
| Design documents | `architecture/`、`docs/`、`adr/`（本文档属于此角色且是 schema 契约的注册表） |
| Offline verification | `assurance/canonical_cli.py`（保留为离线验证路径；其 payload 走 `canonical-cli-*` 独立轨） |
| Rapid prototyping | 新 gate/行为先在 Python 实现验证，再移植 Rust（约定） |
| Schema authority | `runtime/*-v0.1.schema.json` 与 `assurance/*-v0.1.schema.json` 是 JSON Schema 的规范定义；Rust 复制/镜像 |

范围：本契约管辖 **run-event 事件体系**（envelope + 34 事件 payload schema 及其实例）。非 run-event 的 schema（P0-P5 contract、GPS、codex-app-server 等）各有既有契约文档，不在本文管束内。

## 2. 权威层次

- **Rust 生产实现 = payload 形状第一权威**：24 个新 payload schema 的形状取自 Rust 构造点（`orz/crates/orz-loop/src/controller.rs`、`orz/crates/orz-host/src/session.rs`、`orz/crates/orz-bin/src/main.rs`）。Rust 是唯一生产运行时，其实际产出是 conformance 的最终裁判（#7 交叉验证落地时以此为准）。
- **Python = schema 文件权威**：形状一旦固化为 schema 文件，修订必须走 §9 变更流程。
- **payload_schema 字符串 = 轨标识**：同一事件类型在不同轨可以有不同的 payload 形状；`payload_schema` 字段值标识轨。reference-spec 只对 Rust 轨（及各自声明轨）的 shapes 做符合性承诺。

## 3. run-event envelope 契约

`runtime/run-event-v0.1.schema.json` 是唯一事实源（本文件不复制定义，只摘录约束要点）：

- 34 事件 enum（清单见 §4）；13 个 required 字段；`additionalProperties: false`；`schema_version` const `"0.1.0-draft"`。
- `sequence == 0` 时 `previous_event_sha256` 必须为 `null`，否则必须为 sha256（allOf if/else）。
- `run_id` pattern `^(RUN|RST)-[A-Za-z0-9._-]+$`（RST- 为 restore run）；`event_id` pattern `^EVT-[A-Za-z0-9._-]+$`；sha256 pattern `^[a-f0-9]{64}$`（小写）。
- `payload` 为自由 object；形状由 §4 的 payload schema 约束，关联仅靠 `payload_schema` 字符串约定（无 $ref 硬接线）。
- 已知环境事实：jsonschema 4.26 的 `format: date-time` 检查对任意字符串放行（实测 no-op）——envelope 的 timestamp 格式在 conformance 层不强制（结构性约束仍生效）。

## 4. Schema authority 清单（34 事件 ↔ payload schema 文件）

命名惯例 `<slug>-event-payload-v0.1.schema.json`（event_type snake_case → kebab-case），文件在 `runtime/`；`$id` 域 `https://local.scientific-assurance.invalid/schema/<文件名>`；draft/2020-12；`type: object`；`additionalProperties: false`；**无 `schema_version` 字段**（envelope 已管）。哈希字段用本地 `$defs.sha256`。

| event_type | 文件（runtime/ 除非注明） | 形状权威 |
|---|---|---|
| run_preflight | run-preflight-event-payload-v0.1 | Rust session.rs:136-156 |
| run_started | run-started-event-payload-v0.1 | Rust controller.rs:308-313 |
| prompt_submitted | prompt-submitted-event-payload-v0.1 | Rust controller.rs:314-322 |
| model_request | model-request-event-payload-v0.1 | 参考形状（TUI bridge 消费面） |
| model_response_received | model-response-received-event-payload-v0.1 | 参考形状 |
| model_output | model-output-event-payload-v0.1 | Rust controller.rs:433-452 |
| acp_initialize | acp-initialize-event-payload-v0.1 | 参考形状（protocol_version 双类型） |
| acp_session_created | acp-session-created-event-payload-v0.1 | 参考形状 |
| tool_proposal | tool-proposal-event-payload-v0.1 | 参考形状 |
| permission_requested | permission-requested-event-payload-v0.1 | Rust controller.rs:952-961 |
| permission_decision | permission-decision-event-payload-v0.1 | Rust controller.rs:966-979 |
| tool_started | tool-started-event-payload-v0.1 | Rust controller.rs:829-836, 1042-1050 |
| tool_completed | tool-completed-event-payload-v0.1 | Rust controller.rs:862/900/1053/1071 |
| orientation_checkpoint | runtime/orientation-checkpoint-event-payload-v0.1 | **双轨（§6 裁决，slice #17）：Rust 轨 = 本 runtime/ 文件**，形状权威 controller.rs:335-343；assurance/ 同名文件为 orientation 轨形状 |
| runtime_stagnation_guard | runtime/runtime-stagnation-guard-event-payload-v0.1 | **双轨（§6 裁决，slice #17）：Rust 轨 = 本 runtime/ 文件**，形状权威 controller.rs:695-710；assurance/ 同名文件为 orientation 轨形状 |
| tool_availability_check | runtime/tool-availability-check-event-payload-v0.1 | **双轨（§6 裁决，slice #17）：Rust 轨 = 本 runtime/ 文件**，形状权威 controller.rs:300-311（**available/unavailable/degraded/unprobed 为工具名数组**——真实捕获 journal 钉死）；assurance/ 同名文件为 orientation 轨形状 |
| tool_belief_stagnation | assurance/tool-belief-stagnation-event-payload-v0.1 | 既有（assurance 轨，见 §6；Rust 不构造，无双轨） |
| instruction_provenance_gate | instruction-provenance-gate-event-payload-v0.1 | Rust controller.rs:530-539 |
| gate_decision | gate-decision-event-payload-v0.1 | Rust controller.rs:542-549, 654-663（decision 含 `"stop"`） |
| neutral_inquiry | neutral-inquiry-event-payload-v0.1 | 既有（runtime 轨，Rust controller.rs:781-793 对齐） |
| counterexample_gate | counterexample-gate-event-payload-v0.1 | 既有（runtime 轨） |
| retrieval_completion_check | retrieval-completion-check-event-payload-v0.1 | 既有（runtime 轨） |
| context_compressed | context-compressed-event-payload-v0.1 | **A6（2026-08-08）**：Rust controller.rs `compact_messages`（7 必填计数：trigger/target/rounds_since_last_compaction/rounds_dropped/messages_dropped/messages_kept/estimated_tokens_after） |
| snapshot_created | snapshot-created-event-payload-v0.1 | 既有（runtime 轨） |
| snapshot_restored | snapshot-restored-event-payload-v0.1 | 既有（runtime 轨） |
| artifact_registered | artifact-registered-event-payload-v0.1 | 参考形状 |
| plan_proposed | plan-proposed-event-payload-v0.1 | Rust main.rs:324-335（plan_id `^PLAN-`、task_id `^TASK-`） |
| plan_approved | plan-approved-event-payload-v0.1 | Rust main.rs:341-353 |
| plan_rejected | plan-rejected-event-payload-v0.1 | 参考形状 |
| action_approved | action-approved-event-payload-v0.1 | 参考形状 |
| run_finished | run-finished-event-payload-v0.1 | Rust controller.rs:729-738 + acp_server.rs:547-549（required 仅 status） |
| run_failed | run-failed-event-payload-v0.1 | Rust controller.rs:240-250, main.rs:362-375, acp_server.rs:577-580 |
| run_cancelled | run-cancelled-event-payload-v0.1 | Rust controller.rs:240-250（reason 现仅 user_cancelled，可扩展） |
| run_invalidated | run-invalidated-event-payload-v0.1 | Rust controller.rs:721-738 |

实例（fixtures）由 `scripts/generate_run_event_fixtures.py` 生成（幂等），落于 `runtime/fixtures/run-event-v0.1/{payloads,envelope}/`；映射与完整性断言在 `runtime/tests/test_run_event_conformance.py` 与 `scripts/check_repository.py`（双处重复沿 p0 先例，硬断言防漏挂）。

## 5. payload_schema 字符串 ↔ 文件名约定

- **Rust 轨**：全事件 `payload_schema` 字段值统一为 `"run-event-v0.1.schema.json"`（envelope 文件名）——`controller.rs:1151`（EventWriter::record）、`main.rs:393`（record_plan_event）、`session.rs:152`（run_preflight）三处一致。payload schema 文件名与事件的映射由本文件 §4 注册表 + 测试断言保障，**不依赖** Rust 字段值。未来若 Rust 改用 payload 文件名，须同步本契约。
- **canonical-cli 轨**（`canonical_cli.py`）：`canonical-cli-preflight-v0.1` / `canonical-cli-run-started-v0.1` / `canonical-cli-fake-model-request-v0.1` / `canonical-cli-real-model-request-v0.1` / `canonical-cli-fake-model-output-v0.1` / `canonical-cli-real-model-output-v0.1` / `canonical-cli-terminal-v0.1`——2026-08-06 补齐 schema 文件（`assurance/canonical-cli-*-v0.1.schema.json`），闭合此前"字符串指向不存在文件"的完整性漏洞。轨内另有 5 个非 canonical-cli-* 字符串（`instruction-provenance-gate-receipt-v0.1` / `tool-availability-check-event-payload-v0.1` / `orientation-checkpoint-event-payload-v0.1` / `source-visibility-gate-receipt-v0.1` / `canonical-cli-answer-packet-v0.1`），指向的文件均存在，分属 receipt/既有 payload 体系。
- **normalizer 轨**（`grok_event_normalizer.py`）：单一字符串 `grok-runtime-normalized-v0.1`（无 schema 文件，其输出经 envelope schema 校验——test_grok_event_normalizer）。
- **orientation 轨**（`orientation_runtime_journal.py`）：`orientation-stagnation-preflight-v0.1` / `orientation-stagnation-run-started-v0.1`；且该模块以完整文件名（带 `.schema.json` 后缀，如 `"tool-availability-check-event-payload-v0.1.schema.json"`，`:35`）引用 payload schema，而 canonical_cli 用去后缀形式（`canonical_cli.py:926`）——两处写法不一致（已知，§7）。
- **deepseek runtime adapter 轨**（`deepseek_runtime_adapter.py:60`）：`deepseek-runtime-normalized-v0.1`，产出 run_preflight/run_started（`:117/:131`）。
- **runtime preflight 轨**（`runtime_preflight.py:70`）：`gsa-runtime-preflight-projection-v0.1.schema.json#runtime_event_payload`——**第三形态**：带 `#` JSON-pointer fragment，既非纯轨标识也非纯文件名引用，产 run_preflight/run_started/run_finished（`:271/:284/:297`）。
- **cli session lifecycle 轨**（`cli_session_lifecycle.py:14,288-291`）：`cli-session-lifecycle-event-v0.2.schema.json`——以 run-event envelope 产出 6 个 34-enum 事件类型（run_started/model_request/model_output/run_finished/run_failed/run_cancelled），但 payload 为 lifecycle 形状（与 §4 不同）。
- **注册表范围声明**：本表登记已知全部 run-event 产出方；`#7` 交叉验证按 **payload_schema 字符串**（而非 event_type）选择校验 schema，未登记的新产出方必须先入本表。
- **envelope 轨标识**：`payload_schema` 值不带 `.schema.json` 后缀的（`canonical-cli-*`、`grok-runtime-normalized-v0.1`、`orientation-stagnation-*`、`deepseek-runtime-normalized-v0.1`）是轨标识而非文件名引用；带后缀或 fragment 的是文件名/指针引用。
- **#7 交叉验证按本表执行**（slice #17 落地）：`assurance/run_event_journal_validation.py` 的 `PRODUCER_SCHEMAS` + `_resolve_payload_schema` 是本表的唯一执行点——精确字符串匹配，**按 payload_schema 字符串（而非 event_type）选 schema**。规则：① Rust 轨字符串 `"run-event-v0.1.schema.json"` → §4 注册表（双轨 slug → runtime/ 文件）；② 带后缀形式（orientation 轨约定）→ `assurance/` 同名文件，**永不解析到 runtime/ 双轨文件**；③ 去后缀 canonical-cli/assurance 名 → `assurance/` 补 `.schema.json`；④ fragment 形式 → `assurance/gsa-runtime-preflight-projection-v0.1.schema.json` 的 `runtime_event_payload` 子 schema（slice #17 补齐该键——此前指针指向不存在的键）；⑤ `grok-runtime-normalized-v0.1` / `deepseek-runtime-normalized-v0.1` → envelope-only；⑥ 未登记字符串 → 报错并列注册表（"未登记产出方必须先入本表"硬执行）。**后缀写法不一致项（orientation 带后缀 vs canonical-cli 去后缀）裁决：保留登记**——两种形式均可确定解析，不归一化。

## 6. 豁免登记

正式登记的 shape 偏差（conformance 时对 Rust 轨豁免或分别校验）：

1. **Phase 2 接受清单（2026-08-04）**：journal payload 字段形状与 Python no-model fixture 不同（真实 agent 数据）——conformance 豁免；以 Rust 轨 shapes（§4）为准。
2. **text_delta live-only**（Phase 3 slice #6）：`agent_message_chunk` 通知不入 journal，无对应事件——envelope 无此项，无需 schema。
3. **4 个 assurance/ 既有 payload schema 与 Rust 构造形状分歧**（2026-08-06 登记）：

| 事件 | assurance 轨 schema（§4 权威） | Rust 构造形状 |
|---|---|---|
| orientation_checkpoint | required 7 字段（checkpoint_id/…/claim_strength_effect，const true/false/"none"） | `{checkpoint_id, trigger, step_index, message_block}` |
| tool_availability_check | 7 字段（…_report_sha256 + _count × 4 + context_block_injected + model_must_not_guess const true） | `{available, unavailable, degraded, unprobed, gate_decision}` |
| runtime_stagnation_guard | 11 字段（receipt_sha256/decision/action/reason_codes/…/三个 const） | `{decision, reason_codes, max_consecutive_repeated_content, max_ngram_repeat}` |
| tool_belief_stagnation | 7 字段（receipt_sha256/decision/reason_codes/mismatch_count/三个 const） | （Rust 生产不构造） |

**裁决（#7 交叉验证 slice，2026-08-06）**：**双轨并存**——3 个 Rust 构造事件（orientation_checkpoint / tool_availability_check / runtime_stagnation_guard）在 `runtime/` 新建 Rust 轨 schema（形状 = controller.rs 实际构造，经真实捕获 journal 验证钉死），§4 注册表与 fixtures 落 Rust 轨；`assurance/` 4 个文件**原样保留**（`orientation_runtime_journal.py` 与 p0 映射零破坏）；`tool_belief_stagnation` 保持 assurance-only（Rust 从不构造）。备选路线记录：Rust 对齐需在 orz-assurance 新造 Rust 当前没有的数据管线（receipt_sha256/counts/action 等）+ TUI bridge/projection 重建；放松 schema 会削弱承重 const 不变量且 stagnation 缺 6/11 字段退化为任意 object 检查——均否决。

4. **gate_decision.decision 含 `"stop"`**（tool_rounds_limit 变体，controller.rs:654-663）——已纳入 schema enum，非豁免。
5. **run_finished 双变体**（controller.rs 带 turn_count/tool_rounds vs acp_server.rs restore 路径仅 status）——schema required 仅 `{status}`，兼容。

## 7. 已知缺口

- **8 个参考形状事件**（§4 标注"参考形状"）：Rust 生产不构造（acp_initialize/acp_session_created/tool_proposal/model_request/model_response_received/artifact_registered/plan_rejected/action_approved），schema 以 TUI bridge.rs 消费字段为最低面 + normalizer 词汇为 optional 收录——**非定论**，若未来 Rust 构造形状不同须按 §9 修订。
- **canonical_cli 自身缺口**：其 orientation_checkpoint payload `{checkpoint_sha256, trigger_step, task_contract_sha256}`（canonical_cli.py:940-944）与 assurance orientation-checkpoint schema **形状不吻合**——**已修复（#7 slice，2026-08-06）**：`_orientation_checkpoint_payload` helper 从已写的 checkpoint 工件派生 7 字段 assurance 形状（fake/real 两处构造点 + validate_contract 守卫），test_canonical_cli 全绿。
- **跨轨词汇表分轨**：permission_requested 的 `risk` 是 Rust `RiskClass` 的 Debug 串（PascalCase：ReadOnly/LocalMutation/NetworkCall/SandboxEscape）；Python 侧 `instruction_gate.py` 的 risk_class（normal/sensitive/external_side_effect）是另一套词汇——分属不同轨，互不换算，勿混淆。
- **TUI 事件模型**（`orz-tui/src/events.rs`）声明"与 run-event schema 独立但镜像"——仅事件种类镜像，字段形状以本契约 §4 为准。
- **payload_schema 后缀写法不一致**（§5 orientation 轨带后缀 vs canonical-cli 轨去后缀）——**裁决：保留登记**（§5，两种形式均可确定解析）。
- **jsonschema date-time no-op**（§3 环境事实）。
- **哈希规范形**：链校验用 **Rust-parity 形** `json.dumps(sort_keys=True, separators=(",",":"), ensure_ascii=False)`（`assurance/utils.py` 的 RFC8785 `canonical_bytes` 是另一规范形，仅用于 Python 自产 journal 的 `_event_hash`；`#7` 以真实捕获 journal 为 parity 终裁——2026-08-06 六 journal 全部 digest 一致，零 mismatch）。
- **#7 已完成**（2026-08-06，slice #17）：Rust↔Python 交叉验证落地——6 个真实 Rust journal 静态提交 `runtime/fixtures/run-event-v0.1/journals/`，`assurance/run_event_journal_validation.py` 全链校验（envelope + 按轨 payload + 链重算），测试 + check_repository 门禁 + 采集测试（orz `#[ignore]`）。**校验范围声明**：验证器对 Rust 轨 journal 全链执行；orientation/canonical-cli 轨 journal 由各自产出方自校验（orientation_runtime_journal.py 逐事件 envelope+payload+链）兜底，未接入本验证器（设计审查 D3 记录）。审计：`docs/CONFORMANCE_SUITE_SLICE_17_2026-08-06.md`。**交叉验证实战捕获 2 个形状错误**（初版 Rust 轨 schema 的 tool_availability_check 布尔 vs 真实数组、checkpoint_id pattern 缺小写）——真实 journal 是形状终裁的证据。

## 8. Rust 镜像同步纪律

- `orz/crates/orz-assurance/src/journal/event.rs` 头部注释声明镜像 `runtime/run-event-v0.1.schema.json`——envelope enum（34 事件）与 payload 形状同步是本契约的组成部分。
- 新增/改名事件类型：envelope enum + §4 注册表 + fixtures + 测试断言四者同步（`test_all_34_event_types_covered` 与 `test_payload_schema_file_convention` 自动捕获漂移）。
- Rust 侧 payload 构造点的形状变更必须先过 §9 流程，禁止先行改形状再补契约。

## 9. 变更流程（Task D S3 翻转后口径，2026-09-06）

> 2026-09-06 任务 D S3 权威翻转（用户裁决 D-1=方案 α）：registry JSON
> `runtime/run-event-payload-registry-v0.1.json` 是 event_type→payload
> schema 映射的唯一权威；Rust 法官
> （`orz-assurance journal/conformance.rs` + `journal-conformance` CLI）
> 是期刊校验的唯一执法者；`assurance/run_event_journal_validation.py`
> 转**冻结 reference**（dict 视图由 registry 派生、不再执法，仅作为
> Rust↔Python 对拍对照面与 `_WORK_TOOLS` 单源保留）。导出脚本已退役。

1. 修改/新增 registry JSON 的映射条目（唯一权威；Python 侧视图随导入自动派生，无第二步登记）。
2. 修改/新增 `runtime/` 或 `assurance/` 的 schema 文件本体；更新 `scripts/generate_run_event_fixtures.py` 内嵌形状 → 重生成 fixtures。
3. Rust 法官面同步：event.rs 镜像 + payload 构造点（controller.rs/main.rs/session.rs 形状）——spec 表（`families.rs` expected_violations）与 Rust↔Python 对拍（`s2b_family_verdicts_match_python`）必须保持 0 差。
4. 冻结 Python reference 同步（`run_event_journal_validation.py` 机械族与对拍函数一一镜像；单侧改动会被对拍拒绝——这是唯一允许触碰该模块机械族的场景）。
5. 跑 conformance 测试（orz-assurance lib + `journal-conformance` CLI 集成测试）+ pytest `runtime/tests` + `check_repository.py`（三者任一失败即阻断）。
6. 更新本文档 §4 注册表 / §5 解析表 / §6 豁免 / §7 缺口。

## 10. 参考

- `存档/architecture/pre-adr-0010/INTEGRATED_AGENT_LOOP_AND_FORK_DESIGN_v0.2.md`（§5 Phase 3 item 5/7，当时的实现基线；当前权威为 ADR-0010）
- `存档/architecture/pre-adr-0010/FORK_ARCHITECTURE_AND_DESIGN_LANGUAGE_v0.2.md`（§7 Python 六角色）
- `runtime/run-event-v0.1.schema.json`（envelope 事实源）+ `runtime/*-event-payload-v0.1.schema.json`（24 个 2026-08-06 新建 + 5 个既有 + **3 个 slice #17 Rust 轨双轨文件**）
- `assurance/*-event-payload-v0.1.schema.json`（4 个既有）+ `assurance/canonical-cli-*-v0.1.schema.json`（7 个 2026-08-06 补齐）+ `assurance/orientation-stagnation-{preflight,run-started,terminal}-v0.1.schema.json`（3 个 slice #17 补齐）
- `runtime/fixtures/run-event-v0.1/`（fixtures + README + **journals/ 6 个真实捕获**）、`assurance/fixtures/canonical_cli/`
- `runtime/tests/test_run_event_conformance.py`、`runtime/tests/test_run_event_journal_validation.py`、`assurance/run_event_journal_validation.py`（#7 交叉验证器；**2026-09-06 起为冻结 reference——执法权在 Rust 法官**）、`scripts/generate_run_event_fixtures.py`、`scripts/check_repository.py`（期刊校验经 `journal-conformance` CLI 调 Rust 法官）
- 相关记忆：`fusion-phase-tracking.md`（Phase 3 slice #14）
