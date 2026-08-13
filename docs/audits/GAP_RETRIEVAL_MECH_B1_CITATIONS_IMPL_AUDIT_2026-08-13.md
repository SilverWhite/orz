# GAP-RETRIEVAL-MECH B-1 实施审计（2026-08-13）

- 范围：FUS-RETRIEVAL-MECH 实施序列第 1 步（B-1 闭合）——`web_search` 引用
  URL（citations）结构化透传进 loop，使机械预筛获得候选池（ADR-0010 §3.7
  条 12 第二层前提）；本步不包含 web_fetch 计数门禁、机械预筛、输出级引用
  校验器与提示词更新（后续步骤 2/3/4/5/6）
- 设计入口：[`RETRIEVAL_MECHANICAL_CONTROLS_DESIGN_2026-08-13.md`](../RETRIEVAL_MECHANICAL_CONTROLS_DESIGN_2026-08-13.md)
- 迁移纪律：Schema/verifier 先行 → 接缝扩展 → producer → 测试/门禁全量验证

## 1. 已变更（核心）

### 1.1 Schema/verifier（先行）

- `runtime/retrieval-result-event-payload-v0.2.schema.json`：
  - `source_entry` 与 `ref_entry` 新增可选字段 `candidate_urls`
    （非空字符串数组；不设 schema 级 `uniqueItems`——去重由 producer 与
    verifier 双重机械保证，避免 payload 校验短路跨层 verifier 负例）；
  - 描述补充 B-1 语义（候选池为 metadata-grade，预筛后续附加 tier/weight）。
- `assurance/run_event_journal_validation.py`：新规则
  `_verify_v02_search_candidate_pool`（注册进 v0.2 管线）——
  `candidate_urls` 仅允许出现在 `web_search_result` 条目；必须是
  非空唯一字符串列表；`raw_source_refs` 与 ledger 逐条镜像一致。

### 1.2 host 接缝（结构化通道）

- `orz-loop/src/host.rs`：`ToolResult` 新增可选字段
  `structured: Option<serde_json::Value>`——host→loop 的结构化工具元数据
  接缝；仅 `web_search` 填充 `{"citations": [...]}`，其余工具恒 `None`。
- `orz-host/src/tools.rs`：新增 `structured_from_output`——只有
  `ToolOutput::WebSearch` 且 citations 非空时产出结构化负载；
- `orz-host/src/lib.rs`：`call_tool` 接线（`ToolResult.structured`）。
- 全部既有 `ToolResult` 构造点（orz-host/orz-loop/orz-bin 测试）补
  `structured: None`；`cargo check --workspace --tests` 编译驱动逐点闭合。

### 1.3 loop producer（证据账本）

- `orz-loop/src/controller.rs`：
  - `EvidenceRecord` 新增 `candidate_urls`；
  - `structured_candidate_urls`：shape-checked（`citations` 数组仅收字符串）、
    按首见顺序去重、**绝不从模型可见文本解析**；
  - `build_evidence_record` 仅 `web_search` 提取候选池，失败调用不产证据；
  - `build_structured_result`：`web_search_result` ledger 条目写
    `candidate_urls`，`raw_source_refs` 机械镜像；`web_search_result`
    条目仍不写 tier/weight（摘要条目语义不变，候选池为线索非证据）；
  - `source_counts` 不变（候选 URL 不是已观察来源）。

### 1.4 附带机械修复（预存缺口）

- `orz/crates/codegen/orz-workspace/src/handle.rs`：Bash 测试桩补
  `output_encoding: None`——GAP-ENCODING-GATE 后遗留的 latent break，
  仅 `cargo check --workspace --tests` 暴露；机械一致，无语义变化。

## 2. 决策登记

| ID | 决策 | 依据 |
|---|---|---|
| D-1 | 在 `ToolResult` 上增加可选 `structured` 接缝承载 citations，而非在模型可见渲染文本里嵌入 URL | B-1 语义=结构化透传进 loop；模型可见性留给机械预筛（步骤 3）与提示词更新（步骤 6）；output_text 只作线索不作证据 |
| D-2 | 候选池写在 `web_search_result` ledger 条目 + `raw_source_refs` 镜像，不展开为独立候选 ledger 条目 | 候选 URL 是 metadata-grade 线索，不是已观察证据；source_counts 语义不变；预筛步骤再附加 tier/weight/形态原因 |
| D-3 | Schema 不设 `uniqueItems`，去重由 producer/verifier 机械保证 | 保持 schema 形状最小；schema 级拒绝会使 payload 校验失败短路 cross-layer verifier，负例不可触达 |
| D-4 | 非 `web_search` 工具 `structured` 恒 None；`web_search` 无 citations 时也 None（无空池） | 最小接缝；空候选池无审计价值 |

## 3. 边界（明确未做，登记给后续步骤）

- **模型面不可见候选池**：`citations` 只进证据账本，不进子代理提示词文本；
  候选池的模型初选展示由机械预筛（步骤 3）与提示词更新（步骤 6）负责。
- web_fetch 候选机械计数与计数反馈、`ORZ_WEB_FETCH_CANDIDATE_CAP`（步骤 2）
  未做。
- 机械预筛模块与候选 tier/weight 排序标签（步骤 3）未做——B-1 只提供候选池。
- browser_read 范围/模式参数扩展（步骤 4）、输出级引用校验器（步骤 5）、
  提示词缩短与测试更新（步骤 6）未做。
- `tool_completed` 事件不带 `structured` 字段——citations 只在
  `retrieval_result_committed` 审计面出现，避免工具事件面扩表。
- local_browser 二段式不受本步影响。
- DC 剩余两信号（`same_module_no_evidence` / `key_surface_unexamined`）
  并入建议仍登记，未并入本步。
- `ToolResult` 无 serde 派生，`structured` 接缝是进程内契约；若未来
  host/loop 拆为跨进程边界，须显式补序列化契约与机器校验。
- B-1 候选去重为精确字符串级（首见序）；canonical URL/host 级去重按设计
  §2.2 留待机械预筛（步骤 3）附加。

## 4. 验证

- `cargo test -p orz-host`：**200 passed / 0 failed / 4 ignored**（+1
  `structured_from_output_carries_web_search_citations_only`）
- `cargo test -p orz-loop`：**238 passed / 0 failed / 3 ignored**（+1
  `web_search_citations_flow_into_candidate_pool`）
- `cargo test -p orz-assurance`：**139 passed / 0 failed**
- `cargo test -p orz-tui`：**178 passed / 0 failed**
- `cargo check --workspace --tests`：**0 errors**
- Python `runtime/tests/test_run_event_journal_validation.py` +
  `test_run_event_conformance.py`：**134 passed / 0 failed**（+6 候选池规则
  测试：镜像合法、缺镜像、非 web_search_result 携带、重复、ref 带池但账本
  缺池、ref 无对应账本条目）
- Python `assurance/tests` + `runtime/tests` 全量：**1819 passed /
  14 skipped**（审查闭环后复测：**1821 passed / 14 skipped**）
- `scripts/check_repository.py`：**valid / error_count 0**（schemas 241）
- `cargo fmt --all` 已跑；两个仓库 `git diff --check` 均为 0

## 5. 审查发现（实施过程闭环）

- **P1（接缝选择）**：最初考虑把 citations 渲染进模型可见输出再由 loop
  解析；改为 `ToolResult.structured` 结构化接缝——与"结构化透传"字面一致，
  模型合同保持不变，且符合 output_text 只作线索不作证据的既有语义。
- **P2（Schema 负例可达性）**：候选池去重若用 schema `uniqueItems`，
  payload 校验失败会短路全部 cross-layer verifier，负例测试不可达；
  改由 `_verify_v02_search_candidate_pool` 机械校验。
- **P3（预存编译缺口）**：`orz-workspace` 测试桩缺 `output_encoding`，
  是 GAP-ENCODING-GATE 遗留；本次编译驱动补上（机械一致）。

## 6. 仓库边界与提交顺序

- 本步横跨两个 git 仓库：父仓库 `D:\CLI`（Schema/verifier/设计/审计/待办）
  与嵌套仓库 `D:\CLI\orz`（Rust 实现）。
- 提交顺序：**先提交父仓库，再提交 orz 仓库**（与 GAP-SOURCE-WEIGHTING-IMPL
  审计 §7 一致）。
