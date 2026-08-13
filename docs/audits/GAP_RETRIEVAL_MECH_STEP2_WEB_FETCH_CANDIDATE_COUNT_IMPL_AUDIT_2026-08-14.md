# GAP-RETRIEVAL-MECH 步骤 2 实施审计（2026-08-14）

- 范围：FUS-RETRIEVAL-MECH 实施序列第 2 步——web_fetch 候选机械计数门禁与计数反馈
  （设计 §1：`ORZ_WEB_FETCH_CANDIDATE_CAP` 硬门、per-activation 累计、去重后 URL
  计数、超限无 ToolStarted 拒绝、计数反馈进工具结果）；本步不包含机械预筛（步骤 3）、
  browser_read 范围/模式参数（步骤 4）、输出级引用校验器（步骤 5）与提示词缩短（步骤 6）。
- 设计入口：[`RETRIEVAL_MECHANICAL_CONTROLS_DESIGN_2026-08-13.md`](../RETRIEVAL_MECHANICAL_CONTROLS_DESIGN_2026-08-13.md)
- 前置：B-1 已闭合（web_search citations 结构化透传进 loop，提供候选池接缝）；
  本步依赖 B-1 的 `ToolResult.structured` 与 `candidate_urls` 证据账本。
- 迁移纪律：Schema/verifier 先行 → 接缝扩展 → producer → 测试/门禁全量验证。

## 1. 用户裁决

- **2026-08-14：`ORZ_WEB_FETCH_CANDIDATE_CAP` 定档 8**（设计 §1.1 的
  “默认 5–8，最终值待定档”闭合；设计文档同步更新）。

## 2. 已变更（核心）

### 2.1 Schema/verifier（先行）

- `runtime/tool-completed-event-payload-v0.1.schema.json`：`tool_completed`
  新增可选字段 `candidate_count`（≥0 整数）与 `candidate_cap`（≥1 整数），
  描述注明仅 web_fetch 家族事件携带；配对/范围/形状语义由 Python verifier
  机械校验（延续 B-1 D-3：schema 形状最小，跨层负例不短路）。
- `assurance/run_event_journal_validation.py`：新规则
  `_verify_v02_web_fetch_candidate_count`（注册进 cross-layer 管线）——
  `candidate_count`/`candidate_cap` 必须成对出现；仅允许 web_fetch 家族
  （`web_fetch` / `web_fetch_*`）；0 ≤ count ≤ cap、cap ≥ 1；非 error 的
  web_fetch 完成事件必须携带两字段；`web_fetch_candidate_cap_exceeded`
  拒绝事件必须 status=error 且 count == cap（拒绝发生在边界）。

### 2.2 计数域（per-activation，跨 continue/恢复累计）

- `ActivationState` 新增 `web_fetch_candidates: Vec<String>`（精确字符串去重、
  首见序；canonical/host 级去重按设计 §2.2 留待步骤 3，与 B-1 边界一致）。
- `StoredActivation`/快照/恢复同步——计数随 activation 侧车持久化（与
  `tool_rounds_used` 同生命周期：continue 重入不重置，仅 activation 关闭后
  新起；跨 run restore 后继续同一预算）。
- 子代理运行期间 activation 状态移出注册表（既有生命周期），计数域以
  `Arc<Mutex<Vec<String>>>` 经 `LoopProfile.fetch_candidates` 传入共享循环，
  循环结束后写回 activation（成功/失败/取消全路径）；internal lane 传
  `None`（内部车道永不执行 web_fetch，门禁按无域 fail-closed）。

### 2.3 门禁与反馈（orz-loop `run_host_tool`）

- 插入点：mode 门禁之后、permission/ACAF 票/ToolStarted 之前——拒绝不产生
  ToolStarted、不签发动作票；允许时才推进执行链路。
- 计数语义：工具执行前机械计数；新 URL 且 count < cap → 计入并放行；重复
  URL（精确字符串相等）不计新候选但仍可读取；新 URL 且 count ≥ cap →
  拒绝。
- 未超限：`ToolResult.output` 追加 `\n候选 N/M，剩余 K`（成功与 host 错误都
  追加——失败抓取同样消耗了一个候选位），`tool_completed` 事件携带
  `candidate_count`/`candidate_cap`（审计面）。
- 超限：无 ToolStarted 显式拒绝，中性陈述
  “候选核验数量已达上限 {cap}（当前 {count}/{cap}）”，事件 error 码
  `web_fetch_candidate_cap_exceeded`，返回 `PolicyFeedback::Denied`
  （与连续拒绝熔断同一处理面——不给模型重试空间，ADR-0010 §3.5.4）。
- fail-closed 两翼：无计数域（主/grill 车道直接调用，理论上不可达）→
  `web_fetch_candidate_count_unbound`；缺 `url` 参数（无计数身份）→
  `web_fetch_candidate_url_missing`；均无 ToolStarted、均计 Denied。
- `relay.rs` 新增 `is_web_fetch_tool`（`web_fetch` / `web_fetch_*` 前缀边界，
  与 `is_web_retrieval_tool` 同构）；web_search（候选生产者）、browser_read /
  pdf_read（local_browser 面，步骤 4 再入同一计数域）不计。

## 3. 决策登记

| ID | 决策 | 依据 |
|---|---|---|
| D-1 | 计数域挂在 `ActivationState` 并随侧车持久化，经 LoopProfile 的 `Arc<Mutex<Vec<String>>>` 穿过子代理循环 | activation 运行期间状态移出注册表；预算语义（tool_rounds_used）同生命周期；跨 run continue 必须延续同一预算（否则恢复后 cap 可被绕过） |
| D-2 | 精确字符串去重（首见序），不做 canonical/host 级去重 | 与 B-1 候选池边界一致；canonical URL/host 级去重是步骤 3 预筛职责（设计 §2.2） |
| D-3 | 超限拒绝返回 `PolicyFeedback::Denied`（熔断同面） | 设计 §1.2“不给模型重试空间（与连续拒绝熔断同一处理面）”；拒绝 key=tool+reason_code+policy_revision，换 URL 仍同 key → 连续 3 轮熔断 |
| D-4 | 缺 url / 无计数域两翼 fail-closed 拒绝并计 Denied | 硬门必须可机械计数；无计数身份/计数域时放行会形成绕过面；Denied 由不同 reason_code 区分，模型修正后 key 变化即熔断重置 |
| D-5 | `tool_completed` 增加 `candidate_count`/`candidate_cap` 审计字段（Schema 先行） | 计数是步骤 2 自身反馈，模型可见文本不进 journal；事件面携带机器可读计数供回放/审计；与 B-1“citations 不扩表”边界不冲突（B-1 是避免复制候选池本身，本字段不复制 URL） |
| D-6 | 计数反馈同时追加到成功与 host 错误输出 | 失败抓取同样已消耗候选位；模型必须看到一致预算 |
| D-7 | internal lane 传 None、external lane 传共享域 | web_fetch 仅在 external lane 自执行（lane_self_execute）；内部车道永不触达，None 时门禁 fail-closed（防御性） |
| D-8 | 门禁置于 mode 门之后、permission/ACAF/ToolStarted 之前 | 与既有拒绝形态一致（无 ToolStarted）；被拒调用不签发动作票、不咨询 permission |

## 4. 边界（明确未做，登记给后续步骤）

- 机械预筛（候选池净化 + tier/weight 排序标签）——步骤 3。
- browser_read 范围/模式参数（全文/预览/关键词提取）与 local_browser 第二段
  复用同一计数域——步骤 4（当前 browser_read 仍为单调用形态，无第二段可计）。
- 输出级引用校验器与交付边界接线——步骤 5。
- 提示词相应缩短（计数/预筛/引用规则）与测试更新——步骤 6。
- canonical URL / host 级去重——步骤 3 预筛规则（B-1 与 D-2 一致）。
- DC 剩余两信号（`same_module_no_evidence` / `key_surface_unexamined`）——
  建议并入批次，未并入本步。
- `ORZ_WEB_FETCH_CANDIDATE_CAP` 最终值以外的额度管理（无 per-URL 权重等）。

## 5. 验证

- `cargo test -p orz-loop`：**245 passed / 0 failed / 3 ignored**（+4 计数
  门禁测试：满额/去重/拒绝、缺参/无域 fail-closed、cap 覆盖 seam、外部 lane
  计数回流；+1 continue 跨派发累计测试；+1 env 解析规则测试；+1 快照往返
  断言扩展；+relay 家族边界测试）
- `cargo test -p orz-host`：**200 passed / 0 failed / 4 ignored**（未改动）
- `cargo test -p orz-assurance`：**139 passed / 0 failed**
- `cargo test -p orz-tui`：**178 passed / 0 failed**
- `cargo test -p orz-bin`：全绿（含 orz-signer 12）
- Python `runtime/tests/test_run_event_journal_validation.py`：
  **129 passed / 0 failed**（+9 计数规则测试）
- Python `assurance/tests` + `runtime/tests` 全量：**1828 passed /
  14 skipped**（B-1 基线 1821 + 7）
- `scripts/check_repository.py`：**valid / error_count 0**（schemas 241）
- `cargo fmt --all` 已跑；两个仓库 `git diff --check` 均 0
- 备注：`cargo test --workspace` 中 orz-agent 35 项失败为沙箱环境问题
  （git 无法访问临时目录 `拒绝访问`），与本批无关、该 crate 未改动；
  与既有基线一致按 per-crate 验证。

## 6. 仓库边界与提交顺序

- 本步横跨两个 git 仓库：父仓库 `D:\CLI`（Schema/verifier/设计/审计/待办）与
  嵌套仓库 `D:\CLI\orz`（Rust 实现）。
- 提交顺序：**先提交父仓库，再提交 orz 仓库**（与 GAP-SOURCE-WEIGHTING-IMPL
  审计 §7、B-1 审计 §6 一致）。
- 注意：orz 嵌套仓库当前仍携带 B-1 批次未提交改动（host/tools.rs 结构化接缝、
  loop/host.rs ToolResult.structured、orz-workspace 测试桩等）；提交 orz 仓库时
  应把 B-1 遗留与本步改动一并登记（同一 P0-B 实施流）。

## 7. 审查修复（2026-08-14 复核，提交前）

- **P1 修复（verifier 派发包装误报）**：`_verify_v02_web_fetch_candidate_count`
  原规则把“非 error 的 web_fetch 完成事件必须携带计数字段”套到了检索派发包装
  事件上（`run_retrieval_subagent` Ok 路径的 `tool_completed{tool, call_id,
  target, exit_code}`——它只是父调用的派发回执，不是真实抓取），实证任何含
  web_fetch 派发的真实 journal 都会误报。修复：仅当 `target` 缺失（真实车道
  执行）时强制携带；`target` 存在的派发包装不要求也不允许计数字段；补
  `dispatch wrapper valid` 正例与 `cap-exceeded 缺字段` 负例两条测试。
- **P2 修复（黑板 exec 镜像一致性）**：计数反馈现于 match 前统一计算，成功/
  失败路径的 blackboard exec 镜像与对话消息均携带“候选 N/M，剩余 K”
  （此前镜像早于追加，模型经黑板读不到计数）。
- **P2 修复（continue 跨派发累计测试）**：新增
  `web_fetch_candidate_count_accumulates_across_continue_dispatches`——
  continue 后同 activation 继续累计（1/8 → 2/8），contract_revision=1，
  journal 计数 1、2 两事件。
- **P3 加固（cap-exceeded 缺字段）**：verifier 现要求
  `web_fetch_candidate_cap_exceeded` 拒绝事件必须携带
  candidate_count/candidate_cap（此前仅在字段存在时校验 count==cap）。
- **P3 注记（browser_read 计数域）**：设计 §1.3 补注——browser_read 为主
  车道工具（无 activation），步骤 4 实施前须裁决第二段计数域挂载面；TODO
  步骤 4 同步登记。
- **P2 文档修正**：BACKLOG orz-loop 计数 243→244、变更记录测试数更新。
