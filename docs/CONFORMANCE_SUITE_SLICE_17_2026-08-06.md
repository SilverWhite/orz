# Phase 3 #7 — Conformance Suite（Rust↔Python 交叉验证）审计文档（Slice #17）

**状态**: 完成（2026-08-06）。设计 §Phase 3 item 7「验证: 全功能 conformance suite」闭合——**Phase 3 全部条目完成**。

**提交**: 主仓库（Python）含全部代码/schema/fixtures/文档；orz `cli/feat/fusion-architecture` 分支含采集测试 + `.gitattributes`（推送用户手动惯例）。

## 1. 交付总览

契约文档 §7 定义 #7 = "Rust↔Python 交叉验证（真实 journal → envelope + payload schema 校验 + 链校验）——本契约的最终验证器"。本 slice 落地四件事：

1. **真实 journal 采集**（orz 侧）：`crates/orz-bin/src/main.rs` 新增 `#[cfg(test)] mod conformance_capture`——6 个 `#[ignore]` 测试，in-process（FakeProvider + 真实 loop/host/assurance），确定性产出 6 类场景 journal，staging 于 `target/conformance-journals/`（hermetic，workspace 内），Rust `replay_journal` 自校验后供开发期拷贝进主仓库。CI 保持纯 Python（零 orz 二进制执行）。
2. **交叉验证器**（Python 侧）：`assurance/run_event_journal_validation.py`——33 事件注册表（`PAYLOAD_SCHEMA_BY_EVENT_TYPE`，单一来源）+ 轨解析表（`PRODUCER_SCHEMAS`，按 `payload_schema` 字符串选 schema）+ 链校验（镜像 `orz-assurance/journal/chain.rs`，错误文案沿用 Rust 措辞）。**哈希规范形 = Rust-parity 形**（`json.dumps(sort_keys=True, separators=(",",":"), ensure_ascii=False)`），非 RFC8785——捕获 journal 的 digest 一致即 parity 证明。
3. **真实 journal fixtures**：`runtime/fixtures/run-event-v0.1/journals/` 6 个 `.jsonl`（plain / tool-snapshot / plan / cancelled / failed / restore-RST），覆盖 19 种不同事件类型；check_repository 门禁块（集合完整性 + 全链校验）+ required-path。
4. **三个裁决落地**（用户 2026-08-06 确认）：双轨并存（4-schema 分歧）；canonical_cli orientation 缺口修复；orientation-stagnation 3 个 schema 文件补齐 + 自校验。

## 2. 用户裁决记录

| 裁决点 | 选项 | 落地 |
|---|---|---|
| 采集机制 | in-process `#[ignore]` 测试（Recommended） | §1-1；`cargo test -p orz-bin -- --ignored conformance_capture` |
| 4-schema 分歧 | 双轨并存（Recommended） | §3-1；3 个 runtime/ Rust 轨文件 + assurance/ 原样 |
| canonical_cli 缺口 | 顺手修复（Recommended） | §3-3 |
| orientation-stagnation 缺口 | 补齐 3 个 schema 文件 + 自校验 | §3-2 |

## 3. 交付明细

### 3.1 双轨并存（4-schema 分歧裁决）

- **新文件** `runtime/orientation-checkpoint-event-payload-v0.1.schema.json` / `runtime/tool-availability-check-event-payload-v0.1.schema.json` / `runtime/runtime-stagnation-guard-event-payload-v0.1.schema.json`——Rust 轨形状（controller.rs 构造点）。
- §4 注册表 3 行 → runtime/ 文件；`test_run_event_conformance.py` 的 `_PAYLOAD_SCHEMAS` 与 `check_repository.py` 的映射改为**共同消费验证器注册表**（消除第三处重复，§9 流程已更新）。
- 约定测试 `test_payload_schema_file_convention` 重写为**冲突感知**：3 个双轨 slug 必须同时存在于 runtime/ 与 assurance/，其余 slug 恰好一个目录。
- `tool_belief_stagnation` 保持 assurance-only（Rust 从不构造）。
- fixtures 重生成：3 事件 payload 翻 Rust 形状（66 payload + 47 envelope 不变），envelope fixtures 随轨重生成（内嵌 payload 对象）；生成器只清 payloads/envelope/canonical_cli，journals/ 天然不碰。

### 3.2 orientation-stagnation 轨补齐

- 新文件 `assurance/orientation-stagnation-{preflight,run-started,terminal}-v0.1.schema.json`（形状取自 `orientation_runtime_journal.py` 构造点 :277-285/:307-310/:267-272）。
- `orientation_runtime_journal.py` 三构造点后各加 `validate_contract`（fail-fast，对齐既有模式）；事件内 `payload_schema` 字符串保持去后缀轨标识不变。
- **顺带发现并闭合同类漏洞**：`gsa-runtime-preflight-projection-v0.1.schema.json#runtime_event_payload` fragment 指向不存在的键——已补 `runtime_event_payload` 子 schema（3 形状 oneOf，形状取自 `runtime_preflight.py` 构造点；自定义顶层键对既有全文档校验无影响，jsonschema 忽略未知关键字）。

### 3.3 canonical_cli orientation 修复

- 新 helper `_orientation_checkpoint_payload(gates)`（canonical_cli.py）：从已写 checkpoint 工件派生 7 字段 assurance 形状（checkpoint_id/orientation_checkpoint_sha256/task_id/step_index/3 consts）+ `validate_contract` 守卫；fake/real 两处构造点替换。
- **GAK-CRED-001 allowlist 行号漂移修复**：canonical_cli.py:1099→1117（scrub 1137→1164）——helper 插入推后行号的存量缺陷同类（2026-08-05 先例）。

### 3.4 交叉验证器（`assurance/run_event_journal_validation.py`）

- 公共 API：`PAYLOAD_SCHEMA_BY_EVENT_TYPE` / `PRODUCER_SCHEMAS` / `validate_journal_file` / `validate_journal_text` / `_canonical_bytes` / `_payload_sha256` / `_event_sha256`（12 字段显式投影，对齐 chain.rs）/ `_resolve_payload_schema` / `_verify_chain`。
- 轨解析表：Rust 轨字符串 → §4 注册表；带后缀 → assurance/（永不解析到 runtime/ 双轨文件）；去后缀 canonical-cli/assurance 名 → 补后缀；fragment → JSON pointer 子 schema；grok/deepseek → envelope-only；未知字符串 → 报错并列注册表（"未登记产出方必须先入表"硬执行）。
- 链校验镜像 chain.rs：sequence 稠密 / run_id 恒定 / run_manifest_sha256 恒定 / 链链接（seq0 null + 前向相等）/ payload_sha256 重算 / event_sha256 重算 / 终局规则（至多一个、必须在最后、必须恰有一个）。**parity 范围与 Rust 对齐**：不额外检查 event_id 格式/timestamp 可解析/schema_version。
- 错误形状：`list[str]` 逐问题一条，文案沿用 Rust 措辞；envelope 失败先于链（缺字段时哈希不可算），parse 错误先于一切。

### 3.5 采集测试（orz）

- 6 场景：plain（10 事件）、tool-snapshot（16 事件，search_replace + allow-once 应答 + snapshot_created）、plan（13 事件，plan_proposed/plan_approved）、cancelled（6 事件，run_cancelled）、failed（6 事件，run_failed）、restore（3 事件，RST- + snapshot_restored）。
- 每场景：staging wipe+重建 → 跑场景 → 拷 events.jsonl → `replay_journal` 自校验（valid + terminal）→ 断言必需事件类型 → 打印表格行。
- 新增 dev-dep `agent-client-protocol`（workspace 已有包，权限应答需要 ACP 类型；Cargo.lock 仅 orz-bin 依赖列表变更）。
- `orz/.gitattributes` 新建（`*.jsonl text eol=lf` 等）；主仓库 `.gitattributes` 同步加 `*.jsonl text eol=lf`（CI 双平台字节稳定）。
- 采集命令（写入开发流程）：`cargo test -p orz-bin -- --ignored conformance_capture --nocapture --test-threads=1` + 拷贝循环（§5）。

## 4. 交叉验证实战发现（本 slice 的证明价值）

初版 Rust 轨 schema（按探索报告形状编写）在真实捕获 journal 上被验证器**立即捕获 2 个形状错误**——正是"Rust 是形状第一权威、真实 journal 是终裁"的实证：

1. **tool_availability_check**：`available/unavailable/degraded/unprobed` 是**工具名数组**（真实 payload 为 32 个工具名列表），初版 schema 按布尔编写 → 改数组 of string（schema + 生成器 + fixtures）。
2. **checkpoint_id pattern 缺小写**：会话型 run id 含小写 hex/sess-can（`ORIENT-RUN-sess-can-0-0000`），pattern `[A-Z0-9._-]` 拒之 → `[A-Za-z0-9._-]`。

**哈希链 parity：零 mismatch**——6 个 journal 的全部 payload_sha256/event_sha256 用 Rust-parity 规范形重算一致，链完整。最大风险点（canonical 形式跨语言一致性）确认无问题。

## 5. 文件清单

主仓库（D:\CLI）：
- 新建：`assurance/run_event_journal_validation.py`、`assurance/orientation-stagnation-{preflight,run-started,terminal}-v0.1.schema.json`、`runtime/orientation-checkpoint-event-payload-v0.1.schema.json`、`runtime/tool-availability-check-event-payload-v0.1.schema.json`、`runtime/runtime-stagnation-guard-event-payload-v0.1.schema.json`、`runtime/tests/test_run_event_journal_validation.py`、`runtime/fixtures/run-event-v0.1/journals/*.jsonl`（6）、`docs/CONFORMANCE_SUITE_SLICE_17_2026-08-06.md`
- 修改：`assurance/canonical_cli.py`（orientation 修复 + helper）、`assurance/orientation_runtime_journal.py`（3 处自校验）、`assurance/gsa-runtime-preflight-projection-v0.1.schema.json`（fragment 键补齐）、`assurance/tests/test_credential_scrub.py`（allowlist 漂移）、`scripts/generate_run_event_fixtures.py`（Rust 轨形状 + README）、`scripts/check_repository.py`（注册表单一来源 + journals 门禁）、`runtime/tests/test_run_event_conformance.py`（注册表导入 + 冲突感知）、`architecture/PYTHON_REFERENCE_SPEC_CONTRACT_v0.1.md`（§1/§4/§5/§6/§7/§9/§10）、`CLI_PROJECT_INDEX.md`、`README.md`、`.gitattributes`

orz（D:\CLI\orz，feat/fusion-architecture）：
- `crates/orz-bin/src/main.rs`（conformance_capture 模块，~350 行）、`crates/orz-bin/Cargo.toml`（dev-dep）、`.gitattributes`（新建）

## 6. 验证

- `cargo test -p orz-bin -- --ignored conformance_capture`：6 场景全部 valid（plain 10 / tool-snapshot 16 / plan 13 / cancelled 6 / failed 6 / restore 3 事件）
- `runtime/tests`：111 tests OK（新验证器 24 + conformance 7 含冲突感知约定测试）
- `assurance/tests`：1620 tests OK（13 既有 skip）——双轨 + canonical_cli 修复 + orientation 自校验零破坏
- `check_repository.py`：valid、0 errors；counts `run_event_journal_fixtures: 6`、`run_event_journal_validation: 1`、schemas 224
- `compileall`：clean

## 7. 审查闭环（三独立代理 + 实现自查）

**实现正确性**（general-purpose 代理）：**无 P1**。P2-1 修复（TrustPolicy 顶层 import 未用——已移入测试模块）；P2-2 记录（采集测试**首次运行观测到一次 4/6 失败**：journal 出现两条交错链（同 run_id 双 manifest）——7 次重跑含强制重编译均不可复现；机制上 recorder 是单写者任务无重发路径、bootstrap 每测试独立；**自捕获设计已兜底**：verify() 在 copy_journal 之前跑 replay_journal，坏 journal 永不落库，失败测试无 "captured" 行；watch 项，若复发需插桩）；P3-1 修复（docstring 诚实化——Python 在 parse 错误时跳过链校验，比 Rust 更严格，裁决一致但说明不再声称"镜像 abort"）；P3-2 修复（NaN 崩溃——`parse_constant` 拒绝非有限浮点于解析期，serde_json 同语义，加测试）；P3-3 修复（终局规则改为全部适用项都报，对齐 chain.rs 循环内 per-terminal 语义）；**P3-4 修复（最深缺陷）**：gsa fragment 子 schema 的 `$ref: "#/$defs/..."` 解析于 fragment 自身根（PointerToNowhere 崩溃）+ 顶层 `additionalProperties: false` 无 properties 拒一切——内联 pattern + 分支自携带约束 + 回归锁定测试（三合法形状通过/漂移拒绝）——**该轨无人走所以此前沉默，审查实证暴露**；P3-5 修复（cancelled 事件化等待——provider.received_requests 轮询替代固定 sleep）；P3-6 记录（checkpoint_id pattern `{4}` 对 step≥10000 不匹配，单 run 内不现实）。

**符合性**（general-purpose 代理）：**PASS**——33 事件 enum == 注册表（集合精确相等）、单一注册表来源（grep 仅三消费方）、门禁 set-equality 双向失败、生成器 wipe 仅 payloads/envelope/canonical_cli、§5↔代码双向可追踪（7 canonical-cli + 5 非 canonical-cli + grok/deepseek + orientation-stagnation×3 + 带后缀×4 + fragment + cli-session-lifecycle 全枚举核对）、canonical_cli 修复两处、契约文档编辑全部与代码状态一致、计划 A1-A3/B0-B5/C1-C5/D1-D6 全部在场。D2 修复（clippy len_zero——`!is_empty()`）；D3 记录（新模块 fmt 漂移——已跑 `cargo fmt -p orz-bin` 清零、布尔优先形状错误证据为审计文档记录而非 git 历史（新文件未追踪）、MEMORY.md 快照过时——实际已更新）。

**设计合理性**（general-purpose 代理）：**无 D1**。D2×2 修复（NaN 崩溃——见实现 P3-2；staleness 信号——提交 fixture 的**精确事件序列 + 计数断言**（Python `EXPECTED_SEQUENCES` + Rust verify 精确序列），重采集漂移在提交时即失败）；D3 记录（双轨解析规则健全性经全产出方实证（canonical_cli 去后缀 assurance 形状 ↔ assurance/ 文件、带后缀永不解析到 runtime 双轨）、RFC8785 ≡ Rust 规范形在当前 payload 词汇上**实测等价**（canary 测试锁定：`test_rfc8785_equals_rust_form_on_captured_vocabulary`，词汇引入 float 即失败）、capture 机制无 staleness 信号补丁后闭合、"最终验证器"声明范围（orientation/canonical-cli journal 未接入交叉验证——自校验兜底，记录）、cli-session-lifecycle 轨未实测、空 journal 措辞对齐 Rust `"journal contains no valid events"`）。

**实现自查补充**：三代理均审阅于部分修复落地前——修复后自查复验：clippy 零警告、fmt 零漂移、`cargo test -p orz-bin`（1 非忽略 + 6 忽略）绿、验证器 34 tests 绿、门禁 valid。

## 8. 状态与提交

两仓库本地提交完成（推送用户手动惯例）：主仓库（Python 全部代码/schema/fixtures/文档）；orz `feat/fusion-architecture`（采集测试 + Cargo.toml dev-dep + .gitattributes）。

## 8. 遗留

- **已清零**：Phase 3 全部条目（#1-#16 + #7 conformance suite）完成。遗留项（非 Phase 3 范围）：reasoning_content 回放缺口（live 验证后 fork 补丁）、TUI 事件模型与 run-event schema 的"镜像声明"未机械校验、`test_credential_scrub.py` allowlist 1020/1055 历史残留条目（无对应站点，无害）、Rust verifier 不校验 event_id 格式/timestamp 可解析/schema_version（与 Rust parity 一致，有意为之）。
- **重采集说明**：journals 为静态提交，重采集会字节级不同（时间戳/run id）——语义性差异，重算校验不依赖常量比对。
